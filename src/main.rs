use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::{self, Write};
use regex::Regex;
use std::error::Error;
use yi::YiIME;

fn main() -> Result<(), Box<dyn Error>> {
    let mut ime = YiIME::new();
    
    // 加载字典文件
    let dict_path = "assets/彝文音节字典.json";
    ime.load_dictionary(dict_path)?;
        
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
