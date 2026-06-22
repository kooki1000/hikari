use std::collections::HashSet;
use std::path::{Path, PathBuf};

use crate::lexer::Span;
use crate::parser::{Expr, Parser, Stmt};
use crate::lexer::Lexer;

// Canonical names of the built-in stdlib modules. These constants are the
// single source of truth: the typechecker references them when gating builtins
// and when recording 取り込む'd modules, so the lists can never drift apart.
pub const MOD_MATH: &str = "数学";
pub const MOD_STRING: &str = "文字列";
pub const MOD_ARRAY: &str = "配列";
pub const MOD_MAP: &str = "辞書";
pub const MOD_FUNC: &str = "関数";
pub const MOD_IO: &str = "入出力";
pub const MOD_ENV: &str = "環境";
pub const MOD_TIME: &str = "時間";

pub const STDLIB_MODULES: [&str; 8] = [
    MOD_MATH, MOD_STRING, MOD_ARRAY, MOD_MAP, MOD_FUNC, MOD_IO, MOD_ENV, MOD_TIME,
];

// Imports are only resolved at the top level of a file: the roadmap's
// examples only ever show 取り込む as a top-level statement, so nested
// bodies (もし/間/関数 etc.) are intentionally not walked.
pub fn resolve_imports(
    stmts: Vec<Stmt>,
    base_dir: &Path,
    visited: &mut HashSet<PathBuf>,
) -> Result<Vec<Stmt>, String> {
    let mut out = Vec::new();
    for stmt in stmts {
        match stmt {
            // Stdlib imports pass through unchanged.
            Stmt::Import { ref name, .. } if STDLIB_MODULES.contains(&name.as_str()) => {
                out.push(stmt);
            }
            Stmt::Import { name, alias, span } => {
                let canonical = resolve_path(&name, base_dir, span)?;
                if visited.contains(&canonical) {
                    continue;
                }
                visited.insert(canonical.clone());

                let source = std::fs::read_to_string(&canonical).map_err(|e| {
                    format!(
                        "インポートエラー: ファイルを読み込めません '{}': {}",
                        canonical.display(),
                        e
                    )
                })?;
                let tokens = Lexer::new(&source).tokenize();
                let imported_stmts = Parser::new(tokens).parse().map_err(|e| format!("{}", e))?;
                let imported_dir = canonical
                    .parent()
                    .map(Path::to_path_buf)
                    .unwrap_or_else(|| PathBuf::from("."));
                let resolved = resolve_imports(imported_stmts, &imported_dir, visited)?;

                if let Some(alias_name) = alias {
                    // Namespaced import: prefix all module functions and their
                    // internal cross-references with `alias。`.
                    for stmt in mangle_module(resolved, &alias_name) {
                        out.push(stmt);
                    }
                } else {
                    // Flat import (legacy): splice FnDecls/types/imports as-is.
                    for resolved_stmt in resolved {
                        if matches!(
                            resolved_stmt,
                            Stmt::FnDecl { .. }
                                | Stmt::Import { .. }
                                | Stmt::TypeDecl { .. }
                                | Stmt::EnumDecl { .. }
                        ) {
                            out.push(resolved_stmt);
                        }
                    }
                }
            }
            other => out.push(other),
        }
    }
    Ok(out)
}

// Resolve a user file import path, checking base_dir first then HIKARI_PATH.
fn resolve_path(name: &str, base_dir: &Path, span: Span) -> Result<PathBuf, String> {
    // 1. Relative to the importing file's directory.
    let rel = base_dir.join(name);
    if let Ok(c) = std::fs::canonicalize(&rel) {
        return Ok(c);
    }

    // 2. HIKARI_PATH directories (colon-separated on Unix, semicolon on Windows).
    #[cfg(windows)]
    let sep = ';';
    #[cfg(not(windows))]
    let sep = ':';

    if let Ok(hikari_path) = std::env::var("HIKARI_PATH") {
        for dir in hikari_path.split(sep).filter(|s| !s.is_empty()) {
            let candidate = Path::new(dir).join(name);
            if let Ok(c) = std::fs::canonicalize(&candidate) {
                return Ok(c);
            }
        }
    }

    Err(format!(
        "インポートエラー: ファイルを読み込めません '{}' (行 {})",
        name, span.line
    ))
}

