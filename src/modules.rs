use std::collections::HashSet;
use std::path::{Path, PathBuf};

use crate::lexer::Lexer;
use crate::parser::{Parser, Stmt};

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

pub const STDLIB_MODULES: [&str; 7] = [
    MOD_MATH, MOD_STRING, MOD_ARRAY, MOD_MAP, MOD_FUNC, MOD_IO, MOD_ENV,
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
            Stmt::Import { ref name, .. } if STDLIB_MODULES.contains(&name.as_str()) => {
                out.push(stmt);
            }
            Stmt::Import { name, .. } => {
                let path = base_dir.join(&name);
                let canonical = std::fs::canonicalize(&path).map_err(|e| {
                    format!(
                        "インポートエラー: ファイルを読み込めません '{}': {}",
                        path.display(),
                        e
                    )
                })?;
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
            other => out.push(other),
        }
    }
    Ok(out)
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
}
