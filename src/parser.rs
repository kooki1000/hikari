use crate::lexer::{Span, Token, TokenKind};

// ── Types ────────────────────────────────────────────────────────────────────

#[derive(Debug, PartialEq, Clone)]
pub enum HikariType {
    Int,    // 整数
    Float,  // 小数
    String, // 文字列
    Bool,   // 真偽
    Void,   // 無
    Array(Box<HikariType>),
    Map(Box<HikariType>, Box<HikariType>), // key type, value type
    Record(String), // user-defined record type, identified by its declared name
    Enum(String),   // user-defined enum type, identified by its declared name
    // Phase 10: function type — 関数＜(T1、T2) → R＞
    Fn(Vec<HikariType>, Box<HikariType>),
}

// ── AST nodes ────────────────────────────────────────────────────────────────

#[derive(Debug, PartialEq, Clone)]
pub enum Expr {
    LitInt(i64),
    LitFloat(f64),
    LitString(String),
    LitBool(bool),
    Ident(String),
    BinOp {
        op: BinOpKind,
        lhs: Box<Expr>,
        rhs: Box<Expr>,
    },
    Call {
        name: String,
        args: Vec<Expr>,
    },
    UnaryMinus(Box<Expr>),
    UnaryNot(Box<Expr>),
    Array(Vec<Expr>),
    Index {
        array: Box<Expr>,
        index: Box<Expr>,
    },
    NewArray(HikariType),
    MapLit(Vec<(Expr, Expr)>),
    RecordLit {
        type_name: String,
        fields: Vec<(String, Expr)>,
    },
    FieldAccess {
        record: Box<Expr>,
        field: String,
    },
    // Phase 10: anonymous function (lambda) — ｜params｜ → return_ty ｛ body ｝
    Lambda {
        params: Vec<(String, HikariType)>,
        return_ty: HikariType,
        body: Vec<Stmt>,
    },
}

#[derive(Debug, PartialEq, Clone)]
pub enum BinOpKind {
    Add,   // ＋
    Sub,   // ー
    Mul,   // ＊
    Div,   // ／
    Mod,   // ％
    Eq,    // ＝＝
    Lt,    // ＜
    Gt,    // ＞
    LtEq,  // ≦
    GtEq,  // ≧
    NotEq, // ≠
    And,   // かつ
    Or,    // または
}

#[derive(Debug, PartialEq, Clone)]
pub enum Stmt {
    VarDecl {
        ty: HikariType,
        name: String,
        value: Expr,
        span: Span,
    },
    FnDecl {
        name: String,
        params: Vec<(HikariType, String)>,
        return_ty: HikariType,
        body: Vec<Stmt>,
        span: Span,
    },
    Return(Option<Expr>, Span),
    Print(Expr, Span),
    If {
        condition: Expr,
        then_body: Vec<Stmt>,
        else_body: Option<Vec<Stmt>>,
        span: Span,
    },
    While {
        condition: Expr,
        body: Vec<Stmt>,
        span: Span,
    },
    Expr(Expr, Span),
    Assign {
        name: String,
        value: Expr,
        span: Span,
    },
    IndexAssign {
        name: String,
        index: Expr,
        value: Expr,
        span: Span,
    },
    ForRange {
        var: String,
        from: Expr,
        to: Expr,
        body: Vec<Stmt>,
        span: Span,
    },
    ForEach {
        var: String,
        array: Expr,
        body: Vec<Stmt>,
        span: Span,
    },
    TryCatch {
        try_body: Vec<Stmt>,
        error_var: String,
        catch_body: Vec<Stmt>,
        span: Span,
    },
    Import {
        name: String,
        span: Span,
    },
    Break(Span),
    Continue(Span),
    TypeDecl {
        name: String,
        fields: Vec<(HikariType, String)>,
        span: Span,
    },
    // The target is a full Expr (not a bare name like IndexAssign's) because
    // field-assign targets can themselves be the result of an arbitrary
    // expression, e.g. 配列【０】：：ｘ ＝ １；, whereas IndexAssign was modeled
    // around a bare local variable name.
    FieldAssign {
        record: Expr,
        field: String,
        value: Expr,
        span: Span,
    },
    EnumDecl {
        name: String,
        variants: Vec<(String, Vec<HikariType>)>,
        span: Span,
    },
    Match {
        subject: Expr,
        arms: Vec<MatchArm>,
        span: Span,
    },
}

#[derive(Debug, PartialEq, Clone)]
pub struct MatchArm {
    pub variant: String,
    pub binders: Vec<String>,
    pub body: Vec<Stmt>,
}

// ── Error type ────────────────────────────────────────────────────────────────

#[derive(Debug, PartialEq)]
pub enum ParseError {
    UnexpectedToken {
        expected: TokenKind,
        got: TokenKind,
        span: Span,
    },
    ExpectedIdentifier {
        got: TokenKind,
        span: Span,
    },
    ExpectedType {
        got: TokenKind,
        span: Span,
    },
    UnexpectedExprToken {
        got: TokenKind,
        span: Span,
    },
    InvalidNumber {
        text: String,
        span: Span,
    },
}

impl ParseError {
    pub fn span(&self) -> Span {
        match self {
            ParseError::UnexpectedToken { span, .. } => *span,
            ParseError::ExpectedIdentifier { span, .. } => *span,
            ParseError::ExpectedType { span, .. } => *span,
            ParseError::UnexpectedExprToken { span, .. } => *span,
            ParseError::InvalidNumber { span, .. } => *span,
        }
    }
}

// ── Parser ───────────────────────────────────────────────────────────────────

