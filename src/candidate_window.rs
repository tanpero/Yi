use winapi::um::winuser::*;
use winapi::um::wingdi::*;
use winapi::shared::windef::*;
use winapi::shared::minwindef::*;
use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;
use std::ptr;
use winapi::um::libloaderapi::*;
use std::sync::{Arc, Mutex};
use winapi::um::dwmapi::*;
use winapi::shared::winerror::*;
use winapi::um::winreg::*;
use winapi::um::winnt::*;

const DWMWA_USE_IMMERSIVE_DARK_MODE: u32 = 20;

static mut GLOBAL_CANDIDATES: Option<Arc<Mutex<Vec<String>>>> = None;

pub struct CandidateWindow {
    hwnd: HWND,
    candidates: Arc<Mutex<Vec<String>>>,
    selected_index: usize,
    current_input: Arc<Mutex<String>>,
    is_dark_mode: bool,
}

impl CandidateWindow {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let candidates = Arc::new(Mutex::new(Vec::new()));
        let current_input = Arc::new(Mutex::new(String::new()));
        unsafe {
            GLOBAL_CANDIDATES = Some(candidates.clone());
            GLOBAL_INPUT = Some(current_input.clone());
        }
        
        // 检测系统主题
        let is_dark_mode = detect_dark_mode();
        
        // 创建窗口
        let hwnd = unsafe {
            // 注册窗口类
            let class_name = to_wide_string("YiCandidateWindow");
            let wc = WNDCLASSEXW {
                cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
                style: CS_HREDRAW | CS_VREDRAW,
                lpfnWndProc: Some(window_proc),
                cbClsExtra: 0,
                cbWndExtra: 0,
                hInstance: GetModuleHandleW(ptr::null()),
                hIcon: ptr::null_mut(),
                hCursor: LoadCursorW(ptr::null_mut(), IDC_ARROW),
                // 根据主题模式设置背景
                hbrBackground: if is_dark_mode {
                    GetStockObject(BLACK_BRUSH as i32) as HBRUSH // 深色模式用黑色背景支持毛玻璃
                } else {
                    GetStockObject(WHITE_BRUSH as i32) as HBRUSH // 浅色模式用白色背景
                },
                lpszMenuName: ptr::null(),
                lpszClassName: class_name.as_ptr(),
                hIconSm: ptr::null_mut(),
            };
            
            RegisterClassExW(&wc);
            
            // 创建窗口
            CreateWindowExW(
                WS_EX_TOPMOST | WS_EX_NOACTIVATE | WS_EX_LAYERED,
                class_name.as_ptr(),
                to_wide_string("候选词窗口").as_ptr(),
                WS_POPUP,
                0, 0, 300, 200,
                ptr::null_mut(),
                ptr::null_mut(),
                GetModuleHandleW(ptr::null()),
                ptr::null_mut()
            )
        };
        
        if hwnd.is_null() {
            return Err("创建候选词窗口失败".into());
        }
        
        // 启用毛玻璃效果
        unsafe {
            enable_blur_behind(hwnd, is_dark_mode)?;
        }
        
