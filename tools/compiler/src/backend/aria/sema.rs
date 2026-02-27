use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::backend::aria::ast::*;

#[derive(Debug, Clone)]
pub struct SourceFileAst {
    pub path: PathBuf,
    pub unit: CompilationUnit,
}

#[derive(Debug, Clone)]
pub struct Diagnostic {
    pub path: PathBuf,
    pub line: usize,
    pub col: usize,
    pub message: String,
}

#[derive(Debug, Clone)]
struct MethodSig {
    params: Vec<TypeName>,
    return_type: TypeName,
}

#[derive(Debug, Clone)]
struct ClassInfo {
    fields: HashMap<String, TypeName>,
    methods: HashMap<String, Vec<MethodSig>>,
}

pub fn analyze(files: &[SourceFileAst]) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    let class_table = collect_classes(files, &mut diagnostics);

    for file in files {
        for class in &file.unit.classes {
            if class.is_public {
                let stem = file
                    .path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or_default();
                if class.name != stem {
                    diagnostics.push(Diagnostic {
                        path: file.path.clone(),
                        line: class.span.line,
                        col: class.span.col,
                        message: format!(
                            "public class '{}' must be declared in a file named '{}.java'",
                            class.name, class.name
                        ),
                    });
                }
            }
            check_class(file.path.as_path(), class, &class_table, &mut diagnostics);
        }
    }

    diagnostics
}

fn collect_classes(
    files: &[SourceFileAst],
    diagnostics: &mut Vec<Diagnostic>,
) -> HashMap<String, ClassInfo> {
    let mut table = HashMap::new();
    for file in files {
        for class in &file.unit.classes {
            if table.contains_key(&class.name) {
                diagnostics.push(Diagnostic {
                    path: file.path.clone(),
                    line: class.span.line,
                    col: class.span.col,
                    message: format!("duplicate class '{}'", class.name),
                });
                continue;
            }

            let mut fields = HashMap::new();
            let mut methods: HashMap<String, Vec<MethodSig>> = HashMap::new();
            for member in &class.members {
                match member {
                    MemberDecl::Field(field) => {
                        if fields.contains_key(&field.name) {
                            diagnostics.push(Diagnostic {
                                path: file.path.clone(),
                                line: field.span.line,
                                col: field.span.col,
                                message: format!("duplicate field '{}'", field.name),
                            });
                        } else {
                            fields.insert(field.name.clone(), field.ty.clone());
                        }
                    }
                    MemberDecl::Method(method) => {
                        let sig = MethodSig {
                            params: method.params.iter().map(|p| p.ty.clone()).collect(),
                            return_type: method.return_type.clone(),
                        };
                        methods.entry(method.name.clone()).or_default().push(sig);
                    }
                }
            }
            table.insert(class.name.clone(), ClassInfo { fields, methods });
        }
    }
    table
}

fn check_class(
    path: &Path,
    class: &ClassDecl,
    class_table: &HashMap<String, ClassInfo>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let class_info = match class_table.get(&class.name) {
        Some(c) => c,
        None => return,
    };

    for member in &class.members {
        if let MemberDecl::Method(method) = member {
            check_method(path, class, class_info, method, class_table, diagnostics);
        }
    }
}

fn check_method(
    path: &Path,
    class: &ClassDecl,
    class_info: &ClassInfo,
    method: &MethodDecl,
    class_table: &HashMap<String, ClassInfo>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let mut locals: HashMap<String, TypeName> = HashMap::new();
    for param in &method.params {
        if locals.contains_key(&param.name) {
            diagnostics.push(Diagnostic {
                path: path.to_path_buf(),
                line: param.span.line,
                col: param.span.col,
                message: format!("duplicate parameter '{}'", param.name),
            });
        } else {
            locals.insert(param.name.clone(), param.ty.clone());
        }
    }

    let mut ctx = CheckCtx {
        path,
        class,
        class_info,
        method,
        class_table,
        locals: &mut locals,
        diagnostics,
    };
    check_stmt(&method.body, &mut ctx);
}

