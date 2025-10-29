// src/lib.rs
//
// Aria Core - Dummy build test module
// -----------------------------------

#[no_mangle]
pub extern "C" fn aria_core_test() -> i32 {
    println!("Aria Core staticlib build test successful!");
    42 // 더미 리턴값
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dummy() {
        assert_eq!(aria_core_test(), 42);
    }
}
