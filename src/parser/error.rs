use crate::lexer::{Span, TokenKind};

use super::display::token_kind_japanese;

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
    // Input nested past the parser's depth limit (guards against stack overflow
    // on hostile input like thousands of nested parentheses).
    TooDeeplyNested {
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
            ParseError::TooDeeplyNested { span } => *span,
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
            ParseError::TooDeeplyNested { .. } => {
                write!(f, "式または文の入れ子が深すぎます。")
            }
        }
    }
}
