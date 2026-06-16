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

## フェーズ７ — 基本演算と配列操作の完成（Core Operations & Array Stdlib）

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

## フェーズ８ — 制御フローと関数の健全性（Sound Control Flow）

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

## フェーズ９ — ユーザー定義型（User-Defined Types）

This is the largest single leap toward "general purpose."

**9a. レコード型（Structs / Records）**
`型 点 ｛ 整数 ｘ； 整数 ｙ； ｝` with construction (`点 ｛ ｘ：１、ｙ：２ ｝`), field
access (`ｐ。ｘ`), and field assignment. Requires: a new `HikariType::Record(name)`,
a registry of declared types in the type checker, `Value::Record(Rc<RefCell<...>>)`
in the VM, and `GetField`/`SetField`/`MakeRecord` instructions.

**9b. 列挙型とパターン照合（Enums & Pattern Matching）**
Sum types (`列挙 結果 ｛ 成功（整数）、失敗（文字列）｝`) plus a `照合`/match statement.
This subsumes a lot: it gives a principled way to model absence/optionality and
errors as values, complementing try/catch.

**9c. 連想配列（Maps / Dictionaries）**
`辞書＜文字列、整数＞` with literal, lookup, insert, and `鍵一覧`/`値一覧`. Backed by
a `Value::Map`. Hugely useful for real programs (counting, grouping, caching).

---

## フェーズ１０ — 第一級関数（First-Class Functions）

**10a. 関数値とラムダ**
Functions as values, a `関数型` type, and anonymous functions, enabling
`地図`/`絞り込み`/`畳み込み` (map/filter/reduce) over arrays as ordinary library
functions instead of language built-ins. Requires closures or at minimum
function-pointer values; closures need captured-environment support in the VM.

**10b. ジェネリクス（Generics）**
Even minimal parametric types (`配列＜Ｔ＞`, generic `要素数`, generic
`地図＜Ｔ、Ｕ＞`) would remove the current need to special-case every builtin's
types by hand in the type checker.

---

## フェーズ１１ — 入出力と実行環境（I/O & Runtime）

**11a. ファイル入出力** — `ファイル読む（パス）`, `ファイル書く（パス、内容）`.
**11b. 書式付き出力** — `印刷` of multiple values / interpolation, and a no-newline
variant.
**11c. プログラム引数と環境** — access to CLI args / env vars from a running program.
**11d. 実行時エラーの位置情報** — runtime errors currently carry no source span (e.g.
division-by-zero doesn't point at a line). Thread spans into the bytecode so runtime
diagnostics match the quality of compile-time ones.

---

## フェーズ１２ — 堅牢性とツール（Robustness & Tooling）

These harden the implementation itself rather than adding language features.

- **任意精度・境界の見直し:** the constant pool is `u16`-indexed, arg counts are
  `u8`, and each frame has a fixed 256 locals — all silently wrap/corrupt at the
  boundary. Replace with checked widening or dynamic sizing.
- **再帰の性能:** `Frame::new` clones the whole chunk's instruction vector on every
  call (`chunk.instructions.clone()`), making recursion O(chunk size) per call.
  Share instructions via `Rc<[Instruction]>` so frames are cheap.
- **未使用変数・到達不能コードの警告:** beginner-friendly lints.
- **REPL のトランザクション性:** a line that type-checks partway then fails currently
  leaves the persistent checker with half-declared state; make per-line evaluation
  all-or-nothing.
- **スタック深度の上限:** a configurable recursion/frame-depth limit with a clean
  `再帰が深すぎます` error instead of unbounded memory growth.
- **テスト:** property-based / fuzz testing of the lexer and parser to catch the next
  class of panics-on-malformed-input before users do.

---

## Suggested ordering

```
Phase 7  (modulo + array/math stdlib)   ← highest value, smallest effort, unblocks real programs
Phase 8  (sound control flow)           ← closes the last soundness gaps; small
Phase 9  (user-defined types)           ← the big capability leap; do records first
Phase 11 (I/O)                          ← can be done any time after Phase 7
Phase 10 (first-class functions)        ← depends on closures; pairs well with 9b/9c
Phase 12 (robustness)                   ← ongoing; pull items forward as needed
```

If only one phase ships next, it should be **Phase 7** — modulo and basic array
operations are the difference between "a demo language" and "a language you can
actually solve beginner exercises in."
