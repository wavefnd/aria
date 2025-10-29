use std::fs::File;
use std::io::*;
use std::io;

pub struct ClassReader {
    data: Vec<u8>,
    position: usize,
}

impl ClassReader {
    pub fn from_file(path: &str) -> io::Result<Self> {
        let mut file = File::open(path)?;
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer)?;
        Ok(Self { data: buffer, position: 0 })
    }

    pub fn read_u1(&mut self) -> u8 {
        let val = self.data[self.position];
        self.position += 1;
        val
    }

    pub fn read_u2(&mut self) -> u16 {
        let bytes = [self.read_u1(), self.read_u1()];
        u16::from_be_bytes(bytes)
    }

    pub fn read_u4(&mut self) -> u32 {
        let bytes = [self.read_u1(), self.read_u1(), self.read_u1(), self.read_u1()];
        u32::from_be_bytes(bytes)
    }

    pub fn skip(&mut self, n: usize) {
        self.position += n;
    }

    pub fn has_more(&self) -> bool {
        self.position < self.data.len()
    }
}