        let window = CandidateWindow {
            hwnd,
            candidates,
            selected_index: 0,
            current_input,
            is_dark_mode,
        };
        Ok(window)
    }
    
    pub fn show_candidates(&mut self, candidates: Vec<String>, input: &str) {
                
        if let Ok(mut guard) = self.candidates.lock() {
            *guard = candidates;
        }
        
        // 更新当前输入
        if let Ok(mut input_guard) = self.current_input.lock() {
            *input_guard = input.to_string();
        }
        
        self.selected_index = 0;
        
        // 只要有输入内容就显示窗口（不管是否有候选词）
        if !input.is_empty() {
            unsafe {
                ShowWindow(self.hwnd, SW_SHOW);
                InvalidateRect(self.hwnd, ptr::null(), 1);
                UpdateWindow(self.hwnd);
                
                let mut cursor_pos = POINT { x: 0, y: 0 };
                GetCursorPos(&mut cursor_pos);
                
                // 根据候选词数量调整窗口高度，为输入框预留空间
                let candidate_count = self.candidates.lock().unwrap().len();
                let input_box_height = 30; // 输入框高度
                let line_height = 25; // 增加行高以适应更大的彝文字符
                let bottom_margin = 15; // 底部额外空白
                let window_height = input_box_height + 10 + candidate_count * line_height + bottom_margin;
                
                SetWindowPos(
                    self.hwnd,
                    HWND_TOPMOST,
                    cursor_pos.x,
                    cursor_pos.y + 20,
                    300, window_height as i32,
                    SWP_SHOWWINDOW
                );
            }
        } else {
            // 输入为空时隐藏窗口
            self.hide();
        }
    }

    pub fn select_by_number(&mut self, number: usize) -> Option<String> {
        if let Ok(candidates) = self.candidates.lock() {
            if number > 0 && number <= candidates.len() {
                return Some(candidates[number - 1].clone());
            }
        }
        None
    }
    
    pub fn get_selected_candidate(&self) -> Option<String> {
        if let Ok(candidates) = self.candidates.lock() {
            if self.selected_index < candidates.len() {
                return Some(candidates[self.selected_index].clone());
            }
        }
        None
    }
    
    pub fn hide(&self) {
        unsafe {
            ShowWindow(self.hwnd, SW_HIDE);
        }
    }
    
    pub fn get_candidates_count(&self) -> usize {
        if let Ok(candidates) = self.candidates.lock() {
            candidates.len()
        } else {
            0
        }
    }
    
    pub fn create_window(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        unsafe {
            // 注册窗口类
            let class_name = to_wide_string("YiCandidateWindow");
            let wc = WNDCLASSEXW {
                cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
                style: CS_HREDRAW | CS_VREDRAW,
                lpfnWndProc: Some(window_proc),
                cbClsExtra: 0,
                cbWndExtra: 0,
                hInstance: GetModuleHandleW(ptr::null()),
                hIcon: ptr::null_mut(),
                hCursor: LoadCursorW(ptr::null_mut(), IDC_ARROW),
                hbrBackground: (COLOR_WINDOW + 1) as HBRUSH,
                lpszMenuName: ptr::null(),
                lpszClassName: class_name.as_ptr(),
                hIconSm: ptr::null_mut(),
            };
            
            RegisterClassExW(&wc);
            
            // 创建窗口
            self.hwnd = CreateWindowExW(
                WS_EX_TOPMOST | WS_EX_NOACTIVATE,
                class_name.as_ptr(),
                to_wide_string("候选词窗口").as_ptr(),
                WS_POPUP | WS_BORDER,
                0, 0, 300, 200,
                ptr::null_mut(),
                ptr::null_mut(),
                GetModuleHandleW(ptr::null()),
                ptr::null_mut()
            );
            
            if self.hwnd.is_null() {
                return Err("创建候选词窗口失败".into());
            }
        }
        Ok(())
    }
}

// 检测系统是否为深色模式
fn detect_dark_mode() -> bool {
    unsafe {
        let mut hkey: HKEY = ptr::null_mut();
        let subkey = to_wide_string("SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Themes\\Personalize");
        
        let result = RegOpenKeyExW(
            HKEY_CURRENT_USER,
            subkey.as_ptr(),
            0,
            KEY_READ,
            &mut hkey
        );
        
        if result != ERROR_SUCCESS as i32 {
            return false; // 默认浅色模式
        }
        
        let value_name = to_wide_string("AppsUseLightTheme");
        let mut data: DWORD = 0;
        let mut data_size = std::mem::size_of::<DWORD>() as u32;
        let mut value_type: DWORD = 0;
        
        let result = RegQueryValueExW(
            hkey,
            value_name.as_ptr(),
            ptr::null_mut(),
            &mut value_type,
            &mut data as *mut _ as *mut u8,
            &mut data_size
        );
        
        RegCloseKey(hkey);
        
        if result == ERROR_SUCCESS as i32 && value_type == REG_DWORD {
            data == 0 // 0表示深色模式，1表示浅色模式
        } else {
            false // 默认浅色模式
        }
    }
}

