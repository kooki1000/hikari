//! AST pretty-printer for Hikari source code.
//!
//! `format_stmts` turns a parsed AST back into nicely formatted full-width
//! source text. The output is canonical: consistent spacing around operators,
//! one statement per line, and block bodies indented by two full-width spaces.

use crate::parser::{BinOpKind, Expr, HikariType, MatchArm, Stmt};

const INDENT_UNIT: &str = "　　"; // two ideographic spaces per level

/// Format a sequence of statements into a single source string.
pub fn format_stmts(stmts: &[Stmt]) -> String {
    let mut out = String::new();
    for stmt in stmts {
        format_stmt(stmt, 0, &mut out);
    }
    out
}

fn indent(level: usize) -> String {
    INDENT_UNIT.repeat(level)
}

fn format_stmt(stmt: &Stmt, level: usize, out: &mut String) {
    let ind = indent(level);
    match stmt {
        Stmt::Import { name, alias, .. } => {
            if let Some(alias_name) = alias {
                out.push_str(&format!(
                    "{}取り込む 「{}」 として {}；\n",
                    ind, name, alias_name
                ));
            } else {
                out.push_str(&format!("{}取り込む 「{}」；\n", ind, name));
            }
        }
        Stmt::VarDecl {
            ty, name, value, ..
        } => {
            out.push_str(&format!(
                "{}{} {} ＝ {}；\n",
                ind,
                format_type(ty),
                name,
                format_expr(value)
            ));
        }
        Stmt::Assign { name, value, .. } => {
            out.push_str(&format!("{}{} ＝ {}；\n", ind, name, format_expr(value)));
        }
        Stmt::IndexAssign {
            name, index, value, ..
        } => {
            out.push_str(&format!(
                "{}{}【{}】 ＝ {}；\n",
                ind,
                name,
                format_expr(index),
                format_expr(value)
            ));
        }
        Stmt::FieldAssign {
            record,
            field,
            value,
            ..
        } => {
            out.push_str(&format!(
                "{}{}：：{} ＝ {}；\n",
                ind,
                format_expr(record),
                field,
                format_expr(value)
            ));
        }
        Stmt::Return(None, _) => {
            out.push_str(&format!("{}返す；\n", ind));
        }
        Stmt::Return(Some(e), _) => {
            out.push_str(&format!("{}返す {}；\n", ind, format_expr(e)));
        }
        Stmt::Print(exprs, _) => {
            let args = exprs.iter().map(format_expr).collect::<Vec<_>>().join("、");
            out.push_str(&format!("{}印刷（{}）；\n", ind, args));
        }
        Stmt::Expr(e, _) => {
            out.push_str(&format!("{}{}；\n", ind, format_expr(e)));
        }
        Stmt::Break(_) => {
            out.push_str(&format!("{}抜ける；\n", ind));
        }
        Stmt::Continue(_) => {
            out.push_str(&format!("{}続ける；\n", ind));
        }
        Stmt::If {
            condition,
            then_body,
            else_body,
            ..
        } => {
            out.push_str(&format!(
                "{}もし {} ならば ｛\n",
                ind,
                format_expr(condition)
            ));
            for s in then_body {
                format_stmt(s, level + 1, out);
            }
            if let Some(eb) = else_body {
                out.push_str(&format!("{}｝ 違えば ｛\n", ind));
                for s in eb {
                    format_stmt(s, level + 1, out);
                }
            }
            out.push_str(&format!("{}｝\n", ind));
        }
        Stmt::While {
            condition, body, ..
        } => {
            out.push_str(&format!("{}間 {} ならば ｛\n", ind, format_expr(condition)));
            for s in body {
                format_stmt(s, level + 1, out);
            }
            out.push_str(&format!("{}｝\n", ind));
        }
        Stmt::ForRange {
            var,
            from,
            to,
            body,
            ..
        } => {
            out.push_str(&format!(
                "{}繰り返す {} ＝ {} から {} ならば ｛\n",
                ind,
                var,
                format_expr(from),
                format_expr(to)
            ));
            for s in body {
                format_stmt(s, level + 1, out);
            }
            out.push_str(&format!("{}｝\n", ind));
        }
        Stmt::ForEach {
            var, array, body, ..
        } => {
            out.push_str(&format!(
                "{}各 {} ：{} ならば ｛\n",
                ind,
                var,
                format_expr(array)
            ));
            for s in body {
                format_stmt(s, level + 1, out);
            }
            out.push_str(&format!("{}｝\n", ind));
        }
        Stmt::TryCatch {
            try_body,
            error_var,
            catch_body,
            ..
        } => {
            out.push_str(&format!("{}試す ｛\n", ind));
            for s in try_body {
                format_stmt(s, level + 1, out);
            }
            out.push_str(&format!("{}｝ 失敗（{}） ｛\n", ind, error_var));
            for s in catch_body {
                format_stmt(s, level + 1, out);
            }
            out.push_str(&format!("{}｝\n", ind));
        }
        Stmt::FnDecl {
            name,
            params,
            return_ty,
            body,
            is_public,
            ..
        } => {
            let pub_prefix = if *is_public { "公開 " } else { "" };
            let param_str = params
                .iter()
                .map(|(ty, n)| format!("{} {}", format_type(ty), n))
                .collect::<Vec<_>>()
                .join("、");
            out.push_str(&format!(
                "{}{}関数 {}（{}）ー＞{} ｛\n",
                ind,
                pub_prefix,
                name,
                param_str,
                format_type(return_ty)
            ));
            for s in body {
                format_stmt(s, level + 1, out);
            }
            out.push_str(&format!("{}｝\n", ind));
        }
        Stmt::TypeDecl { name, fields, .. } => {
            out.push_str(&format!("{}型 {} ｛\n", ind, name));
            for (ty, field_name) in fields {
                out.push_str(&format!(
                    "{}{}{} {}；\n",
                    ind,
                    INDENT_UNIT,
                    format_type(ty),
                    field_name
                ));
            }
            out.push_str(&format!("{}｝\n", ind));
        }
        Stmt::EnumDecl { name, variants, .. } => {
            let variant_names: Vec<String> = variants
                .iter()
                .map(|(vname, payload)| {
                    if payload.is_empty() {
                        vname.clone()
                    } else {
                        let tys = payload
                            .iter()
                            .map(format_type)
                            .collect::<Vec<_>>()
                            .join("、");
                        format!("{}（{}）", vname, tys)
                    }
                })
                .collect();
            out.push_str(&format!(
                "{}構造 {} ｛ {} ｝\n",
                ind,
                name,
                variant_names.join("、")
            ));
        }
        Stmt::Match { subject, arms, .. } => {
            out.push_str(&format!("{}照合 {} ｛\n", ind, format_expr(subject)));
            for arm in arms {
                format_match_arm(arm, level + 1, out);
            }
            out.push_str(&format!("{}｝\n", ind));
        }
    }
}

