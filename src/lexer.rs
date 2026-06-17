#[derive(Debug, PartialEq, Clone)]
pub enum TokenKind {
    // Type keywords
    TyInt,         // 整数
    TyFloat,       // 小数
    TyString,      // 文字列
    TyBool,        // 真偽
    TyVoid,        // 無
    TyIntArray,    // 整数列
    TyFloatArray,  // 小数列
    TyStringArray, // 文字列列
    TyBoolArray,   // 真偽列

    // Statement keywords
    KwFn,       // 関数
    KwReturn,   // 返す
    KwPrint,    // 印刷
    KwIf,       // もし
    KwThen,     // ならば
    KwElse,     // 違えば
    KwWhile,    // 間
    KwAnd,      // かつ
    KwOr,       // または
    KwNot,      // 否定
    KwForRange, // 繰り返す
    KwFrom,     // から
    KwEach,     // 各
    KwTry,      // 試す
    KwCatch,    // 失敗
    KwImport,   // 取り込む
    KwNewArray, // 新配列
    KwBreak,    // 抜ける
    KwContinue, // 続ける
    KwType,     // 型
    KwEnum,     // 列挙
    KwMatch,    // 照合

    // Literals
    LitInt(i64),
    LitFloat(f64),
    LitString(String),
    LitTrue,  // 真
    LitFalse, // 偽

    // Operators & punctuation
    Assign,      // ＝
    EqEq,        // ＝＝
    Lt,          // ＜
    Gt,          // ＞
    LtEq,        // ≦
    GtEq,        // ≧
    NotEq,       // ≠
    Semi,        // ；
    Plus,        // ＋
    Minus,       // ー  (also prefix of arrow)
    Star,        // ＊
    Slash,       // ／
    Percent,     // ％
    LBrace,      // ｛
    RBrace,      // ｝
    LParen,      // （
    RParen,      // ）
    Comma,       // 、
    Arrow,       // ー＞
    LBracket,    // 【
    RBracket,    // 】
    Colon,       // ：
    DoubleColon, // ：： (record field access)

    // Identifier (user-defined name)
    Ident(String),

    // A lexeme that could not be tokenized (e.g. a malformed or overflowing
    // number literal). Carries the offending text so the parser can report it.
    Invalid(String),

    Eof,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Span {
    pub line: usize,
    pub col: usize,
    pub len: usize,
}

#[derive(Debug, Clone)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}

pub struct Lexer {
    source: Vec<char>,
    pos: usize,
    line: usize,
    col: usize,
}

impl Lexer {
    pub fn new(src: &str) -> Self {
        Self {
            source: src.chars().collect(),
            pos: 0,
            line: 1,
            col: 1,
        }
    }

    fn peek(&self) -> Option<char> {
        self.source.get(self.pos).copied()
    }

    fn advance(&mut self) -> Option<char> {
        let ch = self.source.get(self.pos).copied();
        self.pos += 1;
        if let Some(c) = ch {
            if c == '\n' {
                self.line += 1;
                self.col = 1;
            } else {
                self.col += 1;
            }
        }
        ch
    }

    fn skip_whitespace(&mut self) {
        loop {
            while self.peek().map(|c| c.is_whitespace()).unwrap_or(false) {
                self.advance();
            }
            if self.peek() == Some('＃') {
                while let Some(c) = self.peek() {
                    if c == '\n' {
                        break;
                    }
                    self.advance();
                }
            } else {
                break;
            }
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
            match s.parse() {
                Ok(f) => TokenKind::LitFloat(f),
                Err(_) => TokenKind::Invalid(s),
            }
        } else {
            match s.parse() {
                Ok(n) => TokenKind::LitInt(n),
                Err(_) => TokenKind::Invalid(s),
            }
        }
    }

