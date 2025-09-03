use crate::ime::YiIME;
use crate::segmentation::SegmentResult;

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

    /// 智能转换：输入拼音序列，输出所有可能的彝文组合（包含部首）
    pub fn smart_convert(&self, input: &str) -> Vec<(String, Vec<String>, f32)> {
        // 直接检查输入末尾是否为w，进行特殊处理
        if input.ends_with('w') {
            let base_input = &input[..input.len()-1]; // 去掉末尾的w
            
            // 对去掉w的部分进行正常分词
            let segment_results = self.segment_pinyin(base_input);
            let mut final_results = Vec::new();
            
            for result in segment_results {
                let yi_combinations = self.convert_to_yi(&result);
                
                // 为每个组合添加替字符号ꀕ
                let yi_combinations_with_w: Vec<String> = yi_combinations
                    .into_iter()
                    .map(|combo| format!("{}{}", combo, "ꀕ"))
                    .collect();
                
                let segmentation = format!("{}-w", result.segments.join("-"));
                
                final_results.push((
                    segmentation,
                    yi_combinations_with_w,
                    result.confidence
                ));
            }
            
            return final_results;
        }
        
        // 原有的正常处理逻辑
        let segment_results = self.segment_pinyin(input);
        let mut final_results = Vec::new();
        
        for result in segment_results {
            let mut yi_combinations = self.convert_to_yi(&result);
            let segmentation = result.segments.join("-");
            
            // 检查是否应该添加部首候选项
            if self.should_add_radical(input, &[result.clone()]) {
                // 如果只有一个音节，检查是否有对应的部首
                if result.segments.len() == 1 {
                    if let Some(radical) = self.get_radical_candidate(&result.segments[0]) {
                        yi_combinations.insert(0, format!("[部首] {}", radical));
                    }
                }
            }
            
            final_results.push((
                segmentation,
                yi_combinations,
                result.confidence
            ));
        }
        
        // 特殊处理：如果输入是单音节且在部首字典中
        if !input.contains(char::is_whitespace) {
            if let Some(radical) = self.get_radical_candidate(input) {
                // 如果还没有其他结果，或者需要确保部首出现在候选中
                let radical_result = (input.to_string(), vec![format!("[部首] {}", radical)], 0.9);
                
                // 检查是否已经存在部首候选
                let has_radical = final_results.iter().any(|(_, candidates, _)| {
                    candidates.iter().any(|c| c.starts_with("[部首]"))
                });
                
                if !has_radical {
                    final_results.insert(0, radical_result);
                }
            }
        }
        
        final_results
    }
}