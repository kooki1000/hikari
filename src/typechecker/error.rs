use crate::lexer::Span;
use crate::parser::{BinOpKind, HikariType, hikari_type_japanese};

// ── Error type ────────────────────────────────────────────────────────────────

#[derive(Debug, PartialEq)]
pub enum TypeError {
    // Declared type does not match the inferred type of the initialiser.
    VarDeclMismatch {
        name: String,
        declared: HikariType,
        got: HikariType,
        span: Span,
    },
    // Both sides of a binary operator must share a type.
    BinOpMismatch {
        op: BinOpKind,
        lhs: HikariType,
        rhs: HikariType,
        span: Span,
    },
    // Variable referenced before declaration.
    UndeclaredVariable(String, Span),
    // Return expression type differs from the function's declared return type.
    ReturnTypeMismatch {
        expected: HikariType,
        got: HikariType,
        span: Span,
    },
    // Call to an undeclared function.
    UndeclaredFunction(String, Span),
    // Wrong number of arguments at a call site.
    ArgCountMismatch {
        name: String,
        expected: usize,
        got: usize,
        span: Span,
    },
    // Argument type does not match the parameter type.
    ArgTypeMismatch {
        name: String,
        param: HikariType,
        got: HikariType,
        span: Span,
    },
    // Condition in もし/間 is not Bool.
    ConditionNotBool(HikariType, Span),
    // Operand of a unary operator (単項マイナス／否定) has an unsupported type.
    UnaryOpMismatch {
        got: HikariType,
        span: Span,
    },
    // Array literal has no elements, so its element type cannot be inferred.
    EmptyArrayLiteral(Span),
    // Elements of an array literal do not all share the same type.
    ArrayElementTypeMismatch {
        expected: HikariType,
        got: HikariType,
        span: Span,
    },
    // Attempted 添字 access on a non-array value.
    NotIndexable {
        got: HikariType,
        span: Span,
    },
    // Index expression is not an Int.
    IndexNotInt {
        got: HikariType,
        span: Span,
    },
    // Stdlib builtin used without its module's 取り込む statement.
    ModuleNotImported {
        name: String,
        module: String,
        span: Span,
    },
    // A non-無 function has at least one control-flow path that falls off
    // its end without executing 返す.
    MissingReturn {
        name: String,
        span: Span,
    },
    // 抜ける／続ける used outside any enclosing 間／繰り返す／各 body.
    ControlFlowOutsideLoop {
        keyword: String,
        span: Span,
    },
    // Reference to a record type name that was never declared with 型.
    UndeclaredType(String, Span),
    // Record construction omits a field the type declares.
    MissingField {
        type_name: String,
        field: String,
        span: Span,
    },
    // Record construction (or field access) names a field the type doesn't have.
    UnknownField {
        type_name: String,
        field: String,
        span: Span,
    },
    // A field's value expression doesn't match the field's declared type,
    // whether at construction time or via field assignment. expected/got are
    // boxed (HikariType grew once Enum(String) was added) to keep TypeError
    // itself from exceeding clippy's result_large_err threshold.
    FieldTypeMismatch {
        type_name: String,
        field: String,
        expected: Box<HikariType>,
        got: Box<HikariType>,
        span: Span,
    },
    // ：フィールド access/assignment on a value that isn't a record.
    NotARecord {
        got: HikariType,
        span: Span,
    },
    // Two enum variants (within the same enum decl, or across different
    // enums) share a name; variant names must be globally unique since
    // there's no ：：-qualified construction to disambiguate them.
    DuplicateEnumVariant {
        variant: String,
        span: Span,
    },
    // 照合 subject is not of an enum type.
    NotAnEnum {
        got: HikariType,
        span: Span,
    },
    // A 照合 arm names the same variant as an earlier arm.
    DuplicateMatchArm {
        variant: String,
        span: Span,
    },
    // A 照合 arm names a variant that doesn't belong to the subject's enum
    // (typo, or a variant borrowed from a different enum).
    UndeclaredEnumVariant {
        enum_name: String,
        variant: String,
        span: Span,
    },
    // A 照合 statement does not cover every variant of the subject's enum.
    // Boxed (rather than inline String + Vec<String> + Span fields) to keep
    // TypeError's overall size from growing past clippy's result_large_err
    // threshold.
    NonExhaustiveMatch(Box<NonExhaustiveMatchInfo>),
}

