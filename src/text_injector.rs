use crate::tsf_bridge::TSFBridge;
use winapi::um::winuser::*;
use winapi::shared::minwindef::*;
use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;
use std::thread;
use std::time::Duration;

pub struct TextInjector {
    tsf_bridge: Option<TSFBridge>,
    fallback_mode: bool,
}

impl TextInjector {
    pub fn new() -> Self {
        // 尝试初始化TSF
        let tsf_bridge = match TSFBridge::new() {
            Ok(bridge) => {
                println!("✓ TSF模式已启用");
                Some(bridge)
            },
            Err(e) => {
                println!("⚠ TSF初始化失败: {}，使用剪贴板回退模式", e);
                None
            }
        };
        
        let fallback_mode = tsf_bridge.is_none();
        
        TextInjector {
            tsf_bridge,
            fallback_mode,
        }
    }
    
    pub fn inject_text(&self, text: &str) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(ref tsf) = self.tsf_bridge {
            // 优先使用TSF进行真正的文本插入
            match tsf.insert_text(text) {
                Ok(_) => {
                    println!("✓ TSF文本插入成功: {}", text);
                    Ok(())
                },
                Err(e) => {
                    println!("⚠ TSF文本插入失败: {}，回退到剪贴板模式", e);
                    self.inject_via_clipboard(text)
                }
            }
        } else {
            // 回退到剪贴板模式
            println!("使用剪贴板模式插入文本: {}", text);
            self.inject_via_clipboard(text)
        }
    }
    
    pub fn is_tsf_enabled(&self) -> bool {
        self.tsf_bridge.is_some()
    }
    
    pub fn get_mode_description(&self) -> &'static str {
        if self.tsf_bridge.is_some() {
            "TSF模式（真正的输入法接口）"
        } else {
            "剪贴板模式（Ctrl+V回退）"
        }
    }
    
    // 保留原有的剪贴板方法作为回退
    fn inject_via_clipboard(&self, text: &str) -> Result<(), Box<dyn std::error::Error>> {
        use winapi::um::winuser::{OpenClipboard, EmptyClipboard, SetClipboardData, CloseClipboard};
        use winapi::um::winbase::{GlobalAlloc, GlobalLock, GlobalUnlock, GMEM_MOVEABLE};
        use winapi::um::winuser::CF_UNICODETEXT;
        
        let wide_text: Vec<u16> = OsStr::new(text)
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();
        
        unsafe {
            if OpenClipboard(std::ptr::null_mut()) == 0 {
                return Err("Failed to open clipboard".into());
            }
            
            EmptyClipboard();
            
            let h_mem = GlobalAlloc(GMEM_MOVEABLE, wide_text.len() * 2);
            if h_mem.is_null() {
                CloseClipboard();
                return Err("Failed to allocate memory".into());
            }
            
            let p_mem = GlobalLock(h_mem) as *mut u16;
            std::ptr::copy_nonoverlapping(wide_text.as_ptr(), p_mem, wide_text.len());
            GlobalUnlock(h_mem);
            
            SetClipboardData(CF_UNICODETEXT, h_mem);
            CloseClipboard();
            
            thread::sleep(Duration::from_millis(10));
            
            self.send_ctrl_v();
            
            thread::sleep(Duration::from_millis(50));
            
            if OpenClipboard(std::ptr::null_mut()) != 0 {
                EmptyClipboard();
                CloseClipboard();
            }
        }
        
        Ok(())
    }
    
    fn send_ctrl_v(&self) {
        unsafe {
            let mut inputs = [INPUT {
                type_: INPUT_KEYBOARD,
                u: std::mem::zeroed(),
            }; 4];
            
            *inputs[0].u.ki_mut() = KEYBDINPUT {
                wVk: VK_CONTROL as u16,
                wScan: 0,
                dwFlags: 0,
                time: 0,
                dwExtraInfo: 0,
            };
            
            *inputs[1].u.ki_mut() = KEYBDINPUT {
                wVk: 0x56,
                wScan: 0,
                dwFlags: 0,
                time: 0,
                dwExtraInfo: 0,
            };
            
            *inputs[2].u.ki_mut() = KEYBDINPUT {
                wVk: 0x56,
                wScan: 0,
                dwFlags: KEYEVENTF_KEYUP,
                time: 0,
                dwExtraInfo: 0,
            };
            
            *inputs[3].u.ki_mut() = KEYBDINPUT {
                wVk: VK_CONTROL as u16,
                wScan: 0,
                dwFlags: KEYEVENTF_KEYUP,
                time: 0,
                dwExtraInfo: 0,
            };
            
            SendInput(4, inputs.as_mut_ptr(), std::mem::size_of::<INPUT>() as i32);
        }
    }
}