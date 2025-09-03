use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::{self, Write};
use regex::Regex;

/// 彝文输入法核心结构
struct YiIME {
    /// 字典：彝文字符 -> 拼音编码列表
    dictionary: HashMap<String, Vec<String>>,
    /// 反向索引：拼音编码 -> 彝文字符列表
    pinyin_index: HashMap<String, Vec<String>>,
    /// 所有可能的音节集合，用于分词
    syllable_set: HashSet<String>,
}

/// 分词结果
#[derive(Debug, Clone)]
struct SegmentResult {
    /// 分词方案
    segments: Vec<String>,
    /// 对应的彝文字符
    yi_chars: Vec<Vec<String>>,
    /// 置信度分数
    confidence: f32,
}

impl YiIME {
    /// 创建新的输入法实例
    fn new() -> Self {
        YiIME {
            dictionary: HashMap::new(),
            pinyin_index: HashMap::new(),
            syllable_set: HashSet::new()
        }
    }

    /// 从JSON文件加载字典
    fn load_dictionary(&mut self, file_path: &str) -> Result<(), Box<dyn std::error::Error>> {
        let content = fs::read_to_string(file_path)?;
        let json: Value = serde_json::from_str(&content)?;
        
        if let Value::Object(map) = json {
            for (yi_char, pinyin_array) in map {
                if let Value::Array(pinyins) = pinyin_array {
                    let mut pinyin_list = Vec::new();
                    
                    for pinyin in pinyins {
                        if let Value::String(p) = pinyin {
                            pinyin_list.push(p.clone());
                            
                            // 建立反向索引
                            self.pinyin_index
                                .entry(p.clone())
                                .or_insert_with(Vec::new)
                                .push(yi_char.clone());
                            
                            // 添加到音节集合
                            self.syllable_set.insert(p.clone());
                        }
                    }
                    
                    self.dictionary.insert(yi_char, pinyin_list);
                }
            }
        }
        
        println!("字典加载完成，共 {} 个彝文字符，{} 个音节", 
                self.dictionary.len(), self.syllable_set.len());
        Ok(())
    }

    /// 检查字符是否为歧义字符
    fn is_ambiguous_char(c: char) -> bool {
        matches!(c, 'p' | 't' | 'x' | 'r' | 'y' | 'w')
    }

    /// 智能分词：处理有歧义的拼音序列
    fn segment_pinyin(&self, input: &str) -> Vec<SegmentResult> {
        let mut results = Vec::new();
        
        // 动态规划分词
        let dp_results = self.dp_segment(input);
        results.extend(dp_results);
        
        // 按置信度排序
        results.sort_by(|a, b| b.confidence.partial_cmp(&a.confidence).unwrap());
        
        // 去重并限制结果数量
        self.deduplicate_results(results, 10)
    }

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
        let segment: String = chars.iter().collect();
        
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

