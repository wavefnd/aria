use super::reader::ClassReader;

pub const JAVA_MAGIC: u32 = 0xCAFEBABE;

#[derive(Debug, Clone)]
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
    MethodHandle { reference_kind: u8, reference_index: u16 },
    MethodType { descriptor_index: u16 },
    InvokeDynamic { bootstrap_method_attr_index: u16, name_and_type_index: u16 },
    Module { name_index: u16 },
    Package { name_index: u16 },
    Unknown(u8),
}

#[derive(Debug, Clone)]
pub struct ClassFile {
    pub magic: u32,
    pub minor_version: u16,
    pub major_version: u16,
    pub constant_pool_count: u16,
    pub constant_pool: Vec<ConstantPoolEntry>,
    pub access_flags: u16,
    pub this_class: u16,
    pub super_class: u16,
    pub interfaces: Vec<u16>,
    pub fields: Vec<FieldInfo>,
    pub methods: Vec<MethodInfo>,
    pub attributes: Vec<AttributeInfo>,
}

#[derive(Debug, Clone)]
pub struct FieldInfo {
    pub access_flags: u16,
    pub name_index: u16,
    pub descriptor_index: u16,
    pub attributes: Vec<AttributeInfo>,
}

#[derive(Debug, Clone)]
pub struct AttributeInfo {
    pub name_index: u16,
    pub info: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct MethodInfo {
    pub access_flags: u16,
    pub name_index: u16,
    pub descriptor_index: u16,
    pub code: Option<CodeAttribute>,
    pub attributes: Vec<AttributeInfo>,
}

#[derive(Debug, Clone)]
pub struct ExceptionTableEntry {
    pub start_pc: u16,
    pub end_pc: u16,
    pub handler_pc: u16,
    pub catch_type: u16,
}

#[derive(Debug, Clone)]
pub struct CodeAttribute {
    pub max_stack: u16,
    pub max_locals: u16,
    pub code: Vec<u8>,
    pub exception_table: Vec<ExceptionTableEntry>,
    pub attributes: Vec<AttributeInfo>,
}

impl ClassFile {
    pub fn parse(path: &str) -> Result<Self, String> {
        let mut reader = ClassReader::from_file(path)
            .map_err(|e| format!("Failed to read class file: {}", e))?;

        // Magic check
        let magic = reader.read_u4();
        if magic != JAVA_MAGIC {
            return Err(format!("Invalid magic number: {:X}", magic));
        }

        // Version info
        let minor_version = reader.read_u2();
        let major_version = reader.read_u2();

        // Constant pool
        let constant_pool_count = reader.read_u2();
        let mut constant_pool = Vec::with_capacity((constant_pool_count - 1) as usize);
        let mut i = 1;

        while i < constant_pool_count {
            let tag = reader.read_u1();
            let entry = match tag {
                1 => {
                    let length = reader.read_u2() as usize;
                    let bytes: Vec<u8> = (0..length).map(|_| reader.read_u1()).collect();
                    ConstantPoolEntry::Utf8(String::from_utf8_lossy(&bytes).to_string())
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
                15 => ConstantPoolEntry::MethodHandle {
                    reference_kind: reader.read_u1(),
                    reference_index: reader.read_u2(),
                },
                16 => ConstantPoolEntry::MethodType {
                    descriptor_index: reader.read_u2(),
                },
                18 => ConstantPoolEntry::InvokeDynamic {
                    bootstrap_method_attr_index: reader.read_u2(),
                    name_and_type_index: reader.read_u2(),
                },
                19 => ConstantPoolEntry::Module { name_index: reader.read_u2() },
                20 => ConstantPoolEntry::Package { name_index: reader.read_u2() },
                _ => ConstantPoolEntry::Unknown(tag),
            };

            if matches!(entry, ConstantPoolEntry::Long(_) | ConstantPoolEntry::Double(_)) {
                i += 2;
            } else {
                i += 1;
            }
            constant_pool.push(entry);
        }

        // Class info
        let access_flags = reader.read_u2();
        let this_class = reader.read_u2();
        let super_class = reader.read_u2();

        // Interfaces
        let interfaces_count = reader.read_u2();
        let mut interfaces = Vec::with_capacity(interfaces_count as usize);
        for _ in 0..interfaces_count {
            interfaces.push(reader.read_u2());
        }

        // Fields
        let fields_count = reader.read_u2();
        let mut fields = Vec::with_capacity(fields_count as usize);
        for _ in 0..fields_count {
            let access_flags = reader.read_u2();
            let name_index = reader.read_u2();
            let descriptor_index = reader.read_u2();
            let attributes_count = reader.read_u2();

            let mut attributes = Vec::with_capacity(attributes_count as usize);
            for _ in 0..attributes_count {
                let name_index = reader.read_u2();
                let attr_len = reader.read_u4();
                let mut info = vec![0u8; attr_len as usize];
                for i in 0..attr_len as usize {
                    info[i] = reader.read_u1();
                }
                attributes.push(AttributeInfo { name_index, info });
            }

            fields.push(FieldInfo {
                access_flags,
                name_index,
                descriptor_index,
                attributes,
            });
        }

        // Methods
        let methods_count = reader.read_u2();
        let mut methods = Vec::with_capacity(methods_count as usize);

        for _ in 0..methods_count {
            let access_flags = reader.read_u2();
            let name_index = reader.read_u2();
            let descriptor_index = reader.read_u2();
            let attributes_count = reader.read_u2();

            let mut code: Option<CodeAttribute> = None;
            let mut attributes = Vec::with_capacity(attributes_count as usize);

            for _ in 0..attributes_count {
                let name_index_attr = reader.read_u2();
                let attr_len = reader.read_u4();

                let attr_name = if let Some(ConstantPoolEntry::Utf8(s)) =
                    constant_pool.get((name_index_attr - 1) as usize)
                {
                    s.clone()
                } else {
                    String::new()
                };

                if attr_name == "Code" {
                    let max_stack = reader.read_u2();
                    let max_locals = reader.read_u2();
                    let code_length = reader.read_u4() as usize;

                    let mut code_bytes = vec![0u8; code_length];
                    for i in 0..code_length {
                        code_bytes[i] = reader.read_u1();
                    }

                    // Exception table
                    let ex_table_len = reader.read_u2();
                    let mut ex_table = Vec::with_capacity(ex_table_len as usize);
                    for _ in 0..ex_table_len {
                        ex_table.push(ExceptionTableEntry {
                            start_pc: reader.read_u2(),
                            end_pc: reader.read_u2(),
                            handler_pc: reader.read_u2(),
                            catch_type: reader.read_u2(),
                        });
                    }

                    // Nested attributes
                    let code_attr_count = reader.read_u2();
                    let mut code_attrs = Vec::with_capacity(code_attr_count as usize);
                    for _ in 0..code_attr_count {
                        let sub_name_index = reader.read_u2();
                        let sub_len = reader.read_u4();
                        let mut info = vec![0u8; sub_len as usize];
                        for i in 0..sub_len as usize {
                            info[i] = reader.read_u1();
                        }
                        code_attrs.push(AttributeInfo {
                            name_index: sub_name_index,
                            info,
                        });
                    }

                    code = Some(CodeAttribute {
                        max_stack,
                        max_locals,
                        code: code_bytes,
                        exception_table: ex_table,
                        attributes: code_attrs,
                    });
                } else {
                    let mut info = vec![0u8; attr_len as usize];
                    for i in 0..attr_len as usize {
                        info[i] = reader.read_u1();
                    }
                    attributes.push(AttributeInfo {
                        name_index: name_index_attr,
                        info,
                    });
                }
            }

            methods.push(MethodInfo {
                access_flags,
                name_index,
                descriptor_index,
                code,
                attributes,
            });
        }

        // Class attributes
        let attributes_count = reader.read_u2();
        let mut attributes = Vec::with_capacity(attributes_count as usize);
        for _ in 0..attributes_count {
            let name_index = reader.read_u2();
            let attr_len = reader.read_u4();
            let mut info = vec![0u8; attr_len as usize];
            for i in 0..attr_len as usize {
                info[i] = reader.read_u1();
            }
            attributes.push(AttributeInfo { name_index, info });
        }

        Ok(Self {
            magic,
            minor_version,
            major_version,
            constant_pool_count,
            constant_pool,
            access_flags,
            this_class,
            super_class,
            interfaces,
            fields,
            methods,
            attributes,
        })
    }
}
