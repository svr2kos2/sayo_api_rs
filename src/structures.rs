use encoding_rs::GB18030;
use std::cell::Cell;

use super::byte_converter::{Encoding, RwBytes};

#[repr(C)]
#[derive(Debug, Clone)]

pub struct ByteArray {
    pub bytes: RwBytes,
}
impl ByteArray {
    pub fn data(&self, value: Option<Vec<u8>>) -> Option<Vec<u8>> {
        self.bytes.vec(0, None, value)
    }
}

#[repr(C)]
#[derive(Debug, Clone)]

pub struct HidReportHeader {
    pub bytes: RwBytes,
}
impl HidReportHeader {
    pub fn report_id(&self, value: Option<u8>) -> Option<u8> {
        self.bytes.u8(0, value)
    }

    pub fn echo(&self, value: Option<u8>) -> Option<u8> {
        self.bytes.u8(1, value)
    }

    pub fn crc(&self, value: Option<u16>) -> Option<u16> {
        self.bytes.u16(2, value)
    }
    fn sta_len(&self, value: Option<(u8, u16)>) -> Option<(u8, u16)> {
        if let Some(value) = value {
            // write
            let sta = value.0;
            let len = value.1;
            let sta_len = ((sta as u16) << 10) | (len & 0x3FF);
            self.bytes.u16(4, Some(sta_len));
            return Some(value);
        } else {
            //read
            let sta_len = self.bytes.u16(4, None);
            if let Some(sta_len) = sta_len {
                let sta = (sta_len >> 10) as u8;
                let len = sta_len & 0x03FF;
                return Some((sta, len));
            } else {
                return None;
            }
        }
    }

    pub fn len(&self, value: Option<u16>) -> Option<u16> {
        if let Some(value) = value {
            // write
            let (sta, _) = self
                .sta_len(None)
                .expect("sta_len not found in HidReportHeader");
            self.sta_len(Some((sta, value)));
            return Some(value);
        } else {
            //read
            let (_, len) = self
                .sta_len(None)
                .expect("sta_len not found in HidReportHeader");
            return Some(len);
        }
    }

    pub fn status(&self, value: Option<u8>) -> Option<u8> {
        if let Some(value) = value {
            // write
            let (_, len) = self
                .sta_len(None)
                .expect("sta_len not found in HidReportHeader");
            self.sta_len(Some((value, len)));
            return Some(value);
        } else {
            //read
            let (sta, _) = self
                .sta_len(None)
                .expect("sta_len not found in HidReportHeader");
            return Some(sta);
        }
    }

    pub fn cmd(&self, value: Option<u8>) -> Option<u8> {
        self.bytes.u8(6, value)
    }

    pub fn index(&self, value: Option<u8>) -> Option<u8> {
        self.bytes.u8(7, value)
    }
}

#[repr(C)]
#[derive(Debug, Clone)]

pub struct StringContent {
    pub encoding_byte: Cell<Option<u8>>,
    pub bytes: RwBytes,
}
impl StringContent {
    pub fn create(bytes: RwBytes) -> Self {
        let encoding = bytes
            .u8(0, None)
            .expect("encoding not found in StringContent");
        StringContent {
            encoding_byte: Cell::new(Some(encoding)),
            bytes: bytes
                .ref_at(1, bytes.len() - 1)
                .expect("bytes not found in StringContent"),
        }
    }

    pub fn bytes_len(&self) -> usize {
        self.bytes.len()
    }

    pub fn get_str_len(encoding: u8, str: &String) -> Vec<u8> {
        let mut res: Vec<u8> = Vec::new();
        match encoding {
            0x02 => {
                //gbk
                let mut i = 0;
                while i < str.chars().count() {
                    let char = &str
                        .chars()
                        .nth(i)
                        .expect("char not found in StringContent::get_str_len")
                        .to_string();
                    res.push(GB18030.encode(char).0.to_vec().len() as u8);
                    i += 1;
                }
            }
            0x03 => {
                //utf-16
                let mut i = 0;
                while i < str.chars().count() {
                    let char = &str
                        .chars()
                        .nth(i)
                        .expect("char not found in StringContent::get_str_len")
                        .to_string();
                    res.push(
                        char.encode_utf16()
                            .flat_map(|c| c.to_le_bytes().to_vec())
                            .collect::<Vec<u8>>()
                            .len() as u8,
                    );
                    i += 1;
                }
            }
            _ => {}
        }
        res
    }

    pub fn len(&self) -> u16 {
        let encoding = match self.encoding_byte.get() {
            Some(encoding) => encoding,
            None => return 0,
        };
        let str = match self.str(None) {
            Some(str) => str,
            None => return 0,
        };
        let lens = StringContent::get_str_len(encoding, &str);
        let mut len = 0;
        for l in lens {
            len += l as u16;
        }
        len
    }

    pub fn str(&self, value: Option<String>) -> Option<String> {
        let encoding = match self.encoding_byte.get() {
            Some(encoding) => encoding,
            None => return None,
        };
        self.bytes.str(encoding, 0, value)
    }

    pub fn encoding(&self, value: Option<u8>) -> Option<u8> {
        if let Some(new_encoding) = value {
            let str = match self.str(None) {
                Some(str) => str,
                None => return Some(new_encoding),
            };
            self.encoding_byte.set(Some(new_encoding));
            let lens = StringContent::get_str_len(new_encoding, &str);
            let limit = self.bytes_len();
            let mut truncated_str = String::new();
            let mut truncated_len = 0;
            let mut i = 0;
            for len in lens {
                if (truncated_len + len) as usize > limit {
                    break;
                }
                truncated_str.push(
                    str.chars()
                        .nth(i)
                        .expect("char not found in StringContent::encoding"),
                );
                truncated_len += len;
                i += 1;
            }
            self.bytes.str(new_encoding, 0, Some(truncated_str));
            return Some(new_encoding);
        } else {
            return self.encoding_byte.get();
        }
    }
}

#[repr(C)]
#[derive(Debug, Clone)]

pub struct DeviceInfo {
    pub bytes: RwBytes,
}
impl DeviceInfo {
    pub fn model_code(&self, value: Option<u16>) -> Option<u16> {
        self.bytes.u16(0, value)
    }

    pub fn ver(&self, value: Option<u16>) -> Option<u16> {
        self.bytes.u16(2, value)
    }

    pub fn usb0_ori(&self, value: Option<u8>) -> Option<u8> {
        self.bytes.u8(4, value)
    }

    pub fn usb0_offset(&self, value: Option<u8>) -> Option<u8> {
        self.bytes.u8(5, value)
    }

    pub fn usb1_ori(&self, value: Option<u8>) -> Option<u8> {
        self.bytes.u8(6, value)
    }

    pub fn usb1_offset(&self, value: Option<u8>) -> Option<u8> {
        self.bytes.u8(7, value)
    }

    pub fn batt_lv(&self, value: Option<u8>) -> Option<u8> {
        self.bytes.u8(8, value)
    }

    pub fn key_fn(&self, value: Option<u8>) -> Option<u8> {
        self.bytes.u8(9, value)
    }

    pub fn cpu_load_1s(&self, value: Option<u8>) -> Option<u8> {
        self.bytes.u8(10, value)
    }

    pub fn cpu_load_1ms(&self, value: Option<u8>) -> Option<u8> {
        self.bytes.u8(11, value)
    }

    pub fn api_list(&self, value: Option<Vec<u8>>) -> Option<Vec<u8>> {
        self.bytes.vec(12, None, value)
    }
}

#[repr(C)]
#[derive(Debug, Clone)]

pub struct SystemInfo {
    pub bytes: RwBytes,
}
impl SystemInfo {
    pub fn lcd_width(&self, value: Option<u16>) -> Option<u16> {
        self.bytes.u16(0, value)
    }

    pub fn lcd_height(&self, value: Option<u16>) -> Option<u16> {
        self.bytes.u16(2, value)
    }

    pub fn lcd_refresh_rate(&self, value: Option<u8>) -> Option<u8> {
        self.bytes.u8(4, value)
    }

    pub fn cfg_selection(&self, value: Option<u8>) -> Option<u8> {
        let byte = self
            .bytes
            .u8(5, None)
            .expect("cfg_selection not found in SystemInfo");
        match value {
            Some(value) => {
                //write
                self.bytes.u8(5, Some((byte & 0xF0) | (value & 0x0F)));
                return Some(value);
            }
            None => {
                //read
                return Some(byte & 0x0F);
            }
        }
    }

    pub fn cfg_range(&self) -> Option<u8> {
        let byte = self.bytes.u8(5, None)?;
        Some(byte >> 4)
    }

    pub fn sys_time_ms(&self, value: Option<u16>) -> Option<u16> {
        self.bytes.u16(6, value)
    }

    pub fn sys_time_s(&self, value: Option<u32>) -> Option<u32> {
        self.bytes.u32(8, value)
    }

    pub fn vid(&self, value: Option<u16>) -> Option<u16> {
        self.bytes.u16(12, value)
    }

    pub fn pid(&self, value: Option<u16>) -> Option<u16> {
        self.bytes.u16(14, value)
    }

    pub fn cpu_load_1m(&self, value: Option<u8>) -> Option<u8> {
        self.bytes.u8(16, value)
    }

    pub fn cpu_load_5m(&self, value: Option<u8>) -> Option<u8> {
        self.bytes.u8(17, value)
    }

    pub fn cpu_freq(&self, value: Option<u32>) -> Option<u32> {
        self.bytes.u32(18, value)
    }

