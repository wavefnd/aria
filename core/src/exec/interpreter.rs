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

    fn count_args(desc: &str) -> usize {
        let mut count = 0;
        let mut chars = desc.chars();
        while let Some(c) = chars.next() {
            if c == ')' { break; }
            if c == 'L' {
                while let Some(ec) = chars.next() { if ec == ';' { break; } }
                count += 1;
            } else if c == '[' {
                continue;
          } else {
              count += 1;
           }
        }
       count
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
                        if let ConstantPoolEntry::MethodRef { class_index, name_and_type_index } = entry {
                            let _cp_class_name = class.get_class_name(*class_index).unwrap();
                            let (method_name, descriptor) = class.get_name_and_type(*name_and_type_index).unwrap();

                            let arg_count = Self::count_args(descriptor); 
                            
                            let mut args = Vec::new();
                            for _ in 0..arg_count {
                                args.push(frame.pop());
                            }
                            args.reverse();
                            
                            let object_ref = frame.pop(); // 'this'

                            if let HeapValue::Object(obj) = &object_ref {
                                let real_class_name = &obj.class_name;
                                
                                if let Some(target_class) = Self::resolve_method(class_loader, real_class_name, method_name, descriptor) {
                                    println!("Invoking Virtual: {} on {}", method_name, real_class_name);
                                    
                                }
                            } else {
                                println!("NullPointerException!");
                                return None;
                            }
                        }
                    }
                }

                    Instruction::NewArray(atype_code) => {
                        let count = frame.pop_int();
                        if count < 0 {
                            println!("NegativeArraySizeException");
                        } else {
                            use crate::runtime::heap::ArrayType;
                            let atype = ArrayType::Int;
                            let arr = heap.alloc_array(count as usize, atype);
                            frame.push(HeapValue::Array(arr));
                        }
                    }

                    Instruction::IAStore => {
                        let val = frame.pop();
                        let idx = frame.pop_int();
                        let arr_ref = frame.pop();

                        if let HeapValue::Array(arr) = arr_ref {
                            if let Some(target_arr) = heap.get_array_mut(arr.id) {
                                if idx >= 0 && (idx as usize) < target_arr.content.len() {
                                    target_arr.content[idx as usize] = val;
                                } else {
                                    println!("ArrayIndexOutOfBoundsException");
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

    fn resolve_method<'a>(
        loader: &'a mut ClassLoader,
        class_name: &str,
        method_name: &str,
        descriptor: &str
    ) -> Option<ClassFile> {
        let current_class = loader.load_class(class_name).ok()?;

        for m in &current_class.methods {
            let m_name = current_class.get_utf8(m.name_index)?;
            let m_desc = current_class.get_utf8(m.descriptor_index)?;
            if m_name == method_name && m_desc == descriptor {
                return Some(current_class);
            }
        }

        if let Some(super_name) = current_class.get_class_name(current_class.super_class) {
            if super_name != "java/lang/Object" && !super_name.is_empty() {
                return Self::resolve_method(loader, class_name, method_name, descriptor);
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
