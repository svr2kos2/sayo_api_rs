use pollster::block_on;
use futures::Future;
use futures::future::Either;
use futures::lock::Mutex;
use once_cell::sync::Lazy;
use std::cell::Cell;
use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;
#[cfg(not(target_arch = "wasm32"))]
use std::time::{Duration, Instant};

use crate::device_constants::*;
use crate::utility::future_delay;

use crate::byte_converter::{Encoding, RwBytes};
use hid_rs::{self, HidDevice, SafeCallback, SafeCallback2};

fn block_in_thread<T: Send + 'static>(future: impl Future<Output = T> + Send + 'static) -> T {
    std::thread::spawn(move || block_on(future)).join().expect("async worker panicked")
}

use super::structures::*;
use super::structures_codec::AddressableData;
use crate::structures_codec::CodecableHidPackage;

// use crate::utility::*;

#[path = "report_codec.rs"]
mod report_codec;

fn require_report_codec(uuid: u128) -> Option<Arc<Mutex<report_codec::ReportDecoder>>> {
    let mut binding = REPORT_BUFFER_CODEC.try_lock()?;
    if let Some(existing) = binding.get(&uuid) {
        return Some(existing.clone());
    }
    let decoder = Arc::new(Mutex::new(report_codec::ReportDecoder::new(
        uuid,
        Arc::new(on_broadcast_arrived),
    )));
    binding.insert(uuid, decoder.clone());
    Some(decoder)
}

static REPORT_BUFFER_CODEC: Lazy<Mutex<HashMap<u128, Arc<Mutex<report_codec::ReportDecoder>>>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));
static CONNECTION_CALLBACK: Lazy<SafeCallback2<u128, bool, ()>> = Lazy::new(|| {
    SafeCallback2::new(|hid, connected| {
        println!(
            "CONNECTION_CALLBACK called: {:?} {:?}",
            uuid::Uuid::from_u128(hid),
            connected
        );

        // On some platforms (Android), the caller may not poll the returned future.
        // To ensure the side effects run reliably, spawn the async body and return
        // an already-ready future.
        #[cfg(not(target_arch = "wasm32"))]
        {
            let hid_m = hid;
            let connected_m = connected;
            block_in_thread(async move {
                println!(
                    "on_connection_changed called from CONNECTION_CALLBACK: {:?} {:?}",
                    uuid::Uuid::from_u128(hid_m),
                    connected_m
                );
                let _ = on_connection_changed(hid_m, connected_m).await;
            });
        }

        #[cfg(target_arch = "wasm32")]
        {
            let hid_m = hid;
            let connected_m = connected;
            wasm_bindgen_futures::spawn_local(async move {
                println!(
                    "on_connection_changed called from CONNECTION_CALLBACK: {:?} {:?}",
                    uuid::Uuid::from_u128(hid_m),
                    connected_m
                );
                let _ = on_connection_changed(hid_m, connected_m).await;
            });
        }

        // Return a ready future so the signature is satisfied regardless of polling behavior.
        Box::pin(async {})
    })
});
static REPORT_CALLBACKS: Lazy<Mutex<HashMap<u128, SafeCallback2<u128, Vec<u8>, ()>>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));
static BROADCAST_CALLBACKS: Lazy<Mutex<HashMap<u128, SafeCallback<BroadCast, ()>>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

// Global per-device cache for report-id presence
static REPORT_ID_CACHE_MAP: Lazy<std::sync::Mutex<HashMap<u128, ReportIdCache>>> =
    Lazy::new(|| std::sync::Mutex::new(HashMap::new()));

// pub async fn init_app() {
//     init_sayo_device().await;
// }

pub async fn init_sayo_device() {
    match hid_rs::Hid::init_hid().await {
        Ok(_) => println!("HID initialized."),
        Err(e) => println!("HID initialization failed: {:?}", e),
    }

    // Subscribe to connection changes so we can initialize per-device report decoders on attach.
    match hid_rs::Hid::sub_connection_changed(CONNECTION_CALLBACK.clone()).await {
        Ok(_) => println!("Connection change subscription registered."),
        Err(e) => println!("Connection change subscription failed: {:?}", e),
    }
}

async fn on_connection_changed(uuid: u128, connected: bool) -> bool {
    println!(
        "Device connection changed {:?} {:?}",
        uuid::Uuid::from_u128(uuid),
        connected
    );

    let hid = HidDevice::from(uuid);

    // if !hid.has_report_id(0x21) && !hid.has_report_id(0x22) {
    //     println!("Device {:?} has no report id", hid.uuid);
    //     return false;
    // }

    if connected {
        //println!("device name {:?}", hid.get_product_name());

        // 先处理设备状态
        // 再处理报告缓冲区编解码器
        {
            // Ensure decoder exists (best-effort; ignore if lock is busy)
            let _ = require_report_codec(hid.uuid);
        } // 释放REPORT_BUFFER_CODEC锁

        // 添加报告监听器
        let report_callback = SafeCallback2::new(on_report_arrived);
        println!(
            "Adding report listener for device {:?}",
            uuid::Uuid::from_u128(hid.uuid)
        );
        if let Err(e) = hid.add_report_listener(&report_callback).await {
            println!("Failed to add report listener: {:?}", e);
        }

        // 存储回调
        {
            let mut report_callbacks = REPORT_CALLBACKS.lock().await;
            report_callbacks.insert(hid.uuid, report_callback);
        } // 释放REPORT_CALLBACKS锁
    } else {
        // 移除报告监听器
        {
            let mut report_callbacks = REPORT_CALLBACKS.lock().await;
            if let Some(callback) = report_callbacks.get(&hid.uuid) {
                if let Err(e) = hid.remove_report_listener(&callback).await {
                    println!("Failed to remove report listener: {:?}", e);
                }
                report_callbacks.remove(&hid.uuid);
            }
        } // 释放REPORT_CALLBACKS锁

        // 清理其他资源
        {
            let mut binding = REPORT_BUFFER_CODEC.lock().await;
            binding.remove(&hid.uuid);
        } // 释放REPORT_BUFFER_CODEC锁

        // 清理报告ID缓存
        {
            let mut report_id_map = REPORT_ID_CACHE_MAP.lock().unwrap();
            report_id_map.remove(&hid.uuid);
        }
    }
    println!("Device connection changed done");
    return true;
}

fn on_broadcast_arrived(device: u128, broadcast: &mut BroadCast) {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let callbacks = BROADCAST_CALLBACKS
            .try_lock()
            .expect("BROADCAST_CALLBACKS lock poisoned");
        if let Some(callback) = callbacks.get(&device) {
            let callback_clone = callback.clone();
            drop(callbacks);
            let _ = callback_clone.call(broadcast.clone());
        }
    }

    #[cfg(target_arch = "wasm32")]
    {
        let payload = broadcast.clone();
        wasm_bindgen_futures::spawn_local(async move {
            let callbacks = BROADCAST_CALLBACKS.lock().await;
            if let Some(callback) = callbacks.get(&device) {
                let callback_clone = callback.clone();
                drop(callbacks);
                let _ = callback_clone.call(payload).await;
            }
        });
    }
}

