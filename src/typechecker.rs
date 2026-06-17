use std::collections::HashMap;

use crate::lexer::Span;
use crate::parser::{BinOpKind, Expr, HikariType, Stmt, hikari_type_japanese};

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
                "変体名「{}」は既に使用されています。（ヒント: 変体名はすべての列挙型で一意である必要があります）",
                variant
            ),
            TypeError::NotAnEnum { got, .. } => write!(
                f,
                "「{}」型の値は照合できません。（ヒント: 照合は列挙型の値に対してのみ使用できます）",
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
                "列挙「{}」には変体「{}」がありません。",
                enum_name, variant
            ),
            TypeError::NonExhaustiveMatch(info) => write!(
                f,
                "列挙「{}」のすべての場合を網羅していません（未対応: {}）。",
                info.enum_name,
                info.missing.join("、")
            ),
        }
    }
}

// ── Symbol tables ─────────────────────────────────────────────────────────────

#[derive(Clone)]
struct FnSig {
    params: Vec<HikariType>,
    return_ty: HikariType,
}

fn builtin_sig(name: &str) -> Option<FnSig> {
    match name {
        "文字数" => Some(FnSig {
            params: vec![HikariType::String],
            return_ty: HikariType::Int,
        }),
        "入力" => Some(FnSig {
            params: vec![],
            return_ty: HikariType::String,
        }),
        "整数化" => Some(FnSig {
            params: vec![HikariType::String],
            return_ty: HikariType::Int,
        }),
        "小数化" => Some(FnSig {
            params: vec![HikariType::String],
            return_ty: HikariType::Float,
        }),
        // 文字列化's single param is polymorphic (Int|Float|Bool); the param
        // type here is unused since Expr::Call checks it inline.
        "文字列化" => Some(FnSig {
            params: vec![HikariType::Int],
            return_ty: HikariType::String,
        }),
        "乱数" => Some(FnSig {
            params: vec![HikariType::Int, HikariType::Int],
            return_ty: HikariType::Int,
        }),
        "分割" => Some(FnSig {
            params: vec![HikariType::String, HikariType::String],
            return_ty: HikariType::Array(Box::new(HikariType::String)),
        }),
        "結合" => Some(FnSig {
            params: vec![
                HikariType::Array(Box::new(HikariType::String)),
                HikariType::String,
            ],
            return_ty: HikariType::String,
        }),
        "含む" => Some(FnSig {
            params: vec![HikariType::String, HikariType::String],
            return_ty: HikariType::Bool,
        }),
        "置換" => Some(FnSig {
            params: vec![HikariType::String, HikariType::String, HikariType::String],
            return_ty: HikariType::String,
        }),
        _ => None,
    }
}

// Maps gated stdlib builtins to the module that must be 取り込む'd before
// they can be called. Phase-2 builtins are absent here, meaning ungated.
fn builtin_module(name: &str) -> Option<&'static str> {
    match name {
        "絶対値" | "平方根" | "乱数" | "最大" | "最小" | "累乗" | "切り捨て" | "切り上げ"
        | "四捨五入" | "余り" => Some("数学"),
        "分割" | "結合" | "置換" => Some("文字列"),
        "要素数" | "追加" | "取り出す" | "含む配列" | "位置" | "逆順" | "整列" | "部分列" => {
            Some("配列")
        }
        "鍵一覧" | "値一覧" | "削除" => Some("辞書"),
        "地図" | "絞り込み" | "畳み込み" => Some("関数"),
        _ => None,
    }
}

// Conservative exhaustive-return check: only the LAST statement of a block
// matters for reachability (no dead-code analysis here), and loops never
// count even if their body always returns, since a loop might run zero times.
fn always_returns(stmts: &[Stmt]) -> bool {
    match stmts.last() {
        None => false,
        Some(Stmt::Return(..)) => true,
        Some(Stmt::If {
            then_body,
            else_body: Some(else_body),
            ..
        }) => always_returns(then_body) && always_returns(else_body),
        // A try-body's 返す only guarantees a return if it completes without
        // throwing; since either body always returning only guarantees SOME
        // path returns when BOTH independently always return, requiring both
        // is the safe (if slightly conservative) choice.
        Some(Stmt::TryCatch {
            try_body,
            catch_body,
            ..
        }) => always_returns(try_body) && always_returns(catch_body),
        Some(_) => false,
    }
}

pub struct TypeChecker {
    scopes: Vec<HashMap<String, HikariType>>,
    fns: HashMap<String, FnSig>,
    // Return type expected by the function currently being checked.
    current_return_ty: Option<HikariType>,
    imported_modules: std::collections::HashSet<String>,
    // Number of enclosing 間／繰り返す／各 bodies; 抜ける／続ける require > 0.
    loop_depth: u32,
    // Record type name → ordered (field name, field type) pairs.
    records: HashMap<String, Vec<(String, HikariType)>>,
    // Enum name → ordered (variant name, payload types) pairs.
    enums: HashMap<String, Vec<(String, Vec<HikariType>)>>,
    // Variant name → owning enum name (variant names are globally unique).
    variant_owner: HashMap<String, String>,
}

impl TypeChecker {
    pub fn new() -> Self {
        Self {
            scopes: vec![HashMap::new()],
            fns: HashMap::new(),
            current_return_ty: None,
            imported_modules: std::collections::HashSet::new(),
            loop_depth: 0,
            records: HashMap::new(),
            enums: HashMap::new(),
            variant_owner: HashMap::new(),
        }
    }

    fn enter_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    fn exit_scope(&mut self) {
        self.scopes.pop();
    }

    fn declare_var(&mut self, name: &str, ty: HikariType) {
        self.scopes.last_mut().unwrap().insert(name.to_string(), ty);
    }

    fn lookup_var(&self, name: &str) -> Option<HikariType> {
        self.scopes.iter().rev().find_map(|s| s.get(name).cloned())
    }

    // parse_type's Ident arm accepts ANY identifier as a syntactically valid
    // type, even ones that aren't declared record types, so this is the
    // gatekeeper that rejects undeclared record names wherever a type is
    // actually used (VarDecl, function params/return type).
    fn check_type_declared(&self, ty: &HikariType, span: Span) -> Result<(), TypeError> {
        match ty {
            HikariType::Record(name)
                if !self.records.contains_key(name) && !self.enums.contains_key(name) =>
            {
                Err(TypeError::UndeclaredType(name.clone(), span))
            }
            HikariType::Map(k, v) => {
                self.check_type_declared(k, span)?;
                self.check_type_declared(v, span)
            }
            HikariType::Array(inner) => self.check_type_declared(inner, span),
            HikariType::Fn(params, ret) => {
                for p in params {
                    self.check_type_declared(p, span)?;
                }
                self.check_type_declared(ret, span)
            }
            _ => Ok(()),
        }
    }

    pub fn check(&mut self, stmts: &[Stmt]) -> Result<(), TypeError> {
        for stmt in stmts {
            self.check_stmt(stmt)?;
        }
        Ok(())
    }

