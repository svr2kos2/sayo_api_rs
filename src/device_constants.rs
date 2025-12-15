// 设备广播消息类型常量
pub const BROADCAST_TYPE_SYS_CMD: u8 = 0x01;
pub const BROADCAST_TYPE_LED: u8 = 0x02;
pub const BROADCAST_TYPE_FN_LAYER: u8 = 0x03;
pub const BROADCAST_TYPE_CPU_LOAD: u8 = 0x04;
pub const BROADCAST_TYPE_PROFILE: u8 = 0x05;
pub const BROADCAST_TYPE_KEY_PRESS: u8 = 0x10;
pub const BROADCAST_TYPE_KEY_RELEASE: u8 = 0x11;
pub const BROADCAST_TYPE_HALL_KEY_RELOAD: u8 = 0x19;
pub const BRD_TYPE_SYS_TIME_MS: u8 = 0x80;
pub const BRD_TYPE_SYS_TIME: u8 = 0xC0;
pub const BROADCAST_TYPE_LEVELS: u8 = 0xE1;
pub const BROADCAST_TYPE_ERROR_MSG: u8 = 0xFE;
pub const BROADCAST_TYPE_LOG_MSG: u8 = 0xFF;

// 报告ID常量
pub const REPORT_ID_BOOTUP: u8 = 0x21;
pub const REPORT_ID_MAIN: u8 = 0x22;
pub const REPORT_ID_SLEEP: u8 = 0x23;

// 命令常量
pub const CMD_DEVICE_INFO: u8 = 0x00;
pub const CMD_DEVICE_NAME: u8 = 0x01;
pub const CMD_SYSTEM_INFO: u8 = 0x02;
pub const CMD_DEVICE_CONFIG: u8 = 0x03;
pub const CMD_RF_CONFIG: u8 = 0x04;
pub const CMD_REBOOT: u8 = 0x0E;
pub const CMD_SAVE_ALL: u8 = 0x0D;
pub const CMD_KEY_INFO: u8 = 0x10;
pub const CMD_LED_INFO: u8 = 0x11;
pub const CMD_COLOR_TABLE: u8 = 0x12;
pub const CMD_TOUCH_SENSITIVITY: u8 = 0x13;
pub const CMD_HALL_50UM: u8 = 0x15;
pub const CMD_PASSWORD: u8 = 0x16;
pub const CMD_STRING: u8 = 0x17;
pub const CMD_SCRIPT_NAME: u8 = 0x19;
pub const CMD_KEY_PHYSICAL_STATUS: u8 = 0x1E;
pub const CMD_LED_EFFECT: u8 = 0x26;

// 数据长度常量 - 修复类型匹配
pub const LEVELS_DATA_LEN_34: u8 = 34;
pub const LEVELS_DATA_LEN_35: u8 = 35;
pub const LEVELS_BUFFER_SIZE: usize = 1600;
pub const LEVEL_THRESHOLD: u16 = 50;
pub const LEVEL_MASK: u16 = 0x3FFF;

// 重试和超时常量 - 修复类型匹配
pub const MAX_RETRY_COUNT: usize = 8;
pub const SEND_TIMEOUT_MS: u32 = 1000; // 改为u32
pub const MAX_PACKET_LEN_REPORT_21: usize = 64 - 12;
pub const MAX_PACKET_LEN_REPORT_22: usize = 1024 - 12;
pub const ADDR_ALIGNMENT: usize = 4096;

// 状态码常量
pub const STATUS_OK: u8 = 0x00;
pub const STATUS_PARTIAL: u8 = 0x02;
pub const STATUS_COMPLETE: u8 = 0x03;
pub const STATUS_OVERFLOW: u8 = 0x11;

// 子命令常量
pub const SUBCMD_REBOOT: u8 = 0x01;
pub const SUBCMD_RECOVERY: u8 = 0xFE;
pub const SUBCMD_BOOTLOADER: u8 = 0xFF;
pub const REBOOT_MAGIC: u16 = 0x7296;
