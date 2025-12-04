//#[frb(ignore)]
use crate::byte_converter::RwBytes;
use crate::structures::*;
use std::cell::Cell;
//use crate::debug_log;

// pub(in crate::api) fn debug_decode(header: &mut HidReportHeader, bytes: &[u8]) {
//     let bytes = RwBytes::new(bytes.to_vec());
//     match header.cmd(None).expect("Bad Report Header") {
//         0x00 => debug_log!("{:?}", DeviceInfo::new(bytes)),
//         0x01 => debug_log!("{:?}", StringContent::new(bytes)),
//         0x02 => debug_log!("{:?}", SystemInfo::new(bytes)),
//         0x03 => debug_log!("{:?}", OptionalBytes::new(bytes)),
//         0x04 => debug_log!("{:?}", WirelessConfig::new(bytes)),
//         0x0E => debug_log!("{:?}", AdvancedSystemConfig::new(bytes)),
//         0x10 => debug_log!("{:?}", KeyInfo::new(bytes)),
//         0x11 => debug_log!("{:?}", LEDInfo::new(bytes)),
//         0x12 => debug_log!("{:?}", ColorTable::new(bytes)),
//         0x13 => debug_log!("{:?}", TouchSensitivity::new(bytes)),
//         0x14 => debug_log!("{:?}", MagAxisInfo::new(bytes)),
//         0x1F => match header.index(None).expect("Bad Report Header") {
//             0x00 => debug_log!("{:?}", TriggerKeyboardHid::new(bytes)),
//             0x01 => debug_log!("{:?}", TriggerMouseHid::new(bytes)),
//             0x02 => debug_log!("{:?}", TriggerMeidaHid::new(bytes)),
//             _ => (),
//         },
//         0x20 => debug_log!("{:?}", DisplayAssets::new(bytes)),
//         0x21 => debug_log!("{:?}", LcdDrawData::new(bytes)),
//         0xFF => debug_log!("{:?}", BroadCast::new(bytes)),
//         _ => debug_log!("Unknown command: {:02X?}", header.cmd(None)),
//     }
// }

pub trait CodecableHidPackage {
    const CMD: Option<u8> = None;

    fn new(bytes: RwBytes) -> Self;

    fn into_vec(&self) -> Vec<u8>;

    fn empty() -> Self;

    fn deep_clone(&self) -> Self;
}

pub trait AddressableData {
    fn address(&self, value: Option<u32>) -> Option<u32>;

    fn data(&self, value: Option<Vec<u8>>) -> Option<Vec<u8>>;
}

impl CodecableHidPackage for ByteArray {
    const CMD: Option<u8> = None;

    fn new(bytes: RwBytes) -> Self {
        ByteArray { bytes }
    }

    fn into_vec(&self) -> Vec<u8> {
        self.bytes.clone().into_vec()
    }
    fn empty() -> Self {
        ByteArray {
            bytes: RwBytes::new(vec![]),
        }
    }

    fn deep_clone(&self) -> Self {
        let bytes = self.bytes.deep_clone();
        Self { bytes }
    }
}

impl CodecableHidPackage for HidReportHeader {
    const CMD: Option<u8> = None;

    fn new(bytes: RwBytes) -> Self {
        HidReportHeader { bytes }
    }

    fn into_vec(&self) -> Vec<u8> {
        self.bytes.clone().into_vec()
    }
    fn empty() -> Self {
        HidReportHeader {
            bytes: RwBytes::new(vec![]),
        }
    }

    fn deep_clone(&self) -> Self {
        let bytes = self.bytes.deep_clone();
        Self { bytes }
    }
}

impl CodecableHidPackage for StringContent {
    const CMD: Option<u8> = Some(0x01);

    fn new(bytes: RwBytes) -> Self {
        StringContent {
            bytes,
            encoding_byte: Cell::new(Some(0x03)),
        }
    }