fn on_report_arrived(
    uuid: u128,
    data: Vec<u8>,
) -> Pin<Box<dyn Future<Output = ()> + Send + 'static>> {
    let cmd = data.get(6).cloned().unwrap_or(0);
    let header_bytes = &data[..8.min(data.len())];
    let body_bytes = &data[8.min(data.len())..];
    if cmd != 0xFF && cmd != 0x13 && cmd != 0x25 && cmd != 0x15 && cmd != 0x27 {
        println!("Report arrived: {:02X?} {:02X?}", header_bytes, body_bytes);
    }
    // println!("Report arrived ({:02X?}): {:02X?} {:02X?} end;", data.len(), header_bytes, body_bytes);
    // Lazily ensure a ReportDecoder exists to avoid executor re-entry panics when callbacks race.
    let Some(wrap_codec) = require_report_codec(uuid) else {
        println!("ReportDecoder unavailable (lock busy?) for device {:?}, dropping packet", uuid::Uuid::from_u128(uuid));
        return Box::pin(async {});
    };

    // If the codec lock is busy, drop the report to avoid blocking.
    if let Some(mut codec) = wrap_codec.try_lock() {
        if let Err(e) = codec.join(&mut data.clone()) {
            println!("Failed to join packet: {}", e);
        }
    } else {
        println!("ReportDecoder busy for device {:?}, dropping packet", uuid::Uuid::from_u128(uuid));
    }

    // if data[6] != 0xFF && data[6] != 0x13 && data[6] != 0x25 && data[6] != 0x15 && data[6] != 0x27 {
    //     let packet_len = (data[4] as u16 | (data[5] as u16) << 8) & 0x03FF;
    //     println!("Report arrived: {:02X?} {:02X?}", data[..8].to_vec(), data[8..(8 + packet_len as usize)].to_vec());
    // }
    return Box::pin(async {});
}

pub async fn sub_connection_changed(callback: SafeCallback2<u128, bool, ()>) {
    println!("sub_connection_changed called");
    match hid_rs::Hid::sub_connection_changed(callback).await {
        Ok(_) => (),
        Err(_) => {
            println!("sub_connection_changed failed");
        }
    };
}

pub async fn unsub_connection_changed(callback: SafeCallback2<u128, bool, ()>) {
    match hid_rs::Hid::unsub_connection_changed(callback).await {
        Ok(_) => (),
        Err(_) => {
            println!("unsub_connection_changed failed");
        }
    };
}

pub async fn get_device_list() -> Vec<SayoDeviceApi> {
    let devices = match hid_rs::Hid::get_device_list() {
        Ok(devices) => devices,
        Err(_) => {
            return Vec::new();
        }
    };
    devices.into_iter().map(|device| device.into()).collect()
}

pub enum ScreenLayer {
    Bootup = 0x21,
    Main = 0x22,
    Sleep = 0x23,
}

// Cache for report-id detection with a short warm-up window on native,
// and a simple one-shot initialization on wasm (avoid Instant on wasm).
const REPORT_ID_WARMUP_SECS: u64 = 2;

#[cfg(not(target_arch = "wasm32"))]
struct ReportIdCache {
    created: Instant,
    has_22: Option<bool>,
    has_21: Option<bool>,
}

#[cfg(not(target_arch = "wasm32"))]
impl ReportIdCache {
    fn new() -> Self {
        let now = Instant::now();
        Self {
            created: now,
            has_22: None,
            has_21: None,
        }
    }

    fn in_warmup(&self) -> bool {
        self.created.elapsed() <= Duration::from_secs(REPORT_ID_WARMUP_SECS)
    }

    fn should_refresh(&self) -> bool {
        if self.in_warmup() {
            // During warmup, refresh frequently (no throttling beyond Mutex).
            true
        } else {
            // After warmup, no further refresh unless values were never initialized.
            self.has_22.is_none() || self.has_21.is_none()
        }
    }

    fn mark_refreshed(&mut self) {
        // Nothing to track besides creation/warmup window on native.
    }
}

#[cfg(target_arch = "wasm32")]
struct ReportIdCache {
    created_ms: f64,
    has_22: Option<bool>,
    has_21: Option<bool>,
}

#[cfg(target_arch = "wasm32")]
impl ReportIdCache {
    fn now_ms() -> f64 {
        // Prefer high-resolution performance.now(), fallback to Date.now()
        web_sys::window()
            .and_then(|w| w.performance())
            .map(|p| p.now())
            .unwrap_or_else(|| js_sys::Date::now())
    }

    fn new() -> Self {
        Self {
            created_ms: Self::now_ms(),
            has_22: None,
            has_21: None,
        }
    }

    fn in_warmup(&self) -> bool {
        (Self::now_ms() - self.created_ms) <= (REPORT_ID_WARMUP_SECS as f64) * 1000.0
    }

    fn should_refresh(&self) -> bool {
        if self.in_warmup() {
            true
        } else {
            self.has_22.is_none() || self.has_21.is_none()
        }
    }

    fn mark_refreshed(&mut self) {
        // No state needed besides creation time; nothing to do.
    }
}

#[derive(Debug, Clone)]
pub struct SayoDeviceApi {
    pub uuid: u128,
}
impl From<HidDevice> for SayoDeviceApi {
    fn from(hid_device: HidDevice) -> Self {
        SayoDeviceApi {
            uuid: hid_device.uuid,
        }
    }
}
impl From<u128> for SayoDeviceApi {
    fn from(uuid: u128) -> Self {
        SayoDeviceApi { uuid: uuid }
    }
}

impl Into<HidDevice> for SayoDeviceApi {
    fn into(self) -> HidDevice {
        HidDevice::from(self.uuid)
    }
}

impl SayoDeviceApi {
    #[cfg(not(target_arch = "wasm32"))]
    pub const ECHO: u8 = 0x13;
    #[cfg(target_arch = "wasm32")]
    pub const ECHO: u8 = 0x12;

    pub fn from_uuid(uuid: u128) -> Self {
        SayoDeviceApi { uuid: uuid }
    }

    pub async fn passiv_mode(&self) -> bool {
        return on_connection_changed(self.uuid, false).await;
    }

    pub async fn active_mode(&self) -> bool {
        let hid = HidDevice::from(self.uuid);
        if !hid.has_report_id(0x21) && !hid.has_report_id(0x22) && !hid.has_report_id(0x02) {
            return false;
        }
        return on_connection_changed(self.uuid, true).await;
    }

    pub async fn is_active_mode(&self) -> bool {
        REPORT_BUFFER_CODEC.lock().await.contains_key(&self.uuid)
    }

