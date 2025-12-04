use futures::future::Either;
use futures::future::select;
use futures::lock::Mutex;
use std::collections::HashMap;
use std::sync::Arc;

use super::device_error_handling::{DeviceError, DeviceResult};
use crate::utility::future_delay;

pub struct LockManager<K, V> {
    data: Arc<Mutex<HashMap<K, V>>>,
}

impl<K, V> LockManager<K, V>
where
    K: Clone + std::hash::Hash + Eq,
    V: Clone,
{
    pub fn new() -> Self {
        Self {
            data: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    // 安全的插入操作，带超时
    pub async fn insert_with_timeout(&self, key: K, value: V, timeout_ms: u64) -> DeviceResult<()> {
        let insert_future = async {
            let mut guard = self.data.lock().await;
            guard.insert(key, value);
            Ok(())
        };

        let timeout_future = future_delay(timeout_ms as u32);

        match select(Box::pin(insert_future), Box::pin(timeout_future)).await {
            Either::Left((result, _)) => result,
            Either::Right(_) => Err(DeviceError::LockError("插入操作超时".to_string())),
        }
    }

    // 安全的移除操作，带超时
    pub async fn remove_with_timeout(&self, key: &K, timeout_ms: u64) -> DeviceResult<Option<V>> {
        let remove_future = async {
            let mut guard = self.data.lock().await;
            Ok(guard.remove(key))
        };

        let timeout_future = future_delay(timeout_ms as u32);

        match select(Box::pin(remove_future), Box::pin(timeout_future)).await {
            Either::Left((result, _)) => result,
            Either::Right(_) => Err(DeviceError::LockError("移除操作超时".to_string())),
        }
    }

    // 安全的获取操作，带超时
    pub async fn get_with_timeout(&self, key: &K, timeout_ms: u64) -> DeviceResult<Option<V>> {
        let get_future = async {
            let guard = self.data.lock().await;
            Ok(guard.get(key).cloned())
        };

        let timeout_future = future_delay(timeout_ms as u32);

        match select(Box::pin(get_future), Box::pin(timeout_future)).await {
            Either::Left((result, _)) => result,
            Either::Right(_) => Err(DeviceError::LockError("获取操作超时".to_string())),
        }
    }

    // 检查是否包含键
    pub async fn contains_key_with_timeout(&self, key: &K, timeout_ms: u64) -> DeviceResult<bool> {
        let contains_future = async {
            let guard = self.data.lock().await;
            Ok(guard.contains_key(key))
        };

        let timeout_future = future_delay(timeout_ms as u32);

        match select(Box::pin(contains_future), Box::pin(timeout_future)).await {
            Either::Left((result, _)) => result,
            Either::Right(_) => Err(DeviceError::LockError("检查键操作超时".to_string())),
        }
    }

    // 批量操作 - 清理多个键
    pub async fn batch_remove_with_timeout(
        &self,
        keys: Vec<K>,
        timeout_ms: u64,
    ) -> DeviceResult<()> {
        let batch_remove_future = async {
            let mut guard = self.data.lock().await;
            for key in keys {
                guard.remove(&key);
            }
            Ok(())
        };

        let timeout_future = future_delay(timeout_ms as u32);

        match select(Box::pin(batch_remove_future), Box::pin(timeout_future)).await {
            Either::Left((result, _)) => result,
            Either::Right(_) => Err(DeviceError::LockError("批量移除操作超时".to_string())),
        }
    }

    // 简化版本 - 不带超时的操作，用于性能敏感的场景
    pub async fn insert(&self, key: K, value: V) {
        let mut guard = self.data.lock().await;
        guard.insert(key, value);
    }

    pub async fn remove(&self, key: &K) -> Option<V> {
        let mut guard = self.data.lock().await;
        guard.remove(key)
    }

    pub async fn get(&self, key: &K) -> Option<V> {
        let guard = self.data.lock().await;
        guard.get(key).cloned()
    }

    pub async fn contains_key(&self, key: &K) -> bool {
        let guard = self.data.lock().await;
        guard.contains_key(key)
    }
}

impl<K, V> Clone for LockManager<K, V> {
    fn clone(&self) -> Self {
        Self {
            data: Arc::clone(&self.data),
        }
    }
}
