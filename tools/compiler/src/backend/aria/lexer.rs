use crate::backend::aria::ast::Span;

#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    Class,
    Public,
    Static,
    Void,
    Int,
    Boolean,
    StringKw,
    If,
    Else,
    While,
    Return,
    True,
    False,
    Null,
    This,
    New,
    Identifier(String),
    IntLiteral(i64),
    StringLiteral(String),
    LBrace,
    RBrace,
    LParen,
    RParen,
    LBracket,
    RBracket,
    Semicolon,
    Comma,
    Dot,
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    Bang,
    Assign,
    EqEq,
    NotEq,
    Lt,
    Le,
    Gt,
    Ge,
    AndAnd,
    OrOr,
    Eof,
}

#[derive(Debug, Clone)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct LexError {
    pub line: usize,
    pub col: usize,
    pub message: String,
}

pub fn lex(source: &str) -> Result<Vec<Token>, LexError> {
    let mut lexer = Lexer::new(source);
    lexer.lex_tokens()
}

struct Lexer {
    chars: Vec<char>,
    idx: usize,
    line: usize,
    col: usize,
}

impl Lexer {
    fn new(source: &str) -> Self {
        Self {
            chars: source.chars().collect(),
            idx: 0,
            line: 1,
            col: 1,
        }
    }

    fn lex_tokens(&mut self) -> Result<Vec<Token>, LexError> {
        let mut tokens = Vec::new();
        loop {
            self.skip_whitespace_and_comments()?;
            let span = Span {
                line: self.line,
                col: self.col,
            };
            let Some(ch) = self.peek() else {
                tokens.push(Token {
                    kind: TokenKind::Eof,
                    span,
                });
                break;
            };

            let token = match ch {
                '{' => {
                    self.bump();
                    TokenKind::LBrace
                }
                '}' => {
                    self.bump();
                    TokenKind::RBrace
                }
                '(' => {
                    self.bump();
                    TokenKind::LParen
                }
                ')' => {
                    self.bump();
                    TokenKind::RParen
                }
                '[' => {
                    self.bump();
                    TokenKind::LBracket
                }
                ']' => {
                    self.bump();
                    TokenKind::RBracket
                }
                ';' => {
                    self.bump();
                    TokenKind::Semicolon
                }
                ',' => {
                    self.bump();
                    TokenKind::Comma
                }
                '.' => {
                    self.bump();
                    TokenKind::Dot
                }
                '+' => {
                    self.bump();
                    TokenKind::Plus
                }
                '-' => {
                    self.bump();
                    TokenKind::Minus
                }
                '*' => {
                    self.bump();
                    TokenKind::Star
                }
                '%' => {
                    self.bump();
                    TokenKind::Percent
                }
                '!' => {
                    self.bump();
                    if self.peek() == Some('=') {
                        self.bump();
                        TokenKind::NotEq
                    } else {
                        TokenKind::Bang
                    }
                }
                '=' => {
                    self.bump();
                    if self.peek() == Some('=') {
                        self.bump();
                        TokenKind::EqEq
                    } else {
                        TokenKind::Assign
                    }
                }
                '<' => {
                    self.bump();
                    if self.peek() == Some('=') {
                        self.bump();
                        TokenKind::Le
                    } else {
                        TokenKind::Lt
                    }
                }
                '>' => {
                    self.bump();
                    if self.peek() == Some('=') {
                        self.bump();
                        TokenKind::Ge
                    } else {
                        TokenKind::Gt
                    }
                }
                '&' => {
                    self.bump();
                    if self.peek() == Some('&') {
                        self.bump();
                        TokenKind::AndAnd
                    } else {
                        return Err(self.error_here("Unexpected '&'. Use '&&'."));
                    }
                }
                '|' => {
                    self.bump();
                    if self.peek() == Some('|') {
                        self.bump();
                        TokenKind::OrOr
                    } else {
                        return Err(self.error_here("Unexpected '|'. Use '||'."));
                    }
                }
                '"' => TokenKind::StringLiteral(self.lex_string()?),
                c if c.is_ascii_digit() => TokenKind::IntLiteral(self.lex_int()?),
                c if is_ident_start(c) => self.lex_ident_or_keyword(),
                '/' => {
                    self.bump();
                    TokenKind::Slash
                }
                _ => {
                    return Err(self.error_here(&format!("Unexpected character '{}'.", ch)));
                }
            };

            tokens.push(Token { kind: token, span });
        }
        Ok(tokens)
    }

