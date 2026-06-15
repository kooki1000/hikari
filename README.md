# Hikari (光)

A statically-typed, stack-based bytecode language built in Rust, with Japanese keywords and full-width UTF-8 syntax.

## Language Overview

Hikari's syntax uses Japanese reserved words and full-width (ZenKaku) characters for all operators and punctuation. There are no ASCII symbols in valid Hikari source code.

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

### Function Declaration

```
関数 計算（整数 Ａ）ー＞ 整数 ｛
    返す Ａ ＋ １；
｝
```

## Architecture

The implementation follows a classic pipeline, built strictly with TDD:

```
Source (.hkr)
    │
    ▼
Lexer          src/lexer.rs        — UTF-8 char stream → Vec<Token>
    │
    ▼
Parser         src/parser.rs       — Tokens → AST
    │
    ▼
Type Checker   src/typechecker.rs  — AST → Typed AST (rejects type mismatches)
    │
    ▼
Compiler       src/compiler.rs     — Typed AST → Vec<Instruction> + constant pool
    │
    ▼
VM             src/vm.rs           — Stack-based bytecode interpreter
```

## Building & Testing

```sh
cargo build
cargo test
```

## Status

- [x] Phase 1 — Lexer
- [ ] Phase 2 — Parser
- [ ] Phase 3 — Type Checker
- [ ] Phase 4 — Bytecode Compiler
- [ ] Phase 5 — Virtual Machine
