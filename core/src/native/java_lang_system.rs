use crate::runtime::heap::HeapValue;
use std::time::{SystemTime, UNIX_EPOCH};

pub fn invoke(
    method_name: &str,
    descriptor: &str,
    _args: &[HeapValue],
) -> Option<Option<HeapValue>> {
    match (method_name, descriptor) {
        ("currentTimeMillis", "()J") => {
            let millis = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_millis() as i64;
            Some(Some(HeapValue::Long(millis)))
        }
        _ => None,
    }
}
