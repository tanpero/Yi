use crate::ime::{YiIME, SegmentResult};
use std::collections::HashSet;

impl YiIME {
    /// 智能分词：处理有歧义的拼音序列
    pub fn segment_pinyin(&self, input: &str) -> Vec<SegmentResult> {
        // 直接使用动态规划分词，不再处理 w 替字符号
        let mut results = self.dp_segment(input);
        
        // 按置信度排序
        results.sort_by(|a, b| b.confidence.partial_cmp(&a.confidence).unwrap());
        
        // 去重并限制结果数量
        self.deduplicate_results(results, 10)
    }

    // 删除 handle_w_replacement 函数
    
    /// 动态规划分词算法
    fn dp_segment(&self, input: &str) -> Vec<SegmentResult> {
        let chars: Vec<char> = input.chars().collect();
        let n = chars.len();
        
        if n == 0 {
            return vec![];
        }
        
        // dp[i] 存储到位置i的所有可能分词方案
        let mut dp: Vec<Vec<SegmentResult>> = vec![Vec::new(); n + 1];
        dp[0].push(SegmentResult {
            segments: Vec::new(),
            yi_chars: Vec::new(),
            confidence: 1.0,
        });
        
        for i in 1..=n {
            let mut new_results = Vec::new();
            
            for j in 0..i {
                if dp[j].is_empty() {
                    continue;
                }
                
                let segment: String = chars[j..i].iter().collect();
                
                // 检查是否为有效音节
                if let Some(yi_chars) = self.pinyin_index.get(&segment) {
                    let confidence = self.calculate_segment_confidence(&segment, i - j);
                    
                    for prev_result in &dp[j] {
                        let mut new_result = prev_result.clone();
                        new_result.segments.push(segment.clone());
                        new_result.yi_chars.push(yi_chars.clone());
                        new_result.confidence *= confidence;
                        new_results.push(new_result);
                    }
                }
                
                // 处理歧义字符的特殊情况
                if i > j + 1 {
                    let ambiguous_results = self.handle_ambiguous_segment(&chars[j..i]);
                    for (segments, yi_chars_list, confidence) in ambiguous_results {
                        for prev_result in &dp[j] {
                            let mut new_result = prev_result.clone();
                            new_result.segments.extend(segments.clone());
                            new_result.yi_chars.extend(yi_chars_list.clone());
                            new_result.confidence *= confidence;
                            new_results.push(new_result);
                        }
                    }
                }
            }
            
            dp[i] = new_results;
        }
        
        dp[n].clone()
    }

    /// 处理包含歧义字符的音节段
    fn handle_ambiguous_segment(&self, chars: &[char]) -> Vec<(Vec<String>, Vec<Vec<String>>, f32)> {
        let mut results = Vec::new();
        
        // 检查是否包含歧义字符
        let has_ambiguous = chars.iter().any(|&c| Self::is_ambiguous_char(c));
        if !has_ambiguous {
            return results;
        }
        
        // 尝试不同的分割点
        for split_pos in 1..chars.len() {
            let left: String = chars[..split_pos].iter().collect();
            let right: String = chars[split_pos..].iter().collect();
            
            // 检查分割后的两部分是否都是有效音节
            if let (Some(left_chars), Some(right_chars)) = 
                (self.pinyin_index.get(&left), self.pinyin_index.get(&right)) {
                
                let confidence = self.calculate_ambiguous_confidence(&left, &right);
                results.push((
                    vec![left, right],
                    vec![left_chars.clone(), right_chars.clone()],
                    confidence
                ));
            }
        }
        
        results
    }

    /// 计算音节的置信度
    fn calculate_segment_confidence(&self, segment: &str, length: usize) -> f32 {
        let base_confidence = match length {
            1 => 0.6,  // 单字符音节置信度较低
            2 => 0.9,  // 双字符音节置信度高
            3 => 0.8,  // 三字符音节置信度中等
            _ => 0.7,  // 更长的音节置信度较低
        };
        
        // 如果包含歧义字符，降低置信度
        let has_ambiguous = segment.chars().any(Self::is_ambiguous_char);
        if has_ambiguous {
            base_confidence * 0.8
        } else {
            base_confidence
        }
    }

    /// 计算歧义分割的置信度
    fn calculate_ambiguous_confidence(&self, left: &str, right: &str) -> f32 {
        let left_conf = self.calculate_segment_confidence(left, left.len());
        let right_conf = self.calculate_segment_confidence(right, right.len());
        (left_conf + right_conf) / 2.0 * 0.7 // 歧义分割总体置信度较低
    }

    /// 去重并限制结果数量
    fn deduplicate_results(&self, mut results: Vec<SegmentResult>, limit: usize) -> Vec<SegmentResult> {
        let mut seen = HashSet::new();
        let mut unique_results = Vec::new();
        
        for result in results {
            let key = result.segments.join("-");
            if !seen.contains(&key) && unique_results.len() < limit {
                seen.insert(key);
                unique_results.push(result);
            }
        }
        
        unique_results
    }
}