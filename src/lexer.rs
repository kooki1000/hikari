#[derive(Debug, PartialEq, Clone)]
pub enum TokenKind {
    // Type keywords
    TyInt,    // 整数
    TyFloat,  // 小数
    TyString, // 文字列
    TyBool,   // 真偽
    TyVoid,   // 無

    // Statement keywords
    KwFn,     // 関数
    KwReturn, // 返す

    // Literals
    LitInt(i64),
    LitFloat(f64),
    LitString(String),
    LitTrue,  // 真
    LitFalse, // 偽

    // Operators & punctuation
    Assign, // ＝
    Semi,   // ；
    Plus,   // ＋
    Minus,  // ー  (also prefix of arrow)
    Star,   // ＊
    Slash,  // ／
    LBrace, // ｛
    RBrace, // ｝
    LParen, // （
    RParen, // ）
    Arrow,  // ー＞

    // Identifier (user-defined name)
    Ident(String),

    Eof,
}

#[derive(Debug, Clone)]
pub struct Token {
    pub kind: TokenKind,
}

pub struct Lexer {
    source: Vec<char>,
    pos: usize,
}

impl Lexer {
    pub fn new(src: &str) -> Self {
        Self {
            source: src.chars().collect(),
            pos: 0,
        }
    }

    fn peek(&self) -> Option<char> {
        self.source.get(self.pos).copied()
    }

    fn advance(&mut self) -> Option<char> {
        let ch = self.source.get(self.pos).copied();
        self.pos += 1;
        ch
    }

    fn skip_whitespace(&mut self) {
        while self.peek().map(|c| c.is_whitespace()).unwrap_or(false) {
            self.advance();
        }
    }

    fn read_word(&mut self) -> String {
        let mut word = String::new();
        while let Some(ch) = self.peek() {
            if ch.is_whitespace() || is_symbol(ch) {
                break;
            }
            word.push(ch);
            self.advance();
        }
        word
    }

    fn read_string_literal(&mut self) -> String {
        // Opening 「 already consumed; read until 」.
        let mut s = String::new();
        while let Some(ch) = self.advance() {
            if ch == '」' {
                break;
            }
            s.push(ch);
        }
        s
    }

    fn read_number(&mut self, first: char) -> TokenKind {
        let mut s = String::new();
        s.push(fw_digit_to_ascii(first));
        let mut is_float = false;
        while let Some(ch) = self.peek() {
            if is_fullwidth_digit(ch) {
                s.push(fw_digit_to_ascii(ch));
                self.advance();
            } else if ch == '．' {
                is_float = true;
                s.push('.');
                self.advance();
            } else {
                break;
            }
        }
        if is_float {
            TokenKind::LitFloat(s.parse().unwrap())
        } else {
            TokenKind::LitInt(s.parse().unwrap())
        }
    }

    fn keyword_or_ident(word: &str) -> TokenKind {
        match word {
            "整数" => TokenKind::TyInt,
            "小数" => TokenKind::TyFloat,
            "文字列" => TokenKind::TyString,
            "真偽" => TokenKind::TyBool,
            "無" => TokenKind::TyVoid,
            "関数" => TokenKind::KwFn,
            "返す" => TokenKind::KwReturn,
            "真" => TokenKind::LitTrue,
            "偽" => TokenKind::LitFalse,
            other => TokenKind::Ident(other.to_string()),
        }
    }

    pub fn tokenize(&mut self) -> Vec<Token> {
        let mut tokens = Vec::new();
        loop {
            self.skip_whitespace();
            let Some(ch) = self.peek() else {
                tokens.push(Token {
                    kind: TokenKind::Eof,
                });
                break;
            };

            let kind = match ch {
                'ー' => {
                    self.advance();
                    if self.peek() == Some('＞') {
                        self.advance();
                        TokenKind::Arrow
                    } else {
                        TokenKind::Minus
                    }
                }
                '＝' => {
                    self.advance();
                    TokenKind::Assign
                }
                '；' => {
                    self.advance();
                    TokenKind::Semi
                }
                '＋' => {
                    self.advance();
                    TokenKind::Plus
                }
                '＊' => {
                    self.advance();
                    TokenKind::Star
                }
                '／' => {
                    self.advance();
                    TokenKind::Slash
                }
                '｛' => {
                    self.advance();
                    TokenKind::LBrace
                }
                '｝' => {
                    self.advance();
                    TokenKind::RBrace
                }
                '（' => {
                    self.advance();
                    TokenKind::LParen
                }
                '）' => {
                    self.advance();
                    TokenKind::RParen
                }
                '「' => {
                    self.advance();
                    TokenKind::LitString(self.read_string_literal())
                }
                c if is_fullwidth_digit(c) => {
                    self.advance();
                    self.read_number(c)
                }
                _ => {
                    let word = self.read_word();
                    if word.is_empty() {
                        self.advance(); // skip unrecognised char
                        continue;
                    }
                    Self::keyword_or_ident(&word)
                }
            };

            tokens.push(Token { kind });
        }
        tokens
    }
}

