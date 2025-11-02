use crate::exec::instructions::Instruction;
use crate::bytecode::parser::{ClassFile, ConstantPoolEntry};
use crate::loader::class_loader::ClassLoader;
use crate::native::invoke_native;
use crate::runtime::frame::Frame;
use crate::runtime::heap::{Heap, HeapValue};
use crate::runtime::stack::Stack;
use crate::runtime::gc::Gc;

pub struct Interpreter {
    debug_mode: bool,
}

impl Interpreter {
    pub fn new(debug_mode: bool) -> Self {
        Self { debug_mode }
    }

    pub fn execute(&self, class: &ClassFile) {
        println!("Executing main() ...");

        let main_method = class
            .methods
            .iter()
            .find(|m| class.get_utf8(m.name_index).unwrap_or("") == "main");

        if let Some(method) = main_method {
            if let Some(code_attr) = &method.code {
                let code = &code_attr.code;
                let mut frame = Frame::new(code_attr.max_locals as usize, code_attr.max_stack as usize);
                let mut heap = Heap::new();
                let mut pc = 0;
                let gc = Gc::new(self.debug_mode);

                while pc < code.len() {
                    let instr = Instruction::from_bytecode(code, &mut pc);
                    Self::exec_instr(&mut frame, &mut heap, class, instr);

                    if heap.object_count() > 128 {
                        if self.debug_mode {
                            println!("GC Triggered (heap size = {})", heap.object_count());
                        }
                        gc.collect(&mut heap, &Stack { frames: vec![frame.clone()] });
                    }
                }
            } else {
                println!("main() has no Code attribute");
            }
        } else {
            println!("main() not found");
        }

        println!("Execution finished");
    }

    pub fn execute_method(
        &self,
        class_loader: &mut ClassLoader,
        class: &ClassFile,
        name: &str,
        desc: &str,
        heap: &mut Heap,
    ) -> Option<HeapValue> {
        let mut stack = Stack::new();

        let method = class
            .methods
            .iter()
            .find(|m| class.get_utf8(m.name_index).unwrap_or("") == name)?;

        if let Some(code_attr) = &method.code {
            let code = &code_attr.code;
            stack.push_frame(Frame::new(code_attr.max_locals as usize, code_attr.max_stack as usize));

            let mut pc = 0;

            while pc < code.len() {
                let frame = stack.current_frame_mut().unwrap();
                let instr = Instruction::from_bytecode(code, &mut pc);

                match instr {
                    Instruction::InvokeStatic(index)
                    | Instruction::InvokeVirtual(index)
                    | Instruction::InvokeSpecial(index) => {
                        if let Some(entry) = Self::safe_cp_get(class, index) {
                            if let ConstantPoolEntry::MethodRef {
                                class_index,
                                name_and_type_index,
                            } = entry
                            {
                                let target_class = class.get_class_name(*class_index).unwrap_or("<unknown>");
                                if let Some(ConstantPoolEntry::NameAndType {
                                                name_index,
                                                descriptor_index,
                                            }) = class.constant_pool.get((*name_and_type_index - 1) as usize)
                                {
                                    let method_name = class.get_utf8(*name_index).unwrap_or("<unknown>");
                                    let descriptor = class.get_utf8(*descriptor_index).unwrap_or("<desc>");
                                    println!("Invoking {}.{}{}", target_class, method_name, descriptor);

                                    if invoke_native(target_class, method_name, descriptor, frame) {
                                        continue;
                                    }

                                    if let Ok(target) = class_loader.load_class(target_class) {
                                        self.execute_method(class_loader, &target, method_name, descriptor, heap);
                                    } else {
                                        println!("Could not load target class {}", target_class);
                                    }
                                }
                            }
                        }
                    }

                    Instruction::Return => {
                        let ret_val = frame.peek().cloned();
                        stack.pop_frame();
                        if stack.is_empty() {
                            println!("RETURN -> {:?}", ret_val);
                            return ret_val;
                        }
                    }

                    _ => {
                        let current = stack.current_frame_mut().unwrap();
                        Self::exec_instr(current, heap, class, instr);
                    }
                }
            }
        }
        None
    }