fn format_match_arm(arm: &MatchArm, level: usize, out: &mut String) {
    let ind = indent(level);
    let binder_str = if arm.binders.is_empty() {
        "（）".to_string()
    } else {
        format!("（{}）", arm.binders.join("、"))
    };
    out.push_str(&format!(
        "{}{}{}  ならば ｛\n",
        ind, arm.variant, binder_str
    ));
    for s in &arm.body {
        format_stmt(s, level + 1, out);
    }
    out.push_str(&format!("{}｝\n", ind));
}

fn format_expr(expr: &Expr) -> String {
    match expr {
        Expr::LitInt(n) => format_int(*n),
        Expr::LitFloat(f) => format_float(*f),
        Expr::LitString(s) => format!("「{}」", s),
        Expr::LitBool(b) => {
            if *b {
                "真".to_string()
            } else {
                "偽".to_string()
            }
        }
        Expr::Ident(name) => name.clone(),
        Expr::BinOp { op, lhs, rhs } => {
            // Parenthesize sub-expressions for binary ops to preserve precedence.
            let l = format_expr(lhs);
            let r = format_expr(rhs);
            let needs_paren_l = matches!(**lhs, Expr::BinOp { .. });
            let needs_paren_r = matches!(**rhs, Expr::BinOp { .. });
            let lf = if needs_paren_l {
                format!("（{}）", l)
            } else {
                l
            };
            let rf = if needs_paren_r {
                format!("（{}）", r)
            } else {
                r
            };
            format!("{} {} {}", lf, format_binop(op), rf)
        }
        Expr::UnaryMinus(e) => format!("ー{}", format_expr(e)),
        Expr::UnaryNot(e) => format!("否定 {}", format_expr(e)),
        Expr::Call { name, args, .. } => {
            let arg_str = args.iter().map(format_expr).collect::<Vec<_>>().join("、");
            format!("{}（{}）", name, arg_str)
        }
        Expr::Array(elems) => {
            let elem_str = elems.iter().map(format_expr).collect::<Vec<_>>().join("、");
            format!("【{}】", elem_str)
        }
        Expr::Index { array, index } => {
            format!("{}【{}】", format_expr(array), format_expr(index))
        }
        Expr::NewArray(ty) => format!("新配列＜{}＞", format_type(ty)),
        Expr::MapLit(pairs) => {
            if pairs.is_empty() {
                "｛｝".to_string()
            } else {
                let pair_str = pairs
                    .iter()
                    .map(|(k, v)| format!("{}：{}", format_expr(k), format_expr(v)))
                    .collect::<Vec<_>>()
                    .join("、");
                format!("｛{}｝", pair_str)
            }
        }
        Expr::RecordLit { type_name, fields } => {
            let field_str = fields
                .iter()
                .map(|(k, v)| format!("{}：{}", k, format_expr(v)))
                .collect::<Vec<_>>()
                .join("、");
            format!("{} ｛ {} ｝", type_name, field_str)
        }
        Expr::FieldAccess { record, field } => {
            format!("{}：：{}", format_expr(record), field)
        }
        Expr::Lambda {
            params,
            return_ty,
            body,
        } => {
            let param_str = params
                .iter()
                .map(|(n, ty)| format!("{}：{}", n, format_type(ty)))
                .collect::<Vec<_>>()
                .join("、");
            let mut body_str = String::new();
            for s in body {
                format_stmt(s, 0, &mut body_str);
            }
            // Lambda body is inlined — strip trailing newline and leading spaces.
            let body_inline = body_str.trim().to_string();
            format!(
                "｜{}｜ ー＞ {} ｛ {} ｝",
                param_str,
                format_type(return_ty),
                body_inline
            )
        }
    }
}

