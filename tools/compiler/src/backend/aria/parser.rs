use crate::backend::aria::ast::*;
use crate::backend::aria::lexer::{Token, TokenKind};

#[derive(Debug, Clone)]
pub struct ParseError {
    pub span: Span,
    pub message: String,
}

pub fn parse(tokens: Vec<Token>) -> Result<CompilationUnit, ParseError> {
    let mut parser = Parser { tokens, idx: 0 };
    parser.parse_compilation_unit()
}

struct Parser {
    tokens: Vec<Token>,
    idx: usize,
}

impl Parser {
    fn parse_compilation_unit(&mut self) -> Result<CompilationUnit, ParseError> {
        let mut classes = Vec::new();
        while !self.at_eof() {
            classes.push(self.parse_class_decl()?);
        }
        Ok(CompilationUnit { classes })
    }

    fn parse_class_decl(&mut self) -> Result<ClassDecl, ParseError> {
        let is_public = self.consume_if(|k| matches!(k, TokenKind::Public));
        self.expect(|k| matches!(k, TokenKind::Class), "Expected 'class'.")?;
        let (name, span) = self.expect_ident()?;
        self.expect(
            |k| matches!(k, TokenKind::LBrace),
            "Expected '{' after class name.",
        )?;

        let mut members = Vec::new();
        while !self.at(|k| matches!(k, TokenKind::RBrace | TokenKind::Eof)) {
            members.push(self.parse_member_decl()?);
        }

        self.expect(
            |k| matches!(k, TokenKind::RBrace),
            "Expected '}' at end of class.",
        )?;

        Ok(ClassDecl {
            name,
            is_public,
            members,
            span,
        })
    }

    fn parse_member_decl(&mut self) -> Result<MemberDecl, ParseError> {
        let mut is_public = false;
        let mut is_static = false;
        loop {
            if self.consume_if(|k| matches!(k, TokenKind::Public)) {
                is_public = true;
                continue;
            }
            if self.consume_if(|k| matches!(k, TokenKind::Static)) {
                is_static = true;
                continue;
            }
            break;
        }

        let ty = self.parse_type(true)?;
        let (name, span) = self.expect_ident()?;

        if self.consume_if(|k| matches!(k, TokenKind::LParen)) {
            let params = self.parse_params()?;
            self.expect(
                |k| matches!(k, TokenKind::RParen),
                "Expected ')' after parameter list.",
            )?;
            let body = self.parse_block_stmt()?;
            return Ok(MemberDecl::Method(MethodDecl {
                name,
                return_type: ty,
                params,
                body,
                is_public,
                is_static,
                span,
            }));
        }

        self.expect(
            |k| matches!(k, TokenKind::Semicolon),
            "Expected ';' after field declaration.",
        )?;
        Ok(MemberDecl::Field(FieldDecl { name, ty, span }))
    }

    fn parse_params(&mut self) -> Result<Vec<ParamDecl>, ParseError> {
        let mut params = Vec::new();
        if self.at(|k| matches!(k, TokenKind::RParen)) {
            return Ok(params);
        }

        loop {
            let ty = self.parse_type(false)?;
            let (name, span) = self.expect_ident()?;
            params.push(ParamDecl { name, ty, span });

            if self.consume_if(|k| matches!(k, TokenKind::Comma)) {
                continue;
            }
            break;
        }
        Ok(params)
    }

    fn parse_type(&mut self, allow_void: bool) -> Result<TypeName, ParseError> {
        let base = if self.consume_if(|k| matches!(k, TokenKind::Int)) {
            TypeName::Int
        } else if self.consume_if(|k| matches!(k, TokenKind::Boolean)) {
            TypeName::Boolean
        } else if self.consume_if(|k| matches!(k, TokenKind::StringKw)) {
            TypeName::String
        } else if self.consume_if(|k| matches!(k, TokenKind::Void)) {
            if allow_void {
                TypeName::Void
            } else {
                return Err(self.error_here("void is not allowed in this type position."));
            }
        } else if let Some(TokenKind::Identifier(name)) = self.peek_kind().cloned() {
            self.bump();
            TypeName::Class(name)
        } else {
            return Err(self.error_here("Expected a type."));
        };

        let mut array_depth = 0usize;
        while self.consume_if(|k| matches!(k, TokenKind::LBracket)) {
            self.expect(
                |k| matches!(k, TokenKind::RBracket),
                "Expected ']' after '[' in array type.",
            )?;
            array_depth += 1;
        }

        if array_depth == 0 {
            return Ok(base);
        }

        let suffix = "[]".repeat(array_depth);
        let rendered = format!("{}{}", base.display(), suffix);
        Ok(TypeName::Class(rendered))
    }

