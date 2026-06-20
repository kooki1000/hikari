use crate::lexer::TokenKind;

use super::ast::HikariType;

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
        TokenKind::KwOption => "「省略可」".to_string(),
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
        HikariType::Option(inner) => format!("省略可＜{}＞", hikari_type_japanese(inner)),
        HikariType::Record(name) => name.clone(),
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
