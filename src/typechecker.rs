//! Static type checker: validates the AST between parsing and compilation.

mod checker;
mod error;
mod exprs;
mod generics;
mod symbols;

#[cfg(test)]
mod tests;

pub use checker::TypeChecker;
