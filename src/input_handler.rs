use crate::global_hook::KeyEvent;
use crate::candidate_window::CandidateWindow;
use crate::text_injector::TextInjector;
use yi::YiIME;
use winapi::um::winuser::*;
use std::sync::Arc;

pub struct InputHandler {
    input_buffer: String,
    yi_engine: Arc<YiIME>,
}

impl InputHandler {
    pub fn new(yi_engine: Arc<YiIME>) -> Self {
        Self {
            input_buffer: String::new(),
            yi_engine,
        }
    }
    
    pub fn handle_key_event(
        &mut self, 
        event: KeyEvent,
        candidate_window: &mut CandidateWindow,
        text_injector: &TextInjector
    ) -> Result<bool, Box<dyn std::error::Error>> {
        println!("处理键盘事件: vk_code={}, is_key_down={}", event.vk_code, event.is_key_down);
        
        if !event.is_key_down {
            return Ok(false);
        }
        
        // 更新全局钩子的缓冲区状态
        crate::global_hook::set_input_buffer_empty(self.input_buffer.is_empty());
        
        // 处理退格键
        // 处理退格键
        if event.vk_code == VK_BACK as u32 {
            if !self.input_buffer.is_empty() {
                self.input_buffer.pop();
                println!("退格后输入缓冲区: '{}'", self.input_buffer);
                
                // 更新缓冲区状态
                crate::global_hook::set_input_buffer_empty(self.input_buffer.is_empty());
                
                if self.input_buffer.is_empty() {
                    candidate_window.hide();
                    println!("输入缓冲区已清空，隐藏候选窗口");
                } else {
                    // 立即更新输入框显示，无论是否有候选词
                    candidate_window.show_candidates(vec![], &self.input_buffer);
                    // 返回 true 让主循环更新候选词（基于最后一次合法音节序列）
                    return Ok(true);
                }
            }
            return Ok(false);
        }
        
        // 处理数字键1-9选择候选词
        if event.vk_code >= 0x31 && event.vk_code <= 0x39 {
            if !self.input_buffer.is_empty() {
                let number = (event.vk_code - 0x30) as usize;
                println!("按下数字键: {}", number);
                
                let candidates_count = candidate_window.get_candidates_count();
                
                if number <= candidates_count {
                    if let Some(selected) = candidate_window.select_by_number(number) {
                        println!("选中候选词: {}", selected);
                        self.commit_text(&selected, candidate_window, text_injector)?;
                        return Ok(false);
                    }
                } else {
                    println!("数字键 {} 超出候选词数量 {}，忽略", number, candidates_count);
                }
            }
            return Ok(false);
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
                    // 合法时需要更新候选项
                    return Ok(true);
                } else {
                    // 不合法时显示输入框但不显示候选词
                    candidate_window.show_candidates(vec![], &self.input_buffer);
                }
            }
            return Ok(false);
        }
        
        // 处理空格键
        if event.vk_code == VK_SPACE as u32 {
            if !self.input_buffer.is_empty() {
                if let Some(selected) = candidate_window.get_selected_candidate() {
                    println!("空格键选中第一个候选词: {}", selected);
                    self.commit_text(&selected, candidate_window, text_injector)?;
                }
            }
            return Ok(false);
        }
        
        // 处理ESC键
        if event.vk_code == VK_ESCAPE as u32 {
            println!("ESC键取消输入");
            self.input_buffer.clear();
            crate::global_hook::set_input_buffer_empty(true);
            candidate_window.hide();
            return Ok(false);
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
                if let Some(selected) = candidate_window.get_selected_candidate() {
                    println!("特殊按键提交候选词: {} + 标点: {}", selected, punctuation_char);
                    
                    // 提取实际的彝文文本
                    let yi_text = self.extract_yi_text(&selected);
                    
                    // 组合文本：彝文 + 标点
                    let combined_text = format!("{}{}", yi_text, punctuation_char);
                    
                    // 设置正在注入文本的标志
                    crate::global_hook::set_injecting_text(true);
                    
                    // 注入组合文本
                    text_injector.inject_text(&combined_text)?;
                    
                    // 等待文本注入完成
                    std::thread::sleep(std::time::Duration::from_millis(50));
                    
                    // 重置注入标志
                    crate::global_hook::set_injecting_text(false);
                    
                    // 清空输入缓冲区并隐藏候选窗口
                    self.input_buffer.clear();
                    crate::global_hook::set_input_buffer_empty(true);
                    candidate_window.hide();
                }
                return Ok(false);
            }
        }
        
        Ok(false)
    }
    
    pub fn get_input_buffer(&self) -> &str {
        &self.input_buffer
    }
    
    pub fn clear_input_buffer(&mut self) {
        self.input_buffer.clear();
        crate::global_hook::set_input_buffer_empty(true);
    }
    
    fn commit_text(
        &mut self, 
        text: &str,
        candidate_window: &mut CandidateWindow,
        text_injector: &TextInjector
    ) -> Result<(), Box<dyn std::error::Error>> {
        println!("提交文本: {}", text);
        
        // 提取实际的彝文文本（去掉括号中的拼音部分和[部首]标记）
        let yi_text = self.extract_yi_text(text);
        
        // 设置正在注入文本的标志，避免拦截 ourselves发送的按键
        crate::global_hook::set_injecting_text(true);
        
        // 使用文本注入器将文本插入到当前应用程序
        text_injector.inject_text(yi_text)?;
        
        // 等待一小段时间确保文本注入完成
        std::thread::sleep(std::time::Duration::from_millis(50));
        
        // 重置注入标志
        crate::global_hook::set_injecting_text(false);
        
        // 清空输入缓冲区并隐藏候选窗口
        self.input_buffer.clear();
        
        // 更新全局钩子的缓冲区状态
        crate::global_hook::set_input_buffer_empty(true);
        
        candidate_window.hide();
        
        Ok(())
    }
    
    fn extract_yi_text<'a>(&self, text: &'a str) -> &'a str {
        if let Some(pos) = text.find(" (") {
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
        }
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
    
    fn is_potential_consonant(&self, input: &str) -> bool {
        // 检查是否有以此开头的音节
        self.yi_engine.pinyin_index.keys().any(|pinyin| pinyin.starts_with(input)) ||
        self.yi_engine.radical_pinyin_index.keys().any(|pinyin| pinyin.starts_with(input))
    }
}