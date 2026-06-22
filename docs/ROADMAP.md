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

**All roadmap phases are now complete.** Current state (434 tests passing):

| Phase | Theme | Status |
|-------|-------|--------|
| ７ | Core ops & array/math stdlib (modulo, 要素数/追加/整列/部分列, 累乗/四捨五入…) | ✅ **Done** |
| ８ | Sound control flow (`MissingReturn`, break/continue, bare `返す`) | ✅ **Done** |
| ９ | User-defined types (records, enums + `照合`, maps `辞書`) | ✅ **Done** |
| １０a | First-class functions + lambdas + map/filter/fold HOFs + **closures** | ✅ **Done** |
| １０b | Generics (parametric signatures for the polymorphic stdlib builtins) | ✅ **Done** |
| １１a | File I/O (`ファイル読む`/`ファイル書く`, `入出力` module) | ✅ **Done** |
| １１b | Formatted print — `印字` (no-newline) ✅; multi-value `印刷` ✅ | ✅ **Done** |
| １１c / １３e | Program args & env access (`引数`/`環境変数`, `環境` module) | ✅ **Done** |
| １１d | Runtime error source spans | ✅ **Done** |
| １２ | Robustness — recursion limit ✅, dynamic locals ✅, `Rc<[Instruction]>` ✅, boundary checks ✅, REPL txn ✅, lints ✅, fuzz + parser depth limit ✅ | ✅ **Done** |
| １３ | CLI & distribution — install, `--version`/`--help`, stdin/`-c`, shebang, arg passthrough ✅ | ✅ **Done** |

Every phase in this roadmap has shipped. Completed work is detailed ✅ inline below.
A possible *future* extension beyond this roadmap is **user-written** generic
functions (`関数＜Ｔ＞ …`); 10b delivered the internal parametric signatures the
roadmap called for (removing the per-builtin type special-casing).

### Shipped since this status was added

- **10b — Generics (parametric builtin signatures).** The polymorphic stdlib
  builtins (`要素数`/`追加`/`取り出す`/`含む配列`/`位置`/`逆順`/`部分列`,
  `鍵一覧`/`値一覧`/`削除`, `マップ`/`絞り込み`/`畳み込み`, `印字`) were each
  hand-checked with ~20–50 lines of element-type extraction and matching. They now
  share one table of generic signatures written with type variables
  (`src/typechecker/generics.rs`: `配列＜Ｔ＞`, `マップ＜Ｔ、Ｕ＞`-style) plus a small
  unifier that binds the variables against the actual argument types and
  instantiates the result. This removed ~360 lines of special-casing from the type
  checker with no change in behavior or error messages. (Builtins with non-parametric
  constraints — math numerics, `整列`'s orderable constraint, `含む`'s overload,
  `文字列化`'s union — stay hand-checked.)

- **12 — Fuzz testing + parser depth limit.** A dependency-free, seeded
  property/fuzz harness (`src/fuzz_tests.rs`) drives ~25k pseudo-random and
  hand-picked malformed inputs through lexer → parser → type checker → compiler,
  asserting none panic. It surfaced a real bug: deeply nested input (e.g. tens of
  thousands of `（`) overflowed the recursive-descent parser's stack. Fixed with a
  `MAX_DEPTH` guard in the parser that rejects over-nested input with a clean
  `ParseError::TooDeeplyNested` ("式または文の入れ子が深すぎます。") instead of
  aborting. With this, **Phase 12 is complete**.
- **12 — Beginner lints.** A non-fatal lint pass (`src/lints.rs`) runs after type
  checking succeeds and surfaces two warnings via `diagnostic::render_warning`:
  *unused local variable* (a `型 名前 ＝ …；` never read — parameters, loop vars,
  match binders, and the 失敗 error variable are exempt; scoped so shadowing
  resolves correctly and closure captures count as uses) and *unreachable code*
  (statements after a `返す`／`抜ける`／`続ける` in the same block). Linting runs on
  the user's own file before imports merge in library code, and is skipped in the
  REPL (where per-line "unused" would be noise). Warnings never reject a program.
- **12 — REPL transactionality.** A REPL line that fails at any stage (type,
  compile, or runtime) now leaves no half-applied state. The driver snapshots the
  persistent `TypeChecker` and `Compiler` (both `Clone`) before each line and rolls
  them back on failure, so e.g. `整数 ａ ＝ １； 整数 ｂ ＝ 「x」；` no longer
  half-declares `ａ`. The VM also resets its transient state (drops in-progress call
  frames, clears the stack and pending try handlers, parks frame 0) on an uncaught
  error in `run_repl_line` — fixing a latent bug where leftover call frames could
  resume on the next line. Persistent bindings in frame 0 survive; only the failed
  line's effects are discarded (already-printed output and in-place mutations of
  pre-existing collections are inherently not rolled back).
