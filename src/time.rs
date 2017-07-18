use std::time::{SystemTime, UNIX_EPOCH};

pub fn millis() -> u64 {
    let t = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
    t.as_secs() * 1000 + (t.subsec_nanos() as u64) / 1000000
}

pub fn micros() -> u64 {
    let t = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
    t.as_secs() * 1000000 + (t.subsec_nanos() as u64) / 1000
}
