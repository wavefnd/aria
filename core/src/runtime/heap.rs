use std::{collections::HashMap, fmt};

#[derive(Debug, Clone)]
pub enum HeapValue {
    Int(i32),
    Long(i64),
    Float(f32),
    Double(f64),
    Object(ObjectRef),
    String(String),
    Null,
}

#[derive(Debug, Clone)]
pub struct ObjectRef {
    pub id: u64,
    pub class_name: String,
    pub fields: HashMap<String, HeapValue>,
}

impl ObjectRef {
    pub fn new(id: u64, class_name: &str) -> Self {
        Self {
            id,
            class_name: class_name.to_string(),
            fields: HashMap::new(),
        }
    }

    pub fn get_field(&self, name: &str) -> Option<&HeapValue> {
        self.fields.get(name)
    }

    pub fn set_field(&mut self, name: &str, value: HeapValue) {
        self.fields.insert(name.to_string(), value);
    }
}

impl fmt::Display for HeapValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HeapValue::Int(v) => write!(f, "{}", v),
            HeapValue::Long(v) => write!(f, "{}", v),
            HeapValue::Float(v) => write!(f, "{}", v),
            HeapValue::Double(v) => write!(f, "{}", v),
            HeapValue::Object(o) => write!(f, "[Object {}#{}]", o.class_name, o.id),
            HeapValue::String(s) => write!(f, "\"{}\"", s),
            HeapValue::Null => write!(f, "null"),
        }
    }
}

impl HeapValue {
    pub fn as_int(&self) -> i32 {
        match self {
            HeapValue::Int(v) => *v,
            HeapValue::Long(v) => *v as i32,
            HeapValue::Float(v) => *v as i32,
            HeapValue::Double(v) => *v as i32,
            _ => {
                println!("TypeError: tried to read {:?} as Int", self);
                0
            }
        }
    }

    pub fn as_long(&self) -> i64 {
        match self {
            HeapValue::Long(v) => *v,
            HeapValue::Int(v) => *v as i64,
            _ => {
                println!("TypeError: tried to read {:?} as Long", self);
                0
            }
        }
    }

    pub fn abs(&self) -> HeapValue {
        match self {
            HeapValue::Int(v) => HeapValue::Int(v.abs()),
            HeapValue::Long(v) => HeapValue::Long(v.abs()),
            _ => HeapValue::Null,
        }
    }

    pub fn is_null(&self) -> bool {
        matches!(self, HeapValue::Null)
    }

    pub fn is_object(&self) -> bool {
        matches!(self, HeapValue::Object(_))
    }
}

pub struct Heap {
    next_id: u64,
    pub(crate) objects: HashMap<u64, ObjectRef>,
    string_pool: HashMap<String, u64>,
}

impl Heap {
    pub fn new() -> Self {
        Self {
            next_id: 1,
            objects: HashMap::new(),
            string_pool: HashMap::new(),
        }
    }

    pub fn alloc_object(&mut self, class_name: &str) -> ObjectRef {
        let id = self.next_id;
        self.next_id += 1;

        let obj = ObjectRef::new(id, class_name);
        self.objects.insert(id, obj.clone());

        println!("NEW {} -> ref#{}", class_name, id);
        obj
    }

    pub fn alloc_string(&mut self, value: &str) -> HeapValue {
        if let Some(&id) = self.string_pool.get(value) {
            if let Some(obj) = self.objects.get(&id) {
                return HeapValue::Object(obj.clone());
            }
        }

        let mut obj = self.alloc_object("java/lang/String");
        obj.set_field("value", HeapValue::String(value.to_string()));
        let id = obj.id;

        self.string_pool.insert(value.to_string(), id);
        self.objects.insert(id, obj.clone());

        println!("NEW java/lang/String(\"{}\") -> ref#{}", value, id);
        HeapValue::Object(obj)
    }

    pub fn get(&self, id: u64) -> Option<&ObjectRef> {
        self.objects.get(&id)
    }

    pub fn get_mut(&mut self, id: u64) -> Option<&mut ObjectRef> {
        self.objects.get_mut(&id)
    }

    pub fn dump_objects(&self) {
        println!("==== HEAP OBJECTS ====");
        for (id, obj) in &self.objects {
            println!("#{}: {} => {:?}", id, obj.class_name, obj.fields);
        }
        println!("======================");
    }

    pub fn dump_strings(&self) {
        println!("==== STRING POOL ====");
        for (s, id) in &self.string_pool {
            println!("\"{}\" -> ref#{}", s, id);
        }
        println!("=====================");
    }
}