    fn check_stmt(&mut self, stmt: &Stmt) -> Result<(), TypeError> {
        match stmt {
            Stmt::VarDecl {
                ty,
                name,
                value,
                span,
            } => {
                self.check_type_declared(ty, *span)?;
                // Special case: an empty map literal ｛｝ is valid if the declared
                // type is a Map — infer_expr can't deduce the element types from nothing,
                // so we skip the inference and trust the annotation.
                if matches!(value, Expr::MapLit(pairs) if pairs.is_empty()) {
                    if !matches!(ty, HikariType::Map(..)) {
                        return Err(TypeError::VarDeclMismatch {
                            name: name.clone(),
                            declared: ty.clone(),
                            got: HikariType::Map(
                                Box::new(HikariType::String),
                                Box::new(HikariType::Void),
                            ),
                            span: *span,
                        });
                    }
                    self.declare_var(name, ty.clone());
                    return Ok(());
                }
                let inferred = self.infer_expr(value, *span)?;
                if inferred != *ty {
                    return Err(TypeError::VarDeclMismatch {
                        name: name.clone(),
                        declared: ty.clone(),
                        got: inferred,
                        span: *span,
                    });
                }
                self.declare_var(name, ty.clone());
                Ok(())
            }

            Stmt::FnDecl {
                name,
                params,
                return_ty,
                body,
                span,
            } => {
                for (ty, _) in params {
                    self.check_type_declared(ty, *span)?;
                }
                self.check_type_declared(return_ty, *span)?;
                let sig = FnSig {
                    params: params.iter().map(|(t, _)| t.clone()).collect(),
                    return_ty: return_ty.clone(),
                };
                self.fns.insert(name.clone(), sig);

                // Function bodies are fully isolated: they get a brand new
                // scope stack, matching the VM's independent per-call Frame.
                let outer_scopes = std::mem::replace(&mut self.scopes, vec![HashMap::new()]);
                let outer_return_ty = self.current_return_ty.take();
                // A 抜ける／続ける written inside a nested 関数 body must not
                // be considered "inside a loop" just because the call site
                // (or even the declaration site) happens to sit inside one.
                let outer_loop_depth = std::mem::take(&mut self.loop_depth);

                for (ty, pname) in params {
                    self.declare_var(pname, ty.clone());
                }
                self.current_return_ty = Some(return_ty.clone());

                self.check(body)?;

                if *return_ty != HikariType::Void && !always_returns(body) {
                    return Err(TypeError::MissingReturn {
                        name: name.clone(),
                        span: *span,
                    });
                }

                self.scopes = outer_scopes;
                self.current_return_ty = outer_return_ty;
                self.loop_depth = outer_loop_depth;
                Ok(())
            }

            Stmt::Return(expr, span) => {
                match expr {
                    Some(expr) => {
                        let got = self.infer_expr(expr, *span)?;
                        if let Some(expected) = &self.current_return_ty
                            && got != *expected
                        {
                            return Err(TypeError::ReturnTypeMismatch {
                                expected: expected.clone(),
                                got,
                                span: *span,
                            });
                        }
                    }
                    None => {
                        if let Some(expected) = &self.current_return_ty
                            && *expected != HikariType::Void
                        {
                            return Err(TypeError::ReturnTypeMismatch {
                                expected: expected.clone(),
                                got: HikariType::Void,
                                span: *span,
                            });
                        }
                    }
                }
                Ok(())
            }

            Stmt::Break(span) => {
                if self.loop_depth == 0 {
                    return Err(TypeError::ControlFlowOutsideLoop {
                        keyword: "抜ける".to_string(),
                        span: *span,
                    });
                }
                Ok(())
            }

            Stmt::Continue(span) => {
                if self.loop_depth == 0 {
                    return Err(TypeError::ControlFlowOutsideLoop {
                        keyword: "続ける".to_string(),
                        span: *span,
                    });
                }
                Ok(())
            }

            Stmt::Print(expr, span) => {
                self.infer_expr(expr, *span)?;
                Ok(())
            }

            Stmt::If {
                condition,
                then_body,
                else_body,
                span,
            } => {
                let cond_ty = self.infer_expr(condition, *span)?;
                if cond_ty != HikariType::Bool {
                    return Err(TypeError::ConditionNotBool(cond_ty, *span));
                }
                self.enter_scope();
                self.check(then_body)?;
                self.exit_scope();
                if let Some(body) = else_body {
                    self.enter_scope();
                    self.check(body)?;
                    self.exit_scope();
                }
                Ok(())
            }

            Stmt::While {
                condition,
                body,
                span,
            } => {
                let cond_ty = self.infer_expr(condition, *span)?;
                if cond_ty != HikariType::Bool {
                    return Err(TypeError::ConditionNotBool(cond_ty, *span));
                }
                self.enter_scope();
                self.loop_depth += 1;
                self.check(body)?;
                self.loop_depth -= 1;
                self.exit_scope();
                Ok(())
            }

            Stmt::Expr(expr, span) => {
                self.infer_expr(expr, *span)?;
                Ok(())
            }

            Stmt::Assign { name, value, span } => {
                let declared = self
                    .lookup_var(name)
                    .ok_or_else(|| TypeError::UndeclaredVariable(name.clone(), *span))?;
                let got = self.infer_expr(value, *span)?;
                if got != declared {
                    return Err(TypeError::VarDeclMismatch {
                        name: name.clone(),
                        declared,
                        got,
                        span: *span,
                    });
                }
                Ok(())
            }

            Stmt::IndexAssign {
                name,
                index,
                value,
                span,
            } => {
                let var_ty = self
                    .lookup_var(name)
                    .ok_or_else(|| TypeError::UndeclaredVariable(name.clone(), *span))?;
                match var_ty {
                    HikariType::Array(elem_ty) => {
                        let index_ty = self.infer_expr(index, *span)?;
                        if index_ty != HikariType::Int {
                            return Err(TypeError::IndexNotInt {
                                got: index_ty,
                                span: *span,
                            });
                        }
                        let value_ty = self.infer_expr(value, *span)?;
                        if value_ty != *elem_ty {
                            return Err(TypeError::ArrayElementTypeMismatch {
                                expected: *elem_ty,
                                got: value_ty,
                                span: *span,
                            });
                        }
                    }
                    HikariType::Map(key_ty, val_ty) => {
                        let index_ty = self.infer_expr(index, *span)?;
                        if index_ty != *key_ty {
                            return Err(TypeError::IndexNotInt {
                                got: index_ty,
                                span: *span,
                            });
                        }
                        let value_ty = self.infer_expr(value, *span)?;
                        if value_ty != *val_ty {
                            return Err(TypeError::ArrayElementTypeMismatch {
                                expected: *val_ty,
                                got: value_ty,
                                span: *span,
                            });
                        }
                    }
                    other => {
                        return Err(TypeError::NotIndexable {
                            got: other,
                            span: *span,
                        });
                    }
                }
                Ok(())
            }

            Stmt::ForRange {
                var,
                from,
                to,
                body,
                span,
            } => {
                let from_ty = self.infer_expr(from, *span)?;
                let to_ty = self.infer_expr(to, *span)?;
                if from_ty != HikariType::Int {
                    return Err(TypeError::ArgTypeMismatch {
                        name: var.clone(),
                        param: HikariType::Int,
                        got: from_ty,
                        span: *span,
                    });
                }
                if to_ty != HikariType::Int {
                    return Err(TypeError::ArgTypeMismatch {
                        name: var.clone(),
                        param: HikariType::Int,
                        got: to_ty,
                        span: *span,
                    });
                }
                self.enter_scope();
                self.declare_var(var, HikariType::Int);
                self.loop_depth += 1;
                self.check(body)?;
                self.loop_depth -= 1;
                self.exit_scope();
                Ok(())
            }

            Stmt::ForEach {
                var,
                array,
                body,
                span,
            } => {
                let array_ty = self.infer_expr(array, *span)?;
                let elem_ty = match array_ty {
                    HikariType::Array(inner) => *inner,
                    other => {
                        return Err(TypeError::NotIndexable {
                            got: other,
                            span: *span,
                        });
                    }
                };
                self.enter_scope();
                self.declare_var(var, elem_ty);
                self.loop_depth += 1;
                self.check(body)?;
                self.loop_depth -= 1;
                self.exit_scope();
                Ok(())
            }

            Stmt::TryCatch {
                try_body,
                error_var,
                catch_body,
                ..
            } => {
                self.enter_scope();
                self.check(try_body)?;
                self.exit_scope();

                self.enter_scope();
                self.declare_var(error_var, HikariType::String);
                self.check(catch_body)?;
                self.exit_scope();
                Ok(())
            }

            Stmt::Import { name, .. } => {
                if name == "数学"
                    || name == "文字列"
                    || name == "配列"
                    || name == "辞書"
                    || name == "関数"
                {
                    self.imported_modules.insert(name.clone());
                }
                Ok(())
            }

            Stmt::TypeDecl { name, fields, .. } => {
                let entry = fields.iter().map(|(t, n)| (n.clone(), t.clone())).collect();
                self.records.insert(name.clone(), entry);
                Ok(())
            }

            Stmt::FieldAssign {
                record,
                field,
                value,
                span,
            } => {
                let record_ty = self.infer_expr(record, *span)?;
                let type_name = match record_ty {
                    HikariType::Record(name) => name,
                    other => {
                        return Err(TypeError::NotARecord {
                            got: other,
                            span: *span,
                        });
                    }
                };
                let field_ty = self
                    .records
                    .get(&type_name)
                    .and_then(|fs| fs.iter().find(|(n, _)| n == field).map(|(_, t)| t.clone()))
                    .ok_or_else(|| TypeError::UnknownField {
                        type_name: type_name.clone(),
                        field: field.clone(),
                        span: *span,
                    })?;
                let value_ty = self.infer_expr(value, *span)?;
                if value_ty != field_ty {
                    return Err(TypeError::FieldTypeMismatch {
                        type_name,
                        field: field.clone(),
                        expected: Box::new(field_ty),
                        got: Box::new(value_ty),
                        span: *span,
                    });
                }
                Ok(())
            }

            Stmt::EnumDecl {
                name,
                variants,
                span,
            } => {
                for (variant_name, _) in variants {
                    if self.variant_owner.contains_key(variant_name) {
                        return Err(TypeError::DuplicateEnumVariant {
                            variant: variant_name.clone(),
                            span: *span,
                        });
                    }
                    self.variant_owner
                        .insert(variant_name.clone(), name.clone());
                }
                self.enums.insert(name.clone(), variants.clone());
                Ok(())
            }

            Stmt::Match {
                subject,
                arms,
                span,
            } => {
                let subject_ty = self.infer_expr(subject, *span)?;
                // Enum-typed variables are stored as Record(enum_name) in the
                // type system (parse_type maps any bare Ident to Record), so
                // we accept Record(name) when name is a registered enum, as
                // well as the explicit Enum(name) form.
                let enum_name = match subject_ty {
                    HikariType::Enum(name) => name,
                    HikariType::Record(name) if self.enums.contains_key(&name) => name,
                    other => {
                        return Err(TypeError::NotAnEnum {
                            got: other,
                            span: *span,
                        });
                    }
                };
                // Guaranteed present: subject already typechecked to
                // Enum(enum_name) or Record(enum_name), so enum_name must be registered.
                let declared_variants = self
                    .enums
                    .get(&enum_name)
                    .expect("enum registered by EnumDecl before any value of its type exists")
                    .clone();

                let mut covered: std::collections::HashSet<String> =
                    std::collections::HashSet::new();
                for arm in arms {
                    if !covered.insert(arm.variant.clone()) {
                        return Err(TypeError::DuplicateMatchArm {
                            variant: arm.variant.clone(),
                            span: *span,
                        });
                    }

                    let payload_types = declared_variants
                        .iter()
                        .find(|(n, _)| n == &arm.variant)
                        .map(|(_, tys)| tys.clone())
                        .ok_or_else(|| TypeError::UndeclaredEnumVariant {
                            enum_name: enum_name.clone(),
                            variant: arm.variant.clone(),
                            span: *span,
                        })?;

                    if arm.binders.len() != payload_types.len() {
                        return Err(TypeError::ArgCountMismatch {
                            name: arm.variant.clone(),
                            expected: payload_types.len(),
                            got: arm.binders.len(),
                            span: *span,
                        });
                    }

                    self.enter_scope();
                    for (binder, ty) in arm.binders.iter().zip(payload_types.iter()) {
                        self.declare_var(binder, ty.clone());
                    }
                    self.check(&arm.body)?;
                    self.exit_scope();
                }

                let missing: Vec<String> = declared_variants
                    .iter()
                    .map(|(n, _)| n.clone())
                    .filter(|n| !covered.contains(n))
                    .collect();
                if !missing.is_empty() {
                    return Err(TypeError::NonExhaustiveMatch(Box::new(
                        NonExhaustiveMatchInfo {
                            enum_name,
                            missing,
                            span: *span,
                        },
                    )));
                }

                Ok(())
            }
        }
    }

