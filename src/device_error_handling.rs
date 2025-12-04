use std::fmt;

#[derive(Debug, Clone)]
pub enum DeviceError {
    ConnectionFailed(String),
    SendReportFailed(String),
    ReceiveTimeout,
    InvalidResponse(String),
    DeviceNotFound(u128),
    EncodingError(String),
    InvalidData(String),
    LockError(String),
}

impl fmt::Display for DeviceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DeviceError::ConnectionFailed(msg) => write!(f, "连接失败: {}", msg),
            DeviceError::SendReportFailed(msg) => write!(f, "发送报告失败: {}", msg),
            DeviceError::ReceiveTimeout => write!(f, "接收超时"),
            DeviceError::InvalidResponse(msg) => write!(f, "无效响应: {}", msg),
            DeviceError::DeviceNotFound(uuid) => write!(f, "设备未找到: {}", uuid),
            DeviceError::EncodingError(msg) => write!(f, "编码错误: {}", msg),
            DeviceError::InvalidData(msg) => write!(f, "无效数据: {}", msg),
            DeviceError::LockError(msg) => write!(f, "锁错误: {}", msg),
        }
    }
}

impl std::error::Error for DeviceError {}

pub type DeviceResult<T> = Result<T, DeviceError>;

// 辅助函数用于安全的字符串转换
pub fn safe_string_from_utf8(bytes: Vec<u8>) -> Result<String, DeviceError> {
    String::from_utf8(bytes)
        .map_err(|e| DeviceError::EncodingError(format!("UTF-8转换失败: {}", e)))
}

// 辅助函数用于验证数据长度
pub fn validate_data_length(data: &[u8], expected: usize, name: &str) -> Result<(), DeviceError> {
    if data.len() != expected {
        return Err(DeviceError::InvalidData(format!(
            "{} 数据长度错误: 期望 {}, 实际 {}",
            name,
            expected,
            data.len()
        )));
    }
    Ok(())
}
