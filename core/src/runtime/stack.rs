use crate::runtime::frame::Frame;

#[derive(Debug, Clone)]
pub struct Stack {
    pub(crate) frames: Vec<Frame>,
}

impl Stack {
    pub fn new() -> Self {
        Self { frames: Vec::new() }
    }

    pub fn push_frame(&mut self, frame: Frame) {
        let depth = self.frames.len();
        println!("push_frame(depth={}): {:?}", depth + 1, frame.pc);
        self.frames.push(frame);
    }

    pub fn pop_frame(&mut self) -> Option<Frame> {
        if let Some(frame) = self.frames.pop() {
            println!("pop_frame -> depth now {}", self.frames.len());
            Some(frame)
        } else {
            println!("Tried to pop from empty frame stack");
            None
        }
    }

    pub fn current_frame_mut(&mut self) -> Option<&mut Frame> {
        self.frames.last_mut()
    }

    pub fn current_frame(&self) -> Option<&Frame> {
        self.frames.last()
    }

    pub fn peek_frame(&self) -> Option<&Frame> {
        self.frames.last()
    }

    pub fn is_empty(&self) -> bool {
        self.frames.is_empty()
    }

    pub fn depth(&self) -> usize {
        self.frames.len()
    }

    pub fn dump_stack(&self) {
        println!("====== JVM STACK ======");
        if self.frames.is_empty() {
            println!("(empty)");
        } else {
            for (i, frame) in self.frames.iter().enumerate() {
                println!(
                    "Frame #{} -> locals: {:?}, stack: {:?}, pc: {}",
                    i,
                    frame.local_vars,
                    frame.operand_stack,
                    frame.pc
                );
            }
        }
        println!("=======================");
    }
}
