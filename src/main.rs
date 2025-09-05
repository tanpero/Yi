#![windows_subsystem = "windows"]

mod global_hook;
mod candidate_window;
mod text_injector;
mod tray_icon;
mod input_handler;
mod candidate_manager;
// 移除 text_committer 模块导入
mod app_state;
mod tsf_bridge;

use crate::global_hook::{GlobalHook, KeyEvent};
use crate::candidate_window::CandidateWindow;
use crate::text_injector::TextInjector;
use crate::tray_icon::TrayIcon;
use crate::input_handler::InputHandler;
use crate::candidate_manager::CandidateManager;
use crate::app_state::AppState;
use yi::YiIME;
use winapi::um::winuser::*;
use std::sync::mpsc::Receiver;
use std::thread;
use std::time::Duration;
use winapi::shared::windef::*;

struct GlobalIME {
    hook: GlobalHook,
    candidate_window: CandidateWindow,
    tray_icon: TrayIcon,
    input_handler: InputHandler,
    candidate_manager: CandidateManager,
    text_injector: TextInjector,  // 直接持有 TextInjector
    app_state: AppState,
    key_receiver: Receiver<KeyEvent>,
}

// 在文件顶部添加嵌入的字典数据
const YI_SYLLABLE_DICT: &str = include_str!("../assets/彝文音节字典.json");
const YI_RADICAL_DICT: &str = include_str!("../assets/彝文部首字典.json");

impl GlobalIME {
    fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let mut yi_engine = YiIME::new();
        
        // 使用嵌入的字典数据
        yi_engine.load_dictionary_from_str(YI_SYLLABLE_DICT)?;
        yi_engine.load_radical_dictionary_from_str(YI_RADICAL_DICT)?;
        
        let (mut hook, key_receiver) = GlobalHook::new();
        hook.install()?;
        
        let mut candidate_window = CandidateWindow::new()?;
        candidate_window.create_window()?;
        
        let text_injector = TextInjector::new();
        let tray_icon = TrayIcon::new()?;
        let app_state = AppState::new();
        
        // 设置输入模式回调
        tray_icon.set_input_mode_callback({
            let app_state_clone = app_state.clone();
            move |mode| {
                app_state_clone.set_input_mode(mode);
            }
        });
        
        let input_handler = InputHandler::new(yi_engine.clone().into(), app_state.clone().into());
        let candidate_manager = CandidateManager::new(yi_engine.into());
        
        // 初始化英文输入状态
        app_state.set_english_input_state(crate::app_state::EnglishInputState::Yi);
        
        Ok(GlobalIME {
            hook,
            candidate_window,
            tray_icon,
            input_handler,
            candidate_manager,
            text_injector,  // 直接使用 TextInjector
            app_state,
            key_receiver,
        })
    }
    
    fn run(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        println!("彝文输入法已启动，按F4激活/关闭输入法");
        
        // 主消息循环
        unsafe {
            let mut msg = MSG {
                hwnd: std::ptr::null_mut(),
                message: 0,
                wParam: 0,
                lParam: 0,
                time: 0,
                pt: POINT { x: 0, y: 0 },
            };
            
            loop {
                // 处理键盘事件
                while let Ok(key_event) = self.key_receiver.try_recv() {
                    self.handle_key_event(key_event)?;
                }
                
                // 处理Windows消息
                if PeekMessageW(&mut msg, std::ptr::null_mut(), 0, 0, PM_REMOVE) != 0 {
                    if msg.message == WM_QUIT {
                        break;
                    }
                    TranslateMessage(&msg);
                    DispatchMessageW(&msg);
                }
                
                thread::sleep(Duration::from_millis(10));
            }
        }
        
        Ok(())
    }
    
    fn handle_key_event(&mut self, event: KeyEvent) -> Result<(), Box<dyn std::error::Error>> {
        // 委托给输入处理器
        self.input_handler.handle_key_event(
            event, 
            &mut self.candidate_window, 
            &self.text_injector  // 直接传递 TextInjector
        )?;
        
        // 更新应用状态
        self.app_state.set_input_buffer_empty(
            self.input_handler.get_input_buffer().is_empty()
        );
        
        // 更新候选词
        self.candidate_manager.update_candidates(
            self.input_handler.get_input_buffer(),
            &mut self.candidate_window
        );
        
        Ok(())
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("正在启动彝文输入法...");
    
    let mut ime = GlobalIME::new()?;
    ime.run()?;
    
    Ok(())
}
