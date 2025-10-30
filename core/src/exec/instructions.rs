#[derive(Debug)]
pub enum Instruction {
    GetStatic(u16),
    Ldc(u8),
    InvokeVirtual(u16),
    Return,
    IConst(i32),
    ILoad(u8),
    IStore(u8),
    IAdd,
    ISub,
    IMul,
    IDiv,
    BiPush(i8),
    SiPush(i16),
    InvokeStatic(u16),
    New(u16),
    GetField(u16),
    PutField(u16),
    InvokeSpecial(u16),
    Dup,
    AStore(u8),
    ALoad(u8),
    Unknown(u8),
}

impl Instruction {
    pub fn from_bytecode(code: &[u8], pc: &mut usize) -> Self {
        let opcode = code[*pc];
        *pc += 1;
        match opcode {
            0x1A => Instruction::ILoad(0),
            0x1B => Instruction::ILoad(1),
            0x1C => Instruction::ILoad(2),
            0x1D => Instruction::ILoad(3),
            0x3B => Instruction::IStore(0),
            0x3C => Instruction::IStore(1),
            0x3D => Instruction::IStore(2),
            0x3E => Instruction::IStore(3),
            0x59 => Instruction::Dup,
            0x4B => Instruction::AStore(0),
            0x4C => Instruction::AStore(1),
            0x4D => Instruction::AStore(2),
            0x4E => Instruction::AStore(3),
            0x19 => {
                let index = code[*pc];
                *pc += 1;
                Instruction::ALoad(index)
            }
            0x2A => Instruction::ALoad(0),
            0x2B => Instruction::ALoad(1),
            0x2C => Instruction::ALoad(2),
            0x2D => Instruction::ALoad(3),
            0xbb => { // new
                let index = ((code[*pc] as u16) << 8) | (code[*pc + 1] as u16);
                *pc += 2;
                Instruction::New(index)
            }
            0xb4 => { // getfield
                let index = ((code[*pc] as u16) << 8) | (code[*pc + 1] as u16);
                *pc += 2;
                Instruction::GetField(index)
            }
            0xb5 => { // putfield
                let index = ((code[*pc] as u16) << 8) | (code[*pc + 1] as u16);
                *pc += 2;
                Instruction::PutField(index)
            }
            0xb7 => { // invokespecial <init>
                let index = ((code[*pc] as u16) << 8) | (code[*pc + 1] as u16);
                *pc += 2;
                Instruction::InvokeSpecial(index)
            }
            0x03..=0x08 => Instruction::IConst((opcode - 0x03) as i32), // iconst_0!iconst_5
            0x10 => { // bipush
                let val = code[*pc] as i8;
                *pc += 1;
                Instruction::BiPush(val)
            }
            0x11 => { // sipush
                let high = code[*pc] as i16;
                let low = code[*pc + 1] as i16;
                *pc += 2;
                Instruction::SiPush((high << 8) | low)
            }
            0x15 => { // iload
                let index = code[*pc];
                *pc += 1;
                Instruction::ILoad(index)
            }
            0x36 => { // istore
                let index = code[*pc];
                *pc += 1;
                Instruction::IStore(index)
            }
            0x60 => Instruction::IAdd,
            0x64 => Instruction::ISub,
            0x68 => Instruction::IMul,
            0x6C => Instruction::IDiv,
            0xb8 => { // invokestatic
                let index = ((code[*pc] as u16) << 8) | (code[*pc + 1] as u16);
                *pc += 2;
                Instruction::InvokeStatic(index)
            }
            0xb2 => { // getstatic
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