    fn keyword_or_ident(word: &str) -> TokenKind {
        match word {
            "整数" => TokenKind::TyInt,
            "小数" => TokenKind::TyFloat,
            "文字列" => TokenKind::TyString,
            "真偽" => TokenKind::TyBool,
            "無" => TokenKind::TyVoid,
            "整数列" => TokenKind::TyIntArray,
            "小数列" => TokenKind::TyFloatArray,
            "文字列列" => TokenKind::TyStringArray,
            "真偽列" => TokenKind::TyBoolArray,
            "関数" => TokenKind::KwFn,
            "返す" => TokenKind::KwReturn,
            "印刷" => TokenKind::KwPrint,
            "もし" => TokenKind::KwIf,
            "ならば" => TokenKind::KwThen,
            "違えば" => TokenKind::KwElse,
            "間" => TokenKind::KwWhile,
            "かつ" => TokenKind::KwAnd,
            "または" => TokenKind::KwOr,
            "否定" => TokenKind::KwNot,
            "繰り返す" => TokenKind::KwForRange,
            "から" => TokenKind::KwFrom,
            "各" => TokenKind::KwEach,
            "試す" => TokenKind::KwTry,
            "失敗" => TokenKind::KwCatch,
            "取り込む" => TokenKind::KwImport,
            "新配列" => TokenKind::KwNewArray,
            "抜ける" => TokenKind::KwBreak,
            "続ける" => TokenKind::KwContinue,
            "型" => TokenKind::KwType,
            "列挙" => TokenKind::KwEnum,
            "照合" => TokenKind::KwMatch,
            "真" => TokenKind::LitTrue,
            "偽" => TokenKind::LitFalse,
            other => TokenKind::Ident(other.to_string()),
        }
    }

