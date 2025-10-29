#[derive(Debug)]
pub struct Frame {
    pub local_vars: Vec<i32>,
    pub operand_stack: Vec<i32>,
    pub pc: usize,
}

impl Frame {
    pub fn new(max_locals: usize, max_stack: usize) -> Self {
        Self {
            local_vars: vec![0; max_locals],
            operand_stack: Vec::with_capacity(max_stack),
            pc: 0,
        }
    }

    pub fn push(&mut self, val: i32) {
        self.operand_stack.push(val);
    }

    pub fn pop(&mut self) -> i32 {
        self.operand_stack.pop().unwrap_or(0)
    }
}