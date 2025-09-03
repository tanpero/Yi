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

    /// 智能转换：输入拼音序列，输出所有可能的彝文组合
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
}