- **12 — Bytecode boundary hardening.** The bytecode encodes some counts in
  fixed-width fields (`u16` constant-pool/jump/chunk indices, `u8` arg/payload/
  capture counts); exceeding one used to silently wrap and miscompile. `compile`
  now returns `Result<_, CompileError>`: `u8` sites are checked inline
  (`count_u8`, recording the first overflow), and `u16` structural limits are a
  cheap post-pass — one per-chunk instruction-count check covers every jump-offset
  and literal-size field at once, since those are all bounded by the chunk length.
  A "プログラムが大きすぎます" diagnostic replaces the silent wrap. (These limits
  are unreachable in hand-written programs; this guards against corruption.)
- **12 — Cheap call frames (`Rc<[Instruction]>`).** `Chunk` now holds its
  instructions and span checkpoints as `Rc<[…]>`, and a call `Frame` shares them
  with an O(1) refcount bump instead of cloning the whole body on every call. This
  makes recursion O(depth) instead of O(depth × body size). The REPL rebuilds
  frame 0's slice when appending a line (once per line, not a hot path).
- **12 — Dynamic locals (already resolved).** The earlier "fixed 256 locals that
  silently corrupt" concern is moot: `INITIAL_LOCALS` is only a starting capacity
  and `Frame::set_local` grows the slot vector on demand.
- **11b — Multi-value `印刷`.** `印刷` takes zero or more `、`-separated values,
  printed space-separated with a trailing newline (`印刷（）` prints a blank line).
  `Stmt::Print` now holds a `Vec<Expr>`; the `Print` instruction became
  `PrintLine(u16)` (pops n, joins with a space). String interpolation was left out
  deliberately: `＋` with `文字列化` already covers it without new syntax, and `印字`
  (11a) handles no-newline output.

- **11c / 13e — Program args & environment.** `取り込む 「環境」` unlocks
  `引数（）→文字列列` (the CLI args after the script path / `-c` code / `-`, empty in
  the REPL) and `環境変数（名前）→文字列` (missing reads as `「」`). The VM stores the
  args (`Vm::set_program_args`); `引数` is handled in `step()` like the HOFs, while
  `環境変数` is a pure `call_builtin` arm. New stdlib module `環境` (`MOD_ENV`).
- **10a — Closures.** `Value::Function` gained a `captured: Vec<Value>` and a new
  `MakeClosure` instruction. A lambda is now lexically scoped: the compiler runs a
  free-variable analysis (`free_vars` in `codegen.rs`), captures enclosing locals
  **by value**, and seeds them into the callee's locals right after the params — so
  the body reads/writes them as ordinary locals (no upvalue instruction). Capture is
  by value (reference types still alias via their `Rc`), nested lambdas compose, and
  HOFs (`マップ`/`絞り込み`/`畳み込み`) accept closures. Named `関数` bodies stay
  isolated.
- **11a — File I/O.** `取り込む 「入出力」` unlocks `ファイル読む（パス）→文字列`,
  `ファイル書く（パス、内容）→無`, and `印字（値）→無` (print without a trailing
  newline). New `RuntimeError::IoError`. New stdlib module `入出力` (`MOD_IO`).
- **11d — Runtime error source spans.** Each `Chunk` (and call `Frame`) now carries
  statement-level span checkpoints `(instruction_index, span)`, emitted by the
  compiler. On an uncaught runtime error the VM records the span of the failing
  instruction (`Vm::error_span`), so `main` renders division-by-zero, out-of-bounds,
  missing-key, overflow, etc. with the same source-snippet diagnostic as
  compile-time errors — including errors raised deep inside function bodies, which
  point at the failing statement rather than the call site. Granularity is
  statement-level (Hikari `Expr` nodes carry no spans).
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