    fn into_vec(&self) -> Vec<u8> {
        self.bytes.clone().into_vec()
    }
    fn empty() -> Self {
        StringContent {
            bytes: RwBytes::new(vec![]),
            encoding_byte: Cell::new(Some(0x03)),
        }
    }

    fn deep_clone(&self) -> Self {
        let bytes = self.bytes.deep_clone();
        Self {
            bytes,
            encoding_byte: Cell::new(self.encoding_byte.get()),
        }
    }
}

impl CodecableHidPackage for DeviceInfo {
    const CMD: Option<u8> = Some(0x00);

    fn new(bytes: RwBytes) -> Self {
        DeviceInfo { bytes }
    }

    fn into_vec(&self) -> Vec<u8> {
        self.bytes.clone().into_vec()
    }
    fn empty() -> Self {
        DeviceInfo {
            bytes: RwBytes::new(vec![]),
        }
    }

    fn deep_clone(&self) -> Self {
        let bytes = self.bytes.deep_clone();
        Self { bytes }
    }
}

impl CodecableHidPackage for SystemInfo {
    const CMD: Option<u8> = Some(0x02);

    fn new(bytes: RwBytes) -> Self {
        SystemInfo { bytes }
    }

    fn into_vec(&self) -> Vec<u8> {
        self.bytes.clone().into_vec()
    }
    fn empty() -> Self {
        SystemInfo {
            bytes: RwBytes::new(vec![]),
        }
    }

    fn deep_clone(&self) -> Self {
        let bytes = self.bytes.deep_clone();
        Self { bytes }
    }
}

impl CodecableHidPackage for OptionalBytes {
    const CMD: Option<u8> = Some(0x03);

    fn new(bytes: RwBytes) -> Self {
        OptionalBytes { bytes }
    }

    fn into_vec(&self) -> Vec<u8> {
        self.bytes.clone().into_vec()
    }
    fn empty() -> Self {
        OptionalBytes {
            bytes: RwBytes::new(vec![]),
        }
    }

    fn deep_clone(&self) -> Self {
        let bytes = self.bytes.deep_clone();
        Self { bytes }
    }
}

impl CodecableHidPackage for RFConfig {
    const CMD: Option<u8> = Some(0x04);

    fn new(bytes: RwBytes) -> Self {
        RFConfig { bytes }
    }

    fn into_vec(&self) -> Vec<u8> {
        self.bytes.clone().into_vec()
    }
    fn empty() -> Self {
        RFConfig {
            bytes: RwBytes::new(vec![]),
        }
    }

    fn deep_clone(&self) -> Self {
        let bytes = self.bytes.deep_clone();
        Self { bytes }
    }
}

impl CodecableHidPackage for AdvancedSystemConfig {
    const CMD: Option<u8> = Some(0x0E);

    fn new(bytes: RwBytes) -> Self {
        AdvancedSystemConfig { bytes }
    }

    fn into_vec(&self) -> Vec<u8> {
        self.bytes.clone().into_vec()
    }
    fn empty() -> Self {
        AdvancedSystemConfig {
            bytes: RwBytes::new(vec![]),
        }
    }

    fn deep_clone(&self) -> Self {
        let bytes = self.bytes.deep_clone();
        Self { bytes }
    }
}
impl CodecableHidPackage for KeyInfo {
    const CMD: Option<u8> = Some(0x10);

    fn new(bytes: RwBytes) -> Self {
        KeyInfo { bytes }
    }

    fn into_vec(&self) -> Vec<u8> {
        self.bytes.clone().into_vec()
    }
    fn empty() -> Self {
        KeyInfo {
            bytes: RwBytes::new(vec![]),
        }
    }

    fn deep_clone(&self) -> Self {
        let bytes = self.bytes.deep_clone();
        Self { bytes }
    }
}
impl CodecableHidPackage for KeyData {
    const CMD: Option<u8> = None;

    fn new(bytes: RwBytes) -> Self {
        KeyData { bytes }
    }

    fn into_vec(&self) -> Vec<u8> {
        self.bytes.clone().into_vec()
    }
    fn empty() -> Self {
        KeyData {
            bytes: RwBytes::new(vec![]),
        }
    }

