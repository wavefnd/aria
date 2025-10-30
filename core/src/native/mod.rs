pub mod java_lang_system;
pub mod java_lang_math;
pub mod java_io_printstream;

use crate::runtime::frame::Frame;

pub fn invoke_native(class_name: &str, method_name: &str, descriptor: &str, frame: &mut Frame) -> bool {
    match class_name {
        "java/lang/System" => java_lang_system::invoke(method_name, descriptor, frame),
        "java/lang/Math" => java_lang_math::invoke(method_name, descriptor, frame),
        "java/io/PrintStream" => java_io_printstream::invoke(method_name, descriptor, frame),
        _ => false,
    }
}