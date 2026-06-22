use super::error::RuntimeError;
use super::frame::INITIAL_LOCALS;
use super::*;
use crate::compiler::{Compiler, Instruction, Value};
use crate::lexer::Lexer;
use crate::parser::Parser;

fn run(src: &str) -> Option<Value> {
    let ast = Parser::new(Lexer::new(src).tokenize()).parse().unwrap();
    let mut compiler = Compiler::new();
    let script = compiler.compile(&ast).unwrap();
    Vm::with_chunks(compiler.constants, compiler.chunks, script)
        .run()
        .unwrap()
}

mod builtins;
mod collections;
mod core;
mod errors_repl;
mod flow_records_maps;
mod stdlib;

fn run_result(src: &str) -> Result<Option<Value>, RuntimeError> {
    let ast = Parser::new(Lexer::new(src).tokenize()).parse().unwrap();
    let mut compiler = Compiler::new();
    let script = compiler.compile(&ast).unwrap();
    Vm::with_chunks(compiler.constants, compiler.chunks, script).run()
}

// Run a program with the given CLI arguments installed (for the 引数 builtin).
fn run_with_args(src: &str, args: &[&str]) -> Option<Value> {
    let ast = Parser::new(Lexer::new(src).tokenize()).parse().unwrap();
    let mut compiler = Compiler::new();
    let script = compiler.compile(&ast).unwrap();
    let mut vm = Vm::with_chunks(compiler.constants, compiler.chunks, script);
    vm.set_program_args(args.iter().map(|s| s.to_string()).collect());
    vm.run().unwrap()
}

// Run a program and return the line number reported for an uncaught runtime
// error (via the VM's recorded error span), or None on success / no span.
fn run_error_line(src: &str) -> Option<usize> {
    let ast = Parser::new(Lexer::new(src).tokenize()).parse().unwrap();
    let mut compiler = Compiler::new();
    let script = compiler.compile(&ast).unwrap();
    let script_spans = compiler.script_spans.clone();
    let mut vm = Vm::with_chunks(compiler.constants, compiler.chunks, script);
    vm.set_script_spans(script_spans);
    match vm.run() {
        Err(_) => vm.error_span().map(|s| s.line),
        Ok(_) => None,
    }
}