    fn parse_block_stmt(&mut self) -> Result<Stmt, ParseError> {
        let start = self.expect(
            |k| matches!(k, TokenKind::LBrace),
            "Expected '{' to start block.",
        )?;
        let mut stmts = Vec::new();
        while !self.at(|k| matches!(k, TokenKind::RBrace | TokenKind::Eof)) {
            stmts.push(self.parse_stmt()?);
        }
        self.expect(
            |k| matches!(k, TokenKind::RBrace),
            "Expected '}' to close block.",
        )?;
        Ok(Stmt::Block(stmts, start))
    }

    fn parse_stmt(&mut self) -> Result<Stmt, ParseError> {
        if self.at(|k| matches!(k, TokenKind::LBrace)) {
            return self.parse_block_stmt();
        }
        if self.consume_if(|k| matches!(k, TokenKind::If)) {
            let span = self.current_span();
            self.expect(|k| matches!(k, TokenKind::LParen), "Expected '(' after if.")?;
            let cond = self.parse_expr()?;
            self.expect(
                |k| matches!(k, TokenKind::RParen),
                "Expected ')' after if condition.",
            )?;
            let then_branch = Box::new(self.parse_stmt()?);
            let else_branch = if self.consume_if(|k| matches!(k, TokenKind::Else)) {
                Some(Box::new(self.parse_stmt()?))
            } else {
                None
            };
            return Ok(Stmt::If {
                cond,
                then_branch,
                else_branch,
                span,
            });
        }
        if self.consume_if(|k| matches!(k, TokenKind::While)) {
            let span = self.current_span();
            self.expect(
                |k| matches!(k, TokenKind::LParen),
                "Expected '(' after while.",
            )?;
            let cond = self.parse_expr()?;
            self.expect(
                |k| matches!(k, TokenKind::RParen),
                "Expected ')' after while condition.",
            )?;
            let body = Box::new(self.parse_stmt()?);
            return Ok(Stmt::While { cond, body, span });
        }
        if self.consume_if(|k| matches!(k, TokenKind::Return)) {
            let span = self.current_span();
            let expr = if self.at(|k| matches!(k, TokenKind::Semicolon)) {
                None
            } else {
                Some(self.parse_expr()?)
            };
            self.expect(
                |k| matches!(k, TokenKind::Semicolon),
                "Expected ';' after return statement.",
            )?;
            return Ok(Stmt::Return(expr, span));
        }
        if self.consume_if(|k| matches!(k, TokenKind::Semicolon)) {
            return Ok(Stmt::Empty(self.current_span()));
        }

        if self.looks_like_local_var_decl() {
            let span = self.current_span();
            let ty = self.parse_type(false)?;
            let (name, _) = self.expect_ident()?;
            let init = if self.consume_if(|k| matches!(k, TokenKind::Assign)) {
                Some(self.parse_expr()?)
            } else {
                None
            };
            self.expect(
                |k| matches!(k, TokenKind::Semicolon),
                "Expected ';' after local variable declaration.",
            )?;
            return Ok(Stmt::LocalVar {
                ty,
                name,
                init,
                span,
            });
        }

        let span = self.current_span();
        let expr = self.parse_expr()?;
        self.expect(
            |k| matches!(k, TokenKind::Semicolon),
            "Expected ';' after expression statement.",
        )?;
        Ok(Stmt::Expr(expr, span))
    }

    fn looks_like_local_var_decl(&self) -> bool {
        if self.at(|k| matches!(k, TokenKind::Int | TokenKind::Boolean | TokenKind::StringKw)) {
            return true;
        }

        if matches!(self.peek_kind(), Some(TokenKind::Identifier(_)))
            && matches!(self.peek_n_kind(1), Some(TokenKind::Identifier(_)))
        {
            return true;
        }

        if matches!(self.peek_kind(), Some(TokenKind::Identifier(_)))
            && matches!(self.peek_n_kind(1), Some(TokenKind::LBracket))
        {
            return true;
        }

        false
    }