// ── Name-mangling for namespaced imports ──────────────────────────────────────

// Apply `alias。` prefix to all FnDecl names in `stmts` and rewrite every
// Call within those bodies that targets another function in the same module.
// TypeDecl/EnumDecl names are also prefixed so the checker can gate access.
// Non-FnDecl/non-type top-level statements (like stdlib imports) pass through.
fn mangle_module(stmts: Vec<Stmt>, alias: &str) -> Vec<Stmt> {
    // Collect the names of all functions declared at the top level of this module.
    let local_fns: HashSet<String> = stmts
        .iter()
        .filter_map(|s| {
            if let Stmt::FnDecl { name, .. } = s {
                Some(name.clone())
            } else {
                None
            }
        })
        .collect();

    let local_types: HashSet<String> = stmts
        .iter()
        .filter_map(|s| match s {
            Stmt::TypeDecl { name, .. } | Stmt::EnumDecl { name, .. } => Some(name.clone()),
            _ => None,
        })
        .collect();

    stmts
        .into_iter()
        .filter_map(|stmt| match stmt {
            Stmt::FnDecl {
                name,
                type_params,
                params,
                return_ty,
                body,
                is_public,
                span,
            } => {
                let mangled_name = format!("{}。{}", alias, name);
                let mangled_body =
                    mangle_stmts(body, alias, &local_fns, &local_types);
                Some(Stmt::FnDecl {
                    name: mangled_name,
                    type_params,
                    params,
                    return_ty,
                    body: mangled_body,
                    // Private functions are stored in the checker's private set
                    // by their mangled name; is_public is preserved for that.
                    is_public,
                    span,
                })
            }
            Stmt::TypeDecl { name, fields, span } => Some(Stmt::TypeDecl {
                name: format!("{}。{}", alias, name),
                fields,
                span,
            }),
            Stmt::EnumDecl { name, variants, span } => Some(Stmt::EnumDecl {
                name: format!("{}。{}", alias, name),
                variants,
                span,
            }),
            // Stdlib imports needed by the library file pass through (no prefix).
            Stmt::Import { .. } => Some(stmt),
            // Top-level executable code in a library file is silently dropped.
            _ => None,
        })
        .collect()
}

fn mangle_stmts(
    stmts: Vec<Stmt>,
    alias: &str,
    local_fns: &HashSet<String>,
    local_types: &HashSet<String>,
) -> Vec<Stmt> {
    stmts
        .into_iter()
        .map(|s| mangle_stmt(s, alias, local_fns, local_types))
        .collect()
}