fn format_binop(op: &BinOpKind) -> &'static str {
    match op {
        BinOpKind::Add => "＋",
        BinOpKind::Sub => "ー",
        BinOpKind::Mul => "＊",
        BinOpKind::Div => "／",
        BinOpKind::Mod => "％",
        BinOpKind::Eq => "＝＝",
        BinOpKind::Lt => "＜",
        BinOpKind::Gt => "＞",
        BinOpKind::LtEq => "≦",
        BinOpKind::GtEq => "≧",
        BinOpKind::NotEq => "≠",
        BinOpKind::And => "かつ",
        BinOpKind::Or => "または",
    }
}

pub fn format_type(ty: &HikariType) -> String {
    match ty {
        HikariType::Int => "整数".to_string(),
        HikariType::Float => "小数".to_string(),
        HikariType::String => "文字列".to_string(),
        HikariType::Bool => "真偽".to_string(),
        HikariType::Void => "無".to_string(),
        HikariType::Array(inner) => match inner.as_ref() {
            HikariType::Int => "整数列".to_string(),
            HikariType::Float => "小数列".to_string(),
            HikariType::String => "文字列列".to_string(),
            HikariType::Bool => "真偽列".to_string(),
            other => format!("配列＜{}＞", format_type(other)),
        },
        HikariType::Map(k, v) => format!("辞書＜{}、{}＞", format_type(k), format_type(v)),
        HikariType::Record(name) => name.clone(),
        HikariType::Fn(params, ret) => {
            let param_str = params
                .iter()
                .map(format_type)
                .collect::<Vec<_>>()
                .join("、");
            format!("関数＜（{}）ー＞{}＞", param_str, format_type(ret))
        }
        HikariType::Option(inner) => format!("省略可＜{}＞", format_type(inner)),
    }
}

// ── Number formatting helpers ─────────────────────────────────────────────────

fn format_int(n: i64) -> String {
    if n < 0 {
        format!("ー{}", to_fullwidth_digits((-n) as u64))
    } else {
        to_fullwidth_digits(n as u64)
    }
}

fn format_float(f: f64) -> String {
    // Format as "整数部．小数部" in full-width digits.
    // Use Rust's default float repr then convert ASCII digits/dot.
    let s = format!("{}", f);
    s.chars()
        .map(|c| match c {
            '0'..='9' => char::from_u32(c as u32 - b'0' as u32 + '０' as u32).unwrap(),
            '.' => '．',
            '-' => 'ー',
            other => other,
        })
        .collect()
}