struct CheckCtx<'a> {
    path: &'a Path,
    class: &'a ClassDecl,
    class_info: &'a ClassInfo,
    method: &'a MethodDecl,
    class_table: &'a HashMap<String, ClassInfo>,
    locals: &'a mut HashMap<String, TypeName>,
    diagnostics: &'a mut Vec<Diagnostic>,
}

fn check_stmt(stmt: &Stmt, ctx: &mut CheckCtx<'_>) {
    match stmt {
        Stmt::Block(stmts, _) => {
            let snapshot = ctx.locals.clone();
            for s in stmts {
                check_stmt(s, ctx);
            }
            *ctx.locals = snapshot;
        }
        Stmt::If {
            cond,
            then_branch,
            else_branch,
            ..
        } => {
            let cond_ty = infer_expr(cond, ctx);
            if cond_ty != TypeName::Boolean && cond_ty != TypeName::Unknown {
                add_diag(
                    ctx,
                    cond.span,
                    format!("if condition must be boolean, found {}", cond_ty.display()),
                );
            }
            check_stmt(then_branch, ctx);
            if let Some(e) = else_branch {
                check_stmt(e, ctx);
            }
        }
        Stmt::While { cond, body, .. } => {
            let cond_ty = infer_expr(cond, ctx);
            if cond_ty != TypeName::Boolean && cond_ty != TypeName::Unknown {
                add_diag(
                    ctx,
                    cond.span,
                    format!(
                        "while condition must be boolean, found {}",
                        cond_ty.display()
                    ),
                );
            }
            check_stmt(body, ctx);
        }
        Stmt::Return(expr, span) => match expr {
            Some(e) => {
                let expr_ty = infer_expr(e, ctx);
                if !is_assignable(&ctx.method.return_type, &expr_ty) {
                    add_diag(
                        ctx,
                        *span,
                        format!(
                            "return type mismatch: expected {}, found {}",
                            ctx.method.return_type.display(),
                            expr_ty.display()
                        ),
                    );
                }
            }
            None => {
                if ctx.method.return_type != TypeName::Void {
                    add_diag(
                        ctx,
                        *span,
                        format!(
                            "return statement requires a value of type {}",
                            ctx.method.return_type.display()
                        ),
                    );
                }
            }
        },
        Stmt::LocalVar {
            ty,
            name,
            init,
            span,
        } => {
            if ctx.locals.contains_key(name) {
                add_diag(ctx, *span, format!("duplicate local variable '{}'", name));
            } else {
                if let Some(init_expr) = init {
                    let init_ty = infer_expr(init_expr, ctx);
                    if !is_assignable(ty, &init_ty) {
                        add_diag(
                            ctx,
                            init_expr.span,
                            format!(
                                "cannot assign {} to variable '{}' of type {}",
                                init_ty.display(),
                                name,
                                ty.display()
                            ),
                        );
                    }
                }
                ctx.locals.insert(name.clone(), ty.clone());
            }
        }
        Stmt::Expr(expr, _) => {
            infer_expr(expr, ctx);
        }
        Stmt::Empty(_) => {}
    }
}