    fn parse_expr(&mut self) -> Result<Expr, ParseError> {
        self.parse_assignment()
    }

    fn parse_assignment(&mut self) -> Result<Expr, ParseError> {
        let left = self.parse_or()?;
        if self.consume_if(|k| matches!(k, TokenKind::Assign)) {
            let rhs = self.parse_assignment()?;
            let span = left.span;
            match left.kind {
                ExprKind::Var(name) => {
                    return Ok(Expr {
                        kind: ExprKind::Assign {
                            name,
                            value: Box::new(rhs),
                        },
                        span,
                    });
                }
                _ => {
                    return Err(ParseError {
                        span,
                        message: "Left side of assignment must be a variable.".to_string(),
                    });
                }
            }
        }
        Ok(left)
    }

    fn parse_or(&mut self) -> Result<Expr, ParseError> {
        self.parse_binary_chain(
            |p| p.parse_and(),
            |k| matches!(k, TokenKind::OrOr),
            BinaryOp::Or,
        )
    }

    fn parse_and(&mut self) -> Result<Expr, ParseError> {
        self.parse_binary_chain(
            |p| p.parse_equality(),
            |k| matches!(k, TokenKind::AndAnd),
            BinaryOp::And,
        )
    }

    fn parse_equality(&mut self) -> Result<Expr, ParseError> {
        let mut expr = self.parse_relational()?;
        loop {
            let op = if self.consume_if(|k| matches!(k, TokenKind::EqEq)) {
                Some(BinaryOp::Eq)
            } else if self.consume_if(|k| matches!(k, TokenKind::NotEq)) {
                Some(BinaryOp::Ne)
            } else {
                None
            };
            let Some(op) = op else { break };
            let right = self.parse_relational()?;
            let span = expr.span;
            expr = Expr {
                kind: ExprKind::Binary {
                    op,
                    left: Box::new(expr),
                    right: Box::new(right),
                },
                span,
            };
        }
        Ok(expr)
    }

    fn parse_relational(&mut self) -> Result<Expr, ParseError> {
        let mut expr = self.parse_additive()?;
        loop {
            let op = if self.consume_if(|k| matches!(k, TokenKind::Lt)) {
                Some(BinaryOp::Lt)
            } else if self.consume_if(|k| matches!(k, TokenKind::Le)) {
                Some(BinaryOp::Le)
            } else if self.consume_if(|k| matches!(k, TokenKind::Gt)) {
                Some(BinaryOp::Gt)
            } else if self.consume_if(|k| matches!(k, TokenKind::Ge)) {
                Some(BinaryOp::Ge)
            } else {
                None
            };
            let Some(op) = op else { break };
            let right = self.parse_additive()?;
            let span = expr.span;
            expr = Expr {
                kind: ExprKind::Binary {
                    op,
                    left: Box::new(expr),
                    right: Box::new(right),
                },
                span,
            };
        }
        Ok(expr)
    }

    fn parse_additive(&mut self) -> Result<Expr, ParseError> {
        let mut expr = self.parse_multiplicative()?;
        loop {
            let op = if self.consume_if(|k| matches!(k, TokenKind::Plus)) {
                Some(BinaryOp::Add)
            } else if self.consume_if(|k| matches!(k, TokenKind::Minus)) {
                Some(BinaryOp::Sub)
            } else {
                None
            };
            let Some(op) = op else { break };
            let right = self.parse_multiplicative()?;
            let span = expr.span;
            expr = Expr {
                kind: ExprKind::Binary {
                    op,
                    left: Box::new(expr),
                    right: Box::new(right),
                },
                span,
            };
        }
        Ok(expr)
    }

    fn parse_multiplicative(&mut self) -> Result<Expr, ParseError> {
        let mut expr = self.parse_unary()?;
        loop {
            let op = if self.consume_if(|k| matches!(k, TokenKind::Star)) {
                Some(BinaryOp::Mul)
            } else if self.consume_if(|k| matches!(k, TokenKind::Slash)) {
                Some(BinaryOp::Div)
            } else if self.consume_if(|k| matches!(k, TokenKind::Percent)) {
                Some(BinaryOp::Mod)
            } else {
                None
            };
            let Some(op) = op else { break };
            let right = self.parse_unary()?;
            let span = expr.span;
            expr = Expr {
                kind: ExprKind::Binary {
                    op,
                    left: Box::new(expr),
                    right: Box::new(right),
                },
                span,
            };
        }
        Ok(expr)
    }

