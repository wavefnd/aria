use crate::runtime::heap::HeapValue;

pub fn invoke(
    method_name: &str,
    descriptor: &str,
    args: &[HeapValue],
) -> Option<Option<HeapValue>> {
    match (method_name, descriptor) {
        ("abs", "(I)I") => {
            let val = args.first().cloned().unwrap_or(HeapValue::Int(0));
            Some(Some(val.abs()))
        }
        _ => None,
    }
}
