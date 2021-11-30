use std::time::SystemTime;

pub const US_IN_SEC: u64 = 1000 * 1000;

pub fn get_timestamp_us() -> u64 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH).unwrap()
        .as_micros() as u64
}