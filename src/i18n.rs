use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use serde_json::Value;
use winapi::um::winnls::*;
use std::ffi::OsString;
use std::os::windows::ffi::OsStringExt;

const LOCALE_SISO639LANGNAME: u32 = 0x00000059;
const LOCALE_SISO3166CTRYNAME: u32 = 0x0000005A;

const I18N_JSON: &str = include_str!("../assets/i18n.json");

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Language {
    ChineseSimplified,
    ChineseTraditional,
    English,
    French,
    German,
    Russian,
    Japanese,
    Korean,
}

impl Language {
    pub fn code(&self) -> &'static str {
        match self {
            Language::ChineseSimplified => "zh",
            Language::ChineseTraditional => "zh-tw",
            Language::English => "en",
            Language::French => "fr",
            Language::German => "de",
            Language::Russian => "ru",
            Language::Japanese => "ja",
            Language::Korean => "ko",
        }
    }
    
    pub fn name(&self) -> &'static str {
        match self {
            Language::ChineseSimplified => "简体中文",
            Language::ChineseTraditional => "繁體中文",
            Language::English => "English",
            Language::French => "Français",
            Language::German => "Deutsch",
            Language::Russian => "Русский",
            Language::Japanese => "日本語",
            Language::Korean => "한국어",
        }
    }
    
    fn from_locale_code(code: &str) -> Self {
        match code.to_lowercase().as_str() {
            "zh" | "zh-cn" | "zh-hans" | "zh-sg" => Language::ChineseSimplified,
            "zh-tw" | "zh-hk" | "zh-mo" | "zh-hant" => Language::ChineseTraditional,
            "ko" | "ko-kr" => Language::Korean,
            "fr" | "fr-fr" | "fr-ca" => Language::French,
            "de" | "de-de" | "de-at" | "de-ch" => Language::German,
            "ru" | "ru-ru" => Language::Russian,
            "ja" | "ja-jp" => Language::Japanese,
            _ => Language::English,
        }
    }
}

pub struct I18n {
    current_language: Arc<Mutex<Language>>,
    translations: HashMap<String, HashMap<String, String>>,
}

impl I18n {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let mut i18n = Self {
            current_language: Arc::new(Mutex::new(Language::English)),
            translations: HashMap::new(),
        };
        
        // 加载翻译数据
        i18n.load_translations()?;
        
        // 根据系统locale设置语言
        let system_locale = detect_system_language();
        let system_language = Language::from_locale_code(&system_locale);
        i18n.set_language(system_language);
        
        Ok(i18n)
    }
    
    fn load_translations(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let json_value: Value = serde_json::from_str(I18N_JSON)?;
        
        if let Value::Object(languages) = json_value {
            for (lang_code, translations) in languages {
                if let Value::Object(trans_map) = translations {
                    let mut lang_translations = HashMap::new();
                    for (key, value) in trans_map {
                        if let Value::String(text) = value {
                            lang_translations.insert(key, text);
                        }
                    }
                    self.translations.insert(lang_code, lang_translations);
                }
            }
        }
        
        Ok(())
    }
    
    pub fn set_language(&self, language: Language) {
        *self.current_language.lock().unwrap() = language;
    }
    
    pub fn get_language(&self) -> Language {
        *self.current_language.lock().unwrap()
    }
    
    pub fn t(&self, key: &str) -> String {
        let language = self.get_language();
        let lang_code = language.code();
        
        if let Some(lang_translations) = self.translations.get(lang_code) {
            if let Some(translation) = lang_translations.get(key) {
                return translation.clone();
            }
        }
        
        // 如果当前语言没有找到翻译，尝试使用英语
        if lang_code != "en" {
            if let Some(en_translations) = self.translations.get("en") {
                if let Some(translation) = en_translations.get(key) {
                    return translation.clone();
                }
            }
        }
        
        // 如果都没找到，返回key本身
        key.to_string()
    }
}

static mut I18N_INSTANCE: Option<I18n> = None;
static I18N_INIT: std::sync::Once = std::sync::Once::new();

pub fn init_i18n() -> Result<(), Box<dyn std::error::Error>> {
    I18N_INIT.call_once(|| {
        unsafe {
            match I18n::new() {
                Ok(i18n) => I18N_INSTANCE = Some(i18n),
                Err(e) => eprintln!("Failed to initialize i18n: {}", e),
            }
        }
    });
    Ok(())
}

pub fn get_i18n() -> &'static I18n {
    unsafe {
        I18N_INSTANCE.as_ref().expect("I18n not initialized")
    }
}

pub fn t(key: &str) -> String {
    get_i18n().t(key)
}

pub fn set_language(language: Language) {
    get_i18n().set_language(language);
}

fn detect_system_language() -> String {
        unsafe {
            let lcid = GetUserDefaultLCID();
            
            // 获取语言代码
            let mut buffer = [0u16; 256];
            let len = GetLocaleInfoW(
                lcid as u32,
                LOCALE_SISO639LANGNAME,
                buffer.as_mut_ptr(),
                buffer.len() as i32
            );
            
            if len > 0 {
                let lang_code = OsString::from_wide(&buffer[..len as usize - 1])
                    .to_string_lossy()
                    .to_lowercase();
                
                // 获取国家/地区代码
                let mut country_buffer = [0u16; 256];
                let country_len = GetLocaleInfoW(
                    lcid as u32,
                    LOCALE_SISO3166CTRYNAME,
                    country_buffer.as_mut_ptr(),
                    country_buffer.len() as i32
                );
                
                if country_len > 0 {
                    let country_code = OsString::from_wide(&country_buffer[..country_len as usize - 1])
                        .to_string_lossy()
                        .to_lowercase();
                    
                    return format!("{}-{}", lang_code, country_code);
                }
                
                return lang_code;
            }
            
            "en".to_string() // 默认返回英语
        }
    }