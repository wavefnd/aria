use crate::bytecode::parser::{AttributeInfo, ClassFile, ConstantPoolEntry, MethodInfo};
use crate::exec::instructions::Instruction;
use crate::loader::class_loader::ClassLoader;
use crate::native::invoke_native;
use crate::runtime::frame::Frame;
use crate::runtime::gc::Gc;
use crate::runtime::heap::{Heap, HeapValue};
use crate::runtime::stack::Stack;

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
                let mut frame =
                    Frame::new(code_attr.max_locals as usize, code_attr.max_stack as usize);
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
                        gc.collect(
                            &mut heap,
                            &Stack {
                                frames: vec![frame.clone()],
                            },
                        );
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
        let mut chars = desc.chars().peekable();
        while let Some(c) = chars.next() {
            if c == '(' {
                break;
            }
        }

        let mut count = 0usize;
        while let Some(c) = chars.next() {
            if c == ')' {
                break;
            }
            if c == '[' {
                while matches!(chars.peek(), Some('[')) {
                    let _ = chars.next();
                }
                if matches!(chars.peek(), Some('L')) {
                    let _ = chars.next();
                    while let Some(ec) = chars.next() {
                        if ec == ';' {
                            break;
                        }
                    }
                } else {
                    let _ = chars.next();
                }
                count += 1;
                continue;
            }
            if c == 'L' {
                while let Some(ec) = chars.next() {
                    if ec == ';' {
                        break;
                    }
                }
                count += 1;
                continue;
            }
            count += 1;
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
        initial_locals: &[HeapValue],
    ) -> Option<HeapValue> {
        let mut stack = Stack::new();
        let gc = Gc::new(self.debug_mode);

        let method = class
            .methods
            .iter()
            .find(|m| {
                class.get_utf8(m.name_index).unwrap_or("") == name
                    && class.get_utf8(m.descriptor_index).unwrap_or("") == desc
            })
            .cloned()?;

        let code_attr = method.code.as_ref()?;
        let code = &code_attr.code;
        let mut entry_frame =
            Frame::new(code_attr.max_locals as usize, code_attr.max_stack as usize);
        for (idx, value) in initial_locals.iter().enumerate() {
            entry_frame.set_local(idx, value.clone());
        }
        stack.push_frame(entry_frame);

        let mut pc = 0usize;
        while pc < code.len() {
            let opcode_pc = pc;
            let instr = Instruction::from_bytecode(code, &mut pc);
            let frame = stack.current_frame_mut().unwrap();

            match instr {
                Instruction::Goto(offset) => {
                    pc = Self::branch_target(opcode_pc, offset, code.len())?;
                }
                Instruction::IfEq(offset) => {
                    if frame.pop_int() == 0 {
                        pc = Self::branch_target(opcode_pc, offset, code.len())?;
                    }
                }
                Instruction::IfNe(offset) => {
                    if frame.pop_int() != 0 {
                        pc = Self::branch_target(opcode_pc, offset, code.len())?;
                    }
                }
                Instruction::IfLt(offset) => {
                    if frame.pop_int() < 0 {
                        pc = Self::branch_target(opcode_pc, offset, code.len())?;
                    }
                }
                Instruction::IfGe(offset) => {
                    if frame.pop_int() >= 0 {
                        pc = Self::branch_target(opcode_pc, offset, code.len())?;
                    }
                }
                Instruction::IfGt(offset) => {
                    if frame.pop_int() > 0 {
                        pc = Self::branch_target(opcode_pc, offset, code.len())?;
                    }
                }
                Instruction::IfLe(offset) => {
                    if frame.pop_int() <= 0 {
                        pc = Self::branch_target(opcode_pc, offset, code.len())?;
                    }
                }
                Instruction::IfICmpEq(offset) => {
                    let rhs = frame.pop_int();
                    let lhs = frame.pop_int();
                    if lhs == rhs {
                        pc = Self::branch_target(opcode_pc, offset, code.len())?;
                    }
                }
                Instruction::IfICmpNe(offset) => {
                    let rhs = frame.pop_int();
                    let lhs = frame.pop_int();
                    if lhs != rhs {
                        pc = Self::branch_target(opcode_pc, offset, code.len())?;
                    }
                }
                Instruction::IfICmpLt(offset) => {
                    let rhs = frame.pop_int();
                    let lhs = frame.pop_int();
                    if lhs < rhs {
                        pc = Self::branch_target(opcode_pc, offset, code.len())?;
                    }
                }
                Instruction::IfICmpGe(offset) => {
                    let rhs = frame.pop_int();
                    let lhs = frame.pop_int();
                    if lhs >= rhs {
                        pc = Self::branch_target(opcode_pc, offset, code.len())?;
                    }
                }
                Instruction::IfICmpGt(offset) => {
                    let rhs = frame.pop_int();
                    let lhs = frame.pop_int();
                    if lhs > rhs {
                        pc = Self::branch_target(opcode_pc, offset, code.len())?;
                    }
                }
                Instruction::IfICmpLe(offset) => {
                    let rhs = frame.pop_int();
                    let lhs = frame.pop_int();
                    if lhs <= rhs {
                        pc = Self::branch_target(opcode_pc, offset, code.len())?;
                    }
                }
                Instruction::IfNull(offset) => {
                    if frame.pop().is_null() {
                        pc = Self::branch_target(opcode_pc, offset, code.len())?;
                    }
                }
                Instruction::IfNonNull(offset) => {
                    if !frame.pop().is_null() {
                        pc = Self::branch_target(opcode_pc, offset, code.len())?;
                    }
                }
                Instruction::IInc(index, delta) => {
                    let current = frame
                        .get_local(index as usize)
                        .cloned()
                        .unwrap_or(HeapValue::Int(0))
                        .as_int();
                    frame.set_local(index as usize, HeapValue::Int(current + delta as i32));
                }

                Instruction::InvokeDynamic(index) => {
                    let Some((_indy_name, descriptor)) = Self::resolve_invoke_dynamic(class, index)
                    else {
                        println!("Invalid invokedynamic ref #{}", index);
                        return None;
                    };
                    let arg_count = Self::count_args(descriptor);
                    let mut args = Vec::with_capacity(arg_count);
                    for _ in 0..arg_count {
                        args.push(frame.pop());
                    }
                    args.reverse();
                    if let Some(value) =
                        Self::execute_invokedynamic(class, index, descriptor, &args, heap)
                    {
                        frame.push(value);
                    } else {
                        println!("Unsupported invokedynamic #{} {}", index, descriptor);
                        return None;
                    }
                }

                Instruction::InvokeStatic(index)
                | Instruction::InvokeVirtual(index)
                | Instruction::InvokeSpecial(index)
                | Instruction::InvokeInterface(index) => {
                    let Some((cp_class_name, method_name, descriptor)) =
                        Self::resolve_method_ref(class, index)
                    else {
                        println!("Invalid method ref #{}", index);
                        return None;
                    };

                    let needs_this = matches!(
                        instr,
                        Instruction::InvokeVirtual(_)
                            | Instruction::InvokeSpecial(_)
                            | Instruction::InvokeInterface(_)
                    );
                    let arg_count = Self::count_args(descriptor);
                    let mut args = Vec::with_capacity(arg_count);
                    for _ in 0..arg_count {
                        args.push(frame.pop());
                    }
                    args.reverse();

                    let receiver = if needs_this {
                        let candidate = frame.pop();
                        if candidate.is_null() {
                            println!("NullPointerException");
                            return None;
                        }
                        Some(candidate)
                    } else {
                        None
                    };

                    if matches!(instr, Instruction::InvokeStatic(_))
                        && !self.ensure_class_initialized(class_loader, cp_class_name, heap)
                    {
                        return None;
                    }

                    if let Some(native_result) = invoke_native(
                        cp_class_name,
                        method_name,
                        descriptor,
                        receiver.clone(),
                        &args,
                        heap,
                    ) {
                        if let Some(v) = native_result {
                            frame.push(v);
                        }
                        continue;
                    }

                    let target_class_name = match instr {
                        Instruction::InvokeVirtual(_) | Instruction::InvokeInterface(_) => {
                            match receiver.as_ref() {
                                Some(HeapValue::Object(obj)) => obj.class_name.as_str(),
                                _ => cp_class_name,
                            }
                        }
                        _ => cp_class_name,
                    };

                    let mut locals = Vec::with_capacity(args.len() + receiver.is_some() as usize);
                    if let Some(this_ref) = receiver.clone() {
                        locals.push(this_ref);
                    }
                    locals.extend(args);

                    if let Some((target_class, _target_method)) = Self::resolve_method(
                        class_loader,
                        target_class_name,
                        method_name,
                        descriptor,
                    ) {
                        if let Some(retval) = self.execute_method(
                            class_loader,
                            &target_class,
                            method_name,
                            descriptor,
                            heap,
                            &locals,
                        ) {
                            frame.push(retval);
                        }
                    } else {
                        println!(
                            "Method {}{} not found for {}",
                            method_name, descriptor, target_class_name
                        );
                        return None;
                    }
                }

                Instruction::GetStatic(index) => {
                    let Some((field_class, field_name, field_desc)) =
                        Self::resolve_field_ref(class, index)
                    else {
                        println!("Invalid field ref #{}", index);
                        return None;
                    };
                    let _ = class_loader.load_class(field_class);
                    if !self.ensure_class_initialized(class_loader, field_class, heap) {
                        return None;
                    }

                    if field_class == "java/lang/System" && field_name == "out" {
                        let ps = heap.alloc_object("java/io/PrintStream");
                        frame.push(HeapValue::Object(ps));
                        continue;
                    }

                    let value = class_loader
                        .get_static_field(field_class, field_name)
                        .unwrap_or_else(|| Self::default_value_for_descriptor(field_desc));
                    frame.push(value);
                }

                Instruction::PutStatic(index) => {
                    let Some((field_class, field_name, _field_desc)) =
                        Self::resolve_field_ref(class, index)
                    else {
                        println!("Invalid field ref #{}", index);
                        return None;
                    };
                    if !self.ensure_class_initialized(class_loader, field_class, heap) {
                        return None;
                    }
                    let value = frame.pop();
                    class_loader.set_static_field(field_class, field_name, value);
                }

                Instruction::New(index) => {
                    let Some(new_class_name) = class.get_class_name(index) else {
                        println!("Invalid class ref #{}", index);
                        return None;
                    };
                    if !self.ensure_class_initialized(class_loader, new_class_name, heap) {
                        return None;
                    }
                    let obj = heap.alloc_object(new_class_name);
                    frame.push(HeapValue::Object(obj));
                }

                Instruction::NewArray(atype_code) => {
                    let count = frame.pop_int();
                    if count < 0 {
                        println!("NegativeArraySizeException");
                        return None;
                    }
                    use crate::runtime::heap::ArrayType;
                    let element_type = match atype_code {
                        4 => ArrayType::Boolean,
                        5 => ArrayType::Char,
                        6 => ArrayType::Float,
                        7 => ArrayType::Double,
                        8 => ArrayType::Byte,
                        9 => ArrayType::Short,
                        10 => ArrayType::Int,
                        11 => ArrayType::Long,
                        _ => ArrayType::Int,
                    };
                    let arr = heap.alloc_array(count as usize, element_type);
                    frame.push(HeapValue::Array(arr));
                }

                Instruction::ANewArray(_index) => {
                    let count = frame.pop_int();
                    if count < 0 {
                        println!("NegativeArraySizeException");
                        return None;
                    }
                    use crate::runtime::heap::ArrayType;
                    let arr = heap.alloc_array(count as usize, ArrayType::Reference);
                    frame.push(HeapValue::Array(arr));
                }

                Instruction::IALoad => {
                    let idx = frame.pop_int();
                    let arr_ref = frame.pop();
                    if let HeapValue::Array(arr) = arr_ref {
                        if let Some(target_arr) = heap.get_array_mut(arr.id) {
                            if idx >= 0 && (idx as usize) < target_arr.content.len() {
                                frame.push(target_arr.content[idx as usize].clone());
                            } else {
                                println!("ArrayIndexOutOfBoundsException");
                                return None;
                            }
                        }
                    }
                }

                Instruction::AALoad => {
                    let idx = frame.pop_int();
                    let arr_ref = frame.pop();
                    if let HeapValue::Array(arr) = arr_ref {
                        if let Some(target_arr) = heap.get_array_mut(arr.id) {
                            if idx >= 0 && (idx as usize) < target_arr.content.len() {
                                frame.push(target_arr.content[idx as usize].clone());
                            } else {
                                println!("ArrayIndexOutOfBoundsException");
                                return None;
                            }
                        }
                    }
                }

                Instruction::IAStore | Instruction::AAStore => {
                    let val = frame.pop();
                    let idx = frame.pop_int();
                    let arr_ref = frame.pop();

                    if let HeapValue::Array(arr) = arr_ref {
                        if let Some(target_arr) = heap.get_array_mut(arr.id) {
                            if idx >= 0 && (idx as usize) < target_arr.content.len() {
                                target_arr.content[idx as usize] = val;
                            } else {
                                println!("ArrayIndexOutOfBoundsException");
                                return None;
                            }
                        }
                    }
                }

                Instruction::Return => {
                    let _ = stack.pop_frame();
                    return None;
                }
                Instruction::IReturn
                | Instruction::LReturn
                | Instruction::FReturn
                | Instruction::DReturn
                | Instruction::AReturn => {
                    let value = frame.pop();
                    let _ = stack.pop_frame();
                    return Some(value);
                }

                _ => {
                    Self::exec_instr(frame, heap, class, instr);
                }
            }

            if heap.object_count() > 4096 {
                gc.collect(heap, &stack);
            }
        }

        None
    }

    fn resolve_method<'a>(
        loader: &'a mut ClassLoader,
        class_name: &str,
        method_name: &str,
        descriptor: &str,
    ) -> Option<(ClassFile, MethodInfo)> {
        let current_class = loader.load_class(class_name).ok()?;

        if let Some(method) = current_class
            .methods
            .iter()
            .find(|m| {
                current_class.get_utf8(m.name_index).unwrap_or("") == method_name
                    && current_class.get_utf8(m.descriptor_index).unwrap_or("") == descriptor
            })
            .cloned()
        {
            return Some((current_class.clone(), method));
        }

        if let Some(super_name) = current_class.get_class_name(current_class.super_class) {
            if super_name != "java/lang/Object"
                && !super_name.is_empty()
                && super_name != class_name
            {
                return Self::resolve_method(loader, &super_name, method_name, descriptor);
            }
        }
        None
    }

    fn branch_target(opcode_pc: usize, offset: i16, code_len: usize) -> Option<usize> {
        let target = opcode_pc as isize + offset as isize;
        if target < 0 || target as usize > code_len {
            println!("Invalid branch target: pc={} offset={}", opcode_pc, offset);
            return None;
        }
        Some(target as usize)
    }

    fn resolve_method_ref<'a>(
        class: &'a ClassFile,
        index: u16,
    ) -> Option<(&'a str, &'a str, &'a str)> {
        match Self::safe_cp_get(class, index)? {
            ConstantPoolEntry::MethodRef {
                class_index,
                name_and_type_index,
            }
            | ConstantPoolEntry::InterfaceMethodRef {
                class_index,
                name_and_type_index,
            } => {
                let class_name = class.get_class_name(*class_index)?;
                let (method_name, descriptor) = class.get_name_and_type(*name_and_type_index)?;
                Some((class_name, method_name, descriptor))
            }
            _ => None,
        }
    }

    fn resolve_field_ref<'a>(
        class: &'a ClassFile,
        index: u16,
    ) -> Option<(&'a str, &'a str, &'a str)> {
        if let ConstantPoolEntry::FieldRef {
            class_index,
            name_and_type_index,
        } = Self::safe_cp_get(class, index)?
        {
            let class_name = class.get_class_name(*class_index)?;
            let (field_name, descriptor) = class.get_name_and_type(*name_and_type_index)?;
            Some((class_name, field_name, descriptor))
        } else {
            None
        }
    }

    fn resolve_invoke_dynamic<'a>(class: &'a ClassFile, index: u16) -> Option<(&'a str, &'a str)> {
        if let ConstantPoolEntry::InvokeDynamic {
            name_and_type_index,
            ..
        } = Self::safe_cp_get(class, index)?
        {
            class.get_name_and_type(*name_and_type_index)
        } else {
            None
        }
    }

    fn ensure_class_initialized(
        &self,
        class_loader: &mut ClassLoader,
        class_name: &str,
        heap: &mut Heap,
    ) -> bool {
        let class = match class_loader.load_class(class_name) {
            Ok(c) => c,
            Err(_) if Self::is_builtin_runtime_class(class_name) => return true,
            Err(e) => {
                println!("Class initialization failed for {}: {}", class_name, e);
                return false;
            }
        };

        let canonical_name = class
            .get_class_name(class.this_class)
            .unwrap_or(class_name)
            .to_string();
        if !class_loader.begin_class_init(&canonical_name) {
            return true;
        }

        let has_clinit = class.methods.iter().any(|m| {
            class.get_utf8(m.name_index).unwrap_or("") == "<clinit>"
                && class.get_utf8(m.descriptor_index).unwrap_or("") == "()V"
        });
        if has_clinit {
            let _ = self.execute_method(class_loader, &class, "<clinit>", "()V", heap, &[]);
        }
        class_loader.finish_class_init(&canonical_name);
        true
    }

    fn is_builtin_runtime_class(class_name: &str) -> bool {
        matches!(
            class_name,
            "java/lang/Object"
                | "java/lang/String"
                | "java/lang/System"
                | "java/io/PrintStream"
                | "java/lang/Math"
        )
    }

    fn execute_invokedynamic(
        class: &ClassFile,
        index: u16,
        descriptor: &str,
        args: &[HeapValue],
        heap: &mut Heap,
    ) -> Option<HeapValue> {
        let (name, _) = Self::resolve_invoke_dynamic(class, index)?;
        if !Self::descriptor_returns_string(descriptor) {
            return None;
        }
        if name != "makeConcatWithConstants" && name != "makeConcat" {
            return None;
        }

        let rendered = if name == "makeConcatWithConstants" {
            if let Some((recipe, constants)) = Self::invokedynamic_recipe(class, index) {
                Self::render_concat_recipe(&recipe, &constants, args, heap)
            } else {
                Self::concat_values(args, heap)
            }
        } else {
            Self::concat_values(args, heap)
        };
        Some(heap.alloc_string(&rendered))
    }

    fn descriptor_returns_string(descriptor: &str) -> bool {
        descriptor
            .split_once(')')
            .map(|(_, ret)| ret == "Ljava/lang/String;")
            .unwrap_or(false)
    }

    fn invokedynamic_recipe(class: &ClassFile, index: u16) -> Option<(String, Vec<String>)> {
        let ConstantPoolEntry::InvokeDynamic {
            bootstrap_method_attr_index,
            ..
        } = Self::safe_cp_get(class, index)?
        else {
            return None;
        };

        let attr = Self::class_attribute(class, "BootstrapMethods")?;
        let methods = Self::parse_bootstrap_methods(attr)?;
        let (_, args) = methods.get(*bootstrap_method_attr_index as usize)?;
        let mut constants = Vec::new();
        let mut recipe: Option<String> = None;

        for cp_index in args {
            if let Some(text) = Self::constant_as_string(class, *cp_index) {
                if recipe.is_none() {
                    recipe = Some(text);
                } else {
                    constants.push(text);
                }
            }
        }
        recipe.map(|r| (r, constants))
    }

    fn class_attribute<'a>(class: &'a ClassFile, name: &str) -> Option<&'a AttributeInfo> {
        class
            .attributes
            .iter()
            .find(|attr| class.get_utf8(attr.name_index).unwrap_or("") == name)
    }

    fn parse_bootstrap_methods(attr: &AttributeInfo) -> Option<Vec<(u16, Vec<u16>)>> {
        let mut cursor = 0usize;
        let count = Self::read_u16(&attr.info, &mut cursor)? as usize;
        let mut methods = Vec::with_capacity(count);
        for _ in 0..count {
            let method_ref = Self::read_u16(&attr.info, &mut cursor)?;
            let arg_count = Self::read_u16(&attr.info, &mut cursor)? as usize;
            let mut args = Vec::with_capacity(arg_count);
            for _ in 0..arg_count {
                args.push(Self::read_u16(&attr.info, &mut cursor)?);
            }
            methods.push((method_ref, args));
        }
        Some(methods)
    }

    fn read_u16(bytes: &[u8], cursor: &mut usize) -> Option<u16> {
        if *cursor + 1 >= bytes.len() {
            return None;
        }
        let value = ((bytes[*cursor] as u16) << 8) | (bytes[*cursor + 1] as u16);
        *cursor += 2;
        Some(value)
    }

    fn concat_values(args: &[HeapValue], heap: &Heap) -> String {
        let mut out = String::new();
        for value in args {
            out.push_str(&Self::heap_value_string(value, heap));
        }
        out
    }

    fn render_concat_recipe(
        recipe: &str,
        constants: &[String],
        args: &[HeapValue],
        heap: &Heap,
    ) -> String {
        let mut out = String::new();
        let mut arg_it = args.iter();
        let mut const_it = constants.iter();
        for ch in recipe.chars() {
            match ch {
                '\u{1}' => {
                    if let Some(v) = arg_it.next() {
                        out.push_str(&Self::heap_value_string(v, heap));
                    }
                }
                '\u{2}' => {
                    if let Some(v) = const_it.next() {
                        out.push_str(v);
                    }
                }
                _ => out.push(ch),
            }
        }
        out
    }

    fn heap_value_string(value: &HeapValue, heap: &Heap) -> String {
        match value {
            HeapValue::Object(obj) if obj.class_name == "java/lang/String" => heap
                .get(obj.id)
                .and_then(|real| real.get_field("value"))
                .map(|v| match v {
                    HeapValue::String(s) => s.clone(),
                    other => other.to_string(),
                })
                .unwrap_or_else(|| "null".to_string()),
            HeapValue::String(s) => s.clone(),
            HeapValue::Null => "null".to_string(),
            other => other.to_string(),
        }
    }

    fn constant_as_string(class: &ClassFile, index: u16) -> Option<String> {
        match Self::safe_cp_get(class, index)? {
            ConstantPoolEntry::String { string_index } => {
                class.get_utf8(*string_index).map(|s| s.to_string())
            }
            ConstantPoolEntry::Utf8(s) => Some(s.clone()),
            ConstantPoolEntry::Integer(v) => Some(v.to_string()),
            ConstantPoolEntry::Long(v) => Some(v.to_string()),
            ConstantPoolEntry::Float(v) => Some(v.to_string()),
            ConstantPoolEntry::Double(v) => Some(v.to_string()),
            _ => None,
        }
    }

    fn push_constant(frame: &mut Frame, heap: &mut Heap, class: &ClassFile, index: u16) {
        if let Some(entry) = Self::safe_cp_get(class, index) {
            match entry {
                ConstantPoolEntry::Integer(v) => frame.push(HeapValue::Int(*v)),
                ConstantPoolEntry::Float(v) => frame.push(HeapValue::Float(*v)),
                ConstantPoolEntry::Long(v) => frame.push(HeapValue::Long(*v)),
                ConstantPoolEntry::Double(v) => frame.push(HeapValue::Double(*v)),
                ConstantPoolEntry::String { string_index } => {
                    if let Some(s) = class.get_utf8(*string_index) {
                        frame.push(heap.alloc_string(s));
                    } else {
                        frame.push(HeapValue::Null);
                    }
                }
                ConstantPoolEntry::Utf8(value) => frame.push(HeapValue::String(value.clone())),
                ConstantPoolEntry::Class { name_index } => {
                    let class_name = class.get_utf8(*name_index).unwrap_or("");
                    frame.push(HeapValue::String(class_name.to_string()));
                }
                _ => {
                    println!("Unsupported LDC entry {:?}", entry);
                    frame.push(HeapValue::Null);
                }
            }
        } else {
            frame.push(HeapValue::Null);
        }
    }

    fn default_value_for_descriptor(descriptor: &str) -> HeapValue {
        match descriptor.chars().next() {
            Some('Z') | Some('B') | Some('C') | Some('S') | Some('I') => HeapValue::Int(0),
            Some('J') => HeapValue::Long(0),
            Some('F') => HeapValue::Float(0.0),
            Some('D') => HeapValue::Double(0.0),
            _ => HeapValue::Null,
        }
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

            Instruction::Ldc(index) => Self::push_constant(frame, heap, class, u16::from(index)),
            Instruction::LdcW(index) | Instruction::Ldc2W(index) => {
                Self::push_constant(frame, heap, class, index)
            }

            Instruction::GetField(index) => {
                let obj_ref = frame.pop();
                if let HeapValue::Object(obj) = obj_ref {
                    if let Some(ConstantPoolEntry::FieldRef {
                        name_and_type_index,
                        ..
                    }) = Self::safe_cp_get(class, index)
                    {
                        if let Some(ConstantPoolEntry::NameAndType { name_index, .. }) =
                            class.constant_pool.get((*name_and_type_index - 1) as usize)
                        {
                            let field_name = class.get_utf8(*name_index).unwrap_or("<unknown>");
                            let val = heap
                                .get(obj.id)
                                .and_then(|real| real.fields.get(field_name))
                                .cloned();
                            if let Some(val) = val {
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
                    if let Some(ConstantPoolEntry::FieldRef {
                        name_and_type_index,
                        ..
                    }) = Self::safe_cp_get(class, index)
                    {
                        if let Some(ConstantPoolEntry::NameAndType { name_index, .. }) =
                            class.constant_pool.get((*name_and_type_index - 1) as usize)
                        {
                            let field_name = class.get_utf8(*name_index).unwrap_or("<unknown>");
                            obj.fields.insert(field_name.to_string(), value.clone());
                            if let Some(target) = heap.get_mut(obj.id) {
                                target.fields.insert(field_name.to_string(), value.clone());
                            }
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
