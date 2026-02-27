fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    std::process::exit(aria_core::run_cli(&args));
}