    fn parse_unary(&mut self) -> Result<Expr, ParseError> {
        if self.consume_if(|k| matches!(k, TokenKind::Bang)) {
            let span = self.current_span();
            let expr = self.parse_unary()?;
            return Ok(Expr {
                kind: ExprKind::Unary {
                    op: UnaryOp::Not,
                    expr: Box::new(expr),
                },
                span,
            });
        }
        if self.consume_if(|k| matches!(k, TokenKind::Minus)) {
            let span = self.current_span();
            let expr = self.parse_unary()?;
            return Ok(Expr {
                kind: ExprKind::Unary {
                    op: UnaryOp::Neg,
                    expr: Box::new(expr),
                },
                span,
            });
        }
        self.parse_postfix()
    }

    fn parse_postfix(&mut self) -> Result<Expr, ParseError> {
        let mut expr = self.parse_primary()?;
        loop {
            if self.consume_if(|k| matches!(k, TokenKind::Dot)) {
                let (name, span) = self.expect_ident()?;
                if self.consume_if(|k| matches!(k, TokenKind::LParen)) {
                    let args = self.parse_args()?;
                    self.expect(
                        |k| matches!(k, TokenKind::RParen),
                        "Expected ')' after argument list.",
                    )?;
                    expr = Expr {
                        kind: ExprKind::Call {
                            receiver: Some(Box::new(expr)),
                            method: name,
                            args,
                        },
                        span,
                    };
                } else {
                    expr = Expr {
                        kind: ExprKind::FieldAccess {
                            receiver: Box::new(expr),
                            field: name,
                        },
                        span,
                    };
                }
                continue;
            }

            if self.consume_if(|k| matches!(k, TokenKind::LParen)) {
                let args = self.parse_args()?;
                self.expect(
                    |k| matches!(k, TokenKind::RParen),
                    "Expected ')' after argument list.",
                )?;
                let span = expr.span;
                expr = self.rewrite_as_call(expr, args, span)?;
                continue;
            }
            break;
        }
        Ok(expr)
    }

    fn rewrite_as_call(&self, expr: Expr, args: Vec<Expr>, span: Span) -> Result<Expr, ParseError> {
        match expr.kind {
            ExprKind::Var(name) => Ok(Expr {
                kind: ExprKind::Call {
                    receiver: None,
                    method: name,
                    args,
                },
                span,
            }),
            ExprKind::FieldAccess { receiver, field } => Ok(Expr {
                kind: ExprKind::Call {
                    receiver: Some(receiver),
                    method: field,
                    args,
                },
                span,
            }),
            _ => Err(ParseError {
                span,
                message: "Invalid method call target.".to_string(),
            }),
        }
    }

    fn parse_args(&mut self) -> Result<Vec<Expr>, ParseError> {
        let mut args = Vec::new();
        if self.at(|k| matches!(k, TokenKind::RParen)) {
            return Ok(args);
        }
        loop {
            args.push(self.parse_expr()?);
            if self.consume_if(|k| matches!(k, TokenKind::Comma)) {
                continue;
            }
            break;
        }
        Ok(args)
    }

