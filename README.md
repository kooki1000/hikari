# Hikari (光)

A statically-typed, stack-based bytecode language built in Rust, with Japanese keywords and full-width (ZenKaku) UTF-8 syntax.

---

## Language Overview

Hikari's syntax uses Japanese reserved words and full-width characters for all operators and punctuation. There are no ASCII symbols in valid Hikari source code.

### Primitive Types

| Hikari | Meaning |
|--------|---------|
| `整数` | Integer (`i64`) |
| `小数` | Float (`f64`) |
| `文字列` | String |
| `真偽` | Boolean |
| `無` | Void |

### Array Types

| Hikari | Meaning |
|--------|---------|
| `整数列` | Array of `整数` |
| `小数列` | Array of `小数` |
| `文字列列` | Array of `文字列` |
| `真偽列` | Array of `真偽` |

### Variable Declaration & Reassignment

```
整数 年齢 ＝ ２０；
年齢 ＝ ２１；  （reassignment — must already be declared, type must match）
```

### Comments

```
＃ this is a comment, runs to end of line
整数 年齢 ＝ ２０；  ＃ trailing comments are fine too
```

### Arithmetic (with operator precedence)

```
整数 結果 ＝ ２ ＋ ３ ＊ ４；  （＊ binds tighter than ＋）
整数 負数 ＝ ー５；             （unary minus）
文字列 結果 ＝ 「あ」 ＋ 「い」；（string concatenation via ＋）
整数 余り ＝ １０ ％ ３；       （modulo — same precedence tier as ＊／／）
```

### Comparison Operators

| Symbol | Meaning |
|--------|---------|
| `＝＝` | Equal |
| `≠` | Not equal |
| `＜` | Less than |
| `＞` | Greater than |
| `≦` | Less than or equal |
| `≧` | Greater than or equal |

### Logical Operators

| Hikari | Meaning |
|--------|---------|
| `かつ` | AND (short-circuiting) |
| `または` | OR (short-circuiting) |
| `否定` | NOT |

```
もし 点数 ≧ ６０ かつ 否定 欠席 ならば ｛
    印刷（「合格」）；
｝
```

### Print

```
印刷（結果）；
```

### If / Else

```
もし 点数 ＞ ７０ ならば ｛
    印刷（点数）；
｝ 違えば ｛
    印刷（０）；
｝
```

### While Loop

```
整数 カウンタ ＝ ０；
間 カウンタ ＜ ３ ならば ｛
    印刷（カウンタ）；
    カウンタ ＝ カウンタ ＋ １；
｝
```

### Break and Continue

`抜ける` (break) exits the nearest enclosing loop immediately; `続ける` (continue) skips to the next iteration. Both are only valid inside a `間`/`繰り返す`/`各` body:

```
整数 合計 ＝ ０；
整数 ｉ ＝ １；
間 ｉ ≦ １０ ならば ｛
    もし ｉ ％ ２ ＝＝ ０ ならば ｛
        ｉ ＝ ｉ ＋ １；
        続ける；
    ｝
    合計 ＝ 合計 ＋ ｉ；
    ｉ ＝ ｉ ＋ １；
｝
（合計 is now 1+3+5+7+9 = 25）
```

### Counting For-Loop

```
繰り返す ｉ ＝ ０ から ５ ならば ｛
    印刷（ｉ）；
｝
```

### For-Each Loop

```
整数列 数字 ＝ 【１、２、３】；
各 値 ： 数字 ならば ｛
    印刷（値）；
｝
```

### Arrays

```
整数列 数字 ＝ 【１、２、３】；
印刷（数字【０】）；  （indexing — prints 1）
数字【０】＝ ９９；   （mutation）
```

Arrays have reference semantics: assigning an array to another variable aliases the same underlying storage, so mutating through either variable is visible through the other.

### Creating an Empty Array

Array literals (`【...】`) need at least one element to infer their type from. To build an array up dynamically (e.g. from an empty start), use `新配列＜型＞`:

```
取り込む 「配列」；
整数列 数字 ＝ 新配列＜整数＞；
追加（数字、１）；
追加（数字、２）；
印刷（要素数（数字））；  （prints 2）
```

### Function Declaration and Call

Parameters and call arguments are comma-separated with `、`:

```
関数 加算（整数 Ａ、整数 Ｂ）ー＞ 整数 ｛
    返す Ａ ＋ Ｂ；
｝

返す 加算（２、３）；
```

Function bodies are isolated: they only see their own parameters, not variables from the enclosing scope (matching the call-frame model of the VM — see Scoping below).

A `無`-returning function may use a bare `返す；` (no expression) to return early; non-`無` functions must always return a value, and every control-flow path through a non-`無` function's body must end in `返す` or it's a compile-time error.

### Boolean Literals

```
真偽 フラグ ＝ 真；
もし フラグ ならば ｛
    印刷（１）；
｝ 違えば ｛
    印刷（０）；
｝
```

### Built-in Functions

| Hikari | Signature | Description |
|--------|-----------|-------------|
| `文字数（s）` | `文字列 → 整数` | String length |
| `入力（）` | `→ 文字列` | Read a line from stdin |
| `整数化（s）` | `文字列 → 整数` | Parse a string as an integer |
| `小数化（s）` | `文字列 → 小数` | Parse a string as a float |
| `文字列化（n）` | `整数｜小数｜真偽 → 文字列` | Convert a value to its string form |

### Scoping

Every `｛ ... ｝` block (`もし`/`違えば`, `間`, `繰り返す`, `各`, function bodies) introduces its own scope. Variables declared inside a block are not visible after the block ends, and shadow same-named variables from an enclosing scope without corrupting them:

