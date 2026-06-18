mod compiler;
mod diagnostic;
mod lexer;
mod modules;
mod parser;
mod typechecker;
mod vm;

use std::collections::HashSet;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::{env, fs, process};

use compiler::Compiler;
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
  hikari <ファイル.hkr> [引数...]   ファイルを実行する
  hikari                          対話モード（REPL）を開始する
  hikari - [引数...]               標準入力からプログラムを読んで実行する
  hikari -c \"<コード>\" [引数...]    コードを直接実行する

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

    let ast = modules::resolve_imports(ast, entry_dir, &mut HashSet::new()).unwrap_or_else(|e| {
        eprintln!("{}", e);
        process::exit(1);
    });

    if let Err(e) = TypeChecker::new().check(&ast) {
        eprintln!("{}", diagnostic::render(source, e.span(), &e.to_string()));
        process::exit(1);
    }

    let mut compiler = Compiler::new();
    let instructions = compiler.compile(&ast);
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
        process::exit(1);
    });

    if let Some(value) = result {
        println!("{}", display_value(&value));
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

        let tokens = Lexer::new(line).tokenize();
        let ast = match Parser::new(tokens).parse() {
            Ok(ast) => ast,
            Err(e) => {
                eprintln!("{}", diagnostic::render(line, e.span(), &e.to_string()));
                continue;
            }
        };

        let entry_dir = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let ast = match modules::resolve_imports(ast, &entry_dir, &mut HashSet::new()) {
            Ok(ast) => ast,
            Err(e) => {
                eprintln!("{}", e);
                continue;
            }
        };

        if let Err(e) = checker.check(&ast) {
            eprintln!("{}", diagnostic::render(line, e.span(), &e.to_string()));
            continue;
        }

        let instrs = compiler.compile(&ast);
        let line_spans = compiler.script_spans.clone();
        vm.sync_program(compiler.constants.clone(), compiler.chunks.clone());

        match vm.run_repl_line(instrs, line_spans) {
            Ok(Some(v)) => println!("{}", display_value(&v)),
            Ok(None) => {}
            Err(e) => match vm.error_span() {
                Some(span) => eprintln!(
                    "{}",
                    diagnostic::render(line, span, &format!("実行時エラー: {}", e))
                ),
                None => eprintln!("実行時エラー: {}", e),
            },
        }
    }
}
