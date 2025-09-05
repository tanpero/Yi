use winapi::um::winuser::*;
use winapi::um::shellapi::*;
use winapi::shared::windef::*;
use winapi::shared::minwindef::*;
use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;
use std::ptr;
use winapi::um::libloaderapi::*;
use crate::app_state::InputMode;
use std::sync::{Arc, Mutex};

const WM_TRAYICON: u32 = WM_USER + 1;
const ID_TRAY_ICON: u32 = 1001;

// 菜单项ID
const ID_MENU_ABOUT: i32 = 2001;
const ID_MENU_EXIT: i32 = 2002;
const ID_MENU_INPUT_MODE: i32 = 2003;
const ID_MENU_YI_ONLY: i32 = 2004;
const ID_MENU_PINYIN_YI: i32 = 2005;
const ID_MENU_PINYIN_WITH_YI: i32 = 2006;
const ID_MENU_YI_WITH_PINYIN: i32 = 2007;
const ID_MENU_HTML_RUBY: i32 = 2008;

// 全局变量
static mut CURRENT_INPUT_MODE: InputMode = InputMode::YiOnly;
static mut INPUT_MODE_CALLBACK: Option<Box<dyn Fn(InputMode) + Send + Sync>> = None;

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
            // 加载应用图标
            let hicon = LoadIconW(GetModuleHandleW(ptr::null()), MAKEINTRESOURCEW(1));
            let hicon_sm = LoadIconW(GetModuleHandleW(ptr::null()), MAKEINTRESOURCEW(1));
            
            let wc = WNDCLASSEXW {
                cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
                style: 0,
                lpfnWndProc: Some(tray_window_proc),
                cbClsExtra: 0,
                cbWndExtra: 0,
                hInstance: GetModuleHandleW(ptr::null()),
                hIcon: hicon,
                hCursor: ptr::null_mut(),
                hbrBackground: ptr::null_mut(),
                lpszMenuName: ptr::null(),
                lpszClassName: class_name.as_ptr(),
                hIconSm: hicon_sm,
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
            // 加载自定义图标
            let hicon = LoadIconW(GetModuleHandleW(ptr::null()), MAKEINTRESOURCEW(1));
            let icon = if hicon.is_null() {
                // 如果加载失败，使用默认图标
                LoadIconW(ptr::null_mut(), IDI_APPLICATION)
            } else {
                hicon
            };
            
            let mut nid = NOTIFYICONDATAW {
                cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
                hWnd: self.hwnd,
                uID: ID_TRAY_ICON,
                uFlags: NIF_ICON | NIF_MESSAGE | NIF_TIP,
                uCallbackMessage: WM_TRAYICON,
                hIcon: icon,
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
            let tip = to_wide_string("彝文输入法 - 按F4切换输入模式");
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
    
    pub fn set_input_mode_callback<F>(&self, callback: F) 
    where 
        F: Fn(InputMode) + Send + Sync + 'static,
    {
        unsafe {
            INPUT_MODE_CALLBACK = Some(Box::new(callback));
        }
    }
    
    pub fn update_input_mode(&self, mode: InputMode) {
        unsafe {
            CURRENT_INPUT_MODE = mode;
        }
    }
}

// 修改create_context_menu函数
unsafe fn create_context_menu() -> HMENU {
    let hmenu = CreatePopupMenu();
    
    // 创建"输入形式"子菜单
    let input_mode_submenu = CreatePopupMenu();
    
    // 定义菜单项数据
    let menu_items = [
        (ID_MENU_YI_ONLY, InputMode::YiOnly, "彝文"),
        (ID_MENU_PINYIN_YI, InputMode::PinyinYi, "拼音+彝文"),
        (ID_MENU_PINYIN_WITH_YI, InputMode::PinyinWithYi, "拼音（彝文）"),
        (ID_MENU_YI_WITH_PINYIN, InputMode::YiWithPinyin, "彝文（拼音）"),
        (ID_MENU_HTML_RUBY, InputMode::HtmlRuby, "HTML注音"),
    ];
    
    // 添加子菜单项，只使用原生的 MF_CHECKED 标志
    for (id, mode, text) in menu_items.iter() {
        AppendMenuW(
            input_mode_submenu,
            MF_STRING | if *mode == CURRENT_INPUT_MODE { MF_CHECKED } else { 0 },
            *id as usize,
            to_wide_string(text).as_ptr()
        );
    }
    
    // 添加"输入形式"主菜单项
    AppendMenuW(
        hmenu,
        MF_STRING | MF_POPUP,
        input_mode_submenu as usize,
        to_wide_string("输入形式").as_ptr()
    );
    
    // 添加分隔线
    AppendMenuW(hmenu, MF_SEPARATOR, 0, ptr::null());
    
    // 添加"关于"菜单项
    AppendMenuW(
        hmenu,
        MF_STRING,
        ID_MENU_ABOUT as usize,
        to_wide_string("关于").as_ptr()
    );
    
    // 添加"退出"菜单项
    AppendMenuW(
        hmenu,
        MF_STRING,
        ID_MENU_EXIT as usize,
        to_wide_string("退出").as_ptr()
    );
    
    hmenu
}

unsafe fn show_context_menu(hwnd: HWND) {
    let hmenu = create_context_menu();
    
    // 获取鼠标位置
    let mut pt = POINT { x: 0, y: 0 };
    GetCursorPos(&mut pt);
    
    // 设置前台窗口，确保菜单能正确显示和消失
    SetForegroundWindow(hwnd);
    
    // 显示菜单
    let cmd = TrackPopupMenu(
        hmenu,
        TPM_RIGHTBUTTON | TPM_RETURNCMD,
        pt.x,
        pt.y,
        0,
        hwnd,
        ptr::null()
    );
    
    // 处理菜单选择
    match cmd {
        ID_MENU_ABOUT => {
            MessageBoxW(
                hwnd,
                to_wide_string("彝文输入法 1.0.0\n\n按F4激活/关闭输入彝文输入模式\n\nCamille Dolma © 2025").as_ptr(),
                to_wide_string("关于 - 彝文输入法").as_ptr(),
                MB_OK | MB_ICONINFORMATION
            );
        }
        ID_MENU_YI_ONLY => {
            CURRENT_INPUT_MODE = InputMode::YiOnly;
            if let Some(ref callback) = INPUT_MODE_CALLBACK {
                callback(InputMode::YiOnly);
            }
        }
        ID_MENU_PINYIN_YI => {
            CURRENT_INPUT_MODE = InputMode::PinyinYi;
            if let Some(ref callback) = INPUT_MODE_CALLBACK {
                callback(InputMode::PinyinYi);
            }
        }
        ID_MENU_PINYIN_WITH_YI => {
            CURRENT_INPUT_MODE = InputMode::PinyinWithYi;
            if let Some(ref callback) = INPUT_MODE_CALLBACK {
                callback(InputMode::PinyinWithYi);
            }
        }
        ID_MENU_YI_WITH_PINYIN => {
            CURRENT_INPUT_MODE = InputMode::YiWithPinyin;
            if let Some(ref callback) = INPUT_MODE_CALLBACK {
                callback(InputMode::YiWithPinyin);
            }
        }
        ID_MENU_HTML_RUBY => {
            CURRENT_INPUT_MODE = InputMode::HtmlRuby;
            if let Some(ref callback) = INPUT_MODE_CALLBACK {
                callback(InputMode::HtmlRuby);
            }
        }
        ID_MENU_EXIT => {
            // 移除托盘图标
            let mut nid = NOTIFYICONDATAW {
                cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
                hWnd: hwnd,
                uID: ID_TRAY_ICON,
                uFlags: 0,
                uCallbackMessage: 0,
                hIcon: ptr::null_mut(),
                szTip: [0; 128],
                dwState: 0,
                dwStateMask: 0,
                szInfo: [0; 256],
                u: std::mem::zeroed(),
                szInfoTitle: [0; 64],
                dwInfoFlags: 0,
                guidItem: std::mem::zeroed(),
                hBalloonIcon: ptr::null_mut(),
            };
            Shell_NotifyIconW(NIM_DELETE, &mut nid);
            
            // 强制退出进程，确保彻底关闭
            unsafe {
                use winapi::um::processthreadsapi::ExitProcess;
                ExitProcess(0);
            }
        }
        _ => {}
    }
    
    // 清理菜单
    DestroyMenu(hmenu);
    
    // 发送一个空消息来确保菜单正确消失
    PostMessageW(hwnd, WM_NULL, 0, 0);
}

unsafe extern "system" fn tray_window_proc(
    hwnd: HWND,
    msg: UINT,
    wparam: WPARAM,
    lparam: LPARAM
) -> LRESULT {
    match msg {
        WM_TRAYICON => {
            match lparam as u32 {
                WM_RBUTTONUP => {
                    // 右键单击 - 显示上下文菜单
                    show_context_menu(hwnd);
                }
                WM_LBUTTONDBLCLK => {
                    // 双击左键 - 显示关于对话框
                    MessageBoxW(
                        hwnd,
                        to_wide_string("彝文输入法 v0.2.0\n\n按F4激活/关闭输入法").as_ptr(),
                        to_wide_string("彝文输入法").as_ptr(),
                        MB_OK | MB_ICONINFORMATION
                    );
                }
                _ => {}
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

// 添加 MAKEINTRESOURCEW 宏
macro_rules! MAKEINTRESOURCEW {
    ($i:expr) => {
        $i as *const u16
    };
}