#[derive(Debug, PartialEq)]
pub struct NonExhaustiveMatchInfo {
    pub enum_name: String,
    pub missing: Vec<String>,
    pub span: Span,
}

impl TypeError {
    pub fn span(&self) -> Span {
        match self {
            TypeError::VarDeclMismatch { span, .. } => *span,
            TypeError::BinOpMismatch { span, .. } => *span,
            TypeError::UndeclaredVariable(_, span) => *span,
            TypeError::ReturnTypeMismatch { span, .. } => *span,
            TypeError::UndeclaredFunction(_, span) => *span,
            TypeError::ArgCountMismatch { span, .. } => *span,
            TypeError::ArgTypeMismatch { span, .. } => *span,
            TypeError::ConditionNotBool(_, span) => *span,
            TypeError::UnaryOpMismatch { span, .. } => *span,
            TypeError::EmptyArrayLiteral(span) => *span,
            TypeError::ArrayElementTypeMismatch { span, .. } => *span,
            TypeError::NotIndexable { span, .. } => *span,
            TypeError::IndexNotInt { span, .. } => *span,
            TypeError::ModuleNotImported { span, .. } => *span,
            TypeError::MissingReturn { span, .. } => *span,
            TypeError::ControlFlowOutsideLoop { span, .. } => *span,
            TypeError::UndeclaredType(_, span) => *span,
            TypeError::MissingField { span, .. } => *span,
            TypeError::UnknownField { span, .. } => *span,
            TypeError::FieldTypeMismatch { span, .. } => *span,
            TypeError::NotARecord { span, .. } => *span,
            TypeError::DuplicateEnumVariant { span, .. } => *span,
            TypeError::NotAnEnum { span, .. } => *span,
            TypeError::DuplicateMatchArm { span, .. } => *span,
            TypeError::UndeclaredEnumVariant { span, .. } => *span,
            TypeError::NonExhaustiveMatch(info) => info.span,
        }
    }
}

