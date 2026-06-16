use crate::lexer::{Token, TokenKind};

// ── Types ────────────────────────────────────────────────────────────────────

#[derive(Debug, PartialEq, Clone)]
pub enum HikariType {
    Int,    // 整数
    Float,  // 小数
    String, // 文字列
    Bool,   // 真偽
    Void,   // 無
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
}

#[derive(Debug, PartialEq, Clone)]
pub enum BinOpKind {
    Add, // ＋
    Sub, // ー
    Mul, // ＊
    Div, // ／
    Eq,  // ＝＝
    Lt,  // ＜
    Gt,  // ＞
}

#[derive(Debug, PartialEq, Clone)]
pub enum Stmt {
    VarDecl {
        ty: HikariType,
        name: String,
        value: Expr,
    },
    FnDecl {
        name: String,
        params: Vec<(HikariType, String)>,
        return_ty: HikariType,
        body: Vec<Stmt>,
    },
    Return(Expr),
    Print(Expr),
    If {
        condition: Expr,
        then_body: Vec<Stmt>,
        else_body: Option<Vec<Stmt>>,
    },
    While {
        condition: Expr,
        body: Vec<Stmt>,
    },
    ExprStmt(Expr),
}

// ── Error type ────────────────────────────────────────────────────────────────

#[derive(Debug, PartialEq)]
pub enum ParseError {
    UnexpectedToken { expected: TokenKind, got: TokenKind },
    ExpectedIdentifier(TokenKind),
    ExpectedType(TokenKind),
    UnexpectedExprToken(TokenKind),
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

    fn advance(&mut self) -> &TokenKind {
        let kind = &self.tokens[self.pos].kind;
        if self.pos + 1 < self.tokens.len() {
            self.pos += 1;
        }
        kind
    }