    fn exec_instr(frame: &mut Frame, heap: &mut Heap, class: &ClassFile, instr: Instruction) {
        match instr {
            Instruction::New(index) => {
                if let Some(class_name) = class.get_class_name(index) {
                    let obj = heap.alloc_object(class_name);
                    frame.push(HeapValue::Object(obj.clone()));
                    println!("NEW [{}]", class_name);
                }
            }

            Instruction::Dup => {
                if let Some(top) = frame.peek().cloned() {
                    frame.push(top);
                    println!("DUP");
                }
            }

            Instruction::AStore(index) => {
                let val = frame.pop();
                frame.set_local(index as usize, val.clone());
                println!("ASTORE[{}] = {:?}", index, val);
            }
            Instruction::ALoad(index) => {
                if let Some(val) = frame.get_local(index as usize).cloned() {
                    frame.push(val.clone());
                    println!("ALOAD[{}] -> {:?}", index, val);
                }
            }
            Instruction::IStore(index) => {
                let val = frame.pop();
                frame.set_local(index as usize, val.clone());
                println!("ISTORE[{}] = {:?}", index, val);
            }
            Instruction::ILoad(index) => {
                if let Some(val) = frame.get_local(index as usize).cloned() {
                    frame.push(val.clone());
                    println!("ILOAD[{}] -> {:?}", index, val);
                }
            }

            Instruction::IConst(v) => {
                frame.push(HeapValue::Int(v));
                println!("ICONST {}", v);
            }
            Instruction::BiPush(v) => {
                frame.push(HeapValue::Int(v as i32));
                println!("BIPUSH {}", v);
            }
            Instruction::SiPush(v) => {
                frame.push(HeapValue::Int(v as i32));
                println!("SIPUSH {}", v);
            }
            Instruction::IAdd => {
                let b = frame.pop();
                let a = frame.pop();
                let res = HeapValue::Int(a.as_int() + b.as_int());
                frame.push(res.clone());
                println!("➕ IADD = {:?}", res);
            }
            Instruction::ISub => {
                let b = frame.pop();
                let a = frame.pop();
                let res = HeapValue::Int(a.as_int() - b.as_int());
                frame.push(res.clone());
                println!("ISUB = {:?}", res);
            }
            Instruction::IMul => {
                let b = frame.pop();
                let a = frame.pop();
                let res = HeapValue::Int(a.as_int() * b.as_int());
                frame.push(res.clone());
                println!("IMUL = {:?}", res);
            }
            Instruction::IDiv => {
                let b = frame.pop();
                let a = frame.pop();
                if b.as_int() == 0 {
                    println!("Division by zero");
                } else {
                    let res = HeapValue::Int(a.as_int() / b.as_int());
                    frame.push(res.clone());
                    println!("IDIV = {:?}", res);
                }
            }

            Instruction::Ldc(index) => {
                if let Some(entry) = Self::safe_cp_get(class, u16::from(index)) {
                    match entry {
                        ConstantPoolEntry::String { string_index } => {
                            if let Some(s) = class.get_utf8(*string_index) {
                                let obj = heap.alloc_string(s);
                                frame.push(obj);
                                println!("LDC (String) \"{}\"", s);
                            }
                        }
                        ConstantPoolEntry::Utf8(value) => {
                            frame.push(HeapValue::String(value.clone()));
                            println!("LDC (Utf8) \"{}\"", value);
                        }
                        _ => println!("Unsupported LDC entry {:?}", entry),
                    }
                }
            }

            Instruction::GetField(index) => {
                let obj_ref = frame.pop();
                if let HeapValue::Object(obj) = obj_ref {
                    if let Some(ConstantPoolEntry::FieldRef { name_and_type_index, .. }) =
                        Self::safe_cp_get(class, index)
                    {
                        if let Some(ConstantPoolEntry::NameAndType { name_index, .. }) =
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

            Instruction::PutField(index) => {
                let value = frame.pop();
                let obj_ref = frame.pop();
                if let HeapValue::Object(mut obj) = obj_ref {
                    if let Some(ConstantPoolEntry::FieldRef { name_and_type_index, .. }) =
                        Self::safe_cp_get(class, index)
                    {
                        if let Some(ConstantPoolEntry::NameAndType { name_index, .. }) =
                            class.constant_pool.get((*name_and_type_index - 1) as usize)
                        {
                            let field_name = class.get_utf8(*name_index).unwrap_or("<unknown>");
                            obj.fields.insert(field_name.to_string(), value.clone());
                            println!("PUTFIELD {} = {:?}", field_name, value);
                        }
                    }
                }
            }

            Instruction::Return => {
                println!("RETURN");
            }

            Instruction::Unknown(op) => {
                println!("Unknown opcode: 0x{:02X}", op);
            }

            _ => {
                println!("Unimplemented instruction: {:?}", instr);
            }
        }
    }

    fn safe_cp_get<'a>(class: &'a ClassFile, index: u16) -> Option<&'a ConstantPoolEntry> {
        if index == 0 {
            return None;
        }
        class.constant_pool.get((index - 1) as usize)
    }
}
