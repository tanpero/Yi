use crate::ime::YiIME;
use std::io::{self, Write};

impl YiIME {
    /// 显示智能转换结果
    pub fn display_smart_results(&self, input: &str, results: &[(String, Vec<String>, f32)]) {
        if results.is_empty() {
            println!("无法转换输入: '{}'", input);
            return;
        }
        
        let mut all_candidates = Vec::new();
        
        for (segmentation, yi_combinations, _) in results.iter() {
            for yi_text in yi_combinations {
                all_candidates.push((yi_text.clone(), segmentation.clone()));
            }
        }
        
        // 去重
        all_candidates.sort();
        all_candidates.dedup();
        
        for (i, (yi_text, pinyin)) in all_candidates.iter().enumerate() {
            println!("{}. {} ({})", i + 1, yi_text, pinyin);
        }
    }

    /// 显示查询结果（包含部首和音节变体）
    pub fn display_results(&self, pinyin: &str, results: &[String]) {
        if results.is_empty() {
            println!("未找到拼音 '{}' 对应的彝文字符", pinyin);
        } else {
            let mut all_results = Vec::new();
            
            // 添加原始结果
            for yi_char in results {
                if yi_char.starts_with("[部首]") {
                    let radical = yi_char.strip_prefix("[部首] ").unwrap_or(yi_char);
                    all_results.push((radical.to_string(), pinyin.to_string()));
                } else {
                    all_results.push((yi_char.clone(), pinyin.to_string()));
                }
            }
            
            // 获取所有变体
            let variants = self.get_syllable_variants(pinyin);
            for variant_pinyin in variants {
                if let Some(variant_chars) = self.pinyin_index.get(&variant_pinyin) {
                    for yi_char in variant_chars {
                        all_results.push((yi_char.clone(), variant_pinyin.clone()));
                    }
                }
                // 检查部首变体
                if let Some(radical) = self.radical_pinyin_index.get(&variant_pinyin) {
                    all_results.push((radical.clone(), variant_pinyin.clone()));
                }
            }
            
            // 去重并显示
            all_results.sort();
            all_results.dedup();
            
            for (i, (yi_char, pinyin_used)) in all_results.iter().enumerate() {
                println!("{}. {} ({})", i + 1, yi_char, pinyin_used);
            }
        }
    }

    /// 获取音节的所有变体
    fn get_syllable_variants(&self, pinyin: &str) -> Vec<String> {
        let mut variants = Vec::new();
        
        // 如果音节已经以p、t、x、r结尾，不添加基础变体
        if !pinyin.ends_with('p') && !pinyin.ends_with('t') && !pinyin.ends_with('x') && !pinyin.ends_with('r') {
            // 基础变体：t、x、p、r
            for suffix in ["t", "x", "p", "r"] {
                variants.push(format!("{}{}", pinyin, suffix));
            }
        }
        
        // 特殊处理：u 和 i 的扩展变体
        if pinyin == "u" {
            // u 的扩展：uo 及其变体
            variants.extend(["uo", "uot", "uox", "uop"].iter().map(|s| s.to_string()));
        } else if pinyin == "i" {
            // i 的扩展：ie 及其变体
            variants.extend(["ie", "iet", "iex", "iep"].iter().map(|s| s.to_string()));
        } else if pinyin.ends_with('u') && pinyin.len() > 1 {
            // 复合音节以u结尾的情况（如 bu, zu 等）
            let base = &pinyin[..pinyin.len()-1];
            // 添加 ut、ux、up 变体
            for suffix in ["ut", "ux", "up"] {
                variants.push(format!("{}{}", base, suffix));
            }
            // 添加 uo 及其变体
            for uo_variant in ["uo", "uot", "uox", "uop"] {
                variants.push(format!("{}{}", base, uo_variant));
            }
        } else if pinyin.ends_with('i') && pinyin.len() > 1 {
            // 复合音节以i结尾的情况（如 bi, zi 等）
            let base = &pinyin[..pinyin.len()-1];
            // 添加 it、ix、ip 变体
            for suffix in ["it", "ix", "ip"] {
                variants.push(format!("{}{}", base, suffix));
            }
            // 添加 ie 及其变体
            for ie_variant in ["ie", "iet", "iex", "iep"] {
                variants.push(format!("{}{}", base, ie_variant));
            }
        }
        
        // 过滤出实际存在的音节
        variants.into_iter()
            .filter(|v| self.syllable_set.contains(v) || self.radical_pinyin_index.contains_key(v))
            .collect()
    }

