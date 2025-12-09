// 跨平台工具模块，处理web和desktop环境的差异

use futures::Future;
use std::time::{SystemTime, UNIX_EPOCH};
use pollster::block_on;

// 跨平台的异步任务启动器
pub fn spawn_background_task<F>(future: F)
where
    F: Future<Output = ()> + Send + 'static,
{
    #[cfg(not(target_arch = "wasm32"))]
    {
        // Desktop环境：使用线程池
        std::thread::spawn(move || {
            block_on(future);
        });
    }

    #[cfg(target_arch = "wasm32")]
    {
        // Web环境：使用wasm_bindgen_futures
        wasm_bindgen_futures::spawn_local(future);
    }
}

// 跨平台的本地任务启动器（不需要Send）
pub fn spawn_local_task<F>(future: F)
where
    F: std::future::Future<Output = ()> + Send + 'static,
{
    #[cfg(not(target_arch = "wasm32"))]
    {
        // Desktop环境：使用线程池
        std::thread::spawn(move || {
            block_on(future);
        });
    }

    #[cfg(target_arch = "wasm32")]
    {
        // Web环境：使用wasm_bindgen_futures
        wasm_bindgen_futures::spawn_local(future);
    }
}

// 跨平台的时间获取
pub fn now_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

// 跨平台的性能计时器
#[derive(Debug)]
pub struct CrossPlatformTimer {
    start_time: u64,
}

impl CrossPlatformTimer {
    pub fn new() -> Self {
        Self {
            start_time: now_millis(),
        }
    }

    pub fn elapsed_ms(&self) -> u64 {
        now_millis() - self.start_time
    }

    pub fn elapsed(&self) -> std::time::Duration {
        std::time::Duration::from_millis(self.elapsed_ms())
    }
}

// 跨平台的日志记录
pub fn log_performance(operation: &str, duration_ms: u64) {
    if duration_ms > 1000 {
        println!("⚠️  Slow operation: {} took {}ms", operation, duration_ms);
    } else if duration_ms > 100 {
        println!("⏱️  Operation: {} took {}ms", operation, duration_ms);
    }
}

// 简化的错误处理宏
#[macro_export]
macro_rules! handle_result {
    ($result:expr, $context:expr) => {
        match $result {
            Ok(value) => value,
            Err(e) => {
                debug_log!("Error in {}: {:?}", $context, e);
                return Err(e.into());
            }
        }
    };
}

// 简化的性能监控宏
#[macro_export]
macro_rules! time_operation {
    ($operation:expr, $code:block) => {{
        let timer = crate::cross_platform_utils::CrossPlatformTimer::new();
        let result = $code;
        let duration = timer.elapsed_ms();
        crate::cross_platform_utils::log_performance($operation, duration);
        result
    }};
}
