mod compiler;
mod diagnostic;
mod formatter;
mod lexer;
mod lints;
mod modules;
mod parser;
mod typechecker;
mod vm;

#[cfg(test)]
mod fuzz_tests;

use std::collections::HashSet;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::{env, fs, process};

use compiler::{CompileError, Compiler, Value};
use lexer::Lexer;
use parser::Parser;
use typechecker::TypeChecker;
use vm::{Vm, display_value};

const VERSION: &str = env!("CARGO_PKG_VERSION");

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();

    match args.first().map(String::as_str) {
        None => run_repl(),
        Some("--version" | "-v" | "バージョン") => {
            println!("Hikari {}", VERSION);
        }
        Some("--help" | "-h" | "助け") => print_usage(),
        Some("-c") => {
            // Inline program: `hikari -c "印刷（１）；" 引数1 引数2`
            let Some(code) = args.get(1) else {
                eprintln!("エラー: -c の後にコードがありません。");
                process::exit(1);
            };
            // Inline code resolves relative imports against the current dir.
            let entry_dir = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
            // Args after the code string become the program's 引数.
            let program_args = args.get(2..).map(<[_]>::to_vec).unwrap_or_default();
            run_source(code, &entry_dir, program_args);
        }
        Some("-") => {
            // Program piped on stdin: `echo "..." | hikari - 引数1 引数2`
            let source = io::read_to_string(io::stdin()).unwrap_or_else(|e| {
                eprintln!("エラー: 標準入力を読み込めません: {}", e);
                process::exit(1);
            });
            let entry_dir = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
            let program_args = args.get(1..).map(<[_]>::to_vec).unwrap_or_default();
            run_source(&source, &entry_dir, program_args);
        }
        // `hikari 整形 <file>` — pretty-print a source file to stdout.
        // `hikari 整形 -i <file>` — format in place.
        Some("整形") => {
            let in_place = args.get(1).map(|s| s == "-i").unwrap_or(false);
            let path_idx = if in_place { 2 } else { 1 };
            let Some(path) = args.get(path_idx) else {
                eprintln!("使い方: hikari 整形 [-i] <ファイル.hkr>");
                process::exit(1);
            };
            let source = fs::read_to_string(path).unwrap_or_else(|e| {
                eprintln!("エラー: ファイルを読み込めません '{}': {}", path, e);
                process::exit(1);
            });
            let mut lexer = Lexer::new(&source);
            let tokens = lexer.tokenize();
            let comments = lexer.into_comments();
            let ast = Parser::new(tokens).parse().unwrap_or_else(|e| {
                eprintln!("{}", diagnostic::render(&source, e.span(), &e.to_string()));
                process::exit(1);
            });
            let formatted = formatter::format_stmts_with_comments(&ast, &comments);
            if in_place {
                fs::write(path, &formatted).unwrap_or_else(|e| {
                    eprintln!("エラー: ファイルに書き込めません '{}': {}", path, e);
                    process::exit(1);
                });
            } else {
                print!("{}", formatted);
            }
        }
        // `hikari 試験 <file>` — run every zero-arg, 無-returning top-level
        // 関数 whose name starts with 「試験_」, using 確認 for assertions
        // inside them, and report a pass/fail count.
        Some("試験") => {
            let Some(path) = args.get(1) else {
                eprintln!("使い方: hikari 試験 <ファイル.hkr>");
                process::exit(1);
            };
            run_tests(path);
        }
        Some(flag) if flag.starts_with('-') => {
            eprintln!("エラー: 不明なオプション '{}'", flag);
            print_usage();
            process::exit(1);
        }
        Some(path) => {
            let source = fs::read_to_string(path).unwrap_or_else(|e| {
                eprintln!("エラー: ファイルを読み込めません '{}': {}", path, e);
                process::exit(1);
            });
            let entry_dir = Path::new(path)
                .parent()
                .filter(|p| !p.as_os_str().is_empty())
                .unwrap_or_else(|| Path::new("."))
                .to_path_buf();
            // Args after the script path become the program's 引数.
            let program_args = args.get(1..).map(<[_]>::to_vec).unwrap_or_default();
            run_source(&source, &entry_dir, program_args);
        }
    }
}

