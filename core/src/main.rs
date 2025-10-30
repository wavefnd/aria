pub mod bytecode;
pub mod exec;
pub mod loader;
pub mod native;
pub mod runtime;

use crate::{exec::interpreter::Interpreter, loader::class_loader::ClassLoader};

fn main() {
    let mut loader = ClassLoader::new();
    loader.add_classpath(".");
    loader.add_classpath("./lib/modules/java.base");

    loader.preload_core_classes();

    match loader.load_class("Main") {
        Ok(class_file) => {
            println!("✅ Class loaded: Main");
            Interpreter::execute_method(&mut loader, &class_file, "main", "([Ljava/lang/String;)V");
        }
        Err(err) => eprintln!("❌ {}", err),
    }
}