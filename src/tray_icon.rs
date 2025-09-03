use winapi::um::winuser::*;
use winapi::um::shellapi::*;
use winapi::shared::windef::*;
use winapi::shared::minwindef::*;
use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;
use std::ptr;
use winapi::um::libloaderapi::*;

const WM_TRAYICON: u32 = WM_USER + 1;
const ID_TRAY_ICON: u32 = 1001;

pub struct TrayIcon {
    hwnd: HWND,
    active: bool,
}

impl TrayIcon {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let mut tray = TrayIcon {
            hwnd: ptr::null_mut(),
            active: false,
        };
        tray.create_hidden_window()?;
        tray.add_tray_icon()?;
        Ok(tray)
    }
    
    fn create_hidden_window(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let class_name = to_wide_string("YiTrayWindow");
        
        unsafe {
            let wc = WNDCLASSEXW {
                cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
                style: 0,
                lpfnWndProc: Some(tray_window_proc),
                cbClsExtra: 0,
                cbWndExtra: 0,
                hInstance: GetModuleHandleW(ptr::null()),
                hIcon: ptr::null_mut(),
                hCursor: ptr::null_mut(),
                hbrBackground: ptr::null_mut(),
                lpszMenuName: ptr::null(),
                lpszClassName: class_name.as_ptr(),
                hIconSm: ptr::null_mut(),
            };
            
            RegisterClassExW(&wc);
            
            self.hwnd = CreateWindowExW(
                0,
                class_name.as_ptr(),
                to_wide_string("YiTray").as_ptr(),
                0,
                0, 0, 0, 0,
                ptr::null_mut(),
                ptr::null_mut(),
                GetModuleHandleW(ptr::null()),
                ptr::null_mut()
            );
        }
        Ok(())
    }
    
    fn add_tray_icon(&self) -> Result<(), Box<dyn std::error::Error>> {
        unsafe {
            let mut nid = NOTIFYICONDATAW {
                cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
                hWnd: self.hwnd,
                uID: ID_TRAY_ICON,
                uFlags: NIF_ICON | NIF_MESSAGE | NIF_TIP,
                uCallbackMessage: WM_TRAYICON,
                hIcon: LoadIconW(ptr::null_mut(), IDI_APPLICATION),
                szTip: [0; 128],
                dwState: 0,
                dwStateMask: 0,
                szInfo: [0; 256],
                u: unsafe { std::mem::zeroed() },
                szInfoTitle: [0; 64],
                dwInfoFlags: 0,
                guidItem: unsafe { std::mem::zeroed() },
                hBalloonIcon: ptr::null_mut(),
            };
            
            // 设置提示文本
            let tip = to_wide_string("彝文输入法 - 按F4激活");
            for (i, &ch) in tip.iter().take(127).enumerate() {
                nid.szTip[i] = ch;
            }
            
            Shell_NotifyIconW(NIM_ADD, &mut nid);
        }
        Ok(())
    }
    
    pub fn update_status(&mut self, active: bool) {
        self.active = active;
        // 可以更新托盘图标状态
    }
}

unsafe extern "system" fn tray_window_proc(
    hwnd: HWND,
    msg: UINT,
    wparam: WPARAM,
    lparam: LPARAM
) -> LRESULT {
    match msg {
        WM_TRAYICON => {
            if lparam as u32 == WM_RBUTTONUP {
                // 右键菜单（简化版本暂时不实现）
                MessageBoxW(
                    hwnd,
                    to_wide_string("彝文输入法 v0.1\n按F4激活/关闭").as_ptr(),
                    to_wide_string("关于").as_ptr(),
                    MB_OK
                );
            }
            0
        }
        WM_DESTROY => {
            PostQuitMessage(0);
            0
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam)
    }
}

fn to_wide_string(s: &str) -> Vec<u16> {
    OsStr::new(s).encode_wide().chain(std::iter::once(0)).collect()
}