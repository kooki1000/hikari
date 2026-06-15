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
use vm::Vm;

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

    // Lex
    let tokens = Lexer::new(&source).tokenize();

    // Parse
    let ast = Parser::new(tokens).parse();

    // Type-check
    if let Err(e) = TypeChecker::new().check(&ast) {
        eprintln!("型エラー: {:?}", e);
        process::exit(1);
    }

    // Compile
    let mut compiler = Compiler::new();
    let instructions = compiler.compile(&ast);

    // Run
    let result = Vm::new(compiler.constants, instructions).run();

    if let Some(value) = result {
        println!("{}", display_value(&value));
    }
}

fn display_value(val: &compiler::Value) -> String {
    match val {
        compiler::Value::Int(n) => n.to_string(),
        compiler::Value::Float(f) => f.to_string(),
        compiler::Value::Str(s) => s.clone(),
        compiler::Value::Bool(b) => if *b { "真" } else { "偽" }.to_string(),
    }
}