    /// 显示声母的所有可能音节选项
    pub fn display_consonant_results(&self, consonant: &str) {
        let mut results = Vec::new();
        
        for (pinyin, yi_chars) in &self.pinyin_index {
            if pinyin.starts_with(consonant) {
                for yi_char in yi_chars {
                    results.push((yi_char.clone(), pinyin.clone()));
                }
            }
        }
        
        // 添加部首候选
        for (pinyin, radical) in &self.radical_pinyin_index {
            if pinyin.starts_with(consonant) {
                results.push((radical.clone(), pinyin.clone()));
            }
        }
        
        if results.is_empty() {
            println!("未找到以 '{}' 开头的音节", consonant);
            return;
        }
        
        results.sort();
        results.dedup();
        
        for (i, (yi_char, pinyin)) in results.iter().enumerate() {
            println!("{}. {} ({})", i + 1, yi_char, pinyin);
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
        println!("  直接输入拼音或拼音序列进行智能转换");
        println!("  输入 'help' 查看帮助");
        println!("  输入 'quit' 退出");
        println!("\n特殊说明：");
        println!("  - 单音节查询会自动包含部首候选和变体");
        println!("  - 声母输入会显示所有可能的音节选项");
        println!("  - 多音节输入会进行智能分词和转换");
        println!("  - u/i 音节会自动联想扩展变体");
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
                    } else if !input.is_empty() {
                        // 统一使用智能处理逻辑
                        self.process_input_intelligently(input);
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

    /// 智能处理输入
    fn process_input_intelligently(&self, input: &str) {
        // 1. 检查是否为完整音节（优先级最高）
        if self.syllable_set.contains(input) {
            let results = self.query_by_pinyin(input);
            self.display_results(input, &results);
        }
        // 2. 检查是否为声母或声母组合
        else if input.len() <= 3 && self.is_potential_consonant(input) {
            self.display_consonant_results(input);
        }
        // 3. 默认进行智能转换
        else {
            let results = self.smart_convert(input);
            self.display_smart_results(input, &results);
        }
    }

    /// 检查是否为潜在的声母
    fn is_potential_consonant(&self, input: &str) -> bool {
        // 检查是否有以此开头的音节
        self.pinyin_index.keys().any(|pinyin| pinyin.starts_with(input)) ||
        self.radical_pinyin_index.keys().any(|pinyin| pinyin.starts_with(input))
    }

    /// 显示帮助信息
    fn show_help(&self) {
        println!("\n=== 帮助信息 ===");
        println!("\n1. 单音节查询（包含部首和变体）：");
        println!("   输入: u");
        println!("   输出: u、ut、ux、up、uo、uot、uox、uop 的所有候选");
        println!("   输入: za");
        println!("   输出: za、zat、zax、zap、zar 的所有候选，包括部首");
        
        println!("\n2. 复合音节变体：");
        println!("   输入: bu");
        println!("   输出: bu、but、bux、bup、buo、buot、buox、buop 的所有候选");
        println!("   输入: bi");
        println!("   输出: bi、bit、bix、bip、bie、biet、biex、biep 的所有候选");
        
        println!("\n3. 声母查询：");
        println!("   输入: b");
        println!("   输出: 显示所有以'b'开头的音节及对应彝文");
        
        println!("\n4. 智能转换：");
        println!("   输入: abaka");
        println!("   输出: 智能分词并显示所有可能的彝文组合");
        println!("   输入: bapt");
        println!("   输出: 处理歧义分词（ba-pt, bap-t 等）");
        
        println!("\n5. 部首系统：");
        println!("   单音节查询会自动包含对应的部首候选");
        println!("   例如: za -> 包含普通字符和部首 ꒲");
        
        println!("\n6. 音节变体规则：");
        println!("   - 基础变体：任何音节 + t/x/p/r");
        println!("   - u音节扩展：u → ut/ux/up → uo/uot/uox/uop");
        println!("   - i音节扩展：i → it/ix/ip → ie/iet/iex/iep");
        println!("   - 复合音节：声母+u/i 也遵循相同规则");
        
        println!("\n7. 特殊字符说明：");
        println!("   p,t,x: 声调标记(前一音节) 或 声母(后一音节)");
        println!("   r: 紧喉标记(前一音节) 或 声母(后一音节)");
        println!("   y: 韵母(前一音节) 或 声母(后一音节)");
        println!("   w: 替字符号ꀕ 或 声母(后一音节)");
        println!("================\n");
    }
}