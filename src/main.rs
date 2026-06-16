mod compiler;
mod lexer;
mod parser;
mod typechecker;
mod vm;

use std::{env, fs, process};

use compiler::Compiler;
use lexer::Lexer;
use parser::Parser;
use typechecker::TypeChecker;
use vm::{Vm, display_value};

fn main() {
    let args: Vec<String> = env::args().collect();
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
        eprintln!("構文エラー: {:?}", e);
        process::exit(1);
    });

    if let Err(e) = TypeChecker::new().check(&ast) {
        eprintln!("型エラー: {:?}", e);
        process::exit(1);
    }

    let mut compiler = Compiler::new();
    let instructions = compiler.compile(&ast);
    let result = Vm::with_chunks(compiler.constants, compiler.chunks, instructions)
        .run()
        .unwrap_or_else(|e| {
            eprintln!("実行時エラー: {:?}", e);
            process::exit(1);
        });

    if let Some(value) = result {
        println!("{}", display_value(&value));
    }
}