**10a. 関数値とラムダ + クロージャ** — ✅ *done.*
Functions as values, a `関数型` type, anonymous functions, the
`マップ`/`絞り込み`/`畳み込み` HOFs, and now **closures** all ship. Lambdas are
lexically scoped and capture enclosing locals **by value**: the compiler's
free-variable analysis (`free_vars` in `codegen.rs`) finds the captured names,
pushes their current values, and a `MakeClosure` instruction bundles them into the
`Value::Function`'s `captured` vec. At call time captures are seeded into the
callee's locals right after the params, so the body reads them as ordinary locals —
no upvalue instruction needed. Reference types (arrays/records/maps) still alias via
their `Rc`; nested lambdas compose; named `関数` bodies remain isolated.

**10b. ジェネリクス（Generics）** — ✅ *done.*
The polymorphic stdlib builtins now share parametric signatures written with type
variables (`配列＜Ｔ＞`, generic `要素数`, `マップ＜Ｔ、Ｕ＞`) plus a unifier
(`src/typechecker/generics.rs`), replacing the per-builtin hand-checking in the type
checker (~360 lines removed). User-written generic functions (`関数＜Ｔ＞ …`) remain
a possible future extension beyond this roadmap.

---

## フェーズ１１ — 入出力と実行環境（I/O & Runtime）

**11a. ファイル入出力** — `ファイル読む（パス）`, `ファイル書く（パス、内容）`.
**11b. 書式付き出力** — ✅ *done.* `印刷` now accepts zero or more `、`-separated
values, printed space-separated with a trailing newline (`PrintLine(u16)`); the
no-newline variant `印字` shipped in 11a. Dedicated interpolation syntax was skipped
on purpose — `＋` with `文字列化` already builds formatted strings without new lexer
work.
**11c. プログラム引数と環境** — ✅ *done.* The `環境` module's `引数（）→文字列列`
returns the CLI args passed after the script path (or after `-c`/`-`), and
`環境変数（名前）→文字列` reads an environment variable (missing → empty string).
This also covers **13e**'s arg-passthrough.
**11d. 実行時エラーの位置情報** — ✅ *done.* Previously runtime errors carried no
source span, so a division-by-zero or out-of-bounds index couldn't point at a line —
a jarring drop in quality from the compile-time diagnostics. Now each `Chunk` and
call `Frame` carries statement-level span checkpoints `(instruction_index, span)`
emitted by the compiler; on an uncaught error the VM records the failing
instruction's span (`Vm::error_span`) and `main` renders it with the standard
source-snippet diagnostic. Errors inside function bodies point at the failing
statement, not the call site. (Granularity is statement-level: Hikari `Expr` nodes
carry no spans, so sub-expression precision would require adding spans to the AST.)

---

## フェーズ１２ — 堅牢性とツール（Robustness & Tooling）

These harden the implementation itself rather than adding language features.

- **任意精度・境界の見直し ✅ done:** the constant pool is `u16`-indexed, arg counts
  are `u8`, and chunk/jump indices are `u16`. Exceeding any of these used to silently
  wrap in the compiler; `compile` now returns `Result<_, CompileError>` and rejects
  such a program with a "プログラムが大きすぎます" diagnostic. `u8` counts are checked
  inline; the `u16` structural limits are a per-chunk post-pass (one
  instruction-count check covers all offset and literal-size fields). These limits
  are unreachable in hand-written programs; the check guards against corruption.
- **再帰の性能 ✅ done:** `Chunk` instructions/spans are now `Rc<[…]>` and a call
  `Frame` shares them (refcount bump), so recursion is O(depth), not
  O(depth × body size). Previously `Frame::new` cloned the whole body per call.
- **フレームのローカル ✅ done:** `INITIAL_LOCALS` is just a starting capacity;
  `Frame::set_local` grows the slot vector on demand, so there is no hard 256-slot
  ceiling and no silent corruption.
- **スタック深度の上限 ✅ done:** a `MAX_FRAME_DEPTH` (1024) guard on every
  frame-push path raises a clean, catchable `再帰が深すぎます`
  (`RuntimeError::StackOverflow`) instead of unbounded growth.
- **未使用変数・到達不能コードの警告 ✅ done:** a lint pass (`src/lints.rs`) warns
  on unused local variables and code after a `返す`／`抜ける`／`続ける`. Non-fatal;
  rendered with `diagnostic::render_warning` after type checking passes.
- **REPL のトランザクション性 ✅ done:** the driver snapshots the persistent
  `TypeChecker` and `Compiler` before each line and restores them if the line fails
  at any stage, and the VM resets its transient state on an uncaught error, so a
  partially-evaluated line leaves no half-applied declarations.
