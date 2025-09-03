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
        candidate_window.create_window()?; // 恢复这行
        
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
        
        // 更新全局钩子的缓冲区状态
        crate::global_hook::set_input_buffer_empty(self.input_buffer.is_empty());
        
        // 处理退格键
        if event.vk_code == VK_BACK as u32 {
            if !self.input_buffer.is_empty() {
                self.input_buffer.pop();
                println!("退格后输入缓冲区: '{}'", self.input_buffer);
                
                // 更新缓冲区状态
                crate::global_hook::set_input_buffer_empty(self.input_buffer.is_empty());
                
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
            if !self.input_buffer.is_empty() {
                let number = (event.vk_code - 0x30) as usize;
                println!("按下数字键: {}", number);
                
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
            }
            return Ok(());
        }
        
        // 处理字母键
        if event.vk_code >= 0x41 && event.vk_code <= 0x5A {
            let ch = (event.vk_code as u8 as char).to_lowercase().next().unwrap_or('\0');
            println!("转换的字符: '{}'", ch);
            
            if ch >= 'a' && ch <= 'z' {
                // 直接添加字符到输入缓冲区
                self.input_buffer.push(ch);
                println!("当前输入缓冲区: '{}'", self.input_buffer);
                
                // 更新全局钩子的缓冲区状态
                crate::global_hook::set_input_buffer_empty(false);
                
                // 检查输入序列是否合法
                if self.is_valid_input_sequence(&self.input_buffer) {
                    // 合法时更新候选项
                    self.update_candidates();
                } else {
                    // 不合法时显示输入框但不显示候选词
                    self.candidate_window.show_candidates(vec![], &self.input_buffer);
                }
            }
            return Ok(());
        }
        
        // 处理空格键
        if event.vk_code == VK_SPACE as u32 {
            if !self.input_buffer.is_empty() {
                if let Some(selected) = self.candidate_window.get_selected_candidate() {
                    println!("空格键选中第一个候选词: {}", selected);
                    self.commit_text(&selected)?;
                }
            }
            return Ok(());
        }
        
        // 处理ESC键
        if event.vk_code == VK_ESCAPE as u32 {
            println!("ESC键取消输入");
            self.input_buffer.clear();
            crate::global_hook::set_input_buffer_empty(true);
            self.candidate_window.hide();
            return Ok(());
        }
        
        // 处理特殊标点符号按键（只有在缓冲区不为空时）
        if !self.input_buffer.is_empty() {
            // 检测Shift键状态
            let shift_pressed = unsafe { GetAsyncKeyState(VK_SHIFT) & 0x8000u16 as i16 != 0 };
            
            let (punctuation_char, should_commit) = match event.vk_code {
                0xDB => { // [ 键
                    if shift_pressed {
                        ("{", true) // Shift + [ = {
                    } else {
                        ("【", true) // [ = 【
                    }
                },
                0xDD => { // ] 键
                    if shift_pressed {
                        ("}", true) // Shift + ] = }
                    } else {
                        ("】", true) // ] = 】
                    }
                },
                0xDC => { // \ 键
                    if shift_pressed {
                        ("|", true) // Shift + \ = |
                    } else {
                        ("、", true) // \ = 、
                    }
                },
                0xBA => { // ; 键
                    if shift_pressed {
                        ("：", true) // Shift + ; = :
                    } else {
                        ("；", true) // ; = ；
                    }
                },
                0xBC => { // , 键
                    if shift_pressed {
                        ("《", true) // Shift + , = <
                    } else {
                        ("，", true) // , = ，
                    }
                },
                0xBE => { // . 键
                    if shift_pressed {
                        ("》", true) // Shift + . = >
                    } else {
                        ("。", true) // . = 。
                    }
                },
                _ => ("", false)
            };
            
            if should_commit {
                // 先提交第一个候选词
                if let Some(selected) = self.candidate_window.get_selected_candidate() {
                    println!("特殊按键提交候选词: {} + 标点: {}", selected, punctuation_char);
                    
                    // 提取实际的彝文文本
                    let yi_text = if let Some(pos) = selected.find(" (") {
                        let base_text = &selected[..pos];
                        if base_text.starts_with("[部首] ") {
                            &base_text[9..]
                        } else {
                            base_text
                        }
                    } else {
                        if selected.starts_with("[部首] ") {
                            &selected[9..]
                        } else {
                            &selected
                        }
                    };
                    
                    // 组合文本：彝文 + 标点
                    let combined_text = format!("{}{}", yi_text, punctuation_char);
                    
                    // 设置正在注入文本的标志
                    crate::global_hook::set_injecting_text(true);
                    
                    // 注入组合文本
                    self.text_injector.inject_text(&combined_text)?;
                    
                    // 等待文本注入完成
                    std::thread::sleep(std::time::Duration::from_millis(50));
                    
                    // 重置注入标志
                    crate::global_hook::set_injecting_text(false);
                    
                    // 清空输入缓冲区并隐藏候选窗口
                    self.input_buffer.clear();
                    crate::global_hook::set_input_buffer_empty(true);
                    self.candidate_window.hide();
                }
                return Ok(());
            }
        }
        
        Ok(())
    }
    
    fn update_candidates(&mut self) {
        if self.input_buffer.is_empty() {
            self.candidate_window.hide();
            return;
        }
        
        // 检查输入是否合法（能形成有效的音节组合）
        if !self.is_valid_input_sequence(&self.input_buffer) {
        // 如果输入不合法，保持当前候选项不变，不更新
        return;
        }
        
        let mut candidates = Vec::new();
        
        // 1. 检查是否为完整音节（优先级最高）
        if self.yi_engine.syllable_set.contains(&self.input_buffer) {
            let results = self.yi_engine.query_by_pinyin(&self.input_buffer);
            for yi_char in results.iter().take(9) {
                candidates.push(format!("{} ({})", yi_char, self.input_buffer));
            }
        }
        // 2. 检查是否为声母或声母组合
        else if self.input_buffer.len() <= 3 && self.is_potential_consonant(&self.input_buffer) {
            // 收集声母联想结果
            let mut consonant_results = Vec::new();
            
            for (pinyin, yi_chars) in &self.yi_engine.pinyin_index {
                if pinyin.starts_with(&self.input_buffer) {
                    for yi_char in yi_chars {
                        consonant_results.push((yi_char.clone(), pinyin.clone()));
                    }
                }
            }
            
            // 添加部首候选
            for (pinyin, radical) in &self.yi_engine.radical_pinyin_index {
                if pinyin.starts_with(&self.input_buffer) {
                    consonant_results.push((radical.clone(), pinyin.clone()));
                }
            }
            
            consonant_results.sort();
            consonant_results.dedup();
            
            for (yi_char, pinyin) in consonant_results.iter().take(9) {
                candidates.push(format!("{} ({})", yi_char, pinyin));
            }
        }
        // 3. 默认进行智能转换
        else {
            let results = self.yi_engine.smart_convert(&self.input_buffer);
            for (segmentation, yi_combinations, _confidence) in results.iter().take(9) {
                for yi_text in yi_combinations.iter().take(3) {
                    if candidates.len() < 9 {
                        candidates.push(format!("{} ({})", yi_text, segmentation));
                    }
                }
            }
        }
        
        if !candidates.is_empty() {
            self.candidate_window.show_candidates(candidates, &self.input_buffer);
        } else {
            self.candidate_window.hide();
        }
    }
    
    // 添加辅助方法
    fn is_potential_consonant(&self, input: &str) -> bool {
        // 检查是否有以此开头的音节
        self.yi_engine.pinyin_index.keys().any(|pinyin| pinyin.starts_with(input)) ||
        self.yi_engine.radical_pinyin_index.keys().any(|pinyin| pinyin.starts_with(input))
    }
    
    fn commit_text(&mut self, text: &str) -> Result<(), Box<dyn std::error::Error>> {
        println!("提交文本: {}", text);
        
        // 提取实际的彝文文本（去掉括号中的拼音部分和[部首]标记）
        let yi_text = if let Some(pos) = text.find(" (") {
            let base_text = &text[..pos];
            // 如果包含[部首]标记，去除它
            if base_text.starts_with("[部首] ") {
                &base_text[9..] // 去掉"[部首] "前缀（7个字节）
            } else {
                base_text
            }
        } else {
            // 处理没有括号的情况，也可能包含[部首]标记
            if text.starts_with("[部首] ") {
                &text[9..]
            } else {
                text
            }
        };
        
        // 设置正在注入文本的标志，避免拦截 ourselves发送的按键
        crate::global_hook::set_injecting_text(true);
        
        // 使用文本注入器将文本插入到当前应用程序
        self.text_injector.inject_text(yi_text)?;
        
        // 等待一小段时间确保文本注入完成
        std::thread::sleep(std::time::Duration::from_millis(50));
        
        // 重置注入标志
        crate::global_hook::set_injecting_text(false);
        
        // 清空输入缓冲区并隐藏候选窗口
        self.input_buffer.clear();
        
        // 更新全局钩子的缓冲区状态
        crate::global_hook::set_input_buffer_empty(true);
        
        self.candidate_window.hide();
        
        Ok(())
    }
    
    // 添加新的辅助方法来验证输入序列的合法性
    fn is_valid_input_sequence(&self, input: &str) -> bool {
        // 1. 检查是否为完整音节
        if self.yi_engine.syllable_set.contains(input) {
            return true;
        }
        
        // 2. 检查是否为潜在的声母或声母组合
        if input.len() <= 3 && self.is_potential_consonant(input) {
            return true;
        }
        
        // 3. 检查是否能通过智能分词形成有效组合
        let segment_results = self.yi_engine.segment_pinyin(input);
        if !segment_results.is_empty() {
            return true;
        }
        
        // 4. 检查是否为部分有效音节（允许用户继续输入）
        // 例如：用户输入"zh"，虽然不是完整音节，但可能要输入"zha"、"zhe"等
        for syllable in &self.yi_engine.syllable_set {
            if syllable.starts_with(input) {
                return true;
            }
        }
        
        // 5. 检查部首拼音的前缀匹配
        for pinyin in self.yi_engine.radical_pinyin_index.keys() {
            if pinyin.starts_with(input) {
                return true;
            }
        }
        
        false
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("正在启动彝文输入法...");
    
    let mut ime = GlobalIME::new()?;
    ime.run()?;
    
    Ok(())
}
