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

### Variable Declaration

```
整数 年齢 ＝ ２０；
```

### Arithmetic (with operator precedence)

```
整数 結果 ＝ ２ ＋ ３ ＊ ４；  （＊ binds tighter than ＋）
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
    整数 カウンタ ＝ カウンタ ＋ １；
｝
```

### Function Declaration and Call

```
関数 二倍（整数 Ａ）ー＞ 整数 ｛
    返す Ａ ＊ ２；
｝

返す 二倍（２１）；
```

### Boolean Literals

```
真偽 フラグ ＝ 真；
もし フラグ ならば ｛
    印刷（１）；
｝ 違えば ｛
    印刷（０）；
｝
```

### Comparison Operators

| Symbol | Meaning |
|--------|---------|
| `＝＝` | Equal |
| `＜` | Less than |
| `＞` | Greater than |

---

## Architecture

The implementation follows a classic pipeline, built strictly with TDD (60 tests, all passing):

```
Source (.hkr)
    │
    ▼
Lexer          src/lexer.rs        — UTF-8 char stream → Vec<Token>
    │
    ▼
Parser         src/parser.rs       — Tokens → AST (recursive descent)
    │
    ▼
Type Checker   src/typechecker.rs  — AST → type-checked AST (rejects mismatches)
    │
    ▼
Compiler       src/compiler.rs     — Typed AST → Vec<Instruction> + constant pool
    │
    ▼
VM             src/vm.rs           — Stack-based bytecode interpreter with call frames
```

---

## Building & Running

```sh
cargo build
cargo run -- examples/if.hkr
cargo run -- examples/print.hkr
```

## Testing

```sh
cargo test
```

---

## Current Status

| Feature | Status |
|---------|--------|
| Lexer — all keywords, operators, literals | ✅ Done |
| Parser — variable decls, functions, expressions | ✅ Done |
| Type checker — strict static typing, no implicit coercions | ✅ Done |
| Bytecode compiler — constant pool, local slots | ✅ Done |
| Stack-based VM — call frames, arithmetic | ✅ Done |
| CLI entry point (`hikari <file.hkr>`) | ✅ Done |
| Function declaration and call dispatch | ✅ Done |
| `印刷` built-in (print) | ✅ Done |
| `もし…ならば…違えば` (if/else) | ✅ Done |
| Comparison operators (`＝＝` `＜` `＞`) | ✅ Done |
| `間…ならば` (while loop) | ✅ Done |
| Error recovery (`Result`-based parser/VM errors) | ✅ Done |
| `真` / `偽` boolean literals in programs | ✅ Done |

---

## Next Steps

| Feature | Notes |
|---------|-------|
| Multiple function parameters | Parser handles one param; needs comma-separated param list |
| String concatenation | `＋` on `文字列` operands |
| Standard library builtins | e.g. `長さ` (length), numeric conversions |