unsafe fn enable_blur_behind(hwnd: HWND, is_dark_mode: bool) -> Result<(), Box<dyn std::error::Error>> {
    if is_dark_mode {
        // 深色模式：保持现有的毛玻璃效果逻辑
        // 检查DWM是否可用
        let mut composition_enabled: BOOL = 0;
        let hr = DwmIsCompositionEnabled(&mut composition_enabled);
        if FAILED(hr) || composition_enabled == 0 {
            return Err("DWM组合未启用".into());
        }
        
        // 启用模糊背景效果
        let bb = DWM_BLURBEHIND {
            dwFlags: DWM_BB_ENABLE | DWM_BB_BLURREGION,
            fEnable: 1, // 启用模糊
            hRgnBlur: ptr::null_mut(), // 整个窗口模糊
            fTransitionOnMaximized: 0,
        };
        
        let hr = DwmEnableBlurBehindWindow(hwnd, &bb);
        if FAILED(hr) {
            return Err("启用毛玻璃效果失败".into());
        }
        
        // 设置窗口属性以获得更好的效果
        let attribute = DWMWA_NCRENDERING_ENABLED;
        let mut enabled: BOOL = 1;
        DwmSetWindowAttribute(
            hwnd,
            attribute,
            &mut enabled as *mut _ as *mut _,
            std::mem::size_of::<BOOL>() as u32,
        );
        
        // 深色模式设置
        let dark_mode: BOOL = 1;
        DwmSetWindowAttribute(
            hwnd,
            DWMWA_USE_IMMERSIVE_DARK_MODE,
            &dark_mode as *const _ as *const _,
            std::mem::size_of::<BOOL>() as u32,
        );
        
        SetLayeredWindowAttributes(
            hwnd,
            0, // 不使用颜色键
            230, // Alpha值：0-255，230表示约90%不透明度
            LWA_ALPHA
        );
    } else {
        // 浅色模式：不使用毛玻璃效果，设置完全不透明的白色背景
        
        // 禁用模糊背景效果
        let bb = DWM_BLURBEHIND {
            dwFlags: DWM_BB_ENABLE,
            fEnable: 0, // 禁用模糊
            hRgnBlur: ptr::null_mut(),
            fTransitionOnMaximized: 0,
        };
        
        DwmEnableBlurBehindWindow(hwnd, &bb);
        
        // 设置完全不透明
        SetLayeredWindowAttributes(
            hwnd,
            0, // 不使用颜色键
            255, // Alpha值：255表示完全不透明
            LWA_ALPHA
        );
        
        // 确保浅色模式不使用深色主题
        let dark_mode: BOOL = 0;
        DwmSetWindowAttribute(
            hwnd,
            DWMWA_USE_IMMERSIVE_DARK_MODE,
            &dark_mode as *const _ as *const _,
            std::mem::size_of::<BOOL>() as u32,
        );
    }
    
    Ok(())
}

static mut GLOBAL_INPUT: Option<Arc<Mutex<String>>> = None;
static mut GLOBAL_DARK_MODE: bool = false;

