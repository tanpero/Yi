use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum InputMode {
    YiOnly,           // 彝文
    PinyinYi,         // 拼音+彝文
    PinyinWithYi,     // 拼音（彝文）
    YiWithPinyin,     // 彝文（拼音）
    HtmlRuby,         // HTML注音
}

// 添加英文输入状态枚举
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EnglishInputState {
    Yi,           // 彝文输入模式（默认）
    LowerCase,    // 英文小写输入模式
    UpperCase,    // 英文大写输入模式
}

impl Default for InputMode {
    fn default() -> Self {
        InputMode::YiOnly
    }
}

impl Default for EnglishInputState {
    fn default() -> Self {
        EnglishInputState::Yi
    }
}

#[derive(Clone)]
pub struct AppState {
    pub is_active: Arc<Mutex<bool>>,
    pub input_buffer_empty: Arc<Mutex<bool>>,
    pub injecting_text: Arc<Mutex<bool>>,
    pub input_mode: Arc<Mutex<InputMode>>,
    pub english_input_state: Arc<Mutex<EnglishInputState>>, // 新增英文输入状态
}

impl AppState {
    pub fn new() -> Self {
        Self {
            is_active: Arc::new(Mutex::new(false)),
            input_buffer_empty: Arc::new(Mutex::new(true)),
            injecting_text: Arc::new(Mutex::new(false)),
            input_mode: Arc::new(Mutex::new(InputMode::default())),
            english_input_state: Arc::new(Mutex::new(EnglishInputState::default())),
        }
    }
    
    pub fn set_input_buffer_empty(&self, empty: bool) {
        if let Ok(mut state) = self.input_buffer_empty.lock() {
            *state = empty;
        }
        crate::global_hook::set_input_buffer_empty(empty);
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
    
    pub fn set_english_input_state(&self, state: EnglishInputState) {
        if let Ok(mut current_state) = self.english_input_state.lock() {
            *current_state = state;
        }
        crate::global_hook::set_english_input_state(state);
    }
    
    pub fn get_english_input_state(&self) -> EnglishInputState {
        if let Ok(state) = self.english_input_state.lock() {
            *state
        } else {
            EnglishInputState::default()
        }
    }
}