pub struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, pos: 0 }
    }

    fn peek(&self) -> &TokenKind {
        &self.tokens[self.pos].kind
    }

    fn peek_next(&self) -> &TokenKind {
        let idx = (self.pos + 1).min(self.tokens.len() - 1);
        &self.tokens[idx].kind
    }

    fn peek_at(&self, offset: usize) -> &TokenKind {
        let idx = (self.pos + offset).min(self.tokens.len() - 1);
        &self.tokens[idx].kind
    }

    fn peek_span(&self) -> Span {
        self.tokens[self.pos].span
    }

    fn advance(&mut self) -> &TokenKind {
        let kind = &self.tokens[self.pos].kind;
        if self.pos + 1 < self.tokens.len() {
            self.pos += 1;
        }
        kind
    }

    fn expect(&mut self, expected: &TokenKind) -> Result<(), ParseError> {
        let span = self.peek_span();
        let got = self.advance().clone();
        if std::mem::discriminant(&got) == std::mem::discriminant(expected) {
            Ok(())
        } else {
            Err(ParseError::UnexpectedToken {
                expected: expected.clone(),
                got,
                span,
            })
        }
    }

    pub fn parse(&mut self) -> Result<Vec<Stmt>, ParseError> {
        let mut stmts = Vec::new();
        while self.peek() != &TokenKind::Eof {
            stmts.push(self.parse_stmt()?);
        }
        Ok(stmts)
    }

    fn parse_stmt(&mut self) -> Result<Stmt, ParseError> {
        match self.peek().clone() {
            // Phase 10: 関数＜...＞ name ＝ expr;  is a var decl with Fn type.
            // 関数 name（...） → ... ｛ ... ｝ is a named fn decl.
            TokenKind::KwFn if self.peek_next() == &TokenKind::Lt => self.parse_var_decl(),
            TokenKind::KwFn => self.parse_fn_decl(),
            TokenKind::KwReturn => self.parse_return(),
            TokenKind::KwPrint => self.parse_print(),
            TokenKind::KwIf => self.parse_if(),
            TokenKind::KwWhile => self.parse_while(),
            TokenKind::KwForRange => self.parse_for_range(),
            TokenKind::KwEach => self.parse_for_each(),
            TokenKind::KwTry => self.parse_try_catch(),
            TokenKind::KwImport => self.parse_import(),
            TokenKind::KwBreak => self.parse_break(),
            TokenKind::KwContinue => self.parse_continue(),
            TokenKind::KwType => self.parse_type_decl(),
            TokenKind::KwEnum => self.parse_enum_decl(),
            TokenKind::KwMatch => self.parse_match(),
            kind if is_type_token(&kind) => self.parse_var_decl(),
            TokenKind::Ident(_) if self.peek_next() == &TokenKind::LBracket => {
                self.parse_index_assign()
            }
            TokenKind::Ident(_) if self.peek_next() == &TokenKind::Assign => self.parse_assign(),
            // Two bare identifiers in a row at statement-start is the unique
            // shape of a record-typed var-decl (型名 変数名 ＝ ...); no other
            // current construct starts with two consecutive Idents.
            TokenKind::Ident(_) if matches!(self.peek_next(), TokenKind::Ident(_)) => {
                self.parse_var_decl()
            }
            _ => self.parse_expr_or_field_assign(),
        }
    }

    fn parse_expr_or_field_assign(&mut self) -> Result<Stmt, ParseError> {
        let span = self.peek_span();
        let expr = self.parse_expr()?;
        if let Expr::FieldAccess { record, field } = expr {
            if self.peek() == &TokenKind::Assign {
                self.advance();
                let value = self.parse_expr()?;
                self.expect(&TokenKind::Semi)?;
                return Ok(Stmt::FieldAssign {
                    record: *record,
                    field,
                    value,
                    span,
                });
            }
            self.expect(&TokenKind::Semi)?;
            return Ok(Stmt::Expr(Expr::FieldAccess { record, field }, span));
        }
        self.expect(&TokenKind::Semi)?;
        Ok(Stmt::Expr(expr, span))
    }

    fn parse_type_decl(&mut self) -> Result<Stmt, ParseError> {
        let span = self.peek_span();
        self.advance(); // consume 型
        let name = match (self.peek_span(), self.advance().clone()) {
            (_, TokenKind::Ident(n)) => n,
            (s, other) => {
                return Err(ParseError::ExpectedIdentifier {
                    got: other,
                    span: s,
                });
            }
        };
        self.expect(&TokenKind::LBrace)?;
        let mut fields = Vec::new();
        while self.peek() != &TokenKind::RBrace {
            let ty = self.parse_type()?;
            let fname = match (self.peek_span(), self.advance().clone()) {
                (_, TokenKind::Ident(n)) => n,
                (s, other) => {
                    return Err(ParseError::ExpectedIdentifier {
                        got: other,
                        span: s,
                    });
                }
            };
            self.expect(&TokenKind::Semi)?;
            fields.push((ty, fname));
        }
        self.expect(&TokenKind::RBrace)?;
        Ok(Stmt::TypeDecl { name, fields, span })
    }

    fn parse_enum_decl(&mut self) -> Result<Stmt, ParseError> {
        let span = self.peek_span();
        self.advance(); // consume 構造
        let name = match (self.peek_span(), self.advance().clone()) {
            (_, TokenKind::Ident(n)) => n,
            (s, other) => {
                return Err(ParseError::ExpectedIdentifier {
                    got: other,
                    span: s,
                });
            }
        };
        self.expect(&TokenKind::LBrace)?;
        let mut variants = Vec::new();
        while self.peek() != &TokenKind::RBrace {
            let vname = match (self.peek_span(), self.advance().clone()) {
                (_, TokenKind::Ident(n)) => n,
                (s, other) => {
                    return Err(ParseError::ExpectedIdentifier {
                        got: other,
                        span: s,
                    });
                }
            };
            let mut payload = Vec::new();
            if self.peek() == &TokenKind::LParen {
                self.advance(); // consume （
                while self.peek() != &TokenKind::RParen {
                    payload.push(self.parse_type()?);
                    if self.peek() != &TokenKind::RParen {
                        self.expect(&TokenKind::Comma)?;
                    }
                }
                self.expect(&TokenKind::RParen)?;
            }
            variants.push((vname, payload));
            if self.peek() != &TokenKind::RBrace {
                self.expect(&TokenKind::Comma)?;
            }
        }
        self.expect(&TokenKind::RBrace)?;
        Ok(Stmt::EnumDecl {
            name,
            variants,
            span,
        })
    }

    fn parse_match(&mut self) -> Result<Stmt, ParseError> {
        let span = self.peek_span();
        self.advance(); // consume 照合
        let subject = self.parse_expr()?;
        self.expect(&TokenKind::LBrace)?;
        let mut arms = Vec::new();
        while self.peek() != &TokenKind::RBrace {
            let variant = match (self.peek_span(), self.advance().clone()) {
                (_, TokenKind::Ident(n)) => n,
                (s, other) => {
                    return Err(ParseError::ExpectedIdentifier {
                        got: other,
                        span: s,
                    });
                }
            };
            self.expect(&TokenKind::LParen)?;
            let mut binders = Vec::new();
            while self.peek() != &TokenKind::RParen {
                let bname = match (self.peek_span(), self.advance().clone()) {
                    (_, TokenKind::Ident(n)) => n,
                    (s, other) => {
                        return Err(ParseError::ExpectedIdentifier {
                            got: other,
                            span: s,
                        });
                    }
                };
                binders.push(bname);
                if self.peek() != &TokenKind::RParen {
                    self.expect(&TokenKind::Comma)?;
                }
            }
            self.expect(&TokenKind::RParen)?;
            self.expect(&TokenKind::KwThen)?; // ならば
            self.expect(&TokenKind::LBrace)?;
            let mut body = Vec::new();
            while self.peek() != &TokenKind::RBrace {
                body.push(self.parse_stmt()?);
            }
            self.expect(&TokenKind::RBrace)?;
            arms.push(MatchArm {
                variant,
                binders,
                body,
            });
        }
        self.expect(&TokenKind::RBrace)?;
        Ok(Stmt::Match {
            subject,
            arms,
            span,
        })
    }

    fn parse_var_decl(&mut self) -> Result<Stmt, ParseError> {
        let span = self.peek_span();
        let ty = self.parse_type()?;
        let name = match (self.peek_span(), self.advance().clone()) {
            (_, TokenKind::Ident(n)) => n,
            (s, other) => {
                return Err(ParseError::ExpectedIdentifier {
                    got: other,
                    span: s,
                });
            }
        };
        self.expect(&TokenKind::Assign)?;
        let value = self.parse_expr()?;
        self.expect(&TokenKind::Semi)?;
        Ok(Stmt::VarDecl {
            ty,
            name,
            value,
            span,
        })
    }

    fn parse_assign(&mut self) -> Result<Stmt, ParseError> {
        let span = self.peek_span();
        let name = match self.advance().clone() {
            TokenKind::Ident(n) => n,
            _ => unreachable!("guarded by caller"),
        };
        self.expect(&TokenKind::Assign)?;
        let value = self.parse_expr()?;
        self.expect(&TokenKind::Semi)?;
        Ok(Stmt::Assign { name, value, span })
    }

    fn parse_index_assign(&mut self) -> Result<Stmt, ParseError> {
        let span = self.peek_span();
        let name = match self.advance().clone() {
            TokenKind::Ident(n) => n,
            _ => unreachable!("guarded by caller"),
        };
        self.expect(&TokenKind::LBracket)?;
        let index = self.parse_expr()?;
        self.expect(&TokenKind::RBracket)?;
        self.expect(&TokenKind::Assign)?;
        let value = self.parse_expr()?;
        self.expect(&TokenKind::Semi)?;
        Ok(Stmt::IndexAssign {
            name,
            index,
            value,
            span,
        })
    }

    fn parse_fn_decl(&mut self) -> Result<Stmt, ParseError> {
        let span = self.peek_span();
        self.advance(); // consume 関数
        let name = match (self.peek_span(), self.advance().clone()) {
            (_, TokenKind::Ident(n)) => n,
            (s, other) => {
                return Err(ParseError::ExpectedIdentifier {
                    got: other,
                    span: s,
                });
            }
        };
        self.expect(&TokenKind::LParen)?;
        let mut params = Vec::new();
        while self.peek() != &TokenKind::RParen {
            let ty = self.parse_type()?;
            let pname = match (self.peek_span(), self.advance().clone()) {
                (_, TokenKind::Ident(n)) => n,
                (s, other) => {
                    return Err(ParseError::ExpectedIdentifier {
                        got: other,
                        span: s,
                    });
                }
            };
            params.push((ty, pname));
            if self.peek() != &TokenKind::RParen {
                self.expect(&TokenKind::Comma)?;
            }
        }
        self.expect(&TokenKind::RParen)?;
        self.expect(&TokenKind::Arrow)?;
        let return_ty = self.parse_type()?;
        self.expect(&TokenKind::LBrace)?;
        let mut body = Vec::new();
        while self.peek() != &TokenKind::RBrace {
            body.push(self.parse_stmt()?);
        }
        self.expect(&TokenKind::RBrace)?;
        Ok(Stmt::FnDecl {
            name,
            params,
            return_ty,
            body,
            span,
        })
    }

    fn parse_return(&mut self) -> Result<Stmt, ParseError> {
        let span = self.peek_span();
        self.advance(); // consume 返す
        if self.peek() == &TokenKind::Semi {
            self.advance();
            return Ok(Stmt::Return(None, span));
        }
        let expr = self.parse_expr()?;
        self.expect(&TokenKind::Semi)?;
        Ok(Stmt::Return(Some(expr), span))
    }

    fn parse_break(&mut self) -> Result<Stmt, ParseError> {
        let span = self.peek_span();
        self.advance(); // consume 抜ける
        self.expect(&TokenKind::Semi)?;
        Ok(Stmt::Break(span))
    }

    fn parse_continue(&mut self) -> Result<Stmt, ParseError> {
        let span = self.peek_span();
        self.advance(); // consume 続ける
        self.expect(&TokenKind::Semi)?;
        Ok(Stmt::Continue(span))
    }

    fn parse_if(&mut self) -> Result<Stmt, ParseError> {
        let span = self.peek_span();
        self.advance(); // consume もし
        let condition = self.parse_expr()?;
        self.expect(&TokenKind::KwThen)?; // ならば
        self.expect(&TokenKind::LBrace)?;
        let mut then_body = Vec::new();
        while self.peek() != &TokenKind::RBrace {
            then_body.push(self.parse_stmt()?);
        }
        self.expect(&TokenKind::RBrace)?;
        let else_body = if self.peek() == &TokenKind::KwElse {
            self.advance(); // consume 違えば
            self.expect(&TokenKind::LBrace)?;
            let mut body = Vec::new();
            while self.peek() != &TokenKind::RBrace {
                body.push(self.parse_stmt()?);
            }
            self.expect(&TokenKind::RBrace)?;
            Some(body)
        } else {
            None
        };
        Ok(Stmt::If {
            condition,
            then_body,
            else_body,
            span,
        })
    }

    fn parse_while(&mut self) -> Result<Stmt, ParseError> {
        let span = self.peek_span();
        self.advance(); // consume 間
        let condition = self.parse_expr()?;
        self.expect(&TokenKind::KwThen)?; // ならば
        self.expect(&TokenKind::LBrace)?;
        let mut body = Vec::new();
        while self.peek() != &TokenKind::RBrace {
            body.push(self.parse_stmt()?);
        }
        self.expect(&TokenKind::RBrace)?;
        Ok(Stmt::While {
            condition,
            body,
            span,
        })
    }

    fn parse_for_range(&mut self) -> Result<Stmt, ParseError> {
        let span = self.peek_span();
        self.advance(); // consume 繰り返す
        let var = match (self.peek_span(), self.advance().clone()) {
            (_, TokenKind::Ident(n)) => n,
            (s, other) => {
                return Err(ParseError::ExpectedIdentifier {
                    got: other,
                    span: s,
                });
            }
        };
        self.expect(&TokenKind::Assign)?;
        let from = self.parse_expr()?;
        self.expect(&TokenKind::KwFrom)?; // から
        let to = self.parse_expr()?;
        self.expect(&TokenKind::KwThen)?; // ならば
        self.expect(&TokenKind::LBrace)?;
        let mut body = Vec::new();
        while self.peek() != &TokenKind::RBrace {
            body.push(self.parse_stmt()?);
        }
        self.expect(&TokenKind::RBrace)?;
        Ok(Stmt::ForRange {
            var,
            from,
            to,
            body,
            span,
        })
    }

    fn parse_for_each(&mut self) -> Result<Stmt, ParseError> {
        let span = self.peek_span();
        self.advance(); // consume 各
        let var = match (self.peek_span(), self.advance().clone()) {
            (_, TokenKind::Ident(n)) => n,
            (s, other) => {
                return Err(ParseError::ExpectedIdentifier {
                    got: other,
                    span: s,
                });
            }
        };
        self.expect(&TokenKind::Colon)?; // ：
        let array = self.parse_expr()?;
        self.expect(&TokenKind::KwThen)?; // ならば
        self.expect(&TokenKind::LBrace)?;
        let mut body = Vec::new();
        while self.peek() != &TokenKind::RBrace {
            body.push(self.parse_stmt()?);
        }
        self.expect(&TokenKind::RBrace)?;
        Ok(Stmt::ForEach {
            var,
            array,
            body,
            span,
        })
    }

    fn parse_try_catch(&mut self) -> Result<Stmt, ParseError> {
        let span = self.peek_span();
        self.advance(); // consume 試す
        self.expect(&TokenKind::LBrace)?;
        let mut try_body = Vec::new();
        while self.peek() != &TokenKind::RBrace {
            try_body.push(self.parse_stmt()?);
        }
        self.expect(&TokenKind::RBrace)?;
        self.expect(&TokenKind::KwCatch)?;
        let error_var = match (self.peek_span(), self.advance().clone()) {
            (_, TokenKind::Ident(n)) => n,
            (s, other) => {
                return Err(ParseError::ExpectedIdentifier {
                    got: other,
                    span: s,
                });
            }
        };
        self.expect(&TokenKind::LBrace)?;
        let mut catch_body = Vec::new();
        while self.peek() != &TokenKind::RBrace {
            catch_body.push(self.parse_stmt()?);
        }
        self.expect(&TokenKind::RBrace)?;
        Ok(Stmt::TryCatch {
            try_body,
            error_var,
            catch_body,
            span,
        })
    }

    fn parse_import(&mut self) -> Result<Stmt, ParseError> {
        let span = self.peek_span();
        self.advance(); // consume 取り込む
        let name_span = self.peek_span();
        let name = match self.advance().clone() {
            TokenKind::LitString(s) => s,
            other => {
                return Err(ParseError::UnexpectedToken {
                    expected: TokenKind::LitString(String::new()),
                    got: other,
                    span: name_span,
                });
            }
        };
        self.expect(&TokenKind::Semi)?;
        Ok(Stmt::Import { name, span })
    }

    fn parse_print(&mut self) -> Result<Stmt, ParseError> {
        let span = self.peek_span();
        self.advance(); // consume 印刷
        self.expect(&TokenKind::LParen)?;
        let expr = self.parse_expr()?;
        self.expect(&TokenKind::RParen)?;
        self.expect(&TokenKind::Semi)?;
        Ok(Stmt::Print(expr, span))
    }

    fn parse_expr(&mut self) -> Result<Expr, ParseError> {
        self.parse_or()
    }

    // Logical OR: lowest precedence (または)
    fn parse_or(&mut self) -> Result<Expr, ParseError> {
        let mut lhs = self.parse_and()?;
        while self.peek() == &TokenKind::KwOr {
            self.advance();
            let rhs = self.parse_and()?;
            lhs = Expr::BinOp {
                op: BinOpKind::Or,
                lhs: Box::new(lhs),
                rhs: Box::new(rhs),
            };
        }
        Ok(lhs)
    }

    // Logical AND (かつ)
    fn parse_and(&mut self) -> Result<Expr, ParseError> {
        let mut lhs = self.parse_comparison()?;
        while self.peek() == &TokenKind::KwAnd {
            self.advance();
            let rhs = self.parse_comparison()?;
            lhs = Expr::BinOp {
                op: BinOpKind::And,
                lhs: Box::new(lhs),
                rhs: Box::new(rhs),
            };
        }
        Ok(lhs)
    }

    // Comparison (＝＝  ＜  ＞  ≦  ≧  ≠)
    fn parse_comparison(&mut self) -> Result<Expr, ParseError> {
        let lhs = self.parse_additive()?;
        let op = match self.peek() {
            TokenKind::EqEq => BinOpKind::Eq,
            TokenKind::Lt => BinOpKind::Lt,
            TokenKind::Gt => BinOpKind::Gt,
            TokenKind::LtEq => BinOpKind::LtEq,
            TokenKind::GtEq => BinOpKind::GtEq,
            TokenKind::NotEq => BinOpKind::NotEq,
            _ => return Ok(lhs),
        };
        self.advance();
        let rhs = self.parse_additive()?;
        Ok(Expr::BinOp {
            op,
            lhs: Box::new(lhs),
            rhs: Box::new(rhs),
        })
    }

    fn parse_additive(&mut self) -> Result<Expr, ParseError> {
        let mut lhs = self.parse_multiplicative()?;
        loop {
            let op = match self.peek() {
                TokenKind::Plus => BinOpKind::Add,
                TokenKind::Minus => BinOpKind::Sub,
                _ => break,
            };
            self.advance();
            let rhs = self.parse_multiplicative()?;
            lhs = Expr::BinOp {
                op,
                lhs: Box::new(lhs),
                rhs: Box::new(rhs),
            };
        }
        Ok(lhs)
    }

    fn parse_multiplicative(&mut self) -> Result<Expr, ParseError> {
        let mut lhs = self.parse_primary()?;
        loop {
            let op = match self.peek() {
                TokenKind::Star => BinOpKind::Mul,
                TokenKind::Slash => BinOpKind::Div,
                TokenKind::Percent => BinOpKind::Mod,
                _ => break,
            };
            self.advance();
            let rhs = self.parse_primary()?;
            lhs = Expr::BinOp {
                op,
                lhs: Box::new(lhs),
                rhs: Box::new(rhs),
            };
        }
        Ok(lhs)
    }

    fn parse_primary(&mut self) -> Result<Expr, ParseError> {
        if self.peek() == &TokenKind::Minus {
            self.advance();
            let inner = self.parse_primary()?;
            return Ok(Expr::UnaryMinus(Box::new(inner)));
        }
        if self.peek() == &TokenKind::KwNot {
            self.advance();
            let inner = self.parse_primary()?;
            return Ok(Expr::UnaryNot(Box::new(inner)));
        }
        // Phase 10: lambda — ｜ param：type、...｜ → return_ty ｛ body ｝
        if self.peek() == &TokenKind::Pipe {
            self.advance(); // consume ｜
            let mut params = Vec::new();
            while self.peek() != &TokenKind::Pipe {
                let pname = match (self.peek_span(), self.advance().clone()) {
                    (_, TokenKind::Ident(n)) => n,
                    (s, other) => {
                        return Err(ParseError::ExpectedIdentifier {
                            got: other,
                            span: s,
                        });
                    }
                };
                self.expect(&TokenKind::Colon)?;
                let pty = self.parse_type()?;
                params.push((pname, pty));
                if self.peek() != &TokenKind::Pipe {
                    self.expect(&TokenKind::Comma)?;
                }
            }
            self.expect(&TokenKind::Pipe)?; // consume closing ｜
            self.expect(&TokenKind::Arrow)?;
            let return_ty = self.parse_type()?;
            self.expect(&TokenKind::LBrace)?;
            let mut body = Vec::new();
            while self.peek() != &TokenKind::RBrace {
                body.push(self.parse_stmt()?);
            }
            self.expect(&TokenKind::RBrace)?;
            return Ok(Expr::Lambda {
                params,
                return_ty,
                body,
            });
        }
        let span = self.peek_span();
        let mut expr = match self.advance().clone() {
            TokenKind::LitInt(n) => Ok(Expr::LitInt(n)),
            TokenKind::LitFloat(f) => Ok(Expr::LitFloat(f)),
            TokenKind::LitString(s) => Ok(Expr::LitString(s)),
            TokenKind::LitTrue => Ok(Expr::LitBool(true)),
            TokenKind::LitFalse => Ok(Expr::LitBool(false)),
            TokenKind::Ident(name) => {
                if self.peek() == &TokenKind::LParen {
                    self.advance(); // consume （
                    let mut args = Vec::new();
                    while self.peek() != &TokenKind::RParen {
                        args.push(self.parse_expr()?);
                        if self.peek() != &TokenKind::RParen {
                            self.expect(&TokenKind::Comma)?;
                        }
                    }
                    self.advance(); // consume ）
                    Ok(Expr::Call { name, args })
                } else if self.peek() == &TokenKind::LBrace
                    && matches!(self.peek_at(1), TokenKind::Ident(_))
                    && self.peek_at(2) == &TokenKind::Colon
                {
                    self.advance(); // consume ｛
                    let mut fields = Vec::new();
                    while self.peek() != &TokenKind::RBrace {
                        let fname = match (self.peek_span(), self.advance().clone()) {
                            (_, TokenKind::Ident(n)) => n,
                            (s, other) => {
                                return Err(ParseError::ExpectedIdentifier {
                                    got: other,
                                    span: s,
                                });
                            }
                        };
                        self.expect(&TokenKind::Colon)?;
                        let value = self.parse_expr()?;
                        fields.push((fname, value));
                        if self.peek() != &TokenKind::RBrace {
                            self.expect(&TokenKind::Comma)?;
                        }
                    }
                    self.expect(&TokenKind::RBrace)?;
                    Ok(Expr::RecordLit {
                        type_name: name,
                        fields,
                    })
                } else {
                    Ok(Expr::Ident(name))
                }
            }
            TokenKind::LParen => {
                let expr = self.parse_expr()?;
                self.expect(&TokenKind::RParen)?;
                Ok(expr)
            }
            TokenKind::KwNewArray => {
                self.expect(&TokenKind::Lt)?;
                let ty = self.parse_type()?;
                self.expect(&TokenKind::Gt)?;
                Ok(Expr::NewArray(ty))
            }
            TokenKind::LBracket => {
                let mut elems = Vec::new();
                while self.peek() != &TokenKind::RBracket {
                    elems.push(self.parse_expr()?);
                    if self.peek() != &TokenKind::RBracket {
                        self.expect(&TokenKind::Comma)?;
                    }
                }
                self.advance(); // consume 】
                Ok(Expr::Array(elems))
            }
            // Map literal: ｛ ｝ or ｛ key：val、key：val ｝
            // Note: record literals are parsed via the Ident branch above (Ident ｛ field：val ｝).
            // Here a bare ｛ always starts a map literal.
            TokenKind::LBrace => {
                let mut pairs = Vec::new();
                while self.peek() != &TokenKind::RBrace {
                    let key = self.parse_expr()?;
                    self.expect(&TokenKind::Colon)?;
                    let val = self.parse_expr()?;
                    pairs.push((key, val));
                    if self.peek() != &TokenKind::RBrace {
                        self.expect(&TokenKind::Comma)?;
                    }
                }
                self.advance(); // consume ｝
                Ok(Expr::MapLit(pairs))
            }
            TokenKind::Invalid(text) => Err(ParseError::InvalidNumber { text, span }),
            other => Err(ParseError::UnexpectedExprToken { got: other, span }),
        }?;
        loop {
            if self.peek() == &TokenKind::LBracket {
                self.advance(); // consume 【
                let index = self.parse_expr()?;
                self.expect(&TokenKind::RBracket)?;
                expr = Expr::Index {
                    array: Box::new(expr),
                    index: Box::new(index),
                };
            } else if self.peek() == &TokenKind::DoubleColon {
                self.advance(); // consume ：：
                let field = match (self.peek_span(), self.advance().clone()) {
                    (_, TokenKind::Ident(n)) => n,
                    (s, other) => {
                        return Err(ParseError::ExpectedIdentifier {
                            got: other,
                            span: s,
                        });
                    }
                };
                expr = Expr::FieldAccess {
                    record: Box::new(expr),
                    field,
                };
            } else {
                break;
            }
        }
        Ok(expr)
    }

    fn parse_type(&mut self) -> Result<HikariType, ParseError> {
        let span = self.peek_span();
        match self.advance().clone() {
            TokenKind::TyInt => Ok(HikariType::Int),
            TokenKind::TyFloat => Ok(HikariType::Float),
            TokenKind::TyString => Ok(HikariType::String),
            TokenKind::TyBool => Ok(HikariType::Bool),
            TokenKind::TyVoid => Ok(HikariType::Void),
            TokenKind::TyIntArray => Ok(HikariType::Array(Box::new(HikariType::Int))),
            TokenKind::TyFloatArray => Ok(HikariType::Array(Box::new(HikariType::Float))),
            TokenKind::TyStringArray => Ok(HikariType::Array(Box::new(HikariType::String))),
            TokenKind::TyBoolArray => Ok(HikariType::Array(Box::new(HikariType::Bool))),
            TokenKind::KwMap => {
                self.expect(&TokenKind::Lt)?;
                let key_ty = self.parse_type()?;
                self.expect(&TokenKind::Comma)?;
                let val_ty = self.parse_type()?;
                self.expect(&TokenKind::Gt)?;
                Ok(HikariType::Map(Box::new(key_ty), Box::new(val_ty)))
            }
            // Phase 10: 関数＜(T1、T2) → R＞
            TokenKind::KwFn => {
                self.expect(&TokenKind::Lt)?;
                self.expect(&TokenKind::LParen)?;
                let mut param_types = Vec::new();
                while self.peek() != &TokenKind::RParen {
                    param_types.push(self.parse_type()?);
                    if self.peek() != &TokenKind::RParen {
                        self.expect(&TokenKind::Comma)?;
                    }
                }
                self.expect(&TokenKind::RParen)?;
                self.expect(&TokenKind::Arrow)?;
                let ret_ty = self.parse_type()?;
                self.expect(&TokenKind::Gt)?;
                Ok(HikariType::Fn(param_types, Box::new(ret_ty)))
            }
            TokenKind::Ident(name) => Ok(HikariType::Record(name)),
            other => Err(ParseError::ExpectedType { got: other, span }),
        }
    }
}