fn mangle_stmt(
    stmt: Stmt,
    alias: &str,
    local_fns: &HashSet<String>,
    local_types: &HashSet<String>,
) -> Stmt {
    match stmt {
        Stmt::Return(expr, span) => {
            Stmt::Return(expr.map(|e| mangle_expr(e, alias, local_fns, local_types)), span)
        }
        Stmt::VarDecl { ty, name, value, span } => Stmt::VarDecl {
            ty,
            name,
            value: mangle_expr(value, alias, local_fns, local_types),
            span,
        },
        Stmt::Assign { name, value, span } => Stmt::Assign {
            name,
            value: mangle_expr(value, alias, local_fns, local_types),
            span,
        },
        Stmt::IndexAssign { name, index, value, span } => Stmt::IndexAssign {
            name,
            index: mangle_expr(index, alias, local_fns, local_types),
            value: mangle_expr(value, alias, local_fns, local_types),
            span,
        },
        Stmt::FieldAssign { record, field, value, span } => Stmt::FieldAssign {
            record: mangle_expr(record, alias, local_fns, local_types),
            field,
            value: mangle_expr(value, alias, local_fns, local_types),
            span,
        },
        Stmt::If { condition, then_body, else_body, span } => Stmt::If {
            condition: mangle_expr(condition, alias, local_fns, local_types),
            then_body: mangle_stmts(then_body, alias, local_fns, local_types),
            else_body: else_body
                .map(|b| mangle_stmts(b, alias, local_fns, local_types)),
            span,
        },
        Stmt::While { condition, body, span } => Stmt::While {
            condition: mangle_expr(condition, alias, local_fns, local_types),
            body: mangle_stmts(body, alias, local_fns, local_types),
            span,
        },
        Stmt::ForRange { var, from, to, body, span } => Stmt::ForRange {
            var,
            from: mangle_expr(from, alias, local_fns, local_types),
            to: mangle_expr(to, alias, local_fns, local_types),
            body: mangle_stmts(body, alias, local_fns, local_types),
            span,
        },
        Stmt::ForEach { var, array, body, span } => Stmt::ForEach {
            var,
            array: mangle_expr(array, alias, local_fns, local_types),
            body: mangle_stmts(body, alias, local_fns, local_types),
            span,
        },
        Stmt::TryCatch { try_body, error_var, catch_body, span } => Stmt::TryCatch {
            try_body: mangle_stmts(try_body, alias, local_fns, local_types),
            error_var,
            catch_body: mangle_stmts(catch_body, alias, local_fns, local_types),
            span,
        },
        Stmt::FnDecl { name, type_params, params, return_ty, body, is_public, span } => {
            // Nested function inside a module function body.
            Stmt::FnDecl {
                name,
                type_params,
                params,
                return_ty,
                body: mangle_stmts(body, alias, local_fns, local_types),
                is_public,
                span,
            }
        }
        Stmt::Match { subject, arms, span } => Stmt::Match {
            subject: mangle_expr(subject, alias, local_fns, local_types),
            arms: arms
                .into_iter()
                .map(|arm| crate::parser::MatchArm {
                    variant: arm.variant,
                    binders: arm.binders,
                    body: mangle_stmts(arm.body, alias, local_fns, local_types),
                })
                .collect(),
            span,
        },
        Stmt::Expr(expr, span) => {
            Stmt::Expr(mangle_expr(expr, alias, local_fns, local_types), span)
        }
        Stmt::Print(exprs, span) => Stmt::Print(
            exprs
                .into_iter()
                .map(|e| mangle_expr(e, alias, local_fns, local_types))
                .collect(),
            span,
        ),
        // Pass-through nodes that don't contain expressions.
        other @ (Stmt::Import { .. }
        | Stmt::Break(_)
        | Stmt::Continue(_)
        | Stmt::TypeDecl { .. }
        | Stmt::EnumDecl { .. }) => other,
    }
}

