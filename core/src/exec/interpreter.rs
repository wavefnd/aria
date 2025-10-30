use crate::exec::instructions::Instruction;
use crate::bytecode::parser::ConstantPoolEntry;
use crate::loader::class_loader::ClassLoader;
use crate::native::invoke_native;
use crate::runtime::frame::Frame;
use crate::bytecode::parser::ClassFile;
use crate::runtime::heap::{Heap, HeapValue};

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
                let mut heap = Heap::new();

                while pc < code.len() {
                    let instr = Instruction::from_bytecode(code, &mut pc);
                    match instr {
                        Instruction::New(index) => {
                            if let Some(class_name) = class.get_class_name(index) {
                                let obj = heap.alloc_object(class_name);
                                println!("NEW {} -> [Object {}]", class_name, obj.class_name);
                                frame.push(HeapValue::Object(obj));
                            }
                        }
                        Instruction::Dup => {
                            if let Some(top) = frame.operand_stack.last().cloned() {
                                frame.push(top);
                                println!("DUP -> top value duplicated");
                            }
                        }
                        Instruction::AStore(index) => {
                            let val = frame.pop();
                            frame.local_vars[index as usize] = val.clone();
                            println!("ASTORE {} <- {:?}", index, val);
                        }
                        Instruction::ALoad(index) => {
                            let val = frame.local_vars[index as usize].clone();
                            frame.push(val.clone());
                            println!("ALOAD {} -> {:?}", index, val);
                        }
                        Instruction::PutField(index) => {
                            let value = frame.pop();
                            let obj_ref = frame.pop();
                            if let HeapValue::Object(mut obj) = obj_ref {
                                if let Some(ConstantPoolEntry::FieldRef { class_index: _, name_and_type_index }) =
                                    class.constant_pool.get((index - 1) as usize)
                                {
                                    if let Some(ConstantPoolEntry::NameAndType { name_index, descriptor_index: _ }) =
                                        class.constant_pool.get((*name_and_type_index - 1) as usize)
                                    {
                                        let field_name = class.get_utf8(*name_index).unwrap_or("<unknown>");
                                        obj.fields.insert(field_name.to_string(), value.clone());
                                        println!("PUTFIELD {} = {:?}", field_name, value);
                                    }
                                }
                            }
                        }
                        Instruction::GetField(index) => {
                            let obj_ref = frame.pop();
                            if let HeapValue::Object(obj) = obj_ref {
                                if let Some(ConstantPoolEntry::FieldRef { class_index: _, name_and_type_index }) =
                                    class.constant_pool.get((index - 1) as usize)
                                {
                                    if let Some(ConstantPoolEntry::NameAndType { name_index, descriptor_index: _ }) =
                                        class.constant_pool.get((*name_and_type_index - 1) as usize)
                                    {
                                        let field_name = class.get_utf8(*name_index).unwrap_or("<unknown>");
                                        if let Some(val) = obj.fields.get(field_name) {
                                            frame.push(val.clone());
                                            println!("GETFIELD {} -> {:?}", field_name, val);
                                        }
                                    }
                                }
                            }
                        }
                        Instruction::InvokeSpecial(index) => {
                            println!("INVOKESPECIAL #{}", index);
                        }
                        Instruction::IConst(val) => {
                            frame.push(HeapValue::Int(val));
                            println!("ICONST {}", val);
                        }
                        Instruction::BiPush(val) => {
                            frame.push(HeapValue::Int(val as i32));
                            println!("BIPUSH {}", val);
                        }
                        Instruction::SiPush(val) => {
                            frame.push(HeapValue::Int(val as i32));
                            println!("SIPUSH {}", val);
                        }
                        Instruction::ILoad(index) => {
                            let val = frame.local_vars[index as usize].clone();
                            frame.push(val.clone());
                            println!("ILOAD {} -> {}", index, val);
                        }
                        Instruction::IStore(index) => {
                            let val = frame.pop();
                            frame.local_vars[index as usize] = val.clone();
                            println!("ISTORE {} <- {}", index, val);
                        }
                        Instruction::IAdd => {
                            let v2 = frame.pop();
                            let v1 = frame.pop();
                            let result = HeapValue::Int(v1.as_int() + v2.as_int());
                            frame.push(result.clone());
                            println!("IADD {} + {} = {}", v1, v2, result);
                        }
                        Instruction::ISub => {
                            let v2 = frame.pop();
                            let v1 = frame.pop();
                            let result = HeapValue::Int(v1.as_int() - v2.as_int());
                            frame.push(result.clone());
                            println!("ISUB {} - {} = {}", v1, v2, result);
                        }
                        Instruction::IMul => {
                            let v2 = frame.pop();
                            let v1 = frame.pop();
                            let result = HeapValue::Int(v1.as_int() * v2.as_int());
                            frame.push(result.clone());
                            println!("IMUL {} * {} = {}", v1, v2, result);
                        }
                        Instruction::IDiv => {
                            let v2 = frame.pop();
                            let v1 = frame.pop();
                            if v2.as_int() == 0 {
                                println!("Division by zero");
                            } else {
                                let result = HeapValue::Int(v1.as_int() / v2.as_int());
                                frame.push(result.clone());
                                println!("IDIV {} / {} = {}", v1, v2, result);
                            }
                        }
                        Instruction::InvokeVirtual(index) | Instruction::InvokeStatic(index) => {
                            if let Some(ConstantPoolEntry::MethodRef { class_index, name_and_type_index }) = class.constant_pool.get((index - 1) as usize) {
                                let class_name = class.get_class_name(*class_index).unwrap_or("<unknown>");
                                if let Some(ConstantPoolEntry::NameAndType { name_index, descriptor_index }) = class.constant_pool.get((*name_and_type_index - 1) as usize) {
                                    let name = class.get_utf8(*name_index).unwrap_or("<unknown>");
                                    let desc = class.get_utf8(*descriptor_index).unwrap_or("desc");
                                    if invoke_native(class_name, name, desc, &mut frame) {
                                        continue;
                                    } else {
                                        println!("Unimplemented native call: {}.{}{}", class_name, name, desc);
                                    }
                                }
                            }
                        }
                        Instruction::GetStatic(index) => {
                            println!("GETSTATIC #{}", index);
                        }
                        Instruction::Ldc(index) => {
                            if let Some(ConstantPoolEntry::String { string_index }) = class.constant_pool.get((index - 1) as usize) {
                                if let Some(value) = class.get_utf8(*string_index) {
                                    let obj = heap.alloc_string(value);
                                    frame.push(obj);
                                    println!("LDC (String) \"{}\"", value);
                                }
                            } else if let Some(ConstantPoolEntry::Utf8(value)) = class.constant_pool.get((index - 1) as usize) {
                                frame.push(HeapValue::String(value.clone()));
                                println!("LDC (Utf8) \"{}\"", value);
                            }
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

    pub fn execute_method(
        class_loader: &mut ClassLoader,
        class: &ClassFile,
        method_name: &str,
        descriptor: &str,
    ) -> Option<HeapValue> {
        if let Some(method) = class
            .methods
            .iter()
            .find(|m| class.get_utf8(m.name_index).unwrap_or("") == method_name)
        {
            if let Some(code_attr) = &method.code {
                let mut frame = Frame::new(code_attr.max_locals as usize, code_attr.max_stack as usize);
                let mut pc = 0;
                let code = &code_attr.code;

                while pc < code.len() {
                    let instr = Instruction::from_bytecode(code, &mut pc);
                    match instr {
                        Instruction::InvokeStatic(index)
                        | Instruction::InvokeVirtual(index)
                        | Instruction::InvokeSpecial(index) => {
                            if let Some(ConstantPoolEntry::MethodRef {
                                class_index,
                                name_and_type_index,
                            }) = class.constant_pool.get((index - 1) as usize)
                            {
                                let target_class_name = class.get_class_name(*class_index).unwrap_or("<unknown>");
                                if let Some(ConstantPoolEntry::NameAndType {
                                    name_index,
                                    descriptor_index,
                                }) = class.constant_pool.get((*name_and_type_index - 1) as usize)
                                {
                                    let name = class.get_utf8(*name_index).unwrap_or("<unknown>");
                                    let desc = class.get_utf8(*descriptor_index).unwrap_or("<desc>");

                                    if invoke_native(target_class_name, name, desc, &mut frame) {
                                        continue;
                                    }

                                    if let Ok(target_class) = class_loader.load_class(target_class_name) {
                                        println!(
                                            "➡️ Calling {}.{}{}",
                                            target_class_name, name, desc
                                        );
                                        Self::execute_method(class_loader, &target_class, name, desc);
                                    }
                                }
                            }
                        }
                        Instruction::Return => {
                            println!("↩️ Return from {}", method_name);
                            return None;
                        }
                        _ => {
                            // Existing operation processing (iload, istore, etc.)
                        }
                    }
                }
            }
        }
        None
    }
}
