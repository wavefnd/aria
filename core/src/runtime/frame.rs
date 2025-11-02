use crate::runtime::heap::HeapValue;

#[derive(Debug, Clone)]
pub struct Frame {
    pub local_vars: Vec<HeapValue>,
    pub operand_stack: Vec<HeapValue>,
    pub pc: usize,
    pub max_stack: usize,
}

impl Frame {
    pub fn new(max_locals: usize, max_stack: usize) -> Self {
        Self {
            local_vars: vec![HeapValue::Null; max_locals],
            operand_stack: Vec::with_capacity(max_stack),
            pc: 0,
            max_stack,
        }
    }

    // ===== Local Variables =====

    pub fn get_local(&self, index: usize) -> Option<&HeapValue> {
        self.local_vars.get(index)
    }

    pub fn set_local(&mut self, index: usize, value: HeapValue) {
        if index >= self.local_vars.len() {
            self.local_vars.resize(index + 1, HeapValue::Null);
        }
        self.local_vars[index] = value;
    }

    // ===== Operand Stack =====

    pub fn push(&mut self, value: HeapValue) {
        if self.operand_stack.len() >= self.max_stack {
            println!("Stack overflow (max_stack={})", self.max_stack);
        }
        self.operand_stack.push(value);
    }

    pub fn pop(&mut self) -> HeapValue {
        self.operand_stack.pop().unwrap_or(HeapValue::Null)
    }

    pub fn peek(&self) -> Option<&HeapValue> {
        self.operand_stack.last()
    }

    pub fn peek_mut(&mut self) -> Option<&mut HeapValue> {
        self.operand_stack.last_mut()
    }

    pub fn stack_size(&self) -> usize {
        self.operand_stack.len()
    }

    // ===== Primitive helpers =====

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

    // ===== Utility =====
    pub fn dump_state(&self) {
        println!("----- FRAME STATE -----");
        println!("PC: {}", self.pc);
        println!("Locals: {:?}", self.local_vars);
        println!("Stack: {:?}", self.operand_stack);
        println!("-----------------------");
    }
}