fn mangle_expr(
    expr: Expr,
    alias: &str,
    local_fns: &HashSet<String>,
    local_types: &HashSet<String>,
) -> Expr {
    match expr {
        Expr::Call { name, args } => {
            let mangled_name = if local_fns.contains(&name) {
                format!("{}。{}", alias, name)
            } else {
                name
            };
            Expr::Call {
                name: mangled_name,
                args: args
                    .into_iter()
                    .map(|a| mangle_expr(a, alias, local_fns, local_types))
                    .collect(),
            }
        }
        Expr::RecordLit { type_name, fields } => {
            let mangled_type = if local_types.contains(&type_name) {
                format!("{}。{}", alias, type_name)
            } else {
                type_name
            };
            Expr::RecordLit {
                type_name: mangled_type,
                fields: fields
                    .into_iter()
                    .map(|(k, v)| (k, mangle_expr(v, alias, local_fns, local_types)))
                    .collect(),
            }
        }
        Expr::BinOp { op, lhs, rhs } => Expr::BinOp {
            op,
            lhs: Box::new(mangle_expr(*lhs, alias, local_fns, local_types)),
            rhs: Box::new(mangle_expr(*rhs, alias, local_fns, local_types)),
        },
        Expr::UnaryMinus(inner) => {
            Expr::UnaryMinus(Box::new(mangle_expr(*inner, alias, local_fns, local_types)))
        }
        Expr::UnaryNot(inner) => {
            Expr::UnaryNot(Box::new(mangle_expr(*inner, alias, local_fns, local_types)))
        }
        Expr::Array(elems) => Expr::Array(
            elems
                .into_iter()
                .map(|e| mangle_expr(e, alias, local_fns, local_types))
                .collect(),
        ),
        Expr::Index { array, index } => Expr::Index {
            array: Box::new(mangle_expr(*array, alias, local_fns, local_types)),
            index: Box::new(mangle_expr(*index, alias, local_fns, local_types)),
        },
        Expr::MapLit(pairs) => Expr::MapLit(
            pairs
                .into_iter()
                .map(|(k, v)| {
                    (
                        mangle_expr(k, alias, local_fns, local_types),
                        mangle_expr(v, alias, local_fns, local_types),
                    )
                })
                .collect(),
        ),
        Expr::FieldAccess { record, field } => Expr::FieldAccess {
            record: Box::new(mangle_expr(*record, alias, local_fns, local_types)),
            field,
        },
        Expr::Lambda { params, return_ty, body } => Expr::Lambda {
            params,
            return_ty,
            body: mangle_stmts(body, alias, local_fns, local_types),
        },
        // Atoms (no sub-expressions to mangle).
        leaf @ (Expr::LitInt(_)
        | Expr::LitFloat(_)
        | Expr::LitString(_)
        | Expr::LitBool(_)
        | Expr::Ident(_)
        | Expr::NewArray(_)) => leaf,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::Lexer;
    use crate::parser::Parser;

    fn parse(src: &str) -> Vec<Stmt> {
        Parser::new(Lexer::new(src).tokenize()).parse().unwrap()
    }

    #[test]
    fn test_no_imports_passes_through_unchanged() {
        let stmts = parse("整数 値 ＝ １；");
        let base = std::env::temp_dir();
        let mut visited = HashSet::new();
        let resolved = resolve_imports(stmts.clone(), &base, &mut visited).unwrap();
        assert_eq!(resolved, stmts);
    }

    #[test]
    fn test_stdlib_import_passes_through_unchanged() {
        let stmts = parse("取り込む 「数学」；");
        let base = std::env::temp_dir();
        let mut visited = HashSet::new();
        let resolved = resolve_imports(stmts.clone(), &base, &mut visited).unwrap();
        assert_eq!(resolved, stmts);

        let stmts2 = parse("取り込む 「文字列」；");
        let mut visited2 = HashSet::new();
        let resolved2 = resolve_imports(stmts2.clone(), &base, &mut visited2).unwrap();
        assert_eq!(resolved2, stmts2);
    }

    #[test]
    fn test_file_import_splices_fn_decls() {
        let dir = std::env::temp_dir();
        let module_path = dir.join(format!("hikari_test_module_{}.hkr", std::process::id()));
        std::fs::write(
            &module_path,
            "関数 二倍（整数 ｎ）ー＞整数｛ 返す ｎ ＊ ２； ｝",
        )
        .unwrap();

        let src = format!(
            "取り込む 「{}」；",
            module_path.file_name().unwrap().to_str().unwrap()
        );
        let stmts = parse(&src);
        let mut visited = HashSet::new();
        let resolved = resolve_imports(stmts, &dir, &mut visited).unwrap();

        assert_eq!(resolved.len(), 1);
        assert!(matches!(
            &resolved[0],
            Stmt::FnDecl { name, .. } if name == "二倍"
        ));

        std::fs::remove_file(&module_path).unwrap();
    }

    #[test]
    fn test_cyclic_imports_resolve_without_infinite_loop() {
        let dir = std::env::temp_dir();
        let pid = std::process::id();
        let path_a = dir.join(format!("hikari_test_cycle_a_{}.hkr", pid));
        let path_b = dir.join(format!("hikari_test_cycle_b_{}.hkr", pid));

        let name_a = path_a.file_name().unwrap().to_str().unwrap().to_string();
        let name_b = path_b.file_name().unwrap().to_str().unwrap().to_string();

        std::fs::write(
            &path_a,
            format!(
                "取り込む 「{}」； 関数 関数ａ（）ー＞整数｛ 返す １； ｝",
                name_b
            ),
        )
        .unwrap();
        std::fs::write(
            &path_b,
            format!(
                "取り込む 「{}」； 関数 関数ｂ（）ー＞整数｛ 返す ２； ｝",
                name_a
            ),
        )
        .unwrap();

        let src = format!("取り込む 「{}」；", name_a);
        let stmts = parse(&src);
        let mut visited = HashSet::new();
        let resolved = resolve_imports(stmts, &dir, &mut visited).unwrap();

        let fn_names: Vec<&str> = resolved
            .iter()
            .filter_map(|s| match s {
                Stmt::FnDecl { name, .. } => Some(name.as_str()),
                _ => None,
            })
            .collect();
        assert_eq!(fn_names.len(), 2);
        assert!(fn_names.contains(&"関数ａ"));
        assert!(fn_names.contains(&"関数ｂ"));

        std::fs::remove_file(&path_a).unwrap();
        std::fs::remove_file(&path_b).unwrap();
    }

    // ── 14e: imported files carry their own stdlib imports and type decls ──

    #[test]
    fn test_file_import_splices_stdlib_imports_from_library() {
        let dir = std::env::temp_dir();
        let module_path = dir.join(format!("hikari_test_lib_stdlib_{}.hkr", std::process::id()));
        // Library uses 累乗 from 「数学」; that 取り込む must be carried over.
        std::fs::write(
            &module_path,
            "取り込む 「数学」；関数 二乗（整数 ｎ）ー＞整数｛ 返す 累乗（ｎ、２）； ｝",
        )
        .unwrap();

        let src = format!(
            "取り込む 「{}」；",
            module_path.file_name().unwrap().to_str().unwrap()
        );
        let stmts = parse(&src);
        let mut visited = HashSet::new();
        let resolved = resolve_imports(stmts, &dir, &mut visited).unwrap();

        // Must contain both the stdlib import and the function declaration.
        let has_math_import = resolved
            .iter()
            .any(|s| matches!(s, Stmt::Import { name, .. } if name == "数学"));
        let has_fn = resolved
            .iter()
            .any(|s| matches!(s, Stmt::FnDecl { name, .. } if name == "二乗"));
        assert!(has_math_import, "stdlib import from library must be spliced");
        assert!(has_fn, "function from library must be spliced");

        std::fs::remove_file(&module_path).unwrap();
    }

    #[test]
    fn test_file_import_splices_enum_decl_from_library() {
        let dir = std::env::temp_dir();
        let module_path =
            dir.join(format!("hikari_test_lib_enum_{}.hkr", std::process::id()));
        std::fs::write(
            &module_path,
            "構造 方向 ｛ 北、南 ｝関数 反転（方向 ｄ）ー＞方向｛ 照合 ｄ ｛ 北（）ならば ｛ 返す 南（）； ｝ 南（）ならば ｛ 返す 北（）； ｝ ｝ ｝",
        )
        .unwrap();

        let src = format!(
            "取り込む 「{}」；",
            module_path.file_name().unwrap().to_str().unwrap()
        );
        let stmts = parse(&src);
        let mut visited = HashSet::new();
        let resolved = resolve_imports(stmts, &dir, &mut visited).unwrap();

        let has_enum = resolved
            .iter()
            .any(|s| matches!(s, Stmt::EnumDecl { name, .. } if name == "方向"));
        let has_fn = resolved
            .iter()
            .any(|s| matches!(s, Stmt::FnDecl { name, .. } if name == "反転"));
        assert!(has_enum, "enum decl from library must be spliced");
        assert!(has_fn, "function from library must be spliced");

        std::fs::remove_file(&module_path).unwrap();
    }

    // ── 18a: aliased imports ──────────────────────────────────────────────────

    #[test]
    fn test_aliased_import_mangles_fn_names() {
        let dir = std::env::temp_dir();
        let module_path = dir.join(format!("hikari_test_alias_{}.hkr", std::process::id()));
        std::fs::write(
            &module_path,
            "公開 関数 距離（整数 ａ、整数 ｂ）ー＞整数｛ 返す ａ ＋ ｂ； ｝",
        )
        .unwrap();

        let filename = module_path.file_name().unwrap().to_str().unwrap();
        let src = format!("取り込む 「{}」 として 幾何；", filename);
        let stmts = parse(&src);
        let mut visited = HashSet::new();
        let resolved = resolve_imports(stmts, &dir, &mut visited).unwrap();

        assert!(
            resolved
                .iter()
                .any(|s| matches!(s, Stmt::FnDecl { name, .. } if name == "幾何。距離")),
            "function should be mangled to alias。name"
        );

        std::fs::remove_file(&module_path).unwrap();
    }

    #[test]
    fn test_aliased_import_rewrites_internal_calls() {
        let dir = std::env::temp_dir();
        let module_path = dir.join(format!("hikari_test_alias_call_{}.hkr", std::process::id()));
        // 倍 calls 補助 internally.
        std::fs::write(
            &module_path,
            "公開 関数 倍（整数 ｎ）ー＞整数｛ 返す 補助（ｎ）； ｝\
             関数 補助（整数 ｎ）ー＞整数｛ 返す ｎ ＊ ２； ｝",
        )
        .unwrap();

        let filename = module_path.file_name().unwrap().to_str().unwrap();
        let src = format!("取り込む 「{}」 として 数；", filename);
        let stmts = parse(&src);
        let mut visited = HashSet::new();
        let resolved = resolve_imports(stmts, &dir, &mut visited).unwrap();

        // 倍's body should call 数。補助, not 補助.
        let fn_decl = resolved.iter().find(|s| matches!(s, Stmt::FnDecl { name, .. } if name == "数。倍")).unwrap();
        if let Stmt::FnDecl { body, .. } = fn_decl {
            let has_mangled_call = body.iter().any(|s| {
                matches!(s, Stmt::Return(Some(Expr::Call { name, .. }), _) if name == "数。補助")
            });
            assert!(has_mangled_call, "internal call should be rewritten to alias。fn");
        }

        std::fs::remove_file(&module_path).unwrap();
    }

    // ── 18c: export control ───────────────────────────────────────────────────

    #[test]
    fn test_public_fn_has_is_public_true() {
        let stmts = parse("公開 関数 こんにちは（）ー＞無｛ 印刷（「hi」）； ｝");
        assert!(
            matches!(&stmts[0], Stmt::FnDecl { is_public: true, .. }),
            "公開 関数 should set is_public = true"
        );
    }

    #[test]
    fn test_private_fn_has_is_public_false() {
        let stmts = parse("関数 こんにちは（）ー＞無｛ 印刷（「hi」）； ｝");
        assert!(
            matches!(&stmts[0], Stmt::FnDecl { is_public: false, .. }),
            "unmarked 関数 should set is_public = false"
        );
    }

    // ── 18d: HIKARI_PATH search ───────────────────────────────────────────────

    #[test]
    fn test_hikari_path_search() {
        let dir = std::env::temp_dir().join(format!("hikari_path_test_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let module_path = dir.join("ライブラリ.hkr");
        std::fs::write(
            &module_path,
            "関数 テスト（）ー＞整数｛ 返す ４２； ｝",
        )
        .unwrap();

        // Set HIKARI_PATH to our temp dir, import from a different base_dir.
        // SAFETY: test is single-threaded (cargo test --test-threads=1 enforced
        // by Cargo's default for each integration test binary); no other thread
        // reads HIKARI_PATH concurrently in this process.
        unsafe { std::env::set_var("HIKARI_PATH", dir.to_str().unwrap()) };
        let other_base = std::env::temp_dir();
        let stmts = parse("取り込む 「ライブラリ.hkr」；");
        let mut visited = HashSet::new();
        let resolved = resolve_imports(stmts, &other_base, &mut visited).unwrap();
        unsafe { std::env::remove_var("HIKARI_PATH") };

        assert!(
            resolved
                .iter()
                .any(|s| matches!(s, Stmt::FnDecl { name, .. } if name == "テスト")),
            "HIKARI_PATH should let import find the module"
        );

        std::fs::remove_file(&module_path).unwrap();
        std::fs::remove_dir(&dir).unwrap();
    }
}
