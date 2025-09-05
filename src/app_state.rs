use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum InputMode {
    YiOnly,           // 彝文
    PinyinYi,         // 拼音+彝文
    PinyinWithYi,     // 拼音（彝文）
    YiWithPinyin,     // 彝文（拼音）
    HtmlRuby,         // HTML注音
}

impl Default for InputMode {
    fn default() -> Self {
        InputMode::YiOnly
    }
}

#[derive(Clone)]
pub struct AppState {
    pub is_active: Arc<Mutex<bool>>,
    pub input_buffer_empty: Arc<Mutex<bool>>,
    pub injecting_text: Arc<Mutex<bool>>,
    pub input_mode: Arc<Mutex<InputMode>>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            is_active: Arc::new(Mutex::new(false)),
            input_buffer_empty: Arc::new(Mutex::new(true)),
            injecting_text: Arc::new(Mutex::new(false)),
            input_mode: Arc::new(Mutex::new(InputMode::default())),
        }
    }
    
    pub fn set_input_buffer_empty(&self, empty: bool) {
        if let Ok(mut state) = self.input_buffer_empty.lock() {
            *state = empty;
        }
        crate::global_hook::set_input_buffer_empty(empty);
    }
    
    pub fn set_injecting_text(&self, injecting: bool) {
        if let Ok(mut state) = self.injecting_text.lock() {
            *state = injecting;
        }
        crate::global_hook::set_injecting_text(injecting);
    }
    
    pub fn set_input_mode(&self, mode: InputMode) {
        if let Ok(mut state) = self.input_mode.lock() {
            *state = mode;
        }
    }
    
    pub fn get_input_mode(&self) -> InputMode {
        if let Ok(state) = self.input_mode.lock() {
            *state
        } else {
            InputMode::default()
        }
    }
}