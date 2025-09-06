use crate::global_hook::KeyEvent;
use crate::candidate_window::CandidateWindow;
use crate::text_injector::TextInjector;
use yi::YiIME;
use winapi::um::winuser::*;
use std::sync::Arc;
use crate::app_state::{AppState, InputMode};

pub struct InputHandler {
    input_buffer: String,
    yi_engine: Arc<YiIME>,
    app_state: Arc<AppState>,
}

impl InputHandler {
    pub fn new(yi_engine: Arc<YiIME>, app_state: Arc<AppState>) -> Self {
        Self {
            input_buffer: String::new(),
            yi_engine,
            app_state,
        }
    }
    
    pub fn handle_key_event(
        &mut self, 
        event: KeyEvent,
        candidate_window: &mut CandidateWindow,
        text_injector: &TextInjector
    ) -> Result<bool, Box<dyn std::error::Error>> {
                
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
                                
                // 更新缓冲区状态
                crate::global_hook::set_input_buffer_empty(self.input_buffer.is_empty());
                
                if self.input_buffer.is_empty() {
                    candidate_window.hide();
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
                                
                let candidates_count = candidate_window.get_candidates_count();
                
                if number <= candidates_count {
                    if let Some(selected) = candidate_window.select_by_number(number) {
                                                self.commit_text(&selected, candidate_window, text_injector)?;
                        return Ok(false);
                    }
                } else {
                                    }
            }
            return Ok(false);
        }
        
        // 处理字母键
        if event.vk_code >= 0x41 && event.vk_code <= 0x5A {
            let ch = (event.vk_code as u8 as char).to_lowercase().next().unwrap_or('\0');
                        
            if ch >= 'a' && ch <= 'z' {
                // 直接添加字符到输入缓冲区
                self.input_buffer.push(ch);
                                
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
                                        self.commit_text(&selected, candidate_window, text_injector)?;
                }
            }
            return Ok(false);
        }
        
        // 处理ESC键
        if event.vk_code == VK_ESCAPE as u32 {
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
    
    fn format_text_by_mode(&self, yi_text: &str, pinyin: &str) -> String {
        let mode = self.app_state.get_input_mode();
        
        match mode {
            InputMode::YiOnly => yi_text.to_string(),
            InputMode::PinyinYi => {
                // 拼音+彝文：先输入拼音（音节间用空格代替短横线），跟随一个空格，再跟随彝文
                let formatted_pinyin = pinyin.replace("-", " ");
                format!("{} {}", formatted_pinyin, yi_text)
            },
            InputMode::PinyinWithYi => {
                // 拼音（彝文）：先输入拼音，小括号内有彝文
                let formatted_pinyin = pinyin.replace("-", " ");
                format!("{}（{}）", formatted_pinyin, yi_text)

            },
            InputMode::YiWithPinyin => {
                // 彝文（拼音）：先输入彝文，小括号内有拼音
                let formatted_pinyin = pinyin.replace("-", " ");
                format!("{}（{}）", yi_text, formatted_pinyin)

            },
            InputMode::HtmlRuby => {
                // HTML排版：每个彝文字符都用ruby标签包装
                self.format_as_html_ruby(yi_text, pinyin)
            },
        }
    }
    
    fn format_as_html_ruby(&self, yi_text: &str, pinyin: &str) -> String {
        let yi_chars: Vec<char> = yi_text.chars().collect();
        let pinyin_parts: Vec<&str> = pinyin.split('-').collect();
        
        let mut result = String::new();
        
        for (i, yi_char) in yi_chars.iter().enumerate() {
            let corresponding_pinyin = pinyin_parts.get(i).unwrap_or(&"");
            result.push_str(&format!(
                "<ruby>{}<rp>(</rp><rt>{}</rt><rp>)</rp></ruby>",
                yi_char, corresponding_pinyin
            ));
        }
        
        result
    }
    
    fn commit_text(
        &mut self, 
        text: &str,
        candidate_window: &mut CandidateWindow,
        text_injector: &TextInjector
    ) -> Result<(), Box<dyn std::error::Error>> {
                
        // 提取彝文文本和拼音
        let (yi_text, pinyin) = self.extract_yi_and_pinyin(text);
        
        // 根据输入模式格式化文本
        let formatted_text = self.format_text_by_mode(&yi_text, &pinyin);
        
        // 设置正在注入文本的标志，避免拦截 ourselves发送的按键
        crate::global_hook::set_injecting_text(true);
        
        // 使用格式化后的文本进行注入
        text_injector.inject_text(&formatted_text)?;
        
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
    
    fn extract_yi_and_pinyin(&self, text: &str) -> (String, String) {
        // 解析候选项文本，提取彝文和拼音
        if let Some(start) = text.find(" (") {
            if let Some(end) = text.rfind(')') {
                let yi_part = text[..start].trim();
                let pinyin_part = text[start + 2..end].trim();
                
                // 处理部首标记
                let clean_yi = if yi_part.starts_with("[部首] ") {
                    &yi_part[7..]
                } else {
                    yi_part
                };
                
                return (clean_yi.to_string(), pinyin_part.to_string());
            }
        }
        
        // 如果解析失败，返回原文本作为彝文，空字符串作为拼音
        (text.to_string(), String::new())
    }

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
    
    fn extract_yi_text<'a>(&self, text: &'a str) -> &'a str {
        // 提取实际的彝文文本（去掉括号中的拼音部分和[部首]标记）
        if let Some(pos) = text.find(" (") {
            let base_text = &text[..pos];
            // 如果包含[部首]标记，去除它
            if base_text.starts_with("[部首] ") {
                &base_text[9..] // 去掉"[部首] "前缀（9个字节，因为包含中文字符）
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
}