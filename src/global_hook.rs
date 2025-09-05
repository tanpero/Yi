use winapi::um::winuser::*;
use winapi::um::libloaderapi::GetModuleHandleW;
use winapi::shared::windef::*;
use winapi::shared::minwindef::*;
use std::sync::{Arc, Mutex};
use std::sync::mpsc::{Sender, Receiver, channel};
use crate::app_state::EnglishInputState; // 新增导入

pub struct GlobalHook {
    hook: HHOOK,
    active: Arc<Mutex<bool>>,
    sender: Sender<KeyEvent>,
    has_input: Arc<Mutex<bool>>, // 添加这个字段
}

#[derive(Debug, Clone)]
pub struct KeyEvent {
    pub vk_code: u32,
    pub scan_code: u32,
    pub flags: u32,
    pub is_key_down: bool,
}

// 全局变量
static mut GLOBAL_SENDER: Option<Sender<KeyEvent>> = None;
static mut GLOBAL_ACTIVE: Option<Arc<Mutex<bool>>> = None;
static mut GLOBAL_HAS_INPUT: Option<Arc<Mutex<bool>>> = None;
static mut INPUT_BUFFER_EMPTY: bool = true;
static mut INJECTING_TEXT: bool = false;
static mut ENGLISH_INPUT_STATE: EnglishInputState = EnglishInputState::Yi; // 新增英文输入状态

impl GlobalHook {
    // 修改 new 方法
    pub fn new() -> (Self, Receiver<KeyEvent>) {
        let (sender, receiver) = channel();
        let hook = GlobalHook {
            hook: std::ptr::null_mut(),
            active: Arc::new(Mutex::new(false)),
            sender,
            has_input: Arc::new(Mutex::new(false)),
        };
        (hook, receiver)
    }
    
    // 修改 install 方法
    pub fn install(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        unsafe {
            GLOBAL_SENDER = Some(self.sender.clone());
            GLOBAL_ACTIVE = Some(self.active.clone());
            GLOBAL_HAS_INPUT = Some(self.has_input.clone());
            
            self.hook = SetWindowsHookExW(
                WH_KEYBOARD_LL,
                Some(keyboard_proc),
                GetModuleHandleW(std::ptr::null()),
                0
            );
            
            if self.hook.is_null() {
                return Err("Failed to install keyboard hook".into());
            }
        }
        Ok(())
    }
    
}