    pub fn has_report_id(&self, report_id: u8) -> bool {
        println!("sayo has_report_id {:02X?}", report_id);
        // For the common IDs 0x21 and 0x22, use the same cache strategy as get_report_id.
        if report_id == 0x21 || report_id == 0x22 {
            let mut map = REPORT_ID_CACHE_MAP.lock().unwrap();
            let cache = map.entry(self.uuid).or_insert_with(ReportIdCache::new);
            if cache.should_refresh() {
                let hid = HidDevice::from(self.uuid);
                let now_22 = hid.has_report_id(0x22);
                let now_21 = hid.has_report_id(0x21);
                cache.has_22 = Some(now_22);
                cache.has_21 = Some(now_21);
                cache.mark_refreshed();
            }
            return match report_id {
                0x22 => cache
                    .has_22
                    .unwrap_or_else(|| HidDevice::from(self.uuid).has_report_id(0x22)),
                0x21 => cache
                    .has_21
                    .unwrap_or_else(|| HidDevice::from(self.uuid).has_report_id(0x21)),
                _ => false,
            };
        }
        // For other IDs, fall back to direct query.
        HidDevice::from(self.uuid).has_report_id(report_id)
    }

    async fn send_hid_report(&self, data: Vec<Vec<u8>>) -> Result<(), &'static str> {
        let hid = HidDevice::from(self.uuid);
        for report in data {
            if report[6] != 0x13 && report[6] != 0x25 && report[6] != 0x15 && report[6] != 0x27 {
                println!(
                    "Sending report: {:02X?} {:02X?}",
                    report[..8].to_vec(),
                    report[8..].to_vec()
                );
            }
            // println!("Sending report: {:02X?}", report);
            let timeout = future_delay(SEND_TIMEOUT_MS);
            let send = hid.send_report(report);
            let send_timeout = futures::future::select(Box::pin(send), Box::pin(timeout));
            match send_timeout.await {
                Either::Left(res) => match res.0 {
                    Ok(_) => (),
                    Err(e) => {
                        println!("Send report failed: {:?}", e);
                        return Err("Send report failed");
                    }
                },
                Either::Right(_) => {
                    return Err("Send report Timeout");
                }
            };
        }
        return Ok(());
    }
    async fn request_with_header<T: CodecableHidPackage>(
        &self,
        report_id: u8,
        echo: u8,
        cmd: u8,
        index: u8,
        content: &T,
    ) -> Option<(HidReportHeader, T)> {
        let wrap_codec = match require_report_codec(self.uuid) {
            Some(codec) => codec,
            None => {
                println!("No codec found for device (lock busy?)");
                return None;
            }
        };
        let response = {
            let codec = wrap_codec
                .try_lock()
                .expect("wrap_codec lock poisoned");
            codec.request_response::<T>(report_id, cmd, index)
        };
        // drop(codec);
        let reports = match report_codec::encode_report(report_id, echo, cmd, index, content) {
            Ok(reports) => reports,
            Err(e) => {
                println!("Request with header: Encode report failed: {}", e);
                return None;
            }
        };
        match self.send_hid_report(reports).await {
            Ok(_) => {
                //println!("Request with header: Send report success");
            }
            Err(_) => {
                println!("Request with header: Send report failed");
                return None;
            }
        };
        match response.await {
            Ok((header, content)) => {
                //println!("Request with header: Response from device {:02X?}", cmd);
                return Some((header, content));
            }
            Err(_) => {
                println!("Request with header: No response from device");
                return None;
            }
        }
    }

    async fn request<T: CodecableHidPackage>(
        &self,
        report_id: u8,
        echo: u8,
        cmd: u8,
        index: u8,
        content: &T,
    ) -> Option<T> {
        let response = self
            .request_with_header(report_id, echo, cmd, index, content)
            .await;
        return match response {
            Some((header, content)) => {
                let status = header.status(None).expect("Bad Report Header");
                if status != STATUS_OK && status != STATUS_PARTIAL && status != STATUS_COMPLETE {
                    return None;
                }
                Some(content)
            }
            None => None,
        };
    }

    async fn request_all_index<T: CodecableHidPackage>(&self, report_id: u8, cmd: u8) -> Vec<T> {
        let mut res: Vec<T> = Vec::new();
        let mut index = 0;
        let mut consecutive_failures = 0;

        loop {
            if consecutive_failures >= MAX_RETRY_COUNT {
                println!(
                    "Request all index: Too many consecutive failures for cmd {:02X?}",
                    cmd
                );
                break;
            }

            let response = self
                .request_with_header(report_id, SayoDeviceApi::ECHO, cmd, index, &T::empty())
                .await;

            let (header, content) = match response {
                Some((header, content)) => {
                    // println!("Request all index: Response from device {:02X?} {:02X?}", cmd, index);
                    consecutive_failures = 0; // 重置失败计数
                    (header, content)
                }
                None => {
                    println!(
                        "Request all index: No response from device {:02X?} {:02X?}",
                        cmd, index
                    );
                    consecutive_failures += 1;
                    continue;
                }
            };

            match header.status(None) {
                Some(status) => {
                    if status == STATUS_OK || status == STATUS_PARTIAL || status == STATUS_COMPLETE
                    {
                        res.push(content);
                        index += 1;
                    } else {
                        println!(
                            "Request all index: Response from device with bad status {:02X?} {:02X?} {:02X?}",
                            cmd, index, status
                        );
                        break;
                    }
                }
                None => {
                    println!(
                        "Request all index: Response from device with bad header {:02X?} {:02X?} ",
                        cmd, index
                    );
                    break;
                }
            }

            if index == 0xff {
                println!("Request all index: Reached end of index {:02X?} ", cmd);
                break;
            }

            // 添加小延迟以避免过快的请求
            // if index % 10 == 0 {
            //     future_delay(10).await;
            // }
        }
        println!("Request all index: Done with {:} elements", res.len());
        return res;
    }
}

impl SayoDeviceApi {
    pub fn get_uuid(&self) -> u128 {
        return self.uuid;
    }

    pub fn vid(&self) -> u16 {
        let hid = HidDevice::from(self.uuid);
        match hid.vid() {
            Ok(vid) => vid,
            Err(_) => 0,
        }
    }

    pub fn pid(&self) -> u16 {
        let hid = HidDevice::from(self.uuid);
        match hid.pid() {
            Ok(pid) => pid,
            Err(_) => 0,
        }
    }

    pub fn get_product_name(&self) -> Option<String> {
        // println!("sayo get_product_name");
        return match HidDevice::from(self.uuid).get_product_name() {
            Ok(name) => {
                // println!("sayo get_product_name: {:?}", name);
                name.clone()
            }
            Err(_) => None,
        };
    }

    pub fn get_report_id(&self) -> u8 {
        // println!("sayo get_report_id");
        // Use cached result with warmup/dynamic strategy.
        let mut map = REPORT_ID_CACHE_MAP.lock().unwrap();
        let cache = map.entry(self.uuid).or_insert_with(ReportIdCache::new);
        if cache.should_refresh() {
            let hid = HidDevice::from(self.uuid);
            let now_22 = hid.has_report_id(0x22);
            let now_21 = hid.has_report_id(0x21);
            cache.has_22 = Some(now_22);
            cache.has_21 = Some(now_21);
            cache.mark_refreshed();
        }
        let id = match cache.has_22.unwrap_or(false) {
            true => 0x22,
            false => 0x21,
        };
        // println!("sayo get_report_id: {:02X?}", id);
        return id;
    }

