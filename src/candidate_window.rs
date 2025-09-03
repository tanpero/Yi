use winapi::um::winuser::*;
use winapi::um::wingdi::*;
use winapi::shared::windef::*;
use winapi::shared::minwindef::*;
use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;
use std::ptr;
use winapi::um::libloaderapi::*;
use std::sync::{Arc, Mutex};

// 添加全局候选词存储
static mut GLOBAL_CANDIDATES: Option<Arc<Mutex<Vec<String>>>> = None;

pub struct CandidateWindow {
    hwnd: HWND,
    candidates: Arc<Mutex<Vec<String>>>,
    selected_index: usize,
}

impl CandidateWindow {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let candidates = Arc::new(Mutex::new(Vec::new()));
        unsafe {
            GLOBAL_CANDIDATES = Some(candidates.clone());
        }
        
        let window = CandidateWindow {
            hwnd: ptr::null_mut(),
            candidates,
            selected_index: 0,
        };
        Ok(window)
    }
    
    // 添加缺失的 create_window 方法
    pub fn create_window(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        unsafe {
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
            
            self.hwnd = CreateWindowExW(
                WS_EX_TOPMOST | WS_EX_TOOLWINDOW,
                class_name.as_ptr(),
                to_wide_string("Yi Candidate Window").as_ptr(),
                WS_POPUP | WS_BORDER,
                CW_USEDEFAULT, CW_USEDEFAULT,
                300, 150,
                ptr::null_mut(),
                ptr::null_mut(),
                GetModuleHandleW(ptr::null()),
                ptr::null_mut()
            );
            
            if self.hwnd.is_null() {
                return Err("Failed to create candidate window".into());
            }
        }
        Ok(())
    }
    
    pub fn show_candidates(&mut self, candidates: Vec<String>, input: &str) {
        println!("显示候选词: {:?}", candidates);
        
        if let Ok(mut guard) = self.candidates.lock() {
            *guard = candidates;
        }
        self.selected_index = 0;
        
        if !self.candidates.lock().unwrap().is_empty() {
            unsafe {
                ShowWindow(self.hwnd, SW_SHOW);
                InvalidateRect(self.hwnd, ptr::null(), 1); // 强制重绘
                UpdateWindow(self.hwnd);
                
                // 获取当前光标位置
                let mut cursor_pos = POINT { x: 0, y: 0 };
                GetCursorPos(&mut cursor_pos);
                
                // 将窗口移动到光标附近
                SetWindowPos(
                    self.hwnd,
                    HWND_TOPMOST,
                    cursor_pos.x,
                    cursor_pos.y + 20,
                    300, 150,
                    SWP_SHOWWINDOW
                );
            }
        }
        
        if !self.candidates.lock().unwrap().is_empty() {
            unsafe {
                ShowWindow(self.hwnd, SW_SHOW);
                InvalidateRect(self.hwnd, ptr::null(), 1);
                UpdateWindow(self.hwnd);
                
                let mut cursor_pos = POINT { x: 0, y: 0 };
                GetCursorPos(&mut cursor_pos);
                
                // 根据候选词数量调整窗口高度
                let candidate_count = self.candidates.lock().unwrap().len();
                let window_height = 30 + candidate_count * 22; // 为更大字体调整高度
                
                SetWindowPos(
                    self.hwnd,
                    HWND_TOPMOST,
                    cursor_pos.x,
                    cursor_pos.y + 20,
                    300, window_height as i32,
                    SWP_SHOWWINDOW
                );
            }
        }
    }
    
    // 添加缺失的 select_by_number 方法
    pub fn select_by_number(&mut self, number: usize) -> Option<String> {
        if let Ok(candidates) = self.candidates.lock() {
            if number > 0 && number <= candidates.len() {
                return Some(candidates[number - 1].clone());
            }
        }
        None
    }
    
    // 添加缺失的 get_selected_candidate 方法
    pub fn get_selected_candidate(&self) -> Option<String> {
        if let Ok(candidates) = self.candidates.lock() {
            if self.selected_index < candidates.len() {
                return Some(candidates[self.selected_index].clone());
            }
        }
        None
    }
    
    // 添加缺失的 hide 方法
    pub fn hide(&self) {
        unsafe {
            ShowWindow(self.hwnd, SW_HIDE);
        }
    }
    
    // 添加获取候选词数量的方法
    pub fn get_candidates_count(&self) -> usize {
        if let Ok(candidates) = self.candidates.lock() {
            candidates.len()
        } else {
            0
        }
    }
}

unsafe extern "system" fn window_proc(
    hwnd: HWND,
    msg: UINT,
    wparam: WPARAM,
    lparam: LPARAM
) -> LRESULT {
    match msg {
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
            
            // 创建等线字体，14pt
            let font_name = to_wide_string("等线");
            let font = CreateFontW(
                -18, // 14pt ≈ 18 pixels
                0,
                0,
                0,
                FW_NORMAL,
                0,
                0,
                0,
                DEFAULT_CHARSET,
                OUT_DEFAULT_PRECIS,
                CLIP_DEFAULT_PRECIS,
                DEFAULT_QUALITY,
                DEFAULT_PITCH | FF_DONTCARE,
                font_name.as_ptr()
            );
            
            let old_font = SelectObject(hdc, font as *mut _);
            
            // 绘制实际的候选词
            if let Some(ref candidates_arc) = GLOBAL_CANDIDATES {
                if let Ok(candidates) = candidates_arc.lock() {
                    let mut y = 10;
                    for (i, candidate) in candidates.iter().enumerate().take(9) {
                        let display_text = format!("{}. {}", i + 1, candidate);
                        let text = to_wide_string(&display_text);
                        TextOutW(hdc, 10, y, text.as_ptr(), text.len() as i32 - 1);
                        y += 22; // 增加行间距以适应更大的字体
                    }
                }
            }
            
            // 恢复原字体并删除创建的字体
            SelectObject(hdc, old_font);
            DeleteObject(font as *mut _);
            
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