unsafe extern "system" fn window_proc(
    hwnd: HWND,
    msg: UINT,
    wparam: WPARAM,
    lparam: LPARAM
) -> LRESULT {
    match msg {
        WM_CREATE => {
            // 在窗口创建时检测并存储主题模式
            GLOBAL_DARK_MODE = detect_dark_mode();
            DefWindowProcW(hwnd, msg, wparam, lparam)
        }
        WM_ACTIVATE => {
            // 在窗口激活时重新检测主题并启用毛玻璃效果
            let is_dark = detect_dark_mode();
            GLOBAL_DARK_MODE = is_dark;
            let _ = enable_blur_behind(hwnd, is_dark);
            DefWindowProcW(hwnd, msg, wparam, lparam)
        }
        WM_SETTINGCHANGE => {
            // 监听系统设置变化，重新检测主题
            GLOBAL_DARK_MODE = detect_dark_mode();
            let _ = enable_blur_behind(hwnd, GLOBAL_DARK_MODE);
            InvalidateRect(hwnd, ptr::null(), 1); // 重绘窗口
            DefWindowProcW(hwnd, msg, wparam, lparam)
        }
        WM_PAINT => {
            let mut ps = PAINTSTRUCT {
                hdc: ptr::null_mut(),
                fErase: 0,
                rcPaint: RECT { left: 0, top: 0, right: 0, bottom: 0 },
                fRestore: 0,
                fIncUpdate: 0,
                rgbReserved: [0; 32],
            };
            let hdc = BeginPaint(hwnd, &mut ps);
            
            // 根据主题模式设置背景模式
            if GLOBAL_DARK_MODE {
                // 深色模式：设置透明背景以支持毛玻璃效果
                SetBkMode(hdc, TRANSPARENT as i32);
            } else {
                // 浅色模式：设置不透明白色背景
                SetBkMode(hdc, OPAQUE as i32);
                SetBkColor(hdc, RGB(255, 255, 255)); // 白色背景
            }
            
            // 根据主题模式选择颜色
            let (input_bg_color, text_color, border_color) = if GLOBAL_DARK_MODE {
                // 深色模式：几乎不透明的深色背景
                (RGB(5, 5, 5), RGB(255, 255, 255), RGB(30, 30, 30))
            } else {
                // 浅色模式：白色背景，深色文字
                (RGB(255, 255, 255), RGB(0, 0, 0), RGB(200, 200, 200))
            };
            
            // 创建两种字体：14pt用于普通字符，16pt用于彝文字符
            let font_name = to_wide_string("等线");
            let normal_font = CreateFontW(
                -18, // 14pt ≈ 18 pixels
                0, 0, 0,
                FW_NORMAL,
                0, 0, 0,
                DEFAULT_CHARSET,
                OUT_DEFAULT_PRECIS,
                CLIP_DEFAULT_PRECIS,
                CLEARTYPE_QUALITY,
                DEFAULT_PITCH | FF_DONTCARE,
                font_name.as_ptr()
            );
            
            let yi_font = CreateFontW(
                -21, // 16pt ≈ 21 pixels
                0, 0, 0,
                FW_NORMAL,
                0, 0, 0,
                DEFAULT_CHARSET,
                OUT_DEFAULT_PRECIS,
                CLIP_DEFAULT_PRECIS,
                CLEARTYPE_QUALITY,
                DEFAULT_PITCH | FF_DONTCARE,
                font_name.as_ptr()
            );
            
            let old_font = SelectObject(hdc, normal_font as *mut _);
            
            let mut y = 10;
            
            // 获取当前窗口宽度
            let mut rect = RECT { left: 0, top: 0, right: 0, bottom: 0 };
            GetClientRect(hwnd, &mut rect);
            let window_width = rect.right - rect.left;
            
            // 绘制输入框背景 - 使用动态宽度
            let input_rect = RECT {
                left: 5,
                top: 5,
                right: window_width - 5, // 动态右边界
                bottom: 30,
            };
            
            // 创建半透明背景画刷，与毛玻璃效果协调
            let brush = CreateSolidBrush(input_bg_color);
            
            // 使用更柔和的填充方式
            let old_brush = SelectObject(hdc, brush as *mut _);
            let pen = CreatePen(PS_SOLID as i32, 1, border_color);
            let old_pen = SelectObject(hdc, pen as *mut _);
            
            // 绘制圆角矩形输入框（可选）
            RoundRect(hdc, input_rect.left, input_rect.top, input_rect.right, input_rect.bottom, 6, 6);
            
            SelectObject(hdc, old_pen);
            SelectObject(hdc, old_brush);
            DeleteObject(pen as *mut _);
            DeleteObject(brush as *mut _);
            
            // 设置文字颜色
            SetTextColor(hdc, text_color);
            
            // 绘制当前输入的字母序列（使用普通字体）
            if let Some(ref input_arc) = GLOBAL_INPUT {
                if let Ok(input) = input_arc.lock() {
                    let input_display = format!("👉 {}", input.as_str());
                    let input_text = to_wide_string(&input_display);
                    SelectObject(hdc, normal_font as *mut _);
                    TextOutW(hdc, 10, 10, input_text.as_ptr(), input_text.len() as i32 - 1);
                }
            }
            
            y = 40; // 候选词从输入框下方开始
            
            // 绘制候选词（混合字体大小）
            if let Some(ref candidates_arc) = GLOBAL_CANDIDATES {
                if let Ok(candidates) = candidates_arc.lock() {
                    for (i, candidate) in candidates.iter().enumerate() {
                        let prefix = format!("{}. ", i + 1);
                        let mut x = 10;
                        
                        // 先绘制序号（使用普通字体）
                        SelectObject(hdc, normal_font as *mut _);
                        let prefix_text = to_wide_string(&prefix);
                        TextOutW(hdc, x, y, prefix_text.as_ptr(), prefix_text.len() as i32 - 1);
                        
                        // 计算序号的宽度
                        let mut size = SIZE { cx: 0, cy: 0 };
                        GetTextExtentPoint32W(hdc, prefix_text.as_ptr(), prefix_text.len() as i32 - 1, &mut size);
                        x += size.cx;
                        
                        // 逐字符绘制候选词内容
                        for ch in candidate.chars() {
                            let code = ch as u32;
                            // 检查是否为彝文字符（Unicode范围：U+A000-U+A48F 彝文音节, U+A490-U+A4CF 彝文部首）
                            let is_yi_char = (code >= 0xA000 && code <= 0xA48F) || (code >= 0xA490 && code <= 0xA4CF);
                            
                            // 根据字符类型选择字体
                            if is_yi_char {
                                SelectObject(hdc, yi_font as *mut _);
                            } else {
                                SelectObject(hdc, normal_font as *mut _);
                            }
                            
                            // 绘制单个字符
                            let char_str = ch.to_string();
                            let char_text = to_wide_string(&char_str);
                            TextOutW(hdc, x, y, char_text.as_ptr(), char_text.len() as i32 - 1);
                            
                            // 计算字符宽度并更新x位置
                            let mut char_size = SIZE { cx: 0, cy: 0 };
                            GetTextExtentPoint32W(hdc, char_text.as_ptr(), char_text.len() as i32 - 1, &mut char_size);
                            x += char_size.cx;
                        }
                        
                        y += 25; // 增加行间距以适应更大的彝文字符
                    }
                }
            }
            
            // 恢复原字体并删除创建的字体
            SelectObject(hdc, old_font);
            DeleteObject(normal_font as *mut _);
            DeleteObject(yi_font as *mut _);
            
            EndPaint(hwnd, &ps);
            0
        }
        WM_DESTROY => {
            0
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam)
    }
}

fn to_wide_string(s: &str) -> Vec<u16> {
    OsStr::new(s).encode_wide().chain(std::iter::once(0)).collect()
}