    pub fn hclk_freq(&self, value: Option<u32>) -> Option<u32> {
        self.bytes.u32(22, value)
    }
    pub fn pclk1_freq(&self, value: Option<u32>) -> Option<u32> {
        self.bytes.u32(26, value)
    }
    pub fn pclk2_freq(&self, value: Option<u32>) -> Option<u32> {
        self.bytes.u32(30, value)
    }
    pub fn adc0_freq(&self, value: Option<u32>) -> Option<u32> {
        self.bytes.u32(34, value)
    }
    pub fn adc1_freq(&self, value: Option<u32>) -> Option<u32> {
        self.bytes.u32(38, value)
    }
}

#[repr(C)]
#[derive(Debug, Clone)]

pub struct DeviceConfig {
    pub bytes: RwBytes,
}
impl DeviceConfig {
    pub fn display_width(&self, value: Option<u16>) -> Option<u16> {
        self.bytes.u16(0, value)
    }

    pub fn display_height(&self, value: Option<u16>) -> Option<u16> {
        self.bytes.u16(2, value)
    }

    pub fn dev_feature_selection_0(&self, value: Option<u8>) -> Option<u8> {
        self.bytes.u8(4, value)
    }

    pub fn dev_feature_selection_0_selectable(&self, value: Option<u8>) -> Option<u8> {
        self.bytes.u8(5, value)
    }

    pub fn enc_channel(&self, value: Option<u8>) -> Option<u8> {
        self.bytes.u8(6, value)
    }

    pub fn enc_channel_selectable(&self, value: Option<u8>) -> Option<u8> {
        self.bytes.u8(7, value)
    }

    pub fn key_release_delay(&self, value: Option<u8>) -> Option<u8> {
        self.bytes.u8(8, value)
    }

    pub fn key_release_delay_range(&self, value: Option<u8>) -> Option<u8> {
        self.bytes.u8(9, value)
    }

    pub fn lcd_timeout(&self, value: Option<u8>) -> Option<u8> {
        self.bytes.u8(10, value)
    }

    pub fn lcd_timeout_range(&self, value: Option<u8>) -> Option<u8> {
        self.bytes.u8(11, value)
    }

    pub fn hid_feature_selection_0(&self, value: Option<u8>) -> Option<u8> {
        self.bytes.u8(12, value)
    }

    pub fn hid_feature_selection_0_selectable(&self, value: Option<u8>) -> Option<u8> {
        self.bytes.u8(13, value)
    }

    pub fn hid_feature_selection_1(&self, value: Option<u8>) -> Option<u8> {
        self.bytes.u8(14, value)
    }

    pub fn hid_feature_selection_1_selectable(&self, value: Option<u8>) -> Option<u8> {
        self.bytes.u8(15, value)
    }

    pub fn keyboard_layout(&self, value: Option<u8>) -> Option<u8> {
        self.bytes.u8(16, value)
    }

    pub fn keyboard_layout_select_range(&self, value: Option<u8>) -> Option<u8> {
        self.bytes.u8(17, value)
    }

    pub fn keyboard_language(&self, value: Option<u8>) -> Option<u8> {
        self.bytes.u8(18, value)
    }

    pub fn keyboard_language_select_range(&self, value: Option<u8>) -> Option<u8> {
        self.bytes.u8(19, value)
    }

    pub fn dev_feature_selection_1(&self, value: Option<u8>) -> Option<u8> {
        self.bytes.u8(20, value)
    }

    pub fn dev_feature_selection_1_selectable(&self, value: Option<u8>) -> Option<u8> {
        self.bytes.u8(21, value)
    }

    pub fn usb_speed(&self, value: Option<u8>) -> Option<u8> {
        self.bytes.u8(22, value)
    }

    pub fn usb_speed_select_range(&self, value: Option<u8>) -> Option<u8> {
        self.bytes.u8(23, value)
    }

    pub fn key_press_delay(&self, value: Option<u16>) -> Option<u16> {
        self.bytes.u16(24, value)
    }

    pub fn key_press_delay_range(&self, value: Option<u16>) -> Option<u16> {
        self.bytes.u16(26, value)
    }

    pub fn display_width_negative(&self, value: Option<u16>) -> Option<u16> {
        self.bytes.u16(28, value)
    }

    pub fn display_height_negative(&self, value: Option<u16>) -> Option<u16> {
        self.bytes.u16(30, value)
    }

    pub fn hk_multisampling(&self, value: Option<u8>) -> Option<u8> {
        self.bytes.u8(32, value)
    }

    pub fn hk_multisampling_select_range(&self, value: Option<u8>) -> Option<u8> {
        self.bytes.u8(33, value)
    }

    pub fn led_dimming_time(&self, value: Option<u8>) -> Option<u8> {
        self.bytes.u8(34, value)
    }

    pub fn led_dimming_time_range(&self, value: Option<u8>) -> Option<u8> {
        self.bytes.u8(35, value)
    }

    pub fn led_turn_off_time(&self, value: Option<u8>) -> Option<u8> {
        self.bytes.u8(36, value)
    }

    pub fn led_turn_off_time_range(&self, value: Option<u8>) -> Option<u8> {
        self.bytes.u8(37, value)
    }
}

#[repr(C)]
#[derive(Debug, Clone)]

pub struct RFConfig {
    pub bytes: RwBytes,
}
impl RFConfig {
    pub fn rf_addr(&self, value: Option<u32>) -> Option<u32> {
        self.bytes.u32(0, value)
    }

    pub fn rf_mode(&self, value: Option<u8>) -> Option<u8> {
        self.bytes.u8(4, value)
    }

    pub fn rf_mode_select_range(&self, value: Option<u8>) -> Option<u8> {
        self.bytes.u8(5, value)
    }

    pub fn rf_ch(&self, value: Option<u8>) -> Option<u8> {
        self.bytes.u8(6, value)
    }

    pub fn rf_ch_range(&self, value: Option<u8>) -> Option<u8> {
        self.bytes.u8(7, value)
    }

    pub fn rf_gap(&self, value: Option<u8>) -> Option<u8> {
        self.bytes.u8(8, value)
    }

    pub fn rf_gap_range(&self, value: Option<u8>) -> Option<u8> {
        self.bytes.u8(9, value)
    }

    pub fn rf_time_out(&self, value: Option<u8>) -> Option<u8> {
        self.bytes.u8(10, value)
    }

    pub fn rf_time_out_range(&self, value: Option<u8>) -> Option<u8> {
        self.bytes.u8(11, value)
    }

    pub fn rf_sleep_time(&self, value: Option<u8>) -> Option<u8> {
        self.bytes.u8(12, value)
    }

    pub fn rf_sleep_time_range(&self, value: Option<u8>) -> Option<u8> {
        self.bytes.u8(13, value)
    }

    pub fn rf_led_time(&self, value: Option<u8>) -> Option<u8> {
        self.bytes.u8(14, value)
    }

    pub fn rf_led_time_range(&self, value: Option<u8>) -> Option<u8> {
        self.bytes.u8(15, value)
    }
}

#[repr(C)]
#[derive(Debug, Clone)]

pub struct KeyData {
    pub bytes: RwBytes,
}
impl KeyData {
    const SIZE: usize = 8;

    pub fn key_mode(&self, value: Option<u8>) -> Option<u8> {
        self.bytes.u8(0, value)
    }

    pub fn key_opt0(&self, value: Option<u8>) -> Option<u8> {
        self.bytes.u8(1, value)
    }

    pub fn key_opt1(&self, value: Option<u8>) -> Option<u8> {
        self.bytes.u8(2, value)
    }

    pub fn key_opt2(&self, value: Option<u8>) -> Option<u8> {
        self.bytes.u8(3, value)
    }

    pub fn key_val(&self, value: Option<Vec<u8>>) -> Option<Vec<u8>> {
        self.bytes.vec(4, Some(4), value)
    }
}

#[repr(C)]
#[derive(Debug, Clone)]

pub struct KeyInfo {
    pub bytes: RwBytes,
}
impl KeyInfo {
    pub fn valid(&self, value: Option<u8>) -> Option<u8> {
        self.bytes.u8(0, value)
    }

    pub fn key_class(&self, value: Option<u8>) -> Option<u8> {
        self.bytes.u8(1, value)
    }

    pub fn reserve0(&self, value: Option<u16>) -> Option<u16> {
        self.bytes.u16(2, value)
    }

    pub fn key_site_x(&self, value: Option<u16>) -> Option<u16> {
        self.bytes.u16(4, value)
    }

    pub fn key_site_y(&self, value: Option<u16>) -> Option<u16> {
        self.bytes.u16(6, value)
    }

    pub fn key_width(&self, value: Option<u16>) -> Option<u16> {
        self.bytes.u16(8, value)
    }

    pub fn key_height(&self, value: Option<u16>) -> Option<u16> {
        self.bytes.u16(10, value)
    }

    pub fn fillet_angle(&self, value: Option<u16>) -> Option<u16> {
        self.bytes.u16(12, value)
    }

    pub fn reserve1(&self, value: Option<u16>) -> Option<u16> {
        self.bytes.u16(14, value)
    }

    pub fn key_fn(&self) -> Option<Vec<KeyData>> {
        let mut i = 16;
        let mut res: Vec<KeyData> = Vec::new();
        while i + KeyData::SIZE <= self.bytes.len() {
            let bytes = match self.bytes.ref_at(i, KeyData::SIZE) {
                Some(bytes) => bytes,
                None => break,
            };
            res.push(KeyData { bytes });
            i += KeyData::SIZE;
        }
        Some(res)
    }

