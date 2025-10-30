use crate::runtime::heap::HeapValue;

#[derive(Debug)]
pub struct Frame {
    pub local_vars: Vec<HeapValue>,
    pub operand_stack: Vec<HeapValue>,
    pub pc: usize,
}

impl Frame {
    pub fn new(max_locals: usize, max_stack: usize) -> Self {
        Self {
            local_vars: vec![HeapValue::Null; max_locals],
            operand_stack: Vec::with_capacity(max_stack),
            pc: 0,
        }
    }

    pub fn push(&mut self, val: HeapValue) {
        self.operand_stack.push(val);
    }

    pub fn pop(&mut self) -> HeapValue {
        self.operand_stack.pop().unwrap_or(HeapValue::Null)
    }

    pub fn push_int(&mut self, val: i32) {
        self.push(HeapValue::Int(val));
    }

    pub fn pop_int(&mut self) -> i32 {
        self.pop().as_int()
    }

    pub fn push_long(&mut self, val: i64) {
        self.push(HeapValue::Long(val));
    }

    pub fn pop_long(&mut self) -> i64 {
        self.pop().as_long()
    }

    pub fn peek(&self) -> Option<&HeapValue> {
        self.operand_stack.last()
    }
}