// ── Japanese display helpers ────────────────────────────────────────────────

pub fn token_kind_japanese(kind: &TokenKind) -> String {
    match kind {
        TokenKind::TyInt => "「整数」".to_string(),
        TokenKind::TyFloat => "「小数」".to_string(),
        TokenKind::TyString => "「文字列」".to_string(),
        TokenKind::TyBool => "「真偽」".to_string(),
        TokenKind::TyVoid => "「無」".to_string(),
        TokenKind::TyIntArray => "「整数列」".to_string(),
        TokenKind::TyFloatArray => "「小数列」".to_string(),
        TokenKind::TyStringArray => "「文字列列」".to_string(),
        TokenKind::TyBoolArray => "「真偽列」".to_string(),
        TokenKind::KwFn => "「関数」".to_string(),
        TokenKind::KwReturn => "「返す」".to_string(),
        TokenKind::KwPrint => "「印刷」".to_string(),
        TokenKind::KwIf => "「もし」".to_string(),
        TokenKind::KwThen => "「ならば」".to_string(),
        TokenKind::KwElse => "「違えば」".to_string(),
        TokenKind::KwWhile => "「間」".to_string(),
        TokenKind::KwAnd => "「かつ」".to_string(),
        TokenKind::KwOr => "「または」".to_string(),
        TokenKind::KwNot => "「否定」".to_string(),
        TokenKind::KwForRange => "「繰り返す」".to_string(),
        TokenKind::KwFrom => "「から」".to_string(),
        TokenKind::KwEach => "「各」".to_string(),
        TokenKind::KwTry => "「試す」".to_string(),
        TokenKind::KwCatch => "「失敗」".to_string(),
        TokenKind::KwImport => "「取り込む」".to_string(),
        TokenKind::KwNewArray => "「新配列」".to_string(),
        TokenKind::KwBreak => "「抜ける」".to_string(),
        TokenKind::KwContinue => "「続ける」".to_string(),
        TokenKind::KwType => "「型」".to_string(),
        TokenKind::KwEnum => "「構造」".to_string(),
        TokenKind::KwMatch => "「照合」".to_string(),
        TokenKind::KwMap => "「辞書」".to_string(),
        TokenKind::LitInt(n) => format!("整数リテラル「{}」", n),
        TokenKind::LitFloat(f) => format!("小数リテラル「{}」", f),
        TokenKind::LitString(s) => format!("文字列リテラル「{}」", s),
        TokenKind::LitTrue => "「真」".to_string(),
        TokenKind::LitFalse => "「偽」".to_string(),
        TokenKind::Assign => "「＝」".to_string(),
        TokenKind::EqEq => "「＝＝」".to_string(),
        TokenKind::Lt => "「＜」".to_string(),
        TokenKind::Gt => "「＞」".to_string(),
        TokenKind::LtEq => "「≦」".to_string(),
        TokenKind::GtEq => "「≧」".to_string(),
        TokenKind::NotEq => "「≠」".to_string(),
        TokenKind::Semi => "「；」".to_string(),
        TokenKind::Plus => "「＋」".to_string(),
        TokenKind::Minus => "「ー」".to_string(),
        TokenKind::Star => "「＊」".to_string(),
        TokenKind::Slash => "「／」".to_string(),
        TokenKind::Percent => "「％」".to_string(),
        TokenKind::LBrace => "「｛」".to_string(),
        TokenKind::RBrace => "「｝」".to_string(),
        TokenKind::LParen => "「（」".to_string(),
        TokenKind::RParen => "「）」".to_string(),
        TokenKind::Comma => "「、」".to_string(),
        TokenKind::Arrow => "「ー＞」".to_string(),
        TokenKind::LBracket => "「【」".to_string(),
        TokenKind::RBracket => "「】」".to_string(),
        TokenKind::Colon => "「：」".to_string(),
        TokenKind::DoubleColon => "「：：」".to_string(),
        TokenKind::Pipe => "「｜」".to_string(),
        TokenKind::Ident(name) => format!("識別子「{}」", name),
        TokenKind::Invalid(text) => format!("不正な字句「{}」", text),
        TokenKind::Eof => "ファイルの末尾".to_string(),
    }
}

