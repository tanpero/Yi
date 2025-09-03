use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::error::Error;

/// 彝文输入法核心结构
pub struct YiIME {
    /// 字典：彝文字符 -> 拼音编码列表
    pub dictionary: HashMap<String, Vec<String>>,
    /// 反向索引：拼音编码 -> 彝文字符列表
    pub pinyin_index: HashMap<String, Vec<String>>,
    /// 所有可能的音节集合，用于分词
    pub syllable_set: HashSet<String>,
    /// 部首字典：部首字符 -> 拼音编码
    pub radical_dictionary: HashMap<String, String>,
    /// 部首反向索引：拼音编码 -> 部首字符
    pub radical_pinyin_index: HashMap<String, String>,
}

impl YiIME {
    /// 创建新的输入法实例
    pub fn new() -> Self {
        YiIME {
            dictionary: HashMap::new(),
            pinyin_index: HashMap::new(),
            syllable_set: HashSet::new(),
            radical_dictionary: HashMap::new(),
            radical_pinyin_index: HashMap::new(),
        }
    }

    /// 从JSON文件加载字典
    pub fn load_dictionary(&mut self, file_path: &str) -> Result<(), Box<dyn std::error::Error>> {
        let content = fs::read_to_string(file_path)?;
        let json: Value = serde_json::from_str(&content)?;
        
        if let Value::Object(map) = json {
            for (yi_char, pinyin_value) in map {
                // 修改：直接处理字符串值，而不是数组
                if let Value::String(pinyin) = pinyin_value {
                    // 将单个拼音包装成数组以保持数据结构一致性
                    let pinyin_list = vec![pinyin.clone()];
                    
                    // 建立反向索引
                    self.pinyin_index
                        .entry(pinyin.clone())
                        .or_insert_with(Vec::new)
                        .push(yi_char.clone());
                    
                    // 添加到音节集合
                    self.syllable_set.insert(pinyin.clone());
                    
                    self.dictionary.insert(yi_char, pinyin_list);
                }
            }
        }
        
        println!("字典加载完成，共 {} 个彝文字符，{} 个音节", 
                self.dictionary.len(), self.syllable_set.len());
        Ok(())
    }

    /// 从JSON文件加载部首字典
    pub fn load_radical_dictionary(&mut self, file_path: &str) -> Result<(), Box<dyn std::error::Error>> {
        let content = fs::read_to_string(file_path)?;
        let json: Value = serde_json::from_str(&content)?;
        
        if let Value::Object(map) = json {
            for (radical_char, pinyin_value) in map {
                if let Value::String(pinyin) = pinyin_value {
                    self.radical_dictionary.insert(radical_char.clone(), pinyin.clone());
                    self.radical_pinyin_index.insert(pinyin, radical_char);
                }
            }
        }
        
        println!("部首字典加载完成，共 {} 个部首", self.radical_dictionary.len());
        Ok(())
    }

    /// 检查字符是否为歧义字符
    pub fn is_ambiguous_char(c: char) -> bool {
        matches!(c, 'p' | 't' | 'x' | 'r' | 'y')
    }

    /// 根据拼音编码查询彝文字符（包含部首）
    pub fn query_by_pinyin(&self, pinyin: &str) -> Vec<String> {
        let mut results = self.pinyin_index
            .get(pinyin)
            .cloned()
            .unwrap_or_else(Vec::new);
        
        // 如果该拼音对应一个部首，添加到结果中
        if let Some(radical) = self.radical_pinyin_index.get(pinyin) {
            results.push(format!("[部首] {}", radical));
        }
        
        results
    }

    /// 检查是否应该添加部首候选项
    /// 当输入为单音节，或者分词结果只有一个候选项的一个音节时
    pub fn should_add_radical(&self, input: &str, segment_results: &[crate::segmentation::SegmentResult]) -> bool {
        // 情况1：输入为单音节
        if !input.contains(char::is_whitespace) && self.syllable_set.contains(input) {
            return true;
        }
        
        // 情况2：分词结果只有一个候选项且只有一个音节
        if segment_results.len() == 1 && segment_results[0].segments.len() == 1 {
            return true;
        }
        
        false
    }

    /// 获取部首候选项
    pub fn get_radical_candidate(&self, pinyin: &str) -> Option<String> {
        self.radical_pinyin_index.get(pinyin).cloned()
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