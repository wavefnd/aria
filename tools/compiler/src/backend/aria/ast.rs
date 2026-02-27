#![allow(dead_code)]

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    pub line: usize,
    pub col: usize,
}

#[derive(Debug, Clone)]
pub struct CompilationUnit {
    pub classes: Vec<ClassDecl>,
}

#[derive(Debug, Clone)]
pub struct ClassDecl {
    pub name: String,
    pub is_public: bool,
    pub members: Vec<MemberDecl>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum MemberDecl {
    Field(FieldDecl),
    Method(MethodDecl),
}

#[derive(Debug, Clone)]
pub struct FieldDecl {
    pub name: String,
    pub ty: TypeName,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct MethodDecl {
    pub name: String,
    pub return_type: TypeName,
    pub params: Vec<ParamDecl>,
    pub body: Stmt,
    pub is_static: bool,
    pub is_public: bool,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct ParamDecl {
    pub name: String,
    pub ty: TypeName,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TypeName {
    Int,
    Boolean,
    Void,
    String,
    Class(String),
    Unknown,
}

impl TypeName {
    pub fn display(&self) -> String {
        match self {
            TypeName::Int => "int".to_string(),
            TypeName::Boolean => "boolean".to_string(),
            TypeName::Void => "void".to_string(),
            TypeName::String => "String".to_string(),
            TypeName::Class(c) => c.clone(),
            TypeName::Unknown => "<unknown>".to_string(),
        }
    }

    pub fn is_reference(&self) -> bool {
        matches!(self, TypeName::String | TypeName::Class(_))
    }
}

#[derive(Debug, Clone)]
pub struct Expr {
    pub kind: ExprKind,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum ExprKind {
    IntLiteral(i64),
    BoolLiteral(bool),
    StringLiteral(String),
    Null,
    This,
    Var(String),
    New {
        class_name: String,
        args: Vec<Expr>,
    },
    Unary {
        op: UnaryOp,
        expr: Box<Expr>,
    },
    Binary {
        op: BinaryOp,
        left: Box<Expr>,
        right: Box<Expr>,
    },
    Assign {
        name: String,
        value: Box<Expr>,
    },
    Call {
        receiver: Option<Box<Expr>>,
        method: String,
        args: Vec<Expr>,
    },
    FieldAccess {
        receiver: Box<Expr>,
        field: String,
    },
}

#[derive(Debug, Clone, Copy)]
pub enum UnaryOp {
    Neg,
    Not,
}

#[derive(Debug, Clone, Copy)]
pub enum BinaryOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    And,
    Or,
}

#[derive(Debug, Clone)]
pub enum Stmt {
    Block(Vec<Stmt>, Span),
    If {
        cond: Expr,
        then_branch: Box<Stmt>,
        else_branch: Option<Box<Stmt>>,
        span: Span,
    },
    While {
        cond: Expr,
        body: Box<Stmt>,
        span: Span,
    },
    Return(Option<Expr>, Span),
    LocalVar {
        ty: TypeName,
        name: String,
        init: Option<Expr>,
        span: Span,
    },
    Expr(Expr, Span),
    Empty(Span),
}
