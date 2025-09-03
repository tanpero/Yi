use winapi::um::winuser::*;
use winapi::shared::minwindef::*;
use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;
use std::thread;
use std::time::Duration;

pub struct TextInjector;

impl TextInjector {
    pub fn new() -> Self {
        TextInjector
    }
    
    pub fn inject_text(&self, text: &str) -> Result<(), Box<dyn std::error::Error>> {
        // 方法1：使用剪贴板 + SendInput发送Ctrl+V（更可靠）
        self.inject_via_clipboard(text)
    }
    
    fn inject_via_clipboard(&self, text: &str) -> Result<(), Box<dyn std::error::Error>> {
        use winapi::um::winuser::{OpenClipboard, EmptyClipboard, SetClipboardData, CloseClipboard};
        use winapi::um::winbase::{GlobalAlloc, GlobalLock, GlobalUnlock, GMEM_MOVEABLE};
        use winapi::um::winuser::CF_UNICODETEXT;
        
        let wide_text: Vec<u16> = OsStr::new(text)
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();
        
        unsafe {
            // 打开剪贴板
            if OpenClipboard(std::ptr::null_mut()) == 0 {
                return Err("Failed to open clipboard".into());
            }
            
            // 清空剪贴板
            EmptyClipboard();
            
            // 分配内存
            let h_mem = GlobalAlloc(GMEM_MOVEABLE, wide_text.len() * 2);
            if h_mem.is_null() {
                CloseClipboard();
                return Err("Failed to allocate memory".into());
            }
            
            // 复制文本到内存
            let p_mem = GlobalLock(h_mem) as *mut u16;
            std::ptr::copy_nonoverlapping(wide_text.as_ptr(), p_mem, wide_text.len());
            GlobalUnlock(h_mem);
            
            // 设置剪贴板数据
            SetClipboardData(CF_UNICODETEXT, h_mem);
            CloseClipboard();
            
            // 等待一小段时间确保剪贴板操作完成
            thread::sleep(Duration::from_millis(10));
            
            // 使用 SendInput 发送 Ctrl+V
            self.send_ctrl_v();
            
            // 等待文本输入完成
            thread::sleep(Duration::from_millis(50));
            
            // 清除剪贴板内容
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
            
            // 按下 Ctrl
            *inputs[0].u.ki_mut() = KEYBDINPUT {
                wVk: VK_CONTROL as u16,
                wScan: 0,
                dwFlags: 0,
                time: 0,
                dwExtraInfo: 0,
            };
            
            // 按下 V
            *inputs[1].u.ki_mut() = KEYBDINPUT {
                wVk: 0x56, // V key
                wScan: 0,
                dwFlags: 0,
                time: 0,
                dwExtraInfo: 0,
            };
            
            // 释放 V
            *inputs[2].u.ki_mut() = KEYBDINPUT {
                wVk: 0x56, // V key
                wScan: 0,
                dwFlags: KEYEVENTF_KEYUP,
                time: 0,
                dwExtraInfo: 0,
            };
            
            // 释放 Ctrl
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