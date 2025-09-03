use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::fs;

/// 彝文输入法核心结构
pub struct YiIME {
    /// 字典：彝文字符 -> 拼音编码列表
    pub dictionary: HashMap<String, Vec<String>>,
    /// 反向索引：拼音编码 -> 彝文字符列表
    pub pinyin_index: HashMap<String, Vec<String>>,
    /// 所有可能的音节集合，用于分词
    pub syllable_set: HashSet<String>,
}

/// 分词结果
#[derive(Debug, Clone)]
pub struct SegmentResult {
    /// 分词方案
    pub segments: Vec<String>,
    /// 对应的彝文字符
    pub yi_chars: Vec<Vec<String>>,
    /// 置信度分数
    pub confidence: f32,
}

impl YiIME {
    /// 创建新的输入法实例
    pub fn new() -> Self {
        YiIME {
            dictionary: HashMap::new(),
            pinyin_index: HashMap::new(),
            syllable_set: HashSet::new(),
        }
    }

    /// 从JSON文件加载字典
    pub fn load_dictionary(&mut self, file_path: &str) -> Result<(), Box<dyn std::error::Error>> {
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
    pub fn is_ambiguous_char(c: char) -> bool {
        matches!(c, 'p' | 't' | 'x' | 'r' | 'y')
    }
}