    pub fn key_data(&self, index: u32, value: Option<KeyData>) -> Option<KeyData> {
        if index >= 4 {
            return None;
        }
        let i = 16 + index as usize * KeyData::SIZE;
        if value.is_some() {
            let data = value.clone().expect("value not found in KeyInfo::key_data");
            self.bytes
                .vec(i, Some(KeyData::SIZE), Some(data.bytes.into_vec()));
            return value;
        } else {
            let bytes = match self.bytes.ref_at(i, KeyData::SIZE) {
                Some(bytes) => bytes,
                None => return None,
            };
            Some(KeyData { bytes })
        }
    }
}

#[repr(C)]
#[derive(Debug, Clone)]

pub struct LedData {
    pub bytes: RwBytes,
}
impl LedData {
    const SIZE: usize = 8;

    pub fn led_color_speed(&self, value: Option<(u8, u8, u8)>) -> Option<(u8, u8, u8)> {
        if let Some(value) = value {
            // write
            let led_mode = value.0;
            let color_mod = value.1;
            let speed = value.2;
            let led_color_speed =
                (led_mode as u8) | ((color_mod as u8) << 4) | ((speed as u8) << 6);
            self.bytes.u8(0, Some(led_color_speed));
            return Some(value);
        } else {
            //read
            let led_color_speed = self.bytes.u8(0, None);
            if let Some(led_color_speed) = led_color_speed {
                let led_mode = (led_color_speed & 0x0F) as u8;
                let color_mod = ((led_color_speed >> 4) & 0x03) as u8;
                let speed = (led_color_speed >> 6) as u8;
                return Some((led_mode, color_mod, speed));
            } else {
                return None;
            }
        }
    }

    pub fn led_mode(&self, value: Option<u8>) -> Option<u8> {
        if let Some(value) = value {
            // write
            let (_, color_mod, speed) = self
                .led_color_speed(None)
                .expect("led_color_speed not found in LedData");
            self.led_color_speed(Some((value, color_mod, speed)));
            return Some(value);
        } else {
            //read
            let (led_mode, _, _) = self
                .led_color_speed(None)
                .expect("led_color_speed not found in LedData");
            return Some(led_mode);
        }
    }

    pub fn color_mode(&self, value: Option<u8>) -> Option<u8> {
        if let Some(value) = value {
            // write
            let (led_mode, _, speed) = self
                .led_color_speed(None)
                .expect("led_color_speed not found in LedData");
            self.led_color_speed(Some((led_mode, value, speed)));
            return Some(value);
        } else {
            //read
            let (_, color_mod, _) = self
                .led_color_speed(None)
                .expect("led_color_speed not found in LedData");
            return Some(color_mod);
        }
    }

    pub fn speed(&self, value: Option<u8>) -> Option<u8> {
        if let Some(value) = value {
            // write
            let (led_mode, color_mod, _) = self
                .led_color_speed(None)
                .expect("led_color_speed not found in LedData");
            self.led_color_speed(Some((led_mode, color_mod, value)));
            return Some(value);
        } else {
            //read
            let (_, _, speed) = self
                .led_color_speed(None)
                .expect("led_color_speed not found in LedData");
            return Some(speed);
        }
    }

    pub fn event(&self, value: Option<u8>) -> Option<u8> {
        self.bytes.u8(1, value)
    }

    pub fn lighting_time(&self, value: Option<u8>) -> Option<u8> {
        self.bytes.u8(2, value)
    }

    pub fn dark_time(&self, value: Option<u8>) -> Option<u8> {
        self.bytes.u8(3, value)
    }

    pub fn r(&self, value: Option<u8>) -> Option<u8> {
        self.bytes.u8(4, value)
    }

    pub fn g(&self, value: Option<u8>) -> Option<u8> {
        self.bytes.u8(5, value)
    }

    pub fn b(&self, value: Option<u8>) -> Option<u8> {
        self.bytes.u8(6, value)
    }

    pub fn color(&self, value: Option<(u8, u8, u8)>) -> Option<(u8, u8, u8)> {
        if let Some(value) = value {
            // write
            let r = value.0;
            let g = value.1;
            let b = value.2;
            self.r(Some(r));
            self.g(Some(g));
            self.b(Some(b));
            return Some(value);
        } else {
            //read
            let r = self.r(None);
            let g = self.g(None);
            let b = self.b(None);
            if r.is_some() && g.is_some() && b.is_some() {
                return Some((
                    r.expect("Can not get r in LedData::color"),
                    g.expect("Can not get g in LedData::color"),
                    b.expect("Can not get b in LedData::color"),
                ));
            } else {
                return None;
            }
        }
    }

    pub fn color_table_number(&self, value: Option<u8>) -> Option<u8> {
        self.bytes.u8(7, value)
    }
}

#[repr(C)]
#[derive(Debug, Clone)]

pub struct LEDInfo {
    pub bytes: RwBytes,
}
impl LEDInfo {
    pub fn valid(&self, value: Option<u8>) -> Option<u8> {
        self.bytes.u8(0, value)
    }

    pub fn led_class(&self, value: Option<u8>) -> Option<u8> {
        self.bytes.u8(1, value)
    }

    pub fn reserve0(&self, value: Option<u16>) -> Option<u16> {
        self.bytes.u16(2, value)
    }

    pub fn led_site_x(&self, value: Option<u16>) -> Option<u16> {
        self.bytes.u16(4, value)
    }

    pub fn led_site_y(&self, value: Option<u16>) -> Option<u16> {
        self.bytes.u16(6, value)
    }

    pub fn led_width(&self, value: Option<u16>) -> Option<u16> {
        self.bytes.u16(8, value)
    }

    pub fn led_height(&self, value: Option<u16>) -> Option<u16> {
        self.bytes.u16(10, value)
    }

    pub fn fillet_angle(&self, value: Option<u16>) -> Option<u16> {
        self.bytes.u16(12, value)
    }

    pub fn reserve1(&self, value: Option<u16>) -> Option<u16> {
        self.bytes.u16(14, value)
    }

    pub fn led_fn(&self) -> Option<Vec<LedData>> {
        let mut i = 16;
        let mut res: Vec<LedData> = Vec::new();
        while i + LedData::SIZE <= self.bytes.len() {
            let bytes = match self.bytes.ref_at(i, LedData::SIZE) {
                Some(bytes) => bytes,
                None => break,
            };
            res.push(LedData { bytes });
            i += LedData::SIZE;
        }
        Some(res)
    }
}

#[repr(C)]
#[derive(Debug, Clone)]

pub struct SayoColorData {
    pub bytes: RwBytes,
}
impl SayoColorData {
    const SIZE: usize = 3;

    pub fn r(&self, value: Option<u8>) -> Option<u8> {
        self.bytes.u8(0, value)
    }

    pub fn g(&self, value: Option<u8>) -> Option<u8> {
        self.bytes.u8(1, value)
    }

    pub fn b(&self, value: Option<u8>) -> Option<u8> {
        self.bytes.u8(2, value)
    }
}

#[repr(C)]
#[derive(Debug, Clone)]

pub struct ColorTable {
    pub bytes: RwBytes,
}
impl ColorTable {
    pub fn number_of_colors(&self, value: Option<u8>) -> Option<u8> {
        self.bytes.u8(0, value)
    }

    pub fn reserve0(&self, value: Option<u8>) -> Option<u8> {
        self.bytes.u8(1, value)
    }

    pub fn data(&self) -> Option<Vec<SayoColorData>> {
        let mut i = 2;
        let mut res: Vec<SayoColorData> = Vec::new();
        while i + SayoColorData::SIZE <= self.bytes.len() {
            let bytes = match self.bytes.ref_at(i, SayoColorData::SIZE) {
                Some(bytes) => bytes,
                None => break,
            };
            res.push(SayoColorData { bytes });
            i += SayoColorData::SIZE;
        }
        Some(res)
    }
}

#[repr(C)]
#[derive(Debug, Clone)]

pub struct TouchSensitivity {
    pub bytes: RwBytes,
}
impl TouchSensitivity {
    pub fn trigger_value(&self, value: Option<u16>) -> Option<u16> {
        self.bytes.u16(0, value)
    }

    pub fn trigger_value_range(&self, value: Option<u16>) -> Option<u16> {
        self.bytes.u16(2, value)
    }

    pub fn raw_data(&self, value: Option<u16>) -> Option<u16> {
        self.bytes.u16(4, value)
    }

    pub fn zero_pos(&self, value: Option<u16>) -> Option<u16> {
        self.bytes.u16(6, value)
    }
}

#[repr(C)]
#[derive(Debug, Clone)]

pub struct AnalogKeyInfo {
    pub bytes: RwBytes,
}
impl AnalogKeyInfo {
    fn _codecode_level(&self, offset: usize, level: Option<u16>) -> Option<u16> {
        // 0.01mm
        match level {
            Some(level) => {
                // write
                let levelu8: u8 = match level > 100 {
                    true => 100 + ((level - 100) / 2) as u8,
                    false => level as u8,
                };
                let res = self.bytes.u8(offset, Some(levelu8));
                return match res {
                    Some(_) => Some(level),
                    None => None,
                };
            }
            None => {
                // read
                match self.bytes.u8(offset, None) {
                    Some(level) => {
                        let level = match level > 100 {
                            true => 100 + (level as u16 - 100) * 2,
                            false => level as u16,
                        };
                        return Some(level);
                    }
                    None => {
                        return None;
                    }
                }
            }
        }
    }

    pub fn raw_level(&self, value: Option<u8>) -> Option<u8> {
        self.bytes.u8(0, value)
    }

    pub fn polar(&self, value: Option<u8>) -> Option<u8> {
        self.bytes.u8(1, value)
    }

