use std::ffi::{CString};
use std::os::raw::{c_char, c_int};
use std::sync::{Arc, Mutex};

// 外部C++函数声明
extern "C" {
    fn tsf_initialize() -> c_int;
    fn tsf_insert_text(text: *const c_char) -> c_int;
    fn tsf_cleanup() -> c_int;
}

// TSF桥接结构
pub struct TSFBridge {
    initialized: bool,
}

// 全局TSF实例（确保单例）
static TSF_INSTANCE: Mutex<Option<Arc<TSFBridge>>> = Mutex::new(None);

impl TSFBridge {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        // 检查是否已经初始化
        {
            let instance = TSF_INSTANCE.lock().unwrap();
            if instance.is_some() {
                return Ok(TSFBridge { initialized: true });
            }
        }
        
        unsafe {
            let result = tsf_initialize();
            match result {
                0 => {
                    let bridge = TSFBridge { initialized: true };
                    
                    // 存储全局实例
                    let mut instance = TSF_INSTANCE.lock().unwrap();
                    *instance = Some(Arc::new(TSFBridge { initialized: true }));
                    
                                        Ok(bridge)
                },
                -1 => Err("COM初始化失败".into()),
                -2 => Err("TSF服务创建失败".into()),
                -3 => Err("TSF激活失败".into()),
                _ => Err(format!("TSF初始化失败，错误代码: {}", result).into()),
            }
        }
    }
    
    pub fn insert_text(&self, text: &str) -> Result<(), Box<dyn std::error::Error>> {
        if !self.initialized {
            return Err("TSF未初始化".into());
        }
        
        // 验证文本不为空
        if text.is_empty() {
            return Ok(());
        }
        
        // 转换为C字符串
        let c_text = CString::new(text)
            .map_err(|e| format!("文本转换失败: {}", e))?;
        
        unsafe {
            let result = tsf_insert_text(c_text.as_ptr());
            match result {
                0 => {
                                        Ok(())
                },
                -1 => Err("TSF服务未初始化或文本为空".into()),
                -2 => Err("文本编码转换失败".into()),
                -3 => Err("TSF文本插入失败".into()),
                -4 => Err("无法获取焦点上下文，请确保目标应用程序处于活动状态".into()),
                -5 => Err("内存不足".into()),
                _ => Err(format!("TSF文本插入失败，错误代码: {}", result).into()),
            }
        }
    }
    
}

impl Drop for TSFBridge {
    fn drop(&mut self) {
        if self.initialized {
            unsafe {
                let result = tsf_cleanup();
                if result == 0 {
                                    } else {
                                    }
            }
            
            // 清除全局实例
            let mut instance = TSF_INSTANCE.lock().unwrap();
            *instance = None;
        }
    }
}

// 线程安全实现
unsafe impl Send for TSFBridge {}
unsafe impl Sync for TSFBridge {}
