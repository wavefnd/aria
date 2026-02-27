use crate::bytecode::parser::*;
use crate::runtime::heap::HeapValue;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ClassInitState {
    Initializing,
    Initialized,
}

pub struct ClassLoader {
    search_paths: Vec<PathBuf>,
    pub loaded_classes: HashMap<String, ClassFile>,
    static_fields: HashMap<String, HeapValue>,
    class_init_state: HashMap<String, ClassInitState>,
}

impl ClassLoader {
    pub fn new() -> Self {
        Self {
            search_paths: vec![PathBuf::from(".")],
            loaded_classes: HashMap::new(),
            static_fields: HashMap::new(),
            class_init_state: HashMap::new(),
        }
    }

    pub fn load_class_from_file<P: AsRef<Path>>(&mut self, path: P) -> Result<ClassFile, String> {
        let path_ref = path.as_ref();
        if !path_ref.exists() {
            return Err(format!("Class file not found: {}", path_ref.display()));
        }

        let path_str = path_ref.to_string_lossy().to_string();
        match ClassFile::parse(&path_str) {
            Ok(class) => {
                if let Some(name) = class.get_class_name(class.this_class) {
                    self.init_static_fields_for_class(name, &class);
                    self.loaded_classes.insert(name.to_string(), class.clone());
                }
                Ok(class)
            }
            Err(e) => Err(format!("Failed to parse class file: {}", e)),
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
                println!("Loading class: {}", candidate.display());
                let path = candidate.to_string_lossy().to_string();
                let class_file =
                    ClassFile::parse(&path).map_err(|e| format!("Parse error: {}", e))?;

                let internal_name = class_file
                    .get_class_name(class_file.this_class)
                    .unwrap_or(class_name);
                self.init_static_fields_for_class(internal_name, &class_file);

                if let Some(super_name) = class_file.get_class_name(class_file.super_class) {
                    if super_name != "java/lang/Object" {
                        let _ = self.load_class(super_name);
                    }
                }

                self.loaded_classes
                    .insert(class_name.to_string(), class_file.clone());
                self.loaded_classes
                    .insert(internal_name.to_string(), class_file.clone());
                return Ok(class_file);
            }
        }

        Err(format!("Class not found: {}", class_name))
    }

    pub fn preload_core_classes(&mut self) {
        for cls in [
            "java/lang/Object",
            "java/lang/String",
            "java/lang/System",
            "java/io/PrintStream",
        ] {
            let _ = self.load_class(cls);
        }
    }

    pub fn get_static_field(&self, class_name: &str, field_name: &str) -> Option<HeapValue> {
        self.static_fields
            .get(&Self::static_field_key(class_name, field_name))
            .cloned()
    }

    pub fn set_static_field(&mut self, class_name: &str, field_name: &str, value: HeapValue) {
        self.static_fields
            .insert(Self::static_field_key(class_name, field_name), value);
    }

    pub fn begin_class_init(&mut self, class_name: &str) -> bool {
        match self.class_init_state.get(class_name) {
            Some(ClassInitState::Initializing) | Some(ClassInitState::Initialized) => false,
            None => {
                self.class_init_state
                    .insert(class_name.to_string(), ClassInitState::Initializing);
                true
            }
        }
    }

    pub fn finish_class_init(&mut self, class_name: &str) {
        self.class_init_state
            .insert(class_name.to_string(), ClassInitState::Initialized);
    }

    fn static_field_key(class_name: &str, field_name: &str) -> String {
        format!("{}::{}", class_name, field_name)
    }

    fn init_static_fields_for_class(&mut self, class_name: &str, class: &ClassFile) {
        for field in &class.fields {
            if (field.access_flags & 0x0008) == 0 {
                continue;
            }
            let field_name = class.get_utf8(field.name_index).unwrap_or("");
            let descriptor = class.get_utf8(field.descriptor_index).unwrap_or("");
            let key = Self::static_field_key(class_name, field_name);
            self.static_fields
                .entry(key)
                .or_insert_with(|| Self::default_value_for_descriptor(descriptor));
        }
    }

    fn default_value_for_descriptor(descriptor: &str) -> HeapValue {
        match descriptor.chars().next() {
            Some('Z') | Some('B') | Some('C') | Some('S') | Some('I') => HeapValue::Int(0),
            Some('J') => HeapValue::Long(0),
            Some('F') => HeapValue::Float(0.0),
            Some('D') => HeapValue::Double(0.0),
            _ => HeapValue::Null,
        }
    }
}