    pub fn trigger_level(&self, value: Option<u16>) -> Option<u16> {
        self._codecode_level(2, value)
    }

    pub fn release_level(&self, value: Option<u16>) -> Option<u16> {
        self._codecode_level(3, value)
    }

    pub fn rapid_trigger_top(&self, value: Option<u16>) -> Option<u16> {
        self._codecode_level(4, value)
    }

    pub fn rapid_trigger_area(&self, value: Option<u16>) -> Option<u16> {
        self._codecode_level(5, value)
    }

    pub fn rapid_trigger_level(&self, value: Option<u16>) -> Option<u16> {
        self._codecode_level(6, value)
    }

    pub fn rapid_release_level(&self, value: Option<u16>) -> Option<u16> {
        self._codecode_level(7, value)
    }

    pub fn raw_data(&self, value: Option<u16>) -> Option<u16> {
        self.bytes.u16(8, value)
    }

    pub fn zero_pos(&self, value: Option<u16>) -> Option<u16> {
        self.bytes.u16(10, value)
    }

    pub fn raw_um(&self, value: Option<u16>) -> Option<u16> {
        self.bytes.u16(12, value)
    }

    pub fn reserve(&self, value: Option<u16>) -> Option<u16> {
        self.bytes.u16(14, value)
    }

    pub fn level_data(&self, value: Option<Vec<u8>>) -> Option<Vec<u8>> {
        self.bytes.vec(16, None, value)
    }
}

#[repr(C)]
#[derive(Debug, Clone)]

pub struct SayoScriptContent {
    pub bytes: RwBytes,
}
impl SayoScriptContent {
    pub fn len(&self) -> usize {
        return self.bytes.len();
    }
}

#[repr(C)]
#[derive(Debug, Clone)]

pub struct SayoScriptPacket {
    pub bytes: RwBytes,
}

#[repr(C)]
#[derive(Debug, Clone)]

pub struct AnalogKeyInfo2 {
    pub bytes: RwBytes,
}
impl AnalogKeyInfo2 {
    pub fn from_v1(v1: &mut AnalogKeyInfo, firmware_version: u16) -> Self {
        let bytes = RwBytes::new(vec![0; 104]);
        let res = AnalogKeyInfo2 { bytes };
        res.raw_data(v1.raw_data(None));
        res.raw_um(if firmware_version < 120 {
            Some(
                (v1.raw_level(None)
                    .expect("Can not get raw_level in AnalogKeyInfo2::from_v1")
                    as u16)
                    * 50,
            )
        } else {
            v1.raw_um(None)
        });
        res.zero_pos(v1.zero_pos(None));
        res.max_value(match v1.polar(None) {
            Some(polar) => match polar {
                0xFF => Some(0xFFFF),
                _ => Some(polar as u16),
            },
            None => None,
        });
        res.stroke(Some(80));
        res.rt_mode(Some(0x01));
        res.switch_type(Some(0x00));
        res.trigger_level(if firmware_version < 120 {
            match v1.bytes.u8(2, None) {
                Some(value) => Some((value as u16) * 50),
                None => None,
            }
        } else {
            v1.trigger_level(None)
        });
        res.release_level(if firmware_version < 120 {
            match v1.bytes.u8(3, None) {
                Some(value) => Some((value as u16) * 50),
                None => None,
            }
        } else {
            v1.release_level(None)
        });
        res.rapid_trigger_top(if firmware_version < 120 {
            match v1.bytes.u8(4, None) {
                Some(value) => Some((value as u16) * 50),
                None => None,
            }
        } else {
            v1.rapid_trigger_top(None)
        });
        res.rapid_trigger_area(if firmware_version < 120 {
            match v1.bytes.u8(5, None) {
                Some(value) => Some((value as u16) * 50),
                None => None,
            }
        } else {
            v1.rapid_trigger_area(None)
        });
        res.rapid_trigger_level(if firmware_version < 120 {
            match v1.bytes.u8(6, None) {
                Some(value) => Some((value as u16) * 50),
                None => None,
            }
        } else {
            v1.rapid_trigger_level(None)
        });
        res.rapid_release_level(if firmware_version < 120 {
            match v1.bytes.u8(7, None) {
                Some(value) => Some((value as u16) * 50),
                None => None,
            }
        } else {
            v1.rapid_release_level(None)
        });
        res.bytes
            .vec(24, Some(80), v1.bytes.vec(16, Some(80), None));
        return res;
    }
    pub fn to_v1(&self, firmware_version: u16) -> AnalogKeyInfo {
        let res = AnalogKeyInfo {
            bytes: RwBytes::new(vec![0; 96]),
        };
        res.raw_data(self.raw_data(None));
        res.raw_um(if firmware_version < 120 {
            Some(
                (self
                    .raw_um(None)
                    .expect("Can not get raw_um in AnalogKeyInfo2::tp_v1")
                    / 50) as u16,
            )
        } else {
            self.raw_um(None)
        });
        res.zero_pos(self.zero_pos(None));
        res.polar(match self.max_value(None) {
            Some(max_value) => match max_value {
                0xFFFF => Some(0xFF),
                _ => Some(max_value as u8),
            },
            None => None,
        });
        if firmware_version < 120 {
            res.bytes.u8(
                2,
                match self.trigger_level(None) {
                    Some(value) => Some((value / 50) as u8),
                    None => None,
                },
            );
            res.bytes.u8(
                3,
                match self.release_level(None) {
                    Some(value) => Some((value / 50) as u8),
                    None => None,
                },
            );
            res.bytes.u8(
                4,
                match self.rapid_trigger_top(None) {
                    Some(value) => Some((value / 50) as u8),
                    None => None,
                },
            );
            res.bytes.u8(
                5,
                match self.rapid_trigger_area(None) {
                    Some(value) => Some((value / 50) as u8),
                    None => None,
                },
            );
            res.bytes.u8(
                6,
                match self.rapid_trigger_level(None) {
                    Some(value) => Some((value / 50) as u8),
                    None => None,
                },
            );
            res.bytes.u8(
                7,
                match self.rapid_release_level(None) {
                    Some(value) => Some((value / 50) as u8),
                    None => None,
                },
            );
        } else {
            res.trigger_level(self.trigger_level(None));
            res.release_level(self.release_level(None));
            res.rapid_trigger_top(self.rapid_trigger_top(None));
            res.rapid_trigger_area(self.rapid_trigger_area(None));
            res.rapid_trigger_level(self.rapid_trigger_level(None));
            res.rapid_release_level(self.rapid_release_level(None));
        }
        if self.bytes.len() >= 104 {
            res.bytes
                .vec(16, Some(80), self.bytes.vec(24, Some(80), None));
        }
        return res;
    }

    pub fn raw_data(&self, value: Option<u16>) -> Option<u16> {
        self.bytes.u16(0, value)
    }

    pub fn raw_um(&self, value: Option<u16>) -> Option<u16> {
        self.bytes.u16(2, value)
    }

    pub fn zero_pos(&self, value: Option<u16>) -> Option<u16> {
        self.bytes.u16(4, value)
    }

    pub fn max_value(&self, value: Option<u16>) -> Option<u16> {
        self.bytes.u16(6, value)
    }

    pub fn polar(&self, value: Option<u8>) -> Option<u8> {
        match value {
            Some(value) => {
                let res = self.bytes.u16(6, Some((value as u16) << 15));
                return match res {
                    Some(_) => Some(value),
                    None => None,
                };
            }
            None => {
                return match self.bytes.u16(6, None) {
                    Some(value) => Some(((value & 0x8000) >> 15) as u8),
                    None => None,
                };
            }
        }
    }

    pub fn stroke(&self, value: Option<u8>) -> Option<u8> {
        self.bytes.u8(8, value)
    }
    //
    // pub fn types(&self, value: Option<u8>) -> Option<u8> {
    //     self.bytes.u8(9, value)
    // }

    pub fn rt_mode(&self, value: Option<u8>) -> Option<u8> {
        self.bytes.u8(9, value)
    }

    pub fn switch_type(&self, value: Option<u8>) -> Option<u8> {
        self.bytes.u8(10, value)
    }

    pub fn trigger_level(&self, value: Option<u16>) -> Option<u16> {
        self.bytes.u16(12, value)
    }

    pub fn release_level(&self, value: Option<u16>) -> Option<u16> {
        self.bytes.u16(14, value)
    }

    pub fn rapid_trigger_top(&self, value: Option<u16>) -> Option<u16> {
        self.bytes.u16(16, value)
    }

    pub fn rapid_trigger_area(&self, value: Option<u16>) -> Option<u16> {
        self.bytes.u16(18, value)
    }

    pub fn rapid_trigger_level(&self, value: Option<u16>) -> Option<u16> {
        self.bytes.u16(20, value)
    }

    pub fn rapid_release_level(&self, value: Option<u16>) -> Option<u16> {
        self.bytes.u16(22, value)
    }
}

#[repr(C)]
#[derive(Debug, Clone)]

pub struct AdvancedKeyBinding {
    pub bytes: RwBytes,
}
impl AdvancedKeyBinding {
    pub fn mode(&self, value: Option<u8>) -> Option<u8> {
        self.bytes.u8(0, value)
    }

    pub fn bind_key(&self, value: Option<u8>) -> Option<u8> {
        self.bytes.u8(1, value)
    }

    pub fn res0(&self, value: Option<u8>) -> Option<u8> {
        self.bytes.u8(2, value)
    }

    pub fn res1(&self, value: Option<u8>) -> Option<u8> {
        self.bytes.u8(3, value)
    }