    pub fn is_hispeed(&self) -> bool {
        return self.get_report_id() == 0x22;
    }

    pub async fn reboot(&self) -> bool {
        let report_id = self.get_report_id();
        const INDEX: u8 = 0x00;
        let empty = ByteArray::new(RwBytes::new(vec![
            REBOOT_MAGIC as u8,
            (REBOOT_MAGIC >> 8) as u8,
            SUBCMD_REBOOT,
            !SUBCMD_REBOOT,
        ]));
        let response = self.request(report_id, SayoDeviceApi::ECHO, CMD_REBOOT, INDEX, &empty);
        match response.await {
            Some(_) => false,
            None => true,
        }
    }

    pub async fn recovery(&self) -> bool {
        let report_id = self.get_report_id();
        const INDEX: u8 = 0x00;
        let empty = ByteArray::new(RwBytes::new(vec![
            REBOOT_MAGIC as u8,
            (REBOOT_MAGIC >> 8) as u8,
            SUBCMD_RECOVERY,
            !SUBCMD_RECOVERY,
        ]));
        let response = self.request(report_id, SayoDeviceApi::ECHO, CMD_REBOOT, INDEX, &empty);
        match response.await {
            Some(_) => false,
            None => true,
        }
    }

    pub async fn into_bootloader(&self) -> bool {
        let report_id = self.get_report_id();
        const INDEX: u8 = 0x00;
        let empty = ByteArray::new(RwBytes::new(vec![
            REBOOT_MAGIC as u8,
            (REBOOT_MAGIC >> 8) as u8,
            SUBCMD_BOOTLOADER,
            !SUBCMD_BOOTLOADER,
        ]));
        let response = self.request(report_id, SayoDeviceApi::ECHO, CMD_REBOOT, INDEX, &empty);
        match response.await {
            Some(_) => false,
            None => true,
        }
    }

    pub async fn set_device_name(&self, name: String, len: usize) -> Option<String> {
        let str = StringContent::new(RwBytes::from_str(Encoding::UTF16LE, &name));
        str.encoding_byte.set(Some(u8::from(Encoding::UTF16LE)));
        // str.str(Some(name));
        let report_id = self.get_report_id();
        const CMD: u8 = 0x01;
        const INDEX: u8 = 0x00;
        let mut content = str.bytes.into_vec();
        content.resize(len, 0);
        let bytes_content = ByteArray::new(RwBytes::new(content));
        let response = self.request(report_id, SayoDeviceApi::ECHO, CMD, INDEX, &bytes_content);
        match response.await {
            Some(content) => StringContent {
                encoding_byte: Cell::new(Some(0x03)),
                bytes: content.bytes,
            }
            .str(None),
            None => None,
        }
    }

    pub async fn get_device_name(&self) -> Option<(String, usize)> {
        let str = StringContent::empty();
        str.encoding_byte.set(Some(u8::from(Encoding::UTF16LE)));
        let report_id = self.get_report_id();
        const CMD: u8 = 0x01;
        const INDEX: u8 = 0x00;
        let response = self.request(report_id, SayoDeviceApi::ECHO, CMD, INDEX, &str);
        let content = match response.await {
            Some(content) => content,
            None => return None,
        };
        Some((
            content.str(None).unwrap_or("".to_string()),
            content.bytes_len(),
        ))
    }

    pub async fn get_device_info(&self) -> Option<DeviceInfo> {
        let report_id = self.get_report_id();
        const INDEX: u8 = 0x00;
        let empty = DeviceInfo::empty();
        let response = self.request(report_id, SayoDeviceApi::ECHO, CMD_DEVICE_INFO, INDEX, &empty);
        let device_info = match response.await {
            Some(info) => info,
            None => return None,
        };
        Some(device_info)
    }
    pub async fn set_device_info(&self, device_info: &DeviceInfo) -> Option<DeviceInfo> {
        let report_id = self.get_report_id();
        const INDEX: u8 = 0x00;
        let response = self.request(
            report_id,
            SayoDeviceApi::ECHO,
            CMD_DEVICE_INFO,
            INDEX,
            device_info,
        );
        response.await
    }

    pub async fn get_system_info(&self) -> Option<SystemInfo> {
        let report_id = self.get_report_id();
        const INDEX: u8 = 0x00;
        let empty = SystemInfo::empty();
        let response = self.request(report_id, SayoDeviceApi::ECHO, CMD_SYSTEM_INFO, INDEX, &empty);
        response.await
    }
    pub async fn set_system_info(&self, system_info: &SystemInfo) -> Option<SystemInfo> {
        let report_id = self.get_report_id();
        const INDEX: u8 = 0x00;
        let response = self.request(
            report_id,
            SayoDeviceApi::ECHO,
            CMD_SYSTEM_INFO,
            INDEX,
            system_info,
        );
        response.await
    }

    pub async fn get_optional_bytes(&self) -> Option<DeviceConfig> {
        let report_id = self.get_report_id();
        const CMD: u8 = 0x03;
        const INDEX: u8 = 0x00;
        let empty = DeviceConfig::empty();
        let response = self.request(report_id, SayoDeviceApi::ECHO, CMD, INDEX, &empty);
        response.await
    }
    pub async fn set_optional_bytes(
        &self,
        optional_bytes: &DeviceConfig,
    ) -> Option<DeviceConfig> {
        let report_id = self.get_report_id();
        const CMD: u8 = 0x03;
        const INDEX: u8 = 0x00;
        let response = self.request(report_id, SayoDeviceApi::ECHO, CMD, INDEX, optional_bytes);
        response.await
    }

    pub async fn get_rf_config(&self) -> Option<RFConfig> {
        let report_id = self.get_report_id();
        const CMD: u8 = 0x04;
        const INDEX: u8 = 0x00;
        let empty = RFConfig::empty();
        let response = self.request(report_id, SayoDeviceApi::ECHO, CMD, INDEX, &empty);
        response.await
    }

    pub async fn set_rf_config(&self, rf_config: &RFConfig) -> Option<RFConfig> {
        let report_id = self.get_report_id();
        const CMD: u8 = 0x04;
        const INDEX: u8 = 0x00;
        let response = self.request(report_id, SayoDeviceApi::ECHO, CMD, INDEX, rf_config);
        response.await
    }

    pub async fn lock_device(&self, password: &StringContent) -> Option<bool> {
        if password.encoding_byte.get() != Some(u8::from(Encoding::ASCII)) {
            println!("Password must be ASCII");
            return None;
        }
        if password.bytes_len() > 32 {
            println!("Password length must be between 4 and 32");
            return None;
        }
        let report_id = self.get_report_id();
        const CMD: u8 = 0x05;
        const INDEX: u8 = 0x00;
        let response = self.request(report_id, SayoDeviceApi::ECHO, CMD, INDEX, password);
        match response.await {
            Some(_) => Some(true),
            None => Some(false),
        }
    }

