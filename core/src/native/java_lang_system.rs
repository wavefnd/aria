use crate::runtime::frame::Frame;
use crate::runtime::heap::HeapValue;
use std::time::{SystemTime, UNIX_EPOCH};

pub fn invoke(method_name: &str, descriptor: &str, frame: &mut Frame) -> bool {
    match (method_name, descriptor) {
        ("currentTimeMillis", "()J") => {
            let millis = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_millis() as i64;
            frame.push(HeapValue::Null);
            println!("[native] System.currentTimeMillis() = {}", millis);
            true
        }
        _ => false,
    }
}