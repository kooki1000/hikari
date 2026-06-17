use crate::lexer::{Span, Token, TokenKind};

use super::ast::{BinOpKind, Expr, HikariType, MatchArm, Stmt};
use super::error::ParseError;

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
            // 関数＜...＞ name ＝ expr;  is a var decl with Fn type.
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
        // lambda — ｜ param：type、...｜ → return_ty ｛ body ｝
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
            // 関数＜(T1、T2) → R＞
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
