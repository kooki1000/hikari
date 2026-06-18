use crate::modules::{MOD_ARRAY, MOD_ENV, MOD_FUNC, MOD_IO, MOD_MAP, MOD_MATH, MOD_STRING};
use crate::parser::{HikariType, Stmt};

// ── Symbol tables ─────────────────────────────────────────────────────────────

#[derive(Clone)]
pub(super) struct FnSig {
    pub(super) params: Vec<HikariType>,
    pub(super) return_ty: HikariType,
}

pub(super) fn builtin_sig(name: &str) -> Option<FnSig> {
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
        "ファイル読む" => Some(FnSig {
            params: vec![HikariType::String],
            return_ty: HikariType::String,
        }),
        "ファイル書く" => Some(FnSig {
            params: vec![HikariType::String, HikariType::String],
            return_ty: HikariType::Void,
        }),
        "引数" => Some(FnSig {
            params: vec![],
            return_ty: HikariType::Array(Box::new(HikariType::String)),
        }),
        "環境変数" => Some(FnSig {
            params: vec![HikariType::String],
            return_ty: HikariType::String,
        }),
        _ => None,
    }
}

// Maps gated stdlib builtins to the module that must be 取り込む'd before
// they can be called. Phase-2 builtins are absent here, meaning ungated.
pub(super) fn builtin_module(name: &str) -> Option<&'static str> {
    match name {
        "絶対値" | "平方根" | "乱数" | "最大" | "最小" | "累乗" | "切り捨て" | "切り上げ"
        | "四捨五入" | "余り" => Some(MOD_MATH),
        "分割" | "結合" | "置換" => Some(MOD_STRING),
        "要素数" | "追加" | "取り出す" | "含む配列" | "位置" | "逆順" | "整列" | "部分列" => {
            Some(MOD_ARRAY)
        }
        "鍵一覧" | "値一覧" | "削除" => Some(MOD_MAP),
        "マップ" | "絞り込み" | "畳み込み" => Some(MOD_FUNC),
        "ファイル読む" | "ファイル書く" | "印字" => Some(MOD_IO),
        "引数" | "環境変数" => Some(MOD_ENV),
        _ => None,
    }
}

// Conservative exhaustive-return check: only the LAST statement of a block
// matters for reachability (no dead-code analysis here), and loops never
// count even if their body always returns, since a loop might run zero times.
pub(super) fn always_returns(stmts: &[Stmt]) -> bool {
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