fn infer_expr(expr: &Expr, ctx: &mut CheckCtx<'_>) -> TypeName {
    match &expr.kind {
        ExprKind::IntLiteral(_) => TypeName::Int,
        ExprKind::BoolLiteral(_) => TypeName::Boolean,
        ExprKind::StringLiteral(_) => TypeName::String,
        ExprKind::Null => TypeName::Class("null".to_string()),
        ExprKind::This => TypeName::Class(ctx.class.name.clone()),
        ExprKind::Var(name) => {
            if let Some(t) = ctx.locals.get(name) {
                return t.clone();
            }
            if let Some(t) = ctx.class_info.fields.get(name) {
                if ctx.method.is_static {
                    add_diag(
                        ctx,
                        expr.span,
                        format!(
                            "cannot reference instance field '{}' from static context",
                            name
                        ),
                    );
                }
                return t.clone();
            }
            if ctx.class_table.contains_key(name) || name == "System" {
                return TypeName::Class(name.clone());
            }
            add_diag(ctx, expr.span, format!("undefined symbol '{}'", name));
            TypeName::Unknown
        }
        ExprKind::New { class_name, args } => {
            for arg in args {
                let _ = infer_expr(arg, ctx);
            }
            if ctx.class_table.contains_key(class_name) {
                TypeName::Class(class_name.clone())
            } else {
                add_diag(
                    ctx,
                    expr.span,
                    format!("cannot resolve class '{}' for object creation", class_name),
                );
                TypeName::Unknown
            }
        }
        ExprKind::Unary { op, expr: inner } => {
            let inner_ty = infer_expr(inner, ctx);
            match op {
                UnaryOp::Neg => {
                    if inner_ty != TypeName::Int && inner_ty != TypeName::Unknown {
                        add_diag(
                            ctx,
                            inner.span,
                            format!("unary '-' requires int, found {}", inner_ty.display()),
                        );
                        TypeName::Unknown
                    } else {
                        TypeName::Int
                    }
                }
                UnaryOp::Not => {
                    if inner_ty != TypeName::Boolean && inner_ty != TypeName::Unknown {
                        add_diag(
                            ctx,
                            inner.span,
                            format!("unary '!' requires boolean, found {}", inner_ty.display()),
                        );
                        TypeName::Unknown
                    } else {
                        TypeName::Boolean
                    }
                }
            }
        }
        ExprKind::Binary { op, left, right } => {
            let lt = infer_expr(left, ctx);
            let rt = infer_expr(right, ctx);
            check_binary_type(*op, &lt, &rt, expr.span, ctx)
        }
        ExprKind::Assign { name, value } => {
            let rhs_ty = infer_expr(value, ctx);
            let lhs_ty = if let Some(t) = ctx.locals.get(name) {
                t.clone()
            } else if let Some(t) = ctx.class_info.fields.get(name) {
                t.clone()
            } else {
                add_diag(ctx, expr.span, format!("undefined symbol '{}'", name));
                TypeName::Unknown
            };
            if !is_assignable(&lhs_ty, &rhs_ty) {
                add_diag(
                    ctx,
                    expr.span,
                    format!(
                        "cannot assign {} to '{}', expected {}",
                        rhs_ty.display(),
                        name,
                        lhs_ty.display()
                    ),
                );
            }
            lhs_ty
        }
        ExprKind::Call {
            receiver,
            method,
            args,
        } => {
            let arg_types: Vec<TypeName> = args.iter().map(|a| infer_expr(a, ctx)).collect();

            match receiver {
                None => resolve_call_in_class(method, &arg_types, ctx).unwrap_or(TypeName::Unknown),
                Some(recv_expr) => {
                    let recv_ty = infer_expr(recv_expr, ctx);
                    resolve_call_with_receiver(&recv_ty, method, &arg_types, ctx)
                        .unwrap_or(TypeName::Unknown)
                }
            }
        }
        ExprKind::FieldAccess { receiver, field } => {
            let recv_ty = infer_expr(receiver, ctx);
            resolve_field_type(&recv_ty, field, ctx).unwrap_or(TypeName::Unknown)
        }
    }
}

fn resolve_field_type(recv_ty: &TypeName, field: &str, ctx: &mut CheckCtx<'_>) -> Option<TypeName> {
    if let TypeName::Class(class_name) = recv_ty {
        if class_name == "System" && field == "out" {
            return Some(TypeName::Class("PrintStream".to_string()));
        }
        if let Some(class) = ctx.class_table.get(class_name) {
            if let Some(ty) = class.fields.get(field) {
                return Some(ty.clone());
            }
        }
    }
    None
}

