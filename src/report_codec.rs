use crate::byte_converter::RwBytes;
use crate::structures_codec::CodecableHidPackage;
use std::collections::HashMap;
use std::mem::transmute;
use std::sync::Arc;
use std::{borrow::BorrowMut, cmp::min, collections::VecDeque};

use futures::future::Either;
//use crate::api::sayo_device::structures_codec::structures_codec::*;
use futures::{Future, channel::oneshot};
use std::sync::Mutex;

use crate::device::SayoDeviceApi;
use crate::structures::*;
use crate::utility::future_delay;

// 添加错误类型定义
#[derive(Debug, Clone)]
pub enum ReportError {
    BadHeaderLength(usize),
    BadReportHeader,
    BadReportLength(usize),
    CrcError,
    Timeout,
    ChannelError,
    UnsupportedReportId(u8),
    BadScreenBuffer,
    BadEncodingByte,
}

impl std::fmt::Display for ReportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ReportError::BadHeaderLength(len) => write!(f, "Bad Report Header Length: {}", len),
            ReportError::BadReportHeader => write!(f, "Bad Report Header"),
            ReportError::BadReportLength(len) => write!(f, "Bad Report Length: {}", len),
            ReportError::CrcError => write!(f, "CRC error, broken packet"),
            ReportError::Timeout => write!(f, "Request response timeout"),
            ReportError::ChannelError => write!(f, "Channel error"),
            ReportError::UnsupportedReportId(id) => write!(f, "Unsupported report id: {}", id),
            ReportError::BadScreenBuffer => write!(f, "Bad Screen Buffer"),
            ReportError::BadEncodingByte => write!(f, "Bad encoding byte in StringContent"),
        }
    }
}

impl std::error::Error for ReportError {}

// 常量定义
const REPORT_ID_21: u8 = 0x21;
const REPORT_ID_22: u8 = 0x22;
const MAX_PACKAGE_LEN_21: usize = 56;
const MAX_PACKAGE_LEN_22: usize = 1016;
const HEADER_SIZE: usize = 8;
const TIMEOUT_MS: u32 = 8000;

pub struct ReportDecoder {
    handle: u128,
    buffers: Mutex<HashMap<(u8, u8, u8), Vec<u8>>>,
    waiter_channels:
        Mutex<HashMap<(u8, u8, u8), VecDeque<oneshot::Sender<(HidReportHeader, Vec<u8>)>>>>,
    screen_buffer: Vec<u8>,
    broadcast: Arc<dyn Fn(u128, &mut BroadCast) + Send + Sync + 'static>,
}

impl ReportDecoder {
    pub fn new(
        handle: u128,
        on_broadcast: Arc<dyn Fn(u128, &mut BroadCast) + Send + Sync + 'static>,
    ) -> Self {
        ReportDecoder {
            buffers: Mutex::new(HashMap::new()),
            waiter_channels: Mutex::new(HashMap::new()),
            screen_buffer: Vec::new(),
            handle: handle,
            broadcast: on_broadcast,
        }
    }

