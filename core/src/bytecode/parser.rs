use super::reader::ClassReader;

pub const JAVA_MAGIC: u32 = 0xCAFEBABE;

#[derive(Debug)]
pub enum ConstantPoolEntry {
    Utf8(String),
    Integer(i32),
    Float(f32),
    Long(i64),
    Double(f64),
    Class { name_index: u16 },
    String { string_index: u16 },
    FieldRef { class_index: u16, name_and_type_index: u16 },
    MethodRef { class_index: u16, name_and_type_index: u16 },
    InterfaceMethodRef { class_index: u16, name_and_type_index: u16 },
    NameAndType { name_index: u16, descriptor_index: u16 },
    Unknown(u8),
}

#[derive(Debug)]
pub struct ClassFile {
    pub magic: u32,
    pub minor_version: u16,
    pub major_version: u16,
    pub constant_pool_count: u16,
    pub constant_pool: Vec<ConstantPoolEntry>,
    pub methods: Vec<MethodInfo>,
    // pub fields: Vec<FieldInfo>,
    // pub interfaces: Vec<u16>,
    // pub attributes: Vec<AttributeInfo>,
}

impl ClassFile {
    pub fn parse(path: &str) -> Result<Self, String> {
        let mut reader = ClassReader::from_file(path)
            .map_err(|e| format!("Failed to read class file: {}", e))?;

        let magic = reader.read_u4();
        if magic != JAVA_MAGIC {
            return Err(format!("Invalid magic number: {:X}", magic));
        }

        let minor_version = reader.read_u2();
        let major_version = reader.read_u2();
        let constant_pool_count = reader.read_u2();

        let mut constant_pool = Vec::new();
        let mut i = 1;
        while i < constant_pool_count {
            let tag = reader.read_u1();
            let entry = match tag {
                1 => {
                    let length = reader.read_u2() as usize;
                    let bytes: Vec<u8> = (0..length).map(|_| reader.read_u1()).collect();
                    let string = String::from_utf8_lossy(&bytes).to_string();
                    ConstantPoolEntry::Utf8(string)
                }
                3 => ConstantPoolEntry::Integer(reader.read_u4() as i32),
                4 => ConstantPoolEntry::Float(f32::from_bits(reader.read_u4())),
                5 => {
                    let high = reader.read_u4() as u64;
                    let low = reader.read_u4() as u64;
                    ConstantPoolEntry::Long(((high << 32) | low) as i64)
                }
                6 => {
                    let high = reader.read_u4() as u64;
                    let low = reader.read_u4() as u64;
                    ConstantPoolEntry::Double(f64::from_bits((high << 32) | low))
                }
                7 => ConstantPoolEntry::Class { name_index: reader.read_u2() },
                8 => ConstantPoolEntry::String { string_index: reader.read_u2() },
                9 => ConstantPoolEntry::FieldRef { 
                    class_index: reader.read_u2(), 
                    name_and_type_index: reader.read_u2(), 
                },
                10 => ConstantPoolEntry::MethodRef {
                    class_index: reader.read_u2(),
                    name_and_type_index: reader.read_u2(),
                },
                11 => ConstantPoolEntry::InterfaceMethodRef { 
                    class_index: reader.read_u2(), 
                    name_and_type_index: reader.read_u2(), 
                },
                12 => ConstantPoolEntry::NameAndType { 
                    name_index: reader.read_u2(), 
                    descriptor_index: reader.read_u2(), 
                },
                _ => ConstantPoolEntry::Unknown(tag),
            };

            if matches!(entry, ConstantPoolEntry::Long(_) | ConstantPoolEntry::Double(_)) {
                i += 2;
            } else {
                i += 1;
            }

            constant_pool.push(entry);
        }

        let access_flags = reader.read_u2();
        let this_class = reader.read_u2();
        let super_class = reader.read_u2();

        let interfaces_count = reader.read_u2();
        for _ in 0..interfaces_count {
            reader.read_u2();
        }
        let fields_count = reader.read_u2();
        for _ in 0..fields_count {
            let attr_count = reader.read_u2();
            for _ in 0..attr_count {
                reader.read_u2();
                let attr_len = reader.read_u4();
                reader.skip(attr_len as usize);
            }
        }

        let methods_count = reader.read_u2();
        let mut methods = Vec::new();
        for _ in 0..methods_count {
            let access_flags = reader.read_u2();
            let name_index = reader.read_u2();
            let descriptor_index = reader.read_u2();
            let attributes_count = reader.read_u2();
            let mut code: Option<CodeAttribute> = None;

            for _ in 0..attributes_count {
                let name_index_attr = reader.read_u2();
                let attr_len = reader.read_u4();

                let attr_name = {
                    if let Some(ConstantPoolEntry::Utf8(s)) = constant_pool.get((name_index_attr - 1) as usize) {
                        s.clone()
                    } else {
                        String::new()
                    }
                };

                if attr_name == "Code" {
                    let max_stack = reader.read_u2();
                    let max_locals = reader.read_u2();
                    let code_length = reader.read_u4() as usize;
                    let mut code_bytes = vec![0u8; code_length];
                    for i in 0..code_length {
                        code_bytes[i] = reader.read_u1();
                    }

                    let ex_table_len = reader.read_u2();
                    for _ in 0..ex_table_len {
                        reader.skip(8);
                    }
                    let code_attr_count = reader.read_u2();
                    for _ in 0..code_attr_count {
                        reader.read_u2();
                        let len = reader.read_u4();
                        reader.skip(len as usize);
                    }

                    code = Some(CodeAttribute {
                        max_stack,
                        max_locals,
                        code: code_bytes,
                    });
                } else {
                    reader.skip(attr_len as usize);
                }
            }

            methods.push(MethodInfo {
                access_flags,
                name_index,
                descriptor_index,
                code,
            });
        }

        let attributes_count = reader.read_u2();
        for _ in 0..attributes_count {
            reader.read_u2();
            let attr_len = reader.read_u4();
            reader.skip(attr_len as usize);
        }

        Ok(Self {
            magic,
            minor_version,
            major_version,
            constant_pool_count,
            constant_pool,
            methods,
        })
    }
}

#[derive(Debug)]
pub struct MethodInfo {
    pub access_flags: u16,
    pub name_index: u16,
    pub descriptor_index: u16,
    pub code: Option<CodeAttribute>,
}

#[derive(Debug)]
pub struct CodeAttribute {
    pub max_stack: u16,
    pub max_locals: u16,
    pub code: Vec<u8>,
}