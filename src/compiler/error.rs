// ── Compile-time limit errors ───────────────────────────────────────────────
//
// The bytecode encodes some counts in fixed-width fields: constant-pool and
// jump/chunk indices are `u16`, and argument / payload / capture counts are
// `u8`. A program that exceeds one of these would silently wrap and miscompile,
// so the compiler rejects it with one of these errors instead. These limits are
// unreachable in hand-written programs — the check guards against silent
// corruption, not normal use.

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum CompileError {
    /// More than 65,535 distinct constants in one program.
    TooManyConstants(usize),
    /// More than 65,535 functions (including lambdas) in one program.
    TooManyFunctions(usize),
    /// A single function/script chunk compiled to more than 65,535
    /// instructions (jump offsets and literal sizes are `u16`).
    ChunkTooLarge(usize),
    /// A call or function declaration with more than 255 arguments/parameters.
    TooManyArguments(usize),
    /// An enum variant constructed with more than 255 payload values.
    TooManyPayloadValues(usize),
    /// A lambda capturing more than 255 enclosing variables.
    TooManyCaptures(usize),
}

impl std::fmt::Display for CompileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "プログラムが大きすぎます: ")?;
        match self {
            CompileError::TooManyConstants(n) => {
                write!(f, "定数が多すぎます（{} 個、上限は 65535）。", n)
            }
            CompileError::TooManyFunctions(n) => {
                write!(f, "関数が多すぎます（{} 個、上限は 65535）。", n)
            }
            CompileError::ChunkTooLarge(n) => write!(
                f,
                "一つの関数の命令が多すぎます（{} 個、上限は 65535）。",
                n
            ),
            CompileError::TooManyArguments(n) => {
                write!(f, "引数が多すぎます（{} 個、上限は 255）。", n)
            }
            CompileError::TooManyPayloadValues(n) => {
                write!(f, "構造の付随値が多すぎます（{} 個、上限は 255）。", n)
            }
            CompileError::TooManyCaptures(n) => write!(
                f,
                "無名関数が捕捉する変数が多すぎます（{} 個、上限は 255）。",
                n
            ),
        }
    }
}