    fn deep_clone(&self) -> Self {
        let bytes = self.bytes.deep_clone();
        Self { bytes }
    }
}

impl CodecableHidPackage for LEDInfo {
    const CMD: Option<u8> = Some(0x11);

    fn new(bytes: RwBytes) -> Self {
        LEDInfo { bytes }
    }

    fn into_vec(&self) -> Vec<u8> {
        self.bytes.clone().into_vec()
    }
    fn empty() -> Self {
        LEDInfo {
            bytes: RwBytes::new(vec![]),
        }
    }

    fn deep_clone(&self) -> Self {
        let bytes = self.bytes.deep_clone();
        Self { bytes }
    }
}

impl CodecableHidPackage for ColorTable {
    const CMD: Option<u8> = Some(0x12);

    fn new(bytes: RwBytes) -> Self {
        ColorTable { bytes }
    }

    fn into_vec(&self) -> Vec<u8> {
        self.bytes.clone().into_vec()
    }
    fn empty() -> Self {
        ColorTable {
            bytes: RwBytes::new(vec![]),
        }
    }

    fn deep_clone(&self) -> Self {
        let bytes = self.bytes.deep_clone();
        Self { bytes }
    }
}

impl CodecableHidPackage for TouchSensitivity {
    const CMD: Option<u8> = Some(0x13);

    fn new(bytes: RwBytes) -> Self {
        TouchSensitivity { bytes }
    }

    fn into_vec(&self) -> Vec<u8> {
        self.bytes.clone().into_vec()
    }
    fn empty() -> Self {
        TouchSensitivity {
            bytes: RwBytes::new(vec![]),
        }
    }

    fn deep_clone(&self) -> Self {
        let bytes = self.bytes.deep_clone();
        Self { bytes }
    }
}

impl CodecableHidPackage for AnalogKeyInfo {
    const CMD: Option<u8> = Some(0x14);

    fn new(bytes: RwBytes) -> Self {
        AnalogKeyInfo { bytes }
    }

    fn into_vec(&self) -> Vec<u8> {
        self.bytes.clone().into_vec()
    }
    fn empty() -> Self {
        AnalogKeyInfo {
            bytes: RwBytes::new(vec![]),
        }
    }

    fn deep_clone(&self) -> Self {
        let bytes = self.bytes.deep_clone();
        Self { bytes }
    }
}

impl CodecableHidPackage for SayoScript {
    const CMD: Option<u8> = Some(0x1A);

    fn new(bytes: RwBytes) -> Self {
        SayoScript { bytes }
    }

    fn into_vec(&self) -> Vec<u8> {
        self.bytes.clone().into_vec()
    }
    fn empty() -> Self {
        SayoScript {
            bytes: RwBytes::new(vec![]),
        }
    }

    fn deep_clone(&self) -> Self {
        let bytes = self.bytes.deep_clone();
        Self { bytes }
    }
}

impl CodecableHidPackage for SayoScriptPacket {
    const CMD: Option<u8> = Some(0x1A);

    fn new(bytes: RwBytes) -> Self {
        SayoScriptPacket { bytes }
    }

    fn into_vec(&self) -> Vec<u8> {
        self.bytes.clone().into_vec()
    }
    fn empty() -> Self {
        SayoScriptPacket {
            bytes: RwBytes::new(vec![]),
        }
    }

    fn deep_clone(&self) -> Self {
        let bytes = self.bytes.deep_clone();
        Self { bytes }
    }
}
impl AddressableData for SayoScriptPacket {
    fn address(&self, value: Option<u32>) -> Option<u32> {
        self.bytes.u32(0, value)
    }

    fn data(&self, value: Option<Vec<u8>>) -> Option<Vec<u8>> {
        self.bytes.vec(4, None, value)
    }
}

impl CodecableHidPackage for AnalogKeyInfo2 {
    const CMD: Option<u8> = Some(0x1C);