- **テスト ✅ done:** a seeded, dependency-free property/fuzz harness
  (`src/fuzz_tests.rs`) drives random and malformed input through the whole front
  end asserting no panics. It found a parser stack-overflow on deeply nested input,
  now guarded by a `MAX_DEPTH` limit (`ParseError::TooDeeplyNested`).

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

**13e. 終了コードと引数の引き渡し** — ✅ *done.* Process exit codes are meaningful
(0 on success, non-zero on error via `process::exit`), and the script's own CLI
arguments are exposed through `引数（）` (see **11c**).

**Milestone:** `chmod +x hello.hkr && ./hello.hkr`, and
`echo "印刷（「やあ」）；" | hikari -`, both work after `cargo install`.

---

## Status: complete

**Every phase of this roadmap has shipped** — 7–10 (closures + generics), all of
11 (I/O, formatted print, program args/env, runtime spans), all of 12 (recursion
limit, dynamic locals, cheap frames, boundary hardening, REPL transactionality,
lints, fuzz testing + parser depth limit), and 13 (the Python-like CLI). Hikari
went from "a language you can solve beginner exercises in" to a small but genuinely
general-purpose language.

The original v2 plan is fully shipped. What follows is **Roadmap v3** — the next
horizon, informed by a fresh code review.

---

# Hikari Roadmap v3 — From "general purpose" to "robust & ergonomic"

Updated 2026-06-20. v2 made Hikari a small general-purpose language. v3 has three
goals: **(0)** fix the soundness/quality bugs the review surfaced, **(1)** raise
expressive power (user generics, options, richer types), and **(2)** make Hikari
pleasant to *live in* (better errors, tooling, a real stdlib, a module system).

Phases are independently shippable and roughly ordered by impact. The detailed
bug write-ups live in [KNOWN_ISSUES.md](KNOWN_ISSUES.md); internals they touch are
described in [ARCHITECTURE.md](ARCHITECTURE.md).

## Status (v3)

| Phase | Theme | Status |
|-------|-------|--------|
| １４ | Correctness fixes from the review | ✅ **Done** |
| １５ | Optionality & error values (`省略可`, `?`, `結果` sugar) | ✅ **Done** (15a+15b; 15c deferred) |
| １６ | User-written generics (`関数＜Ｔ＞ …`) | ✅ **Done** |
| １７ | Standard-library expansion (string/number/collection/time/JSON) | ✅ **Done** (17a–17e; JSON deferred) |
| １８ | A real module & namespace system | ⬜ Planned |
| １９ | Diagnostics & developer tooling (fmt, expr spans, multi-error) | ⬜ Planned |
| ２０ | Performance (peephole, constant folding, faster dispatch) | ⬜ Planned |

---

## フェーズ１４ — レビュー指摘の修正（Correctness Fixes）

*Small, high-value fixes; ship first. Each maps to a KNOWN_ISSUES entry.*

