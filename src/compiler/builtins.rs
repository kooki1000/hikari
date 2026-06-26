// ── Built-in functions ────────────────────────────────────────────────────────

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum BuiltinFn {
    Len,            // 文字数
    Input,          // 入力
    ParseInt,       // 整数化
    ParseFloat,     // 小数化
    ToStr,          // 文字列化
    Abs,            // 絶対値
    Sqrt,           // 平方根
    Random,         // 乱数
    Max,            // 最大
    Min,            // 最小
    Split,          // 分割
    Join,           // 結合
    Contains,       // 含む
    Replace,        // 置換
    Pow,            // 累乗
    Floor,          // 切り捨て
    Ceil,           // 切り上げ
    Round,          // 四捨五入
    Rem,            // 余り
    ArrayLen,       // 要素数
    Push,           // 追加
    Pop,            // 取り出す
    ArrayContains,  // 含む配列
    IndexOf,        // 位置
    Reverse,        // 逆順
    Sort,           // 整列
    Slice,          // 部分列
    MapKeys,        // 鍵一覧
    MapValues,      // 値一覧
    MapDelete,      // 削除
    ReadFile,       // ファイル読む
    WriteFile,      // ファイル書く
    PrintNoNewline, // 印字 (print without trailing newline)
    ProgramArgs,    // 引数 (CLI args passed to the program)
    EnvVar,         // 環境変数 (read an environment variable)
    // Higher-order functions (special: they take a fn value)
    MapArray,    // マップ
    FilterArray, // 絞り込み
    FoldArray,   // 畳み込み
    // Phase 15: safe access returning 省略可
    SafeGet, // 取得 (safe map/array lookup → 省略可)
    SafePos, // 位置可 (safe indexOf → 省略可＜整数＞)
    // Phase 17a: richer strings
    ToUpperCase, // 大文字
    ToLowerCase, // 小文字
    Trim,        // 整形
    StartsWith,  // 先頭一致
    EndsWith,    // 末尾一致
    Substring,   // 部分文字列
    StrFind,     // 文字列位置 (→ 省略可＜整数＞)
    RepeatStr,   // 繰り返し文字列
    // Phase 17b: more numerics
    Sign,     // 符号
    Clamp,    // 挟む
    Sum,      // 総和 (整数列): empty → 整数 0
    SumFloat, // 総和 (小数列): empty → 小数 0.0 — chosen by codegen from the
    // type checker's element-type info so an empty 小数列 sums to 0.0, not 0.
    Average,  // 平均
    ArrayMax, // 最大値
    ArrayMin, // 最小値
    Sin,      // 正弦
    Cos,      // 余弦
    Tan,      // 正接
    Ln,       // 対数
    Exp,      // 指数
    // Phase 17c: more array ops
    Concat,     // 連結
    Flatten,    // 平坦化
    AnyArray,   // どれか (HOF)
    AllArray,   // すべて (HOF)
    CountArray, // 数える (HOF)
    // Phase 17d: more map ops
    MapMerge,        // 併合
    MapSize,         // 数
    MapGetOrDefault, // 取得既定
    // Phase 17e: time
    NowMillis, // 現在時刻
    Elapsed,   // 経過
    Sleep,     // 眠る
}

pub fn builtin_name(name: &str) -> Option<BuiltinFn> {
    match name {
        "文字数" => Some(BuiltinFn::Len),
        "入力" => Some(BuiltinFn::Input),
        "整数化" => Some(BuiltinFn::ParseInt),
        "小数化" => Some(BuiltinFn::ParseFloat),
        "文字列化" => Some(BuiltinFn::ToStr),
        "絶対値" => Some(BuiltinFn::Abs),
        "平方根" => Some(BuiltinFn::Sqrt),
        "乱数" => Some(BuiltinFn::Random),
        "最大" => Some(BuiltinFn::Max),
        "最小" => Some(BuiltinFn::Min),
        "分割" => Some(BuiltinFn::Split),
        "結合" => Some(BuiltinFn::Join),
        "含む" => Some(BuiltinFn::Contains),
        "置換" => Some(BuiltinFn::Replace),
        "累乗" => Some(BuiltinFn::Pow),
        "切り捨て" => Some(BuiltinFn::Floor),
        "切り上げ" => Some(BuiltinFn::Ceil),
        "四捨五入" => Some(BuiltinFn::Round),
        "余り" => Some(BuiltinFn::Rem),
        "要素数" => Some(BuiltinFn::ArrayLen),
        "追加" => Some(BuiltinFn::Push),
        "取り出す" => Some(BuiltinFn::Pop),
        "含む配列" => Some(BuiltinFn::ArrayContains),
        "位置" => Some(BuiltinFn::IndexOf),
        "逆順" => Some(BuiltinFn::Reverse),
        "整列" => Some(BuiltinFn::Sort),
        "部分列" => Some(BuiltinFn::Slice),
        "鍵一覧" => Some(BuiltinFn::MapKeys),
        "値一覧" => Some(BuiltinFn::MapValues),
        "削除" => Some(BuiltinFn::MapDelete),
        "ファイル読む" => Some(BuiltinFn::ReadFile),
        "ファイル書く" => Some(BuiltinFn::WriteFile),
        "印字" => Some(BuiltinFn::PrintNoNewline),
        "引数" => Some(BuiltinFn::ProgramArgs),
        "環境変数" => Some(BuiltinFn::EnvVar),
        "マップ" => Some(BuiltinFn::MapArray),
        "絞り込み" => Some(BuiltinFn::FilterArray),
        "畳み込み" => Some(BuiltinFn::FoldArray),
        "取得" => Some(BuiltinFn::SafeGet),
        "位置可" => Some(BuiltinFn::SafePos),
        // 17a
        "大文字" => Some(BuiltinFn::ToUpperCase),
        "小文字" => Some(BuiltinFn::ToLowerCase),
        "整形" => Some(BuiltinFn::Trim),
        "先頭一致" => Some(BuiltinFn::StartsWith),
        "末尾一致" => Some(BuiltinFn::EndsWith),
        "部分文字列" => Some(BuiltinFn::Substring),
        "文字列位置" => Some(BuiltinFn::StrFind),
        "繰り返し文字列" => Some(BuiltinFn::RepeatStr),
        // 17b
        "符号" => Some(BuiltinFn::Sign),
        "挟む" => Some(BuiltinFn::Clamp),
        "総和" => Some(BuiltinFn::Sum),
        "平均" => Some(BuiltinFn::Average),
        "最大値" => Some(BuiltinFn::ArrayMax),
        "最小値" => Some(BuiltinFn::ArrayMin),
        "正弦" => Some(BuiltinFn::Sin),
        "余弦" => Some(BuiltinFn::Cos),
        "正接" => Some(BuiltinFn::Tan),
        "対数" => Some(BuiltinFn::Ln),
        "指数" => Some(BuiltinFn::Exp),
        // 17c
        "連結" => Some(BuiltinFn::Concat),
        "平坦化" => Some(BuiltinFn::Flatten),
        "どれか" => Some(BuiltinFn::AnyArray),
        "すべて" => Some(BuiltinFn::AllArray),
        "数える" => Some(BuiltinFn::CountArray),
        // 17d
        "併合" => Some(BuiltinFn::MapMerge),
        "数" => Some(BuiltinFn::MapSize),
        "取得既定" => Some(BuiltinFn::MapGetOrDefault),
        // 17e
        "現在時刻" => Some(BuiltinFn::NowMillis),
        "経過" => Some(BuiltinFn::Elapsed),
        "眠る" => Some(BuiltinFn::Sleep),
        _ => None,
    }
}
