use std::fs::File;
use std::io::{self, Read, Seek, SeekFrom};

#[derive(Debug)]
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

    pub fn from_bytes(bytes: Vec<u8>) -> Self {
        Self { data: bytes, position: 0 }
    }

    pub fn read_u1(&mut self) -> u8 {
        if self.remaining() < 1 {
            panic!(
                "read_u1() out of bounds: pos={} len={}",
                self.position,
                self.data.len()
            );
        }
        let val = self.data[self.position];
        self.position += 1;
        val
    }

    pub fn read_u2(&mut self) -> u16 {
        if self.remaining() < 2 {
            panic!(
                "read_u2() out of bounds: pos={} len={}",
                self.position,
                self.data.len()
            );
        }
        let bytes = [self.read_u1(), self.read_u1()];
        u16::from_be_bytes(bytes)
    }

    pub fn read_u4(&mut self) -> u32 {
        if self.remaining() < 4 {
            panic!(
                "read_u4() out of bounds: pos={} len={}",
                self.position,
                self.data.len()
            );
        }
        let bytes = [self.read_u1(), self.read_u1(), self.read_u1(), self.read_u1()];
        u32::from_be_bytes(bytes)
    }

    pub fn skip(&mut self, n: usize) {
        if self.remaining() < n {
            panic!(
                "skip({}) out of bounds: pos={} len={}",
                n, self.position, self.data.len()
            );
        }
        self.position += n;
    }

    pub fn position(&self) -> usize {
        self.position
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn remaining(&self) -> usize {
        self.data.len().saturating_sub(self.position)
    }

    pub fn has_more(&self) -> bool {
        self.position < self.data.len()
    }

    pub fn dump_bytes(&self, count: usize) {
        let end = usize::min(self.position + count, self.data.len());
        let slice = &self.data[self.position..end];
        print!("[{}..{}] ", self.position, end);
        for b in slice {
            print!("{:02X} ", b);
        }
        println!();
    }

    pub fn seek(&mut self, pos: usize) {
        if pos > self.data.len() {
            panic!(
                "seek({}) out of bounds: len={}",
                pos, self.data.len()
            );
        }
        self.position = pos;
    }
}