```
整数 値 ＝ １；
もし 真 ならば ｛
    整数 値 ＝ ２；  （shadows the outer 値 inside this block only）
｝
印刷（値）；  （prints 1 — the outer 値 is untouched）
```

### Try / Catch

`試す ｛ ... ｝ 失敗 エラー ｛ ... ｝` catches a runtime error (division by zero, type mismatch, an out-of-range index, a failed `整数化`/`小数化` conversion, etc.) raised while executing the try-body instead of crashing the program. The identifier after `失敗` is bound to the error's message as a `文字列`, scoped to the catch-body only:

```
試す ｛
    整数 結果 ＝ １ ／ ０；
｝ 失敗 エラー ｛
    印刷（「エラーを捕まえました： 」 ＋ エラー）；
｝
印刷（「続行中」）；
```

If the try-body completes without error, the catch-body is skipped entirely. Errors raised deep inside a nested function call invoked from the try-body are also caught (the VM unwinds call frames and the stack back to where the try-block started). This only catches *runtime* errors — a type error inside a try-body is still rejected at compile time, unaffected by try/catch.

### Modules

`取り込む 「name」；` imports a module. If `name` matches a recognized standard-library module (`数学`, `文字列`, `配列`, see below), it unlocks that module's builtin functions for the rest of the program — calling one before importing it is a compile-time error. Otherwise `name` is treated as a relative path to another `.hkr` file: it's parsed and only its top-level `関数` declarations are merged into the program (imports inside the imported file resolve relative to *that* file; cyclic imports are deduplicated, not an error).

```
取り込む 「utils.hkr」；
取り込む 「数学」；

印刷（二倍（２１））；        （二倍 declared in utils.hkr）
印刷（絶対値（ー５））；      （from the 数学 stdlib module）
```

### Standard Library

| Module | Function | Signature | Description |
|--------|----------|-----------|--------------|
| `数学` | `絶対値（n）` | `整数｜小数 → 同じ型` | Absolute value |
| `数学` | `平方根（n）` | `整数｜小数 → 小数` | Square root |
| `数学` | `乱数（min、max）` | `整数、整数 → 整数` | Random integer in `[min, max]` |
| `数学` | `最大（a、b）` | `整数｜小数 → 同じ型` | Larger of two values |
| `数学` | `最小（a、b）` | `整数｜小数 → 同じ型` | Smaller of two values |
| `数学` | `累乗（底、指数）` | `整数｜小数 → 同じ型` | Power (negative integer exponents are a runtime error) |
| `数学` | `切り捨て（n）` | `小数 → 整数` | Floor |
| `数学` | `切り上げ（n）` | `小数 → 整数` | Ceiling |
| `数学` | `四捨五入（n）` | `小数 → 整数` | Round to nearest |
| `数学` | `余り（a、b）` | `整数｜小数 → 同じ型` | Function form of `％` |
| `文字列` | `分割（s、区切り）` | `文字列、文字列 → 文字列列` | Split a string |
| `文字列` | `結合（配列、区切り）` | `文字列列、文字列 → 文字列` | Join a string array |
| `文字列` | `含む（s、部分）` | `文字列、文字列 → 真偽` | Substring check |
| `文字列` | `置換（s、旧、新）` | `文字列、文字列、文字列 → 文字列` | Replace all occurrences |
| `配列` | `要素数（配列）` | `配列 → 整数` | Length |
| `配列` | `追加（配列、値）` | `配列、要素型 → 無` | Append in place |
| `配列` | `取り出す（配列）` | `配列 → 要素型` | Pop the last element in place (empty array is a runtime error) |
| `配列` | `含む配列（配列、値）` | `配列、要素型 → 真偽` | Membership test |
| `配列` | `位置（配列、値）` | `配列、要素型 → 整数` | Index of first match, or `-1` if absent |
| `配列` | `逆順（配列）` | `配列 → 同じ配列` | Reverse in place |
| `配列` | `整列（配列）` | `配列 → 同じ配列` | Sort in place (numbers or strings only) |
| `配列` | `部分列（配列、開始、終了）` | `配列、整数、整数 → 新しい配列` | Slice `[開始, 終了)` — returns a copy, does not mutate the original |

### REPL

Running `hikari` with no arguments starts an interactive session. Variables, functions, and imported modules all persist across lines:

```
$ hikari
Hikari 対話モード (Ctrl+D で終了)
> 整数 値 ＝ １０；
> 印刷（値）；
10
> 値 ＝ 値 ＋ ５；
> 印刷（値）；
15
```

A bad line (parse, type, or runtime error) is reported and the session keeps going rather than exiting.

---

## Architecture

The implementation follows a classic pipeline, built strictly with TDD (200+ tests, all passing):

```
Source (.hkr)
    │
    ▼
Lexer          src/lexer.rs        — UTF-8 char stream → Vec<Token> (with line/col spans)
    │
    ▼
Parser         src/parser.rs       — Tokens → AST (recursive descent)
    │
    ▼
Type Checker   src/typechecker.rs  — AST → type-checked AST (scoped, rejects mismatches)
    │
    ▼
Compiler       src/compiler.rs     — Typed AST → Vec<Instruction> + constant pool
    │
    ▼
VM             src/vm.rs           — Stack-based bytecode interpreter with call frames
    │
    ▼
Diagnostics    src/diagnostic.rs   — Renders Japanese errors with source snippets
```

---

## Building & Running

```sh
cargo build
cargo run -- examples/if.hkr
cargo run -- examples/print.hkr
cargo run            # no file argument — starts the REPL
```

## Testing

```sh
cargo test
```

Before committing, also run:

```sh
cargo fmt
cargo check
```

---

This completes every phase of the original roadmap (フェーズ０〜６).
