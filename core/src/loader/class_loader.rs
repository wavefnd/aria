use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::fs;

use crate::bytecode::parser::*;

pub struct ClassLoader {
    search_paths: Vec<PathBuf>,
    pub loaded_classes: HashMap<String, ClassFile>,
}

impl ClassLoader {
    pub fn new() -> Self {
        Self {
            search_paths: vec![PathBuf::from(".")],
            loaded_classes: HashMap::new(),
        }
    }

    pub fn add_classpath<P: AsRef<Path>>(&mut self, path: P) {
        self.search_paths.push(path.as_ref().to_path_buf());
    }

    pub fn load_class(&mut self, class_name: &str) -> Result<ClassFile, String> {
        if let Some(cached) = self.loaded_classes.get(class_name) {
            return Ok(cached.clone());
        }
    
        let file_path = class_name.replace('.', "/") + ".class";
        for base in &self.search_paths {
            let candidate = base.join(&file_path);
            if candidate.exists() {
                println!("📦 Loading class: {}", candidate.display());
                let class_file = ClassFile::parse(candidate.to_str().unwrap())
                    .map_err(|e| format!("Parse error: {}", e))?;
            
                if let Some(super_name) = class_file.get_class_name(class_file.super_class) {
                    if super_name != "java/lang/Object" {
                        let _ = self.load_class(super_name);
                    }
                }
            
                self.loaded_classes.insert(class_name.to_string(), class_file.clone());
                return Ok(class_file);
            }
        }
    
        Err(format!("Class not found: {}", class_name))
    }

    pub fn preload_core_classes(&mut self) {
        for cls in ["java/lang/Object", "java/lang/String", "java/lang/System", "java/io/PrintStream"] {
            let _ = self.load_class(cls);
        }
    }
}