    pub async fn unlock_device(&self, password: &StringContent) -> Option<bool> {
        if password.encoding_byte.get() != Some(u8::from(Encoding::ASCII)) {
            println!("Password must be ASCII");
            return None;
        }
        if password.bytes_len() > 32 || password.bytes_len() < 4 {
            println!("Password length must be between 4 and 32");
            return None;
        }
        let report_id = self.get_report_id();
        const CMD: u8 = 0x06;
        const INDEX: u8 = 0x00;
        let response = self.request(report_id, SayoDeviceApi::ECHO, CMD, INDEX, password);
        match response.await {
            Some(_) => Some(true),
            None => Some(false),
        }
    }

    pub async fn get_key_infos(&self) -> Vec<KeyInfo> {
        println!("get_key_infos");
        let report_id = self.get_report_id();
        const CMD: u8 = 0x10;
        self.request_all_index::<KeyInfo>(report_id, CMD).await
    }

    pub async fn set_key_info(&self, index: u8, key_info: &KeyInfo) -> Option<KeyInfo> {
        let report_id = self.get_report_id();
        const CMD: u8 = 0x10;
        let response = self.request(report_id, SayoDeviceApi::ECHO, CMD, index, key_info);
        response.await
    }

    pub async fn get_led_infos(&self) -> Vec<LEDInfo> {
        let report_id = self.get_report_id();
        const CMD: u8 = 0x11;
        self.request_all_index::<LEDInfo>(report_id, CMD).await
    }

    pub async fn set_led_info(&self, index: u8, led_info: &LEDInfo) -> Option<LEDInfo> {
        let report_id = self.get_report_id();
        const CMD: u8 = 0x11;
        let response = self.request(report_id, SayoDeviceApi::ECHO, CMD, index, led_info);
        response.await
    }

    pub async fn get_color_tables(&self) -> Vec<ColorTable> {
        let report_id = self.get_report_id();
        const CMD: u8 = 0x12;
        self.request_all_index::<ColorTable>(report_id, CMD).await
    }

    pub async fn set_color_table(&self, index: u8, color_table: &ColorTable) -> Option<ColorTable> {
        let report_id = self.get_report_id();
        const CMD: u8 = 0x12;
        let response = self.request(report_id, SayoDeviceApi::ECHO, CMD, index, color_table);
        response.await
    }

    pub async fn get_touch_sensitivity(&self, index: u8) -> Option<TouchSensitivity> {
        let report_id = self.get_report_id();
        const CMD: u8 = 0x13;
        let empty = TouchSensitivity::empty();
        let response = self.request(report_id, SayoDeviceApi::ECHO, CMD, index, &empty);
        response.await
    }

    pub async fn get_touch_sensitivitys(&self) -> Vec<TouchSensitivity> {
        let report_id = self.get_report_id();
        const CMD: u8 = 0x13;
        self.request_all_index::<TouchSensitivity>(report_id, CMD)
            .await
    }

    pub async fn set_touch_sensitivity(
        &self,
        index: u8,
        touch_sensitivity: &TouchSensitivity,
    ) -> Option<TouchSensitivity> {
        let report_id = self.get_report_id();
        const CMD: u8 = 0x13;
        let response = self.request(report_id, SayoDeviceApi::ECHO, CMD, index, touch_sensitivity);
        response.await
    }

    pub async fn get_passwords(&self) -> Vec<StringContent> {
        let report_id = self.get_report_id();
        const CMD: u8 = 0x16;
        self.request_all_index::<StringContent>(report_id, CMD)
            .await
    }

    pub async fn set_password(&self, index: u8, value: StringContent) -> Option<StringContent> {
        let report_id = self.get_report_id();
        const CMD: u8 = 0x16;
        let response = self.request(report_id, SayoDeviceApi::ECHO, CMD, index, &value);
        response.await
    }

    pub async fn get_strings(&self) -> Vec<StringContent> {
        let report_id = self.get_report_id();
        const CMD: u8 = 0x17;
        self.request_all_index::<StringContent>(report_id, CMD)
            .await
    }

    pub async fn set_string(&self, index: u8, value: StringContent) -> Option<StringContent> {
        let report_id = self.get_report_id();
        const CMD: u8 = 0x17;
        let response = self.request(report_id, SayoDeviceApi::ECHO, CMD, index, &value);
        response.await
    }

    pub async fn get_script_names(&self) -> Vec<StringContent> {
        let report_id = self.get_report_id();
        const CMD: u8 = 0x19;
        self.request_all_index::<StringContent>(report_id, CMD)
            .await
    }

    pub async fn set_script_name(&self, index: u8, value: StringContent) -> Option<StringContent> {
        let report_id = self.get_report_id();
        const CMD: u8 = 0x19;
        let response = self.request(report_id, SayoDeviceApi::ECHO, CMD, index, &value);
        response.await
    }

    pub async fn pull_screen_buffer(&self, len: &u32) -> Vec<u8> {
        let Some(wrap_codec) = require_report_codec(self.uuid) else {
            println!("No codec found for device (lock busy?)");
            return Vec::new();
        };
        //codec.join(&mut data.clone());
        let mut codec = wrap_codec
            .try_lock()
            .expect("wrap_codec lock poisoned");
        codec.resize_screen_buffer(len.clone() as usize);
        let mut res: Vec<u8> = vec![0; len.clone() as usize];
        codec.get_screen_buffer(&mut res);
        drop(codec);
        let report_id = self.get_report_id();
        let cmd: u8 = ScreenBuffer::CMD.expect("No CMD found for ScreenBuffer");
        let index: u8 = 0x00;
        let empty = ScreenBuffer::empty();
        let reports =
            match report_codec::encode_report(report_id, SayoDeviceApi::ECHO, cmd, index, &empty) {
                Ok(reports) => reports,
                Err(e) => {
                    println!("Pull screen buffer: Encode report failed: {}", e);
                    return res;
                }
            };
        match self.send_hid_report(reports).await {
            Ok(_) => (),
            Err(_) => {
                println!("Pull screen buffer: Send report failed");
            }
        }
        return res;
    }

    pub async fn get_lcd_draw_datas(&self, layer: ScreenLayer) -> Vec<LcdDrawData> {
        let report_id = self.get_report_id();
        let cmd = layer as u8;
        return self.request_all_index(report_id, cmd).await;
    }

    pub async fn set_lcd_draw_data(
        &self,
        layer: u8,
        index: u8,
        data: &LcdDrawData,
    ) -> Option<LcdDrawData> {
        let report_id = self.get_report_id();
        let cmd = layer;
        let response = self.request(report_id, SayoDeviceApi::ECHO, cmd, index, data);
        response.await
    }