// 全局变量（用于回调函数）
// 修改键盘处理函数
unsafe extern "system" fn keyboard_proc(
    n_code: i32,
    w_param: WPARAM,
    l_param: LPARAM
) -> LRESULT {
    if n_code >= 0 {
        let kb_struct = *(l_param as *const KBDLLHOOKSTRUCT);
        let is_key_down = w_param == WM_KEYDOWN as WPARAM || w_param == WM_SYSKEYDOWN as WPARAM;
        
        // 如果正在注入文本，不要拦截任何按键
        if INJECTING_TEXT {
            return CallNextHookEx(std::ptr::null_mut(), n_code, w_param, l_param);
        }
        
        // 检测组合键状态
        let ctrl_pressed = GetAsyncKeyState(VK_CONTROL) & 0x8000u16 as i16 != 0;
        let alt_pressed = GetAsyncKeyState(VK_MENU) & 0x8000u16 as i16 != 0;
        let shift_pressed = GetAsyncKeyState(VK_SHIFT) & 0x8000u16 as i16 != 0;
        
        // F4 键切换输入法状态
        if kb_struct.vkCode == VK_F4 as u32 && is_key_down {
            if let Some(ref active) = GLOBAL_ACTIVE {
                if let Ok(mut state) = active.lock() {
                    *state = !*state;
                    // 当输入法状态改变时，重置英文输入状态为彝文模式
                    if *state {
                        ENGLISH_INPUT_STATE = EnglishInputState::Yi;
                    }
                    // println!("输入法状态: {}", if *state { "激活" } else { "关闭" });
                }
            }
            return 1; // 阻止F4传递给应用程序
        }
        
        // 检查输入法是否激活
        let is_active = if let Some(ref active) = GLOBAL_ACTIVE {
            *active.lock().unwrap_or_else(|_| panic!("Failed to lock mutex"))
        } else {
            false
        };
        
        if is_active {
            // println!("输入法已激活，检查按键: vk_code={}", kb_struct.vkCode);
            
            // 处理Shift键和Caps Lock键（只在缓冲区为空时）
            if INPUT_BUFFER_EMPTY && is_key_down {
                // 处理Shift键
                if kb_struct.vkCode == VK_SHIFT as u32 {
                    match ENGLISH_INPUT_STATE {
                        EnglishInputState::Yi => {
                            ENGLISH_INPUT_STATE = EnglishInputState::LowerCase;
                            // println!("切换到英文小写输入模式");
                        },
                        EnglishInputState::LowerCase | EnglishInputState::UpperCase => {
                            ENGLISH_INPUT_STATE = EnglishInputState::Yi;
                            // println!("恢复彝文输入模式");
                        }
                    }
                    // 让系统处理Shift键，不拦截
                    return CallNextHookEx(std::ptr::null_mut(), n_code, w_param, l_param);
                }
                
                // 处理Caps Lock键
                if kb_struct.vkCode == VK_CAPITAL as u32 {
                    match ENGLISH_INPUT_STATE {
                        EnglishInputState::Yi => {
                            ENGLISH_INPUT_STATE = EnglishInputState::UpperCase;
                            // println!("切换到英文大写输入模式");
                        },
                        EnglishInputState::LowerCase | EnglishInputState::UpperCase => {
                            ENGLISH_INPUT_STATE = EnglishInputState::Yi;
                            // println!("恢复彝文输入模式");
                        }
                    }
                    // 让系统处理Caps Lock键，不拦截
                    return CallNextHookEx(std::ptr::null_mut(), n_code, w_param, l_param);
                }
            }
            
            // 如果缓冲区为空，根据英文输入状态决定是否拦截字母键
            if INPUT_BUFFER_EMPTY {
                // 在英文输入模式下，不拦截字母键，让系统处理
                if matches!(ENGLISH_INPUT_STATE, EnglishInputState::LowerCase | EnglishInputState::UpperCase) {
                    if kb_struct.vkCode >= 0x41 && kb_struct.vkCode <= 0x5A && !ctrl_pressed && !alt_pressed {
                        // println!("英文输入模式，让系统处理字母键: {}", kb_struct.vkCode);
                        return CallNextHookEx(std::ptr::null_mut(), n_code, w_param, l_param);
                    }
                }
                
                // 在彝文输入模式下，只有在没有任何修饰键按下时才拦截字母键
                if matches!(ENGLISH_INPUT_STATE, EnglishInputState::Yi) {
                    if kb_struct.vkCode >= 0x41 && kb_struct.vkCode <= 0x5A && !ctrl_pressed && !alt_pressed && !shift_pressed {
                        // println!("发送字母键事件: {}", kb_struct.vkCode);
                        if let Some(ref sender) = GLOBAL_SENDER {
                            let event = KeyEvent {
                                vk_code: kb_struct.vkCode,
                                scan_code: kb_struct.scanCode,
                                flags: kb_struct.flags,
                                is_key_down,
                            };
                            if let Err(e) = sender.send(event) {
                                // println!("发送事件失败: {:?}", e);
                            }
                        }
                        return 1; // 阻止按键传递给应用程序
                    }
                }
                
                // 缓冲区为空时，所有其他按键都让系统处理
                return CallNextHookEx(std::ptr::null_mut(), n_code, w_param, l_param);
            }
            
            // 处理字母键 A-Z（只有在没有修饰键时才作为输入处理）
            if kb_struct.vkCode >= 0x41 && kb_struct.vkCode <= 0x5A && !ctrl_pressed && !alt_pressed {
                // println!("发送字母键事件: {}", kb_struct.vkCode);
                if let Some(ref sender) = GLOBAL_SENDER {
                    let event = KeyEvent {
                        vk_code: kb_struct.vkCode,
                        scan_code: kb_struct.scanCode,
                        flags: kb_struct.flags,
                        is_key_down,
                    };
                    if let Err(e) = sender.send(event) {
                        // println!("发送事件失败: {:?}", e);
                    }
                }
                return 1; // 阻止按键传递给应用程序
            }
            // 处理数字键 1-9
            else if kb_struct.vkCode >= 0x31 && kb_struct.vkCode <= 0x39 && !ctrl_pressed && !alt_pressed {
                // println!("发送数字键事件: {}", kb_struct.vkCode);
                if let Some(ref sender) = GLOBAL_SENDER {
                    let event = KeyEvent {
                        vk_code: kb_struct.vkCode,
                        scan_code: kb_struct.scanCode,
                        flags: kb_struct.flags,
                        is_key_down,
                    };
                    if let Err(e) = sender.send(event) {
                        // println!("发送事件失败: {:?}", e);
                    }
                }
                return 1; // 阻止按键传递给应用程序
            }
            // 处理退格键
            else if kb_struct.vkCode == VK_BACK as u32 && !ctrl_pressed && !alt_pressed {
                // println!("发送退格键事件");
                if let Some(ref sender) = GLOBAL_SENDER {
                    let event = KeyEvent {
                        vk_code: kb_struct.vkCode,
                        scan_code: kb_struct.scanCode,
                        flags: kb_struct.flags,
                        is_key_down,
                    };
                    if let Err(e) = sender.send(event) {
                        // println!("发送事件失败: {:?}", e);
                    }
                }
                return 1; // 阻止按键传递给应用程序
            }
            // 处理空格键
            else if kb_struct.vkCode == VK_SPACE as u32 && !ctrl_pressed && !alt_pressed {
                // println!("发送空格键事件");
                if let Some(ref sender) = GLOBAL_SENDER {
                    let event = KeyEvent {
                        vk_code: kb_struct.vkCode,
                        scan_code: kb_struct.scanCode,
                        flags: kb_struct.flags,
                        is_key_down,
                    };
                    if let Err(e) = sender.send(event) {
                        // println!("发送事件失败: {:?}", e);
                    }
                }
                return 1; // 阻止按键传递给应用程序
            }
            // 处理ESC键
            else if kb_struct.vkCode == VK_ESCAPE as u32 && !ctrl_pressed && !alt_pressed {
                // println!("发送ESC键事件");
                if let Some(ref sender) = GLOBAL_SENDER {
                    let event = KeyEvent {
                        vk_code: kb_struct.vkCode,
                        scan_code: kb_struct.scanCode,
                        flags: kb_struct.flags,
                        is_key_down,
                    };
                    if let Err(e) = sender.send(event) {
                        // println!("发送事件失败: {:?}", e);
                    }
                }
                return 1; // 阻止按键传递给应用程序
            }
            // 处理特殊标点符号按键（缓冲区不为空时）
            else if !INPUT_BUFFER_EMPTY && (
                kb_struct.vkCode == 0xDB || // [ 键
                kb_struct.vkCode == 0xDD || // ] 键
                kb_struct.vkCode == 0xDC || // \ 键
                kb_struct.vkCode == 0xBA || // ; 键
                kb_struct.vkCode == 0xBC || // , 键
                kb_struct.vkCode == 0xBE    // . 键
            ) {
                // println!("发送特殊标点符号事件: {}", kb_struct.vkCode);
                if let Some(ref sender) = GLOBAL_SENDER {
                    let event = KeyEvent {
                        vk_code: kb_struct.vkCode,
                        scan_code: kb_struct.scanCode,
                        flags: kb_struct.flags,
                        is_key_down,
                    };
                    if let Err(e) = sender.send(event) {
                        // println!("发送事件失败: {:?}", e);
                    }
                }
                return 1; // 阻止按键传递给应用程序
            }
            // 所有其他按键（包括组合键）都让系统处理
            else {
                return CallNextHookEx(std::ptr::null_mut(), n_code, w_param, l_param);
            }
        }
    }
    
    CallNextHookEx(std::ptr::null_mut(), n_code, w_param, l_param)
}

// 添加设置注入状态的函数
pub fn set_injecting_text(injecting: bool) {
    unsafe {
        INJECTING_TEXT = injecting;
    }
}

// 添加设置输入缓冲区状态的函数
pub fn set_input_buffer_empty(empty: bool) {
    unsafe {
        INPUT_BUFFER_EMPTY = empty;
    }
}

// 添加设置英文输入状态的函数
pub fn set_english_input_state(state: EnglishInputState) {
    unsafe {
        ENGLISH_INPUT_STATE = state;
    }
}

// 添加获取英文输入状态的函数
pub fn get_english_input_state() -> EnglishInputState {
    unsafe {
        ENGLISH_INPUT_STATE
    }
}