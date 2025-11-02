#[derive(Debug, Clone)]
pub enum Instruction {
    // Constants & Loads
    IConst(i32),
    BiPush(i8),
    SiPush(i16),
    Ldc(u8),

    ILoad(u8),
    IStore(u8),
    ALoad(u8),
    AStore(u8),

    // Stack ops
    Dup,
    Dup2,
    DupX1,
    DupX2,
    Pop,

    // Arithmetic
    IAdd,
    ISub,
    IMul,
    IDiv,

    // Control flow
    Goto(i16),
    IfEq(i16),
    IfNe(i16),
    IfLt(i16),
    IfGe(i16),
    IfGt(i16),
    IfLe(i16),
    IInc(u8, i8),

    // Field & Method
    GetStatic(u16),
    GetField(u16),
    PutField(u16),
    InvokeVirtual(u16),
    InvokeStatic(u16),
    InvokeSpecial(u16),

    // Object & Return
    New(u16),
    Return,

    // Fallback
    Unknown(u8),
}

impl Instruction {
    pub fn from_bytecode(code: &[u8], pc: &mut usize) -> Self {
        if *pc >= code.len() {
            return Instruction::Unknown(0xFF);
        }

        let opcode = code[*pc];
        *pc += 1;

        macro_rules! read_u8 {
            () => {{
                if *pc >= code.len() {
                    return Instruction::Unknown(opcode);
                }
                let val = code[*pc];
                *pc += 1;
                val
            }};
        }

        macro_rules! read_u16 {
            () => {{
                if *pc + 1 >= code.len() {
                    return Instruction::Unknown(opcode);
                }
                let val = ((code[*pc] as u16) << 8) | (code[*pc + 1] as u16);
                *pc += 2;
                val
            }};
        }

        macro_rules! read_i16 {
            () => {{
                let val = read_u16!() as i16;
                val
            }};
        }

        match opcode {
            // --- Constants ---
            0x02 => Instruction::IConst(-1),
            0x03..=0x08 => Instruction::IConst((opcode - 0x03) as i32), // iconst_0..iconst_5
            0x10 => Instruction::BiPush(read_u8!() as i8),
            0x11 => {
                let high = read_u8!() as i16;
                let low = read_u8!() as i16;
                Instruction::SiPush((high << 8) | (low & 0xFF))
            }
            0x12 => Instruction::Ldc(read_u8!()),

            // --- Load / Store ---
            0x15 => Instruction::ILoad(read_u8!()),
            0x36 => Instruction::IStore(read_u8!()),
            0x19 => Instruction::ALoad(read_u8!()),
            0x3A => Instruction::AStore(read_u8!()),

            0x1A => Instruction::ILoad(0),
            0x1B => Instruction::ILoad(1),
            0x1C => Instruction::ILoad(2),
            0x1D => Instruction::ILoad(3),

            0x3B => Instruction::IStore(0),
            0x3C => Instruction::IStore(1),
            0x3D => Instruction::IStore(2),
            0x3E => Instruction::IStore(3),

            0x2A => Instruction::ALoad(0),
            0x2B => Instruction::ALoad(1),
            0x2C => Instruction::ALoad(2),
            0x2D => Instruction::ALoad(3),

            0x4B => Instruction::AStore(0),
            0x4C => Instruction::AStore(1),
            0x4D => Instruction::AStore(2),
            0x4E => Instruction::AStore(3),

            // --- Stack operations ---
            0x59 => Instruction::Dup,
            0x5A => Instruction::DupX1,
            0x5B => Instruction::DupX2,
            0x5C => Instruction::Dup2,
            0x57 => Instruction::Pop,

            // --- Arithmetic ---
            0x60 => Instruction::IAdd,
            0x64 => Instruction::ISub,
            0x68 => Instruction::IMul,
            0x6C => Instruction::IDiv,
            0x84 => {
                let index = read_u8!();
                let val = read_u8!() as i8;
                Instruction::IInc(index, val)
            }

            // --- Control flow ---
            0xA7 => Instruction::Goto(read_i16!()),
            0x99 => Instruction::IfEq(read_i16!()),
            0x9A => Instruction::IfNe(read_i16!()),
            0x9B => Instruction::IfLt(read_i16!()),
            0x9C => Instruction::IfGe(read_i16!()),
            0x9D => Instruction::IfGt(read_i16!()),
            0x9E => Instruction::IfLe(read_i16!()),

            // --- Object / Field / Method ---
            0xBB => Instruction::New(read_u16!()),
            0xB2 => Instruction::GetStatic(read_u16!()),
            0xB4 => Instruction::GetField(read_u16!()),
            0xB5 => Instruction::PutField(read_u16!()),
            0xB6 => Instruction::InvokeVirtual(read_u16!()),
            0xB7 => Instruction::InvokeSpecial(read_u16!()),
            0xB8 => Instruction::InvokeStatic(read_u16!()),

            // --- Return ---
            0xB1 => Instruction::Return,

            // --- Fallback ---
            _ => Instruction::Unknown(opcode),
        }
    }
}
