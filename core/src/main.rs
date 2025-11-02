mod bytecode;
mod exec;
mod loader;
mod runtime;
mod native;

use crate::exec::interpreter::Interpreter;
use crate::loader::class_loader::ClassLoader;
use crate::runtime::heap::Heap;
use std::env;
use std::path::Path;

fn main() {
    println!("===============================");
    println!("☕ AriaJVM - Rust Implementation");
    println!("===============================");

    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: aria <MainClass or path/to/Main.class>");
        std::process::exit(1);
    }

    let target = &args[1];

    let mut loader = ClassLoader::new();
    println!("📦 Loading class: {}", target);

    let class_file = if Path::new(target).exists() {
        loader.load_class_from_file(target)
    } else {
        loader.load_class(target)
    };

    let class_file = match class_file {
        Ok(c) => {
            println!("✅ Class loaded: {}", target);
            c
        }
        Err(e) => {
            eprintln!("❌ Failed to load class: {e}");
            std::process::exit(1);
        }
    };

    let interp = Interpreter::new(true);
    let mut heap = Heap::new();

    println!("Executing main() ...");
    let result = interp.execute_method(
        &mut loader,
        &class_file,
        "main",
        "([Ljava/lang/String;)V",
        &mut heap,
    );

    match result {
        Some(v) => println!("Execution finished, return: {:?}", v),
        None => println!("Execution finished (void return)"),
    }

    println!("🧹 Program terminated gracefully.");
}