    fn infer_expr(&mut self, expr: &Expr, span: Span) -> Result<HikariType, TypeError> {
        match expr {
            Expr::LitInt(_) => Ok(HikariType::Int),
            Expr::LitFloat(_) => Ok(HikariType::Float),
            Expr::LitString(_) => Ok(HikariType::String),
            Expr::LitBool(_) => Ok(HikariType::Bool),

            Expr::Ident(name) => {
                // First look up in local variable scope.
                if let Some(ty) = self.lookup_var(name) {
                    return Ok(ty);
                }
                // Phase 10: a bare identifier that names a known function can
                // be used as a first-class function value.
                if let Some(sig) = self.fns.get(name).cloned() {
                    return Ok(HikariType::Fn(sig.params, Box::new(sig.return_ty)));
                }
                Err(TypeError::UndeclaredVariable(name.clone(), span))
            }

            Expr::BinOp { op, lhs, rhs } => {
                let lty = self.infer_expr(lhs, span)?;
                let rty = self.infer_expr(rhs, span)?;
                if matches!(op, BinOpKind::And | BinOpKind::Or) {
                    if lty != HikariType::Bool {
                        return Err(TypeError::BinOpMismatch {
                            op: op.clone(),
                            lhs: lty,
                            rhs: rty,
                            span,
                        });
                    }
                    if rty != HikariType::Bool {
                        return Err(TypeError::BinOpMismatch {
                            op: op.clone(),
                            lhs: lty,
                            rhs: rty,
                            span,
                        });
                    }
                    return Ok(HikariType::Bool);
                }
                if lty != rty {
                    return Err(TypeError::BinOpMismatch {
                        op: op.clone(),
                        lhs: lty,
                        rhs: rty,
                        span,
                    });
                }
                let numeric = matches!(lty, HikariType::Int | HikariType::Float);
                let mismatch = || TypeError::BinOpMismatch {
                    op: op.clone(),
                    lhs: lty.clone(),
                    rhs: rty.clone(),
                    span,
                };
                match op {
                    // Equality works for any two values of the same type.
                    BinOpKind::Eq | BinOpKind::NotEq => Ok(HikariType::Bool),
                    // Ordering is only defined for numbers (the VM has no
                    // ordering for 文字列/真偽/配列).
                    BinOpKind::Lt | BinOpKind::Gt | BinOpKind::LtEq | BinOpKind::GtEq => {
                        if numeric {
                            Ok(HikariType::Bool)
                        } else {
                            Err(mismatch())
                        }
                    }
                    // ＋ also concatenates strings; the rest are numbers only.
                    BinOpKind::Add => {
                        if numeric || lty == HikariType::String {
                            Ok(lty)
                        } else {
                            Err(mismatch())
                        }
                    }
                    BinOpKind::Sub | BinOpKind::Mul | BinOpKind::Div | BinOpKind::Mod => {
                        if numeric { Ok(lty) } else { Err(mismatch()) }
                    }
                    BinOpKind::And | BinOpKind::Or => unreachable!("handled above"),
                }
            }

            Expr::UnaryMinus(inner) => {
                let ity = self.infer_expr(inner, span)?;
                match ity {
                    HikariType::Int | HikariType::Float => Ok(ity),
                    other => Err(TypeError::UnaryOpMismatch { got: other, span }),
                }
            }

            Expr::UnaryNot(inner) => {
                let ity = self.infer_expr(inner, span)?;
                match ity {
                    HikariType::Bool => Ok(HikariType::Bool),
                    other => Err(TypeError::UnaryOpMismatch { got: other, span }),
                }
            }

            Expr::Call { name, args } => {
                if let Some(owning_enum) = self.variant_owner.get(name).cloned() {
                    let payload_types = self
                        .enums
                        .get(&owning_enum)
                        .and_then(|vs| vs.iter().find(|(n, _)| n == name))
                        .map(|(_, tys)| tys.clone())
                        .expect("variant_owner entry implies a registered enum/variant");
                    if args.len() != payload_types.len() {
                        return Err(TypeError::ArgCountMismatch {
                            name: name.clone(),
                            expected: payload_types.len(),
                            got: args.len(),
                            span,
                        });
                    }
                    for (arg, param_ty) in args.iter().zip(payload_types.iter()) {
                        let arg_ty = self.infer_expr(arg, span)?;
                        if arg_ty != *param_ty {
                            return Err(TypeError::ArgTypeMismatch {
                                name: name.clone(),
                                param: param_ty.clone(),
                                got: arg_ty,
                                span,
                            });
                        }
                    }
                    // Return as Record so the type matches a VarDecl whose
                    // declared type was parsed as HikariType::Record(enum_name).
                    return Ok(HikariType::Record(owning_enum));
                }

                if let Some(module) = builtin_module(name)
                    && !self.imported_modules.contains(module)
                {
                    return Err(TypeError::ModuleNotImported {
                        name: name.clone(),
                        module: module.to_string(),
                        span,
                    });
                }

                if name == "絶対値" || name == "平方根" {
                    if args.len() != 1 {
                        return Err(TypeError::ArgCountMismatch {
                            name: name.clone(),
                            expected: 1,
                            got: args.len(),
                            span,
                        });
                    }
                    let arg_ty = self.infer_expr(&args[0], span)?;
                    if !matches!(arg_ty, HikariType::Int | HikariType::Float) {
                        return Err(TypeError::ArgTypeMismatch {
                            name: name.clone(),
                            param: HikariType::Int,
                            got: arg_ty,
                            span,
                        });
                    }
                    return Ok(if name == "平方根" {
                        HikariType::Float
                    } else {
                        arg_ty
                    });
                }

                if name == "最大" || name == "最小" || name == "累乗" || name == "余り" {
                    if args.len() != 2 {
                        return Err(TypeError::ArgCountMismatch {
                            name: name.clone(),
                            expected: 2,
                            got: args.len(),
                            span,
                        });
                    }
                    let a_ty = self.infer_expr(&args[0], span)?;
                    let b_ty = self.infer_expr(&args[1], span)?;
                    if !matches!(a_ty, HikariType::Int | HikariType::Float) {
                        return Err(TypeError::ArgTypeMismatch {
                            name: name.clone(),
                            param: HikariType::Int,
                            got: a_ty,
                            span,
                        });
                    }
                    if a_ty != b_ty {
                        return Err(TypeError::ArgTypeMismatch {
                            name: name.clone(),
                            param: a_ty,
                            got: b_ty,
                            span,
                        });
                    }
                    return Ok(a_ty);
                }

                if name == "切り捨て" || name == "切り上げ" || name == "四捨五入" {
                    if args.len() != 1 {
                        return Err(TypeError::ArgCountMismatch {
                            name: name.clone(),
                            expected: 1,
                            got: args.len(),
                            span,
                        });
                    }
                    let arg_ty = self.infer_expr(&args[0], span)?;
                    if arg_ty != HikariType::Float {
                        return Err(TypeError::ArgTypeMismatch {
                            name: name.clone(),
                            param: HikariType::Float,
                            got: arg_ty,
                            span,
                        });
                    }
                    return Ok(HikariType::Int);
                }

                if name == "要素数" {
                    if args.len() != 1 {
                        return Err(TypeError::ArgCountMismatch {
                            name: name.clone(),
                            expected: 1,
                            got: args.len(),
                            span,
                        });
                    }
                    let arg_ty = self.infer_expr(&args[0], span)?;
                    if !matches!(arg_ty, HikariType::Array(_)) {
                        return Err(TypeError::ArgTypeMismatch {
                            name: name.clone(),
                            param: HikariType::Array(Box::new(HikariType::Int)),
                            got: arg_ty,
                            span,
                        });
                    }
                    return Ok(HikariType::Int);
                }

                if name == "追加" {
                    if args.len() != 2 {
                        return Err(TypeError::ArgCountMismatch {
                            name: name.clone(),
                            expected: 2,
                            got: args.len(),
                            span,
                        });
                    }
                    let arr_ty = self.infer_expr(&args[0], span)?;
                    let elem_ty = match arr_ty {
                        HikariType::Array(inner) => *inner,
                        other => {
                            return Err(TypeError::ArgTypeMismatch {
                                name: name.clone(),
                                param: HikariType::Array(Box::new(HikariType::Int)),
                                got: other,
                                span,
                            });
                        }
                    };
                    let val_ty = self.infer_expr(&args[1], span)?;
                    if val_ty != elem_ty {
                        return Err(TypeError::ArgTypeMismatch {
                            name: name.clone(),
                            param: elem_ty,
                            got: val_ty,
                            span,
                        });
                    }
                    return Ok(HikariType::Void);
                }

                if name == "取り出す" {
                    if args.len() != 1 {
                        return Err(TypeError::ArgCountMismatch {
                            name: name.clone(),
                            expected: 1,
                            got: args.len(),
                            span,
                        });
                    }
                    let arr_ty = self.infer_expr(&args[0], span)?;
                    let elem_ty = match arr_ty {
                        HikariType::Array(inner) => *inner,
                        other => {
                            return Err(TypeError::ArgTypeMismatch {
                                name: name.clone(),
                                param: HikariType::Array(Box::new(HikariType::Int)),
                                got: other,
                                span,
                            });
                        }
                    };
                    return Ok(elem_ty);
                }

                if name == "含む配列" || name == "位置" {
                    if args.len() != 2 {
                        return Err(TypeError::ArgCountMismatch {
                            name: name.clone(),
                            expected: 2,
                            got: args.len(),
                            span,
                        });
                    }
                    let arr_ty = self.infer_expr(&args[0], span)?;
                    let elem_ty = match arr_ty {
                        HikariType::Array(inner) => *inner,
                        other => {
                            return Err(TypeError::ArgTypeMismatch {
                                name: name.clone(),
                                param: HikariType::Array(Box::new(HikariType::Int)),
                                got: other,
                                span,
                            });
                        }
                    };
                    let val_ty = self.infer_expr(&args[1], span)?;
                    if val_ty != elem_ty {
                        return Err(TypeError::ArgTypeMismatch {
                            name: name.clone(),
                            param: elem_ty,
                            got: val_ty,
                            span,
                        });
                    }
                    return Ok(if name == "位置" {
                        HikariType::Int
                    } else {
                        HikariType::Bool
                    });
                }

                if name == "逆順" {
                    if args.len() != 1 {
                        return Err(TypeError::ArgCountMismatch {
                            name: name.clone(),
                            expected: 1,
                            got: args.len(),
                            span,
                        });
                    }
                    let arr_ty = self.infer_expr(&args[0], span)?;
                    match arr_ty {
                        HikariType::Array(inner) => return Ok(HikariType::Array(inner)),
                        other => {
                            return Err(TypeError::ArgTypeMismatch {
                                name: name.clone(),
                                param: HikariType::Array(Box::new(HikariType::Int)),
                                got: other,
                                span,
                            });
                        }
                    }
                }

                if name == "整列" {
                    if args.len() != 1 {
                        return Err(TypeError::ArgCountMismatch {
                            name: name.clone(),
                            expected: 1,
                            got: args.len(),
                            span,
                        });
                    }
                    let arr_ty = self.infer_expr(&args[0], span)?;
                    match &arr_ty {
                        HikariType::Array(inner)
                            if matches!(
                                inner.as_ref(),
                                HikariType::Int | HikariType::Float | HikariType::String
                            ) =>
                        {
                            return Ok(arr_ty);
                        }
                        _ => {
                            return Err(TypeError::ArgTypeMismatch {
                                name: name.clone(),
                                param: HikariType::Array(Box::new(HikariType::Int)),
                                got: arr_ty,
                                span,
                            });
                        }
                    }
                }

                if name == "部分列" {
                    if args.len() != 3 {
                        return Err(TypeError::ArgCountMismatch {
                            name: name.clone(),
                            expected: 3,
                            got: args.len(),
                            span,
                        });
                    }
                    let arr_ty = self.infer_expr(&args[0], span)?;
                    if !matches!(arr_ty, HikariType::Array(_)) {
                        return Err(TypeError::ArgTypeMismatch {
                            name: name.clone(),
                            param: HikariType::Array(Box::new(HikariType::Int)),
                            got: arr_ty,
                            span,
                        });
                    }
                    let start_ty = self.infer_expr(&args[1], span)?;
                    if start_ty != HikariType::Int {
                        return Err(TypeError::ArgTypeMismatch {
                            name: name.clone(),
                            param: HikariType::Int,
                            got: start_ty,
                            span,
                        });
                    }
                    let end_ty = self.infer_expr(&args[2], span)?;
                    if end_ty != HikariType::Int {
                        return Err(TypeError::ArgTypeMismatch {
                            name: name.clone(),
                            param: HikariType::Int,
                            got: end_ty,
                            span,
                        });
                    }
                    return Ok(arr_ty);
                }

                // Map builtins (require 辞書 module import).
                if name == "鍵一覧" {
                    if args.len() != 1 {
                        return Err(TypeError::ArgCountMismatch {
                            name: name.clone(),
                            expected: 1,
                            got: args.len(),
                            span,
                        });
                    }
                    let arg_ty = self.infer_expr(&args[0], span)?;
                    match arg_ty {
                        HikariType::Map(k, _) => {
                            return Ok(HikariType::Array(k));
                        }
                        other => {
                            return Err(TypeError::ArgTypeMismatch {
                                name: name.clone(),
                                param: HikariType::Map(
                                    Box::new(HikariType::String),
                                    Box::new(HikariType::Void),
                                ),
                                got: other,
                                span,
                            });
                        }
                    }
                }

                if name == "値一覧" {
                    if args.len() != 1 {
                        return Err(TypeError::ArgCountMismatch {
                            name: name.clone(),
                            expected: 1,
                            got: args.len(),
                            span,
                        });
                    }
                    let arg_ty = self.infer_expr(&args[0], span)?;
                    match arg_ty {
                        HikariType::Map(_, v) => {
                            return Ok(HikariType::Array(v));
                        }
                        other => {
                            return Err(TypeError::ArgTypeMismatch {
                                name: name.clone(),
                                param: HikariType::Map(
                                    Box::new(HikariType::String),
                                    Box::new(HikariType::Void),
                                ),
                                got: other,
                                span,
                            });
                        }
                    }
                }

                if name == "削除" {
                    if args.len() != 2 {
                        return Err(TypeError::ArgCountMismatch {
                            name: name.clone(),
                            expected: 2,
                            got: args.len(),
                            span,
                        });
                    }
                    let map_ty = self.infer_expr(&args[0], span)?;
                    let key_ty = match map_ty {
                        HikariType::Map(k, _) => *k,
                        other => {
                            return Err(TypeError::ArgTypeMismatch {
                                name: name.clone(),
                                param: HikariType::Map(
                                    Box::new(HikariType::String),
                                    Box::new(HikariType::Void),
                                ),
                                got: other,
                                span,
                            });
                        }
                    };
                    let arg_key_ty = self.infer_expr(&args[1], span)?;
                    if arg_key_ty != key_ty {
                        return Err(TypeError::ArgTypeMismatch {
                            name: name.clone(),
                            param: key_ty,
                            got: arg_key_ty,
                            span,
                        });
                    }
                    return Ok(HikariType::Void);
                }

                // 含む is polymorphic: String × String → Bool (文字列 module)
                // or Map × Key → Bool (辞書 module).
                if name == "含む" && args.len() == 2 {
                    let first_ty = self.infer_expr(&args[0], span)?;
                    if let HikariType::Map(k, _) = &first_ty {
                        // Map membership check — requires 辞書 import.
                        if !self.imported_modules.contains("辞書") {
                            return Err(TypeError::ModuleNotImported {
                                name: name.clone(),
                                module: "辞書".to_string(),
                                span,
                            });
                        }
                        let key_arg = self.infer_expr(&args[1], span)?;
                        if key_arg != **k {
                            return Err(TypeError::ArgTypeMismatch {
                                name: name.clone(),
                                param: (**k).clone(),
                                got: key_arg,
                                span,
                            });
                        }
                        return Ok(HikariType::Bool);
                    }
                    // Fall through to 文字列-module 含む (String × String).
                    if !self.imported_modules.contains("文字列") {
                        return Err(TypeError::ModuleNotImported {
                            name: name.clone(),
                            module: "文字列".to_string(),
                            span,
                        });
                    }
                    let second_ty = self.infer_expr(&args[1], span)?;
                    if first_ty != HikariType::String || second_ty != HikariType::String {
                        return Err(TypeError::ArgTypeMismatch {
                            name: name.clone(),
                            param: HikariType::String,
                            got: if first_ty != HikariType::String {
                                first_ty
                            } else {
                                second_ty
                            },
                            span,
                        });
                    }
                    return Ok(HikariType::Bool);
                }

                // Phase 10: higher-order function builtins.
                if name == "地図" {
                    if args.len() != 2 {
                        return Err(TypeError::ArgCountMismatch {
                            name: name.clone(),
                            expected: 2,
                            got: args.len(),
                            span,
                        });
                    }
                    let arr_ty = self.infer_expr(&args[0], span)?;
                    let elem_ty = match arr_ty {
                        HikariType::Array(inner) => *inner,
                        other => {
                            return Err(TypeError::ArgTypeMismatch {
                                name: name.clone(),
                                param: HikariType::Array(Box::new(HikariType::Int)),
                                got: other,
                                span,
                            });
                        }
                    };
                    let fn_ty = self.infer_expr(&args[1], span)?;
                    let ret_ty = match fn_ty {
                        HikariType::Fn(params, ret)
                            if params.len() == 1 && params[0] == elem_ty =>
                        {
                            *ret
                        }
                        other => {
                            return Err(TypeError::ArgTypeMismatch {
                                name: name.clone(),
                                param: HikariType::Fn(vec![elem_ty], Box::new(HikariType::Int)),
                                got: other,
                                span,
                            });
                        }
                    };
                    return Ok(HikariType::Array(Box::new(ret_ty)));
                }

                if name == "絞り込み" {
                    if args.len() != 2 {
                        return Err(TypeError::ArgCountMismatch {
                            name: name.clone(),
                            expected: 2,
                            got: args.len(),
                            span,
                        });
                    }
                    let arr_ty = self.infer_expr(&args[0], span)?;
                    let elem_ty = match arr_ty.clone() {
                        HikariType::Array(inner) => *inner,
                        other => {
                            return Err(TypeError::ArgTypeMismatch {
                                name: name.clone(),
                                param: HikariType::Array(Box::new(HikariType::Int)),
                                got: other,
                                span,
                            });
                        }
                    };
                    let fn_ty = self.infer_expr(&args[1], span)?;
                    match fn_ty {
                        HikariType::Fn(params, ret)
                            if params.len() == 1
                                && params[0] == elem_ty
                                && *ret == HikariType::Bool => {}
                        other => {
                            return Err(TypeError::ArgTypeMismatch {
                                name: name.clone(),
                                param: HikariType::Fn(
                                    vec![elem_ty.clone()],
                                    Box::new(HikariType::Bool),
                                ),
                                got: other,
                                span,
                            });
                        }
                    }
                    return Ok(arr_ty);
                }

                if name == "畳み込み" {
                    if args.len() != 3 {
                        return Err(TypeError::ArgCountMismatch {
                            name: name.clone(),
                            expected: 3,
                            got: args.len(),
                            span,
                        });
                    }
                    let arr_ty = self.infer_expr(&args[0], span)?;
                    let elem_ty = match arr_ty {
                        HikariType::Array(inner) => *inner,
                        other => {
                            return Err(TypeError::ArgTypeMismatch {
                                name: name.clone(),
                                param: HikariType::Array(Box::new(HikariType::Int)),
                                got: other,
                                span,
                            });
                        }
                    };
                    let acc_ty = self.infer_expr(&args[1], span)?;
                    let fn_ty = self.infer_expr(&args[2], span)?;
                    match &fn_ty {
                        HikariType::Fn(params, ret)
                            if params.len() == 2
                                && params[0] == acc_ty
                                && params[1] == elem_ty
                                && **ret == acc_ty => {}
                        other => {
                            return Err(TypeError::ArgTypeMismatch {
                                name: name.clone(),
                                param: HikariType::Fn(
                                    vec![acc_ty.clone(), elem_ty],
                                    Box::new(acc_ty.clone()),
                                ),
                                got: other.clone(),
                                span,
                            });
                        }
                    }
                    return Ok(acc_ty);
                }

                if let Some(sig) = builtin_sig(name) {
                    if args.len() != sig.params.len() {
                        return Err(TypeError::ArgCountMismatch {
                            name: name.clone(),
                            expected: sig.params.len(),
                            got: args.len(),
                            span,
                        });
                    }
                    if name == "文字列化" {
                        let arg_ty = self.infer_expr(&args[0], span)?;
                        if !matches!(
                            arg_ty,
                            HikariType::Int | HikariType::Float | HikariType::Bool
                        ) {
                            return Err(TypeError::ArgTypeMismatch {
                                name: name.clone(),
                                param: HikariType::Int,
                                got: arg_ty,
                                span,
                            });
                        }
                    } else {
                        for (arg, param_ty) in args.iter().zip(sig.params.iter()) {
                            let arg_ty = self.infer_expr(arg, span)?;
                            if arg_ty != *param_ty {
                                return Err(TypeError::ArgTypeMismatch {
                                    name: name.clone(),
                                    param: param_ty.clone(),
                                    got: arg_ty,
                                    span,
                                });
                            }
                        }
                    }
                    return Ok(sig.return_ty);
                }

                // Phase 10: check if name is a Fn-typed local variable.
                if let Some(var_ty) = self.lookup_var(name) {
                    match var_ty {
                        HikariType::Fn(params, ret) => {
                            if args.len() != params.len() {
                                return Err(TypeError::ArgCountMismatch {
                                    name: name.clone(),
                                    expected: params.len(),
                                    got: args.len(),
                                    span,
                                });
                            }
                            for (arg, param_ty) in args.iter().zip(params.iter()) {
                                let arg_ty = self.infer_expr(arg, span)?;
                                if arg_ty != *param_ty {
                                    return Err(TypeError::ArgTypeMismatch {
                                        name: name.clone(),
                                        param: param_ty.clone(),
                                        got: arg_ty,
                                        span,
                                    });
                                }
                            }
                            return Ok(*ret);
                        }
                        _ => {
                            return Err(TypeError::UndeclaredFunction(name.clone(), span));
                        }
                    }
                }

                let sig = self
                    .fns
                    .get(name)
                    .cloned()
                    .ok_or_else(|| TypeError::UndeclaredFunction(name.clone(), span))?;
                if args.len() != sig.params.len() {
                    return Err(TypeError::ArgCountMismatch {
                        name: name.clone(),
                        expected: sig.params.len(),
                        got: args.len(),
                        span,
                    });
                }
                for (arg, param_ty) in args.iter().zip(sig.params.iter()) {
                    let arg_ty = self.infer_expr(arg, span)?;
                    if arg_ty != *param_ty {
                        return Err(TypeError::ArgTypeMismatch {
                            name: name.clone(),
                            param: param_ty.clone(),
                            got: arg_ty,
                            span,
                        });
                    }
                }
                Ok(sig.return_ty)
            }

            Expr::Array(elems) => {
                let Some(first) = elems.first() else {
                    return Err(TypeError::EmptyArrayLiteral(span));
                };
                let expected = self.infer_expr(first, span)?;
                for elem in &elems[1..] {
                    let got = self.infer_expr(elem, span)?;
                    if got != expected {
                        return Err(TypeError::ArrayElementTypeMismatch {
                            expected,
                            got,
                            span,
                        });
                    }
                }
                Ok(HikariType::Array(Box::new(expected)))
            }

            Expr::Index { array, index } => {
                let array_ty = self.infer_expr(array, span)?;
                match array_ty {
                    HikariType::Array(inner) => {
                        let index_ty = self.infer_expr(index, span)?;
                        if index_ty != HikariType::Int {
                            return Err(TypeError::IndexNotInt {
                                got: index_ty,
                                span,
                            });
                        }
                        Ok(*inner)
                    }
                    HikariType::Map(key_ty, val_ty) => {
                        let index_ty = self.infer_expr(index, span)?;
                        if index_ty != *key_ty {
                            return Err(TypeError::IndexNotInt {
                                got: index_ty,
                                span,
                            });
                        }
                        Ok(*val_ty)
                    }
                    other => Err(TypeError::NotIndexable { got: other, span }),
                }
            }

            Expr::MapLit(pairs) => {
                // Empty map literal ｛｝ is handled by the VarDecl special-case
                // above; reaching here with an empty literal is an error.
                let Some((first_k, first_v)) = pairs.first() else {
                    return Err(TypeError::EmptyArrayLiteral(span));
                };
                let key_ty = self.infer_expr(first_k, span)?;
                if key_ty != HikariType::String {
                    return Err(TypeError::IndexNotInt { got: key_ty, span });
                }
                let val_ty = self.infer_expr(first_v, span)?;
                for (k, v) in &pairs[1..] {
                    let kt = self.infer_expr(k, span)?;
                    if kt != HikariType::String {
                        return Err(TypeError::IndexNotInt { got: kt, span });
                    }
                    let vt = self.infer_expr(v, span)?;
                    if vt != val_ty {
                        return Err(TypeError::ArrayElementTypeMismatch {
                            expected: val_ty.clone(),
                            got: vt,
                            span,
                        });
                    }
                }
                Ok(HikariType::Map(
                    Box::new(HikariType::String),
                    Box::new(val_ty),
                ))
            }

            Expr::NewArray(ty) => Ok(HikariType::Array(Box::new(ty.clone()))),

            Expr::RecordLit { type_name, fields } => {
                let declared = self
                    .records
                    .get(type_name)
                    .ok_or_else(|| TypeError::UndeclaredType(type_name.clone(), span))?
                    .clone();

                let provided: std::collections::HashSet<&str> =
                    fields.iter().map(|(n, _)| n.as_str()).collect();
                let required: std::collections::HashSet<&str> =
                    declared.iter().map(|(n, _)| n.as_str()).collect();

                if let Some(missing) = required.difference(&provided).next() {
                    return Err(TypeError::MissingField {
                        type_name: type_name.clone(),
                        field: missing.to_string(),
                        span,
                    });
                }
                if let Some(extra) = provided.difference(&required).next() {
                    return Err(TypeError::UnknownField {
                        type_name: type_name.clone(),
                        field: extra.to_string(),
                        span,
                    });
                }

                for (fname, fexpr) in fields {
                    let expected = declared
                        .iter()
                        .find(|(n, _)| n == fname)
                        .map(|(_, t)| t.clone())
                        .expect("field presence already validated above");
                    let got = self.infer_expr(fexpr, span)?;
                    if got != expected {
                        return Err(TypeError::FieldTypeMismatch {
                            type_name: type_name.clone(),
                            field: fname.clone(),
                            expected: Box::new(expected),
                            got: Box::new(got),
                            span,
                        });
                    }
                }

                Ok(HikariType::Record(type_name.clone()))
            }

            Expr::FieldAccess { record, field } => {
                let record_ty = self.infer_expr(record, span)?;
                let type_name = match record_ty {
                    HikariType::Record(name) => name,
                    other => return Err(TypeError::NotARecord { got: other, span }),
                };
                self.records
                    .get(&type_name)
                    .and_then(|fs| fs.iter().find(|(n, _)| n == field).map(|(_, t)| t.clone()))
                    .ok_or_else(|| TypeError::UnknownField {
                        type_name,
                        field: field.clone(),
                        span,
                    })
            }

            // Phase 10: anonymous function
            Expr::Lambda {
                params,
                return_ty,
                body,
            } => {
                // Type-check the lambda body in an isolated function scope.
                let outer_scopes = std::mem::replace(&mut self.scopes, vec![HashMap::new()]);
                let outer_return_ty = self.current_return_ty.take();
                let outer_loop_depth = std::mem::take(&mut self.loop_depth);

                for (pname, pty) in params {
                    self.check_type_declared(pty, span)?;
                    self.declare_var(pname, pty.clone());
                }
                self.check_type_declared(return_ty, span)?;
                self.current_return_ty = Some(return_ty.clone());

                self.check(body)?;

                // Restore outer context.
                self.scopes = outer_scopes;
                self.current_return_ty = outer_return_ty;
                self.loop_depth = outer_loop_depth;

                let param_types: Vec<HikariType> = params.iter().map(|(_, t)| t.clone()).collect();
                Ok(HikariType::Fn(param_types, Box::new(return_ty.clone())))
            }
        }
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::Lexer;
    use crate::parser::{Parser, Stmt};

    fn parse(src: &str) -> Vec<Stmt> {
        Parser::new(Lexer::new(src).tokenize()).parse().unwrap()
    }

    #[test]
    fn test_typecheck_valid_var_decl() {
        // 整数 年齢 ＝ ２０；  — declared Int, assigned Int literal: OK
        let ast = parse("整数 年齢 ＝ ２０；");
        assert!(TypeChecker::new().check(&ast).is_ok());
    }

    #[test]
    fn test_typecheck_type_mismatch_var_decl() {
        // 整数 名前 ＝ 「太郎」；  — declared Int, assigned String: must fail
        let ast = parse("整数 名前 ＝ 「太郎」；");
        let err = TypeChecker::new().check(&ast).unwrap_err();
        assert!(matches!(
            err,
            TypeError::VarDeclMismatch {
                declared: HikariType::Int,
                got: HikariType::String,
                ..
            }
        ));
    }

    #[test]
    fn test_typecheck_binop_type_mismatch() {
        // 整数 結果 ＝ １ ＋ 「文字」；  — Int + String: must fail
        let ast = parse("整数 結果 ＝ １ ＋ 「文字」；");
        let err = TypeChecker::new().check(&ast).unwrap_err();
        assert!(matches!(
            err,
            TypeError::BinOpMismatch {
                lhs: HikariType::Int,
                rhs: HikariType::String,
                ..
            }
        ));
    }

    #[test]
    fn test_typecheck_undeclared_variable() {
        // 返す 年齢；  — 年齢 never declared
        let ast = parse("返す 年齢；");
        let err = TypeChecker::new().check(&ast).unwrap_err();
        assert!(matches!(err, TypeError::UndeclaredVariable(n, _) if n == "年齢"));
    }

    #[test]
    fn test_typecheck_valid_function() {
        // 関数 計算（整数 Ａ）ー＞ 整数 ｛ 返す Ａ ＋ １； ｝
        let src = "関数 計算（整数 Ａ）ー＞ 整数 ｛ 返す Ａ ＋ １； ｝";
        let ast = parse(src);
        assert!(TypeChecker::new().check(&ast).is_ok());
    }

    #[test]
    fn test_typecheck_bool_literal_as_if_condition() {
        // 真偽 フラグ ＝ 真；もし フラグ ならば ｛ 印刷（１）； ｝
        let ast = parse("真偽 フラグ ＝ 真；もし フラグ ならば ｛ 印刷（１）； ｝");
        assert!(TypeChecker::new().check(&ast).is_ok());
    }

    #[test]
    fn test_typecheck_while_valid() {
        let src = "整数 Ｎ ＝ ０；間 Ｎ ＜ ３ ならば ｛ 整数 Ｎ ＝ Ｎ ＋ １； ｝";
        let ast = parse(src);
        assert!(TypeChecker::new().check(&ast).is_ok());
    }

    #[test]
    fn test_typecheck_while_non_bool_condition() {
        let src = "整数 Ｎ ＝ ０；間 Ｎ ならば ｛ 整数 Ｎ ＝ Ｎ ＋ １； ｝";
        let ast = parse(src);
        let err = TypeChecker::new().check(&ast).unwrap_err();
        assert!(matches!(
            err,
            TypeError::ConditionNotBool(HikariType::Int, _)
        ));
    }

    #[test]
    fn test_typecheck_if_non_bool_condition() {
        let src = "整数 Ｎ ＝ ０；もし Ｎ ならば ｛ 印刷（Ｎ）； ｝";
        let ast = parse(src);
        let err = TypeChecker::new().check(&ast).unwrap_err();
        assert!(matches!(
            err,
            TypeError::ConditionNotBool(HikariType::Int, _)
        ));
    }

    #[test]
    fn test_typecheck_reassignment_valid() {
        let ast = parse("整数 年齢 ＝ ２０；年齢 ＝ ３０；");
        assert!(TypeChecker::new().check(&ast).is_ok());
    }

    #[test]
    fn test_typecheck_string_concat() {
        let ast = parse("文字列 結果 ＝ 「あ」 ＋ 「い」；");
        assert!(TypeChecker::new().check(&ast).is_ok());
    }

    #[test]
    fn test_typecheck_reassignment_type_mismatch() {
        // 整数 年齢 ＝ ２０； 年齢 ＝ 「太郎」；
        let ast = parse("整数 年齢 ＝ ２０；年齢 ＝ 「太郎」；");
        let err = TypeChecker::new().check(&ast).unwrap_err();
        assert!(matches!(
            err,
            TypeError::VarDeclMismatch {
                declared: HikariType::Int,
                got: HikariType::String,
                ..
            }
        ));
    }

    #[test]
    fn test_typecheck_builtin_strlen() {
        let ast = parse("整数 結果 ＝ 文字数（「あ」）；");
        assert!(TypeChecker::new().check(&ast).is_ok());
    }

    #[test]
    fn test_typecheck_builtin_strlen_arg_type_mismatch() {
        let ast = parse("整数 結果 ＝ 文字数（１）；");
        let err = TypeChecker::new().check(&ast).unwrap_err();
        assert!(matches!(
            err,
            TypeError::ArgTypeMismatch {
                param: HikariType::String,
                got: HikariType::Int,
                ..
            }
        ));
    }

    #[test]
    fn test_typecheck_builtin_input() {
        let ast = parse("文字列 結果 ＝ 入力（）；");
        assert!(TypeChecker::new().check(&ast).is_ok());
    }

    #[test]
    fn test_typecheck_builtin_input_arg_count_mismatch() {
        let ast = parse("文字列 結果 ＝ 入力（「余分」）；");
        let err = TypeChecker::new().check(&ast).unwrap_err();
        assert!(matches!(
            err,
            TypeError::ArgCountMismatch {
                expected: 0,
                got: 1,
                ..
            }
        ));
    }

    #[test]
    fn test_typecheck_builtin_parse_int() {
        let ast = parse("整数 結果 ＝ 整数化（「４２」）；");
        assert!(TypeChecker::new().check(&ast).is_ok());
    }

    #[test]
    fn test_typecheck_builtin_parse_float() {
        let ast = parse("小数 結果 ＝ 小数化（「３．５」）；");
        assert!(TypeChecker::new().check(&ast).is_ok());
    }

    #[test]
    fn test_typecheck_builtin_to_str_polymorphic() {
        let ast = parse("文字列 結果 ＝ 文字列化（１）；");
        assert!(TypeChecker::new().check(&ast).is_ok());

        let ast = parse("文字列 結果 ＝ 文字列化（１．５）；");
        assert!(TypeChecker::new().check(&ast).is_ok());

        let ast = parse("文字列 結果 ＝ 文字列化（真）；");
        assert!(TypeChecker::new().check(&ast).is_ok());
    }

    #[test]
    fn test_typecheck_builtin_to_str_rejects_string_arg() {
        let ast = parse("文字列 結果 ＝ 文字列化（「だめ」）；");
        let err = TypeChecker::new().check(&ast).unwrap_err();
        assert!(matches!(
            err,
            TypeError::ArgTypeMismatch {
                got: HikariType::String,
                ..
            }
        ));
    }

    #[test]
    fn test_typecheck_reassignment_undeclared_variable() {
        let ast = parse("年齢 ＝ ２０；");
        let err = TypeChecker::new().check(&ast).unwrap_err();
        assert!(matches!(err, TypeError::UndeclaredVariable(n, _) if n == "年齢"));
    }

    #[test]
    fn test_typecheck_unary_minus_int_ok() {
        let ast = parse("整数 結果 ＝ ー５；");
        assert!(TypeChecker::new().check(&ast).is_ok());
    }

    #[test]
    fn test_typecheck_unary_minus_on_bool_fails() {
        let ast = parse("真偽 フラグ ＝ 真；整数 結果 ＝ ーフラグ；");
        let err = TypeChecker::new().check(&ast).unwrap_err();
        assert!(matches!(
            err,
            TypeError::UnaryOpMismatch {
                got: HikariType::Bool,
                ..
            }
        ));
    }

    #[test]
    fn test_typecheck_logical_and_or_require_bool() {
        let ast = parse("真偽 結果 ＝ 真 かつ 偽；");
        assert!(TypeChecker::new().check(&ast).is_ok());

        let ast = parse("真偽 結果 ＝ 真 または 偽；");
        assert!(TypeChecker::new().check(&ast).is_ok());

        let ast = parse("真偽 結果 ＝ １ かつ 真；");
        let err = TypeChecker::new().check(&ast).unwrap_err();
        assert!(matches!(err, TypeError::BinOpMismatch { .. }));
    }

    #[test]
    fn test_typecheck_unary_not_requires_bool() {
        let ast = parse("真偽 結果 ＝ 否定 真；");
        assert!(TypeChecker::new().check(&ast).is_ok());

        let ast = parse("真偽 結果 ＝ 否定 １；");
        let err = TypeChecker::new().check(&ast).unwrap_err();
        assert!(matches!(
            err,
            TypeError::UnaryOpMismatch {
                got: HikariType::Int,
                ..
            }
        ));
    }

    #[test]
    fn test_typecheck_additional_comparison_operators() {
        let ast = parse("真偽 結果 ＝ ３ ≦ ５；");
        assert!(TypeChecker::new().check(&ast).is_ok());
        let ast = parse("真偽 結果 ＝ ５ ≧ ３；");
        assert!(TypeChecker::new().check(&ast).is_ok());
        let ast = parse("真偽 結果 ＝ １ ≠ ２；");
        assert!(TypeChecker::new().check(&ast).is_ok());
    }

    #[test]
    fn test_typecheck_return_type_mismatch() {
        // Function declared ー＞ 整数 but returns a 文字列 literal: must fail
        let src = "関数 誤り（）ー＞ 整数 ｛ 返す 「間違い」； ｝";
        let ast = parse(src);
        let err = TypeChecker::new().check(&ast).unwrap_err();
        assert!(matches!(
            err,
            TypeError::ReturnTypeMismatch {
                expected: HikariType::Int,
                got: HikariType::String,
                ..
            }
        ));
    }

    #[test]
    fn test_typecheck_array_literal_valid() {
        let ast = parse("整数列 数字 ＝ 【１、２、３】；");
        assert!(TypeChecker::new().check(&ast).is_ok());
    }

    #[test]
    fn test_typecheck_array_element_type_mismatch() {
        let ast = parse("整数列 数字 ＝ 【１、「あ」】；");
        let err = TypeChecker::new().check(&ast).unwrap_err();
        assert!(matches!(
            err,
            TypeError::ArrayElementTypeMismatch {
                expected: HikariType::Int,
                got: HikariType::String,
                ..
            }
        ));
    }

    #[test]
    fn test_typecheck_empty_array_literal() {
        let ast = parse("整数列 数字 ＝ 【】；");
        let err = TypeChecker::new().check(&ast).unwrap_err();
        assert!(matches!(err, TypeError::EmptyArrayLiteral(_)));
    }

    #[test]
    fn test_typecheck_index_non_array() {
        let ast = parse("整数 値 ＝ ５；返す 値【０】；");
        let err = TypeChecker::new().check(&ast).unwrap_err();
        assert!(matches!(
            err,
            TypeError::NotIndexable {
                got: HikariType::Int,
                ..
            }
        ));
    }

    #[test]
    fn test_typecheck_index_not_int() {
        let ast = parse("整数列 数字 ＝ 【１、２】；返す 数字【「あ」】；");
        let err = TypeChecker::new().check(&ast).unwrap_err();
        assert!(matches!(
            err,
            TypeError::IndexNotInt {
                got: HikariType::String,
                ..
            }
        ));
    }

    #[test]
    fn test_typecheck_index_assign_valid() {
        let ast = parse("整数列 数字 ＝ 【１、２】；数字【０】＝ ９；");
        assert!(TypeChecker::new().check(&ast).is_ok());
    }

    #[test]
    fn test_typecheck_index_assign_type_mismatch() {
        let ast = parse("整数列 数字 ＝ 【１、２】；数字【０】＝ 「あ」；");
        let err = TypeChecker::new().check(&ast).unwrap_err();
        assert!(matches!(
            err,
            TypeError::ArrayElementTypeMismatch {
                expected: HikariType::Int,
                got: HikariType::String,
                ..
            }
        ));
    }

    #[test]
    fn test_typecheck_for_range_valid() {
        let src = "繰り返す カウンタ ＝ ０ から ５ ならば ｛ 印刷（カウンタ）； ｝";
        let ast = parse(src);
        assert!(TypeChecker::new().check(&ast).is_ok());
    }

    #[test]
    fn test_typecheck_for_range_non_int_bound() {
        let src = "繰り返す カウンタ ＝ 「あ」 から ５ ならば ｛ 印刷（カウンタ）； ｝";
        let ast = parse(src);
        let err = TypeChecker::new().check(&ast).unwrap_err();
        assert!(matches!(err, TypeError::ArgTypeMismatch { .. }));
    }

    #[test]
    fn test_typecheck_for_each_valid() {
        let src = "整数列 数字 ＝ 【１、２、３】；各 要素 ： 数字 ならば ｛ 印刷（要素）； ｝";
        let ast = parse(src);
        assert!(TypeChecker::new().check(&ast).is_ok());
    }

    #[test]
    fn test_typecheck_for_each_non_array() {
        let src = "整数 数字 ＝ ５；各 要素 ： 数字 ならば ｛ 印刷（要素）； ｝";
        let ast = parse(src);
        let err = TypeChecker::new().check(&ast).unwrap_err();
        assert!(matches!(err, TypeError::NotIndexable { .. }));
    }

    #[test]
    fn test_typecheck_if_body_var_not_visible_after_block() {
        let src = "もし 真 ならば ｛ 整数 Ｎ ＝ ５； ｝ 返す Ｎ；";
        let ast = parse(src);
        let err = TypeChecker::new().check(&ast).unwrap_err();
        assert!(matches!(err, TypeError::UndeclaredVariable(n, _) if n == "Ｎ"));
    }

    #[test]
    fn test_typecheck_while_body_var_not_visible_after_block() {
        let src = "間 真 ならば ｛ 整数 Ｎ ＝ ５； ｝ 返す Ｎ；";
        let ast = parse(src);
        let err = TypeChecker::new().check(&ast).unwrap_err();
        assert!(matches!(err, TypeError::UndeclaredVariable(n, _) if n == "Ｎ"));
    }

    #[test]
    fn test_typecheck_for_range_var_not_visible_after_loop() {
        let src = "繰り返す カウンタ ＝ ０ から ５ ならば ｛ 印刷（カウンタ）； ｝返す カウンタ；";
        let ast = parse(src);
        let err = TypeChecker::new().check(&ast).unwrap_err();
        assert!(matches!(err, TypeError::UndeclaredVariable(n, _) if n == "カウンタ"));
    }

    #[test]
    fn test_typecheck_for_each_var_not_visible_after_loop() {
        let src =
            "整数列 数字 ＝ 【１、２】；各 要素 ： 数字 ならば ｛ 印刷（要素）； ｝返す 要素；";
        let ast = parse(src);
        let err = TypeChecker::new().check(&ast).unwrap_err();
        assert!(matches!(err, TypeError::UndeclaredVariable(n, _) if n == "要素"));
    }

    #[test]
    fn test_typecheck_outer_var_visible_inside_nested_block() {
        let src = "整数 外 ＝ １０；もし 真 ならば ｛ 間 真 ならば ｛ 印刷（外）； ｝ ｝";
        let ast = parse(src);
        assert!(TypeChecker::new().check(&ast).is_ok());
    }

    #[test]
    fn test_typecheck_shadowing_does_not_corrupt_outer_type() {
        // Outer 値 is Int; inner block shadows it with String, then exits.
        // After the block, 値 should still be Int, so adding it to an Int works.
        let src =
            "整数 値 ＝ １；もし 真 ならば ｛ 文字列 値 ＝ 「あ」； ｝ 整数 結果 ＝ 値 ＋ ２；";
        let ast = parse(src);
        assert!(TypeChecker::new().check(&ast).is_ok());
    }

    #[test]
    fn test_typecheck_function_body_isolated_from_outer_scope() {
        // 外 is declared in the script scope, not as a param of 関数.
        // The function body must NOT see it.
        let src = "整数 外 ＝ １；関数 計算（整数 Ａ）ー＞ 整数 ｛ 返す 外； ｝";
        let ast = parse(src);
        let err = TypeChecker::new().check(&ast).unwrap_err();
        assert!(matches!(err, TypeError::UndeclaredVariable(n, _) if n == "外"));
    }

    #[test]
    fn test_typecheck_if_then_and_else_have_separate_scopes() {
        let src = "もし 真 ならば ｛ 整数 Ａ ＝ １； ｝ 違えば ｛ 返す Ａ； ｝";
        let ast = parse(src);
        let err = TypeChecker::new().check(&ast).unwrap_err();
        assert!(matches!(err, TypeError::UndeclaredVariable(n, _) if n == "Ａ"));
    }

    #[test]
    fn test_typecheck_try_catch_error_var_is_string() {
        let src = "試す ｛ 印刷（１）； ｝ 失敗 失敗内容 ｛ 印刷（失敗内容）； ｝";
        let ast = parse(src);
        assert!(TypeChecker::new().check(&ast).is_ok());
    }

    #[test]
    fn test_typecheck_try_body_var_does_not_leak() {
        let src = "試す ｛ 整数 Ａ ＝ １； ｝ 失敗 失敗内容 ｛ 返す Ａ； ｝";
        let ast = parse(src);
        let err = TypeChecker::new().check(&ast).unwrap_err();
        assert!(matches!(err, TypeError::UndeclaredVariable(n, _) if n == "Ａ"));

        let src2 = "試す ｛ 整数 Ａ ＝ １； ｝ 失敗 失敗内容 ｛ 印刷（失敗内容）； ｝返す Ａ；";
        let ast2 = parse(src2);
        let err2 = TypeChecker::new().check(&ast2).unwrap_err();
        assert!(matches!(err2, TypeError::UndeclaredVariable(n, _) if n == "Ａ"));
    }

    #[test]
    fn test_typecheck_try_catch_error_var_not_visible_after_block() {
        let src =
            "試す ｛ 印刷（１）； ｝ 失敗 失敗内容 ｛ 印刷（失敗内容）； ｝印刷（失敗内容）；";
        let ast = parse(src);
        let err = TypeChecker::new().check(&ast).unwrap_err();
        assert!(matches!(err, TypeError::UndeclaredVariable(n, _) if n == "失敗内容"));
    }

    #[test]
    fn test_typecheck_try_body_type_error_still_rejected() {
        let src = "試す ｛ 整数 Ａ ＝ 「文字」； ｝ 失敗 失敗内容 ｛ ｝";
        let ast = parse(src);
        let err = TypeChecker::new().check(&ast).unwrap_err();
        assert!(matches!(err, TypeError::VarDeclMismatch { .. }));
    }

    #[test]
    fn test_typecheck_math_builtins_after_import() {
        let src = "取り込む 「数学」；整数 結果 ＝ 絶対値（ー５）；";
        let ast = parse(src);
        assert!(TypeChecker::new().check(&ast).is_ok());

        let src = "取り込む 「数学」；小数 結果 ＝ 平方根（９）；";
        let ast = parse(src);
        assert!(TypeChecker::new().check(&ast).is_ok());

        let src = "取り込む 「数学」；整数 結果 ＝ 乱数（１、１０）；";
        let ast = parse(src);
        assert!(TypeChecker::new().check(&ast).is_ok());

        let src = "取り込む 「数学」；整数 結果 ＝ 最大（１、２）；";
        let ast = parse(src);
        assert!(TypeChecker::new().check(&ast).is_ok());

        let src = "取り込む 「数学」；小数 結果 ＝ 最小（１．０、２．０）；";
        let ast = parse(src);
        assert!(TypeChecker::new().check(&ast).is_ok());
    }

    #[test]
    fn test_typecheck_string_builtins_after_import() {
        let src = "取り込む 「文字列」；文字列列 結果 ＝ 分割（「あ、い」、「、」）；";
        let ast = parse(src);
        assert!(TypeChecker::new().check(&ast).is_ok());

        let src = "取り込む 「文字列」；文字列 結果 ＝ 結合（【「あ」、「い」】、「、」）；";
        let ast = parse(src);
        assert!(TypeChecker::new().check(&ast).is_ok());

        let src = "取り込む 「文字列」；真偽 結果 ＝ 含む（「あいう」、「い」）；";
        let ast = parse(src);
        assert!(TypeChecker::new().check(&ast).is_ok());

        let src = "取り込む 「文字列」；文字列 結果 ＝ 置換（「あいう」、「い」、「え」）；";
        let ast = parse(src);
        assert!(TypeChecker::new().check(&ast).is_ok());
    }

    #[test]
    fn test_typecheck_stdlib_builtin_without_import_fails() {
        let src = "整数 結果 ＝ 絶対値（ー５）；";
        let ast = parse(src);
        let err = TypeChecker::new().check(&ast).unwrap_err();
        assert!(matches!(
            err,
            TypeError::ModuleNotImported { module, .. } if module == "数学"
        ));

        let src = "真偽 結果 ＝ 含む（「あ」、「い」）；";
        let ast = parse(src);
        let err = TypeChecker::new().check(&ast).unwrap_err();
        assert!(matches!(
            err,
            TypeError::ModuleNotImported { module, .. } if module == "文字列"
        ));
    }

    #[test]
    fn test_typecheck_abs_sqrt_polymorphic_mismatch() {
        let src = "取り込む 「数学」；整数 結果 ＝ 絶対値（「文字」）；";
        let ast = parse(src);
        let err = TypeChecker::new().check(&ast).unwrap_err();
        assert!(matches!(err, TypeError::ArgTypeMismatch { .. }));

        let src = "取り込む 「数学」；小数 結果 ＝ 平方根（「文字」）；";
        let ast = parse(src);
        let err = TypeChecker::new().check(&ast).unwrap_err();
        assert!(matches!(err, TypeError::ArgTypeMismatch { .. }));
    }

    #[test]
    fn test_typecheck_max_min_polymorphic_mismatch() {
        let src = "取り込む 「数学」；整数 結果 ＝ 最大（１、「あ」）；";
        let ast = parse(src);
        let err = TypeChecker::new().check(&ast).unwrap_err();
        assert!(matches!(err, TypeError::ArgTypeMismatch { .. }));

        let src = "取り込む 「数学」；文字列 結果 ＝ 最小（「あ」、「い」）；";
        let ast = parse(src);
        let err = TypeChecker::new().check(&ast).unwrap_err();
        assert!(matches!(err, TypeError::ArgTypeMismatch { .. }));
    }

    #[test]
    fn test_typecheck_arithmetic_on_bool_is_rejected() {
        // 真 ＋ 偽 has matching operand types but ＋ is undefined for 真偽.
        let ast = parse("真偽 結果 ＝ 真 ＋ 偽；");
        let err = TypeChecker::new().check(&ast).unwrap_err();
        assert!(matches!(err, TypeError::BinOpMismatch { .. }));
    }

    #[test]
    fn test_typecheck_subtraction_on_strings_is_rejected() {
        // ＋ concatenates strings, but ー/＊/／ are numbers-only.
        let ast = parse("文字列 結果 ＝ 「あ」 ー 「い」；");
        let err = TypeChecker::new().check(&ast).unwrap_err();
        assert!(matches!(err, TypeError::BinOpMismatch { .. }));
    }

    #[test]
    fn test_typecheck_ordering_on_strings_is_rejected() {
        // ＜/＞/≦/≧ are only defined for numbers.
        let ast = parse("真偽 結果 ＝ 「あ」 ＜ 「い」；");
        let err = TypeChecker::new().check(&ast).unwrap_err();
        assert!(matches!(err, TypeError::BinOpMismatch { .. }));
    }

    #[test]
    fn test_typecheck_equality_on_strings_is_allowed() {
        // ＝＝/≠ remain valid for any two values of the same type.
        let ast = parse("真偽 結果 ＝ 「あ」 ＝＝ 「い」；");
        assert!(TypeChecker::new().check(&ast).is_ok());
    }

    #[test]
    fn test_typecheck_string_concatenation_still_allowed() {
        let ast = parse("文字列 結果 ＝ 「あ」 ＋ 「い」；");
        assert!(TypeChecker::new().check(&ast).is_ok());
    }

    // ── 7a: modulo ───────────────────────────────────────────────────────

    #[test]
    fn test_typecheck_modulo_numeric_only() {
        let ast = parse("整数 結果 ＝ １０ ％ ３；");
        assert!(TypeChecker::new().check(&ast).is_ok());

        let ast = parse("文字列 結果 ＝ 「あ」 ％ 「い」；");
        let err = TypeChecker::new().check(&ast).unwrap_err();
        assert!(matches!(err, TypeError::BinOpMismatch { .. }));
    }

    // ── 7b: array builtins ──────────────────────────────────────────────

    #[test]
    fn test_typecheck_array_builtins_require_import() {
        let ast = parse("整数列 数字 ＝ 【１】；整数 結果 ＝ 要素数（数字）；");
        let err = TypeChecker::new().check(&ast).unwrap_err();
        assert!(matches!(
            err,
            TypeError::ModuleNotImported { module, .. } if module == "配列"
        ));
    }

    #[test]
    fn test_typecheck_array_len_happy_and_mismatch() {
        let ast = parse("取り込む 「配列」；整数列 数字 ＝ 【１】；整数 結果 ＝ 要素数（数字）；");
        assert!(TypeChecker::new().check(&ast).is_ok());

        let ast = parse("取り込む 「配列」；整数 結果 ＝ 要素数（５）；");
        let err = TypeChecker::new().check(&ast).unwrap_err();
        assert!(matches!(err, TypeError::ArgTypeMismatch { .. }));
    }

    #[test]
    fn test_typecheck_push_happy_and_mismatch() {
        let ast = parse("取り込む 「配列」；整数列 数字 ＝ 【１】；追加（数字、２）；");
        assert!(TypeChecker::new().check(&ast).is_ok());

        let ast = parse("取り込む 「配列」；整数列 数字 ＝ 【１】；追加（数字、「あ」）；");
        let err = TypeChecker::new().check(&ast).unwrap_err();
        assert!(matches!(err, TypeError::ArgTypeMismatch { .. }));
    }

    #[test]
    fn test_typecheck_pop_happy_and_mismatch() {
        let ast =
            parse("取り込む 「配列」；整数列 数字 ＝ 【１】；整数 結果 ＝ 取り出す（数字）；");
        assert!(TypeChecker::new().check(&ast).is_ok());

        let ast = parse("取り込む 「配列」；整数 結果 ＝ 取り出す（５）；");
        let err = TypeChecker::new().check(&ast).unwrap_err();
        assert!(matches!(err, TypeError::ArgTypeMismatch { .. }));
    }

    #[test]
    fn test_typecheck_contains_array_and_index_of_happy_and_mismatch() {
        let ast =
            parse("取り込む 「配列」；整数列 数字 ＝ 【１】；真偽 結果 ＝ 含む配列（数字、１）；");
        assert!(TypeChecker::new().check(&ast).is_ok());

        let ast =
            parse("取り込む 「配列」；整数列 数字 ＝ 【１】；整数 結果 ＝ 位置（数字、１）；");
        assert!(TypeChecker::new().check(&ast).is_ok());

        let ast = parse(
            "取り込む 「配列」；整数列 数字 ＝ 【１】；真偽 結果 ＝ 含む配列（数字、「あ」）；",
        );
        let err = TypeChecker::new().check(&ast).unwrap_err();
        assert!(matches!(err, TypeError::ArgTypeMismatch { .. }));
    }

    #[test]
    fn test_typecheck_reverse_happy_and_mismatch() {
        let ast = parse("取り込む 「配列」；整数列 数字 ＝ 【１】；整数列 結果 ＝ 逆順（数字）；");
        assert!(TypeChecker::new().check(&ast).is_ok());

        let ast = parse("取り込む 「配列」；整数 結果 ＝ 逆順（５）；");
        let err = TypeChecker::new().check(&ast).unwrap_err();
        assert!(matches!(err, TypeError::ArgTypeMismatch { .. }));
    }

    #[test]
    fn test_typecheck_sort_happy_and_rejects_bool_array() {
        let ast = parse("取り込む 「配列」；整数列 数字 ＝ 【１】；整列（数字）；");
        assert!(TypeChecker::new().check(&ast).is_ok());

        let ast = parse("取り込む 「配列」；真偽列 旗 ＝ 【真】；整列（旗）；");
        let err = TypeChecker::new().check(&ast).unwrap_err();
        assert!(matches!(err, TypeError::ArgTypeMismatch { .. }));
    }

    #[test]
    fn test_typecheck_slice_happy_and_mismatch() {
        let ast = parse(
            "取り込む 「配列」；整数列 数字 ＝ 【１、２】；整数列 結果 ＝ 部分列（数字、０、１）；",
        );
        assert!(TypeChecker::new().check(&ast).is_ok());

        let ast = parse(
            "取り込む 「配列」；整数列 数字 ＝ 【１】；整数列 結果 ＝ 部分列（数字、「あ」、１）；",
        );
        let err = TypeChecker::new().check(&ast).unwrap_err();
        assert!(matches!(err, TypeError::ArgTypeMismatch { .. }));
    }

    #[test]
    fn test_typecheck_new_array_expr() {
        let ast = parse("整数列 数字 ＝ 新配列＜整数＞；");
        assert!(TypeChecker::new().check(&ast).is_ok());

        let ast = parse("小数列 数字 ＝ 新配列＜小数＞；");
        assert!(TypeChecker::new().check(&ast).is_ok());

        let ast = parse("文字列列 文字 ＝ 新配列＜文字列＞；");
        assert!(TypeChecker::new().check(&ast).is_ok());

        let ast = parse("真偽列 旗 ＝ 新配列＜真偽＞；");
        assert!(TypeChecker::new().check(&ast).is_ok());
    }

    // ── 7c: more math builtins ─────────────────────────────────────────

    #[test]
    fn test_typecheck_pow_happy_and_mismatch() {
        let ast = parse("取り込む 「数学」；整数 結果 ＝ 累乗（２、３）；");
        assert!(TypeChecker::new().check(&ast).is_ok());

        let ast = parse("取り込む 「数学」；整数 結果 ＝ 累乗（２、「あ」）；");
        let err = TypeChecker::new().check(&ast).unwrap_err();
        assert!(matches!(err, TypeError::ArgTypeMismatch { .. }));
    }

    #[test]
    fn test_typecheck_floor_ceil_round_happy_and_reject_int() {
        let ast = parse("取り込む 「数学」；整数 結果 ＝ 切り捨て（３．５）；");
        assert!(TypeChecker::new().check(&ast).is_ok());

        let ast = parse("取り込む 「数学」；整数 結果 ＝ 切り上げ（３．５）；");
        assert!(TypeChecker::new().check(&ast).is_ok());

        let ast = parse("取り込む 「数学」；整数 結果 ＝ 四捨五入（３．５）；");
        assert!(TypeChecker::new().check(&ast).is_ok());

        let ast = parse("取り込む 「数学」；整数 結果 ＝ 切り捨て（３）；");
        let err = TypeChecker::new().check(&ast).unwrap_err();
        assert!(matches!(err, TypeError::ArgTypeMismatch { .. }));
    }

    #[test]
    fn test_typecheck_remainder_function_form() {
        let ast = parse("取り込む 「数学」；整数 結果 ＝ 余り（１０、３）；");
        assert!(TypeChecker::new().check(&ast).is_ok());

        let ast = parse("取り込む 「数学」；整数 結果 ＝ 余り（１０、「あ」）；");
        let err = TypeChecker::new().check(&ast).unwrap_err();
        assert!(matches!(err, TypeError::ArgTypeMismatch { .. }));
    }

    #[test]
    fn test_typecheck_math_builtins_without_import_fails() {
        let ast = parse("整数 結果 ＝ 累乗（２、３）；");
        let err = TypeChecker::new().check(&ast).unwrap_err();
        assert!(matches!(
            err,
            TypeError::ModuleNotImported { module, .. } if module == "数学"
        ));
    }

    // ── 8a: exhaustive-return analysis ───────────────────────────────────

    #[test]
    fn test_typecheck_if_else_both_return_is_ok() {
        let src = "関数 計算（整数 Ａ）ー＞ 整数 ｛ もし Ａ ＞ ０ ならば ｛ 返す １； ｝ 違えば ｛ 返す ０； ｝ ｝";
        let ast = parse(src);
        assert!(TypeChecker::new().check(&ast).is_ok());
    }

    #[test]
    fn test_typecheck_if_without_else_is_missing_return() {
        let src = "関数 計算（整数 Ａ）ー＞ 整数 ｛ もし Ａ ＞ ０ ならば ｛ 返す １； ｝ ｝";
        let ast = parse(src);
        let err = TypeChecker::new().check(&ast).unwrap_err();
        assert!(matches!(err, TypeError::MissingReturn { name, .. } if name == "計算"));
    }

    #[test]
    fn test_typecheck_trailing_return_after_if_else_is_ok_even_if_branches_dont_return() {
        let src = "関数 計算（整数 Ａ）ー＞ 整数 ｛ もし Ａ ＞ ０ ならば ｛ 印刷（１）； ｝ 違えば ｛ 印刷（０）； ｝ 返す Ａ； ｝";
        let ast = parse(src);
        assert!(TypeChecker::new().check(&ast).is_ok());
    }

    #[test]
    fn test_typecheck_void_function_exempt_from_missing_return() {
        let src = "関数 表示（整数 Ａ）ー＞ 無 ｛ 印刷（Ａ）； ｝";
        let ast = parse(src);
        assert!(TypeChecker::new().check(&ast).is_ok());
    }

    #[test]
    fn test_typecheck_while_loop_with_return_is_still_missing_return() {
        let src = "関数 計算（整数 Ａ）ー＞ 整数 ｛ 間 Ａ ＞ ０ ならば ｛ 返す Ａ； ｝ ｝";
        let ast = parse(src);
        let err = TypeChecker::new().check(&ast).unwrap_err();
        assert!(matches!(err, TypeError::MissingReturn { name, .. } if name == "計算"));
    }

    #[test]
    fn test_typecheck_try_catch_both_branches_return_is_ok() {
        let src = "関数 計算（）ー＞ 整数 ｛ 試す ｛ 返す １； ｝ 失敗 失敗内容 ｛ 返す ０； ｝ ｝";
        let ast = parse(src);
        assert!(TypeChecker::new().check(&ast).is_ok());
    }

    #[test]
    fn test_typecheck_try_catch_only_one_branch_returns_is_missing_return() {
        let src = "関数 計算（）ー＞ 整数 ｛ 試す ｛ 返す １； ｝ 失敗 失敗内容 ｛ 印刷（失敗内容）； ｝ ｝";
        let ast = parse(src);
        let err = TypeChecker::new().check(&ast).unwrap_err();
        assert!(matches!(err, TypeError::MissingReturn { name, .. } if name == "計算"));
    }

    // ── 8b: break / continue ─────────────────────────────────────────────

    #[test]
    fn test_typecheck_break_continue_inside_loops_ok() {
        assert!(
            TypeChecker::new()
                .check(&parse("間 真 ならば ｛ 抜ける； ｝"))
                .is_ok()
        );
        assert!(
            TypeChecker::new()
                .check(&parse("間 真 ならば ｛ 続ける； ｝"))
                .is_ok()
        );
        assert!(
            TypeChecker::new()
                .check(&parse("繰り返す ｉ ＝ ０ から ５ ならば ｛ 抜ける； ｝"))
                .is_ok()
        );
        assert!(
            TypeChecker::new()
                .check(&parse(
                    "整数列 数字 ＝ 【１】；各 要素 ： 数字 ならば ｛ 続ける； ｝"
                ))
                .is_ok()
        );
    }

    #[test]
    fn test_typecheck_break_continue_outside_loop_is_error() {
        let err = TypeChecker::new().check(&parse("抜ける；")).unwrap_err();
        assert!(matches!(err, TypeError::ControlFlowOutsideLoop { .. }));

        let err = TypeChecker::new().check(&parse("続ける；")).unwrap_err();
        assert!(matches!(err, TypeError::ControlFlowOutsideLoop { .. }));
    }

    #[test]
    fn test_typecheck_break_inside_if_inside_loop_is_ok() {
        let src = "間 真 ならば ｛ もし 真 ならば ｛ 抜ける； ｝ ｝";
        assert!(TypeChecker::new().check(&parse(src)).is_ok());
    }

    #[test]
    fn test_typecheck_break_inside_function_body_not_itself_in_loop_is_error() {
        // The function is CALLED from inside a loop, but its own body has no
        // enclosing loop, proving loop_depth resets per function.
        let src = "関数 内部（）ー＞ 無 ｛ 抜ける； ｝間 真 ならば ｛ 内部（）； 抜ける； ｝";
        let err = TypeChecker::new().check(&parse(src)).unwrap_err();
        assert!(matches!(err, TypeError::ControlFlowOutsideLoop { .. }));
    }

    // ── 8c: bare return / void semantics ──────────────────────────────────

    #[test]
    fn test_typecheck_bare_return_in_void_function_is_ok() {
        let src = "関数 表示（）ー＞ 無 ｛ 返す； ｝";
        assert!(TypeChecker::new().check(&parse(src)).is_ok());
    }

    #[test]
    fn test_typecheck_bare_return_in_non_void_function_is_error() {
        let src = "関数 計算（）ー＞ 整数 ｛ 返す； ｝";
        let err = TypeChecker::new().check(&parse(src)).unwrap_err();
        assert!(matches!(
            err,
            TypeError::ReturnTypeMismatch {
                expected: HikariType::Int,
                got: HikariType::Void,
                ..
            }
        ));
    }

    #[test]
    fn test_typecheck_bare_return_at_top_level_is_ok() {
        assert!(TypeChecker::new().check(&parse("返す；")).is_ok());
    }

    // ── 9a: records ───────────────────────────────────────────────────────

    #[test]
    fn test_typecheck_record_construction_and_field_read_valid() {
        let src = "型 点 ｛ 整数 ｘ； 整数 ｙ； ｝点 ｐ ＝ 点 ｛ ｘ：１、ｙ：２ ｝；整数 結果 ＝ ｐ：：ｘ；";
        let ast = parse(src);
        assert!(TypeChecker::new().check(&ast).is_ok());
    }

    #[test]
    fn test_typecheck_field_read_unknown_field() {
        let src = "型 点 ｛ 整数 ｘ； ｝点 ｐ ＝ 点 ｛ ｘ：１ ｝；整数 結果 ＝ ｐ：：ｚ；";
        let ast = parse(src);
        let err = TypeChecker::new().check(&ast).unwrap_err();
        assert!(matches!(err, TypeError::UnknownField { field, .. } if field == "ｚ"));
    }

    #[test]
    fn test_typecheck_field_read_on_non_record_value() {
        let ast = parse("整数 値 ＝ ５；整数 結果 ＝ 値：：ｘ；");
        let err = TypeChecker::new().check(&ast).unwrap_err();
        assert!(matches!(
            err,
            TypeError::NotARecord {
                got: HikariType::Int,
                ..
            }
        ));
    }

    #[test]
    fn test_typecheck_record_construction_missing_field() {
        let src = "型 点 ｛ 整数 ｘ； 整数 ｙ； ｝点 ｐ ＝ 点 ｛ ｘ：１ ｝；";
        let ast = parse(src);
        let err = TypeChecker::new().check(&ast).unwrap_err();
        assert!(matches!(err, TypeError::MissingField { field, .. } if field == "ｙ"));
    }

    #[test]
    fn test_typecheck_record_construction_extra_field() {
        let src = "型 点 ｛ 整数 ｘ； ｝点 ｐ ＝ 点 ｛ ｘ：１、ｚ：２ ｝；";
        let ast = parse(src);
        let err = TypeChecker::new().check(&ast).unwrap_err();
        assert!(matches!(err, TypeError::UnknownField { field, .. } if field == "ｚ"));
    }

    #[test]
    fn test_typecheck_record_construction_field_type_mismatch() {
        let src = "型 点 ｛ 整数 ｘ； ｝点 ｐ ＝ 点 ｛ ｘ：「あ」 ｝；";
        let ast = parse(src);
        let err = TypeChecker::new().check(&ast).unwrap_err();
        match err {
            TypeError::FieldTypeMismatch { expected, got, .. } => {
                assert_eq!(*expected, HikariType::Int);
                assert_eq!(*got, HikariType::String);
            }
            other => panic!("expected FieldTypeMismatch, got {:?}", other),
        }
    }

    #[test]
    fn test_typecheck_field_assign_happy_path() {
        let src = "型 点 ｛ 整数 ｘ； ｝点 ｐ ＝ 点 ｛ ｘ：１ ｝；ｐ：：ｘ ＝ ９；";
        let ast = parse(src);
        assert!(TypeChecker::new().check(&ast).is_ok());
    }

    #[test]
    fn test_typecheck_field_assign_wrong_value_type() {
        let src = "型 点 ｛ 整数 ｘ； ｝点 ｐ ＝ 点 ｛ ｘ：１ ｝；ｐ：：ｘ ＝ 「あ」；";
        let ast = parse(src);
        let err = TypeChecker::new().check(&ast).unwrap_err();
        match err {
            TypeError::FieldTypeMismatch { expected, got, .. } => {
                assert_eq!(*expected, HikariType::Int);
                assert_eq!(*got, HikariType::String);
            }
            other => panic!("expected FieldTypeMismatch, got {:?}", other),
        }
    }

    #[test]
    fn test_typecheck_undeclared_type_in_construction() {
        let src = "点 ｐ ＝ 点 ｛ ｘ：１ ｝；";
        let ast = parse(src);
        let err = TypeChecker::new().check(&ast).unwrap_err();
        assert!(matches!(err, TypeError::UndeclaredType(n, _) if n == "点"));
    }

    #[test]
    fn test_typecheck_undeclared_type_in_var_decl() {
        let src = "型 点 ｛ 整数 ｘ； ｝存在しない ｐ ＝ 点 ｛ ｘ：１ ｝；";
        let ast = parse(src);
        let err = TypeChecker::new().check(&ast).unwrap_err();
        assert!(matches!(err, TypeError::UndeclaredType(n, _) if n == "存在しない"));
    }

    // ── 9b: enums and pattern matching ──────────────────────────────────

    #[test]
    fn test_typecheck_variant_construction_happy_path() {
        let src = "列挙 結果 ｛ 成功（整数）、 異常（文字列） ｝結果 値 ＝ 成功（１）；";
        let ast = parse(src);
        assert!(TypeChecker::new().check(&ast).is_ok());
    }

    #[test]
    fn test_typecheck_variant_construction_zero_payload() {
        let src = "列挙 信号 ｛ 赤、 黄、 青 ｝信号 値 ＝ 赤（）；";
        let ast = parse(src);
        assert!(TypeChecker::new().check(&ast).is_ok());
    }

    #[test]
    fn test_typecheck_variant_construction_wrong_arg_count() {
        let src = "列挙 結果 ｛ 成功（整数） ｝結果 値 ＝ 成功（）；";
        let ast = parse(src);
        let err = TypeChecker::new().check(&ast).unwrap_err();
        assert!(matches!(
            err,
            TypeError::ArgCountMismatch {
                expected: 1,
                got: 0,
                ..
            }
        ));
    }

    #[test]
    fn test_typecheck_variant_construction_wrong_arg_type() {
        let src = "列挙 結果 ｛ 成功（整数） ｝結果 値 ＝ 成功（「あ」）；";
        let ast = parse(src);
        let err = TypeChecker::new().check(&ast).unwrap_err();
        assert!(matches!(
            err,
            TypeError::ArgTypeMismatch {
                param: HikariType::Int,
                got: HikariType::String,
                ..
            }
        ));
    }

    #[test]
    fn test_typecheck_duplicate_enum_variant_across_enums() {
        let src = "列挙 結果 ｛ 成功 ｝列挙 状態 ｛ 成功 ｝";
        let ast = parse(src);
        let err = TypeChecker::new().check(&ast).unwrap_err();
        assert!(
            matches!(err, TypeError::DuplicateEnumVariant { variant, .. } if variant == "成功")
        );
    }

    #[test]
    fn test_typecheck_match_exhaustive_is_ok() {
        let src = "列挙 信号 ｛ 赤、 青 ｝信号 値 ＝ 赤（）；照合 値 ｛ 赤（） ならば ｛ 印刷（１）； ｝ 青（） ならば ｛ 印刷（２）； ｝ ｝";
        let ast = parse(src);
        assert!(TypeChecker::new().check(&ast).is_ok());
    }

    #[test]
    fn test_typecheck_match_non_exhaustive_lists_missing_variant() {
        let src = "列挙 信号 ｛ 赤、 黄、 青 ｝信号 値 ＝ 赤（）；照合 値 ｛ 赤（） ならば ｛ 印刷（１）； ｝ ｝";
        let ast = parse(src);
        let err = TypeChecker::new().check(&ast).unwrap_err();
        match &err {
            TypeError::NonExhaustiveMatch(info) => {
                assert_eq!(info.missing, vec!["黄".to_string(), "青".to_string()]);
            }
            other => panic!("expected NonExhaustiveMatch, got {:?}", other),
        }
        assert!(err.to_string().contains("黄"));
        assert!(err.to_string().contains("青"));
    }

    #[test]
    fn test_typecheck_match_duplicate_arm() {
        let src = "列挙 信号 ｛ 赤、 青 ｝信号 値 ＝ 赤（）；照合 値 ｛ 赤（） ならば ｛ ｝ 赤（） ならば ｛ ｝ 青（） ならば ｛ ｝ ｝";
        let ast = parse(src);
        let err = TypeChecker::new().check(&ast).unwrap_err();
        assert!(matches!(err, TypeError::DuplicateMatchArm { variant, .. } if variant == "赤"));
    }

    #[test]
    fn test_typecheck_match_arm_from_different_enum_is_undeclared_variant() {
        let src = "列挙 信号 ｛ 赤、 青 ｝列挙 状態 ｛ 開始 ｝信号 値 ＝ 赤（）；照合 値 ｛ 赤（） ならば ｛ ｝ 開始（） ならば ｛ ｝ ｝";
        let ast = parse(src);
        let err = TypeChecker::new().check(&ast).unwrap_err();
        assert!(matches!(
            err,
            TypeError::UndeclaredEnumVariant { enum_name, variant, .. }
            if enum_name == "信号" && variant == "開始"
        ));
    }

    #[test]
    fn test_typecheck_match_arm_wrong_binder_count() {
        let src = "列挙 結果 ｛ 成功（整数） ｝結果 値 ＝ 成功（１）；照合 値 ｛ 成功（ａ、ｂ） ならば ｛ ｝ ｝";
        let ast = parse(src);
        let err = TypeChecker::new().check(&ast).unwrap_err();
        assert!(matches!(
            err,
            TypeError::ArgCountMismatch {
                expected: 1,
                got: 2,
                ..
            }
        ));
    }

    #[test]
    fn test_typecheck_match_on_non_enum_value() {
        let src = "整数 値 ＝ ５；照合 値 ｛ ｝";
        let ast = parse(src);
        let err = TypeChecker::new().check(&ast).unwrap_err();
        assert!(matches!(
            err,
            TypeError::NotAnEnum {
                got: HikariType::Int,
                ..
            }
        ));
    }

    #[test]
    fn test_typecheck_match_arm_binder_scoped_to_its_own_arm() {
        let src = "列挙 結果 ｛ 成功（整数）、 異常（文字列） ｝結果 値 ＝ 成功（１）；照合 値 ｛ 成功（ｎ） ならば ｛ 印刷（ｎ）； ｝ 異常（ｅ） ならば ｛ 印刷（ｎ）； ｝ ｝";
        let ast = parse(src);
        let err = TypeChecker::new().check(&ast).unwrap_err();
        assert!(matches!(err, TypeError::UndeclaredVariable(n, _) if n == "ｎ"));
    }

    #[test]
    fn test_typecheck_match_binder_not_visible_after_match() {
        let src = "列挙 結果 ｛ 成功（整数） ｝結果 値 ＝ 成功（１）；照合 値 ｛ 成功（ｎ） ならば ｛ 印刷（ｎ）； ｝ ｝印刷（ｎ）；";
        let ast = parse(src);
        let err = TypeChecker::new().check(&ast).unwrap_err();
        assert!(matches!(err, TypeError::UndeclaredVariable(n, _) if n == "ｎ"));
    }

    #[test]
    fn test_typecheck_void_call_result_cannot_be_used_as_value() {
        let src = "関数 表示（整数 Ａ）ー＞ 無 ｛ 印刷（Ａ）； ｝整数 結果 ＝ 表示（５）；";
        let err = TypeChecker::new().check(&parse(src)).unwrap_err();
        assert!(matches!(
            err,
            TypeError::VarDeclMismatch {
                got: HikariType::Void,
                ..
            }
        ));
    }
}