    fn expect(&mut self, expected: &TokenKind) -> Result<(), ParseError> {
        let got = self.advance().clone();
        if std::mem::discriminant(&got) == std::mem::discriminant(expected) {
            Ok(())
        } else {
            Err(ParseError::UnexpectedToken {
                expected: expected.clone(),
                got,
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
            TokenKind::KwFn => self.parse_fn_decl(),
            TokenKind::KwReturn => self.parse_return(),
            TokenKind::KwPrint => self.parse_print(),
            TokenKind::KwIf => self.parse_if(),
            TokenKind::KwWhile => self.parse_while(),
            kind if is_type_token(&kind) => self.parse_var_decl(),
            _ => {
                let expr = self.parse_expr()?;
                self.expect(&TokenKind::Semi)?;
                Ok(Stmt::ExprStmt(expr))
            }
        }
    }

    fn parse_var_decl(&mut self) -> Result<Stmt, ParseError> {
        let ty = self.parse_type()?;
        let name = match self.advance().clone() {
            TokenKind::Ident(n) => n,
            other => return Err(ParseError::ExpectedIdentifier(other)),
        };
        self.expect(&TokenKind::Assign)?;
        let value = self.parse_expr()?;
        self.expect(&TokenKind::Semi)?;
        Ok(Stmt::VarDecl { ty, name, value })
    }

    fn parse_fn_decl(&mut self) -> Result<Stmt, ParseError> {
        self.advance(); // consume 関数
        let name = match self.advance().clone() {
            TokenKind::Ident(n) => n,
            other => return Err(ParseError::ExpectedIdentifier(other)),
        };
        self.expect(&TokenKind::LParen)?;
        let mut params = Vec::new();
        while self.peek() != &TokenKind::RParen {
            let ty = self.parse_type()?;
            let pname = match self.advance().clone() {
                TokenKind::Ident(n) => n,
                other => return Err(ParseError::ExpectedIdentifier(other)),
            };
            params.push((ty, pname));
            // future: handle comma-separated params
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
        })
    }

    fn parse_return(&mut self) -> Result<Stmt, ParseError> {
        self.advance(); // consume 返す
        let expr = self.parse_expr()?;
        self.expect(&TokenKind::Semi)?;
        Ok(Stmt::Return(expr))
    }

    fn parse_if(&mut self) -> Result<Stmt, ParseError> {
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
        })
    }

    fn parse_while(&mut self) -> Result<Stmt, ParseError> {
        self.advance(); // consume 間
        let condition = self.parse_expr()?;
        self.expect(&TokenKind::KwThen)?; // ならば
        self.expect(&TokenKind::LBrace)?;
        let mut body = Vec::new();
        while self.peek() != &TokenKind::RBrace {
            body.push(self.parse_stmt()?);
        }
        self.expect(&TokenKind::RBrace)?;
        Ok(Stmt::While { condition, body })
    }

    fn parse_print(&mut self) -> Result<Stmt, ParseError> {
        self.advance(); // consume 印刷
        self.expect(&TokenKind::LParen)?;
        let expr = self.parse_expr()?;
        self.expect(&TokenKind::RParen)?;
        self.expect(&TokenKind::Semi)?;
        Ok(Stmt::Print(expr))
    }

    fn parse_expr(&mut self) -> Result<Expr, ParseError> {
        self.parse_comparison()
    }

    // Comparison: lowest precedence  (＝＝  ＜  ＞)
    fn parse_comparison(&mut self) -> Result<Expr, ParseError> {
        let lhs = self.parse_additive()?;
        let op = match self.peek() {
            TokenKind::EqEq => BinOpKind::Eq,
            TokenKind::Lt => BinOpKind::Lt,
            TokenKind::Gt => BinOpKind::Gt,
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
        match self.advance().clone() {
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
                    }
                    self.advance(); // consume ）
                    Ok(Expr::Call { name, args })
                } else {
                    Ok(Expr::Ident(name))
                }
            }
            TokenKind::LParen => {
                let expr = self.parse_expr()?;
                self.expect(&TokenKind::RParen)?;
                Ok(expr)
            }
            other => Err(ParseError::UnexpectedExprToken(other)),
        }
    }

    fn parse_type(&mut self) -> Result<HikariType, ParseError> {
        match self.advance().clone() {
            TokenKind::TyInt => Ok(HikariType::Int),
            TokenKind::TyFloat => Ok(HikariType::Float),
            TokenKind::TyString => Ok(HikariType::String),
            TokenKind::TyBool => Ok(HikariType::Bool),
            TokenKind::TyVoid => Ok(HikariType::Void),
            other => Err(ParseError::ExpectedType(other)),
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
            Stmt::VarDecl { ty: HikariType::Int, name, value: Expr::LitInt(20) }
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
                value: Expr::BinOp { op: BinOpKind::Add, lhs, rhs }
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
            Stmt::Return(Expr::BinOp { op: BinOpKind::Add, lhs, rhs })
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
            Stmt::Return(Expr::BinOp {
                op: BinOpKind::Add,
                ..
            })
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
        assert!(matches!(then_body[0], Stmt::Print(_)));
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
        let Stmt::While { condition, body } = &ast[0] else {
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
        assert!(matches!(body[0], Stmt::Print(_)));
    }

    #[test]
    fn test_parse_print_stmt() {
        // 印刷（年齢）；
        let tokens = Lexer::new("印刷（年齢）；").tokenize();
        let ast = Parser::new(tokens).parse().unwrap();
        assert_eq!(ast.len(), 1);
        assert!(matches!(
            &ast[0],
            Stmt::Print(Expr::Ident(n)) if n == "年齢"
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
            ParseError::ExpectedIdentifier(TokenKind::Assign)
        ));
    }

    #[test]
    fn test_parse_unexpected_expr_token_returns_error() {
        // 整数 結果 ＝ ；  （missing expression before semicolon）
        let tokens = Lexer::new("整数 結果 ＝ ；").tokenize();
        let err = Parser::new(tokens).parse().unwrap_err();
        assert!(matches!(
            err,
            ParseError::UnexpectedExprToken(TokenKind::Semi)
        ));
    }
}
