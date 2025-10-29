pub mod bytecode;
pub mod exec;
pub mod loader;
pub mod native;
pub mod runtime;

use bytecode::parser::ClassFile;

fn main() {
    let path = "Main.class"; // Test
    match ClassFile::parse(path) {
        Ok(class_file) => {
            println!("✅ Parsed class successfully:");
            println!("Magic: 0x{:X}", class_file.magic);
            println!("Version: {}.{}", class_file.major_version, class_file.minor_version);
            println!("Constant Pool Count: {}", class_file.constant_pool_count);
        }
        Err(err) => eprintln!("❌ Failed to parse: {}", err),
    }
}
