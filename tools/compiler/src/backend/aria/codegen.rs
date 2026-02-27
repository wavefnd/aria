use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use crate::backend::aria::ast::*;
use crate::backend::aria::sema::SourceFileAst;

const CLASSFILE_MAJOR_VERSION: u16 = 61;

#[derive(Debug)]
pub struct CodegenError {
    pub path: PathBuf,
    pub line: usize,
    pub col: usize,
    pub message: String,
}

pub fn emit_classes(files: &[SourceFileAst], out_dir: &Path) -> Result<(), Vec<CodegenError>> {
    let mut errors = Vec::new();
    let class_members = build_class_members(files);
    for file in files {
        for class in &file.unit.classes {
            if let Err(err) = emit_class(file.path.as_path(), class, out_dir, &class_members) {
                errors.push(err);
            }
        }
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

fn build_class_members(files: &[SourceFileAst]) -> HashMap<String, ClassMembers> {
    let mut out = HashMap::<String, ClassMembers>::new();
    for file in files {
        for class in &file.unit.classes {
            let mut members = ClassMembers::default();
            for member in &class.members {
                match member {
                    MemberDecl::Field(field) => {
                        members.fields.insert(field.name.clone(), field.ty.clone());
                    }
                    MemberDecl::Method(method) => {
                        let _ = members.methods.add(method);
                    }
                }
            }
            out.insert(class.name.clone(), members.clone());
            out.entry(class.name.replace('.', "/"))
                .or_insert_with(|| members.clone());
            out.entry(class.name.replace('/', ".")).or_insert(members);
        }
    }
    out
}

fn emit_class(
    path: &Path,
    class: &ClassDecl,
    out_dir: &Path,
    class_members: &HashMap<String, ClassMembers>,
) -> Result<(), CodegenError> {
    fs::create_dir_all(out_dir).map_err(|e| CodegenError {
        path: path.to_path_buf(),
        line: class.span.line,
        col: class.span.col,
        message: format!("failed to create output directory: {}", e),
    })?;

    let class_name_slash = class.name.replace('.', "/");
    let mut cp = ConstantPool::new();
    let this_class = cp.class(&class_name_slash);
    let super_class = cp.class("java/lang/Object");
    let code_attr_name = cp.utf8("Code");
    let stack_map_attr_name = cp.utf8("StackMapTable");

    let mut fields_out = Vec::new();
    for member in &class.members {
        if let MemberDecl::Field(field) = member {
            let name_idx = cp.utf8(&field.name);
            let desc = type_descriptor(&field.ty).map_err(|m| CodegenError {
                path: path.to_path_buf(),
                line: field.span.line,
                col: field.span.col,
                message: m,
            })?;
            let desc_idx = cp.utf8(&desc);
            let access = 0x0001u16;
            fields_out.push(FieldOut {
                access,
                name_idx,
                desc_idx,
            });
        }
    }

    let mut methods_out = Vec::new();
    methods_out.push(default_constructor_method(
        class,
        &mut cp,
        code_attr_name,
        stack_map_attr_name,
    ));
    for member in &class.members {
        if let MemberDecl::Method(method) = member {
            let method_ctx = MethodContext {
                class,
                class_members,
            };
            let code = compile_method(path, &method_ctx, method, &mut cp)?;
            let name_idx = cp.utf8(&method.name);
            let desc = method_descriptor(method).map_err(|m| CodegenError {
                path: path.to_path_buf(),
                line: method.span.line,
                col: method.span.col,
                message: m,
            })?;
            let desc_idx = cp.utf8(&desc);
            let mut access = 0u16;
            if method.is_public {
                access |= 0x0001;
            }
            if method.is_static {
                access |= 0x0008;
            }
            methods_out.push(MethodOut {
                access,
                name_idx,
                desc_idx,
                code,
                code_attr_name,
                stack_map_attr_name,
            });
        }
    }

    let mut bytes = Vec::new();
    push_u4(&mut bytes, 0xCAFEBABE);
    push_u2(&mut bytes, 0);
    push_u2(&mut bytes, CLASSFILE_MAJOR_VERSION);

    let cp_bytes = cp.to_bytes();
    push_u2(
        &mut bytes,
        u16::try_from(cp.count_with_implicit_zero()).unwrap_or(u16::MAX),
    );
    bytes.extend(cp_bytes);

    push_u2(&mut bytes, 0x0021);
    push_u2(&mut bytes, this_class);
    push_u2(&mut bytes, super_class);

    push_u2(&mut bytes, 0);
    push_u2(&mut bytes, u16::try_from(fields_out.len()).unwrap_or(0));
    for f in &fields_out {
        push_u2(&mut bytes, f.access);
        push_u2(&mut bytes, f.name_idx);
        push_u2(&mut bytes, f.desc_idx);
        push_u2(&mut bytes, 0);
    }

    push_u2(&mut bytes, u16::try_from(methods_out.len()).unwrap_or(0));
    for m in &methods_out {
        push_u2(&mut bytes, m.access);
        push_u2(&mut bytes, m.name_idx);
        push_u2(&mut bytes, m.desc_idx);
        push_u2(&mut bytes, 1);
        push_u2(&mut bytes, m.code_attr_name);
        let code_len_u32 = u32::try_from(m.code.code.len()).unwrap_or(u32::MAX);
        let mut code_nested_attr_len = 0u32;
        let mut code_nested_attr_count = 0u16;
        if m.code.stack_map_table.len() > 2 {
            code_nested_attr_len +=
                6 + u32::try_from(m.code.stack_map_table.len()).unwrap_or(u32::MAX);
            code_nested_attr_count += 1;
        }
        let attr_len = 12 + code_len_u32 + code_nested_attr_len;
        push_u4(&mut bytes, attr_len);
        push_u2(&mut bytes, m.code.max_stack);
        push_u2(&mut bytes, m.code.max_locals);
        push_u4(&mut bytes, code_len_u32);
        bytes.extend(&m.code.code);
        push_u2(&mut bytes, 0);
        push_u2(&mut bytes, code_nested_attr_count);
        if m.code.stack_map_table.len() > 2 {
            push_u2(&mut bytes, m.stack_map_attr_name);
            push_u4(
                &mut bytes,
                u32::try_from(m.code.stack_map_table.len()).unwrap_or(u32::MAX),
            );
            bytes.extend(&m.code.stack_map_table);
        }
    }

    push_u2(&mut bytes, 0);

    let out_file = out_dir.join(format!("{}.class", class.name.replace('.', "/")));
    if let Some(parent) = out_file.parent() {
        fs::create_dir_all(parent).map_err(|e| CodegenError {
            path: path.to_path_buf(),
            line: class.span.line,
            col: class.span.col,
            message: format!(
                "failed to create class output directory {}: {}",
                parent.display(),
                e
            ),
        })?;
    }
    fs::write(&out_file, bytes).map_err(|e| CodegenError {
        path: path.to_path_buf(),
        line: class.span.line,
        col: class.span.col,
        message: format!("failed to write class file {}: {}", out_file.display(), e),
    })?;
    Ok(())
}

fn default_constructor_method(
    class: &ClassDecl,
    cp: &mut ConstantPool,
    code_attr_name: u16,
    stack_map_attr_name: u16,
) -> MethodOut {
    let owner = cp.class("java/lang/Object");
    let nat = cp.name_and_type("<init>", "()V");
    let super_init = cp.method_ref(owner, nat);

    let mut code = Vec::new();
    code.push(0x2a);
    code.push(0xb7);
    push_u2(&mut code, super_init);
    code.push(0xb1);

    MethodOut {
        access: if class.is_public { 0x0001 } else { 0x0000 },
        name_idx: cp.utf8("<init>"),
        desc_idx: cp.utf8("()V"),
        code: MethodCode {
            max_stack: 1,
            max_locals: 1,
            code,
            stack_map_table: vec![0, 0],
        },
        code_attr_name,
        stack_map_attr_name,
    }
}

#[derive(Default, Clone)]
struct MethodSigs {
    by_name: HashMap<String, Vec<MethodSig>>,
}

#[derive(Default, Clone)]
struct ClassMembers {
    fields: HashMap<String, TypeName>,
    methods: MethodSigs,
}

#[derive(Clone)]
struct MethodSig {
    params: Vec<TypeName>,
    return_type: TypeName,
    is_static: bool,
}

impl MethodSigs {
    fn add(&mut self, method: &MethodDecl) -> Result<(), CodegenError> {
        self.by_name
            .entry(method.name.clone())
            .or_default()
            .push(MethodSig {
                params: method.params.iter().map(|p| p.ty.clone()).collect(),
                return_type: method.return_type.clone(),
                is_static: method.is_static,
            });
        Ok(())
    }

    fn resolve(&self, name: &str, argc: usize) -> Option<&MethodSig> {
        self.by_name
            .get(name)
            .and_then(|v| v.iter().find(|m| m.params.len() == argc))
    }
}

fn find_class_members<'a>(
    class_members: &'a HashMap<String, ClassMembers>,
    class_name: &str,
) -> Option<&'a ClassMembers> {
    class_members
        .get(class_name)
        .or_else(|| class_members.get(&class_name.replace('/', ".")))
        .or_else(|| class_members.get(&class_name.replace('.', "/")))
}

