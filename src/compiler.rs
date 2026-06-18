//! Compiles the typed AST into bytecode chunks for the VM.

mod builtins;
mod bytecode;
mod codegen;
mod error;
mod value;

#[cfg(test)]
mod tests;

pub use builtins::BuiltinFn;
pub use bytecode::{Chunk, Instruction};
pub use codegen::Compiler;
pub use error::CompileError;
pub use value::Value;
