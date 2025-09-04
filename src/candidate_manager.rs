use yi::YiIME;
use crate::candidate_window::CandidateWindow;
use std::sync::Arc;

pub struct CandidateManager {
    yi_engine: Arc<YiIME>,
}

impl CandidateManager {
    pub fn new(yi_engine: Arc<YiIME>) -> Self {
        Self { yi_engine }
    }
    
    pub fn update_candidates(
        &self, 
        input_buffer: &str, 
        candidate_window: &mut CandidateWindow
    ) {
        if input_buffer.is_empty() {
            candidate_window.hide();
            return;
        }
        
        // 检查输入是否合法（能形成有效的音节组合）
        if !self.is_valid_input_sequence(input_buffer) {
            // 如果输入不合法，保持当前候选项不变，不更新
            return;
        }
        
        let mut candidates = Vec::new();
        
        // 1. 检查是否为完整音节（优先级最高）
        if self.yi_engine.syllable_set.contains(input_buffer) {
            let results = self.yi_engine.query_by_pinyin(input_buffer);
            for yi_char in results.iter() {
                candidates.push(format!("{} ({})", yi_char, input_buffer));
            }
        }
        // 2. 检查是否为声母或声母组合
        else if input_buffer.len() <= 3 && self.is_potential_consonant(input_buffer) {
            // 收集声母联想结果
            let mut consonant_results = Vec::new();
            
            for (pinyin, yi_chars) in &self.yi_engine.pinyin_index {
                if pinyin.starts_with(input_buffer) {
                    for yi_char in yi_chars {
                        consonant_results.push((yi_char.clone(), pinyin.clone()));
                    }
                }
            }
            
            // 添加部首候选
            for (pinyin, radical) in &self.yi_engine.radical_pinyin_index {
                if pinyin.starts_with(input_buffer) {
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
            let results = self.yi_engine.smart_convert(input_buffer);
            for (segmentation, yi_combinations, _confidence) in results.iter().take(9) {
                for yi_text in yi_combinations.iter().take(3) {
                    if candidates.len() < 9 {
                        candidates.push(format!("{} ({})", yi_text, segmentation));
                    }
                }
            }
        }
        
        if !candidates.is_empty() {
            candidate_window.show_candidates(candidates, input_buffer);
        } else {
            candidate_window.hide();
        }
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
}