use crate::bytecode::parser::*;

impl ClassFile {
    pub fn get_utf8(&self, index: u16) -> Option<&str> {
        match self.constant_pool.get((index - 1) as usize)? {
            ConstantPoolEntry::Utf8(s) => Some(s),
            _ => None,
        }
    }

    pub fn get_class_name(&self, index: u16) -> Option<&str> {
        match self.constant_pool.get((index - 1) as usize)? {
            ConstantPoolEntry::Class { name_index } => self.get_utf8(*name_index),
            _ => None,
        }
    }
}