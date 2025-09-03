use winapi::um::winuser::*;
use winapi::um::libloaderapi::GetModuleHandleW;
use winapi::shared::windef::*;
use winapi::shared::minwindef::*;
use std::sync::{Arc, Mutex};
use std::sync::mpsc::{Sender, Receiver, channel};

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
static mut INPUT_BUFFER_EMPTY: bool = true; // 添加这个变量
static mut INJECTING_TEXT: bool = false; // 新增：标记是否正在注入文本

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
    
    pub fn uninstall(&mut self) {
        if !self.hook.is_null() {
            unsafe {
                UnhookWindowsHookEx(self.hook);
            }
            self.hook = std::ptr::null_mut();
        }
    }
    
    pub fn set_active(&self, active: bool) {
        if let Ok(mut state) = self.active.lock() {
            *state = active;
        }
    }
    
    pub fn is_active(&self) -> bool {
        *self.active.lock().unwrap_or_else(|_| panic!("Failed to lock mutex"))
    }
    
    // 添加 set_has_input 方法到 impl 块中
    pub fn set_has_input(&self, has_input: bool) {
        if let Ok(mut state) = self.has_input.lock() {
            *state = has_input;
        }
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
        
        // F4 键切换输入法状态
        if kb_struct.vkCode == VK_F4 as u32 && is_key_down {
            if let Some(ref active) = GLOBAL_ACTIVE {
                if let Ok(mut state) = active.lock() {
                    *state = !*state;
                    println!("输入法状态: {}", if *state { "激活" } else { "关闭" });
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
            println!("输入法已激活，检查按键: vk_code={}", kb_struct.vkCode);
            
            // 处理字母键 A-Z
            if kb_struct.vkCode >= 0x41 && kb_struct.vkCode <= 0x5A {
                println!("发送字母键事件: {}", kb_struct.vkCode);
                if let Some(ref sender) = GLOBAL_SENDER {
                    let event = KeyEvent {
                        vk_code: kb_struct.vkCode,
                        scan_code: kb_struct.scanCode,
                        flags: kb_struct.flags,
                        is_key_down,
                    };
                    if let Err(e) = sender.send(event) {
                        println!("发送事件失败: {:?}", e);
                    }
                }
                return 1; // 阻止按键传递给应用程序
            }
            // 处理数字键 1-9
            else if kb_struct.vkCode >= 0x31 && kb_struct.vkCode <= 0x39 {
                // 如果输入缓冲区为空，不拦截数字键，让系统处理
                if INPUT_BUFFER_EMPTY {
                    return CallNextHookEx(std::ptr::null_mut(), n_code, w_param, l_param);
                }
                
                println!("发送数字键事件: {}", kb_struct.vkCode);
                if let Some(ref sender) = GLOBAL_SENDER {
                    let event = KeyEvent {
                        vk_code: kb_struct.vkCode,
                        scan_code: kb_struct.scanCode,
                        flags: kb_struct.flags,
                        is_key_down,
                    };
                    if let Err(e) = sender.send(event) {
                        println!("发送事件失败: {:?}", e);
                    }
                }
                return 1; // 阻止按键传递给应用程序
            }
            // 处理退格键
            else if kb_struct.vkCode == VK_BACK as u32 {
                // 如果输入缓冲区为空，不拦截退格键，让系统处理
                if INPUT_BUFFER_EMPTY {
                    return CallNextHookEx(std::ptr::null_mut(), n_code, w_param, l_param);
                }
                
                println!("发送退格键事件");
                if let Some(ref sender) = GLOBAL_SENDER {
                    let event = KeyEvent {
                        vk_code: kb_struct.vkCode,
                        scan_code: kb_struct.scanCode,
                        flags: kb_struct.flags,
                        is_key_down,
                    };
                    if let Err(e) = sender.send(event) {
                        println!("发送事件失败: {:?}", e);
                    }
                }
                return 1; // 阻止按键传递给应用程序
            }
            // 处理空格键
            else if kb_struct.vkCode == VK_SPACE as u32 {
                println!("发送空格键事件");
                if let Some(ref sender) = GLOBAL_SENDER {
                    let event = KeyEvent {
                        vk_code: kb_struct.vkCode,
                        scan_code: kb_struct.scanCode,
                        flags: kb_struct.flags,
                        is_key_down,
                    };
                    if let Err(e) = sender.send(event) {
                        println!("发送事件失败: {:?}", e);
                    }
                }
                return 1; // 阻止按键传递给应用程序
            }
            // 处理ESC键
            else if kb_struct.vkCode == VK_ESCAPE as u32 {
                println!("发送ESC键事件");
                if let Some(ref sender) = GLOBAL_SENDER {
                    let event = KeyEvent {
                        vk_code: kb_struct.vkCode,
                        scan_code: kb_struct.scanCode,
                        flags: kb_struct.flags,
                        is_key_down,
                    };
                    if let Err(e) = sender.send(event) {
                        println!("发送事件失败: {:?}", e);
                    }
                }
                return 1; // 阻止按键传递给应用程序
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