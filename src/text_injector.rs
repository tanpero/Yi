use winapi::um::winuser::*;
use winapi::shared::minwindef::*;
use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;

pub struct TextInjector;

impl TextInjector {
    pub fn new() -> Self {
        TextInjector
    }
    
    pub fn inject_text(&self, text: &str) -> Result<(), Box<dyn std::error::Error>> {
        // 方法1：使用剪贴板 + Ctrl+V（最简单可靠）
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
            
            // 发送 Ctrl+V
            self.send_ctrl_v();
        }
        
        Ok(())
    }
    
    fn send_ctrl_v(&self) {
        unsafe {
            // 按下 Ctrl
            keybd_event(VK_CONTROL as u8, 0, 0, 0);
            // 按下 V
            keybd_event(0x56, 0, 0, 0); // V key
            // 释放 V
            keybd_event(0x56, 0, KEYEVENTF_KEYUP, 0);
            // 释放 Ctrl
            keybd_event(VK_CONTROL as u8, 0, KEYEVENTF_KEYUP, 0);
        }
    }
}