    pub fn join(&mut self, packet: &mut Vec<u8>) -> Result<(), ReportError> {
        // println!("report received {:02X?}", packet);
        if packet.len() < HEADER_SIZE {
            println!("Bad Report Header Length {:?}", packet.len());
            return Err(ReportError::BadHeaderLength(packet.len()));
        }

        let header = HidReportHeader::new(RwBytes::new(packet[0..HEADER_SIZE].to_vec()));
        let report_id = header.report_id(None).ok_or(ReportError::BadReportHeader)?;
        if report_id != REPORT_ID_21 && report_id != REPORT_ID_22 {
            return Ok(()); // 不是我们关心的报告ID，直接返回
        }
        let echo = header.echo(None).ok_or(ReportError::BadReportHeader)?;
        if echo != SayoDeviceApi::ECHO && echo != 0x00 {
            return Ok(()); // 不是我们关心的echo，直接返回
        }

        if echo != 0x00 {
            //check crc
            let packet_crc = packet[2] as u16 | (packet[3] as u16) << 8;
            packet[2] = 0;
            packet[3] = 0;
            let crc = get_crc16(&packet);
            // println!("crc: {:02X?} {:02X?}", packet_crc, crc);
            if packet_crc != crc {
                println!("CRC error, broken packet: {:02X?}", packet);
                return Err(ReportError::CrcError);
            }
        }

        let cmd = header.cmd(None).ok_or(ReportError::BadReportHeader)?;
        let index = header.index(None).ok_or(ReportError::BadReportHeader)?;
        let len = header.len(None).ok_or(ReportError::BadReportHeader)?;
        let handle = (report_id, cmd, index);
        if len + 4 > packet.len() as u16 {
            println!("Bad Report Length {:?}", packet.len());
            return Err(ReportError::BadReportLength(packet.len()));
        }

        // 使用切片避免不必要的内存分配
        let data_slice = &packet[HEADER_SIZE..len as usize + 4];
        let mut data: Vec<u8> = data_slice.to_vec();
        //println!("recive package : {:02X?}", &packet[0..len as usize + 4]);

        let status = header.status(None).ok_or(ReportError::BadReportHeader)?;
        self.log_status(status, cmd, index, &packet);

        match status {
            0x01 => {
                // success & continue
                if let Ok(mut buffers) = self.buffers.lock() {
                    let buffer = buffers.entry(handle).or_insert(Vec::new());
                    buffer.extend(data);
                }
                //println!("Report arrived cotinue: {:?}", index);
            }
            _ => {
                if let Ok(mut buffers) = self.buffers.lock() {
                    if buffers.contains_key(&handle) {
                        let buffer = buffers.entry(handle).or_insert(Vec::new());
                        data.splice(0..0, buffer.clone());
                        buffers.remove(&handle);
                    }
                }
                //println!("Report arrived done: {:?}", index);
                self.on_package_complete(header, data);
            }
        }

        Ok(())
    }

    fn log_status(&self, status: u8, cmd: u8, index: u8, data: &[u8]) {
        match status {
            0x00 => {
                // success & end
            }
            0x01 => {
                // success & continue
            }
            0x02 => {
                // gb18030 string
            }
            0x03 => {
                // utf16le string
                println!("UTF16LE string: {:02X?} {:02X?}", cmd, index);
            }
            0x10 => {
                // index does not exist
                println!("Index does not exist: {:02X?} {:02X?}", cmd, index);
            }
            0x11 => {
                // data length too long
                println!(
                    "Data length too long: {:02X?} {:02X?} max len {:02X?}",
                    cmd, index, data
                );
            }
            0x12 => {
                // data length too short
                println!("Data length too short: {:02X?} {:02X?}", cmd, index);
            }
            0x13 => {
                // data mismatch
                println!("Data mismatch: {:02X?} {:02X?}", cmd, index);
            }
            0x14 => {
                // alignment error
                println!("Alignment error: {:02X?} {:02X?}", cmd, index);
            }
            0x3C => {
                // crc error
                println!("CRC error: {:02X?} {:02X?} {:02X?}", cmd, index, data);
            }
            0x3D => {
                // data length too long
                println!("Data length too long: {:02X?} {:02X?}", cmd, index);
            }
            0x3E => {
                // index cannot be written
                println!("Index cannot be written: {:02X?} {:02X?}", cmd, index);
            }
            0x3F => {
                // cmd does not exist
                println!("Cmd does not exist: {:02X?} {:02X?}", cmd, index);
            }
            _ => {
                // unknown status
                println!("Unknown status: {:02X?} {:02X?}", cmd, status);
            }
        }
    }

    pub fn resize_screen_buffer(&mut self, len: usize) {
        if self.screen_buffer.len() != len {
            self.screen_buffer.resize(len, 0);
        }
    }

    pub fn get_screen_buffer(&self, vec: &mut Vec<u8>) {
        if vec.len() != self.screen_buffer.len() {
            vec.resize(self.screen_buffer.len(), 0);
        }
        vec.clone_from_slice(&self.screen_buffer);
    }

    fn fill_screen_buffer(&mut self, data: Vec<u8>) -> Result<(), ReportError> {
        let buffer = ScreenBuffer::new(RwBytes::new(data));
        let address = buffer.addr(None).ok_or(ReportError::BadScreenBuffer)?;
        let bytes = buffer.data(None).ok_or(ReportError::BadScreenBuffer)?;
        let end = std::cmp::min(address as usize + bytes.len(), self.screen_buffer.len());
        self.screen_buffer
            .splice(address as usize..end, bytes.iter().cloned());
        Ok(())
    }

