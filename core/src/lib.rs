pub mod bytecode;
pub mod exec;
pub mod loader;
pub mod native;
pub mod runtime;

use crate::exec::interpreter::Interpreter;
use crate::loader::class_loader::ClassLoader;
use crate::runtime::heap::Heap;
use std::path::Path;

const ARIA_VERSION: &str = include_str!("../../VERSION");
const JAVA_VERSION: &str = include_str!("../../VERSION_JAVA");

fn print_banner() {
    println!("===============================");
    println!("AriaJVM - Rust Implementation");
    println!("===============================");
}

fn print_usage() {
    eprintln!("Usage: java [-version] [-cp <path>] <MainClass|path/to/Main.class>");
}

fn print_version() {
    let java_version = JAVA_VERSION.trim();
    let aria_version = ARIA_VERSION.trim();
    eprintln!("openjdk version \"{}\" aria", java_version);
    eprintln!("AriaJDK Runtime Environment (build {})", aria_version);
    eprintln!(
        "AriaJDK 64-Bit Server VM (build {}, interpreted mode)",
        aria_version
    );
}

fn classpath_separator() -> char {
    if cfg!(windows) {
        ';'
    } else {
        ':'
    }
}

pub fn run_cli(args: &[String]) -> i32 {
    if args.is_empty() {
        print_usage();
        return 1;
    }

    if args[0] == "-version" || args[0] == "--version" {
        print_version();
        return 0;
    }

    if args[0] == "-h" || args[0] == "--help" || args[0] == "-help" {
        print_usage();
        return 0;
    }

    let mut idx = 0usize;
    let mut classpath = vec![String::from(".")];
    let mut target: Option<String> = None;

    while idx < args.len() {
        let arg = &args[idx];
        match arg.as_str() {
            "-cp" | "-classpath" | "--class-path" => {
                idx += 1;
                if idx >= args.len() {
                    eprintln!("Missing value for classpath option: {}", arg);
                    return 1;
                }
                for entry in args[idx].split(classpath_separator()) {
                    if !entry.is_empty() {
                        classpath.push(entry.to_string());
                    }
                }
            }
            _ if arg.starts_with('-') => {
                eprintln!("Unsupported option: {}", arg);
                return 1;
            }
            _ => {
                target = Some(arg.clone());
                break;
            }
        }
        idx += 1;
    }

    let target = match target {
        Some(value) => value,
        None => {
            print_usage();
            return 1;
        }
    };

    print_banner();

    let mut loader = ClassLoader::new();
    for entry in classpath {
        loader.add_classpath(entry);
    }

    println!("Loading class: {}", target);
    let class_file = if Path::new(&target).exists() {
        loader.load_class_from_file(&target)
    } else {
        loader.load_class(&target)
    };

    let class_file = match class_file {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Failed to load class: {e}");
            return 1;
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
        &[],
    );

    match result {
        Some(v) => println!("Execution finished, return: {:?}", v),
        None => println!("Execution finished (void return)"),
    }

    0
}