    /// 将分词结果转换为彝文
    fn convert_to_yi(&self, segment_result: &SegmentResult) -> Vec<String> {
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
    fn smart_convert(&self, input: &str) -> Vec<(String, Vec<String>, f32)> {
        // 处理w字符替换：当且仅当输入的拼音的最后一个字符是"w"时，替换成符号ꀕ
        let processed_input = if input.ends_with('w') {
            let mut chars: Vec<char> = input.chars().collect();
            chars.pop(); // 移除最后的'w'
            let mut result = chars.into_iter().collect::<String>();
            result.push('ꀕ'); // 添加符号ꀕ
            result
        } else {
            input.to_string()
        };
        
        let segment_results = self.segment_pinyin(&processed_input);
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

    /// 显示智能转换结果
    fn display_smart_results(&self, input: &str, results: &[(String, Vec<String>, f32)]) {
        if results.is_empty() {
            println!("无法转换输入: '{}'", input);
            return;
        }
        
        println!("输入 '{}' 的转换结果：", input);
        println!("{}", "=".repeat(50));
        
        for (i, (segmentation, yi_combinations, confidence)) in results.iter().enumerate() {
            println!("\n方案 {} (置信度: {:.2}):", i + 1, confidence);
            println!("  分词: {}", segmentation);
            println!("  彝文候选:");
            
            for (j, yi_text) in yi_combinations.iter().enumerate().take(10) {
                println!("    {}. {}", j + 1, yi_text);
            }
            
            if yi_combinations.len() > 10 {
                println!("    ... 还有 {} 个候选", yi_combinations.len() - 10);
            }
        }
    }

    /// 根据拼音编码查询彝文字符
    fn query_by_pinyin(&self, pinyin: &str) -> Vec<String> {
        self.pinyin_index
            .get(pinyin)
            .cloned()
            .unwrap_or_else(Vec::new)
    }

    /// 模糊查询：查找包含指定拼音前缀的所有候选
    fn fuzzy_query(&self, prefix: &str) -> Vec<(String, Vec<String>)> {
        let mut results = Vec::new();
        
        for (pinyin, yi_chars) in &self.pinyin_index {
            if pinyin.starts_with(prefix) {
                results.push((pinyin.clone(), yi_chars.clone()));
            }
        }
        
        results.sort_by(|a, b| a.0.len().cmp(&b.0.len()));
        results
    }

    /// 显示查询结果
    fn display_results(&self, pinyin: &str, results: &[String]) {
        if results.is_empty() {
            println!("未找到拼音 '{}' 对应的彝文字符", pinyin);
        } else {
            println!("拼音 '{}' 的候选字符：", pinyin);
            for (i, yi_char) in results.iter().enumerate() {
                if let Some(all_pinyins) = self.dictionary.get(yi_char) {
                    println!("  {}. {} ({})", i + 1, yi_char, all_pinyins.join(", "));
                }
            }
        }
    }

    /// 显示模糊查询结果
    fn display_fuzzy_results(&self, prefix: &str, results: &[(String, Vec<String>)]) {
        if results.is_empty() {
            println!("未找到以 '{}' 开头的拼音", prefix);
        } else {
            println!("以 '{}' 开头的拼音候选：", prefix);
            for (i, (pinyin, yi_chars)) in results.iter().enumerate() {
                println!("  {}. {} -> {}", i + 1, pinyin, yi_chars.join(" "));
            }
        }
    }

    /// 交互式输入法界面
    fn interactive_mode(&self) {
        println!("\n=== 彝文智能输入法 ===");
        println!("输入命令：");
        println!("  直接输入拼音进行精确查询");
        println!("  输入 'fuzzy:前缀' 进行模糊查询");
        println!("  输入 'smart:拼音序列' 进行智能转换（支持词组和语句）");
        println!("  输入 'help' 查看帮助");
        println!("  输入 'quit' 退出");
        println!("\n特殊说明：");
        println!("  - p,t,x 可能是声调标记或声母");
        println!("  - r 可能是紧喉标记或声母");
        println!("  - y 可能是韵母或声母");
        println!("  - w 可能是替字符号(ꀕ)或声母");
        println!("==================\n");

        loop {
            print!("彝文输入法> ");
            io::stdout().flush().unwrap();

            let mut input = String::new();
            match io::stdin().read_line(&mut input) {
                Ok(_) => {
                    let input = input.trim();
                    
                    if input == "quit" {
                        println!("再见！");
                        break;
                    }
                    
                    if input == "help" {
                        self.show_help();
                    } else if input.starts_with("fuzzy:") {
                        let prefix = &input[6..];
                        let results = self.fuzzy_query(prefix);
                        self.display_fuzzy_results(prefix, &results);
                    } else if input.starts_with("smart:") {
                        let text = &input[6..];
                        let results = self.smart_convert(text);
                        self.display_smart_results(text, &results);
                    } else if !input.is_empty() {
                        // 尝试智能转换
                        if input.len() > 3 || input.chars().any(Self::is_ambiguous_char) {
                            let results = self.smart_convert(input);
                            self.display_smart_results(input, &results);
                        } else {
                            // 简单查询
                            let results = self.query_by_pinyin(input);
                            self.display_results(input, &results);
                        }
                    }
                }
                Err(error) => {
                    println!("输入错误: {}", error);
                    break;
                }
            }
            
            println!();
        }
    }

    /// 显示帮助信息
    fn show_help(&self) {
        println!("\n=== 帮助信息 ===");
        println!("\n1. 基本查询：");
        println!("   输入: a");
        println!("   输出: 显示所有拼音为'a'的彝文字符");
        
        println!("\n2. 模糊查询：");
        println!("   输入: fuzzy:b");
        println!("   输出: 显示所有以'b'开头的拼音及对应彝文");
        
        println!("\n3. 智能转换（推荐）：");
        println!("   输入: smart:abaka 或直接输入 abaka");
        println!("   输出: 智能分词并显示所有可能的彝文组合");
        
        println!("\n4. 歧义处理示例：");
        println!("   输入: bapt (可能是 ba-pt, bap-t, 或其他组合)");
        println!("   输入: bary (可能是 ba-ry, bar-y, 或其他组合)");
        println!("   输入: kaw (w可能是替字符号ꀕ或声母)");
        
        println!("\n5. 特殊字符说明：");
        println!("   p,t,x: 声调标记(前一音节) 或 声母(后一音节)");
        println!("   r: 紧喉标记(前一音节) 或 声母(后一音节)");
        println!("   y: 韵母(前一音节) 或 声母(后一音节)");
        println!("   w: 替字符号ꀕ 或 声母(后一音节)");
        println!("================\n");
    }
}

mod ime;
mod segmentation;
mod conversion;
mod repl;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut ime = YiIME::new();
    
    // 加载字典文件
    let dict_path = "assets/彝文音节字典.json";
    ime.load_dictionary(dict_path)?;
    
    // 演示智能转换功能
    println!("\n=== 智能转换演示 ===");
    
    let test_cases = [
        "abaka",      // 简单组合
        "bapt",       // 包含歧义的p,t
        "bary",       // 包含歧义的r,y
        "kaw",        // 包含w替字符号
        "ddabbapt",   // 复杂组合
        "mgaw",       // w的歧义处理
    ];
    
    for test_case in &test_cases {
        println!("\n测试用例: {}", test_case);
        println!("{}", "-".repeat(30));
        let results = ime.smart_convert(test_case);
        ime.display_smart_results(test_case, &results);
    }
    
    // 启动交互模式
    ime.interactive_mode();
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ambiguous_segmentation() {
        let mut ime = YiIME::new();
        
        // 添加测试数据
        ime.pinyin_index.insert("ba".to_string(), vec!["ꀠ".to_string()]);
        ime.pinyin_index.insert("p".to_string(), vec!["ꀋ".to_string()]);
        ime.pinyin_index.insert("bap".to_string(), vec!["ꀡ".to_string()]);
        ime.syllable_set.insert("ba".to_string());
        ime.syllable_set.insert("p".to_string());
        ime.syllable_set.insert("bap".to_string());
        
        let results = ime.segment_pinyin("bap");
        assert!(!results.is_empty());
        
        // 应该有多种分词方案
        let has_single = results.iter().any(|r| r.segments == vec!["bap"]);
        let has_split = results.iter().any(|r| r.segments == vec!["ba", "p"]);
        
        assert!(has_single || has_split);
    }

    #[test]
    fn test_w_replacement() {
        let mut ime = YiIME::new();
        ime.pinyin_index.insert("ka".to_string(), vec!["ꇤ".to_string()]);
        ime.syllable_set.insert("ka".to_string());
        
        let results = ime.smart_convert("kaw");
        assert!(!results.is_empty());
    }
}
