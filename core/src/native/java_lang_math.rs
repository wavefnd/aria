use crate::runtime::frame::Frame;
use crate::runtime::heap::HeapValue;

pub fn invoke(method_name: &str, descriptor: &str, frame: &mut Frame) -> bool {
    match (method_name, descriptor) {
        ("abs", "(I)I") => {
            let val = frame.pop();
            let result = val.abs();
            frame.push(result.clone());
            println!("[native] Math.abs({}) = {}", val, result);
            true
        }
        ("currentTimeMillis", "()J") => {
            use std::time::{SystemTime, UNIX_EPOCH};
            let millis = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_millis() as i64;
            frame.push(HeapValue::Long(millis));
            println!("[native] System.currentTimeMillis() = {}", millis);
            true
        }
        _ => false,
    }
}
