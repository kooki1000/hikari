//! Static type checker: validates the AST between parsing and compilation.

mod checker;
mod error;
mod exprs;
mod generics;
pub(crate) mod symbols;

#[cfg(test)]
mod tests;

pub use checker::TypeChecker;
pub use symbols::builtin_module;
