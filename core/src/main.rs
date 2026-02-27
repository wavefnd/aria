use std::env;

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    std::process::exit(aria_core::run_cli(&args));
}