    fn new(bytes: RwBytes) -> Self {
        AnalogKeyInfo2 { bytes }
    }

    fn into_vec(&self) -> Vec<u8> {
        self.bytes.clone().into_vec()
    }
    fn empty() -> Self {
        AnalogKeyInfo2 {
            bytes: RwBytes::new(vec![]),
        }
    }

    fn deep_clone(&self) -> Self {
        let bytes = self.bytes.deep_clone();
        Self { bytes }
    }
}

impl CodecableHidPackage for AdvancedKeyBinding {
    const CMD: Option<u8> = Some(0x1D);

    fn new(bytes: RwBytes) -> Self {
        AdvancedKeyBinding { bytes }
    }

    fn into_vec(&self) -> Vec<u8> {
        self.bytes.clone().into_vec()
    }
    fn empty() -> Self {
        AdvancedKeyBinding {
            bytes: RwBytes::new(vec![]),
        }
    }

    fn deep_clone(&self) -> Self {
        let bytes = self.bytes.deep_clone();
        Self { bytes }
    }
}

impl CodecableHidPackage for TriggerKeyboardHid {
    const CMD: Option<u8> = Some(0x1F);

    fn new(bytes: RwBytes) -> Self {
        TriggerKeyboardHid { bytes }
    }

    fn into_vec(&self) -> Vec<u8> {
        self.bytes.clone().into_vec()
    }
    fn empty() -> Self {
        TriggerKeyboardHid {
            bytes: RwBytes::new(vec![]),
        }
    }

    fn deep_clone(&self) -> Self {
        let bytes = self.bytes.deep_clone();
        Self { bytes }
    }
}

impl CodecableHidPackage for TriggerMouseHid {
    const CMD: Option<u8> = Some(0x1F);

    fn new(bytes: RwBytes) -> Self {
        TriggerMouseHid { bytes }
    }

    fn into_vec(&self) -> Vec<u8> {
        self.bytes.clone().into_vec()
    }
    fn empty() -> Self {
        TriggerMouseHid {
            bytes: RwBytes::new(vec![]),
        }
    }

    fn deep_clone(&self) -> Self {
        let bytes = self.bytes.deep_clone();
        Self { bytes }
    }
}

impl CodecableHidPackage for TriggerMeidaHid {
    const CMD: Option<u8> = Some(0x1F);

    fn new(bytes: RwBytes) -> Self {
        TriggerMeidaHid { bytes }
    }

    fn into_vec(&self) -> Vec<u8> {
        self.bytes.clone().into_vec()
    }
    fn empty() -> Self {
        TriggerMeidaHid {
            bytes: RwBytes::new(vec![]),
        }
    }

    fn deep_clone(&self) -> Self {
        let bytes = self.bytes.deep_clone();
        Self { bytes }
    }
}

impl CodecableHidPackage for DisplayData {
    const CMD: Option<u8> = None;

    fn new(bytes: RwBytes) -> Self {
        DisplayData { bytes }
    }

    fn into_vec(&self) -> Vec<u8> {
        self.bytes.clone().into_vec()
    }
    fn empty() -> Self {
        DisplayData {
            bytes: RwBytes::new(vec![]),
        }
    }

    fn deep_clone(&self) -> Self {
        let bytes = self.bytes.deep_clone();
        Self { bytes }
    }
}

impl CodecableHidPackage for DisplayAssets {
    const CMD: Option<u8> = Some(0x20);

    fn new(bytes: RwBytes) -> Self {
        DisplayAssets { bytes }
    }

    fn into_vec(&self) -> Vec<u8> {
        self.bytes.clone().into_vec()
    }
    fn empty() -> Self {
        DisplayAssets {
            bytes: RwBytes::new(vec![]),
        }
    }

    fn deep_clone(&self) -> Self {
        let bytes = self.bytes.deep_clone();
        Self { bytes }
    }
}

impl CodecableHidPackage for DisplayAssetsPacket {
    const CMD: Option<u8> = Some(0x20);

    fn new(bytes: RwBytes) -> Self {
        DisplayAssetsPacket { bytes }
    }