    fn parse_primary(&mut self) -> Result<Expr, ParseError> {
        let span = self.current_span();
        let Some(kind) = self.peek_kind().cloned() else {
            return Err(ParseError {
                span,
                message: "Unexpected end of input.".to_string(),
            });
        };

        let kind = match kind {
            TokenKind::IntLiteral(v) => {
                self.bump();
                ExprKind::IntLiteral(v)
            }
            TokenKind::StringLiteral(v) => {
                self.bump();
                ExprKind::StringLiteral(v)
            }
            TokenKind::True => {
                self.bump();
                ExprKind::BoolLiteral(true)
            }
            TokenKind::False => {
                self.bump();
                ExprKind::BoolLiteral(false)
            }
            TokenKind::Null => {
                self.bump();
                ExprKind::Null
            }
            TokenKind::This => {
                self.bump();
                ExprKind::This
            }
            TokenKind::Identifier(name) => {
                self.bump();
                ExprKind::Var(name)
            }
            TokenKind::New => {
                self.bump();
                let class_name = self.parse_qualified_ident()?;
                self.expect(
                    |k| matches!(k, TokenKind::LParen),
                    "Expected '(' after type name in object creation.",
                )?;
                let args = self.parse_args()?;
                self.expect(
                    |k| matches!(k, TokenKind::RParen),
                    "Expected ')' after constructor argument list.",
                )?;
                ExprKind::New { class_name, args }
            }
            TokenKind::LParen => {
                self.bump();
                let expr = self.parse_expr()?;
                self.expect(
                    |k| matches!(k, TokenKind::RParen),
                    "Expected ')' after parenthesized expression.",
                )?;
                return Ok(expr);
            }
            _ => {
                return Err(ParseError {
                    span,
                    message: "Expected expression.".to_string(),
                })
            }
        };

        Ok(Expr { kind, span })
    }

    fn parse_qualified_ident(&mut self) -> Result<String, ParseError> {
        let (mut name, span) = self.expect_ident()?;
        while self.consume_if(|k| matches!(k, TokenKind::Dot)) {
            let (part, _) = self.expect_ident()?;
            name.push('.');
            name.push_str(&part);
        }
        if name.is_empty() {
            return Err(ParseError {
                span,
                message: "Expected type name.".to_string(),
            });
        }
        Ok(name)
    }

    fn parse_binary_chain<F, G>(
        &mut self,
        mut parse_lower: F,
        mut is_op: G,
        op_kind: BinaryOp,
    ) -> Result<Expr, ParseError>
    where
        F: FnMut(&mut Self) -> Result<Expr, ParseError>,
        G: FnMut(&TokenKind) -> bool,
    {
        let mut expr = parse_lower(self)?;
        while self.at(|k| is_op(k)) {
            self.bump();
            let right = parse_lower(self)?;
            let span = expr.span;
            expr = Expr {
                kind: ExprKind::Binary {
                    op: op_kind,
                    left: Box::new(expr),
                    right: Box::new(right),
                },
                span,
            };
        }
        Ok(expr)
    }

    fn expect_ident(&mut self) -> Result<(String, Span), ParseError> {
        let span = self.current_span();
        match self.peek_kind().cloned() {
            Some(TokenKind::Identifier(name)) => {
                self.bump();
                Ok((name, span))
            }
            _ => Err(ParseError {
                span,
                message: "Expected identifier.".to_string(),
            }),
        }
    }

    fn expect<F>(&mut self, mut pred: F, message: &str) -> Result<Span, ParseError>
    where
        F: FnMut(&TokenKind) -> bool,
    {
        let span = self.current_span();
        if self.at(|k| pred(k)) {
            self.bump();
            Ok(span)
        } else {
            Err(ParseError {
                span,
                message: message.to_string(),
            })
        }
    }

    fn consume_if<F>(&mut self, mut pred: F) -> bool
    where
        F: FnMut(&TokenKind) -> bool,
    {
        if self.at(|k| pred(k)) {
            self.bump();
            true
        } else {
            false
        }
    }

    fn at<F>(&self, mut pred: F) -> bool
    where
        F: FnMut(&TokenKind) -> bool,
    {
        self.peek_kind().is_some_and(|k| pred(k))
    }

    fn at_eof(&self) -> bool {
        matches!(self.peek_kind(), Some(TokenKind::Eof))
    }

    fn peek_kind(&self) -> Option<&TokenKind> {
        self.tokens.get(self.idx).map(|t| &t.kind)
    }

    fn peek_n_kind(&self, n: usize) -> Option<&TokenKind> {
        self.tokens.get(self.idx + n).map(|t| &t.kind)
    }

    fn current_span(&self) -> Span {
        self.tokens
            .get(self.idx)
            .map(|t| t.span)
            .unwrap_or(Span { line: 1, col: 1 })
    }

    fn bump(&mut self) {
        if self.idx < self.tokens.len() {
            self.idx += 1;
        }
    }

    fn error_here(&self, message: &str) -> ParseError {
        ParseError {
            span: self.current_span(),
            message: message.to_string(),
        }
    }
}
