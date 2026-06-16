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

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() == 1 {
        run_repl();
        return;
    }
    if args.len() != 2 {
        eprintln!("使い方: hikari <ファイル.hkr>");
        process::exit(1);
    }

    let path = &args[1];
    let source = fs::read_to_string(path).unwrap_or_else(|e| {
        eprintln!("エラー: ファイルを読み込めません '{}': {}", path, e);
        process::exit(1);
    });

    let tokens = Lexer::new(&source).tokenize();
    let ast = Parser::new(tokens).parse().unwrap_or_else(|e| {
        eprintln!("{}", diagnostic::render(&source, e.span(), &e.to_string()));
        process::exit(1);
    });

    let entry_dir = Path::new(path).parent().unwrap_or_else(|| Path::new("."));
    let ast = modules::resolve_imports(ast, entry_dir, &mut HashSet::new()).unwrap_or_else(|e| {
        eprintln!("{}", e);
        process::exit(1);
    });

    if let Err(e) = TypeChecker::new().check(&ast) {
        eprintln!("{}", diagnostic::render(&source, e.span(), &e.to_string()));
        process::exit(1);
    }

    let mut compiler = Compiler::new();
    let instructions = compiler.compile(&ast);
    let result = Vm::with_chunks(compiler.constants, compiler.chunks, instructions)
        .run()
        .unwrap_or_else(|e| {
            eprintln!("実行時エラー: {}", e);
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
        vm.sync_program(compiler.constants.clone(), compiler.chunks.clone());

        match vm.run_repl_line(instrs) {
            Ok(Some(v)) => println!("{}", display_value(&v)),
            Ok(None) => {}
            Err(e) => eprintln!("実行時エラー: {}", e),
        }
    }
}
