# Hikari Roadmap v2 — Toward a Fully Functional Language

The original roadmap (フェーズ０〜６) is complete: Hikari has a lexer, a scoped
static type checker, a bytecode compiler, a stack VM, Japanese diagnostics,
arrays, loops, try/catch, modules, a small standard library, and a REPL.

This document captures what a **comprehensive code review** found still missing
before Hikari is a genuinely usable general-purpose language, and proposes a
prioritized plan. Items are grouped by theme; within each phase they are roughly
ordered by impact. Each phase is independently shippable.

> Note: the six concrete *bugs* the review found (void-function halting, operator
> type-soundness holes, lexer/VM panics on bad numbers and overflow) are fixed
> separately and are **not** part of this roadmap — this document is about
> missing *capabilities*.

---

## Status (updated 2026-06-18)

Since v2 was first written, most of the early phases have shipped. Current state
(369 tests passing):

| Phase | Theme | Status |
|-------|-------|--------|
| ７ | Core ops & array/math stdlib (modulo, 要素数/追加/整列/部分列, 累乗/四捨五入…) | ✅ **Done** |
| ８ | Sound control flow (`MissingReturn`, break/continue, bare `返す`) | ✅ **Done** |
| ９ | User-defined types (records, enums + `照合`, maps `辞書`) | ✅ **Done** |
| １０a | First-class functions + lambdas + map/filter/fold HOFs | 🟡 **Partial** — lambdas are **non-capturing** (no closures) |
| １０b | Generics | ❌ Not started |
| １１a | File I/O (`ファイル読む`/`ファイル書く`, `入出力` module) | ✅ **Done** |
| １１b | Formatted print — `印字` (no-newline) done; multi-value/interpolation | 🟡 **Partial** |
| １１c | Program args / env access | ❌ Not started |
| １１d | Runtime error source spans | ❌ Not started |
| １２ | Robustness — recursion limit ✅; `Rc<[Instruction]>`, boundary checks, lints | 🟡 **Partial** |
| １３ | CLI & distribution — install, `--version`/`--help`, stdin/`-c`, shebang ✅; arg passthrough ❌ | 🟡 **Partial** |

The remaining sections below describe the open work. Completed work is marked ✅
inline. Current focus: **closures (10a), runtime spans (11d), remaining robustness
(12), and arg passthrough (11c/13e).**

### Shipped since this status was added

- **11a — File I/O.** `取り込む 「入出力」` unlocks `ファイル読む（パス）→文字列`,
  `ファイル書く（パス、内容）→無`, and `印字（値）→無` (print without a trailing
  newline). New `RuntimeError::IoError`. New stdlib module `入出力` (`MOD_IO`).
- **12 — Recursion depth limit.** A `MAX_FRAME_DEPTH` (1024) guard in every
  frame-push path raises a clean `RuntimeError::StackOverflow`
  (`再帰が深すぎます`) — catchable by try/catch — instead of unbounded growth.
- **13 — CLI & distribution.** `cargo install --path .` yields a `hikari` command;
  `--version`/`-v`, `--help`/`-h`, `hikari -` (stdin), and `hikari -c "<code>"` are
  supported; the lexer skips a leading ASCII `#!` shebang so `.hkr` files can be
  made directly executable.

---

## フェーズ７ — 基本演算と配列操作の完成（Core Operations & Array Stdlib） ✅ DONE

**The most impactful gap: common programs are currently inexpressible.**

**7a. 剰余演算（Modulo）** — *blocking for many beginner programs.*
There is no remainder operator, so canonical exercises like FizzBuzz
(`Ｎ ％ ３ ＝＝ ０`) cannot be written at all. Add a `％` token, `BinOpKind::Mod`,
an `Instruction::Mod`, and VM support (Int→Int with a zero-divisor check reusing
`DivisionByZero`; decide whether to allow Float modulo). Type-check like the other
arithmetic operators (numbers only).

**7b. 配列の標準関数（Array Builtins）** — *arrays are currently nearly write-only.*
User code cannot even ask an array its length. Add gated/ungated builtins:
- `要素数（配列）` → 整数 (length; the VM already has an `ArrayLen` instruction used
  internally by for-each — expose it).
- `追加（配列、値）` (append), `取り出す（配列）` (pop) — requires arrays to support
  growth; `Value::Array` is already `Rc<RefCell<Vec<Value>>>`, so mutation is cheap.
