use crate::runtime::{frame::Frame, heap::HeapValue};

pub fn invoke(method_name: &str, descriptor: &str, frame: &mut Frame) -> bool {
    match (method_name, descriptor) {
        ("println", "(I)V") => {
            if let HeapValue::Int(val) = frame.pop() {
                println!("{}", val);
            }
            true
        }
        ("println", "(J)V") => {
            if let HeapValue::Long(val) = frame.pop() {
                println!("{}", val);
            }
            true
        }
        ("println", "(Ljava/lang/String;)V") => {
            println!("[native] println(<string>)");
            true
        }
        _ => false,
    }
}