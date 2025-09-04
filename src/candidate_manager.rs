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
    
    pub fn update_candidates(&mut self, input_buffer: &str, candidate_window: &mut CandidateWindow) {
        if input_buffer.is_empty() {
            candidate_window.hide();
            return;
        }
        
        // 始终显示输入框，即使没有候选词
        if !self.is_valid_input_sequence(input_buffer) {
            // 输入序列不合法时，显示输入框但不显示候选词
            candidate_window.show_candidates(vec![], input_buffer);
            return;
        }
        
        let mut candidates = Vec::new();
        
        // 1. 检查是否为完整音节
        let is_complete_syllable = self.yi_engine.syllable_set.contains(input_buffer);
        
        if is_complete_syllable {
            // 添加完整音节的直接匹配结果
            let results = self.yi_engine.query_by_pinyin(input_buffer);
            for yi_char in results.iter().take(3) { // 限制为前3个，为联想结果留空间
                candidates.push(format!("{} ({})", yi_char, input_buffer));
            }
        }
        
        // 2. 检查是否应该进行声母联想（包括完整音节的联想）
        if input_buffer.len() <= 3 && self.is_potential_consonant(input_buffer) {
            // 收集声母联想结果
            let consonant_results = self.get_sorted_consonant_results(input_buffer);
            
            // 如果是完整音节，跳过与输入完全相同的结果，只添加联想结果
            for (yi_char, pinyin) in consonant_results.iter() {
                if candidates.len() >= 9 {
                    break;
                }
                
                // 如果是完整音节，跳过与输入相同的拼音
                if is_complete_syllable && pinyin == input_buffer {
                    continue;
                }
                
                candidates.push(format!("{} ({})", yi_char, pinyin));
            }
        }
        
        // 3. 如果还没有足够的候选项，进行智能转换
        if candidates.len() < 9 && !is_complete_syllable {
            let results = self.yi_engine.smart_convert(input_buffer);
            for (segmentation, yi_combinations, _confidence) in results.iter() {
                for yi_text in yi_combinations.iter() {
                    if candidates.len() >= 9 {
                        break;
                    }
                    candidates.push(format!("{} ({})", yi_text, segmentation));
                }
                if candidates.len() >= 9 {
                    break;
                }
            }
        }
        
        if !candidates.is_empty() {
            candidate_window.show_candidates(candidates, input_buffer);
        } else {
            // 即使没有候选词，也要显示输入框
            candidate_window.show_candidates(vec![], input_buffer);
        }
    }
    
    /// 获取排序后的声母联想结果
    fn get_sorted_consonant_results(&self, input_buffer: &str) -> Vec<(String, String)> {
        let mut consonant_results = Vec::new();
        let mut priority_results = Vec::new(); // 优先结果：声母本身的候选项
        let mut other_results = Vec::new();    // 其他结果
        
        // 收集所有匹配的拼音和彝文字符
        for (pinyin, yi_chars) in &self.yi_engine.pinyin_index {
            if pinyin.starts_with(input_buffer) {
                for yi_char in yi_chars {
                    let result = (yi_char.clone(), pinyin.clone());
                    
                    // 判断是否为声母本身的候选项
                    if self.is_consonant_itself_candidate(input_buffer, pinyin) {
                        priority_results.push(result);
                    } else {
                        other_results.push(result);
                    }
                }
            }
        }
        
        // 添加部首候选
        for (pinyin, radical) in &self.yi_engine.radical_pinyin_index {
            if pinyin.starts_with(input_buffer) {
                let result = (radical.clone(), pinyin.clone());
                
                if self.is_consonant_itself_candidate(input_buffer, pinyin) {
                    priority_results.push(result);
                } else {
                    other_results.push(result);
                }
            }
        }
        
        // 排序并去重
        priority_results.sort();
        priority_results.dedup();
        other_results.sort();
        other_results.dedup();
        
        // 合并结果：优先结果在前，其他结果在后
        consonant_results.extend(priority_results);
        consonant_results.extend(other_results);
        
        consonant_results
    }
    
    /// 判断是否为声母本身的候选项
    /// 例如：输入 h，hat、hax、ha、hap 等是声母本身的候选项
    /// 而 hmat、hmax 等不是
    fn is_consonant_itself_candidate(&self, consonant: &str, pinyin: &str) -> bool {
        if consonant.len() == 1 {
            // 单字母声母：检查拼音是否以该声母开头且第二个字符是元音
            if let Some(second_char) = pinyin.chars().nth(1) {
                matches!(second_char, 'a' | 'e' | 'i' | 'o' | 'u')
            } else {
                false
            }
        } else {
            // 多字母声母：直接匹配
            pinyin.starts_with(consonant)
        }
    }
    
    fn is_valid_input_sequence(&self, input: &str) -> bool {
        // 特殊处理：单个 w 总是有效的
        if input == "w" {
            return true;
        }
        
        // 特殊处理：以 w 结尾的输入序列
        if input.ends_with('w') && input.len() > 1 {
            let base_input = &input[..input.len()-1];
            // 检查去掉w后的部分是否能形成有效的分词
            let segment_results = self.yi_engine.segment_pinyin(base_input);
            if !segment_results.is_empty() {
                return true;
            }
        }
        
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