    pub fn tokenize(&mut self) -> Vec<Token> {
        let mut tokens = Vec::new();
        loop {
            self.skip_whitespace();
            let start_line = self.line;
            let start_col = self.col;
            let Some(ch) = self.peek() else {
                tokens.push(Token {
                    kind: TokenKind::Eof,
                    span: Span {
                        line: start_line,
                        col: start_col,
                        len: 1,
                    },
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
                    if self.peek() == Some('＝') {
                        self.advance();
                        TokenKind::EqEq
                    } else {
                        TokenKind::Assign
                    }
                }
                '＜' => {
                    self.advance();
                    TokenKind::Lt
                }
                '＞' => {
                    self.advance();
                    TokenKind::Gt
                }
                '≦' => {
                    self.advance();
                    TokenKind::LtEq
                }
                '≧' => {
                    self.advance();
                    TokenKind::GtEq
                }
                '≠' => {
                    self.advance();
                    TokenKind::NotEq
                }
                '、' => {
                    self.advance();
                    TokenKind::Comma
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
                '％' => {
                    self.advance();
                    TokenKind::Percent
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
                '【' => {
                    self.advance();
                    TokenKind::LBracket
                }
                '】' => {
                    self.advance();
                    TokenKind::RBracket
                }
                '：' => {
                    self.advance();
                    if self.peek() == Some('：') {
                        self.advance();
                        TokenKind::DoubleColon
                    } else {
                        TokenKind::Colon
                    }
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

            let len = self.col.saturating_sub(start_col).max(1);
            tokens.push(Token {
                kind,
                span: Span {
                    line: start_line,
                    col: start_col,
                    len,
                },
            });
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
// 'ー' (katakana long vowel mark, e.g. in エラー/コード) is deliberately
// excluded: it's only a Minus/Arrow token when it STARTS a token (handled
// by the dispatch in tokenize()), not when it appears inside a word already
// being read by read_word().
fn is_symbol(ch: char) -> bool {
    matches!(
        ch,
        '＝' | '；'
            | '＋'
            | '＊'
            | '／'
            | '％'
            | '｛'
            | '｝'
            | '（'
            | '）'
            | '「'
            | '」'
            | '＜'
            | '＞'
            | '≦'
            | '≧'
            | '≠'
            | '、'
            | '【'
            | '】'
            | '：'
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
    fn test_lex_identifier_containing_long_vowel_mark() {
        // ー is a common katakana long vowel mark in loanwords (エラー =
        // "error"); it must stay part of the identifier when it's not the
        // first character of a token.
        let tokens = Lexer::new("エラー").tokenize();
        assert_eq!(tokens[0].kind, TokenKind::Ident("エラー".to_string()));
        assert_eq!(tokens[1].kind, TokenKind::Eof);
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

    #[test]
    fn test_lex_print_keyword() {
        let tokens = Lexer::new("印刷").tokenize();
        assert_eq!(tokens.len(), 2);
        assert_eq!(tokens[0].kind, TokenKind::KwPrint);
        assert_eq!(tokens[1].kind, TokenKind::Eof);
    }

    #[test]
    fn test_lex_if_keywords() {
        let src = "もし ならば 違えば";
        let tokens = Lexer::new(src).tokenize();
        let kinds: Vec<&TokenKind> = tokens.iter().map(|t| &t.kind).collect();
        assert_eq!(
            kinds,
            vec![
                &TokenKind::KwIf,
                &TokenKind::KwThen,
                &TokenKind::KwElse,
                &TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn test_lex_while_keyword() {
        let tokens = Lexer::new("間").tokenize();
        assert_eq!(tokens.len(), 2);
        assert_eq!(tokens[0].kind, TokenKind::KwWhile);
        assert_eq!(tokens[1].kind, TokenKind::Eof);
    }

    #[test]
    fn test_lex_comparison_operators() {
        let src = "＝＝ ＜ ＞";
        let tokens = Lexer::new(src).tokenize();
        let kinds: Vec<&TokenKind> = tokens.iter().map(|t| &t.kind).collect();
        assert_eq!(
            kinds,
            vec![
                &TokenKind::EqEq,
                &TokenKind::Lt,
                &TokenKind::Gt,
                &TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn test_lex_comment_skipped() {
        let src = "＃ これはコメントです\n整数 年齢 ＝ ２０；";
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
    fn test_lex_comment_at_eof_with_no_trailing_newline() {
        let tokens = Lexer::new("＃ comment only").tokenize();
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0].kind, TokenKind::Eof);
    }

    #[test]
    fn test_lex_comma_token() {
        let tokens = Lexer::new("、").tokenize();
        assert_eq!(tokens[0].kind, TokenKind::Comma);
        assert_eq!(tokens[1].kind, TokenKind::Eof);
    }

    #[test]
    fn test_lex_extended_comparison_operators() {
        let src = "≦ ≧ ≠";
        let tokens = Lexer::new(src).tokenize();
        let kinds: Vec<&TokenKind> = tokens.iter().map(|t| &t.kind).collect();
        assert_eq!(
            kinds,
            vec![
                &TokenKind::LtEq,
                &TokenKind::GtEq,
                &TokenKind::NotEq,
                &TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn test_lex_logical_keywords() {
        let src = "かつ または 否定";
        let tokens = Lexer::new(src).tokenize();
        let kinds: Vec<&TokenKind> = tokens.iter().map(|t| &t.kind).collect();
        assert_eq!(
            kinds,
            vec![
                &TokenKind::KwAnd,
                &TokenKind::KwOr,
                &TokenKind::KwNot,
                &TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn test_lex_array_type_keywords() {
        let src = "整数列 小数列 文字列列 真偽列";
        let tokens = Lexer::new(src).tokenize();
        let kinds: Vec<&TokenKind> = tokens.iter().map(|t| &t.kind).collect();
        assert_eq!(
            kinds,
            vec![
                &TokenKind::TyIntArray,
                &TokenKind::TyFloatArray,
                &TokenKind::TyStringArray,
                &TokenKind::TyBoolArray,
                &TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn test_lex_bracket_tokens() {
        let tokens = Lexer::new("【１、２】").tokenize();
        let kinds: Vec<TokenKind> = tokens.into_iter().map(|t| t.kind).collect();
        assert_eq!(
            kinds,
            vec![
                TokenKind::LBracket,
                TokenKind::LitInt(1),
                TokenKind::Comma,
                TokenKind::LitInt(2),
                TokenKind::RBracket,
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn test_lex_try_catch_keywords() {
        let src = "試す 失敗";
        let tokens = Lexer::new(src).tokenize();
        let kinds: Vec<&TokenKind> = tokens.iter().map(|t| &t.kind).collect();
        assert_eq!(
            kinds,
            vec![&TokenKind::KwTry, &TokenKind::KwCatch, &TokenKind::Eof,]
        );
    }

    #[test]
    fn test_lex_for_range_and_each_keywords() {
        let src = "繰り返す から 各 ：";
        let tokens = Lexer::new(src).tokenize();
        let kinds: Vec<&TokenKind> = tokens.iter().map(|t| &t.kind).collect();
        assert_eq!(
            kinds,
            vec![
                &TokenKind::KwForRange,
                &TokenKind::KwFrom,
                &TokenKind::KwEach,
                &TokenKind::Colon,
                &TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn test_lex_import_keyword() {
        let src = "取り込む 「数学」；";
        let tokens = Lexer::new(src).tokenize();
        let kinds: Vec<&TokenKind> = tokens.iter().map(|t| &t.kind).collect();
        assert_eq!(
            kinds,
            vec![
                &TokenKind::KwImport,
                &TokenKind::LitString("数学".to_string()),
                &TokenKind::Semi,
                &TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn test_lex_overflowing_integer_is_invalid_token() {
        // Too large for i64: a clean Invalid token rather than a panic.
        let tokens = Lexer::new("９９９９９９９９９９９９９９９９９９９９").tokenize();
        assert_eq!(
            tokens[0].kind,
            TokenKind::Invalid("99999999999999999999".to_string())
        );
    }

    #[test]
    fn test_lex_malformed_number_is_invalid_token() {
        // Two decimal points: an Invalid token rather than a panic.
        let tokens = Lexer::new("１．２．３").tokenize();
        assert_eq!(tokens[0].kind, TokenKind::Invalid("1.2.3".to_string()));
    }

    #[test]
    fn test_lex_percent_token() {
        let tokens = Lexer::new("１０ ％ ３").tokenize();
        let kinds: Vec<TokenKind> = tokens.into_iter().map(|t| t.kind).collect();
        assert_eq!(
            kinds,
            vec![
                TokenKind::LitInt(10),
                TokenKind::Percent,
                TokenKind::LitInt(3),
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn test_lex_break_continue_keywords() {
        let src = "抜ける 続ける";
        let tokens = Lexer::new(src).tokenize();
        let kinds: Vec<&TokenKind> = tokens.iter().map(|t| &t.kind).collect();
        assert_eq!(
            kinds,
            vec![&TokenKind::KwBreak, &TokenKind::KwContinue, &TokenKind::Eof,]
        );
    }

    #[test]
    fn test_lex_type_keyword() {
        let tokens = Lexer::new("型").tokenize();
        assert_eq!(tokens.len(), 2);
        assert_eq!(tokens[0].kind, TokenKind::KwType);
        assert_eq!(tokens[1].kind, TokenKind::Eof);
    }

    #[test]
    fn test_lex_enum_and_match_keywords() {
        let src = "列挙 照合";
        let tokens = Lexer::new(src).tokenize();
        let kinds: Vec<&TokenKind> = tokens.iter().map(|t| &t.kind).collect();
        assert_eq!(
            kinds,
            vec![&TokenKind::KwEnum, &TokenKind::KwMatch, &TokenKind::Eof,]
        );
    }

    #[test]
    fn test_lex_new_array_keyword() {
        let tokens = Lexer::new("新配列＜整数＞").tokenize();
        let kinds: Vec<TokenKind> = tokens.into_iter().map(|t| t.kind).collect();
        assert_eq!(
            kinds,
            vec![
                TokenKind::KwNewArray,
                TokenKind::Lt,
                TokenKind::TyInt,
                TokenKind::Gt,
                TokenKind::Eof,
            ]
        );
    }
}
