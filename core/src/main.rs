pub mod bytecode;
pub mod exec;
pub mod loader;
pub mod native;
pub mod runtime;

use crate::{exec::interpreter::Interpreter, loader::class_loader::ClassLoader};

fn main() {
    let mut loader = ClassLoader::new();
    loader.add_classpath(".");

    match loader.load_class("Main") {
        Ok(class_file) => {
            println!("✅ Class loaded: Main");
            Interpreter::execute(&class_file);
        }
        Err(err) => eprintln!("❌ Error: {}", err),
    }
}