    pub fn key_data(&self, index: u32, value: Option<KeyData>) -> Option<KeyData> {
        if index >= 4 {
            return None;
        }
        let i = 4 + index as usize * KeyData::SIZE;
        if value.is_some() {
            let data = value
                .clone()
                .expect("value not found in AdvancedKeyBinding::key_data");
            self.bytes
                .vec(i, Some(KeyData::SIZE), Some(data.bytes.into_vec()));
            return value;
        } else {
            let bytes = match self.bytes.ref_at(i, KeyData::SIZE) {
                Some(bytes) => bytes,
                None => return None,
            };
            Some(KeyData { bytes })
        }
    }

    pub fn key_datas(&self, value: Option<Vec<KeyData>>) -> Option<Vec<KeyData>> {
        if value.is_some() {
            let mut i = 4;
            for data in value
                .clone()
                .expect("value not found in AdvancedKeyBinding::key_datas")
            {
                self.bytes
                    .vec(i, Some(KeyData::SIZE), Some(data.bytes.into_vec()));
                i += KeyData::SIZE;
                if i >= 36 {
                    break;
                }
            }
            return value;
        } else {
            let mut i = 4;
            let mut res: Vec<KeyData> = Vec::new();
            while i + KeyData::SIZE <= 36 {
                let bytes = match self.bytes.ref_at(i, KeyData::SIZE) {
                    Some(bytes) => bytes,
                    None => break,
                };
                res.push(KeyData { bytes });
                i += KeyData::SIZE;
            }
            Some(res)
        }
    }
    //
    // pub fn key_data(&self, value: Option<Vec<AdvancedKeyData>>) -> Option<Vec<AdvancedKeyData>> {
    //     let mut i = 4;
    //     let mut res: Vec<AdvancedKeyData> = Vec::new();
    //     while i + 8 < self.bytes.len() && i + 8 <= 36 {
    //         let mut data_bytes = match self.bytes.ref_at(i, 8) {
    //             Some(bytes) => bytes,
    //             None => break,
    //         };
    //         if let Some(ref value) = value {
    //             if let Some(data) = value.get(i / 8) {
    //                 data_bytes.vec(0, None, Some(data.clone().bytes.into_vec()));
    //             }
    //         };
    //         res.push(AdvancedKeyData { bytes: data_bytes });
    //         i += 8;
    //     }
    //     Some(res)
    // }

    pub fn func_opt(&self, index: usize, value: Option<u8>) -> Option<u8> {
        if index >= 12 {
            return None;
        }
        self.bytes.u8(36 + index, value)
    }
    pub fn func_opts(&self, value: Option<Vec<u8>>) -> Option<Vec<u8>> {
        self.bytes.vec(36, Some(12), value)
    }
}

// #[repr(C)]
// #[derive(Debug, Clone)]
//
// pub struct AdvancedKeyData {
//     pub bytes: RwBytes,
// }
// impl AdvancedKeyData {
//
//     pub fn key_mode(&self, value: Option<u8>) -> Option<u8> {
//         self.bytes.u8(0, value)
//     }
//
//     pub fn key_opt(&self, index: usize, value: Option<u8>) -> Option<u8> {
//         if index >= 3 {
//             return None;
//         }
//         self.bytes.u8(1 + index, value)
//     }
//
//     pub fn key_value(&self, index: usize, value: Option<u8>) -> Option<u8> {
//         if index >= 4 {
//             return None;
//         }
//         self.bytes.u8(4 + index, value)
//     }
// }

#[repr(C)]
#[derive(Debug, Clone)]

pub struct TriggerKeyboardHid {
    pub bytes: RwBytes,
}
impl TriggerKeyboardHid {
    pub fn modifier_keys(&self, value: Option<u8>) -> Option<u8> {
        self.bytes.u8(0, value)
    }

    pub fn reserve0(&self, value: Option<u8>) -> Option<u8> {
        self.bytes.u8(1, value)
    }

    pub fn key_code(&self, value: Option<Vec<u8>>) -> Option<Vec<u8>> {
        self.bytes.vec(4, Some(4), value)
    }
}

#[repr(C)]
#[derive(Debug, Clone)]

pub struct TriggerMouseHid {
    pub bytes: RwBytes,
}
impl TriggerMouseHid {
    pub fn mouse_keys(&self, value: Option<u8>) -> Option<u8> {
        self.bytes.u8(0, value)
    }

    pub fn x(&self, value: Option<u8>) -> Option<u8> {
        self.bytes.u8(1, value)
    }

    pub fn y(&self, value: Option<u8>) -> Option<u8> {
        self.bytes.u8(2, value)
    }

    pub fn scroll(&self, value: Option<u8>) -> Option<u8> {
        self.bytes.u8(3, value)
    }
}

#[repr(C)]
#[derive(Debug, Clone)]

pub struct TriggerMeidaHid {
    pub bytes: RwBytes,
}
impl TriggerMeidaHid {
    pub fn key_code(&self, value: Option<u16>) -> Option<u16> {
        self.bytes.u16(0, value)
    }
}

#[repr(C)]
#[derive(Debug, Clone)]

pub struct DisplayData {
    pub bytes: RwBytes, //4 bytes alignment
}
impl DisplayData {
    pub fn create(
        data_type: u8,
        frame_number: u8,
        encoding_or_colot_table_count: u16,
        width: u16,
        height: u16,
        data: Vec<u8>,
    ) -> DisplayData {
        let mut data_len = data.len();
        let padding = if data_len % 4 != 0 {
            4 - (data_len % 4)
        } else {
            0
        };
        data_len += padding;
        let bytes = RwBytes::new(vec![0xCC; 12 + data_len]);
        bytes.u8(0, Some(data_type));
        bytes.u8(1, Some(frame_number));
        bytes.u16(2, Some(encoding_or_colot_table_count));
        bytes.u16(4, Some(width));
        bytes.u16(6, Some(height));
        bytes.u32(8, Some(data_len as u32));
        bytes.vec(12, None, Some(data));
        DisplayData { bytes }
    }

    pub fn data_type(&self, value: Option<u8>) -> Option<u8> {
        self.bytes.u8(0, value)
    }

    pub fn frame_number(&self, value: Option<u8>) -> Option<u8> {
        self.bytes.u8(1, value)
    }

    pub fn character_code(&self, value: Option<u16>) -> Option<u16> {
        self.bytes.u16(2, value)
    }

    pub fn color_table_count(&self, value: Option<u8>) -> Option<u8> {
        self.bytes.u8(2, value)
    }

    pub fn width(&self, value: Option<u16>) -> Option<u16> {
        self.bytes.u16(4, value)
    }

    pub fn height(&self, value: Option<u16>) -> Option<u16> {
        self.bytes.u16(6, value)
    }

    pub fn data_len(&self, value: Option<u32>) -> Option<u32> {
        self.bytes.u32(8, value)
    }

    pub fn len(&self) -> u32 {
        self.bytes.len() as u32
    }

    pub fn data(&self, value: Option<Vec<u8>>) -> Option<Vec<u8>> {
        let len = match self.data_len(None) {
            Some(len) => len as usize,
            None => return None,
        };
        self.bytes.vec(12, Some(len), value)
    }

    pub(in crate::structures) fn packet_len(bytes: &RwBytes, at: u32) -> Option<u32> {
        let data_type = bytes
            .u8(at as usize, None)
            .expect("Can not get data_type in DisplayData::packet_len");
        if data_type != 1 && data_type != 2 && data_type != 6 {
            return None;
        }
        Some(
            12 + bytes
                .u32((at + 8) as usize, None)
                .expect("Can not get data_len in DisplayData::packet_len") as u32,
        )
    }
}

#[repr(C)]
#[derive(Debug, Clone)]

pub struct DisplayAssets {
    pub bytes: RwBytes,
}
impl DisplayAssets {
    pub fn create(datas: Vec<DisplayData>) -> DisplayAssets {
        let len = datas.iter().map(|data| data.bytes.len()).sum::<usize>();

        let bytes = RwBytes::new(vec![0; len]);
        let mut offset = 0;
        for data in datas {
            let data_len = data.bytes.len();
            bytes.vec(offset, Some(data.bytes.len()), Some(data.bytes.into_vec()));
            offset += data_len;
        }
        DisplayAssets { bytes }
    }

    pub fn datas(&self) -> Option<Vec<DisplayData>> {
        let mut res: Vec<DisplayData> = Vec::new();
        let mut len = 0;
        while len < self.bytes.len() {
            let packet_len = match DisplayData::packet_len(&self.bytes, len as u32) {
                Some(packet_len) => packet_len,
                None => {
                    println!("DisplayAssets::datas: packet_len is None");
                    break;
                }
            };
            let bytes = match self.bytes.ref_at(len, packet_len as usize) {
                Some(bytes) => bytes,
                None => {
                    println!("DisplayAssets::datas: ref bytes is None");
                    break;
                }
            };
            res.push(DisplayData { bytes });
            len += packet_len as usize;
        }
        Some(res)
    }

    pub fn len(&self) -> u32 {
        self.bytes.len() as u32
    }

    pub fn used_len(&self) -> u32 {
        let mut len = 0;
        while len < self.bytes.len() {
            let packet_len = match DisplayData::packet_len(&self.bytes, len as u32) {
                Some(packet_len) => packet_len,
                None => break,
            };
            len += packet_len as usize;
        }
        len as u32
    }
}

#[repr(C)]
#[derive(Debug, Clone)]

pub struct DisplayAssetsPacket {
    pub bytes: RwBytes,
}
impl DisplayAssetsPacket {
    pub fn addr(&self, value: Option<u32>) -> Option<u32> {
        self.bytes.u32(0, value)
    }