fn print_usage() {
    println!(
        "Hikari {ver} — 光プログラミング言語

使い方:
  hikari <ファイル.hkr> [引数...]       ファイルを実行する
  hikari                              対話モード（REPL）を開始する
  hikari - [引数...]                   標準入力からプログラムを読んで実行する
  hikari -c \"<コード>\" [引数...]        コードを直接実行する
  hikari 整形 <ファイル.hkr>            整形済みコードを標準出力に表示する
  hikari 整形 -i <ファイル.hkr>         ファイルを直接整形する（上書き）
  hikari 試験 <ファイル.hkr>            「試験_」で始まる関数を実行して結果を報告する

（[引数...] は「環境」モジュールの 引数（） で取得できます）

オプション:
  -h, --help, 助け         この使い方を表示する
  -v, --version, バージョン  バージョンを表示する",
        ver = VERSION
    );
}

/// Compile and run a complete Hikari program. Imports resolve relative to
/// `entry_dir`; `program_args` are exposed to the program via the 引数 builtin.
/// On any error, prints a diagnostic and exits non-zero.
fn run_source(source: &str, entry_dir: &Path, program_args: Vec<String>) {
    let tokens = Lexer::new(source).tokenize();
    let ast = Parser::new(tokens).parse().unwrap_or_else(|e| {
        eprintln!("{}", diagnostic::render(source, e.span(), &e.to_string()));
        process::exit(1);
    });

    // Lint the user's own file (before imports merge in library code), but only
    // surface the warnings once the program is known to be type-valid.
    let warnings = lints::check(&ast);

    let ast = modules::resolve_imports(ast, entry_dir, &mut HashSet::new()).unwrap_or_else(|e| {
        eprintln!("{}", e);
        process::exit(1);
    });

    // Collect all type errors (multi-error mode) so beginners see every
    // problem in one run rather than fix-one-rerun-fix-one-rerun.
    let mut checker = TypeChecker::new();
    let type_errors = checker.check_all(&ast);
    if !type_errors.is_empty() {
        for e in &type_errors {
            eprintln!("{}", diagnostic::render(source, e.span(), &e.to_string()));
        }
        process::exit(1);
    }

    for w in &warnings {
        eprintln!("{}", diagnostic::render_warning(source, w.span, &w.message));
    }

    let mut compiler = Compiler::new();
    // Hand the compiler the type checker's float-element 総和 sites (keyed by
    // node address into this same `ast`) so those calls lower to SumFloat.
    compiler.set_float_sum_sites(checker.take_float_sum_sites());
    let instructions = compiler.compile(&ast).unwrap_or_else(|e: CompileError| {
        eprintln!("コンパイルエラー: {}", e);
        process::exit(1);
    });
    let script_spans = compiler.script_spans.clone();
    let mut vm = Vm::with_chunks(compiler.constants, compiler.chunks, instructions);
    vm.set_script_spans(script_spans);
    vm.set_program_args(program_args);
    let result = vm.run().unwrap_or_else(|e| {
        // A runtime error now carries a source span (when known): render it
        // with the same snippet style as compile-time diagnostics.
        match vm.error_span() {
            Some(span) => eprintln!(
                "{}",
                diagnostic::render(source, span, &format!("実行時エラー: {}", e))
            ),
            None => eprintln!("実行時エラー: {}", e),
        }
        print_stack_trace(vm.error_trace());
        process::exit(1);
    });

    if let Some(value) = result {
        println!("{}", display_value(&value));
    }
}

/// Run every zero-arg, 無-returning top-level 関数 whose name starts with
/// 「試験_」 in `path`, reporting a pass/fail count. Each test function is
/// invoked inside a synthesized 試す/失敗 so one failing 確認 (or any other
/// runtime error) doesn't stop the remaining tests from running. Exits
/// non-zero if any test fails.
fn run_tests(path: &str) {
    let source = fs::read_to_string(path).unwrap_or_else(|e| {
        eprintln!("エラー: ファイルを読み込めません '{}': {}", path, e);
        process::exit(1);
    });
    let tokens = Lexer::new(&source).tokenize();
    let ast = Parser::new(tokens).parse().unwrap_or_else(|e| {
        eprintln!("{}", diagnostic::render(&source, e.span(), &e.to_string()));
        process::exit(1);
    });

    let test_names: Vec<String> = ast
        .iter()
        .filter_map(|stmt| match stmt {
            parser::Stmt::FnDecl {
                name,
                params,
                return_ty,
                ..
            } if name.starts_with("試験_")
                && params.is_empty()
                && *return_ty == parser::HikariType::Void =>
            {
                Some(name.clone())
            }
            _ => None,
        })
        .collect();

    if test_names.is_empty() {
        eprintln!("エラー: 「試験_」で始まる関数（引数なし、無を返す）が見つかりません。");
        process::exit(1);
    }

    // Each test runs inside its own 試す/失敗 so a failing 確認 (or any other
    // runtime error) is caught and tallied instead of aborting the run.
    let mut harness = String::from("取り込む 「入出力」；整数 合格数 ＝ ０；整数 失敗数 ＝ ０；");
    for name in &test_names {
        harness.push_str(&format!(
            "試す ｛ {name}（）； 合格数 ＝ 合格数 ＋ １； ｝ \
             失敗 エラー内容 ｛ 失敗数 ＝ 失敗数 ＋ １； \
             エラー印刷（「  失敗: {name} - 」＋エラー内容）； ｝",
        ));
    }
    harness.push_str(
        "印刷（「合格: 」＋文字列化（合格数）＋「  失敗: 」＋文字列化（失敗数）＋\
         「  (全」＋文字列化（合格数＋失敗数）＋「件)」）；\
         もし 失敗数 ＞ ０ ならば ｛ 終了（１）； ｝",
    );

    let combined = format!("{}\n{}", source, harness);
    let entry_dir = Path::new(path)
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."))
        .to_path_buf();
    run_source(&combined, &entry_dir, Vec::new());
}

