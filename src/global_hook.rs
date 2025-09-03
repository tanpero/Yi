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
}

#[derive(Debug, Clone)]
pub struct KeyEvent {
    pub vk_code: u32,
    pub scan_code: u32,
    pub flags: u32,
    pub is_key_down: bool,
}

impl GlobalHook {
    pub fn new() -> (Self, Receiver<KeyEvent>) {
        let (sender, receiver) = channel();
        let hook = GlobalHook {
            hook: std::ptr::null_mut(),
            active: Arc::new(Mutex::new(false)),
            sender,
        };
        (hook, receiver)
    }
    
    pub fn install(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        unsafe {
            // 设置全局变量供回调函数使用
            GLOBAL_SENDER = Some(self.sender.clone());
            GLOBAL_ACTIVE = Some(self.active.clone());
            
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
}

// 全局变量（用于回调函数）
static mut GLOBAL_SENDER: Option<Sender<KeyEvent>> = None;
static mut GLOBAL_ACTIVE: Option<Arc<Mutex<bool>>> = None;

unsafe extern "system" fn keyboard_proc(
    n_code: i32,
    w_param: WPARAM,
    l_param: LPARAM
) -> LRESULT {
    if n_code >= 0 {
        let kb_struct = *(l_param as *const KBDLLHOOKSTRUCT);
        let is_key_down = w_param == WM_KEYDOWN as WPARAM || w_param == WM_SYSKEYDOWN as WPARAM;
        
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
            // 只处理字母键
            if kb_struct.vkCode >= 0x41 && kb_struct.vkCode <= 0x5A { // A-Z
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
        }
    }
    
    CallNextHookEx(std::ptr::null_mut(), n_code, w_param, l_param)
}