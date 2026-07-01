// ── Error type ────────────────────────────────────────────────────────────────

#[derive(Debug, PartialEq)]
pub enum RuntimeError {
    StackUnderflow,
    UninitializedLocal(u16),
    DivisionByZero,
    TypeMismatch,
    InvalidConversion(String),
    IndexOutOfBounds { index: i64, len: usize },
    IntegerOverflow,
    // 取り出す on an empty array: there is no valid index to report, so this
    // gets its own variant rather than overloading IndexOutOfBounds.
    EmptyArray,
    // Map key lookup failed.
    KeyNotFound(String),
    // Call-frame depth exceeded the configured limit (runaway recursion).
    StackOverflow,
    // A file I/O operation failed (open/read/write).
    IoError(String),
    // 確認（真偽） was called with 偽.
    AssertionFailed,
}

impl std::fmt::Display for RuntimeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RuntimeError::StackUnderflow => {
                write!(f, "スタックが空の状態で値を取り出そうとしました。")
            }
            RuntimeError::UninitializedLocal(slot) => write!(
                f,
                "変数（スロット {}）が初期化される前に使用されました。",
                slot
            ),
            RuntimeError::DivisionByZero => write!(
                f,
                "ゼロで割ることはできません。（ヒント: 割る数が０にならないか確認してください）"
            ),
            RuntimeError::TypeMismatch => write!(f, "演算で扱う値の型が一致しません。"),
            RuntimeError::InvalidConversion(msg) => write!(f, "変換に失敗しました: {}", msg),
            RuntimeError::IndexOutOfBounds { index, len } => {
                write!(f, "添字 {} は範囲外です（配列の長さ: {}）。", index, len)
            }
            RuntimeError::IntegerOverflow => {
                write!(f, "整数の計算結果が大きすぎます（オーバーフロー）。")
            }
            RuntimeError::EmptyArray => {
                write!(f, "空の配列から要素を取り出すことはできません。")
            }
            RuntimeError::KeyNotFound(key) => {
                write!(f, "辞書にキー「{}」が見つかりません。", key)
            }
            RuntimeError::StackOverflow => write!(
                f,
                "再帰が深すぎます。（ヒント: 無限再帰になっていないか、終了条件を確認してください）"
            ),
            RuntimeError::IoError(msg) => {
                write!(f, "ファイル操作に失敗しました: {}", msg)
            }
            RuntimeError::AssertionFailed => write!(f, "確認が失敗しました。"),
        }
    }
}
