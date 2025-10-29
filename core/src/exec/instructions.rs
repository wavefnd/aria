#[derive(Debug)]
pub enum Instruction {
    GetStatic(u16),
    Ldc(u8),
    InvokeVirtual(u16),
    Return,
    Unknown(u8),
}

impl Instruction {
    pub fn from_bytecode(code: &[u8], pc: &mut usize) -> Self {
        let opcode = code[*pc];
        *pc += 1;
        match opcode {
            0xb2 => { //getstatic
                let index = ((code[*pc] as u16) << 8) | (code[*pc + 1] as u16);
                *pc += 2;
                Instruction::GetStatic(index)
            }
            0x12 => { // ldc
                let index = code[*pc];
                *pc += 1;
                Instruction::Ldc(index)
            }
            0xb6 => { // invokevirtual
                let index = ((code[*pc] as u16) << 8) | (code[*pc + 1] as u16);
                *pc += 2;
                Instruction::InvokeVirtual(index)
            }
            0xb1 => Instruction::Return, // return
            _ => Instruction::Unknown(opcode),
        }
    }
}