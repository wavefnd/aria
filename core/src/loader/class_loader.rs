use std::path::{Path, PathBuf};
use std::fs;

use crate::bytecode::parser::*;

pub struct ClassLoader {
    search_paths: Vec<PathBuf>,
}

impl ClassLoader {
    pub fn new() -> Self {
        Self {
            search_paths: vec![PathBuf::from(".")],
        }
    }

    pub fn add_classpath<P: AsRef<Path>>(&mut self, path: P) {
        self.search_paths.push(path.as_ref().to_path_buf());
    }

    pub fn load_class(&self, class_name: &str) -> Result<ClassFile, String> {
        let file_path = class_name.replace(".", "/") + ".class";

        for base in &self.search_paths {
            let candidate = base.join(&file_path);
            if candidate.exists() {
                println!("Loading class: {}", candidate.display());
                return ClassFile::parse(candidate.to_str().unwrap());
            }
        }

        Err(format!("Class not found: {}", class_name))
    }
}