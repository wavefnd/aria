use crate::exec::instructions::Instruction;
use crate::bytecode::parser::ConstantPoolEntry;
use crate::runtime::frame::Frame;
use crate::bytecode::parser::ClassFile;

pub struct Interpreter;

impl Interpreter {
    pub fn execute(class: &ClassFile) {
        println!("🚀 Executing main() ...");

        let method = class
            .methods
            .iter()
            .find(|m| class.get_utf8(m.name_index).unwrap_or("") == "main");

        if let Some(method) = method {
            if let Some(code_attr) = &method.code {
                let code = &code_attr.code;
                let mut frame = Frame::new(code_attr.max_locals as usize, code_attr.max_stack as usize);
                let mut pc = 0;

                while pc < code.len() {
                    let instr = Instruction::from_bytecode(code, &mut pc);
                    match instr {
                        Instruction::GetStatic(index) => {
                            println!("GETSTATIC #{}", index);
                        }
                        Instruction::Ldc(index) => {
                            if let Some(ConstantPoolEntry::String { string_index }) =
                                class.constant_pool.get((index - 1) as usize)
                            {
                                if let Some(value) = class.get_utf8(*string_index) {
                                    println!("LDC \"{}\"", value);
                                    frame.push(0);
                                    println!("{}", value);
                                }
                            }
                        }
                        Instruction::InvokeVirtual(index) => {
                            println!("INVOKEVIRTUAL #{}", index);
                        }
                        Instruction::Return => {
                            println!("RETURN");
                            break;
                        }
                        Instruction::Unknown(op) => {
                            println!("⚠️ Unknown opcode: 0x{:X}", op);
                            break;
                        }
                    }
                }
            } else {
                println!("❌ main() has no Code attribute");
            }
        } else {
            println!("❌ main() not found");
        }

        println!("✅ Execution finished");
    }
}
