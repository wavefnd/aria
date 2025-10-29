pub mod bytecode;
pub mod exec;
pub mod loader;
pub mod native;
pub mod runtime;

use crate::loader::{class_loader::ClassLoader};

fn main() {
    let mut loader = ClassLoader::new();
    loader.add_classpath(".");

    match loader.load_class("Main") {
        Ok(class_file) => {
            println!("✅ Class loaded successfully!");
            println!(
                "Class version: {}.{}",
                class_file.major_version, class_file.minor_version
            );

            if let Some(name) = class_file.get_class_name(21) {
                println!("Class Name (from pool): {}", name);
            }

            if let Some(super_name) = class_file.get_class_name(2) {
                println!("Superclass: {}", super_name);
            }
        }
        Err(err) => eprintln!("❌ Error loading class: {}", err),
    }
}