use encoding_rs::{GB18030, UTF_16LE};
use std::sync::{Arc, Mutex};

// 添加错误类型定义
#[derive(Debug, Clone)]
pub enum ByteConverterError {
    InvalidEncoding(u8),
    IndexOutOfBounds {
        index: usize,
        len: usize,
        total: usize,
    },
    InvalidUtf8,
    InsufficientSpace,
}

impl std::fmt::Display for ByteConverterError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ByteConverterError::InvalidEncoding(val) => {
                write!(f, "Invalid encoding value: {}", val)
            }
            ByteConverterError::IndexOutOfBounds { index, len, total } => {
                write!(f, "Index out of bounds: {} + {} > {}", index, len, total)
            }
            ByteConverterError::InvalidUtf8 => write!(f, "Invalid UTF-8 sequence"),
            ByteConverterError::InsufficientSpace => write!(f, "Insufficient space for operation"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Encoding {
    GB18030 = 0x02,
    UTF16LE = 0x03,
    ASCII = 0x04,
}

impl TryFrom<u8> for Encoding {
    type Error = ByteConverterError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0x02 => Ok(Encoding::GB18030),
            0x03 => Ok(Encoding::UTF16LE),
            0x04 => Ok(Encoding::ASCII),
            _ => Err(ByteConverterError::InvalidEncoding(value)),
        }
    }
}

impl From<Encoding> for u8 {
    fn from(encoding: Encoding) -> Self {
        match encoding {
            Encoding::GB18030 => 0x02,
            Encoding::UTF16LE => 0x03,
            Encoding::ASCII => 0x04,
        }
    }
}

#[derive(Debug, Clone)]
pub struct RwBytes {
    bytes: Arc<Mutex<Vec<u8>>>,
    offset: usize,
    len: usize,
}

// 改进 Display 实现，避免在格式化时使用 block_on
impl std::fmt::Display for RwBytes {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "RwBytes {{ offset: {}, len: {} }}",
            self.offset, self.len
        )
    }
}

