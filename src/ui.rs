use crate::ime::YiIME;
use std::io::{self, Write};

impl YiIME {
    /// 显示智能转换结果
    pub fn display_smart_results(&self, input: &str, results: &[(String, Vec<String>, f32)]) {
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

    /// 显示查询结果（包含部首）
    pub fn display_results(&self, pinyin: &str, results: &[String]) {
        if results.is_empty() {
            println!("未找到拼音 '{}' 对应的彝文字符", pinyin);
        } else {
            println!("拼音 '{}' 的候选字符：", pinyin);
            for (i, yi_char) in results.iter().enumerate() {
                if yi_char.starts_with("[部首]") {
                    // 部首候选项的特殊显示
                    let radical = yi_char.strip_prefix("[部首] ").unwrap_or(yi_char);
                    println!("  {}. {} (部首)", i + 1, radical);
                } else if let Some(all_pinyins) = self.dictionary.get(yi_char) {
                    println!("  {}. {} ({})", i + 1, yi_char, all_pinyins.join(", "));
                } else {
                    println!("  {}. {}", i + 1, yi_char);
                }
            }
        }
    }

    /// 显示模糊查询结果
    pub fn display_fuzzy_results(&self, prefix: &str, results: &[(String, Vec<String>)]) {
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
    pub fn interactive_mode(&self) {
        println!("\n=== 彝文智能输入法 ===");
        println!("输入命令：");
        println!("  直接输入拼音进行精确查询（包含部首候选）");
        println!("  输入 'fuzzy:前缀' 进行模糊查询");
        println!("  输入 'smart:拼音序列' 进行智能转换（支持词组和语句）");
        println!("  输入 'help' 查看帮助");
        println!("  输入 'quit' 退出");
        println!("\n特殊说明：");
        println!("  - 单音节查询会自动包含对应的部首候选");
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
                        // 检查是否为简单的单音节查询
                        if input.len() <= 4 && !input.chars().any(Self::is_ambiguous_char) && self.syllable_set.contains(input) {
                            // 简单查询（包含部首）
                            let results = self.query_by_pinyin(input);
                            self.display_results(input, &results);
                        } else {
                            // 智能转换
                            let results = self.smart_convert(input);
                            self.display_smart_results(input, &results);
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
        println!("\n1. 基本查询（包含部首）：");
        println!("   输入: za");
        println!("   输出: 显示所有拼音为'za'的彝文字符，包括部首 ꒲");
        
        println!("\n2. 模糊查询：");
        println!("   输入: fuzzy:b");
        println!("   输出: 显示所有以'b'开头的拼音及对应彝文");
        
        println!("\n3. 智能转换（推荐）：");
        println!("   输入: smart:abaka 或直接输入 abaka");
        println!("   输出: 智能分词并显示所有可能的彝文组合");
        
        println!("\n4. 部首系统：");
        println!("   单音节查询会自动包含对应的部首候选");
        println!("   例如: za -> 包含普通字符 ꊖ 和部首 ꒲");
        println!("   例如: li -> 包含普通字符和部首 ꒑");
        println!("   例如: ga -> 包含普通字符和部首 ꒡");
        
        println!("\n5. 歧义处理示例：");
        println!("   输入: bapt (可能是 ba-pt, bap-t, 或其他组合)");
        println!("   输入: bary (可能是 ba-ry, bar-y, 或其他组合)");
        println!("   输入: kaw (w可能是替字符号ꀕ或声母)");
        
        println!("\n6. 特殊字符说明：");
        println!("   p,t,x: 声调标记(前一音节) 或 声母(后一音节)");
        println!("   r: 紧喉标记(前一音节) 或 声母(后一音节)");
        println!("   y: 韵母(前一音节) 或 声母(后一音节)");
        println!("   w: 替字符号ꀕ 或 声母(后一音节)");
        println!("================\n");
    }
}