    fn on_package_complete(&mut self, header: HidReportHeader, data: Vec<u8>) {
        let echo = header.echo(None).unwrap_or(0);
        let cmd = header.cmd(None).unwrap_or(0);
        // if cmd != 0xFF && cmd != 0x13 && cmd != 0x25 && cmd != 0x15 && cmd != 0x27 {
        //     println!("Report arrived: {:02X?} {:02X?}", header.bytes.vec(0, None, None).unwrap_or(Vec::new()), data);
        // }
        if echo == 0x00 && cmd == 0xff {
            let broadcast = &mut BroadCast::new(RwBytes::new(data));
            self.broadcast.clone()(self.handle, broadcast);
        } else {
            self.on_response_arrived(header, data);
        }
    }

    fn on_response_arrived(&mut self, header: HidReportHeader, data: Vec<u8>) {
        let handle = (
            header.report_id(None).unwrap_or(0),
            header.cmd(None).unwrap_or(0),
            header.index(None).unwrap_or(0),
        );

        if let (Some(cmd), Some(screen_cmd)) = (header.cmd(None), ScreenBuffer::CMD) {
            if cmd == screen_cmd {
                if let Err(e) = self.fill_screen_buffer(data) {
                    println!("Failed to fill screen buffer: {}", e);
                }
                return;
            }
        }

        // let cmd = header.cmd(None).expect("Can not get cmd for header in on_response_arrived");
        // if cmd != 0x13 && cmd != 0x14 && cmd != 0x15 {
        //     println!("package arrived: {:02X?} {:02X?}", header.into_vec(), data);
        // }
        let mut waiter_channels = match self.waiter_channels.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };
        let waiters = waiter_channels.get_mut(&handle);
        let waiter = match waiters {
            Some(waiters) => {
                //println!("Waiter list found length: {:?} {:02X?}", waiters.len(), header.into_vec());
                let mut waiter = None;
                while !waiters.is_empty() {
                    if let Some(tx) = waiters.pop_front() {
                        if !tx.is_canceled() {
                            waiter = Some(tx);
                            break;
                        }
                    }
                }
                waiter
            }
            None => {
                println!("No waiter list found for: {:02X?}", handle.1);
                return;
            }
        };
        drop(waiter_channels);
        match waiter {
            Some(tx) => {
                //_ = tx.send((header, data));
                match tx.send((header, data)) {
                    Ok(_) => (), //println!("tx sent"),
                    Err(err) => println!("tx send Error: {:?}", err),
                }
            }
            None => println!("No waiter found"),
        };
    }

    //add a request to the waiter list
    pub fn request_response<T: CodecableHidPackage>(
        &self,
        report_id: u8,
        cmd: u8,
        index: u8,
    ) -> impl Future<Output = Result<(HidReportHeader, T), ReportError>> + use<T> {
        let handle = (report_id, cmd, index);
        //println!("Request response: {:02X?}", handle);
        let (tx, rx) = oneshot::channel::<(HidReportHeader, Vec<u8>)>();
        let mut waiter_channels = match self.waiter_channels.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };
        let waiters = waiter_channels.entry(handle).or_insert(VecDeque::new());
        waiters.push_back(tx);
        // if cmd != 0x25 && cmd != 0x14 && cmd != 0x15 && cmd != 0x1C {
        //     println!("tx added to waiter list length: {:?}", waiters.len());
        // }

        drop(waiter_channels);

        async move {
            let timeout = future_delay(TIMEOUT_MS);
            // Box::pin to make the timeout future Unpin for select on Android
            let rx_timeout = futures::future::select(rx, timeout);

            let rx_res = rx_timeout.await;

            let rx_data = match rx_res {
                Either::Left((rx_data, _)) => rx_data,
                Either::Right(_) => {
                    println!("request_response Timeout {:02X?}", handle);
                    return Err(ReportError::Timeout);
                }
            };

            let res = match rx_data {
                Ok((header, data)) => {
                    //println!("rx received {:02X?} ", header.into_vec());
                    let mut res = T::new(RwBytes::new(data));

                    // 使用更安全的方式处理 StringContent
                    if T::CMD == StringContent::CMD {
                        // 这里需要一个更安全的方式来设置 encoding_byte
                        // 暂时使用 unsafe，但应该在 StringContent 中添加安全的设置方法
                        if let Some(status) = header.status(None) {
                            unsafe {
                                let str_content =
                                    transmute::<&mut T, &mut StringContent>(res.borrow_mut());
                                str_content.encoding_byte.set(Some(status));
                            }
                        } else {
                            return Err(ReportError::BadReportHeader);
                        }
                    }
                    (header, res)
                }
                Err(_) => {
                    println!("rx Error");
                    return Err(ReportError::ChannelError);
                }
            };
            //println!("tx received");
            Ok(res)
        }
    }
}