pub fn hikari_type_japanese(ty: &HikariType) -> String {
    match ty {
        HikariType::Int => "整数".to_string(),
        HikariType::Float => "小数".to_string(),
        HikariType::String => "文字列".to_string(),
        HikariType::Bool => "真偽".to_string(),
        HikariType::Void => "無".to_string(),
        HikariType::Array(inner) => format!("{}列", hikari_type_japanese(inner)),
        HikariType::Map(k, v) => {
            format!(
                "辞書＜{}、{}＞",
                hikari_type_japanese(k),
                hikari_type_japanese(v)
            )
        }
        HikariType::Record(name) => name.clone(),
        HikariType::Enum(name) => name.clone(),
        HikariType::Fn(params, ret) => {
            let param_strs: Vec<String> = params.iter().map(hikari_type_japanese).collect();
            format!(
                "関数＜({}) → {}＞",
                param_strs.join("、"),
                hikari_type_japanese(ret)
            )
        }
    }
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParseError::UnexpectedToken { expected, got, .. } => {
                write!(
                    f,
                    "{}が必要ですが、{}が見つかりました。",
                    token_kind_japanese(expected),
                    token_kind_japanese(got)
                )?;
                if *expected == TokenKind::Semi {
                    write!(f, "（ヒント: 文の終わりに「；」を追加してください）")?;
                }
                Ok(())
            }
            ParseError::ExpectedIdentifier { got, .. } => {
                write!(
                    f,
                    "識別子（名前）が必要ですが、{}が見つかりました。",
                    token_kind_japanese(got)
                )
            }
            ParseError::ExpectedType { got, .. } => {
                write!(
                    f,
                    "型（整数・小数・文字列・真偽・無のいずれか）が必要ですが、{}が見つかりました。",
                    token_kind_japanese(got)
                )
            }
            ParseError::UnexpectedExprToken { got, .. } => {
                write!(
                    f,
                    "式が必要な位置に{}が見つかりました。",
                    token_kind_japanese(got)
                )
            }
            ParseError::InvalidNumber { text, .. } => {
                write!(f, "「{}」は正しい数値ではありません。", text)
            }
        }
    }
}