- `含む配列（配列、値）` (membership), `位置（配列、値）` (indexOf), `逆順（配列）` (reverse),
  `整列（配列）` (sort, numbers/strings only), `部分列（配列、開始、終了）` (slice).
- A `空配列＜型＞` literal or `新配列（型）` so empty typed arrays can be created
  (today `【】` is rejected as `EmptyArrayLiteral` with no alternative).

**7c. 数学関数の拡充（More Math）**
`累乗（底、指数）` (power), `切り捨て`/`切り上げ`/`四捨五入` (floor/ceil/round),
`余り（a、b）` as a function form, optional trig. Extends the existing `数学` module
— no new machinery, just more `builtin_sig` / `call_builtin` arms.

**マイルストーン:** real FizzBuzz; reading N numbers into an array and sorting them.

---

## フェーズ８ — 制御フローと関数の健全性（Sound Control Flow） ✅ DONE

**8a. 全経路リターン解析（Exhaustive-Return Analysis）** — *type-soundness gap.*
A non-`無` function that doesn't return on every path currently compiles and then
fails at runtime with a `StackUnderflow` (the called frame falls off its end with
nothing on the stack). Add a static check: every path through a non-void function
body must end in `返す`. Report a dedicated `TypeError::MissingReturn`.

**8b. ループ制御（break / continue）**
`抜ける` (break) and `続ける` (continue) for `間`/`繰り返す`/`各`. Needs the compiler
to track enclosing-loop patch points and the VM nothing new (just `Jump`s).

**8c. 早期リターンと `無` 値（Void Semantics）**
Allow `返す；` (bare return) in `無` functions, and decide whether `無`-typed
expression positions are ever valid (currently a void call can't be used as a value,
which the type checker correctly rejects — document this as intended).

---

## フェーズ９ — ユーザー定義型（User-Defined Types） ✅ DONE

This is the largest single leap toward "general purpose."

**9a. レコード型（Structs / Records）**
`型 点 ｛ 整数 ｘ； 整数 ｙ； ｝` with construction (`点 ｛ ｘ：１、ｙ：２ ｝`), field
access (`ｐ。ｘ`), and field assignment. Requires: a new `HikariType::Record(name)`,
a registry of declared types in the type checker, `Value::Record(Rc<RefCell<...>>)`
in the VM, and `GetField`/`SetField`/`MakeRecord` instructions.

**9b. 構造型とパターン照合（Enums & Pattern Matching）**
Sum types (`構造 結果 ｛ 成功（整数）、失敗（文字列）｝`) plus a `照合`/match statement.
This subsumes a lot: it gives a principled way to model absence/optionality and
errors as values, complementing try/catch.

**9c. 連想配列（Maps / Dictionaries）**
`辞書＜文字列、整数＞` with literal, lookup, insert, and `鍵一覧`/`値一覧`. Backed by
a `Value::Map`. Hugely useful for real programs (counting, grouping, caching).

---

## フェーズ１０ — 第一級関数（First-Class Functions）

**10a. 関数値とラムダ** — 🟡 *partially done.*
Functions as values, a `関数型` type, and anonymous functions are **implemented**,
and `マップ`/`絞り込み`/`畳み込み` (map/filter/fold) ship as library functions. What
remains is **closures**: today's lambdas are *non-capturing* — a lambda body cannot
reference variables from the enclosing scope (function bodies are deliberately
isolated, matching the call-frame model). True closures need captured-environment
support in the VM (an upvalue/environment mechanism on `Value::Function`). Until
then, HOFs can only use self-contained lambdas, which sharply limits their utility.
This is the single most surprising gap for users who reach for functional patterns.

**10b. ジェネリクス（Generics）**
Even minimal parametric types (`配列＜Ｔ＞`, generic `要素数`, generic
`マップ＜Ｔ、Ｕ＞`) would remove the current need to special-case every builtin's
types by hand in the type checker.

---

## フェーズ１１ — 入出力と実行環境（I/O & Runtime）

**11a. ファイル入出力** — `ファイル読む（パス）`, `ファイル書く（パス、内容）`.
**11b. 書式付き出力** — `印刷` of multiple values / interpolation, and a no-newline
variant.
**11c. プログラム引数と環境** — access to CLI args / env vars from a running program.
**11d. 実行時エラーの位置情報** — *highest-leverage item.* Runtime errors currently
carry no source span: `RuntimeError` (`src/vm/error.rs`) has no line/col, so a
division-by-zero or out-of-bounds index can't point at a line — a jarring drop in
quality from the excellent compile-time diagnostics. Thread spans into the bytecode
(parallel to the instruction vector) so runtime diagnostics match compile-time ones.