fn resolve_call_in_class(
    method_name: &str,
    arg_types: &[TypeName],
    ctx: &mut CheckCtx<'_>,
) -> Option<TypeName> {
    if let Some(overloads) = ctx.class_info.methods.get(method_name) {
        if let Some(sig) = overloads.iter().find(|m| m.params.len() == arg_types.len()) {
            return Some(sig.return_type.clone());
        }
    }
    add_diag(
        ctx,
        ctx.method.span,
        format!(
            "no matching method '{}' with {} argument(s) in class '{}'",
            method_name,
            arg_types.len(),
            ctx.class.name
        ),
    );
    None
}

fn resolve_call_with_receiver(
    recv_ty: &TypeName,
    method_name: &str,
    arg_types: &[TypeName],
    ctx: &mut CheckCtx<'_>,
) -> Option<TypeName> {
    if let TypeName::Class(class_name) = recv_ty {
        if class_name == "PrintStream" && method_name == "println" {
            return Some(TypeName::Void);
        }

        if let Some(class) = ctx.class_table.get(class_name) {
            if let Some(overloads) = class.methods.get(method_name) {
                if let Some(sig) = overloads.iter().find(|m| m.params.len() == arg_types.len()) {
                    return Some(sig.return_type.clone());
                }
            }
            add_diag(
                ctx,
                ctx.method.span,
                format!(
                    "no matching method '{}.{}' with {} argument(s)",
                    class_name,
                    method_name,
                    arg_types.len()
                ),
            );
            return None;
        }
    }
    None
}

fn check_binary_type(
    op: BinaryOp,
    left: &TypeName,
    right: &TypeName,
    span: Span,
    ctx: &mut CheckCtx<'_>,
) -> TypeName {
    match op {
        BinaryOp::Add | BinaryOp::Sub | BinaryOp::Mul | BinaryOp::Div | BinaryOp::Mod => {
            if left == &TypeName::Int && right == &TypeName::Int {
                TypeName::Int
            } else if matches!(op, BinaryOp::Add)
                && (left == &TypeName::String || right == &TypeName::String)
            {
                TypeName::String
            } else {
                add_diag(
                    ctx,
                    span,
                    format!(
                        "arithmetic operator requires int operands, found {} and {}",
                        left.display(),
                        right.display()
                    ),
                );
                TypeName::Unknown
            }
        }
        BinaryOp::Lt | BinaryOp::Le | BinaryOp::Gt | BinaryOp::Ge => {
            if left == &TypeName::Int && right == &TypeName::Int {
                TypeName::Boolean
            } else {
                add_diag(
                    ctx,
                    span,
                    format!(
                        "comparison operator requires int operands, found {} and {}",
                        left.display(),
                        right.display()
                    ),
                );
                TypeName::Unknown
            }
        }
        BinaryOp::Eq | BinaryOp::Ne => TypeName::Boolean,
        BinaryOp::And | BinaryOp::Or => {
            if left == &TypeName::Boolean && right == &TypeName::Boolean {
                TypeName::Boolean
            } else {
                add_diag(
                    ctx,
                    span,
                    format!(
                        "logical operator requires boolean operands, found {} and {}",
                        left.display(),
                        right.display()
                    ),
                );
                TypeName::Unknown
            }
        }
    }
}

fn is_assignable(target: &TypeName, source: &TypeName) -> bool {
    if target == source {
        return true;
    }
    if target.is_reference() && source == &TypeName::Class("null".to_string()) {
        return true;
    }
    if target == &TypeName::Unknown || source == &TypeName::Unknown {
        return true;
    }
    false
}

fn add_diag(ctx: &mut CheckCtx<'_>, span: Span, message: String) {
    ctx.diagnostics.push(Diagnostic {
        path: ctx.path.to_path_buf(),
        line: span.line,
        col: span.col,
        message,
    });
}
