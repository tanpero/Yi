use winapi::um::winuser::*;
use winapi::um::shellapi::*;
use winapi::shared::windef::*;
use winapi::shared::minwindef::*;
use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;
use std::ptr;
use winapi::um::libloaderapi::*;
use crate::app_state::InputMode;
use crate::i18n::{t, Language, set_language};

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

// 语言菜单常量
const ID_MENU_LANG_ZH: i32 = 2009;
const ID_MENU_LANG_ZH_TW: i32 = 2010;  // 新增繁体中文
const ID_MENU_LANG_EN: i32 = 2011;
const ID_MENU_LANG_FR: i32 = 2012;
const ID_MENU_LANG_DE: i32 = 2013;
const ID_MENU_LANG_RU: i32 = 2014;
const ID_MENU_LANG_JA: i32 = 2015;
const ID_MENU_LANG_KO: i32 = 2016;     // 新增韩语

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
                u: std::mem::zeroed(),
                szInfoTitle: [0; 64],
                dwInfoFlags: 0,
                guidItem: std::mem::zeroed(),
                hBalloonIcon: ptr::null_mut(),
            };
            
            // 设置提示文本
            let tip = to_wide_string(&t("tray_tooltip"));
            for (i, &ch) in tip.iter().take(127).enumerate() {
                nid.szTip[i] = ch;
            }
            
            Shell_NotifyIconW(NIM_ADD, &mut nid);
        }
        Ok(())
    }
        
    pub fn set_input_mode_callback<F>(&self, callback: F) 
    where 
        F: Fn(InputMode) + Send + Sync + 'static,
    {
        unsafe {
            INPUT_MODE_CALLBACK = Some(Box::new(callback));
        }
    }
    
}

// 修改create_context_menu函数
unsafe fn create_context_menu() -> HMENU {
    let hmenu = CreatePopupMenu();
    
    // 创建"输入形式"子菜单
    let input_mode_submenu = CreatePopupMenu();
    
    // 定义菜单项数据 - 使用国际化文本
    let menu_items = [
        (ID_MENU_YI_ONLY, InputMode::YiOnly, t("menu_yi_only")),
        (ID_MENU_PINYIN_YI, InputMode::PinyinYi, t("menu_pinyin_yi")),
        (ID_MENU_PINYIN_WITH_YI, InputMode::PinyinWithYi, t("menu_pinyin_with_yi")),
        (ID_MENU_YI_WITH_PINYIN, InputMode::YiWithPinyin, t("menu_yi_with_pinyin")),
        (ID_MENU_HTML_RUBY, InputMode::HtmlRuby, t("menu_html_ruby")),
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
        to_wide_string(&t("menu_input_mode")).as_ptr()
    );
    
    // 添加语言选择子菜单
    let language_submenu = CreatePopupMenu();
    let languages = [
        (ID_MENU_LANG_ZH, Language::ChineseSimplified),
        (ID_MENU_LANG_ZH_TW, Language::ChineseTraditional),
        (ID_MENU_LANG_EN, Language::English),
        (ID_MENU_LANG_FR, Language::French),
        (ID_MENU_LANG_DE, Language::German),
        (ID_MENU_LANG_RU, Language::Russian),
        (ID_MENU_LANG_JA, Language::Japanese),
        (ID_MENU_LANG_KO, Language::Korean),
    ];
    
    for (id, lang) in &languages {
        AppendMenuW(
            language_submenu,
            MF_STRING,
            *id as usize,
            to_wide_string(lang.name()).as_ptr()
        );
    }
    
    AppendMenuW(
        hmenu,
        MF_STRING | MF_POPUP,
        language_submenu as usize,
        to_wide_string(&t("menu_language")).as_ptr()
    );
    
    // 添加分隔线
    AppendMenuW(hmenu, MF_SEPARATOR, 0, ptr::null());
    
    // 添加"关于"菜单项
    AppendMenuW(
        hmenu,
        MF_STRING,
        ID_MENU_ABOUT as usize,
        to_wide_string(&t("menu_about")).as_ptr()
    );
    
    // 添加"退出"菜单项
    AppendMenuW(
        hmenu,
        MF_STRING,
        ID_MENU_EXIT as usize,
        to_wide_string(&t("menu_exit")).as_ptr()
    );
    
    hmenu
}

// 处理菜单选择
unsafe fn handle_menu_command(hwnd: HWND, cmd: i32) {
    match cmd {
        ID_MENU_ABOUT => {
            MessageBoxW(
                hwnd,
                to_wide_string(&t("about_message")).as_ptr(),
                to_wide_string(&t("about_title")).as_ptr(),
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
        ID_MENU_LANG_ZH => set_language(Language::ChineseSimplified),
ID_MENU_LANG_ZH_TW => set_language(Language::ChineseTraditional),
ID_MENU_LANG_EN => set_language(Language::English),
ID_MENU_LANG_FR => set_language(Language::French),
ID_MENU_LANG_DE => set_language(Language::German),
ID_MENU_LANG_RU => set_language(Language::Russian),
ID_MENU_LANG_JA => set_language(Language::Japanese),
ID_MENU_LANG_KO => set_language(Language::Korean),
        ID_MENU_EXIT => {
            // 移除托盘图标
            let mut nid: NOTIFYICONDATAW = std::mem::zeroed();
            nid.cbSize = std::mem::size_of::<NOTIFYICONDATAW>() as u32;
            nid.hWnd = hwnd;
            nid.uID = ID_TRAY_ICON;
            Shell_NotifyIconW(NIM_DELETE, &mut nid);
            
            // 强制退出进程，确保彻底关闭
            use winapi::um::processthreadsapi::ExitProcess;
            ExitProcess(0);
        }
        _ => {}
    }
}

// 显示上下文菜单
unsafe fn show_context_menu(hwnd: HWND) {
    let hmenu = create_context_menu();
    
    // 获取鼠标位置
    let mut pt: POINT = std::mem::zeroed();
    GetCursorPos(&mut pt);
    
    // 设置前台窗口以确保菜单正确显示
    SetForegroundWindow(hwnd);
    
    // 显示上下文菜单
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
    if cmd != 0 {
        handle_menu_command(hwnd, cmd);
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