impl RwBytes {
    fn lock_bytes(&self) -> std::sync::MutexGuard<'_, Vec<u8>> {
        self.bytes.lock().expect("bytes lock poisoned")
    }

    pub fn deep_clone(&self) -> Self {
        RwBytes {
            bytes: Arc::new(Mutex::new(self.lock_bytes().clone())),
            offset: self.offset,
            len: self.len,
        }
    }

    pub fn new(bytes: Vec<u8>) -> Self {
        let len = bytes.len();
        RwBytes {
            bytes: Arc::new(Mutex::new(bytes)),
            offset: 0,
            len,
        }
    }

    pub fn from_str(encoding: Encoding, value: &str) -> Self {
        let bytes = Self::encode_string(encoding, value);
        let len = bytes.len();
        println!("from_str: {:02X?}", bytes);
        RwBytes {
            bytes: Arc::new(Mutex::new(bytes)),
            offset: 0,
            len,
        }
    }

    // 辅助方法：字符串编码
    fn encode_string(encoding: Encoding, value: &str) -> Vec<u8> {
        let mut bytes = match encoding {
            Encoding::ASCII => value.as_bytes().to_vec(),
            Encoding::GB18030 => GB18030.encode(value).0.to_vec(),
            Encoding::UTF16LE => {
                let mut result = Vec::with_capacity(value.len() * 2 + 2);
                for ch in value.encode_utf16() {
                    result.extend_from_slice(&ch.to_le_bytes());
                }
                result
            }
        };

        // 添加终止符
        match encoding {
            Encoding::UTF16LE => bytes.extend_from_slice(&[0, 0]),
            _ => bytes.push(0),
        }

        bytes
    }

    pub fn ref_at(&self, index: usize, len: usize) -> Option<RwBytes> {
        let offset = self.offset + index;
        let data = self.lock_bytes();

        if offset + len > data.len() {
            println!(
                "Index out of bounds for bytes: {} + {} > {}",
                offset,
                len,
                data.len()
            );
            return None;
        }

        Some(RwBytes {
            bytes: self.bytes.clone(),
            offset,
            len,
        })
    }

    pub fn into_vec(self) -> Vec<u8> {
        let bytes = self.lock_bytes();
        if self.offset + self.len > bytes.len() {
            // 使用 Result 类型会更好，但为了保持兼容性，这里仍使用 panic
            panic!(
                "Index out of bounds for bytes: {} + {} > {}",
                self.offset,
                self.len,
                bytes.len()
            );
        }
        bytes[self.offset..self.offset + self.len].to_vec()
    }

    pub fn len(&self) -> usize {
        self.len
    }

    // 添加只读方法

    pub fn read_u8(&self, index: usize) -> Option<u8> {
        let data = self.lock_bytes();
        let actual_index = self.offset + index;
        if actual_index >= data.len() {
            return None;
        }
        Some(data[actual_index])
    }

    pub fn u8(&self, index: usize, value: Option<u8>) -> Option<u8> {
        let mut data = self.lock_bytes();
        let actual_index = self.offset + index;
        if actual_index >= data.len() {
            return None;
        }
        if let Some(value) = value {
            data[actual_index] = value;
        }
        Some(data[actual_index])
    }

    pub fn read_u16(&self, index: usize) -> Option<u16> {
        let data = self.lock_bytes();
        let actual_index = self.offset + index;
        if actual_index + 1 >= data.len() {
            return None;
        }
        Some(u16::from_le_bytes([
            data[actual_index],
            data[actual_index + 1],
        ]))
    }

    pub fn u16(&self, index: usize, value: Option<u16>) -> Option<u16> {
        let mut data = self.lock_bytes();
        let actual_index = self.offset + index;
        if actual_index + 1 >= data.len() {
            return None;
        }
        if let Some(value) = value {
            let bytes = value.to_le_bytes();
            data[actual_index..actual_index + 2].copy_from_slice(&bytes);
        }
        Some(u16::from_le_bytes([
            data[actual_index],
            data[actual_index + 1],
        ]))
    }

    pub fn read_i16(&self, index: usize) -> Option<i16> {
        let data = self.lock_bytes();
        let actual_index = self.offset + index;
        if actual_index + 1 >= data.len() {
            return None;
        }
        Some(i16::from_le_bytes([
            data[actual_index],
            data[actual_index + 1],
        ]))
    }

    pub fn i16(&self, index: usize, value: Option<i16>) -> Option<i16> {
        let mut data = self.lock_bytes();
        let actual_index = self.offset + index;
        if actual_index + 1 >= data.len() {
            return None;
        }
        if let Some(value) = value {
            let bytes = value.to_le_bytes();
            data[actual_index..actual_index + 2].copy_from_slice(&bytes);
        }
        Some(i16::from_le_bytes([
            data[actual_index],
            data[actual_index + 1],
        ]))
    }

    pub fn read_u32(&self, index: usize) -> Option<u32> {
        let data = self.lock_bytes();
        let actual_index = self.offset + index;
        if actual_index + 3 >= data.len() {
            return None;
        }
        Some(u32::from_le_bytes([
            data[actual_index],
            data[actual_index + 1],
            data[actual_index + 2],
            data[actual_index + 3],
        ]))
    }

    pub fn u32(&self, index: usize, value: Option<u32>) -> Option<u32> {
        let mut data = self.lock_bytes();
        let actual_index = self.offset + index;
        if actual_index + 3 >= data.len() {
            return None;
        }
        if let Some(value) = value {
            let bytes = value.to_le_bytes();
            data[actual_index..actual_index + 4].copy_from_slice(&bytes);
        }
        Some(u32::from_le_bytes([
            data[actual_index],
            data[actual_index + 1],
            data[actual_index + 2],
            data[actual_index + 3],
        ]))
    }

    pub fn vec(&self, index: usize, len: Option<usize>, value: Option<Vec<u8>>) -> Option<Vec<u8>> {
        let mut data = self.lock_bytes();
        let actual_index = self.offset + index;

        if let Some(value) = value {
            // 写操作
            let write_len = len.unwrap_or(value.len());
            if actual_index + write_len > data.len() || write_len > value.len() {
                return None;
            }
            data[actual_index..actual_index + write_len].copy_from_slice(&value[..write_len]);
            Some(value)
        } else {
            // 读操作
            let read_len = len.unwrap_or(data.len().saturating_sub(actual_index));
            if actual_index + read_len > data.len() {
                return None;
            }
            Some(data[actual_index..actual_index + read_len].to_vec())
        }
    }

    pub fn str(&self, encoding: u8, index: usize, value: Option<String>) -> Option<String> {
        let encoding = match Encoding::try_from(encoding) {
            Ok(enc) => enc,
            Err(_) => return None,
        };

        let mut data = self.lock_bytes();
        let actual_index = self.offset + index;

        if let Some(value) = value {
            // 写操作
            let encoded_bytes = Self::encode_string(encoding, &value);
            if actual_index + encoded_bytes.len() > data.len() {
                return None;
            }
            data[actual_index..actual_index + encoded_bytes.len()].copy_from_slice(&encoded_bytes);
            Some(value)
        } else {
            // 读操作
            let end_index = self.find_string_end(&data, actual_index, encoding);
            if end_index <= actual_index || end_index > data.len() {
                return None;
            }

            let bytes = &data[actual_index..end_index];
            self.decode_string(encoding, bytes)
        }
    }

    // 辅助方法：查找字符串结束位置
    fn find_string_end(&self, data: &[u8], start: usize, encoding: Encoding) -> usize {
        match encoding {
            Encoding::ASCII | Encoding::GB18030 => {
                let mut i = start;
                while i < data.len() && data[i] != 0 {
                    i += 1;
                }
                i
            }
            Encoding::UTF16LE => {
                let mut i = start;
                while i + 1 < data.len() && (data[i] != 0 || data[i + 1] != 0) {
                    i += 2;
                }
                i
            }
        }
    }

    // 辅助方法：字符串解码
    fn decode_string(&self, encoding: Encoding, bytes: &[u8]) -> Option<String> {
        match encoding {
            Encoding::ASCII => String::from_utf8(bytes.to_vec()).ok(),
            Encoding::GB18030 => Some(GB18030.decode(bytes).0.to_string()),
            Encoding::UTF16LE => Some(UTF_16LE.decode(bytes).0.to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encoding_conversion() {
        // 测试有效的编码转换
        assert_eq!(Encoding::try_from(0x02).unwrap(), Encoding::GB18030);
        assert_eq!(Encoding::try_from(0x03).unwrap(), Encoding::UTF16LE);
        assert_eq!(Encoding::try_from(0x04).unwrap(), Encoding::ASCII);

        // 测试无效的编码转换
        assert!(Encoding::try_from(0x01).is_err());
        assert!(Encoding::try_from(0xFF).is_err());
    }

    #[test]
    fn test_rwbytes_basic_operations() {
        let data = vec![1, 2, 3, 4, 5];
        let rw_bytes = RwBytes::new(data);

        // 测试长度
        assert_eq!(rw_bytes.len(), 5);

        // 测试读取 u8
        assert_eq!(rw_bytes.read_u8(0), Some(1));
        assert_eq!(rw_bytes.read_u8(4), Some(5));
        assert_eq!(rw_bytes.read_u8(5), None); // 越界

        // 测试写入 u8
        assert_eq!(rw_bytes.u8(0, Some(10)), Some(10));
        assert_eq!(rw_bytes.read_u8(0), Some(10));
    }

    #[test]
    fn test_rwbytes_u16_operations() {
        let data = vec![0x01, 0x02, 0x03, 0x04];
        let rw_bytes = RwBytes::new(data);

        // 测试读取 u16 (小端序)
        assert_eq!(rw_bytes.read_u16(0), Some(0x0201)); // 0x01, 0x02 -> 0x0201
        assert_eq!(rw_bytes.read_u16(2), Some(0x0403)); // 0x03, 0x04 -> 0x0403
        assert_eq!(rw_bytes.read_u16(3), None); // 越界

        // 测试写入 u16
        assert_eq!(rw_bytes.u16(0, Some(0x1234)), Some(0x1234));
        assert_eq!(rw_bytes.read_u16(0), Some(0x1234));
    }

    #[test]
    fn test_rwbytes_string_operations() {
        let rw_bytes = RwBytes::new(vec![0; 100]);

        // 测试 ASCII 字符串
        let test_str = "Hello";
        assert!(
            rw_bytes
                .str(Encoding::ASCII as u8, 0, Some(test_str.to_string()))
                .is_some()
        );
        assert_eq!(
            rw_bytes.str(Encoding::ASCII as u8, 0, None),
            Some(test_str.to_string())
        );

        // 测试 UTF16LE 字符串
        let test_str_utf16 = "测试";
        assert!(
            rw_bytes
                .str(
                    Encoding::UTF16LE as u8,
                    20,
                    Some(test_str_utf16.to_string())
                )
                .is_some()
        );
        assert_eq!(
            rw_bytes.str(Encoding::UTF16LE as u8, 20, None),
            Some(test_str_utf16.to_string())
        );
    }

    #[test]
    fn test_rwbytes_ref_at() {
        let data = vec![1, 2, 3, 4, 5, 6, 7, 8];
        let rw_bytes = RwBytes::new(data);

        // 测试正常的引用
        let sub_bytes = rw_bytes.ref_at(2, 3).unwrap();
        assert_eq!(sub_bytes.len(), 3);
        assert_eq!(sub_bytes.read_u8(0), Some(3)); // 原始数据的索引 2

        // 测试越界
        assert!(rw_bytes.ref_at(6, 5).is_none()); // 6 + 5 > 8
    }

    #[test]
    fn test_from_str_encoding() {
        // 测试 ASCII 编码
        let ascii_bytes = RwBytes::from_str(Encoding::ASCII, "Hello");
        let expected_ascii = vec![b'H', b'e', b'l', b'l', b'o', 0];
        assert_eq!(ascii_bytes.into_vec(), expected_ascii);

        // 测试 UTF16LE 编码
        let utf16_bytes = RwBytes::from_str(Encoding::UTF16LE, "A");
        let expected_utf16 = vec![0x41, 0x00, 0x00, 0x00]; // 'A' in UTF16LE + null terminator
        assert_eq!(utf16_bytes.into_vec(), expected_utf16);
    }

    #[test]
    fn test_error_handling() {
        let rw_bytes = RwBytes::new(vec![1, 2, 3]);

        // 测试无效编码
        assert!(rw_bytes.str(0xFF, 0, Some("test".to_string())).is_none());

        // 测试越界访问
        assert!(rw_bytes.u8(10, Some(1)).is_none());
        assert!(rw_bytes.read_u16(2).is_none()); // 需要 2 个字节，但只有 1 个可用
    }

    #[test]
    fn test_display_implementation() {
        let rw_bytes = RwBytes::new(vec![1, 2, 3, 4, 5]);
        let display_str = format!("{}", rw_bytes);
        assert!(display_str.contains("offset: 0"));
        assert!(display_str.contains("len: 5"));
    }
}