    pub fn data(&self, value: Option<Vec<u8>>) -> Option<Vec<u8>> {
        self.bytes.vec(4, None, value)
    }
}

#[repr(C)]
#[derive(Debug, Clone)]
pub struct LCDFill {
    pub bytes: RwBytes,
}
impl LCDFill {
    const SIZE: usize = 4;

    pub fn width(&self, value: Option<u16>) -> Option<u16> {
        self.bytes.u16(0, value)
    }

    pub fn height(&self, value: Option<u16>) -> Option<u16> {
        self.bytes.u16(2, value)
    }
}

#[repr(C)]
#[derive(Debug, Clone)]
pub struct LCDWidget {
    pub bytes: RwBytes,
}
impl LCDWidget {
    const SIZE: usize = 2;

    pub fn index(&self, value: Option<u8>) -> Option<u8> {
        self.bytes.u8(0, value)
    }

    pub fn mix_mode(&self, value: Option<u8>) -> Option<u8> {
        self.bytes.u8(1, value)
    }
}

#[repr(C)]
#[derive(Debug, Clone)]
pub struct LCDFont {
    pub bytes: RwBytes,
}
impl LCDFont {
    const SIZE: usize = 3;

    pub fn size(&self, value: Option<u8>) -> Option<u8> {
        self.bytes.u8(0, value)
    }

    pub fn mixed_mode(&self, value: Option<u8>) -> Option<u8> {
        self.bytes.u8(1, value)
    }

    pub fn digit(&self, value: Option<u8>) -> Option<u8> {
        self.bytes.u8(2, value)
    }
}

#[repr(C)]
#[derive(Debug, Clone)]
pub struct LCDImage {
    pub bytes: RwBytes,
}
impl LCDImage {
    const SIZE: usize = 1;

    pub fn index(&self, value: Option<u8>) -> Option<u8> {
        self.bytes.u8(0, value)
    }
}

#[repr(C)]
#[derive(Debug, Clone)]
pub struct LCDInfo {
    pub bytes: RwBytes,
}
impl LCDInfo {
    const SIZE: usize = 4;

    pub fn lcd_fill(&self) -> Option<LCDFill> {
        let bytes = match self.bytes.ref_at(0, LCDFill::SIZE) {
            Some(bytes) => bytes,
            None => return None,
        };
        Some(LCDFill { bytes })
    }

    pub fn lcd_widget(&self) -> Option<LCDWidget> {
        let bytes = match self.bytes.ref_at(0, LCDWidget::SIZE) {
            Some(bytes) => bytes,
            None => return None,
        };
        Some(LCDWidget { bytes })
    }

    pub fn lcd_font(&self) -> Option<LCDFont> {
        let bytes = match self.bytes.ref_at(0, LCDFont::SIZE) {
            Some(bytes) => bytes,
            None => return None,
        };
        Some(LCDFont { bytes })
    }

    pub fn lcd_image(&self) -> Option<LCDImage> {
        let bytes = match self.bytes.ref_at(0, LCDImage::SIZE) {
            Some(bytes) => bytes,
            None => return None,
        };
        Some(LCDImage { bytes })
    }
}

#[repr(C)]
#[derive(Debug, Clone)]

pub struct LCDDrawData {
    pub bytes: RwBytes,
}
impl LCDDrawData {
    pub fn data_type(&self, value: Option<u8>) -> Option<u8> {
        self.bytes.u8(0, value)
    }

    pub fn event_key_id(&self, value: Option<u8>) -> Option<u8> {
        self.bytes.u8(1, value)
    }

    pub fn event_type(&self, value: Option<u8>) -> Option<u8> {
        self.bytes.u8(2, value)
    }

    pub fn fn_mask(&self, value: Option<u8>) -> Option<u8> {
        self.bytes.u8(3, value)
    }

    pub fn info(&self) -> Option<LCDInfo> {
        let bytes = match self.bytes.ref_at(4, LCDInfo::SIZE) {
            Some(bytes) => bytes,
            None => return None,
        };
        Some(LCDInfo { bytes })
    }

    pub fn site_x(&self, value: Option<i16>) -> Option<i16> {
        self.bytes.i16(8, value)
    }

    pub fn site_y(&self, value: Option<i16>) -> Option<i16> {
        self.bytes.i16(10, value)
    }

    pub fn color(&self, value: Option<u16>) -> Option<u16> {
        self.bytes.u16(12, value)
    }

    pub fn bg_color(&self, value: Option<u16>) -> Option<u16> {
        self.bytes.u16(14, value)
    }

    pub fn reserve(&self, value: Option<u32>) -> Option<u32> {
        self.bytes.u32(16, value)
    }

    pub fn text(&self, value: Option<String>) -> Option<String> {
        let encoding = match self.data_type(None) {
            Some(4) => u8::from(Encoding::ASCII),
            Some(5) => u8::from(Encoding::UTF16LE),
            _ => return None,
        };
        self.bytes.str(encoding, 20, value)
    }
}

#[repr(C)]
#[derive(Debug, Clone)]

pub struct ScreenBuffer {
    pub bytes: RwBytes,
}
impl ScreenBuffer {
    pub fn addr(&self, value: Option<u32>) -> Option<u32> {
        self.bytes.u32(0, value)
    }

    pub fn data(&self, value: Option<Vec<u8>>) -> Option<Vec<u8>> {
        self.bytes.vec(4, None, value)
    }
}

#[repr(C)]
#[derive(Debug, Clone)]

pub struct LedEffect {
    pub bytes: RwBytes,
}
impl LedEffect {
    fn swap_bg_channel(color: u32) -> u32 {
        let r = color & 0xFF;
        let g = (color >> 8) & 0xFF;
        let b = (color >> 16) & 0xFF;
        let a = (color >> 24) & 0xFF;
        (r << 16) | (g << 8) | b | (a << 24)
    }

    pub fn r(&self, value: Option<u8>) -> Option<u8> {
        self.bytes.u8(0, value)
    }

    pub fn g(&self, value: Option<u8>) -> Option<u8> {
        self.bytes.u8(1, value)
    }

    pub fn b(&self, value: Option<u8>) -> Option<u8> {
        self.bytes.u8(2, value)
    }

    pub fn enabled(&self, value: Option<u8>) -> Option<u8> {
        self.bytes.u8(3, value)
    }

    pub fn color(&self, color: Option<u32>) -> Option<u32> {
        let offset = 0;
        match color {
            Some(value) => {
                // self.bytes.u32(offset, Some(LedEffect::swap_bg_channel(value)));
                self.r(Some(((value >> 16) & 0xFF) as u8));
                self.g(Some(((value >> 8) & 0xFF) as u8));
                self.b(Some((value & 0xFF) as u8));
                return Some(value);
            }
            None => {
                return match self.bytes.u32(offset, None) {
                    Some(value) => Some(LedEffect::swap_bg_channel(value) | 0xFF000000),
                    None => None,
                };
            }
        }
    }

    pub fn mode(&self, value: Option<u8>) -> Option<u8> {
        self.bytes.u8(4, value)
    }

    pub fn sub_mode(&self, value: Option<u8>) -> Option<u8> {
        self.bytes.u8(5, value)
    }

    pub fn speed(&self, value: Option<u8>) -> Option<u8> {
        self.bytes.u8(6, value)
    }

    pub fn brightness(&self, value: Option<u8>) -> Option<u8> {
        self.bytes.u8(7, value)
    }

    pub fn profile_color(&self, index: u8, color: Option<u32>) -> Option<u32> {
        if index >= 4 {
            return None;
        }
        let offset = 8 + index as usize * 4;
        match color {
            Some(value) => {
                self.bytes
                    .u32(offset, Some(LedEffect::swap_bg_channel(value)));
                return Some(value);
            }
            None => {
                return match self.bytes.u32(offset, None) {
                    Some(value) => Some(LedEffect::swap_bg_channel(value)),
                    None => None,
                };
            }
        }
    }

    pub fn numlock_color(&self, value: Option<u32>) -> Option<u32> {
        match value {
            Some(value) => {
                self.bytes.u32(24, Some(LedEffect::swap_bg_channel(value)));
                return Some(value);
            }
            None => {
                return match self.bytes.u32(24, None) {
                    Some(value) => Some(LedEffect::swap_bg_channel(value)),
                    None => None,
                };
            }
        }
    }

    pub fn capslock_color(&self, value: Option<u32>) -> Option<u32> {
        match value {
            Some(value) => {
                self.bytes.u32(28, Some(LedEffect::swap_bg_channel(value)));
                return Some(value);
            }
            None => {
                return match self.bytes.u32(28, None) {
                    Some(value) => Some(LedEffect::swap_bg_channel(value)),
                    None => None,
                };
            }
        }
    }

    pub fn scrolllock_color(&self, value: Option<u32>) -> Option<u32> {
        match value {
            Some(value) => {
                self.bytes.u32(32, Some(LedEffect::swap_bg_channel(value)));
                return Some(value);
            }
            None => {
                return match self.bytes.u32(32, None) {
                    Some(value) => Some(LedEffect::swap_bg_channel(value)),
                    None => None,
                };
            }
        }
    }

    pub fn socd_color(&self, value: Option<u32>) -> Option<u32> {
        match value {
            Some(value) => {
                self.bytes.u32(36, Some(LedEffect::swap_bg_channel(value)));
                return Some(value);
            }
            None => {
                return match self.bytes.u32(36, None) {
                    Some(value) => Some(LedEffect::swap_bg_channel(value)),
                    None => None,
                };
            }
        }
    }