---

## フェーズ１２ — 堅牢性とツール（Robustness & Tooling）

These harden the implementation itself rather than adding language features.

- **任意精度・境界の見直し:** the constant pool is `u16`-indexed, arg counts are
  `u8`, and each frame has a fixed 256 locals — all silently wrap/corrupt at the
  boundary. Replace with checked widening or dynamic sizing.
- **再帰の性能:** `Frame::new` clones the whole chunk's instruction vector on every
  call (`chunk.instructions.clone()` in `src/vm/frame.rs`), making recursion
  O(chunk size) per call. Share instructions via `Rc<[Instruction]>` so frames are cheap.
- **スタック深度の上限 (still open):** the call path in `src/vm/machine.rs` has **no
  recursion/frame-depth guard** — `frame_depth` exists only for try/catch unwinding,
  so infinite recursion grows memory unbounded instead of raising a clean
  `再帰が深すぎます` error. Small fix, high safety payoff.
- **未使用変数・到達不能コードの警告:** beginner-friendly lints.
- **REPL のトランザクション性:** a line that type-checks partway then fails currently
  leaves the persistent checker with half-declared state; make per-line evaluation
  all-or-nothing.
- **テスト:** property-based / fuzz testing of the lexer and parser to catch the next
  class of panics-on-malformed-input before users do.

---

## フェーズ１３ — CLI と配布（CLI & Distribution）

**Goal: make Hikari runnable like Python — a `hikari` command on `PATH` with the
ergonomics users expect of an interpreter.** The language mechanics already exist:
`hikari ファイル.hkr` runs a file and bare `hikari` starts the REPL (`src/main.rs`).
What's missing is the distribution and CLI surface:

**13a. インストール可能なバイナリ** — document/support `cargo install --path .` (or a
release build + symlink) so users get a global `hikari` command instead of
`cargo run -- …`. Add a `[[bin]]` name if needed and a short install section to the
README.

**13b. 引数パーサの整備** — the current arg handling is hand-rolled `args.len()`
checks that reject anything but a single path. Add `--version`/`-v`,
`--help`/`-h`, and graceful unknown-flag handling.

**13c. 標準入力からの実行** — Python-style `hikari -` (read program from stdin) and
optionally `hikari -c "コード"` (inline). Enables piping and one-liners.

**13d. シェバン対応 (`#!/usr/bin/env hikari`)** — let `.hkr` files be directly
executable. The lexer must skip a leading ASCII `#!` line (Hikari's own comment
marker is full-width `＃`, so an ASCII shebang currently fails to lex). Then a
chmod-+x script with a shebang runs as a normal executable.

**13e. 終了コードと引数の引き渡し** — propagate meaningful process exit codes
(0 success, non-zero on error — partly there via `process::exit`) and expose the
script's own CLI arguments to the running program (overlaps **11c**).

**Milestone:** `chmod +x hello.hkr && ./hello.hkr`, and
`echo "印刷（「やあ」）；" | hikari -`, both work after `cargo install`.

---

## Suggested ordering

```
Phase 11d (runtime error spans)   ← highest leverage; infra partly exists; closes the
                                     biggest compile-time/runtime quality gap
Phase 12  (recursion limit first) ← turn an OOM crash into a clean error; trivial & safe
Phase 11a/b (file I/O + print)    ← the difference between "toy" and "writes real programs"
Phase 13  (CLI & distribution)    ← makes `hikari` feel like python; mostly small, independent
Phase 10a (closures)              ← unlocks the HOFs already shipped; largest design effort here
Phase 12  (Rc<[Instruction]> + boundary hardening) ← mechanical perf/safety; do alongside
Phase 10b (generics)              ← last; biggest design cost, lowest completeness payoff
```

Phases ７–９ are complete. If only one thing ships next, make it **11d (runtime error
spans)** — the infrastructure to thread spans is partly in place, and it removes the
most jarring inconsistency users hit. For the *Python-like CLI* goal specifically,
**Phase 13** is largely independent of the language work and can be done at any time;
13a alone (a `cargo install`-able binary) already gets you a real `hikari` command.