fn resolve_method_sig_for_class(
    class_members: &HashMap<String, ClassMembers>,
    class_name: &str,
    method_name: &str,
    argc: usize,
) -> Option<MethodSig> {
    let members = find_class_members(class_members, class_name)?;
    members.methods.resolve(method_name, argc).cloned()
}

struct MethodContext<'a> {
    class: &'a ClassDecl,
    class_members: &'a HashMap<String, ClassMembers>,
}

struct FieldOut {
    access: u16,
    name_idx: u16,
    desc_idx: u16,
}

struct MethodOut {
    access: u16,
    name_idx: u16,
    desc_idx: u16,
    code: MethodCode,
    code_attr_name: u16,
    stack_map_attr_name: u16,
}

#[derive(Clone)]
struct MethodCode {
    max_stack: u16,
    max_locals: u16,
    code: Vec<u8>,
    stack_map_table: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum EvalType {
    Int,
    Bool,
    Ref(String),
    ClassRef(String),
    Void,
}

#[derive(Clone)]
struct Local {
    slot: u16,
    ty: EvalType,
}

struct LocalScopes {
    scopes: Vec<HashMap<String, Local>>,
    next_slot: u16,
}

impl LocalScopes {
    fn new(next_slot: u16) -> Self {
        Self {
            scopes: vec![HashMap::new()],
            next_slot,
        }
    }

    fn enter(&mut self) {
        self.scopes.push(HashMap::new());
    }

    fn exit(&mut self) {
        if self.scopes.len() > 1 {
            self.scopes.pop();
        }
    }

    fn define(&mut self, name: String, ty: EvalType) -> u16 {
        let slot = self.next_slot;
        self.next_slot += 1;
        if let Some(scope) = self.scopes.last_mut() {
            scope.insert(name, Local { slot, ty });
        }
        slot
    }

    fn get(&self, name: &str) -> Option<&Local> {
        for s in self.scopes.iter().rev() {
            if let Some(v) = s.get(name) {
                return Some(v);
            }
        }
        None
    }