/// Print an uncaught runtime error's call chain beneath the primary
/// diagnostic, innermost frame first (skipping the innermost frame itself,
/// already shown by the diagnostic snippet). No-op if there's nothing beyond
/// the top-level script frame.
fn print_stack_trace(trace: &[(Option<std::rc::Rc<str>>, Option<lexer::Span>)]) {
    if trace.len() <= 1 {
        return;
    }
    eprintln!("呼び出し元:");
    for (name, span) in &trace[1..] {
        let where_ = match name {
            Some(n) => format!("関数 {}", n),
            None => "トップレベル".to_string(),
        };
        match span {
            Some(s) => eprintln!("  {} (行 {})", where_, s.line),
            None => eprintln!("  {}", where_),
        }
    }
}

fn run_repl() {
    println!("Hikari 対話モード (Ctrl+D で終了)");

    let mut checker = TypeChecker::new();
    let mut compiler = Compiler::new();
    let mut vm = Vm::with_chunks(Vec::new(), Vec::new(), Vec::new());

    loop {
        print!("> ");
        if io::stdout().flush().is_err() {
            return;
        }

        let mut line = String::new();
        let bytes_read = io::stdin().read_line(&mut line).unwrap_or(0);
        if bytes_read == 0 {
            println!();
            return;
        }

        let line = line.trim_end_matches(['\n', '\r']);
        if line.trim().is_empty() {
            continue;
        }

        // Per-line transactionality: snapshot the persistent checker and
        // compiler so a line that fails at any stage (type, compile, or
        // runtime) leaves no half-applied declarations behind. The VM resets
        // its own transient state on an uncaught error (see run_repl_line).
        let checker_snapshot = checker.clone();
        let compiler_snapshot = compiler.clone();

        match eval_repl_line(line, &mut checker, &mut compiler, &mut vm) {
            Ok(Some(v)) => println!("{}", display_value(&v)),
            Ok(None) => {}
            Err(msg) => {
                eprintln!("{}", msg);
                checker = checker_snapshot;
                compiler = compiler_snapshot;
            }
        }
    }
}

/// Evaluate one REPL line against the persistent checker/compiler/VM, returning
/// the produced value (if any) or a rendered error message. On `Err` the caller
/// rolls the checker and compiler back to their pre-line snapshots.
fn eval_repl_line(
    line: &str,
    checker: &mut TypeChecker,
    compiler: &mut Compiler,
    vm: &mut Vm,
) -> Result<Option<Value>, String> {
    let tokens = Lexer::new(line).tokenize();
    let ast = Parser::new(tokens)
        .parse()
        .map_err(|e| diagnostic::render(line, e.span(), &e.to_string()))?;

    let entry_dir = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let ast = modules::resolve_imports(ast, &entry_dir, &mut HashSet::new())
        .map_err(|e| e.to_string())?;

    checker
        .check(&ast)
        .map_err(|e| diagnostic::render(line, e.span(), &e.to_string()))?;

    // Float-element 総和 sites for this line's AST (consumed immediately below
    // by compile() on the same AST, so the node addresses stay valid).
    compiler.set_float_sum_sites(checker.take_float_sum_sites());

    let instrs = compiler
        .compile(&ast)
        .map_err(|e| format!("コンパイルエラー: {}", e))?;
    let line_spans = compiler.script_spans.clone();
    vm.sync_program(compiler.constants.clone(), compiler.chunks.clone());

    vm.run_repl_line(instrs, line_spans)
        .map_err(|e| match vm.error_span() {
            Some(span) => diagnostic::render(line, span, &format!("実行時エラー: {}", e)),
            None => format!("実行時エラー: {}", e),
        })
}