fn is_fullwidth_digit(ch: char) -> bool {
    ('\u{FF10}'..='\u{FF19}').contains(&ch)
}

fn fw_digit_to_ascii(ch: char) -> char {
    char::from_u32(ch as u32 - 0xFF10 + b'0' as u32).unwrap()
}

// Returns true for full-width punctuation that acts as a token boundary.
fn is_symbol(ch: char) -> bool {
    matches!(
        ch,
        '＝' | '；' | '＋' | 'ー' | '＊' | '／' | '｛' | '｝' | '（' | '）' | '「' | '」'
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lex_integer_keyword() {
        let tokens = Lexer::new("整数").tokenize();
        assert_eq!(tokens.len(), 2);
        assert_eq!(tokens[0].kind, TokenKind::TyInt);
        assert_eq!(tokens[1].kind, TokenKind::Eof);
    }

    #[test]
    fn test_lex_all_keywords() {
        let src = "整数 小数 文字列 真偽 無 関数 返す";
        let tokens = Lexer::new(src).tokenize();
        let kinds: Vec<&TokenKind> = tokens.iter().map(|t| &t.kind).collect();
        assert_eq!(
            kinds,
            vec![
                &TokenKind::TyInt,
                &TokenKind::TyFloat,
                &TokenKind::TyString,
                &TokenKind::TyBool,
                &TokenKind::TyVoid,
                &TokenKind::KwFn,
                &TokenKind::KwReturn,
                &TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn test_lex_full_width_integer_literal() {
        let tokens = Lexer::new("２０").tokenize();
        assert_eq!(tokens[0].kind, TokenKind::LitInt(20));
        assert_eq!(tokens[1].kind, TokenKind::Eof);
    }

    #[test]
    fn test_lex_full_width_operators() {
        let src = "＋ ー ＊ ／ ＝ ；";
        let tokens = Lexer::new(src).tokenize();
        let kinds: Vec<&TokenKind> = tokens.iter().map(|t| &t.kind).collect();
        assert_eq!(
            kinds,
            vec![
                &TokenKind::Plus,
                &TokenKind::Minus,
                &TokenKind::Star,
                &TokenKind::Slash,
                &TokenKind::Assign,
                &TokenKind::Semi,
                &TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn test_lex_return_arrow() {
        // ー＞ must be a single Arrow token, not Minus followed by something.
        let tokens = Lexer::new("ー＞").tokenize();
        assert_eq!(tokens.len(), 2);
        assert_eq!(tokens[0].kind, TokenKind::Arrow);
        assert_eq!(tokens[1].kind, TokenKind::Eof);
    }

    #[test]
    fn test_lex_minus_not_arrow() {
        // A lone ー (not followed by ＞) must remain Minus.
        let tokens = Lexer::new("ー").tokenize();
        assert_eq!(tokens[0].kind, TokenKind::Minus);
    }

    #[test]
    fn test_lex_string_literal() {
        let tokens = Lexer::new("「こんにちは」").tokenize();
        assert_eq!(
            tokens[0].kind,
            TokenKind::LitString("こんにちは".to_string())
        );
        assert_eq!(tokens[1].kind, TokenKind::Eof);
    }

    #[test]
    fn test_lex_variable_declaration() {
        // 整数 年齢 ＝ ２０；
        let src = "整数 年齢 ＝ ２０；";
        let tokens = Lexer::new(src).tokenize();
        let kinds: Vec<TokenKind> = tokens.into_iter().map(|t| t.kind).collect();
        assert_eq!(
            kinds,
            vec![
                TokenKind::TyInt,
                TokenKind::Ident("年齢".to_string()),
                TokenKind::Assign,
                TokenKind::LitInt(20),
                TokenKind::Semi,
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn test_lex_function_declaration() {
        // 関数 計算（整数 Ａ）ー＞ 整数 ｛ 返す Ａ ＋ １； ｝
        let src = "関数 計算（整数 Ａ）ー＞ 整数 ｛ 返す Ａ ＋ １； ｝";
        let tokens = Lexer::new(src).tokenize();
        let kinds: Vec<TokenKind> = tokens.into_iter().map(|t| t.kind).collect();
        assert_eq!(
            kinds,
            vec![
                TokenKind::KwFn,
                TokenKind::Ident("計算".to_string()),
                TokenKind::LParen,
                TokenKind::TyInt,
                TokenKind::Ident("Ａ".to_string()),
                TokenKind::RParen,
                TokenKind::Arrow,
                TokenKind::TyInt,
                TokenKind::LBrace,
                TokenKind::KwReturn,
                TokenKind::Ident("Ａ".to_string()),
                TokenKind::Plus,
                TokenKind::LitInt(1),
                TokenKind::Semi,
                TokenKind::RBrace,
                TokenKind::Eof,
            ]
        );
    }
}