fn get_crc16(data: &[u8]) -> u16 {
    let mut crc: u16 = 0x0000;
    for i in 0..data.len() {
        crc = crc.wrapping_add(match i % 2 {
            0 => data[i] as u16,
            _ => (data[i] as u16) << 8,
        });
    }
    return crc;
}

// pub fn encode_end_report(report_id: u8, echo: u8, cmd: u8, index: u8) -> Vec<Vec<u8>> {
//     let mut reports: Vec<Vec<u8>> = Vec::new();
//     let mut header = HidReportHeader::new(RwBytes::new(vec![0; 8]));
//     header.report_id(Some(report_id));
//     header.echo(Some(echo));
//     header.cmd(Some(cmd));
//     header.index(Some(index));
//     header.status(Some(0x00));
//     header.len(Some(0x00));
//     let mut data = header.into_vec();
//     let crc = get_crc16(&data);
//     data[2] = crc as u8;
//     data[3] = (crc >> 8) as u8;
//     reports.push(data);
//     return reports;
// }

pub fn encode_report<T: CodecableHidPackage>(
    report_id: u8,
    echo: u8,
    cmd: u8,
    index: u8,
    value: &T,
) -> Result<Vec<Vec<u8>>, ReportError> {
    //println!("Encoding report: {:02X?}", report_id);
    let max_package_len = match report_id {
        REPORT_ID_21 => MAX_PACKAGE_LEN_21,
        REPORT_ID_22 => MAX_PACKAGE_LEN_22,
        _ => return Err(ReportError::UnsupportedReportId(report_id)),
    };
    let value_bytes = value.into_vec();
    let mut reports: Vec<Vec<u8>> = Vec::new();

    let mut packaged_len = 0;
    while packaged_len < value_bytes.len() || packaged_len == 0 {
        let status = if packaged_len + max_package_len >= value_bytes.len() {
            match T::CMD {
                StringContent::CMD => unsafe {
                    let str_content = transmute::<&T, &StringContent>(value);
                    str_content
                        .encoding_byte
                        .get()
                        .ok_or(ReportError::BadEncodingByte)?
                },
                _ => 0x00,
            }
        } else {
            0x01
        };

        let body_len = min(max_package_len, value_bytes.len() - packaged_len);

        let header = HidReportHeader::new(RwBytes::new(vec![0; HEADER_SIZE]));
        header.report_id(Some(report_id));
        header.echo(Some(echo));
        header.cmd(Some(cmd));
        header.index(Some(index));
        header.status(Some(status));
        // this should a single package length, not the whole value length
        header.len(Some((body_len + 0x04) as u16));
        //println!("cmd: {:?}", header.cmd(Some(cmd)));
        //println!("index: {:?}", header.index(Some(index)));
        //println!("status: {:?}", header.status(Some(status)));
        //println!("package_len: {:?}", header.len(Some((value_bytes.len() + 0x04) as u16)));

        let mut data = header.into_vec();
        let body = &value_bytes[packaged_len..packaged_len + body_len];
        data.extend(body);
        let crc = get_crc16(&data);
        //println!("report {:02X?}, crc: {:02X?}", data, crc);
        data[2] = crc as u8;
        data[3] = (crc >> 8) as u8;

        // 4字节对齐
        if data.len() % 4 != 0 {
            let padding = 4 - data.len() % 4;
            data.resize(data.len() + padding, 0);
        }

        packaged_len += body_len;
        reports.push(data);
        if packaged_len == 0 {
            break;
        }
    }

    Ok(reports)
}
