use crate::text_injector::TextInjector;
use crate::candidate_window::CandidateWindow;

pub struct TextCommitter {
    pub text_injector: TextInjector,
}

impl TextCommitter {
    pub fn new(text_injector: TextInjector) -> Self {
        Self { text_injector }
    }
    
    pub fn commit_text(
        &self, 
        text: &str,
        candidate_window: &mut CandidateWindow
    ) -> Result<(), Box<dyn std::error::Error>> {
        // println!("提交文本: {}", text);
        
        // 提取实际的彝文文本
        let yi_text = self.extract_yi_text(text);
        
        // 设置正在注入文本的标志
        crate::global_hook::set_injecting_text(true);
        
        // 使用文本注入器将文本插入到当前应用程序
        self.text_injector.inject_text(yi_text)?;
        
        // 等待文本注入完成
        std::thread::sleep(std::time::Duration::from_millis(50));
        
        // 重置注入标志
        crate::global_hook::set_injecting_text(false);
        
        // 隐藏候选窗口
        candidate_window.hide();
        
        Ok(())
    }
    
    pub fn commit_with_punctuation(
        &self,
        selected_text: &str,
        punctuation: &str,
        candidate_window: &mut CandidateWindow
    ) -> Result<(), Box<dyn std::error::Error>> {
        let yi_text = self.extract_yi_text(selected_text);
        let combined_text = format!("{}{}", yi_text, punctuation);
        
        crate::global_hook::set_injecting_text(true);
        self.text_injector.inject_text(&combined_text)?;
        std::thread::sleep(std::time::Duration::from_millis(50));
        crate::global_hook::set_injecting_text(false);
        
        candidate_window.hide();
        Ok(())
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