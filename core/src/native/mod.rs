pub mod java_io_printstream;
pub mod java_lang_math;
pub mod java_lang_system;

use crate::runtime::heap::{Heap, HeapValue};

pub fn invoke_native(
    class_name: &str,
    method_name: &str,
    descriptor: &str,
    receiver: Option<HeapValue>,
    args: &[HeapValue],
    heap: &mut Heap,
) -> Option<Option<HeapValue>> {
    match class_name {
        "java/lang/Object" if method_name == "<init>" && descriptor == "()V" => Some(None),
        "java/lang/System" => java_lang_system::invoke(method_name, descriptor, args),
        "java/lang/Math" => java_lang_math::invoke(method_name, descriptor, args),
        "java/io/PrintStream" => {
            java_io_printstream::invoke(method_name, descriptor, receiver.as_ref(), args, heap)
        }
        _ => None,
    }
}