    fn active_locals(&self) -> Vec<VerificationType> {
        let mut out = vec![VerificationType::Top; self.next_slot as usize];
        for scope in &self.scopes {
            for local in scope.values() {
                if let Some(slot) = out.get_mut(local.slot as usize) {
                    *slot = verify_type_from_eval(&local.ty);
                }
            }
        }
        trim_trailing_top(&mut out);
        out
    }
}

struct CodeBuilder {
    bytes: Vec<u8>,
    labels: Vec<LabelInfo>,
    patches: Vec<JumpPatch>,
}

#[derive(Clone, Copy)]
struct JumpPatch {
    opcode_pos: usize,
    offset_pos: usize,
    label: usize,
}

#[derive(Clone)]
struct LabelInfo {
    offset: Option<usize>,
    frame: Option<FrameState>,
    referenced: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct FrameState {
    locals: Vec<VerificationType>,
    stack: Vec<VerificationType>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum VerificationType {
    Top,
    Integer,
    Null,
    Object(String),
}

impl CodeBuilder {
    fn new() -> Self {
        Self {
            bytes: Vec::new(),
            labels: Vec::new(),
            patches: Vec::new(),
        }
    }

    fn emit_u1(&mut self, v: u8) {
        self.bytes.push(v);
    }

    fn emit_u2(&mut self, v: u16) {
        push_u2(&mut self.bytes, v);
    }

    fn new_label(&mut self) -> usize {
        self.labels.push(LabelInfo {
            offset: None,
            frame: None,
            referenced: false,
        });
        self.labels.len() - 1
    }

    fn bind_label(&mut self, label: usize) -> Result<(), String> {
        let Some(info) = self.labels.get_mut(label) else {
            return Err(format!("invalid label id {}", label));
        };
        if info.offset.is_some() {
            return Err(format!("label {} already bound", label));
        }
        info.offset = Some(self.bytes.len());
        Ok(())
    }

    fn bind_label_with_frame(&mut self, label: usize, frame: FrameState) -> Result<(), String> {
        self.record_label_frame(label, frame)?;
        self.bind_label(label)
    }

    fn record_label_frame(&mut self, label: usize, frame: FrameState) -> Result<(), String> {
        let Some(info) = self.labels.get_mut(label) else {
            return Err(format!("invalid label id {}", label));
        };
        if let Some(existing) = &info.frame {
            if existing != &frame {
                return Err(format!("incompatible frame merge for label {}", label));
            }
        } else {
            info.frame = Some(frame);
        }
        Ok(())
    }

    fn emit_jump(&mut self, opcode: u8, label: usize) -> Result<(), String> {
        let Some(info) = self.labels.get_mut(label) else {
            return Err(format!("invalid label id {}", label));
        };
        info.referenced = true;
        let opcode_pos = self.bytes.len();
        self.emit_u1(opcode);
        let offset_pos = self.bytes.len();
        self.emit_u1(0);
        self.emit_u1(0);
        self.patches.push(JumpPatch {
            opcode_pos,
            offset_pos,
            label,
        });
        Ok(())
    }

    fn patch_labels(&mut self) -> Result<(), String> {
        for patch in &self.patches {
            let Some(target) = self.labels.get(patch.label).and_then(|x| x.offset) else {
                return Err(format!("unresolved label {}", patch.label));
            };
            let offset = target as isize - patch.opcode_pos as isize;
            if offset < i16::MIN as isize || offset > i16::MAX as isize {
                return Err("branch offset out of i16 range".to_string());
            }
            let off = offset as i16 as u16;
            self.bytes[patch.offset_pos] = (off >> 8) as u8;
            self.bytes[patch.offset_pos + 1] = (off & 0xff) as u8;
        }
        Ok(())
    }

    fn emit_push_int(&mut self, v: i64, cp: &mut ConstantPool) -> Result<(), String> {
        if v == -1 {
            self.emit_u1(0x02);
            return Ok(());
        }
        if (0..=5).contains(&v) {
            self.emit_u1(0x03 + u8::try_from(v).map_err(|_| "int literal overflow".to_string())?);
            return Ok(());
        }
        if (-128..=127).contains(&v) {
            self.emit_u1(0x10);
            self.emit_u1(v as i8 as u8);
            return Ok(());
        }
        if (-32768..=32767).contains(&v) {
            self.emit_u1(0x11);
            self.emit_u2(v as i16 as u16);
            return Ok(());
        }
        let idx = cp.integer(i32::try_from(v).map_err(|_| "int literal overflow".to_string())?);
        if idx > u8::MAX as u16 {
            return Err("constant pool too large for ldc".to_string());
        }
        self.emit_u1(0x12);
        self.emit_u1(idx as u8);
        Ok(())
    }

    fn emit_load(&mut self, slot: u16, ty: &EvalType) -> Result<(), String> {
        match ty {
            EvalType::Int | EvalType::Bool => {
                if slot <= 3 {
                    self.emit_u1(0x1a + slot as u8);
                } else if slot <= 255 {
                    self.emit_u1(0x15);
                    self.emit_u1(slot as u8);
                } else {
                    return Err("local variable slot too large".to_string());
                }
            }
            EvalType::Ref(_) | EvalType::ClassRef(_) => {
                if slot <= 3 {
                    self.emit_u1(0x2a + slot as u8);
                } else if slot <= 255 {
                    self.emit_u1(0x19);
                    self.emit_u1(slot as u8);
                } else {
                    return Err("local variable slot too large".to_string());
                }
            }
            EvalType::Void => return Err("cannot load void".to_string()),
        }
        Ok(())
    }

    fn emit_store(&mut self, slot: u16, ty: &EvalType) -> Result<(), String> {
        match ty {
            EvalType::Int | EvalType::Bool => {
                if slot <= 3 {
                    self.emit_u1(0x3b + slot as u8);
                } else if slot <= 255 {
                    self.emit_u1(0x36);
                    self.emit_u1(slot as u8);
                } else {
                    return Err("local variable slot too large".to_string());
                }
            }
            EvalType::Ref(_) | EvalType::ClassRef(_) => {
                if slot <= 3 {
                    self.emit_u1(0x4b + slot as u8);
                } else if slot <= 255 {
                    self.emit_u1(0x3a);
                    self.emit_u1(slot as u8);
                } else {
                    return Err("local variable slot too large".to_string());
                }
            }
            EvalType::Void => return Err("cannot store void".to_string()),
        }
        Ok(())
    }

    fn finish(
        mut self,
        max_locals: u16,
        initial_frame: FrameState,
        cp: &mut ConstantPool,
    ) -> Result<MethodCode, String> {
        self.materialize_terminal_labels();
        self.patch_labels()?;
        let stack_map_table = self.build_stack_map_table(&initial_frame, cp)?;
        Ok(MethodCode {
            max_stack: 64,
            max_locals,
            code: self.bytes,
            stack_map_table,
        })
    }

    fn materialize_terminal_labels(&mut self) {
        let end = self.bytes.len();
        if self
            .labels
            .iter()
            .any(|label| label.referenced && label.offset == Some(end))
        {
            self.emit_u1(0x00);
        }
    }

    fn build_stack_map_table(
        &self,
        initial_frame: &FrameState,
        cp: &mut ConstantPool,
    ) -> Result<Vec<u8>, String> {
        let mut targets = Vec::<(usize, FrameState)>::new();
        for (idx, label) in self.labels.iter().enumerate() {
            if !label.referenced {
                continue;
            }
            let Some(offset) = label.offset else {
                return Err(format!("unbound referenced label {}", idx));
            };
            let Some(frame) = label.frame.clone() else {
                return Err(format!("missing stack map frame for label {}", idx));
            };
            targets.push((offset, frame));
        }
        targets.sort_by_key(|(offset, _)| *offset);

        let mut dedup = Vec::<(usize, FrameState)>::new();
        for (offset, frame) in targets {
            if let Some((last_offset, last_frame)) = dedup.last() {
                if *last_offset == offset {
                    if last_frame != &frame {
                        return Err(format!("conflicting stack map frames at offset {}", offset));
                    }
                    continue;
                }
            }
            dedup.push((offset, frame));
        }

        let mut out = Vec::new();
        push_u2(
            &mut out,
            u16::try_from(dedup.len()).map_err(|_| "too many stack map frames".to_string())?,
        );

        let mut prev_offset: isize = -1;
        let mut prev_frame = initial_frame.clone();
        for (offset, frame) in dedup {
            let delta = offset as isize - prev_offset - 1;
            if !(0..=u16::MAX as isize).contains(&delta) {
                return Err(format!("invalid frame offset delta at {}", offset));
            }
            emit_frame(&mut out, delta as u16, &prev_frame, &frame, cp)?;
            prev_offset = offset as isize;
            prev_frame = frame;
        }

        Ok(out)
    }
}

fn compile_method(
    path: &Path,
    ctx: &MethodContext<'_>,
    method: &MethodDecl,
    cp: &mut ConstantPool,
) -> Result<MethodCode, CodegenError> {
    let mut code = CodeBuilder::new();
    let mut locals = LocalScopes::new(0);
    if !method.is_static {
        locals.define("this".to_string(), EvalType::Ref(ctx.class.name.clone()));
    }
    for param in &method.params {
        locals.define(param.name.clone(), eval_type_from_ast(&param.ty));
    }

    let initial_frame = FrameState {
        locals: locals.active_locals(),
        stack: Vec::new(),
    };

    compile_stmt(path, ctx, &mut code, &mut locals, &method.body, cp)?;

    if method.return_type == TypeName::Void {
        if !matches!(code.bytes.last(), Some(0xb1)) {
            code.emit_u1(0xb1);
        }
    }

    code.finish(locals.next_slot, initial_frame, cp)
        .map_err(|m| cg_err(path, method.span, m))
}

fn compile_stmt(
    path: &Path,
    ctx: &MethodContext<'_>,
    code: &mut CodeBuilder,
    locals: &mut LocalScopes,
    stmt: &Stmt,
    cp: &mut ConstantPool,
) -> Result<(), CodegenError> {
    match stmt {
        Stmt::Block(stmts, _) => {
            locals.enter();
            for s in stmts {
                compile_stmt(path, ctx, code, locals, s, cp)?;
                if stmt_always_returns(s) {
                    break;
                }
            }
            locals.exit();
            Ok(())
        }
        Stmt::LocalVar {
            ty,
            name,
            init,
            span,
        } => {
            let eval_ty = eval_type_from_ast(ty);
            let slot = locals.define(name.clone(), eval_ty.clone());
            if let Some(expr) = init {
                let t = compile_expr(path, ctx, code, locals, expr, cp)?;
                if !is_assignable(&eval_ty, &t) {
                    return Err(cg_err(
                        path,
                        expr.span,
                        format!(
                            "cannot initialize '{}' of type {:?} with value type {:?}",
                            name, eval_ty, t
                        ),
                    ));
                }
            } else {
                emit_default_value(code, &eval_ty, cp).map_err(|m| cg_err(path, *span, m))?;
            }
            code.emit_store(slot, &eval_ty)
                .map_err(|m| cg_err(path, *span, m))?;
            Ok(())
        }
        Stmt::Expr(expr, span) => {
            let t = compile_expr(path, ctx, code, locals, expr, cp)?;
            if !matches!(t, EvalType::Void) {
                code.emit_u1(0x57);
            }
            if matches!(t, EvalType::ClassRef(_)) {
                return Err(cg_err(
                    path,
                    *span,
                    "class literal cannot be used as expression statement",
                ));
            }
            Ok(())
        }
        Stmt::Return(None, _) => {
            code.emit_u1(0xb1);
            Ok(())
        }
        Stmt::Return(Some(expr), span) => {
            let t = compile_expr(path, ctx, code, locals, expr, cp)?;
            match t {
                EvalType::Int | EvalType::Bool => code.emit_u1(0xac),
                EvalType::Ref(_) => code.emit_u1(0xb0),
                EvalType::Void | EvalType::ClassRef(_) => {
                    return Err(cg_err(path, *span, "invalid return expression"))
                }
            }
            Ok(())
        }
        Stmt::If {
            cond,
            then_branch,
            else_branch,
            span,
        } => {
            let else_label = code.new_label();
            let then_returns = stmt_always_returns(then_branch);
            let else_returns = else_branch
                .as_deref()
                .map(stmt_always_returns)
                .unwrap_or(false);
            let end_label = if else_branch.is_some() && !(then_returns && else_returns) {
                Some(code.new_label())
            } else {
                None
            };
            emit_condition_jump_false(path, ctx, code, locals, cond, cp, else_label)?;
            compile_stmt(path, ctx, code, locals, then_branch, cp)?;
            if let Some(join_label) = end_label {
                let join_frame = empty_frame_from_locals(locals);
                code.record_label_frame(join_label, join_frame)
                    .map_err(|m| cg_err(path, *span, m))?;
                code.emit_jump(0xa7, join_label)
                    .map_err(|m| cg_err(path, *span, m))?;
            }
            code.bind_label_with_frame(else_label, empty_frame_from_locals(locals))
                .map_err(|m| cg_err(path, *span, m))?;
            if let Some(else_stmt) = else_branch {
                compile_stmt(path, ctx, code, locals, else_stmt, cp)?;
                if let Some(join_label) = end_label {
                    code.bind_label_with_frame(join_label, empty_frame_from_locals(locals))
                        .map_err(|m| cg_err(path, *span, m))?;
                }
            }
            Ok(())
        }
        Stmt::While { cond, body, span } => {
            let start_label = code.new_label();
            let end_label = code.new_label();
            let loop_frame = empty_frame_from_locals(locals);
            code.record_label_frame(start_label, loop_frame.clone())
                .map_err(|m| cg_err(path, *span, m))?;
            code.record_label_frame(end_label, loop_frame.clone())
                .map_err(|m| cg_err(path, *span, m))?;
            code.bind_label_with_frame(start_label, loop_frame)
                .map_err(|m| cg_err(path, *span, m))?;
            emit_condition_jump_false(path, ctx, code, locals, cond, cp, end_label)?;
            compile_stmt(path, ctx, code, locals, body, cp)?;
            code.record_label_frame(start_label, empty_frame_from_locals(locals))
                .map_err(|m| cg_err(path, *span, m))?;
            code.emit_jump(0xa7, start_label)
                .map_err(|m| cg_err(path, *span, m))?;
            code.bind_label_with_frame(end_label, empty_frame_from_locals(locals))
                .map_err(|m| cg_err(path, *span, m))?;
            Ok(())
        }
        Stmt::Empty(_) => Ok(()),
    }
}

fn emit_condition_jump_false(
    path: &Path,
    ctx: &MethodContext<'_>,
    code: &mut CodeBuilder,
    locals: &mut LocalScopes,
    cond: &Expr,
    cp: &mut ConstantPool,
    false_label: usize,
) -> Result<(), CodegenError> {
    let cond_ty = compile_expr(path, ctx, code, locals, cond, cp)?;
    match cond_ty {
        EvalType::Bool | EvalType::Int => {
            code.record_label_frame(false_label, empty_frame_from_locals(locals))
                .map_err(|m| cg_err(path, cond.span, m))?;
            code.emit_jump(0x99, false_label)
                .map_err(|m| cg_err(path, cond.span, m))?;
            Ok(())
        }
        _ => Err(cg_err(
            path,
            cond.span,
            "condition expression must evaluate to int/boolean",
        )),
    }
}

fn emit_bool_result_from_branch(
    path: &Path,
    code: &mut CodeBuilder,
    locals: &LocalScopes,
    span: Span,
    true_branch_opcode: u8,
) -> Result<(), CodegenError> {
    let true_label = code.new_label();
    let end_label = code.new_label();
    code.record_label_frame(true_label, empty_frame_from_locals(locals))
        .map_err(|m| cg_err(path, span, m))?;
    code.record_label_frame(
        end_label,
        frame_with_stack(locals, vec![VerificationType::Integer]),
    )
    .map_err(|m| cg_err(path, span, m))?;
    code.emit_jump(true_branch_opcode, true_label)
        .map_err(|m| cg_err(path, span, m))?;
    code.emit_u1(0x03);
    code.emit_jump(0xa7, end_label)
        .map_err(|m| cg_err(path, span, m))?;
    code.bind_label_with_frame(true_label, empty_frame_from_locals(locals))
        .map_err(|m| cg_err(path, span, m))?;
    code.emit_u1(0x04);
    code.bind_label_with_frame(
        end_label,
        frame_with_stack(locals, vec![VerificationType::Integer]),
    )
    .map_err(|m| cg_err(path, span, m))?;
    Ok(())
}

fn compile_expr(
    path: &Path,
    ctx: &MethodContext<'_>,
    code: &mut CodeBuilder,
    locals: &mut LocalScopes,
    expr: &Expr,
    cp: &mut ConstantPool,
) -> Result<EvalType, CodegenError> {
    match &expr.kind {
        ExprKind::IntLiteral(v) => {
            code.emit_push_int(*v, cp)
                .map_err(|m| cg_err(path, expr.span, m))?;
            Ok(EvalType::Int)
        }
        ExprKind::BoolLiteral(v) => {
            code.emit_push_int(if *v { 1 } else { 0 }, cp)
                .map_err(|m| cg_err(path, expr.span, m))?;
            Ok(EvalType::Bool)
        }
        ExprKind::StringLiteral(v) => {
            let idx = cp.string(v);
            if idx > u8::MAX as u16 {
                return Err(cg_err(
                    path,
                    expr.span,
                    "constant pool too large for string ldc",
                ));
            }
            code.emit_u1(0x12);
            code.emit_u1(idx as u8);
            Ok(EvalType::Ref("java/lang/String".to_string()))
        }
        ExprKind::Null => {
            code.emit_u1(0x01);
            Ok(EvalType::Ref("null".to_string()))
        }
        ExprKind::This => {
            let Some(local) = locals.get("this").cloned() else {
                return Err(cg_err(
                    path,
                    expr.span,
                    "'this' is not available in static context",
                ));
            };
            code.emit_load(local.slot, &local.ty)
                .map_err(|m| cg_err(path, expr.span, m))?;
            Ok(local.ty)
        }
        ExprKind::Var(name) => {
            if let Some(local) = locals.get(name) {
                code.emit_load(local.slot, &local.ty)
                    .map_err(|m| cg_err(path, expr.span, m))?;
                return Ok(local.ty.clone());
            }
            if name == "System" {
                return Ok(EvalType::ClassRef("java/lang/System".to_string()));
            }
            if find_class_members(ctx.class_members, name).is_some() {
                return Ok(EvalType::ClassRef(normalize_class_name(name)));
            }
            Err(cg_err(
                path,
                expr.span,
                format!("unknown variable '{}'", name),
            ))
        }
        ExprKind::New { class_name, args } => {
            if !args.is_empty() {
                return Err(cg_err(
                    path,
                    expr.span,
                    format!(
                        "object creation with constructor arguments is not implemented yet: {} argument(s)",
                        args.len()
                    ),
                ));
            }
            let owner_name = normalize_class_name(class_name);
            if find_class_members(ctx.class_members, class_name).is_none()
                && find_class_members(ctx.class_members, &owner_name).is_none()
            {
                return Err(cg_err(
                    path,
                    expr.span,
                    format!("cannot resolve class '{}' for object creation", class_name),
                ));
            }

            let owner = cp.class(&owner_name);
            let nat = cp.name_and_type("<init>", "()V");
            let init_ref = cp.method_ref(owner, nat);
            code.emit_u1(0xbb);
            code.emit_u2(owner);
            code.emit_u1(0x59);
            code.emit_u1(0xb7);
            code.emit_u2(init_ref);
            Ok(EvalType::Ref(owner_name))
        }
        ExprKind::Unary { op, expr: inner } => {
            let t = compile_expr(path, ctx, code, locals, inner, cp)?;
            match op {
                UnaryOp::Neg => {
                    if !matches!(t, EvalType::Int) {
                        return Err(cg_err(path, expr.span, "unary '-' requires int"));
                    }
                    code.emit_u1(0x74);
                    Ok(EvalType::Int)
                }
                UnaryOp::Not => {
                    if !matches!(t, EvalType::Bool) {
                        return Err(cg_err(path, expr.span, "unary '!' requires boolean"));
                    }
                    code.emit_u1(0x04);
                    code.emit_u1(0x82);
                    Ok(EvalType::Bool)
                }
            }
        }
        ExprKind::Binary { op, left, right } => match op {
            BinaryOp::And => {
                let false_label = code.new_label();
                let end_label = code.new_label();
                code.record_label_frame(false_label, empty_frame_from_locals(locals))
                    .map_err(|m| cg_err(path, expr.span, m))?;
                code.record_label_frame(
                    end_label,
                    frame_with_stack(locals, vec![VerificationType::Integer]),
                )
                .map_err(|m| cg_err(path, expr.span, m))?;
                let lt = compile_expr(path, ctx, code, locals, left, cp)?;
                if !is_int_or_bool(&lt) {
                    return Err(cg_err(
                        path,
                        left.span,
                        "left operand of && must be int/boolean",
                    ));
                }
                code.emit_jump(0x99, false_label)
                    .map_err(|m| cg_err(path, expr.span, m))?;
                let rt = compile_expr(path, ctx, code, locals, right, cp)?;
                if !is_int_or_bool(&rt) {
                    return Err(cg_err(
                        path,
                        right.span,
                        "right operand of && must be int/boolean",
                    ));
                }
                code.emit_jump(0x99, false_label)
                    .map_err(|m| cg_err(path, expr.span, m))?;
                code.emit_u1(0x04);
                code.emit_jump(0xa7, end_label)
                    .map_err(|m| cg_err(path, expr.span, m))?;
                code.bind_label_with_frame(false_label, empty_frame_from_locals(locals))
                    .map_err(|m| cg_err(path, expr.span, m))?;
                code.emit_u1(0x03);
                code.bind_label_with_frame(
                    end_label,
                    frame_with_stack(locals, vec![VerificationType::Integer]),
                )
                .map_err(|m| cg_err(path, expr.span, m))?;
                Ok(EvalType::Bool)
            }
            BinaryOp::Or => {
                let true_label = code.new_label();
                let end_label = code.new_label();
                code.record_label_frame(true_label, empty_frame_from_locals(locals))
                    .map_err(|m| cg_err(path, expr.span, m))?;
                code.record_label_frame(
                    end_label,
                    frame_with_stack(locals, vec![VerificationType::Integer]),
                )
                .map_err(|m| cg_err(path, expr.span, m))?;
                let lt = compile_expr(path, ctx, code, locals, left, cp)?;
                if !is_int_or_bool(&lt) {
                    return Err(cg_err(
                        path,
                        left.span,
                        "left operand of || must be int/boolean",
                    ));
                }
                code.emit_jump(0x9a, true_label)
                    .map_err(|m| cg_err(path, expr.span, m))?;
                let rt = compile_expr(path, ctx, code, locals, right, cp)?;
                if !is_int_or_bool(&rt) {
                    return Err(cg_err(
                        path,
                        right.span,
                        "right operand of || must be int/boolean",
                    ));
                }
                code.emit_jump(0x9a, true_label)
                    .map_err(|m| cg_err(path, expr.span, m))?;
                code.emit_u1(0x03);
                code.emit_jump(0xa7, end_label)
                    .map_err(|m| cg_err(path, expr.span, m))?;
                code.bind_label_with_frame(true_label, empty_frame_from_locals(locals))
                    .map_err(|m| cg_err(path, expr.span, m))?;
                code.emit_u1(0x04);
                code.bind_label_with_frame(
                    end_label,
                    frame_with_stack(locals, vec![VerificationType::Integer]),
                )
                .map_err(|m| cg_err(path, expr.span, m))?;
                Ok(EvalType::Bool)
            }
            BinaryOp::Add | BinaryOp::Sub | BinaryOp::Mul | BinaryOp::Div | BinaryOp::Mod => {
                let lt = compile_expr(path, ctx, code, locals, left, cp)?;
                let rt = compile_expr(path, ctx, code, locals, right, cp)?;
                if matches!(lt, EvalType::Int | EvalType::Bool)
                    && matches!(rt, EvalType::Int | EvalType::Bool)
                {
                    let opcode = match op {
                        BinaryOp::Add => 0x60,
                        BinaryOp::Sub => 0x64,
                        BinaryOp::Mul => 0x68,
                        BinaryOp::Div => 0x6c,
                        BinaryOp::Mod => 0x70,
                        _ => unreachable!(),
                    };
                    code.emit_u1(opcode);
                    Ok(EvalType::Int)
                } else {
                    Err(cg_err(
                        path,
                        expr.span,
                        "arithmetic binary op currently supports only int/bool operands",
                    ))
                }
            }
            BinaryOp::Eq | BinaryOp::Ne => {
                let lt = compile_expr(path, ctx, code, locals, left, cp)?;
                let rt = compile_expr(path, ctx, code, locals, right, cp)?;
                let op = match (
                    op,
                    is_ref_like(&lt),
                    is_ref_like(&rt),
                    is_int_or_bool(&lt),
                    is_int_or_bool(&rt),
                ) {
                    (BinaryOp::Eq, true, true, _, _) => 0xa5,
                    (BinaryOp::Ne, true, true, _, _) => 0xa6,
                    (BinaryOp::Eq, _, _, true, true) => 0x9f,
                    (BinaryOp::Ne, _, _, true, true) => 0xa0,
                    _ => {
                        return Err(cg_err(
                            path,
                            expr.span,
                            format!("unsupported equality operand types: {:?} and {:?}", lt, rt),
                        ))
                    }
                };
                emit_bool_result_from_branch(path, code, locals, expr.span, op)?;
                Ok(EvalType::Bool)
            }
            BinaryOp::Lt | BinaryOp::Le | BinaryOp::Gt | BinaryOp::Ge => {
                let lt = compile_expr(path, ctx, code, locals, left, cp)?;
                let rt = compile_expr(path, ctx, code, locals, right, cp)?;
                if !(is_int_or_bool(&lt) && is_int_or_bool(&rt)) {
                    return Err(cg_err(
                        path,
                        expr.span,
                        format!(
                            "comparison requires int/boolean operands, found {:?} and {:?}",
                            lt, rt
                        ),
                    ));
                }
                let op = match op {
                    BinaryOp::Lt => 0xa1,
                    BinaryOp::Le => 0xa4,
                    BinaryOp::Gt => 0xa3,
                    BinaryOp::Ge => 0xa2,
                    _ => unreachable!(),
                };
                emit_bool_result_from_branch(path, code, locals, expr.span, op)?;
                Ok(EvalType::Bool)
            }
        },
        ExprKind::Assign { name, value } => {
            let rhs = compile_expr(path, ctx, code, locals, value, cp)?;
            let Some(local) = locals.get(name).cloned() else {
                return Err(cg_err(
                    path,
                    expr.span,
                    format!("unknown variable '{}'", name),
                ));
            };
            if !is_assignable(&local.ty, &rhs) {
                return Err(cg_err(
                    path,
                    expr.span,
                    format!(
                        "cannot assign value type {:?} to variable '{}' of type {:?}",
                        rhs, name, local.ty
                    ),
                ));
            }
            code.emit_u1(0x59);
            code.emit_store(local.slot, &local.ty)
                .map_err(|m| cg_err(path, expr.span, m))?;
            Ok(local.ty)
        }
        ExprKind::FieldAccess { receiver, field } => {
            if let ExprKind::Var(base) = &receiver.kind {
                if base == "System" && field == "out" {
                    let owner = cp.class("java/lang/System");
                    let nat = cp.name_and_type("out", "Ljava/io/PrintStream;");
                    let field_ref = cp.field_ref(owner, nat);
                    code.emit_u1(0xb2);
                    code.emit_u2(field_ref);
                    return Ok(EvalType::Ref("java/io/PrintStream".to_string()));
                }
            }

            let recv_ty = compile_expr(path, ctx, code, locals, receiver, cp)?;
            let EvalType::Ref(recv_class) = recv_ty else {
                return Err(cg_err(
                    path,
                    expr.span,
                    "field access receiver must be an object reference",
                ));
            };
            let Some(class_members) = find_class_members(ctx.class_members, &recv_class) else {
                return Err(cg_err(
                    path,
                    expr.span,
                    format!("unknown receiver class '{}' for field access", recv_class),
                ));
            };
            let Some(field_ty) = class_members.fields.get(field).cloned() else {
                return Err(cg_err(
                    path,
                    expr.span,
                    format!("no field '{}' in class '{}'", field, recv_class),
                ));
            };
            let desc = type_descriptor(&field_ty).map_err(|m| cg_err(path, expr.span, m))?;
            let owner = cp.class(&normalize_class_name(&recv_class));
            let nat = cp.name_and_type(field, &desc);
            let field_ref = cp.field_ref(owner, nat);
            code.emit_u1(0xb4);
            code.emit_u2(field_ref);
            Ok(eval_type_from_ast(&field_ty))
        }
        ExprKind::Call {
            receiver,
            method,
            args,
        } => {
            if let Some(recv) = receiver {
                if let ExprKind::FieldAccess {
                    receiver: inner,
                    field,
                } = &recv.kind
                {
                    if let ExprKind::Var(base) = &inner.kind {
                        if base == "System" && field == "out" && method == "println" {
                            let recv_ty = compile_expr(path, ctx, code, locals, recv, cp)?;
                            if !matches!(recv_ty, EvalType::Ref(_)) {
                                return Err(cg_err(path, recv.span, "invalid println receiver"));
                            }
                            if args.len() != 1 {
                                return Err(cg_err(
                                    path,
                                    expr.span,
                                    "println currently supports exactly one argument",
                                ));
                            }
                            let arg_ty = compile_expr(path, ctx, code, locals, &args[0], cp)?;
                            let desc = match arg_ty {
                                EvalType::Int | EvalType::Bool => "(I)V".to_string(),
                                EvalType::Ref(ref t) if t == "java/lang/String" => {
                                    "(Ljava/lang/String;)V".to_string()
                                }
                                EvalType::Ref(_) => "(Ljava/lang/Object;)V".to_string(),
                                _ => {
                                    return Err(cg_err(
                                        path,
                                        args[0].span,
                                        "unsupported println argument type",
                                    ))
                                }
                            };
                            let owner = cp.class("java/io/PrintStream");
                            let nat = cp.name_and_type("println", &desc);
                            let mref = cp.method_ref(owner, nat);
                            code.emit_u1(0xb6);
                            code.emit_u2(mref);
                            return Ok(EvalType::Void);
                        }
                    }
                }
                let recv_ty = compile_expr(path, ctx, code, locals, recv, cp)?;
                let (owner_class, has_object_ref) = match recv_ty {
                    EvalType::Ref(c) => (c, true),
                    EvalType::ClassRef(c) => (c, false),
                    _ => {
                        return Err(cg_err(
                            path,
                            recv.span,
                            "method receiver must be an object or class reference",
                        ))
                    }
                };
                let sig = resolve_method_sig_for_class(
                    ctx.class_members,
                    &owner_class,
                    method,
                    args.len(),
                )
                .ok_or_else(|| {
                    cg_err(
                        path,
                        expr.span,
                        format!(
                            "no method '{}.{}' with {} argument(s)",
                            owner_class,
                            method,
                            args.len()
                        ),
                    )
                })?;

                if !sig.is_static && !has_object_ref {
                    return Err(cg_err(
                        path,
                        expr.span,
                        format!(
                            "cannot call instance method '{}.{}' through class reference",
                            owner_class, method
                        ),
                    ));
                }

                for (idx, arg) in args.iter().enumerate() {
                    let at = compile_expr(path, ctx, code, locals, arg, cp)?;
                    let expected = eval_type_from_ast(&sig.params[idx]);
                    if !is_assignable(&expected, &at) {
                        return Err(cg_err(
                            path,
                            arg.span,
                            format!(
                                "argument {} type mismatch: expected {:?}, found {:?}",
                                idx, expected, at
                            ),
                        ));
                    }
                }

                let desc = method_descriptor_from_types(&sig.params, &sig.return_type)
                    .map_err(|m| cg_err(path, expr.span, m))?;
                let owner = cp.class(&normalize_class_name(&owner_class));
                let nat = cp.name_and_type(method, &desc);
                let mref = cp.method_ref(owner, nat);
                if sig.is_static {
                    if has_object_ref {
                        code.emit_u1(0x57);
                    }
                    code.emit_u1(0xb8);
                } else {
                    code.emit_u1(0xb6);
                }
                code.emit_u2(mref);
                return Ok(eval_type_from_ast(&sig.return_type));
            }

            let sig = resolve_method_sig_for_class(
                ctx.class_members,
                &ctx.class.name,
                method,
                args.len(),
            )
            .ok_or_else(|| {
                cg_err(
                    path,
                    expr.span,
                    format!("no method '{}' with {} argument(s)", method, args.len()),
                )
            })?;

            if !sig.is_static {
                let Some(this_local) = locals.get("this").cloned() else {
                    return Err(cg_err(
                        path,
                        expr.span,
                        format!(
                            "cannot call instance method '{}' from static context",
                            method
                        ),
                    ));
                };
                code.emit_load(this_local.slot, &this_local.ty)
                    .map_err(|m| cg_err(path, expr.span, m))?;
            }

            for (idx, arg) in args.iter().enumerate() {
                let at = compile_expr(path, ctx, code, locals, arg, cp)?;
                let expected = eval_type_from_ast(&sig.params[idx]);
                if !is_assignable(&expected, &at) {
                    return Err(cg_err(
                        path,
                        arg.span,
                        format!(
                            "argument {} type mismatch: expected {:?}, found {:?}",
                            idx, expected, at
                        ),
                    ));
                }
            }
            let owner = cp.class(&normalize_class_name(&ctx.class.name));
            let desc = method_descriptor_from_types(&sig.params, &sig.return_type)
                .map_err(|m| cg_err(path, expr.span, m))?;
            let nat = cp.name_and_type(method, &desc);
            let mref = cp.method_ref(owner, nat);
            if sig.is_static {
                code.emit_u1(0xb8);
            } else {
                code.emit_u1(0xb6);
            }
            code.emit_u2(mref);
            Ok(eval_type_from_ast(&sig.return_type))
        }
    }
}

fn emit_default_value(
    code: &mut CodeBuilder,
    ty: &EvalType,
    cp: &mut ConstantPool,
) -> Result<(), String> {
    match ty {
        EvalType::Int | EvalType::Bool => code.emit_push_int(0, cp),
        EvalType::Ref(_) => {
            code.emit_u1(0x01);
            Ok(())
        }
        EvalType::ClassRef(_) | EvalType::Void => Ok(()),
    }
}

fn eval_type_from_ast(ty: &TypeName) -> EvalType {
    match ty {
        TypeName::Int => EvalType::Int,
        TypeName::Boolean => EvalType::Bool,
        TypeName::Void => EvalType::Void,
        TypeName::String => EvalType::Ref("java/lang/String".to_string()),
        TypeName::Class(c) => EvalType::Ref(normalize_reference_name(c)),
        TypeName::Unknown => EvalType::Ref("java/lang/Object".to_string()),
    }
}

fn method_descriptor(method: &MethodDecl) -> Result<String, String> {
    let params = method
        .params
        .iter()
        .map(|p| p.ty.clone())
        .collect::<Vec<_>>();
    method_descriptor_from_types(&params, &method.return_type)
}

fn method_descriptor_from_types(params: &[TypeName], ret: &TypeName) -> Result<String, String> {
    let mut desc = String::from("(");
    for p in params {
        desc.push_str(&type_descriptor(p)?);
    }
    desc.push(')');
    desc.push_str(&type_descriptor(ret)?);
    Ok(desc)
}

fn type_descriptor(ty: &TypeName) -> Result<String, String> {
    match ty {
        TypeName::Int => Ok("I".to_string()),
        TypeName::Boolean => Ok("Z".to_string()),
        TypeName::Void => Ok("V".to_string()),
        TypeName::String => Ok("Ljava/lang/String;".to_string()),
        TypeName::Class(name) => Ok(type_descriptor_from_class_name(name)),
        TypeName::Unknown => Err("unknown type has no JVM descriptor".to_string()),
    }
}

fn normalize_class_name(name: &str) -> String {
    if name == "String" {
        "java/lang/String".to_string()
    } else if name == "Object" {
        "java/lang/Object".to_string()
    } else {
        name.replace('.', "/")
    }
}

fn is_assignable(target: &EvalType, source: &EvalType) -> bool {
    if target == source {
        return true;
    }
    matches!((target, source), (EvalType::Ref(_), EvalType::Ref(s)) if s == "null")
}

fn is_int_or_bool(t: &EvalType) -> bool {
    matches!(t, EvalType::Int | EvalType::Bool)
}

fn is_ref_like(t: &EvalType) -> bool {
    matches!(t, EvalType::Ref(_) | EvalType::ClassRef(_))
}

fn stmt_always_returns(stmt: &Stmt) -> bool {
    match stmt {
        Stmt::Return(_, _) => true,
        Stmt::Block(stmts, _) => stmts.last().map(stmt_always_returns).unwrap_or(false),
        Stmt::If {
            then_branch,
            else_branch: Some(else_branch),
            ..
        } => stmt_always_returns(then_branch) && stmt_always_returns(else_branch),
        _ => false,
    }
}

fn empty_frame_from_locals(locals: &LocalScopes) -> FrameState {
    FrameState {
        locals: locals.active_locals(),
        stack: Vec::new(),
    }
}

fn frame_with_stack(locals: &LocalScopes, stack: Vec<VerificationType>) -> FrameState {
    FrameState {
        locals: locals.active_locals(),
        stack,
    }
}

fn verify_type_from_eval(ty: &EvalType) -> VerificationType {
    match ty {
        EvalType::Int | EvalType::Bool => VerificationType::Integer,
        EvalType::Ref(name) => {
            if name == "null" {
                VerificationType::Null
            } else {
                VerificationType::Object(normalize_reference_name(name))
            }
        }
        EvalType::ClassRef(name) => VerificationType::Object(normalize_reference_name(name)),
        EvalType::Void => VerificationType::Top,
    }
}

fn trim_trailing_top(values: &mut Vec<VerificationType>) {
    while matches!(values.last(), Some(VerificationType::Top)) {
        values.pop();
    }
}

fn normalize_reference_name(name: &str) -> String {
    if let Some(component) = name.strip_suffix("[]") {
        return format!("[{}", type_descriptor_from_class_name(component));
    }
    if name.starts_with('[') {
        return name.to_string();
    }
    normalize_class_name(name)
}

fn type_descriptor_from_class_name(name: &str) -> String {
    if name.starts_with('[') {
        return name.to_string();
    }
    if let Some(component) = name.strip_suffix("[]") {
        return format!("[{}", type_descriptor_from_class_name(component));
    }
    if name == "int" {
        return "I".to_string();
    }
    if name == "boolean" {
        return "Z".to_string();
    }
    format!("L{};", normalize_class_name(name))
}

fn cg_err(path: &Path, span: Span, message: impl Into<String>) -> CodegenError {
    CodegenError {
        path: path.to_path_buf(),
        line: span.line,
        col: span.col,
        message: message.into(),
    }
}

fn emit_frame(
    out: &mut Vec<u8>,
    offset_delta: u16,
    previous: &FrameState,
    current: &FrameState,
    cp: &mut ConstantPool,
) -> Result<(), String> {
    if current.stack.is_empty() && current.locals == previous.locals && offset_delta <= 63 {
        out.push(offset_delta as u8);
        return Ok(());
    }
    if current.locals == previous.locals && current.stack.len() == 1 && offset_delta <= 63 {
        out.push(64 + offset_delta as u8);
        emit_verification_type(out, &current.stack[0], cp)?;
        return Ok(());
    }
    out.push(255);
    push_u2(out, offset_delta);
    push_u2(
        out,
        u16::try_from(current.locals.len())
            .map_err(|_| "too many locals in stack map frame".to_string())?,
    );
    for local in &current.locals {
        emit_verification_type(out, local, cp)?;
    }
    push_u2(
        out,
        u16::try_from(current.stack.len())
            .map_err(|_| "too many stack entries in stack map frame".to_string())?,
    );
    for stack_ty in &current.stack {
        emit_verification_type(out, stack_ty, cp)?;
    }
    Ok(())
}

fn emit_verification_type(
    out: &mut Vec<u8>,
    ty: &VerificationType,
    cp: &mut ConstantPool,
) -> Result<(), String> {
    match ty {
        VerificationType::Top => out.push(0),
        VerificationType::Integer => out.push(1),
        VerificationType::Null => out.push(5),
        VerificationType::Object(name) => {
            out.push(7);
            push_u2(out, cp.class(name));
        }
    }
    Ok(())
}

fn push_u2(out: &mut Vec<u8>, v: u16) {
    out.push((v >> 8) as u8);
    out.push((v & 0xff) as u8);
}

fn push_u4(out: &mut Vec<u8>, v: u32) {
    out.push((v >> 24) as u8);
    out.push(((v >> 16) & 0xff) as u8);
    out.push(((v >> 8) & 0xff) as u8);
    out.push((v & 0xff) as u8);
}

#[derive(Clone)]
enum CpEntry {
    Utf8(String),
    Integer(i32),
    Class(u16),
    String(u16),
    FieldRef(u16, u16),
    MethodRef(u16, u16),
    NameAndType(u16, u16),
}

struct ConstantPool {
    entries: Vec<CpEntry>,
    utf8_map: HashMap<String, u16>,
    int_map: HashMap<i32, u16>,
    class_map: HashMap<String, u16>,
    string_map: HashMap<String, u16>,
    nat_map: HashMap<(String, String), u16>,
    field_ref_map: HashMap<(u16, u16), u16>,
    method_ref_map: HashMap<(u16, u16), u16>,
}

impl ConstantPool {
    fn new() -> Self {
        Self {
            entries: Vec::new(),
            utf8_map: HashMap::new(),
            int_map: HashMap::new(),
            class_map: HashMap::new(),
            string_map: HashMap::new(),
            nat_map: HashMap::new(),
            field_ref_map: HashMap::new(),
            method_ref_map: HashMap::new(),
        }
    }

    fn count_with_implicit_zero(&self) -> usize {
        self.entries.len() + 1
    }

    fn utf8(&mut self, s: &str) -> u16 {
        if let Some(idx) = self.utf8_map.get(s) {
            return *idx;
        }
        let idx = self.push(CpEntry::Utf8(s.to_string()));
        self.utf8_map.insert(s.to_string(), idx);
        idx
    }

    fn integer(&mut self, i: i32) -> u16 {
        if let Some(idx) = self.int_map.get(&i) {
            return *idx;
        }
        let idx = self.push(CpEntry::Integer(i));
        self.int_map.insert(i, idx);
        idx
    }

    fn class(&mut self, name: &str) -> u16 {
        if let Some(idx) = self.class_map.get(name) {
            return *idx;
        }
        let utf = self.utf8(name);
        let idx = self.push(CpEntry::Class(utf));
        self.class_map.insert(name.to_string(), idx);
        idx
    }

    fn string(&mut self, value: &str) -> u16 {
        if let Some(idx) = self.string_map.get(value) {
            return *idx;
        }
        let utf = self.utf8(value);
        let idx = self.push(CpEntry::String(utf));
        self.string_map.insert(value.to_string(), idx);
        idx
    }

    fn name_and_type(&mut self, name: &str, desc: &str) -> u16 {
        let key = (name.to_string(), desc.to_string());
        if let Some(idx) = self.nat_map.get(&key) {
            return *idx;
        }
        let name_idx = self.utf8(name);
        let desc_idx = self.utf8(desc);
        let idx = self.push(CpEntry::NameAndType(name_idx, desc_idx));
        self.nat_map.insert(key, idx);
        idx
    }

    fn field_ref(&mut self, class_idx: u16, nat_idx: u16) -> u16 {
        let key = (class_idx, nat_idx);
        if let Some(idx) = self.field_ref_map.get(&key) {
            return *idx;
        }
        let idx = self.push(CpEntry::FieldRef(class_idx, nat_idx));
        self.field_ref_map.insert(key, idx);
        idx
    }

    fn method_ref(&mut self, class_idx: u16, nat_idx: u16) -> u16 {
        let key = (class_idx, nat_idx);
        if let Some(idx) = self.method_ref_map.get(&key) {
            return *idx;
        }
        let idx = self.push(CpEntry::MethodRef(class_idx, nat_idx));
        self.method_ref_map.insert(key, idx);
        idx
    }

    fn push(&mut self, entry: CpEntry) -> u16 {
        self.entries.push(entry);
        u16::try_from(self.entries.len()).unwrap_or(u16::MAX)
    }

    fn to_bytes(&self) -> Vec<u8> {
        let mut out = Vec::new();
        for e in &self.entries {
            match e {
                CpEntry::Utf8(s) => {
                    out.push(1);
                    push_u2(&mut out, u16::try_from(s.len()).unwrap_or(0));
                    out.extend(s.as_bytes());
                }
                CpEntry::Integer(v) => {
                    out.push(3);
                    push_u4(&mut out, *v as u32);
                }
                CpEntry::Class(name_idx) => {
                    out.push(7);
                    push_u2(&mut out, *name_idx);
                }
                CpEntry::String(utf_idx) => {
                    out.push(8);
                    push_u2(&mut out, *utf_idx);
                }
                CpEntry::FieldRef(c, nt) => {
                    out.push(9);
                    push_u2(&mut out, *c);
                    push_u2(&mut out, *nt);
                }
                CpEntry::MethodRef(c, nt) => {
                    out.push(10);
                    push_u2(&mut out, *c);
                    push_u2(&mut out, *nt);
                }
                CpEntry::NameAndType(n, d) => {
                    out.push(12);
                    push_u2(&mut out, *n);
                    push_u2(&mut out, *d);
                }
            }
        }
        out
    }
}