    pub async fn get_hall_50um(&self, key_to_record: Option<u8>) -> Option<ByteArray> {
        let report_id = self.get_report_id();
        const CMD: u8 = 0x15;
        let bytes = match key_to_record {
            Some(key) => ByteArray::new(RwBytes::new(vec![key])),
            None => ByteArray::empty(),
        };
        let response = self.request(report_id, SayoDeviceApi::ECHO, CMD, 0, &bytes);
        response.await
    }

    pub async fn get_hall_info_um(&self, key_to_record: Option<u8>) -> Option<ByteArray> {
        let report_id = self.get_report_id();
        const CMD: u8 = 0x15;
        let bytes = match key_to_record {
            Some(key) => ByteArray::new(RwBytes::new(vec![key])),
            None => ByteArray::empty(),
        };
        let response = self.request(report_id, SayoDeviceApi::ECHO, CMD, 1, &bytes);
        response.await
    }

    pub async fn get_analog_key_infos(&self) -> Vec<AnalogKeyInfo> {
        let report_id = self.get_report_id();
        let cmd: u8 = AnalogKeyInfo::CMD.expect("No CMD found for AnalogKeyInfo");
        let response = self.request_all_index::<AnalogKeyInfo>(report_id, cmd);
        response.await
    }

    pub async fn get_analog_key_info(&self, index: u8) -> Option<AnalogKeyInfo> {
        let report_id = self.get_report_id();
        let cmd: u8 = AnalogKeyInfo::CMD.expect("No CMD found for AnalogKeyInfo");
        let empty = AnalogKeyInfo::empty();
        let response = self.request(report_id, SayoDeviceApi::ECHO, cmd, index, &empty);
        response.await
    }

    pub async fn set_analog_key_info(
        &self,
        index: u8,
        key_info: &AnalogKeyInfo,
    ) -> Option<AnalogKeyInfo> {
        let report_id = self.get_report_id();
        let cmd: u8 = AnalogKeyInfo::CMD.expect("No CMD found for AnalogKeyInfo");
        let response = self.request(report_id, SayoDeviceApi::ECHO, cmd, index, key_info);
        response.await
    }

    pub async fn save_all(&self) -> bool {
        let report_id = self.get_report_id();
        const INDEX: u8 = 0x00;
        let empty = ByteArray::new(RwBytes::new(vec![0x96, 0x72]));
        let response = self.request(report_id, SayoDeviceApi::ECHO, CMD_SAVE_ALL, INDEX, &empty);
        match response.await {
            Some(_) => true,
            None => false,
        }
    }

    pub async fn get_display_assets_address_len(&self, index: u8) -> u32 {
        self.get_addressable_data_len::<DisplayAssetsPacket>(index)
            .await
    }

    pub async fn get_display_assets_with_addr(
        &self,
        index: u8,
        addr: u32,
    ) -> Option<DisplayAssetsPacket> {
        self.get_addressable_data_with_addr::<DisplayAssetsPacket>(index, addr)
            .await
    }

    //max len, display assets
    pub async fn get_display_assets(&self, index: u8) -> Option<(u32, DisplayAssets)> {
        let (size, bytes) = match self
            .get_addressable_data::<DisplayAssetsPacket>(index)
            .await
        {
            Some((size, data)) => (size, data),
            None => return None,
        };
        Some((size, DisplayAssets::new(bytes)))
    }

    pub async fn set_display_assets(
        &self,
        index: u8,
        display_assets: &DisplayAssets,
        base_addr: usize,
        on_progress: impl Fn(f32) -> Pin<Box<dyn Future<Output = bool> + Send + 'static>>
        + Send
        + Sync
        + 'static,
    ) -> bool {
        self.set_addressable_data::<DisplayAssetsPacket>(
            index,
            display_assets.bytes.clone(),
            base_addr,
            on_progress,
        )
        .await
    }

    pub async fn get_script_address_len(&self, index: u8) -> u32 {
        self.get_addressable_data_len::<SayoScriptPacket>(index)
            .await
    }

    pub async fn get_script_with_addr(&self, index: u8, addr: u32) -> Option<SayoScriptPacket> {
        self.get_addressable_data_with_addr::<SayoScriptPacket>(index, addr)
            .await
    }

    pub async fn get_script(&self, index: u8) -> Option<(u32, SayoScriptContent)> {
        //max address, script
        let (size, bytes) = match self.get_addressable_data::<SayoScriptPacket>(index).await {
            Some((size, data)) => (size, data),
            None => return None,
        };
        Some((size, SayoScriptContent::new(bytes)))
    }

    pub async fn get_all_scripts(&self) -> Vec<(u32, SayoScriptContent)> {
        let mut res = Vec::new();
        let mut index = 0;
        loop {
            match self.get_script(index).await {
                Some((max_len, script)) => res.push((max_len, script)),
                None => break,
            }
            index += 1;
        }
        return res;
    }

    pub async fn set_script(
        &self,
        index: u8,
        script: &SayoScriptContent,
        base_addr: usize,
        on_progress: impl Fn(f32) -> Pin<Box<dyn Future<Output = bool> + Send + 'static>>
        + Send
        + Sync
        + 'static,
    ) -> bool {
        self.set_addressable_data::<SayoScriptPacket>(
            index,
            script.bytes.clone(),
            base_addr,
            on_progress,
        )
        .await
    }

    pub async fn get_addressable_data_len<T: AddressableData + CodecableHidPackage>(
        &self,
        index: u8,
    ) -> u32 {
        let report_id = self.get_report_id();
        let cmd = T::CMD.expect("No CMD found for AddressableData in get_addressable_data_len");
        let over_addr = T::new(RwBytes::new(vec![0xFF, 0xFF, 0xFF, 0xFF]));
        let res = self
            .request_with_header(report_id, SayoDeviceApi::ECHO, cmd, index, &over_addr)
            .await;
        if res.is_none() {
            return 0;
        }
        let (header, body) = res.expect("No response from device");
        if header.status(None) != Some(STATUS_OVERFLOW) {
            return 0;
        }
        return match body.address(None) {
            Some(addr) => addr,
            None => 0,
        };
    }

    pub async fn get_addressable_data_with_addr<T: AddressableData + CodecableHidPackage>(
        &self,
        index: u8,
        addr: u32,
    ) -> Option<T> {
        let report_id = self.get_report_id();
        let cmd: u8 =
            T::CMD.expect("No CMD found for AddressableData in get_addressable_data_with_addr");
        let empty = T::new(RwBytes::new(vec![
            addr as u8,
            (addr >> 8) as u8,
            (addr >> 16) as u8,
            (addr >> 24) as u8,
        ]));

        let response = self.request(report_id, SayoDeviceApi::ECHO, cmd, index, &empty);
        response.await
    }

