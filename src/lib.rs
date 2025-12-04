pub mod byte_converter;
pub mod cross_platform_utils;
pub mod device;
pub mod device_constants;
pub mod device_error_handling;
pub mod lock_manager;
pub mod report_codec;
pub mod structures;
pub mod structures_codec;
mod utility;

pub fn add(left: u64, right: u64) -> u64 {
    left + right
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }
}
