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
}

impl YiIME {
    /// 创建新的输入法实例
    pub fn new() -> Self {
        YiIME {
            dictionary: HashMap::new(),
            pinyin_index: HashMap::new(),
            syllable_set: HashSet::new()
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

    /// 检查字符是否为歧义字符
    pub fn is_ambiguous_char(c: char) -> bool {
        matches!(c, 'p' | 't' | 'x' | 'r' | 'y')
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