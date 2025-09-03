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
    
    // 加载部首字典
    let radical_dict_path = "assets/彝文部首字典.json";
    ime.load_radical_dictionary(radical_dict_path)?;
    
    // 演示智能转换功能（包含部首测试）
    println!("\n=== 智能转换演示（包含部首） ===");
    
    let test_cases = [
        "abaka",      // 简单组合
        "bapt",       // 包含歧义的p,t
        "bary",       // 包含歧义的r,y
        "kaw",        // 包含w替字符号
        "ddabbapt",   // 复杂组合
        "mgaw",       // w的歧义处理
        "li",         // 单音节，应该包含部首候选
        "ga",         // 单音节，应该包含部首候选
        "yo",         // 单音节，应该包含部首候选
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
    fn test_basic_query_with_radical() {
        let mut ime = YiIME::new();
        
        // 添加测试数据
        ime.pinyin_index.insert("za".to_string(), vec!["ꊖ".to_string()]);
        ime.radical_pinyin_index.insert("za".to_string(), "꒲".to_string());
        ime.syllable_set.insert("za".to_string());
        
        let results = ime.query_by_pinyin("za");
        
        // 应该包含普通字符和部首
        assert!(results.contains(&"ꊖ".to_string()), "应该包含普通字符");
        assert!(results.iter().any(|r| r.contains("꒲")), "应该包含部首候选项");
    }

    #[test]
    fn test_radical_integration() {
        let mut ime = YiIME::new();
        
        // 添加测试数据
        ime.pinyin_index.insert("li".to_string(), vec!["ꆹ".to_string()]);
        ime.radical_pinyin_index.insert("li".to_string(), "꒑".to_string());
        ime.syllable_set.insert("li".to_string());
        
        let results = ime.smart_convert("li");
        
        // 应该包含部首候选
        let has_radical = results.iter().any(|(_, candidates, _)| {
            candidates.iter().any(|c| c.contains("꒑"))
        });
        
        assert!(has_radical, "应该包含部首候选项");
    }

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