    fn into_vec(&self) -> Vec<u8> {
        self.bytes.clone().into_vec()
    }
    fn empty() -> Self {
        DisplayAssetsPacket {
            bytes: RwBytes::new(vec![]),
        }
    }

    fn deep_clone(&self) -> Self {
        let bytes = self.bytes.deep_clone();
        Self { bytes }
    }
}
impl AddressableData for DisplayAssetsPacket {
    fn address(&self, value: Option<u32>) -> Option<u32> {
        self.bytes.u32(0, value)
    }

    fn data(&self, value: Option<Vec<u8>>) -> Option<Vec<u8>> {
        self.bytes.vec(4, None, value)
    }
}

impl CodecableHidPackage for LcdDrawData {
    const CMD: Option<u8> = None;

    fn new(bytes: RwBytes) -> Self {
        LcdDrawData { bytes }
    }

    fn into_vec(&self) -> Vec<u8> {
        self.bytes.clone().into_vec()
    }
    fn empty() -> Self {
        LcdDrawData {
            bytes: RwBytes::new(vec![]),
        }
    }

    fn deep_clone(&self) -> Self {
        let bytes = self.bytes.deep_clone();
        Self { bytes }
    }
}

impl CodecableHidPackage for ScreenBuffer {
    const CMD: Option<u8> = Some(0x25);

    fn new(bytes: RwBytes) -> Self {
        ScreenBuffer { bytes }
    }

    fn into_vec(&self) -> Vec<u8> {
        self.bytes.clone().into_vec()
    }
    fn empty() -> Self {
        ScreenBuffer {
            bytes: RwBytes::new(vec![]),
        }
    }

    fn deep_clone(&self) -> Self {
        let bytes = self.bytes.deep_clone();
        Self { bytes }
    }
}

impl CodecableHidPackage for LedEffect {
    const CMD: Option<u8> = Some(0x26);

    fn new(bytes: RwBytes) -> Self {
        LedEffect { bytes }
    }

    fn into_vec(&self) -> Vec<u8> {
        self.bytes.clone().into_vec()
    }
    fn empty() -> Self {
        LedEffect {
            bytes: RwBytes::new(vec![]),
        }
    }

    fn deep_clone(&self) -> Self {
        let bytes = self.bytes.deep_clone();
        Self { bytes }
    }
}

impl CodecableHidPackage for GamePadCfg {
    const CMD: Option<u8> = Some(0x28);

    fn new(bytes: RwBytes) -> Self {
        GamePadCfg { bytes }
    }

    fn into_vec(&self) -> Vec<u8> {
        self.bytes.clone().into_vec()
    }
    fn empty() -> Self {
        GamePadCfg {
            bytes: RwBytes::new(vec![]),
        }
    }

    fn deep_clone(&self) -> Self {
        let bytes = self.bytes.deep_clone();
        Self { bytes }
    }
}

impl CodecableHidPackage for AmbientLED {
    const CMD: Option<u8> = Some(0x2A);

    fn new(bytes: RwBytes) -> Self {
        AmbientLED { bytes }
    }

    fn into_vec(&self) -> Vec<u8> {
        self.bytes.clone().into_vec()
    }
    fn empty() -> Self {
        AmbientLED {
            bytes: RwBytes::new(vec![]),
        }
    }

    fn deep_clone(&self) -> Self {
        let bytes = self.bytes.deep_clone();
        Self { bytes }
    }
}

impl CodecableHidPackage for BroadCast {
    const CMD: Option<u8> = Some(0xFF);

    fn new(bytes: RwBytes) -> Self {
        BroadCast { bytes }
    }

    fn into_vec(&self) -> Vec<u8> {
        self.bytes.clone().into_vec()
    }
    fn empty() -> Self {
        BroadCast {
            bytes: RwBytes::new(vec![]),
        }
    }

    fn deep_clone(&self) -> Self {
        let bytes = self.bytes.deep_clone();
        Self { bytes }
    }
}
