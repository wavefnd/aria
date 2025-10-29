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

        Ok(Self {
            magic,
            minor_version,
            major_version,
            constant_pool_count,
            constant_pool,
        })
    }
}