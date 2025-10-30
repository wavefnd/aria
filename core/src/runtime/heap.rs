use std::{collections::HashMap, fmt, str};

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

impl fmt::Display for HeapValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HeapValue::Int(v) => write!(f, "{}", v),
            HeapValue::Long(v) => write!(f, "{}", v),
            HeapValue::Float(v) => write!(f, "{}", v),
            HeapValue::Double(v) => write!(f, "{}", v),
            HeapValue::Object(o) => write!(f, "[Object {}]", o.class_name),
            HeapValue::String(s) => write!(f, "{}", s),
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
            _ => 0,
        }
    }

    pub fn as_long(&self) -> i64 {
        match self {
            HeapValue::Int(v) => *v as i64,
            HeapValue::Long(v) => *v,
            _ => 0,
        }
    }

    pub fn abs(&self) -> HeapValue {
        match self {
            HeapValue::Int(v) => HeapValue::Int(v.abs()),
            HeapValue::Long(v) => HeapValue::Long(v.abs()),
            _ => HeapValue::Null,
        }
    }
}

pub struct Heap {
    next_id: u64,
    objects: HashMap<u64, ObjectRef>,
    pub string_pool: HashMap<String, u64>,
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
        let obj = ObjectRef {
            id,
            class_name: class_name.to_string(),
            fields: HashMap::new(),
        };
        self.objects.insert(id, obj.clone());
        obj
    }

    pub fn alloc_string(&mut self, value: &str) -> HeapValue {
        if let Some(&id) = self.string_pool.get(value) {
            HeapValue::Object(self.objects.get(&id).unwrap().clone())
        } else {
            let obj = self.alloc_object("java/lang/String");
            self.objects.insert(obj.id, obj.clone());
            self.string_pool.insert(value.to_string(), obj.id);
        
            let object_mut = self.objects.get_mut(&obj.id).unwrap();
            object_mut.fields.insert(
                "value".to_string(),
                HeapValue::String(value.to_string()),
            );
        
            println!("NEW java/lang/String(\"{}\") -> ref#{}", value, obj.id);
            HeapValue::Object(obj)
        }
    }

    pub fn get(&self, id: u64) -> Option<&ObjectRef> {
        self.objects.get(&id)
    }

    pub fn get_mut(&mut self, id: u64) -> Option<&mut ObjectRef> {
        self.objects.get_mut(&id)
    }
}