impl std::fmt::Display for TypeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TypeError::VarDeclMismatch {
                name,
                declared,
                got,
                ..
            } => write!(
                f,
                "変数「{}」の型が一致しません: 「{}」として宣言されましたが、「{}」の値が代入されました。",
                name,
                hikari_type_japanese(declared),
                hikari_type_japanese(got)
            ),
            TypeError::BinOpMismatch { lhs, rhs, .. } => write!(
                f,
                "演算子の両辺の型が一致しません: 「{}」と「{}」は一緒に演算できません。",
                hikari_type_japanese(lhs),
                hikari_type_japanese(rhs)
            ),
            TypeError::UndeclaredVariable(name, _) => write!(
                f,
                "変数「{}」は宣言されていません。（ヒント: 使用する前に型と一緒に宣言してください）",
                name
            ),
            TypeError::ReturnTypeMismatch { expected, got, .. } => write!(
                f,
                "戻り値の型が一致しません: 「{}」を返す必要がありますが、「{}」が返されました。",
                hikari_type_japanese(expected),
                hikari_type_japanese(got)
            ),
            TypeError::UndeclaredFunction(name, _) => {
                write!(f, "関数「{}」は宣言されていません。", name)
            }
            TypeError::ArgCountMismatch {
                name,
                expected,
                got,
                ..
            } => write!(
                f,
                "関数「{}」の引数の数が一致しません: {}個必要ですが、{}個指定されました。",
                name, expected, got
            ),
            TypeError::ArgTypeMismatch {
                name, param, got, ..
            } => write!(
                f,
                "関数「{}」の引数の型が一致しません: 「{}」が必要ですが、「{}」が渡されました。",
                name,
                hikari_type_japanese(param),
                hikari_type_japanese(got)
            ),
            TypeError::ConditionNotBool(got, _) => write!(
                f,
                "条件式は「真偽」型である必要がありますが、「{}」が指定されました。",
                hikari_type_japanese(got)
            ),
            TypeError::UnaryOpMismatch { got, .. } => write!(
                f,
                "この単項演算には「{}」型を使用できません。",
                hikari_type_japanese(got)
            ),
            TypeError::EmptyArrayLiteral(_) => write!(
                f,
                "空の配列リテラルは型を推論できません。（ヒント: 少なくとも１つの要素を指定してください）"
            ),
            TypeError::ArrayElementTypeMismatch { expected, got, .. } => write!(
                f,
                "配列の要素の型が一致しません: 「{}」の配列に「{}」の値が含まれています。",
                hikari_type_japanese(expected),
                hikari_type_japanese(got)
            ),
            TypeError::NotIndexable { got, .. } => write!(
                f,
                "「{}」型の値には添字でアクセスできません。",
                hikari_type_japanese(got)
            ),
            TypeError::IndexNotInt { got, .. } => write!(
                f,
                "添字は「整数」型である必要がありますが、「{}」が指定されました。",
                hikari_type_japanese(got)
            ),
            TypeError::ModuleNotImported { name, module, .. } => write!(
                f,
                "「{}」を使うには「取り込む 「{}」；」が必要です。",
                name, module
            ),
            TypeError::MissingReturn { name, .. } => write!(
                f,
                "関数「{}」のすべての実行経路が値を返すとは限りません。",
                name
            ),
            TypeError::ControlFlowOutsideLoop { keyword, .. } => write!(
                f,
                "「{}」はループ（間／繰り返す／各）の中でのみ使用できます。",
                keyword
            ),
            TypeError::UndeclaredType(name, _) => write!(
                f,
                "型「{}」は宣言されていません。（ヒント: 「型 {} ｛ ... ｝」で宣言してください）",
                name, name
            ),
            TypeError::MissingField {
                type_name, field, ..
            } => write!(
                f,
                "型「{}」のフィールド「{}」が指定されていません。",
                type_name, field
            ),
            TypeError::UnknownField {
                type_name, field, ..
            } => write!(
                f,
                "型「{}」にはフィールド「{}」がありません。",
                type_name, field
            ),
            TypeError::FieldTypeMismatch {
                type_name,
                field,
                expected,
                got,
                ..
            } => write!(
                f,
                "型「{}」のフィールド「{}」の型が一致しません: 「{}」が必要ですが、「{}」が指定されました。",
                type_name,
                field,
                hikari_type_japanese(expected),
                hikari_type_japanese(got)
            ),
            TypeError::NotARecord { got, .. } => write!(
                f,
                "「{}」型の値にはフィールドでアクセスできません。",
                hikari_type_japanese(got)
            ),
            TypeError::DuplicateEnumVariant { variant, .. } => write!(
                f,
                "変体名「{}」は既に使用されています。（ヒント: 変体名はすべての構造型で一意である必要があります）",
                variant
            ),
            TypeError::NotAnEnum { got, .. } => write!(
                f,
                "「{}」型の値は照合できません。（ヒント: 照合は構造型の値に対してのみ使用できます）",
                hikari_type_japanese(got)
            ),
            TypeError::DuplicateMatchArm { variant, .. } => write!(
                f,
                "変体「{}」に対する場合がすでに指定されています。",
                variant
            ),
            TypeError::UndeclaredEnumVariant {
                enum_name, variant, ..
            } => write!(
                f,
                "構造「{}」には変体「{}」がありません。",
                enum_name, variant
            ),
            TypeError::NonExhaustiveMatch(info) => write!(
                f,
                "構造「{}」のすべての場合を網羅していません（未対応: {}）。",
                info.enum_name,
                info.missing.join("、")
            ),
        }
    }
}