fn is_type_token(kind: &TokenKind) -> bool {
    matches!(
        kind,
        TokenKind::TyInt
            | TokenKind::TyFloat
            | TokenKind::TyString
            | TokenKind::TyBool
            | TokenKind::TyVoid
            | TokenKind::TyIntArray
            | TokenKind::TyFloatArray
            | TokenKind::TyStringArray
            | TokenKind::TyBoolArray
            | TokenKind::KwMap
            | TokenKind::KwFn
    )
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::Lexer;

    #[test]
    fn test_parse_var_decl() {
        let tokens = Lexer::new("整数 年齢 ＝ ２０；").tokenize();
        let mut parser = Parser::new(tokens);
        let ast = parser.parse().unwrap();
        assert_eq!(ast.len(), 1);
        assert!(matches!(
            &ast[0],
            Stmt::VarDecl { ty: HikariType::Int, name, value: Expr::LitInt(20), .. }
            if name == "年齢"
        ));
    }

    #[test]
    fn test_parse_binary_expression() {
        // 整数 結果 ＝ １ ＋ ２；
        let tokens = Lexer::new("整数 結果 ＝ １ ＋ ２；").tokenize();
        let ast = Parser::new(tokens).parse().unwrap();
        assert!(matches!(
            &ast[0],
            Stmt::VarDecl {
                ty: HikariType::Int,
                name,
                value: Expr::BinOp { op: BinOpKind::Add, lhs, rhs },
                ..
            }
            if name == "結果"
                && matches!(lhs.as_ref(), Expr::LitInt(1))
                && matches!(rhs.as_ref(), Expr::LitInt(2))
        ));
    }

    #[test]
    fn test_parse_operator_precedence() {
        // 整数 結果 ＝ ２ ＋ ３ ＊ ４；
        // Should parse as 2 + (3 * 4), not (2 + 3) * 4
        let tokens = Lexer::new("整数 結果 ＝ ２ ＋ ３ ＊ ４；").tokenize();
        let ast = Parser::new(tokens).parse().unwrap();
        let Stmt::VarDecl { value, .. } = &ast[0] else {
            panic!()
        };
        // outer op must be Add
        let Expr::BinOp { op, lhs, rhs } = value else {
            panic!()
        };
        assert_eq!(op, &BinOpKind::Add);
        assert!(matches!(lhs.as_ref(), Expr::LitInt(2)));
        // rhs must be Mul(3, 4)
        let Expr::BinOp {
            op: inner_op,
            lhs: il,
            rhs: ir,
        } = rhs.as_ref()
        else {
            panic!()
        };
        assert_eq!(inner_op, &BinOpKind::Mul);
        assert!(matches!(il.as_ref(), Expr::LitInt(3)));
        assert!(matches!(ir.as_ref(), Expr::LitInt(4)));
    }

    #[test]
    fn test_parse_return_stmt() {
        // 返す 年齢 ＋ １；
        let tokens = Lexer::new("返す 年齢 ＋ １；").tokenize();
        let ast = Parser::new(tokens).parse().unwrap();
        assert!(matches!(
            &ast[0],
            Stmt::Return(Some(Expr::BinOp { op: BinOpKind::Add, lhs, rhs }), _)
            if matches!(lhs.as_ref(), Expr::Ident(n) if n == "年齢")
                && matches!(rhs.as_ref(), Expr::LitInt(1))
        ));
    }

    #[test]
    fn test_parse_fn_decl() {
        // 関数 計算（整数 Ａ）ー＞ 整数 ｛ 返す Ａ ＋ １； ｝
        let src = "関数 計算（整数 Ａ）ー＞ 整数 ｛ 返す Ａ ＋ １； ｝";
        let tokens = Lexer::new(src).tokenize();
        let ast = Parser::new(tokens).parse().unwrap();
        assert_eq!(ast.len(), 1);
        let Stmt::FnDecl {
            name,
            params,
            return_ty,
            body,
            ..
        } = &ast[0]
        else {
            panic!("expected FnDecl")
        };
        assert_eq!(name, "計算");
        assert_eq!(params, &[(HikariType::Int, "Ａ".to_string())]);
        assert_eq!(return_ty, &HikariType::Int);
        assert_eq!(body.len(), 1);
        assert!(matches!(
            &body[0],
            Stmt::Return(
                Some(Expr::BinOp {
                    op: BinOpKind::Add,
                    ..
                }),
                _
            )
        ));
    }

    #[test]
    fn test_parse_if_stmt() {
        // もし １ ＝＝ １ ならば ｛ 印刷（１）； ｝
        let src = "もし １ ＝＝ １ ならば ｛ 印刷（１）； ｝";
        let ast = Parser::new(Lexer::new(src).tokenize()).parse().unwrap();
        assert_eq!(ast.len(), 1);
        let Stmt::If {
            condition,
            then_body,
            else_body,
            ..
        } = &ast[0]
        else {
            panic!("expected If stmt")
        };
        assert!(matches!(
            condition,
            Expr::BinOp {
                op: BinOpKind::Eq,
                ..
            }
        ));
        assert_eq!(then_body.len(), 1);
        assert!(matches!(then_body[0], Stmt::Print(_, _)));
        assert!(else_body.is_none());
    }

    #[test]
    fn test_parse_if_else_stmt() {
        // もし Ａ ＝＝ ０ ならば ｛ 印刷（１）； ｝ 違えば ｛ 印刷（２）； ｝
        let src = "もし Ａ ＝＝ ０ ならば ｛ 印刷（１）； ｝ 違えば ｛ 印刷（２）； ｝";
        let ast = Parser::new(Lexer::new(src).tokenize()).parse().unwrap();
        let Stmt::If { else_body, .. } = &ast[0] else {
            panic!()
        };
        assert!(else_body.is_some());
        assert_eq!(else_body.as_ref().unwrap().len(), 1);
    }

    #[test]
    fn test_parse_while_stmt() {
        // 間 カウンタ ＜ ３ ならば ｛ 印刷（カウンタ）； ｝
        let src = "間 カウンタ ＜ ３ ならば ｛ 印刷（カウンタ）； ｝";
        let ast = Parser::new(Lexer::new(src).tokenize()).parse().unwrap();
        assert_eq!(ast.len(), 1);
        let Stmt::While {
            condition, body, ..
        } = &ast[0]
        else {
            panic!("expected While stmt")
        };
        assert!(matches!(
            condition,
            Expr::BinOp {
                op: BinOpKind::Lt,
                ..
            }
        ));
        assert_eq!(body.len(), 1);
        assert!(matches!(body[0], Stmt::Print(_, _)));
    }

    #[test]
    fn test_parse_bool_literals() {
        // 真偽 フラグ ＝ 真；
        let tokens = Lexer::new("真偽 フラグ ＝ 真；").tokenize();
        let ast = Parser::new(tokens).parse().unwrap();
        assert!(matches!(
            &ast[0],
            Stmt::VarDecl {
                ty: HikariType::Bool,
                value: Expr::LitBool(true),
                ..
            }
        ));

        // 真偽 フラグ ＝ 偽；
        let tokens = Lexer::new("真偽 フラグ ＝ 偽；").tokenize();
        let ast = Parser::new(tokens).parse().unwrap();
        assert!(matches!(
            &ast[0],
            Stmt::VarDecl {
                ty: HikariType::Bool,
                value: Expr::LitBool(false),
                ..
            }
        ));
    }

    #[test]
    fn test_parse_print_stmt() {
        // 印刷（年齢）；
        let tokens = Lexer::new("印刷（年齢）；").tokenize();
        let ast = Parser::new(tokens).parse().unwrap();
        assert_eq!(ast.len(), 1);
        assert!(matches!(
            &ast[0],
            Stmt::Print(Expr::Ident(n), _) if n == "年齢"
        ));
    }

    #[test]
    fn test_parse_missing_semicolon_returns_error() {
        // 整数 年齢 ＝ ２０  （no trailing ；）
        let tokens = Lexer::new("整数 年齢 ＝ ２０").tokenize();
        let err = Parser::new(tokens).parse().unwrap_err();
        assert!(matches!(
            err,
            ParseError::UnexpectedToken {
                expected: TokenKind::Semi,
                got: TokenKind::Eof,
                ..
            }
        ));
    }

    #[test]
    fn test_parse_missing_identifier_returns_error() {
        // 整数 ＝ ２０；  （missing variable name）
        let tokens = Lexer::new("整数 ＝ ２０；").tokenize();
        let err = Parser::new(tokens).parse().unwrap_err();
        assert!(matches!(
            err,
            ParseError::ExpectedIdentifier {
                got: TokenKind::Assign,
                ..
            }
        ));
    }

    #[test]
    fn test_parse_reassignment() {
        // 整数 年齢 ＝ ２０； 年齢 ＝ ３０；
        let tokens = Lexer::new("整数 年齢 ＝ ２０；年齢 ＝ ３０；").tokenize();
        let ast = Parser::new(tokens).parse().unwrap();
        assert_eq!(ast.len(), 2);
        assert!(matches!(
            &ast[1],
            Stmt::Assign { name, value: Expr::LitInt(30), .. } if name == "年齢"
        ));
    }

    #[test]
    fn test_parse_multi_param_fn_decl() {
        // 関数 加算（整数 Ａ、整数 Ｂ）ー＞ 整数 ｛ 返す Ａ ＋ Ｂ； ｝
        let src = "関数 加算（整数 Ａ、整数 Ｂ）ー＞ 整数 ｛ 返す Ａ ＋ Ｂ； ｝";
        let ast = Parser::new(Lexer::new(src).tokenize()).parse().unwrap();
        let Stmt::FnDecl { params, .. } = &ast[0] else {
            panic!("expected FnDecl")
        };
        assert_eq!(
            params,
            &[
                (HikariType::Int, "Ａ".to_string()),
                (HikariType::Int, "Ｂ".to_string())
            ]
        );
    }

    #[test]
    fn test_parse_multi_arg_call() {
        // 関数 加算（整数 Ａ、整数 Ｂ）ー＞ 整数 ｛ 返す Ａ ＋ Ｂ； ｝
        // 返す 加算（１、２）；
        let src =
            "関数 加算（整数 Ａ、整数 Ｂ）ー＞ 整数 ｛ 返す Ａ ＋ Ｂ； ｝返す 加算（１、２）；";
        let ast = Parser::new(Lexer::new(src).tokenize()).parse().unwrap();
        let Stmt::Return(Some(Expr::Call { name, args }), _) = &ast[1] else {
            panic!("expected Return(Call)")
        };
        assert_eq!(name, "加算");
        assert_eq!(args.len(), 2);
    }

    #[test]
    fn test_parse_unary_minus() {
        // 整数 結果 ＝ ー５；
        let tokens = Lexer::new("整数 結果 ＝ ー５；").tokenize();
        let ast = Parser::new(tokens).parse().unwrap();
        assert!(matches!(
            &ast[0],
            Stmt::VarDecl { value: Expr::UnaryMinus(inner), .. }
            if matches!(inner.as_ref(), Expr::LitInt(5))
        ));
    }

    #[test]
    fn test_parse_unary_minus_in_addition() {
        // 整数 結果 ＝ １０ ＋ ー３；
        let tokens = Lexer::new("整数 結果 ＝ １０ ＋ ー３；").tokenize();
        let ast = Parser::new(tokens).parse().unwrap();
        let Stmt::VarDecl { value, .. } = &ast[0] else {
            panic!()
        };
        let Expr::BinOp { op, rhs, .. } = value else {
            panic!()
        };
        assert_eq!(op, &BinOpKind::Add);
        assert!(matches!(rhs.as_ref(), Expr::UnaryMinus(_)));
    }

    #[test]
    fn test_parse_logical_and_or_precedence() {
        // １ ＝＝ １ かつ ２ ＝＝ ２
        let tokens = Lexer::new("返す １ ＝＝ １ かつ ２ ＝＝ ２；").tokenize();
        let ast = Parser::new(tokens).parse().unwrap();
        let Stmt::Return(Some(expr), _) = &ast[0] else {
            panic!()
        };
        let Expr::BinOp { op, lhs, rhs } = expr else {
            panic!()
        };
        assert_eq!(op, &BinOpKind::And);
        assert!(matches!(
            lhs.as_ref(),
            Expr::BinOp {
                op: BinOpKind::Eq,
                ..
            }
        ));
        assert!(matches!(
            rhs.as_ref(),
            Expr::BinOp {
                op: BinOpKind::Eq,
                ..
            }
        ));
    }

    #[test]
    fn test_parse_unary_not() {
        // 返す 否定 真；
        let tokens = Lexer::new("返す 否定 真；").tokenize();
        let ast = Parser::new(tokens).parse().unwrap();
        assert!(matches!(
            &ast[0],
            Stmt::Return(Some(Expr::UnaryNot(inner)), _) if matches!(inner.as_ref(), Expr::LitBool(true))
        ));
    }

    #[test]
    fn test_parse_additional_comparison_operators() {
        let tokens = Lexer::new("返す ３ ≦ ３；").tokenize();
        let ast = Parser::new(tokens).parse().unwrap();
        assert!(matches!(
            &ast[0],
            Stmt::Return(
                Some(Expr::BinOp {
                    op: BinOpKind::LtEq,
                    ..
                }),
                _
            )
        ));
    }

    #[test]
    fn test_parse_unexpected_expr_token_returns_error() {
        // 整数 結果 ＝ ；  （missing expression before semicolon）
        let tokens = Lexer::new("整数 結果 ＝ ；").tokenize();
        let err = Parser::new(tokens).parse().unwrap_err();
        assert!(matches!(
            err,
            ParseError::UnexpectedExprToken {
                got: TokenKind::Semi,
                ..
            }
        ));
    }

    #[test]
    fn test_parse_array_literal() {
        let tokens = Lexer::new("整数列 数字 ＝ 【１、２、３】；").tokenize();
        let ast = Parser::new(tokens).parse().unwrap();
        assert!(matches!(
            &ast[0],
            Stmt::VarDecl {
                ty: HikariType::Array(inner),
                value: Expr::Array(elems),
                ..
            }
            if **inner == HikariType::Int && elems.len() == 3
        ));
    }

    #[test]
    fn test_parse_index_expr() {
        let tokens = Lexer::new("返す 数字【１】；").tokenize();
        let ast = Parser::new(tokens).parse().unwrap();
        assert!(matches!(
            &ast[0],
            Stmt::Return(Some(Expr::Index { array, index }), _)
            if matches!(array.as_ref(), Expr::Ident(n) if n == "数字")
                && matches!(index.as_ref(), Expr::LitInt(1))
        ));
    }

    #[test]
    fn test_parse_index_assign() {
        let tokens = Lexer::new("数字【０】＝ ９９；").tokenize();
        let ast = Parser::new(tokens).parse().unwrap();
        assert!(matches!(
            &ast[0],
            Stmt::IndexAssign { name, index: Expr::LitInt(0), value: Expr::LitInt(99), .. }
            if name == "数字"
        ));
    }

    #[test]
    fn test_parse_for_range_stmt() {
        let src = "繰り返す カウンタ ＝ ０ から ５ ならば ｛ 印刷（カウンタ）； ｝";
        let ast = Parser::new(Lexer::new(src).tokenize()).parse().unwrap();
        assert!(matches!(
            &ast[0],
            Stmt::ForRange { var, from: Expr::LitInt(0), to: Expr::LitInt(5), body, .. }
            if var == "カウンタ" && body.len() == 1
        ));
    }

    #[test]
    fn test_parse_for_each_stmt() {
        let src = "各 要素 ： 数字 ならば ｛ 印刷（要素）； ｝";
        let ast = Parser::new(Lexer::new(src).tokenize()).parse().unwrap();
        assert!(matches!(
            &ast[0],
            Stmt::ForEach { var, array: Expr::Ident(arr_name), body, .. }
            if var == "要素" && arr_name == "数字" && body.len() == 1
        ));
    }

    #[test]
    fn test_parse_try_catch_stmt() {
        let src = "試す ｛ 印刷（１）； ｝ 失敗 失敗内容 ｛ 印刷（失敗内容）； ｝";
        let ast = Parser::new(Lexer::new(src).tokenize()).parse().unwrap();
        assert!(matches!(
            &ast[0],
            Stmt::TryCatch { try_body, error_var, catch_body, .. }
            if try_body.len() == 1 && error_var == "失敗内容" && catch_body.len() == 1
        ));
    }

    #[test]
    fn test_parse_try_catch_missing_error_var_returns_error() {
        let src = "試す ｛ 印刷（１）； ｝ 失敗 ｛ 印刷（１）； ｝";
        let tokens = Lexer::new(src).tokenize();
        let err = Parser::new(tokens).parse().unwrap_err();
        assert!(matches!(err, ParseError::ExpectedIdentifier { .. }));
    }

    #[test]
    fn test_parse_import_stmt() {
        let src = "取り込む 「数学」；";
        let ast = Parser::new(Lexer::new(src).tokenize()).parse().unwrap();
        assert!(matches!(
            &ast[0],
            Stmt::Import { name, .. } if name == "数学"
        ));
    }

    #[test]
    fn test_parse_import_missing_string_literal_returns_error() {
        let src = "取り込む 数学；";
        let tokens = Lexer::new(src).tokenize();
        let err = Parser::new(tokens).parse().unwrap_err();
        assert!(matches!(err, ParseError::UnexpectedToken { .. }));
    }

    #[test]
    fn test_parse_import_missing_semi_returns_error() {
        let src = "取り込む 「数学」";
        let tokens = Lexer::new(src).tokenize();
        let err = Parser::new(tokens).parse().unwrap_err();
        assert!(matches!(
            err,
            ParseError::UnexpectedToken {
                expected: TokenKind::Semi,
                ..
            }
        ));
    }

    #[test]
    fn test_parse_malformed_number_returns_invalid_number_error() {
        let src = "整数 Ｘ ＝ １．２．３；";
        let tokens = Lexer::new(src).tokenize();
        let err = Parser::new(tokens).parse().unwrap_err();
        assert!(matches!(err, ParseError::InvalidNumber { .. }));
    }

    #[test]
    fn test_parse_modulo_precedence() {
        // １０ ％ ３ ＋ １ should parse as (10 % 3) + 1, Mod binding tighter than Add.
        let tokens = Lexer::new("返す １０ ％ ３ ＋ １；").tokenize();
        let ast = Parser::new(tokens).parse().unwrap();
        let Stmt::Return(Some(expr), _) = &ast[0] else {
            panic!()
        };
        let Expr::BinOp { op, lhs, rhs } = expr else {
            panic!()
        };
        assert_eq!(op, &BinOpKind::Add);
        assert!(matches!(rhs.as_ref(), Expr::LitInt(1)));
        let Expr::BinOp {
            op: inner_op,
            lhs: il,
            rhs: ir,
        } = lhs.as_ref()
        else {
            panic!()
        };
        assert_eq!(inner_op, &BinOpKind::Mod);
        assert!(matches!(il.as_ref(), Expr::LitInt(10)));
        assert!(matches!(ir.as_ref(), Expr::LitInt(3)));
    }

    #[test]
    fn test_parse_break_stmt() {
        let ast = parse_helper("間 真 ならば ｛ 抜ける； ｝");
        let Stmt::While { body, .. } = &ast[0] else {
            panic!()
        };
        assert!(matches!(body[0], Stmt::Break(_)));
    }

    #[test]
    fn test_parse_continue_stmt() {
        let ast = parse_helper("間 真 ならば ｛ 続ける； ｝");
        let Stmt::While { body, .. } = &ast[0] else {
            panic!()
        };
        assert!(matches!(body[0], Stmt::Continue(_)));
    }

    #[test]
    fn test_parse_bare_return() {
        let ast = parse_helper("返す；");
        assert!(matches!(ast[0], Stmt::Return(None, _)));
    }

    #[test]
    fn test_parse_return_with_expr_still_works() {
        let ast = parse_helper("返す ５；");
        assert!(matches!(ast[0], Stmt::Return(Some(Expr::LitInt(5)), _)));
    }

    fn parse_helper(src: &str) -> Vec<Stmt> {
        Parser::new(Lexer::new(src).tokenize()).parse().unwrap()
    }

    // ── 9a: records ───────────────────────────────────────────────────────

    #[test]
    fn test_parse_type_decl() {
        let src = "型 点 ｛ 整数 ｘ； 整数 ｙ； ｝";
        let ast = parse_helper(src);
        assert!(matches!(
            &ast[0],
            Stmt::TypeDecl { name, fields, .. }
            if name == "点" && fields == &[
                (HikariType::Int, "ｘ".to_string()),
                (HikariType::Int, "ｙ".to_string()),
            ]
        ));
    }

    #[test]
    fn test_parse_record_construction() {
        let src = "点 ｐ ＝ 点 ｛ ｘ：１、ｙ：２ ｝；";
        let ast = parse_helper(src);
        let Stmt::VarDecl {
            ty: HikariType::Record(tyname),
            value: Expr::RecordLit { type_name, fields },
            ..
        } = &ast[0]
        else {
            panic!("expected VarDecl with RecordLit")
        };
        assert_eq!(tyname, "点");
        assert_eq!(type_name, "点");
        assert_eq!(fields.len(), 2);
        assert_eq!(fields[0].0, "ｘ");
        assert_eq!(fields[1].0, "ｙ");
    }

    #[test]
    fn test_parse_field_access() {
        let ast = parse_helper("返す ｐ：：ｘ；");
        assert!(matches!(
            &ast[0],
            Stmt::Return(Some(Expr::FieldAccess { record, field }), _)
            if matches!(record.as_ref(), Expr::Ident(n) if n == "ｐ") && field == "ｘ"
        ));
    }

    #[test]
    fn test_parse_chained_field_access() {
        let ast = parse_helper("返す ａ：：ｂ：：ｃ；");
        let Stmt::Return(Some(Expr::FieldAccess { record, field }), _) = &ast[0] else {
            panic!("expected FieldAccess")
        };
        assert_eq!(field, "ｃ");
        let Expr::FieldAccess {
            record: inner_record,
            field: inner_field,
        } = record.as_ref()
        else {
            panic!("expected nested FieldAccess")
        };
        assert_eq!(inner_field, "ｂ");
        assert!(matches!(inner_record.as_ref(), Expr::Ident(n) if n == "ａ"));
    }

    #[test]
    fn test_parse_field_assignment() {
        let ast = parse_helper("ｐ：：ｘ ＝ ９９；");
        assert!(matches!(
            &ast[0],
            Stmt::FieldAssign { record: Expr::Ident(n), field, value: Expr::LitInt(99), .. }
            if n == "ｐ" && field == "ｘ"
        ));
    }

    #[test]
    fn test_parse_index_then_field_access_chain() {
        let ast = parse_helper("返す 配列【０】：：ｘ；");
        let Stmt::Return(Some(Expr::FieldAccess { record, field }), _) = &ast[0] else {
            panic!("expected FieldAccess")
        };
        assert_eq!(field, "ｘ");
        assert!(matches!(record.as_ref(), Expr::Index { .. }));
    }

    #[test]
    fn test_parse_field_then_index_access_chain() {
        let ast = parse_helper("返す 点：：配列フィールド【０】；");
        let Stmt::Return(Some(Expr::Index { array, .. }), _) = &ast[0] else {
            panic!("expected Index")
        };
        assert!(matches!(array.as_ref(), Expr::FieldAccess { .. }));
    }

    #[test]
    fn test_parse_type_decl_missing_semi_returns_error() {
        let src = "型 点 ｛ 整数 ｘ ｝";
        let tokens = Lexer::new(src).tokenize();
        let err = Parser::new(tokens).parse().unwrap_err();
        assert!(matches!(err, ParseError::UnexpectedToken { .. }));
    }

    #[test]
    fn test_parse_record_construction_missing_colon_returns_error() {
        let src = "点 ｐ ＝ 点 ｛ ｘ １ ｝；";
        let tokens = Lexer::new(src).tokenize();
        let err = Parser::new(tokens).parse().unwrap_err();
        assert!(matches!(err, ParseError::UnexpectedToken { .. }));
    }

    #[test]
    fn test_parse_type_decl_missing_field_name_returns_error() {
        let src = "型 点 ｛ 整数； ｝";
        let tokens = Lexer::new(src).tokenize();
        let err = Parser::new(tokens).parse().unwrap_err();
        assert!(matches!(err, ParseError::ExpectedIdentifier { .. }));
    }

    // ── 9b: enums and pattern matching ──────────────────────────────────

    #[test]
    fn test_parse_enum_decl_with_payload_and_payloadless_variants() {
        let src = "構造 結果 ｛ 成功（整数）、 異常（文字列）、 不明 ｝";
        let ast = parse_helper(src);
        assert!(matches!(
            &ast[0],
            Stmt::EnumDecl { name, variants, .. }
            if name == "結果" && variants == &[
                ("成功".to_string(), vec![HikariType::Int]),
                ("異常".to_string(), vec![HikariType::String]),
                ("不明".to_string(), vec![]),
            ]
        ));
    }

    #[test]
    fn test_parse_match_stmt_with_multiple_arms_including_zero_payload() {
        let src = "構造 結果 ｛ 成功（整数）、 異常 ｝照合 値 ｛ 成功（ｎ） ならば ｛ 印刷（ｎ）； ｝ 異常（） ならば ｛ 印刷（０）； ｝ ｝";
        let ast = parse_helper(src);
        let Stmt::Match { subject, arms, .. } = &ast[1] else {
            panic!("expected Match")
        };
        assert!(matches!(subject, Expr::Ident(n) if n == "値"));
        assert_eq!(arms.len(), 2);
        assert_eq!(arms[0].variant, "成功");
        assert_eq!(arms[0].binders, vec!["ｎ".to_string()]);
        assert_eq!(arms[0].body.len(), 1);
        assert_eq!(arms[1].variant, "異常");
        assert!(arms[1].binders.is_empty());
    }

    #[test]
    fn test_parse_variant_construction_is_ordinary_call() {
        let ast = parse_helper("返す 成功（１２３）；");
        assert!(matches!(
            &ast[0],
            Stmt::Return(Some(Expr::Call { name, args }), _)
            if name == "成功" && args.len() == 1
        ));
    }

    #[test]
    fn test_parse_zero_payload_variant_construction_requires_parens() {
        let ast = parse_helper("返す 異常（）；");
        assert!(matches!(
            &ast[0],
            Stmt::Return(Some(Expr::Call { name, args }), _)
            if name == "異常" && args.is_empty()
        ));
    }

    #[test]
    fn test_parse_enum_decl_missing_comma_between_variants_returns_error() {
        let src = "構造 結果 ｛ 成功 異常 ｝";
        let tokens = Lexer::new(src).tokenize();
        let err = Parser::new(tokens).parse().unwrap_err();
        assert!(matches!(err, ParseError::UnexpectedToken { .. }));
    }

    #[test]
    fn test_parse_match_arm_missing_then_keyword_returns_error() {
        let src = "照合 値 ｛ 成功（） ｛ 印刷（０）； ｝ ｝";
        let tokens = Lexer::new(src).tokenize();
        let err = Parser::new(tokens).parse().unwrap_err();
        assert!(matches!(
            err,
            ParseError::UnexpectedToken {
                expected: TokenKind::KwThen,
                ..
            }
        ));
    }

    #[test]
    fn test_parse_match_arm_missing_parens_returns_error() {
        let src = "照合 値 ｛ 成功 ならば ｛ 印刷（０）； ｝ ｝";
        let tokens = Lexer::new(src).tokenize();
        let err = Parser::new(tokens).parse().unwrap_err();
        assert!(matches!(
            err,
            ParseError::UnexpectedToken {
                expected: TokenKind::LParen,
                ..
            }
        ));
    }

    #[test]
    fn test_parse_new_array_expr() {
        let tokens = Lexer::new("整数列 数字 ＝ 新配列＜整数＞；").tokenize();
        let ast = Parser::new(tokens).parse().unwrap();
        assert!(matches!(
            &ast[0],
            Stmt::VarDecl {
                ty: HikariType::Array(inner),
                value: Expr::NewArray(elem_ty),
                ..
            }
            if **inner == HikariType::Int && *elem_ty == HikariType::Int
        ));
    }
}
