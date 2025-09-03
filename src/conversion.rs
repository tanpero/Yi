use crate::ime::{YiIME, SegmentResult};

impl YiIME {
    /// 将分词结果转换为彝文
    pub fn convert_to_yi(&self, segment_result: &SegmentResult) -> Vec<String> {
        let mut yi_combinations = vec![String::new()];
        
        for yi_chars_group in &segment_result.yi_chars {
            let mut new_combinations = Vec::new();
            
            for combination in &yi_combinations {
                for yi_char in yi_chars_group {
                    new_combinations.push(format!("{}{}", combination, yi_char));
                }
            }
            
            yi_combinations = new_combinations;
            
            // 限制组合数量以避免爆炸性增长
            if yi_combinations.len() > 50 {
                yi_combinations.truncate(50);
            }
        }
        
        yi_combinations
    }

    /// 智能转换：输入拼音序列，输出所有可能的彝文组合
    pub fn smart_convert(&self, input: &str) -> Vec<(String, Vec<String>, f32)> {
        let segment_results = self.segment_pinyin(input);
        let mut final_results = Vec::new();
        
        for result in segment_results {
            let yi_combinations = self.convert_to_yi(&result);
            let segmentation = result.segments.join("-");
            
            final_results.push((
                segmentation,
                yi_combinations,
                result.confidence
            ));
        }
        
        final_results
    }

    /// 根据拼音编码查询彝文字符
    pub fn query_by_pinyin(&self, pinyin: &str) -> Vec<String> {
        self.pinyin_index
            .get(pinyin)
            .cloned()
            .unwrap_or_else(Vec::new)
    }

    /// 模糊查询：查找包含指定拼音前缀的所有候选
    pub fn fuzzy_query(&self, prefix: &str) -> Vec<(String, Vec<String>)> {
        let mut results = Vec::new();
        
        for (pinyin, yi_chars) in &self.pinyin_index {
            if pinyin.starts_with(prefix) {
                results.push((pinyin.clone(), yi_chars.clone()));
            }
        }
        
        results.sort_by(|a, b| a.0.len().cmp(&b.0.len()));
        results
    }
}