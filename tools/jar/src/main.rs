use std::process::Command;
use std::env;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: aria <MainClass>");
        return;
    }

    let class_name = &args[1];
    println!("Launching class: {}", class_name);

    Command::new("../core/target/release/corevm")
        .arg(class_name)
        .status()
        .expect("Failed to launch AriaJDK");
}