    pub async fn get_display_assets_data_stream(
        &self,
        index: u8,
        on_data_recv: SafeCallback<Vec<u8>, bool>,
    ) {
        let max_len = match self
            .get_addressable_data_len::<DisplayAssetsPacket>(index)
            .await
        {
            0 => {
                #[cfg(target_arch = "wasm32")]
                on_data_recv.call(vec![0x00; 0]).await;
                #[cfg(not(target_arch = "wasm32"))]
                on_data_recv.call(vec![0x00; 0]).await;
                return;
            }
            len => len,
        };
        #[cfg(target_arch = "wasm32")]
        on_data_recv.call(max_len.to_le_bytes().to_vec()).await;
        #[cfg(not(target_arch = "wasm32"))]
        on_data_recv.call(max_len.to_le_bytes().to_vec()).await;
        let mut bytes = Vec::new();
        let mut retry_cnt = 0;

        while bytes.len() < max_len as usize {
            let data_packet = match self
                .get_addressable_data_with_addr::<DisplayAssetsPacket>(index, bytes.len() as u32)
                .await
            {
                Some(data) => data,
                None => {
                    if retry_cnt >= 3 {
                        break;
                    }
                    retry_cnt += 1;
                    continue;
                }
            };
            retry_cnt = 0;
            if data_packet
                .address(None)
                .expect("Can not get address for data_packet")
                != bytes.len() as u32
            {
                #[cfg(target_arch = "wasm32")]
                on_data_recv.call(vec![0x00; 0]).await;
                #[cfg(not(target_arch = "wasm32"))]
                on_data_recv.call(vec![0x00; 0]).await;
                break;
            }
            #[cfg(target_arch = "wasm32")]
            let next = on_data_recv
                .call(
                    data_packet
                        .data(None)
                        .expect("Can not get data for data_packet"),
                )
                .await;
            #[cfg(not(target_arch = "wasm32"))]
            let next = on_data_recv
                .call(
                    data_packet
                        .data(None)
                        .expect("Can not get data for data_packet"),
                )
                .await;
            if !next {
                println!("on_data_recv done");
                break;
            }
            bytes.append(
                &mut data_packet
                    .data(None)
                    .expect("Can not get data for data_packet"),
            );
        }
        // println!("recv data: len: {:?} [{:02X?}]", bytes.len(), bytes);
        _ = self
            .get_addressable_data_len::<DisplayAssetsPacket>(index)
            .await;
        #[cfg(target_arch = "wasm32")]
        on_data_recv.call(vec![0x00; 0]).await;
        #[cfg(not(target_arch = "wasm32"))]
        on_data_recv.call(vec![0x00; 0]).await;
        return;
    }

    pub async fn get_addressable_data<T: AddressableData + CodecableHidPackage>(
        &self,
        index: u8,
    ) -> Option<(u32, RwBytes)> {
        let max_len = match self.get_addressable_data_len::<T>(index).await {
            0 => return None,
            len => len,
        };
        let mut bytes = Vec::new();
        // let mut current_data_end = 0;

        while bytes.len() < max_len as usize {
            let data_packet = match self
                .get_addressable_data_with_addr::<T>(index, bytes.len() as u32)
                .await
            {
                Some(data) => data,
                None => break,
            };
            if data_packet
                .address(None)
                .expect("Can not get address for data_packet")
                != bytes.len() as u32
            {
                panic!("Data addr not match");
            }
            bytes.append(
                &mut data_packet
                    .data(None)
                    .expect("Can not get data for data_packet"),
            );

            // if bytes.len() <= current_data_end {
            //     continue;
            // }

            // TODO: check if data ends
            // let data_type = bytes.u8(current_data_end, None).unwrap();
            // if data_type != 1 && data_type != 2 && data_type != 6 {
            //     println!("Data data type not valid: {:?} at {:08X?}", data_type, current_data_end);
            //     break;
            // }

            // if bytes.len() <= current_data_end + 12 {
            //     continue;
            // }

            // let data_len = bytes[current_data_end + 8]         |
            //                   (bytes[current_data_end + 9] << 8)   |
            //                   (bytes[current_data_end + 10] << 16) |
            //                   (bytes[current_data_end + 11] << 24);

            // // let data_len = RwBytes::new(bytes).u32(current_data_end + 8, None).expect("Can not get data len");
            // current_data_end += 12 + data_len as usize;
        }
        // println!("recv data: len: {:?} [{:02X?}]", bytes.len(), bytes);
        _ = self.get_addressable_data_len::<T>(index).await;
        if bytes.len() < max_len as usize {
            bytes.resize(max_len as usize, 0x00);
        }
        Some((max_len, RwBytes::new(bytes)))
    }

    // data should be whole data, that mean data should begin at address 0x00000000
    pub async fn set_addressable_data<T: AddressableData + CodecableHidPackage>(
        &self,
        index: u8,
        data: RwBytes,
        base_addr: usize,
        on_progress: impl Fn(f32) -> Pin<Box<dyn Future<Output = bool> + Send + 'static>>
        + Send
        + Sync
        + 'static,
    ) -> bool {
        println!("set_addressable_data: {:?} at {:?}", data, base_addr);
        let report_id = self.get_report_id();
        let cmd: u8 = T::CMD.expect("No CMD found for AddressableData in set_addressable_data");

        let max_packet_len = match report_id {
            REPORT_ID_BOOTUP => MAX_PACKET_LEN_REPORT_21,
            REPORT_ID_MAIN => MAX_PACKET_LEN_REPORT_22,
            _ => 0,
        };

        let mut address = base_addr;
        let mut addr_end = data.len() as usize;
        if address % ADDR_ALIGNMENT != 0 {
            address -= address % ADDR_ALIGNMENT;
        }
        if addr_end == address {
            addr_end += 1;
        }
        if addr_end % ADDR_ALIGNMENT != 0 {
            addr_end += ADDR_ALIGNMENT - (addr_end % ADDR_ALIGNMENT);
        }

        println!(
            "set_addressable_data: address: {:?} addr_end: {:?} data len: {:?}",
            address,
            addr_end,
            data.len()
        );

        let bytes = if addr_end > data.len() {
            let data_len = data.len();
            let mut copy = match data.ref_at(address, data_len - address) {
                Some(data) => data.into_vec(),
                None => {
                    println!("Can not get ref_at in set_addressable_data 0");
                    return false;
                }
            };
            copy.append(&mut vec![0x00; addr_end - data_len as usize]);
            RwBytes::new(copy)
        } else {
            match data.ref_at(address, addr_end - address) {
                Some(data) => data,
                None => {
                    println!("Can not get ref_at in set_addressable_data 1");
                    return false;
                }
            }
        };

        println!("set_addressable_data: bytes: {:?}", bytes);

        // println!("send data: len: {:?} {:02X?}", bytes.len(), bytes.clone().into_vec());
        let mut packets: Vec<T> = Vec::new();
        for i in (0..bytes.len()).step_by(max_packet_len) {
            let addr = address + i;
            let mut packet_data = Vec::new();
            let packet_len = std::cmp::min(max_packet_len, bytes.len() - i);
            packet_data.push(addr as u8);
            packet_data.push((addr >> 8) as u8);
            packet_data.push((addr >> 16) as u8);
            packet_data.push((addr >> 24) as u8);
            packet_data.append(
                &mut bytes
                    .ref_at(i, packet_len)
                    .expect(format!("Can not get ref_at in set_addressable_data 2").as_str())
                    .into_vec(),
            );
            let packet = T::new(RwBytes::new(packet_data));
            packets.push(packet);
        }
        let mut responses = Vec::new();
        for packet in &packets {
            let response = self.request(report_id, SayoDeviceApi::ECHO, cmd, index, packet);
            responses.push(response);
        }
        let mut complate = true;
        let mut failed_index = Vec::new();
        let mut res_index = 0;
        for response in responses {
            match response.await {
                Some(_) => (),
                None => {
                    complate = false;
                    failed_index.push(res_index);
                }
            }
            res_index += 1;
            let progress = res_index as f32 / packets.len() as f32;
            #[cfg(target_arch = "wasm32")]
            let _ = on_progress(progress).await;
            #[cfg(not(target_arch = "wasm32"))]
            let _ = block_in_thread(on_progress(progress));
        }
        _ = self.get_addressable_data_len::<T>(index).await;
        if complate {
            println!(
                "send addressable data complate with len {:?} in {:?} packets",
                bytes.len(),
                packets.len()
            );
        } else {
            println!(
                "send addressable data failed with packets {:?}",
                failed_index
            );
        }
        return complate;
    }

