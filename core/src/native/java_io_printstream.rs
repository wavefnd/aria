use crate::runtime::heap::{Heap, HeapValue};

pub fn invoke(
    method_name: &str,
    descriptor: &str,
    receiver: Option<&HeapValue>,
    args: &[HeapValue],
    heap: &mut Heap,
) -> Option<Option<HeapValue>> {
    if !matches!(receiver, Some(HeapValue::Object(_))) {
        return None;
    }
    match (method_name, descriptor) {
        ("println", "(I)V") => {
            let val = args.first().map(|v| v.as_int()).unwrap_or(0);
            println!("{}", val);
            Some(None)
        }
        ("println", "(J)V") => {
            let val = match args.first() {
                Some(HeapValue::Long(v)) => *v,
                Some(v) => v.as_long(),
                None => 0,
            };
            println!("{}", val);
            Some(None)
        }
        ("println", "(Ljava/lang/String;)V") => {
            let rendered = match args.first() {
                Some(HeapValue::Object(obj)) if obj.class_name == "java/lang/String" => heap
                    .get(obj.id)
                    .and_then(|real| real.get_field("value"))
                    .map(|v| match v {
                        HeapValue::String(s) => s.clone(),
                        other => other.to_string(),
                    })
                    .unwrap_or_else(|| "<string>".to_string()),
                Some(HeapValue::String(s)) => s.clone(),
                Some(v) => v.to_string(),
                None => "".to_string(),
            };
            println!("{}", rendered);
            Some(None)
        }
        _ => None,
    }
}