fn to_fullwidth_digits(n: u64) -> String {
    n.to_string()
        .chars()
        .map(|c| char::from_u32(c as u32 - b'0' as u32 + '０' as u32).unwrap())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::Lexer;
    use crate::parser::Parser;

    fn round_trip(src: &str) -> String {
        let ast = Parser::new(Lexer::new(src).tokenize()).parse().unwrap();
        format_stmts(&ast)
    }

    #[test]
    fn test_format_var_decl() {
        let out = round_trip("整数 ｘ ＝ ４２；");
        assert!(out.contains("整数 ｘ ＝ ４２；"), "got: {:?}", out);
    }

    #[test]
    fn test_format_fn_decl() {
        let out = round_trip("関数 二倍（整数 ｎ）ー＞整数｛ 返す ｎ ＊ ２； ｝");
        assert!(out.contains("関数 二倍（整数 ｎ）"), "got: {:?}", out);
        assert!(out.contains("返す ｎ ＊ ２；"), "got: {:?}", out);
    }

    #[test]
    fn test_format_pub_fn_decl() {
        let out = round_trip("公開 関数 こんにちは（）ー＞無｛ 印刷（「hi」）； ｝");
        assert!(out.contains("公開 関数 こんにちは"), "got: {:?}", out);
    }

    #[test]
    fn test_format_if_else() {
        let out = round_trip("もし 真 ならば ｛ 整数 ａ ＝ １； ｝ 違えば ｛ 整数 ｂ ＝ ２； ｝");
        assert!(out.contains("もし 真 ならば ｛"), "got: {:?}", out);
        assert!(out.contains("｝ 違えば ｛"), "got: {:?}", out);
    }

    #[test]
    fn test_format_import() {
        let out = round_trip("取り込む 「数学」；");
        assert_eq!(out.trim(), "取り込む 「数学」；");
    }

    #[test]
    fn test_format_aliased_import() {
        let out = round_trip("取り込む 「幾何.hkr」 として 幾何；");
        assert!(out.contains("として 幾何"), "got: {:?}", out);
    }

    #[test]
    fn test_format_while_loop() {
        let out = round_trip("間 真 ならば ｛ 抜ける； ｝");
        assert!(out.contains("間 真 ならば ｛"), "got: {:?}", out);
    }

    #[test]
    fn test_format_for_range() {
        let out = round_trip("繰り返す ｉ ＝ １ から ５ ならば ｛ 印刷（ｉ）； ｝");
        assert!(
            out.contains("繰り返す ｉ ＝ １ から ５ ならば ｛"),
            "got: {:?}",
            out
        );
    }

    #[test]
    fn test_format_for_each() {
        let out = round_trip("整数列 ａ ＝ 【１、２】；各 ｘ ：ａ ならば ｛ 印刷（ｘ）； ｝");
        assert!(out.contains("各 ｘ ：ａ ならば ｛"), "got: {:?}", out);
    }

    #[test]
    fn test_format_array_literal() {
        let out = round_trip("整数列 ａ ＝ 【１、２、３】；");
        assert!(out.contains("【１、２、３】"), "got: {:?}", out);
    }

    #[test]
    fn test_format_binop_preserves_operands() {
        let out = round_trip("整数 ｒ ＝ ３ ＊ ４ ＋ ５；");
        assert!(out.contains("＊"), "got: {:?}", out);
        assert!(out.contains("＋"), "got: {:?}", out);
    }

    #[test]
    fn test_format_map_literal() {
        let out = round_trip("辞書＜文字列、整数＞ ｍ ＝ ｛「ａ」：１｝；");
        assert!(out.contains("「ａ」：１"), "got: {:?}", out);
    }

    #[test]
    fn test_format_type_renders_compound_arrays() {
        assert_eq!(
            format_type(&HikariType::Array(Box::new(HikariType::Int))),
            "整数列"
        );
        assert_eq!(
            format_type(&HikariType::Array(Box::new(HikariType::Array(Box::new(
                HikariType::Int
            ))))),
            "配列＜整数列＞"
        );
    }

    #[test]
    fn test_format_negative_int() {
        let out = round_trip("整数 ｘ ＝ ー４２；");
        assert!(
            out.contains("ー４２") || out.contains("ー ４２"),
            "got: {:?}",
            out
        );
    }
}
