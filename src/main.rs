mod global_hook;
mod candidate_window;
mod text_injector;
mod tray_icon;

use crate::global_hook::{GlobalHook, KeyEvent};
use crate::candidate_window::CandidateWindow;
use crate::text_injector::TextInjector;
use crate::tray_icon::TrayIcon;
use yi::{YiIME};
use winapi::um::winuser::*;
use std::sync::mpsc::Receiver;
use std::thread;
use std::time::Duration;
use winapi::shared::windef::*;

struct GlobalIME {
    yi_engine: YiIME,
    hook: GlobalHook,
    candidate_window: CandidateWindow,
    text_injector: TextInjector,
    tray_icon: TrayIcon,
    input_buffer: String,
    key_receiver: Receiver<KeyEvent>,
}

impl GlobalIME {
    fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let mut yi_engine = YiIME::new();
        
        // 加载字典
        yi_engine.load_dictionary("assets/彝文音节字典.json")?;
        yi_engine.load_radical_dictionary("assets/彝文部首字典.json")?;
        
        let (mut hook, key_receiver) = GlobalHook::new();
        hook.install()?;
        
        let mut candidate_window = CandidateWindow::new()?;
        candidate_window.create_window()?;
        
        let text_injector = TextInjector::new();
        let tray_icon = TrayIcon::new()?;
        
        Ok(GlobalIME {
            yi_engine,
            hook,
            candidate_window,
            text_injector,
            tray_icon,
            input_buffer: String::new(),
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
                // 处理键盘事件 - 直接使用 self.key_receiver
                while let Ok(key_event) = self.key_receiver.try_recv() {
                    println!("收到键盘事件: {:?}", key_event); // 添加调试信息
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
        println!("处理键盘事件: vk_code={}, is_key_down={}", event.vk_code, event.is_key_down);
        
        if !event.is_key_down {
            return Ok(());
        }
        
        // 处理退格键
        if event.vk_code == VK_BACK as u32 {
            if !self.input_buffer.is_empty() {
                self.input_buffer.pop();
                println!("退格后输入缓冲区: '{}'", self.input_buffer);
                
                if self.input_buffer.is_empty() {
                    self.candidate_window.hide();
                    println!("输入缓冲区已清空，隐藏候选窗口");
                } else {
                    self.update_candidates();
                }
            }
            return Ok(());
        }
        
        // 处理数字键1-9选择候选词
        if event.vk_code >= 0x31 && event.vk_code <= 0x39 {
            let number = (event.vk_code - 0x30) as usize;
            println!("按下数字键: {}", number);
            
            // 使用新的公共方法获取候选词数量
            let candidates_count = self.candidate_window.get_candidates_count();
            
            if number <= candidates_count {
                if let Some(selected) = self.candidate_window.select_by_number(number) {
                    println!("选中候选词: {}", selected);
                    self.commit_text(&selected)?;
                    return Ok(());
                }
            } else {
                println!("数字键 {} 超出候选词数量 {}，忽略", number, candidates_count);
            }
            return Ok(());
        }
        
        // 处理字母键
        if event.vk_code >= 0x41 && event.vk_code <= 0x5A {
            let ch = (event.vk_code as u8 as char).to_lowercase().next().unwrap_or('\0');
            println!("转换的字符: '{}'", ch);
            
            if ch >= 'a' && ch <= 'z' {
                self.input_buffer.push(ch);
                println!("当前输入缓冲区: '{}'", self.input_buffer);
                self.update_candidates();
            }
            return Ok(());
        }
        
        // 处理空格键
        if event.vk_code == VK_SPACE as u32 {
            if let Some(selected) = self.candidate_window.get_selected_candidate() {
                println!("空格键选中第一个候选词: {}", selected);
                self.commit_text(&selected)?;
            }
            return Ok(());
        }
        
        // 处理ESC键
        if event.vk_code == VK_ESCAPE as u32 {
            println!("ESC键取消输入");
            self.input_buffer.clear();
            self.candidate_window.hide();
            return Ok(());
        }
        
        Ok(())
    }
    
    fn update_candidates(&mut self) {
        if self.input_buffer.is_empty() {
            self.candidate_window.hide();
            return;
        }
        
        // 使用现有的智能转换功能
        let results = self.yi_engine.smart_convert(&self.input_buffer);
        
        let mut candidates = Vec::new();
        for (segmentation, yi_combinations, _confidence) in results.iter().take(9) {
            for yi_text in yi_combinations.iter().take(3) { // 每个分词最多3个候选
                if candidates.len() < 9 {
                    candidates.push(format!("{} ({})", yi_text, segmentation));
                }
            }
        }
        
        if !candidates.is_empty() {
            self.candidate_window.show_candidates(candidates, &self.input_buffer);
        }
    }
    
    fn commit_text(&mut self, text: &str) -> Result<(), Box<dyn std::error::Error>> {
        println!("提交文本: {}", text);
        
        // 提取实际的彝文文本（去掉括号中的拼音部分）
        let yi_text = if let Some(pos) = text.find(" (") {
            &text[..pos]
        } else {
            text
        };
        
        // 设置正在注入文本的标志，避免拦截我们自己发送的按键
        crate::global_hook::set_injecting_text(true);
        
        // 使用文本注入器将文本插入到当前应用程序
        self.text_injector.inject_text(yi_text)?;
        
        // 等待一小段时间确保文本注入完成
        std::thread::sleep(std::time::Duration::from_millis(50));
        
        // 重置注入标志
        crate::global_hook::set_injecting_text(false);
        
        // 清空输入缓冲区并隐藏候选窗口
        self.input_buffer.clear();
        self.candidate_window.hide();
        
        Ok(())
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("正在启动彝文输入法...");
    
    let mut ime = GlobalIME::new()?;
    ime.run()?;
    
    Ok(())
}
