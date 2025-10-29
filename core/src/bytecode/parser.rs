use super::reader::ClassReader;

pub const JAVA_MAGIC: u32 = 0xCAFEBABE;

#[derive(Debug)]
pub struct ClassFile {
    pub magic: u32,
    pub minor_version: u16,
    pub major_version: u16,
    pub constant_pool_count: u16,
    // Constant_pool, access_flags, methods, etc. will be added later.
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

        Ok(Self {
            magic,
            minor_version,
            major_version,
            constant_pool_count,
        })
    }
}