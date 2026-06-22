use crate::lexer::Span;

// ── Types ────────────────────────────────────────────────────────────────────

#[derive(Debug, PartialEq, Clone)]
pub enum HikariType {
    Int,    // 整数
    Float,  // 小数
    String, // 文字列
    Bool,   // 真偽
    Void,   // 無
    Array(Box<HikariType>),
    Map(Box<HikariType>, Box<HikariType>), // key type, value type
    Record(String), // user-defined record/enum type, identified by its declared name
    // function type — 関数＜(T1、T2) → R＞
    Fn(Vec<HikariType>, Box<HikariType>),
    // built-in option type — 省略可＜T＞
    Option(Box<HikariType>),
}

// ── AST nodes ────────────────────────────────────────────────────────────────

#[derive(Debug, PartialEq, Clone)]
pub enum Expr {
    LitInt(i64),
    LitFloat(f64),
    LitString(String),
    LitBool(bool),
    Ident(String),
    BinOp {
        op: BinOpKind,
        lhs: Box<Expr>,
        rhs: Box<Expr>,
    },
    Call {
        name: String,
        args: Vec<Expr>,
    },
    UnaryMinus(Box<Expr>),
    UnaryNot(Box<Expr>),
    Array(Vec<Expr>),
    Index {
        array: Box<Expr>,
        index: Box<Expr>,
    },
    NewArray(HikariType),
    MapLit(Vec<(Expr, Expr)>),
    RecordLit {
        type_name: String,
        fields: Vec<(String, Expr)>,
    },
    FieldAccess {
        record: Box<Expr>,
        field: String,
    },
    // anonymous function (lambda) — ｜params｜ → return_ty ｛ body ｝
    Lambda {
        params: Vec<(String, HikariType)>,
        return_ty: HikariType,
        body: Vec<Stmt>,
    },
}

#[derive(Debug, PartialEq, Clone)]
pub enum BinOpKind {
    Add,   // ＋
    Sub,   // ー
    Mul,   // ＊
    Div,   // ／
    Mod,   // ％
    Eq,    // ＝＝
    Lt,    // ＜
    Gt,    // ＞
    LtEq,  // ≦
    GtEq,  // ≧
    NotEq, // ≠
    And,   // かつ
    Or,    // または
}

#[derive(Debug, PartialEq, Clone)]
pub enum Stmt {
    VarDecl {
        ty: HikariType,
        name: String,
        value: Expr,
        span: Span,
    },
    FnDecl {
        name: String,
        type_params: Vec<String>, // e.g. ["Ｔ", "Ｕ"] for 関数＜Ｔ、Ｕ＞
        params: Vec<(HikariType, String)>,
        return_ty: HikariType,
        body: Vec<Stmt>,
        is_public: bool, // 公開 marker — controls export visibility
        span: Span,
    },
    Return(Option<Expr>, Span),
    // 印刷（…）: zero or more values, printed space-separated with a trailing
    // newline. An empty list prints just a newline.
    Print(Vec<Expr>, Span),
    If {
        condition: Expr,
        then_body: Vec<Stmt>,
        else_body: Option<Vec<Stmt>>,
        span: Span,
    },
    While {
        condition: Expr,
        body: Vec<Stmt>,
        span: Span,
    },
    Expr(Expr, Span),
    Assign {
        name: String,
        value: Expr,
        span: Span,
    },
    IndexAssign {
        name: String,
        index: Expr,
        value: Expr,
        span: Span,
    },
    ForRange {
        var: String,
        from: Expr,
        to: Expr,
        body: Vec<Stmt>,
        span: Span,
    },
    ForEach {
        var: String,
        array: Expr,
        body: Vec<Stmt>,
        span: Span,
    },
    TryCatch {
        try_body: Vec<Stmt>,
        error_var: String,
        catch_body: Vec<Stmt>,
        span: Span,
    },
    Import {
        name: String,
        alias: Option<String>, // として エイリアス — None for unaliased / stdlib imports
        span: Span,
    },
    Break(Span),
    Continue(Span),
    TypeDecl {
        name: String,
        fields: Vec<(HikariType, String)>,
        span: Span,
    },
    // The target is a full Expr (not a bare name like IndexAssign's) because
    // field-assign targets can themselves be the result of an arbitrary
    // expression, e.g. 配列【０】：：ｘ ＝ １；, whereas IndexAssign was modeled
    // around a bare local variable name.
    FieldAssign {
        record: Expr,
        field: String,
        value: Expr,
        span: Span,
    },
    EnumDecl {
        name: String,
        variants: Vec<(String, Vec<HikariType>)>,
        span: Span,
    },
    Match {
        subject: Expr,
        arms: Vec<MatchArm>,
        span: Span,
    },
}

#[derive(Debug, PartialEq, Clone)]
pub struct MatchArm {
    pub variant: String,
    pub binders: Vec<String>,
    pub body: Vec<Stmt>,
}