    fn skip_whitespace_and_comments(&mut self) -> Result<(), LexError> {
        loop {
            while let Some(ch) = self.peek() {
                if ch.is_whitespace() {
                    self.bump();
                } else {
                    break;
                }
            }

            if self.peek() == Some('/') && self.peek_next() == Some('/') {
                while let Some(ch) = self.peek() {
                    self.bump();
                    if ch == '\n' {
                        break;
                    }
                }
                continue;
            }

            if self.peek() == Some('/') && self.peek_next() == Some('*') {
                self.bump();
                self.bump();
                loop {
                    match (self.peek(), self.peek_next()) {
                        (Some('*'), Some('/')) => {
                            self.bump();
                            self.bump();
                            break;
                        }
                        (Some(_), _) => {
                            self.bump();
                        }
                        (None, _) => return Err(self.error_here("Unterminated block comment.")),
                    }
                }
                continue;
            }
            break;
        }
        Ok(())
    }

    fn lex_string(&mut self) -> Result<String, LexError> {
        let mut value = String::new();
        self.bump();
        loop {
            match self.peek() {
                Some('"') => {
                    self.bump();
                    return Ok(value);
                }
                Some('\\') => {
                    self.bump();
                    let Some(esc) = self.peek() else {
                        return Err(self.error_here("Unterminated escape sequence."));
                    };
                    self.bump();
                    let mapped = match esc {
                        'n' => '\n',
                        't' => '\t',
                        'r' => '\r',
                        '"' => '"',
                        '\\' => '\\',
                        other => other,
                    };
                    value.push(mapped);
                }
                Some(ch) => {
                    self.bump();
                    value.push(ch);
                }
                None => return Err(self.error_here("Unterminated string literal.")),
            }
        }
    }

    fn lex_int(&mut self) -> Result<i64, LexError> {
        let mut value = String::new();
        while let Some(ch) = self.peek() {
            if ch.is_ascii_digit() {
                value.push(ch);
                self.bump();
            } else {
                break;
            }
        }
        value
            .parse::<i64>()
            .map_err(|_| self.error_here("Invalid integer literal."))
    }

    fn lex_ident_or_keyword(&mut self) -> TokenKind {
        let mut ident = String::new();
        while let Some(ch) = self.peek() {
            if is_ident_part(ch) {
                ident.push(ch);
                self.bump();
            } else {
                break;
            }
        }
        match ident.as_str() {
            "class" => TokenKind::Class,
            "public" => TokenKind::Public,
            "static" => TokenKind::Static,
            "void" => TokenKind::Void,
            "int" => TokenKind::Int,
            "boolean" => TokenKind::Boolean,
            "String" => TokenKind::StringKw,
            "if" => TokenKind::If,
            "else" => TokenKind::Else,
            "while" => TokenKind::While,
            "return" => TokenKind::Return,
            "true" => TokenKind::True,
            "false" => TokenKind::False,
            "null" => TokenKind::Null,
            "this" => TokenKind::This,
            "new" => TokenKind::New,
            _ => TokenKind::Identifier(ident),
        }
    }

    fn peek(&self) -> Option<char> {
        self.chars.get(self.idx).copied()
    }

    fn peek_next(&self) -> Option<char> {
        self.chars.get(self.idx + 1).copied()
    }

    fn bump(&mut self) {
        if let Some(ch) = self.peek() {
            self.idx += 1;
            if ch == '\n' {
                self.line += 1;
                self.col = 1;
            } else {
                self.col += 1;
            }
        }
    }

    fn error_here(&self, message: &str) -> LexError {
        LexError {
            line: self.line,
            col: self.col,
            message: message.to_string(),
        }
    }
}

fn is_ident_start(ch: char) -> bool {
    ch == '_' || ch == '$' || ch.is_ascii_alphabetic()
}

fn is_ident_part(ch: char) -> bool {
    is_ident_start(ch) || ch.is_ascii_digit()
}