    pub fn fn_diff_color(&self, value: Option<u32>) -> Option<u32> {
        match value {
            Some(value) => {
                self.bytes.u32(40, Some(LedEffect::swap_bg_channel(value)));
                return Some(value);
            }
            None => {
                return match self.bytes.u32(40, None) {
                    Some(value) => Some(LedEffect::swap_bg_channel(value)),
                    None => None,
                };
            }
        }
    }

    pub fn tap_color(&self, value: Option<u32>) -> Option<u32> {
        match value {
            Some(value) => {
                self.bytes.u32(44, Some(LedEffect::swap_bg_channel(value)));
                return Some(value);
            }
            None => {
                return match self.bytes.u32(44, None) {
                    Some(value) => Some(LedEffect::swap_bg_channel(value)),
                    None => None,
                };
            }
        }
    }
}

#[repr(C)]
#[derive(Debug, Clone)]

pub struct GamePadCfg {
    pub bytes: RwBytes,
}

impl GamePadCfg {
    pub fn gamepad_type(&self, value: Option<u8>) -> Option<u8> {
        self.bytes.u8(0, value)
    }

    pub fn options(&self, value: Option<u8>) -> Option<u8> {
        self.bytes.u8(1, value)
    }

    pub fn res(&self, value: Option<u16>) -> Option<u16> {
        self.bytes.u16(2, value)
    }

    pub fn point(&self, index: usize, value: Option<(u8, u8)>) -> Option<(u8, u8)> {
        if index >= 8 {
            return None;
        }
        let offset = 4 + index * 2;
        if let Some((x, y)) = value {
            self.bytes.u8(offset, Some(x));
            self.bytes.u8(offset + 1, Some(y));
            Some((x, y))
        } else {
            let x = self.bytes.u8(offset, None)?;
            let y = self.bytes.u8(offset + 1, None)?;
            Some((x, y))
        }
    }

    pub fn points(&self, value: Option<Vec<(u8, u8)>>) -> Option<Vec<(u8, u8)>> {
        if let Some(points) = value {
            let mut result = Vec::new();
            for (i, (x, y)) in points.iter().enumerate().take(8) {
                if let Some(point) = self.point(i, Some((*x, *y))) {
                    result.push(point);
                }
            }
            Some(result)
        } else {
            let mut result = Vec::new();
            for i in 0..8 {
                if let Some(point) = self.point(i, None) {
                    result.push(point);
                }
            }
            Some(result)
        }
    }

    pub fn map(&self, index: usize, value: Option<u8>) -> Option<u8> {
        if index >= 36 {
            return None;
        }
        let offset = 20 + index; // 4 + 8*2 = 20
        self.bytes.u8(offset, value)
    }

    pub fn maps(&self, value: Option<Vec<u8>>) -> Option<Vec<u8>> {
        if let Some(maps) = value {
            let mut result = Vec::new();
            for (i, &map_val) in maps.iter().enumerate().take(36) {
                if let Some(val) = self.map(i, Some(map_val)) {
                    result.push(val);
                }
            }
            Some(result)
        } else {
            let mut result = Vec::new();
            for i in 0..36 {
                if let Some(val) = self.map(i, None) {
                    result.push(val);
                }
            }
            Some(result)
        }
    }
}

// #[repr(C)]
// #[derive(Clone)]
//
// pub struct AmbientLEDEffect {
//     pub bytes: RwBytes,
// }
// impl AmbientLEDEffect {
//
//     pub fn mode(&self, value: Option<u8>) -> Option<u8> {
//         self.bytes.u8(0, value)
//     }
//
//     pub fn r0(&self, value: Option<u8>) -> Option<u8> {
//         self.bytes.u8(1, value)
//     }
//
//     pub fn g0(&self, value: Option<u8>) -> Option<u8> {
//         self.bytes.u8(2, value)
//     }
//
//     pub fn b0(&self, value: Option<u8>) -> Option<u8> {
//         self.bytes.u8(3, value)
//     }
//
//     pub fn sub_mode(&self, value: Option<u8>) -> Option<u8> {
//         self.bytes.u8(4, value)
//     }
//
//     pub fn r1(&self, value: Option<u8>) -> Option<u8> {
//         self.bytes.u8(5, value)
//     }
//
//     pub fn g1(&self, value: Option<u8>) -> Option<u8> {
//         self.bytes.u8(6, value)
//     }
//
//     pub fn b1(&self, value: Option<u8>) -> Option<u8> {
//         self.bytes.u8(7, value)
//     }
//
//     pub fn reserve(&self, value: Option<u8>) -> Option<u8> {
//         self.bytes.u8(8, value)
//     }
//
//     pub fn r2(&self, value: Option<u8>) -> Option<u8> {
//         self.bytes.u8(9, value)
//     }
//
//     pub fn g2(&self, value: Option<u8>) -> Option<u8> {
//         self.bytes.u8(10, value)
//     }
//
//     pub fn b2(&self, value: Option<u8>) -> Option<u8> {
//         self.bytes.u8(11, value)
//     }

//
//     pub fn color0(&self, value: Option<u32>) -> Option<u32> {
//         match value {
//             Some(value) => {
//                 self.r0(Some((value & 0xFF) as u8));
//                 self.g0(Some(((value >> 8) & 0xFF) as u8));
//                 self.b0(Some(((value >> 16) & 0xFF) as u8));
//                 Some(value)
//             }
//             None => {
//                 let r = self.r0(None)?;
//                 let g = self.g0(None)?;
//                 let b = self.b0(None)?;
//                 Some((r as u32) | ((g as u32) << 8) | ((b as u32) << 16))
//             }
//         }
//     }
//
//     pub fn color1(&self, value: Option<u32>) -> Option<u32> {
//         match value {
//             Some(value) => {
//                 self.r1(Some((value & 0xFF) as u8));
//                 self.g1(Some(((value >> 8) & 0xFF) as u8));
//                 self.b1(Some(((value >> 16) & 0xFF) as u8));
//                 Some(value)
//             }
//             None => {
//                 let r = self.r1(None)?;
//                 let g = self.g1(None)?;
//                 let b = self.b1(None)?;
//                 Some((r as u32) | ((g as u32) << 8) | ((b as u32) << 16))
//             }
//         }
//     }
//
//     pub fn color2(&self, value: Option<u32>) -> Option<u32> {
//         match value {
//             Some(value) => {
//                 self.r2(Some((value & 0xFF) as u8));
//                 self.g2(Some(((value >> 8) & 0xFF) as u8));
//                 self.b2(Some(((value >> 16) & 0xFF) as u8));
//                 Some(value)
//             }
//             None => {
//                 let r = self.r2(None)?;
//                 let g = self.g2(None)?;
//                 let b = self.b2(None)?;
//                 Some((r as u32) | ((g as u32) << 8) | ((b as u32) << 16))
//             }
//         }
//     }
// }

#[repr(C)]
#[derive(Debug, Clone)]
pub struct AmbientLED {
    pub bytes: RwBytes,
}
impl AmbientLED {
    pub fn brightness(&self, value: Option<u8>) -> Option<u8> {
        self.bytes.u8(0, value)
    }

    pub fn speed(&self, value: Option<u8>) -> Option<u8> {
        self.bytes.u8(1, value)
    }

    pub fn led_count(&self, value: Option<u8>) -> Option<u8> {
        self.bytes.u8(2, value)
    }

    pub fn reserve(&self, value: Option<u8>) -> Option<u8> {
        self.bytes.u8(3, value)
    }

    pub fn mode(&self, value: Option<u8>) -> Option<u8> {
        self.bytes.u8(4, value)
    }

    pub fn r(&self, value: Option<u8>) -> Option<u8> {
        self.bytes.u8(5, value)
    }

    pub fn g(&self, value: Option<u8>) -> Option<u8> {
        self.bytes.u8(6, value)
    }

    pub fn b(&self, value: Option<u8>) -> Option<u8> {
        self.bytes.u8(7, value)
    }

    pub fn sub_mode(&self, value: Option<u8>) -> Option<u8> {
        self.bytes.u8(8, value)
    }

    pub fn r1(&self, value: Option<u8>) -> Option<u8> {
        self.bytes.u8(9, value)
    }

    pub fn g1(&self, value: Option<u8>) -> Option<u8> {
        self.bytes.u8(10, value)
    }

    pub fn b1(&self, value: Option<u8>) -> Option<u8> {
        self.bytes.u8(11, value)
    }

    pub fn res1(&self, value: Option<u8>) -> Option<u8> {
        self.bytes.u8(12, value)
    }

    pub fn r2(&self, value: Option<u8>) -> Option<u8> {
        self.bytes.u8(13, value)
    }

    pub fn g2(&self, value: Option<u8>) -> Option<u8> {
        self.bytes.u8(14, value)
    }

    pub fn b2(&self, value: Option<u8>) -> Option<u8> {
        self.bytes.u8(15, value)
    }

    pub fn res2(&self, value: Option<u32>) -> Option<u32> {
        self.bytes.u32(16, value)
    }

    pub fn led_map(&self, value: Option<Vec<bool>>) -> Option<Vec<bool>> {
        let bytes = match self.bytes.ref_at(20, 16) {
            Some(bytes) => bytes,
            None => return None,
        };
        if let Some(vec) = value {
            if vec.len() > 128 || vec.len() < 128 {
                return None;
            }
            for i in 0..16 {
                let mut byte = 0u8;
                for j in 0..8 {
                    if let Some(bit) = vec.get(i * 8 + j) {
                        if *bit {
                            byte |= 1 << j;
                        }
                    } else {
                        return None; // Not enough bits provided
                    }
                }
                bytes.u8(i, Some(byte));
            }
            Some(vec)
        } else {
            let mut vec = Vec::with_capacity(128);
            for i in 0..16 {
                let byte = bytes.u8(i, None)?;
                for j in 0..8 {
                    vec.push((byte & (1 << j)) != 0);
                }
            }
            Some(vec)
        }
    }