    pub async fn get_analog_key_infos2(&self) -> Vec<AnalogKeyInfo2> {
        let report_id = self.get_report_id();
        let cmd: u8 = AnalogKeyInfo2::CMD.expect("No CMD found for AnalogKeyInfo2");
        let response = self.request_all_index::<AnalogKeyInfo2>(report_id, cmd);
        response.await
    }

    pub async fn get_analog_key_info2(&self, index: u8) -> Option<AnalogKeyInfo2> {
        let report_id = self.get_report_id();
        let cmd: u8 = AnalogKeyInfo2::CMD.expect("No CMD found for AnalogKeyInfo2");
        let empty = AnalogKeyInfo2::empty();
        let response = self.request(report_id, SayoDeviceApi::ECHO, cmd, index, &empty);
        response.await
    }

    pub async fn set_analog_key_info2(
        &self,
        index: u8,
        key_info: &mut AnalogKeyInfo2,
    ) -> Option<AnalogKeyInfo2> {
        let report_id = self.get_report_id();
        let cmd: u8 = AnalogKeyInfo2::CMD.expect("No CMD found for AnalogKeyInfo2");
        let response = self.request(report_id, SayoDeviceApi::ECHO, cmd, index, key_info);
        response.await
    }

    pub async fn get_advanced_keys(&self) -> Vec<AdvancedKeyBinding> {
        let report_id = self.get_report_id();
        let cmd: u8 = AdvancedKeyBinding::CMD.expect("No CMD found for AdvancedKeyBinding");
        let response = self.request_all_index::<AdvancedKeyBinding>(report_id, cmd);
        response.await
    }

    pub async fn get_advanced_key(&self, index: u8) -> Option<AdvancedKeyBinding> {
        let report_id = self.get_report_id();
        let cmd: u8 = AdvancedKeyBinding::CMD.expect("No CMD found for AdvancedKeyBinding");
        let empty = AdvancedKeyBinding::empty();
        let response = self.request(report_id, SayoDeviceApi::ECHO, cmd, index, &empty);
        response.await
    }

    pub async fn set_advanced_key(
        &self,
        index: u8,
        key_info: &AdvancedKeyBinding,
    ) -> Option<AdvancedKeyBinding> {
        let report_id = self.get_report_id();
        let cmd: u8 = AdvancedKeyBinding::CMD.expect("No CMD found for AdvancedKeyBinding");
        let response = self.request(report_id, SayoDeviceApi::ECHO, cmd, index, key_info);
        response.await
    }

    pub async fn get_key_phyical_status(&self) -> Vec<u8> {
        let report_id = self.get_report_id();
        let cmd: u8 = 0x1E;
        let response = self
            .request(report_id, SayoDeviceApi::ECHO, cmd, 0, &ByteArray::empty())
            .await;
        match response {
            Some(data) => data.into_vec(),
            None => Vec::new(),
        }
    }

    pub async fn set_key_phyical_status(&self, status: Vec<u8>) -> bool {
        let report_id = self.get_report_id();
        let cmd: u8 = 0x1E;
        let response = self
            .request(
                report_id,
                SayoDeviceApi::ECHO,
                cmd,
                0,
                &ByteArray::new(RwBytes::new(status)),
            )
            .await;
        response.is_some()
    }

    pub async fn get_led_effect(&self) -> Option<LedEffect> {
        let report_id = self.get_report_id();
        let cmd: u8 = 0x26;
        self.request(report_id, SayoDeviceApi::ECHO, cmd, 0, &LedEffect::empty())
            .await
    }

    pub async fn set_led_effect(&self, effect: &LedEffect) -> Option<LedEffect> {
        let report_id = self.get_report_id();
        let cmd: u8 = 0x26;
        self.request(report_id, SayoDeviceApi::ECHO, cmd, 0, effect)
            .await
    }

    pub async fn get_led_index_count(&self) -> u8 {
        let report_id = self.get_report_id();
        const CMD: u8 = 0x27;

        self.request_all_index::<ByteArray>(report_id, CMD)
            .await
            .len() as u8
    }

    pub async fn get_led_status(&self, from_index: Option<u8>) -> Option<ByteArray> {
        let report_id = self.get_report_id();
        const CMD: u8 = 0x27;
        let bytes = ByteArray::empty();
        let response = self.request(
            report_id,
            SayoDeviceApi::ECHO,
            CMD,
            from_index.unwrap_or(0x00),
            &bytes,
        );
        response.await
    }

    pub async fn get_gamepad_cfg(&self) -> Option<GamePadCfg> {
        self.request(
            self.get_report_id(),
            SayoDeviceApi::ECHO,
            0x28,
            0,
            &GamePadCfg::empty(),
        )
        .await
    }

    pub async fn set_gamepad_cfg(&self, cfg: &GamePadCfg) -> Option<GamePadCfg> {
        self.request(self.get_report_id(), SayoDeviceApi::ECHO, 0x28, 0, cfg)
            .await
    }

    pub async fn get_ambient_led(&self, index: u8) -> Option<AmbientLED> {
        let report_id = self.get_report_id();
        const CMD: u8 = 0x2A;
        let empty = AmbientLED::empty();
        let response = self.request(report_id, SayoDeviceApi::ECHO, CMD, index, &empty);
        response.await
    }

    pub async fn set_ambient_led(&self, index: u8, ambient_led: &AmbientLED) -> Option<AmbientLED> {
        let report_id = self.get_report_id();
        const CMD: u8 = 0x2A;
        let response = self.request(report_id, SayoDeviceApi::ECHO, CMD, index, ambient_led);
        response.await
    }

    pub async fn get_ambient_leds(&self) -> Vec<AmbientLED> {
        let report_id = self.get_report_id();
        const CMD: u8 = 0x2A;
        self.request_all_index::<AmbientLED>(report_id, CMD).await
    }
}