**14a. Exhaustive `照合` counts as a returning path.** Add a `Stmt::Match` arm to
`always_returns` ([symbols.rs](../src/typechecker/symbols.rs:100)) returning
`arms.iter().all(|a| always_returns(&a.body))`. Exhaustiveness is already proven
by the checker, so this is sound. Removes the need for a dead trailing `返す`.
*(KNOWN_ISSUES #1.)*

**14b. Float display keeps the decimal point.** `display_value` should render an
integral `小数` as `１．０`, not `１` ([value_ops.rs](../src/vm/value_ops.rs:8)).
Decide `NaN`/`inf` rendering at the same time. *(KNOWN_ISSUES #3.)*

**14c. `絶対値` overflow is checked.** Replace `wrapping_abs` with
`checked_abs → IntegerOverflow` for consistency with all other integer ops.
*(KNOWN_ISSUES #4.)*

**14d. Defensive sort comparators.** Replace the `_ => Equal` fallbacks in
`sort_values`/`MapKeys` with `unreachable!`/`TypeMismatch`. *(KNOWN_ISSUES #5.)*

**14e. Imported files are self-contained.** Splice top-level `取り込む`/`型`/
`構造` from imported files (not just `関数`), or at minimum emit a diagnostic that
names the imported file instead of pointing into the importer. Folds into Phase 18
if a fuller module system lands first. *(KNOWN_ISSUES #2.)*

**マイルストーン:** the four KNOWN_ISSUES reproductions all behave correctly; no
regression in the 434-test suite.

---

## フェーズ１５ — 省略可能性とエラー値（Optionality & Errors as Values）

**Motivation.** Several builtins encode "absence" as sentinels: `位置` returns
`-1`, `環境変数` returns `「」`, map lookup *raises* on a missing key. A
first-class option type makes absence explicit and type-checked, and pairs with
the existing `照合` machinery.

**15a. `省略可＜Ｔ＞` (Option).** A built-in generic sum type with variants `有る（Ｔ）`
and `無し`. Lowers to the existing enum representation (`Value::Enum`), so no new
runtime — just checker/parser support and `照合` integration:

```
省略可＜整数＞ ｖ ＝ 探す（スコア、「アリス」）；
照合 ｖ ｛
  有る（ｎ） ならば ｛ 印刷（ｎ）； ｝
  無し（） ならば ｛ 印刷（「見つかりません」）； ｝
｝
```

**15b. Safe map/array access returning `省略可`.** Add `取得（m、key）→省略可＜Ｖ＞`
and `取得（配列、添字）→省略可＜Ｔ＞` as non-raising lookups; migrate `位置` to return
`省略可＜整数＞` (or add `位置可`). Existing raising index syntax (`m【key】`) stays.

**15c. `結果＜Ｔ、Ｅ＞` sugar (optional).** A standard `成功（Ｔ）`/`失敗（Ｅ）` enum plus
a `？` postfix that early-returns the error — bridging `照合`-style error values
and `試す/失敗`. Lowers to a match + `返す`.

**Dependency:** 15c benefits from Phase 16 (user generics) but `省略可`/`結果` can
ship as *built-in* generics first.

---

## フェーズ１６ — ユーザー定義ジェネリクス（User-Written Generics） ✅ Done

**16a ✅ Done. Generic function declarations.** `関数＜Ｔ＞ 恒等（Ｔ ｘ）ー＞ Ｔ ｛ 返す ｘ； ｝`
and multi-parameter forms `関数＜Ｔ、Ｕ＞ …` are fully supported. The parser
reads a `＜…＞` type-variable list on `関数`; the checker registers each name as a
scoped type variable (exempting it from `UndeclaredType`); at call sites the
substitution is inferred from argument types using a `HikariType`-based unifier,
and the return type is instantiated accordingly. Also added `配列＜Ｔ＞` as a
general parameterized array type syntax (formerly only `整数列` / `小数列` etc.
shorthands existed), needed for generic functions over arrays. 20 tests (12
typechecker + 8 VM). No VM changes — checker-only generics with a single shared
chunk per function (no monomorphization needed, as the VM is already type-erased).

**16b Deferred. Generic records/enums.** `型 箱＜Ｔ＞ ｛ Ｔ 値； ｝` and
`構造 対＜Ａ、Ｂ＞ ｛ … ｝`. Deferred — the `HikariType::Record(name)` representation
would need extending to carry type arguments at instantiation sites.

**16c ✅ Implemented as design note.** The VM is already type-erased at runtime
(`Value` is `Value`), so checker-only generics with a single shared chunk per
generic function require no monomorphization — documented and applied in 16a.

**Design notes.** Inference is local (per call site), no global HM unification or
let-generalization. Bounds (`整列可`/orderable) are deferred to a later phase.

---

## フェーズ１７ — 標準ライブラリの拡充（Standard Library）

*Mostly new `builtin_sig`/`generic_builtin_sig` + `call_builtin` arms — little new
machinery. Group by module; each is independently shippable.*

**17a. 文字列 (richer strings).** `大文字`/`小文字` (upper/lower), `整形`
(trim), `先頭一致`/`末尾一致` (starts/ends-with), `部分文字列（s、開始、終了）`,
`文字列位置（s、部分）→省略可＜整数＞`, `繰り返し文字列（s、回数）`. Define string
indexing semantics explicitly (char-based, consistent with `文字数`).

**17b. 数学 (more numerics).** `符号` (sign), `挟む（値、下、上）` (clamp),
`総和`/`平均`/`最大値`/`最小値` over numeric arrays, trig/log behind the existing
`数学` gate.

**17c. 配列 (more collections).** `平坦化` (flatten), `連結` (concat),
`重複除去` (dedup), `分割（配列、サイズ）` (chunk), `畳み込み右` (foldr),
`どれか`/`すべて` (any/all over a predicate), `数える（配列、述語）`.

**17d. 辞書 (more maps).** `併合（a、b）` (merge), `数（m）` (size),
`取得既定（m、key、既定）` (get-or-default), `項目一覧` (entries as a pair array
— needs Phase 16's generic `対＜Ａ、Ｂ＞`).

**17e. 時間 (time) module.** `現在時刻（）→整数` (epoch millis), `経過（開始）`,
`眠る（ミリ秒）`. New `MOD_TIME`.

**17f. JSON / 直列化 module.** `JSON化（値）→文字列` and `JSON解析（文字列）→…`.
Non-trivial without a dynamic/`任意` type — likely depends on Phase 15/16
(an `JSON値` enum). Tracked here, scheduled after generics.

---

## フェーズ１８ — モジュールと名前空間（Modules & Namespaces）

**Motivation.** Today `取り込む` flattens a file's top-level `関数` into one global
namespace — name collisions are silent, and a file's types/imports are dropped
(KNOWN_ISSUES #2). v3 makes modules real.

**18a. Namespaced imports.** `取り込む 「幾何」 として 幾何；` then `幾何。距離（…）`
(or `幾何：距離`). Requires qualified-name resolution in the parser/checker and a
module symbol table keyed by alias.

**18b. Self-contained library files.** Resolve and merge an imported file's own
`取り込む`/`型`/`構造`, scoped to that module (subsumes 14e). Prevents a library's
internal stdlib needs from leaking to the importer.

**18c. Export control.** A way to mark which `関数`/`型` are public (e.g. a
`公開` keyword) so libraries can have private helpers.

**18d. A standard search path.** Beyond relative paths, an env var
(`HIKARI＿PATH`) or a conventional `ライブラリ/` directory so shared modules don't
need relative `../../` paths.

---

## フェーズ１９ — 診断と開発ツール（Diagnostics & Tooling）

**19a. Expression-level spans.** Add `Span` to `Expr` nodes so type errors and
runtime errors point at the offending *sub-expression*, not just the statement
(today's granularity, per ARCHITECTURE §8). The single biggest diagnostic-quality
win; touches the parser, AST, checker, and the compiler's span checkpoints.

**19b. Multi-error reporting.** Collect and report several type errors per run
instead of stopping at the first, so a beginner sees all problems at once.
Requires the checker to accumulate errors and recover at statement boundaries.

**19c. A formatter (`hikari 整形`).** Canonical pretty-printer for `.hkr` files
(the AST `Display` in [display.rs](../src/parser/display.rs) is a starting point).
Critical for a full-width language where spacing is easy to get wrong.

**19d. Editor support.** An LSP shim (or at least a TextMate/Tree-sitter grammar)
for syntax highlighting, go-to-definition, and inline diagnostics. Lower priority;
big ergonomics payoff.

**19e. Lint expansion.** Build on [lints.rs](../src/lints.rs): unused functions,
unused imports, shadowed-without-use, constant conditions, `照合` arms that can
never match.

---

## フェーズ２０ — 性能（Performance）

*Only if real programs get big enough to need it — correctness and ergonomics
first.*

**20a. Constant folding & peephole.** Fold literal arithmetic at compile time;
collapse `LoadConst`-then-`Negate`, redundant jumps, and dead `Jump`-to-next.

**20b. Faster dispatch.** Profile the `step` loop; consider a computed-goto-style
dispatch or instruction superinstructions for hot patterns (loop increments).

**20c. Local-slot reuse.** `next_slot` grows monotonically per function
(ARCHITECTURE §6); reclaim slots on scope exit to shrink frames for large
functions.

**20d. String interning / small-value optimization.** If string-heavy programs
dominate, intern constant-pool strings and consider a `SmallVec`-backed stack.

---

## Sequencing summary

1. **Phase 14** first — small, fixes real bugs, no dependencies.
2. **Phases 15 + 16** are the expressiveness core; `省略可`/`結果` can ship as
   built-in generics (15) before user generics (16), then 15c/17f build on 16.
3. **Phase 17** (stdlib) can proceed in parallel, module by module.
4. **Phase 18** (modules) subsumes 14e and unblocks larger codebases.
5. **Phases 19–20** (tooling, perf) are ongoing quality work, lowest urgency.
