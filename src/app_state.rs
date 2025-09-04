use std::sync::{Arc, Mutex};

pub struct AppState {
    pub is_active: Arc<Mutex<bool>>,
    pub input_buffer_empty: Arc<Mutex<bool>>,
    pub injecting_text: Arc<Mutex<bool>>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            is_active: Arc::new(Mutex::new(false)),
            input_buffer_empty: Arc::new(Mutex::new(true)),
            injecting_text: Arc::new(Mutex::new(false)),
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
}