    pub fn color0(&self, value: Option<u32>) -> Option<u32> {
        match value {
            Some(value) => {
                self.r(Some(((value >> 16) & 0xFF) as u8));
                self.g(Some(((value >> 8) & 0xFF) as u8));
                self.b(Some((value & 0xFF) as u8));
                Some(value)
            }
            None => {
                let r = self.r(None)?;
                let g = self.g(None)?;
                let b = self.b(None)?;
                Some((b as u32) | ((g as u32) << 8) | ((r as u32) << 16))
            }
        }
    }

    pub fn color1(&self, value: Option<u32>) -> Option<u32> {
        match value {
            Some(value) => {
                self.r1(Some(((value >> 16) & 0xFF) as u8));
                self.g1(Some(((value >> 8) & 0xFF) as u8));
                self.b1(Some((value & 0xFF) as u8));
                Some(value)
            }
            None => {
                let r = self.r1(None)?;
                let g = self.g1(None)?;
                let b = self.b1(None)?;
                Some((b as u32) | ((g as u32) << 8) | ((r as u32) << 16))
            }
        }
    }

    pub fn color2(&self, value: Option<u32>) -> Option<u32> {
        match value {
            Some(value) => {
                self.r2(Some(((value >> 16) & 0xFF) as u8));
                self.g2(Some(((value >> 8) & 0xFF) as u8));
                self.b2(Some((value & 0xFF) as u8));
                Some(value)
            }
            None => {
                let r = self.r2(None)?;
                let g = self.g2(None)?;
                let b = self.b2(None)?;
                Some((b as u32) | ((g as u32) << 8) | ((r as u32) << 16))
            }
        }
    }

    //
    // pub fn effect(&self, index: u32, value: Option<AmbientLEDEffect>) -> Option<AmbientLEDEffect> {
    //     if index >= 4 {
    //         return None;
    //     }
    //     let offset = 8 + index as usize * 12; // 8 + 4 * 3
    //     let bytes = match self.bytes.ref_at(offset, 12) {
    //         Some(bytes) => bytes,
    //         None => return None,
    //     };
    //     match value {
    //         Some(effect) => {
    //             self.bytes.vec(offset, Some(12), Some(effect.bytes.clone().into_vec()));
    //             Some(effect)
    //         }
    //         None => Some(AmbientLEDEffect { bytes }),
    //     }
    // }
}

#[repr(C)]
#[derive(Clone)]

pub struct BroadCastData {
    pub bytes: RwBytes,
}
impl BroadCastData {
    pub fn data_type(&self, value: Option<u8>) -> Option<u8> {
        self.bytes.u8(0, value)
    }

    pub fn len(&self) -> Option<u8> {
        match self.data_len() {
            Some(len) => Some(len + 1),
            None => None,
        }
    }
    fn data_len(&self) -> Option<u8> {
        let tp = match self.data_type(None) {
            Some(tp) => tp,
            None => return None,
        };
        let len = if tp < 0x80 {
            1
        } else if tp >= 0x80 && tp < 0xC0 {
            2
        } else if tp >= 0xC0 && tp < 0xE0 {
            4
        } else {
            match self.bytes.u8(1, None) {
                Some(len) => len,
                None => return None,
            }
        };
        Some(len)
    }

    pub fn should_skip_first_byte(&self) -> Option<bool> {
        match self.data_type(None) {
            Some(tp) => {
                if tp >= 0xE0 {
                    Some(true)
                } else {
                    Some(false)
                }
            }
            None => None,
        }
    }

    pub fn data(&self, value: Option<Vec<u8>>) -> Option<Vec<u8>> {
        let mut len = match self.data_len() {
            Some(len) => Some(len as usize),
            None => {
                println!("BroadCastData::data: len is None");
                return None;
            }
        };
        let begin = match self.should_skip_first_byte() {
            Some(true) => {
                len = Some(len.expect("Can not get len in BroadCastData::data") - 1);
                2
            }
            Some(false) => 1,
            None => {
                println!("BroadCastData::data: should_skip_first_byte is None");
                return None;
            }
        };
        self.bytes.vec(begin, len, value)
    }

    pub fn type_str(&self) -> String {
        let res = match self
            .data_type(None)
            .expect("Can not get type in BroadCastData::type_str")
        {
            0x00 => "BRD_STOP",
            0x01 => "BRD_TYPE_SYS_CMD",
            0x02 => "BRD_TYPE_SYS_KB_LED",
            0x03 => "BRD_TYPE_SYS_KEY_LAY",
            0x04 => "BRD_TYPE_SYS_CPU_LOAD",
            0x05 => "BRD_TYPE_SYS_PROFILE",
            0x10 => "BRD_TYPE_KB_KEY_PRESS",
            0x11 => "BRD_TYPE_KB_KEY_RELEASE",
            0x12 => "BRD_TYPE_KB_VALUE_SK_ADD",
            0x13 => "BRD_TYPE_KB_VALUE_SK_DEL",
            0x14 => "BRD_TYPE_KB_VALUE_GK_ADD",
            0x15 => "BRD_TYPE_KB_VALUE_GK_DEL",
            0x16 => "BRD_TYPE_KB_VALUE_JOYSTICK_ADD",
            0x17 => "BRD_TYPE_KB_VALUE_JOYSTICK_DEL",
            0x18 => "BRD_TYPE_KB_VALUE_JOYSTICK_HAT",
            0x80 => "BRD_TYPE_SYS_TIME_MS",
            0x81 => "BRD_TYPE_MU_DATA",
            0xC0 => "BRD_TYPE_SYS_TIME",
            0xC1 => "BRD_TYPE_SYS_CON",
            0xC2 => "BRD_TYPE_MO_DATA",
            0xC3 => "BRD_TYPE_POINT",
            0xE0 => "BRD_TYPE_EX",
            0xE1 => "BRD_TYPE_KEY_PRESS_LEN_UM",
            _ => "Unknown",
        };
        res.to_string()
    }
}
impl std::fmt::Debug for BroadCastData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let tp = match self.clone().data_type(None) {
            Some(tp) => tp,
            None => return write!(f, "Type: None"),
        };
        let fmt_tp = format!("Type: 0x{:02X}", tp);
        let type_str = match tp {
            0x00 => "BRD_STOP",
            0x01 => "BRD_TYPE_SYS_CMD",
            0x02 => "BRD_TYPE_SYS_KB_LED",
            0x03 => "BRD_TYPE_SYS_KEY_LAY",
            0x04 => "BRD_TYPE_SYS_CPU_LOAD",
            0x05 => "BRD_TYPE_SYS_PROFILE",

            0x10 => "BRD_TYPE_KB_KEY_PRESS",
            0x11 => "BRD_TYPE_KB_KEY_RELEASE",
            0x12 => "BRD_TYPE_KB_VALUE_SK_ADD",
            0x13 => "BRD_TYPE_KB_VALUE_SK_DEL",
            0x14 => "BRD_TYPE_KB_VALUE_GK_ADD",
            0x15 => "BRD_TYPE_KB_VALUE_GK_DEL",
            0x16 => "BRD_TYPE_KB_VALUE_JOYSTICK_ADD",
            0x17 => "BRD_TYPE_KB_VALUE_JOYSTICK_DEL",
            0x18 => "BRD_TYPE_KB_VALUE_JOYSTICK_HAT",

            0x80 => "BRD_TYPE_SYS_TIME_MS",
            0x81 => "BRD_TYPE_MU_DATA",

            0xC0 => "BRD_TYPE_SYS_TIME",
            0xC1 => "BRD_TYPE_SYS_CON",
            0xC2 => "BRD_TYPE_MO_DATA",
            0xC3 => "BRD_TYPE_POINT",

            0xE0 => "BRD_TYPE_EX",

            _ => fmt_tp.as_str(),
        };
        f.debug_struct("BroadCastData")
            .field("type_", &type_str)
            // .field("data", &self.bytes.deep_clone().into_vec())
            .finish()
    }
}
#[repr(C)]
#[derive(Debug, Clone)]

pub struct BroadCast {
    pub bytes: RwBytes,
}
impl BroadCast {
    pub fn data(&self) -> Option<Vec<BroadCastData>> {
        let mut i = 0;
        let mut res: Vec<BroadCastData> = Vec::new();
        while i < self.bytes.len() {
            let bytes = match self.bytes.ref_at(i, self.bytes.len() - i) {
                Some(bytes) => bytes,
                None => {
                    println!("BroadCast::data: ref bytes is None");
                    break;
                }
            };
            let data = BroadCastData { bytes };
            let data_len = match data.len() {
                Some(len) => len as usize,
                None => {
                    println!("BroadCast::data: len is None");
                    break;
                }
            };
            let bytes = match self.bytes.ref_at(i, data_len) {
                Some(bytes) => bytes,
                None => {
                    println!("BroadCast::data: ref bytes is None");
                    break;
                }
            };
            let data = BroadCastData { bytes };
            i += data_len;
            if data.data_type(None) == Some(0x00) {
                println!("BroadCast::data: end");
                break;
            }
            let tp = data.data_type(None);
            res.push(data);
            if tp == Some(0xE1) {
                // my boss said, just skip the rest of data
                break;
            }
        }
        Some(res)
    }
}
