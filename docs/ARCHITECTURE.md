# Hikari Architecture & Internals

This document describes how the Hikari implementation works internally — the
compilation pipeline, the data structures that flow between stages, the bytecode
and the VM that executes it. It complements the [README](../README.md) (which is
the *language* reference) and the [ROADMAP](ROADMAP.md) (which tracks feature
work). It is written for contributors who want to change the compiler or VM.

Everything here is grounded in the current source tree; file/line references are
clickable from an editor that supports them.

---

## 1. The pipeline at a glance

```
Source (.hkr, UTF-8)
   │
   ▼  src/lexer.rs
Lexer ──────────────► Vec<Token>            (each Token carries a Span: line, col, len)
   │
   ▼  src/parser/
Parser (recursive descent) ──► Vec<Stmt>    (AST; statements carry spans; expressions mostly don't — Expr::Call is the exception)
   │
   ▼  src/modules.rs
Import resolution ──► Vec<Stmt>             (file imports flattened; stdlib imports left as markers)
   │
   ├──► src/lints.rs        (non-fatal warnings, computed on the user's pre-import AST)
   │
   ▼  src/typechecker/
Type checker ──► () or TypeError            (scoped static checking; no AST rewrite)
   │
   ▼  src/compiler/
Compiler (codegen) ──► Vec<Instruction> + constant pool + Vec<Chunk>
   │
   ▼  src/vm/
VM (stack machine) ──► Option<Value>        (the program's result, if any)
   │
   ▼  src/diagnostic.rs
Diagnostics ──► Japanese error with a source snippet (compile-time and runtime)
```

The driver that wires these together is [`run_source`](../src/main.rs:133) (for
files / `-c` / stdin) and [`eval_repl_line`](../src/main.rs:241) (for the REPL).

A key design property: **the type checker does not transform the AST**. It only
accepts or rejects. The compiler then re-walks the *same* AST. This means the
compiler assumes well-typedness and uses `expect`/`panic!` at points the checker
has already proven safe (e.g. "this name resolves to a slot"). When changing one
stage, keep this contract in mind — a checker gap becomes a compiler panic.

The one piece of information the checker passes forward is a small **side
channel**, not an AST rewrite: the set of `総和` call sites whose argument is a
`小数列` (keyed by `Expr::Call`'s span), so the compiler can lower an empty
float-sum to `0.0` instead of the integer `0`. The driver hands this set from the
checker to the compiler between the two passes.

---

## 2. Lexer — `src/lexer.rs`

- Input is decoded to a `Vec<char>` up front; the lexer indexes into it. All
  source is full-width (ZenKaku) UTF-8 — there are **no ASCII symbols** in valid
  Hikari, with two deliberate exceptions: the `#!` shebang on line 1
  ([`tokenize`](../src/lexer.rs:250)) and ASCII inside string literals.
- Comments start with the full-width `＃` and run to end of line
  ([`skip_whitespace`](../src/lexer.rs:135)). They are currently **discarded**
  here (not emitted as tokens), which is why `整形` cannot preserve them — see
  [Known Issues #6](KNOWN_ISSUES.md).
- Numbers are read from full-width digits `０-９`; a `．` makes it a float
  ([`read_number`](../src/lexer.rs:177)). An unparseable/overflowing literal
  becomes `TokenKind::Invalid(text)` rather than panicking — the parser turns
  that into a clean `ParseError::InvalidNumber`.
- Identifiers vs keywords are resolved in [`keyword_or_ident`](../src/lexer.rs:206).
  A word boundary is any whitespace or a symbol per [`is_symbol`](../src/lexer.rs:425).
  Note `ー` (chōonpu / minus) is intentionally **excluded** from `is_symbol` so
  katakana words like `エラー` stay whole; `ー` only becomes `Minus`/`Arrow` when
  it *starts* a token.
- Every token carries a `Span { line, col, len }`. `len` is derived from the
  column delta and is single-line (tokens never span lines).

---

## 3. Parser — `src/parser/`

A hand-written recursive-descent parser with precedence climbing. Entry point is
[`Parser::parse`](../src/parser/parse.rs:72). The precedence ladder (lowest →
highest):

```
parse_or          または            (left-assoc, short-circuit)
parse_and         かつ              (left-assoc, short-circuit)
parse_comparison  ＝＝ ≠ ＜ ＞ ≦ ≧   (NON-associative — chaining is a parse error)
parse_additive    ＋ ー             (left-assoc)
parse_multiplicative ＊ ／ ％        (left-assoc)
parse_primary     literals, calls, unary ー/否定, lambdas, ( ), [ ], { },
                  postfix index 【…】 and field access ：：
```

Statement dispatch is in [`parse_stmt_inner`](../src/parser/parse.rs:102). A few
disambiguations worth knowing when editing:

- `関数` is a **var-decl of function type** when followed by `＜`
  (`関数＜…＞ f ＝ …`), otherwise a **named function declaration**.
- A bare `Ident Ident` at statement start is the unique shape of a
  **record-typed var-decl** (`型名 変数名 ＝ …`).
- In `parse_primary`, `Ident （` is a **call**, and `Ident ｛ field：…`
  is a **record literal**; a bare `｛` is always a **map literal**.

**Stack-overflow guard.** Every statement and expression entry is wrapped in
[`with_depth`](../src/parser/parse.rs:82), which rejects input nested deeper than
`MAX_DEPTH = 32` with `ParseError::TooDeeplyNested`. This was added after the
fuzz harness found that tens of thousands of `（` could overflow the parser's own
call stack. Keep new recursive parse entry points inside `with_depth`.

The AST types live in [`src/parser/ast.rs`](../src/parser/ast.rs): `HikariType`,
`Expr`, `Stmt`, `MatchArm`, `BinOpKind`. **Statements carry a `Span`; most
expressions do not** — `Expr::Call` is the lone exception (its span is the callee
name's location, used for call-site diagnostics and for keying the float-`総和`
lowering). Runtime/diagnostic granularity is therefore still statement-level.

---

## 4. Module resolution — `src/modules.rs`

[`resolve_imports`](../src/modules.rs:27) runs between parsing and type checking:

- A `取り込む 「数学」；` whose name is a known stdlib module
  (`STDLIB_MODULES`) is **left in place** as a marker the type checker reads to
  gate builtins.
- Any other name is treated as a **relative `.hkr` path**: the file is read,
  lexed, parsed, and recursively resolved. Cycles are deduplicated via a
  `visited` set of canonical paths (not an error).
- An imported file's top-level `関数`, `取り込む`, `型`, and `構造` are spliced
  in (so a library file is self-contained — its own stdlib imports and type
  declarations come along). A **flat** `取り込む 「lib.hkr」；` merges names as-is;
  a **namespaced** `取り込む 「lib.hkr」 として エイリアス；` mangles every name to
  `エイリアス。name` (see [`mangle_module`](../src/modules.rs:119)), and `公開`
  marks which functions are callable across the module boundary.

Imports are resolved at the top level only (not inside `もし`/`関数`/etc.).

---

## 5. Type system — `src/typechecker/`

The checker is a `TypeChecker` ([checker.rs](../src/typechecker/checker.rs:13))
holding the symbol tables: a scope stack (`Vec<HashMap<String, HikariType>>`),
function signatures (`fns`), declared `records`/`enums`, a `variant_owner` index
(variant name → enum, variants are globally unique), the set of
`imported_modules`, the `current_return_ty`, and a `loop_depth` counter for
break/continue validation.

Layout:

| File | Responsibility |
|------|----------------|
| [`checker.rs`](../src/typechecker/checker.rs) | Statement checking, scope/return/loop bookkeeping, `infer_value_expr` (rejects `無` in value position) |
| [`exprs.rs`](../src/typechecker/exprs.rs) | `infer_expr` — expression type inference, builtin/call resolution |
| [`generics.rs`](../src/typechecker/generics.rs) | Parametric signatures + unifier for the polymorphic builtins |
| [`symbols.rs`](../src/typechecker/symbols.rs) | `builtin_sig`, `builtin_module` (gating), `always_returns` |
| [`error.rs`](../src/typechecker/error.rs) | `TypeError` variants + Japanese `Display` |

### Call-resolution order (`infer_expr`, `Expr::Call`)

The order in [exprs.rs](../src/typechecker/exprs.rs:190) matters — it's the
precedence by which a name is interpreted:

1. **Enum variant constructor** (`variant_owner`) → returns `Record(enum_name)`.
2. **Module gating** — if the name is a gated builtin (`builtin_module`) whose
   module isn't imported, error early. (Note: `含む` and `文字列化` are *not* in
   `builtin_module`; their gating/overloads are handled inline below.)
3. **Generic builtin** (`generic_builtin_sig`) → unify args, instantiate result.
4. **Hand-checked builtins** with non-parametric constraints: math numerics
   (`絶対値`/`平方根`/`最大`/`最小`/`累乗`/`余り`), float→int rounding,
   `整列` (orderable), the `含む` String-vs-Map overload, `文字列化` (union).
5. **Monomorphic builtins** (`builtin_sig`).
6. **Fn-typed local variable** called as a function.
7. **User-declared function** (`fns`).

### Generics — how the unifier works

`generic_builtin_sig` returns a `GenericSig { params, ret }` written with
`SigType` (concrete types or `Var(n)` type variables). `unify` binds variables
against actual argument types into a `subst: HashMap<u8, HikariType>`; `instantiate`
resolves a `SigType` back to a concrete `HikariType` using that substitution.
This is what makes `要素数`/`マップ`/`畳み込み`/etc. work over any element type
without per-builtin code. Unbound variables instantiate to `整数` (only affects
the "expected type" shown in an error message).

**User-written generics** (`関数＜Ｔ＞ …`, `関数＜Ｔ、Ｕ＞ …`) are supported: the
parser reads a `＜…＞` type-variable list on `関数`, the checker registers each
name as a scoped type variable, and at each call site the substitution is
inferred from the argument types via the same `HikariType` unifier
([generics.rs](../src/typechecker/generics.rs)) and the return type instantiated.
This is **checker-only** — the VM is type-erased, so one shared chunk per generic
function suffices (no monomorphization). Generic *records/enums* are not yet
supported (see the roadmap).

### Exhaustive-return analysis

[`always_returns`](../src/typechecker/symbols.rs:206) decides whether a non-`無`
function body is guaranteed to return on every path. It is **conservative**: only
the last statement matters, and loops never count (they may run zero times).
`もし` counts only with an `else` where both branches return; `試す/失敗` only when
both bodies return; and an **exhaustive `照合`** counts when every arm returns
(exhaustiveness is already proven by the checker, so this is sound).

---

## 6. Bytecode & compiler — `src/compiler/`

### Values and the constant pool

[`Value`](../src/compiler/value.rs) is shared by the constant pool and the VM
stack:

- `Int(i64)`, `Float(f64)`, `Str(String)`, `Bool(bool)` — value types.
- `Array`, `Record`, `Map` — all `Rc<RefCell<…>>`, giving **reference
  semantics** (aliasing + in-place mutation visible through aliases).
- `Enum { enum_name, variant, payload }` — plain by-value (no mutation op).
- `Function { chunk_index, arity, captured }` — first-class functions; `captured`
  holds closed-over values (capture-by-value).

### Instruction set

[`Instruction`](../src/compiler/bytecode.rs:10) is a flat enum. Jumps use
**absolute** offsets within a chunk. Highlights: arithmetic/compare ops pop two
and push one; `JumpIfFalse/JumpIfTrue/Jump` for control flow; `Call(fn_idx,
argc)` and `CallValue(argc)` for direct vs value calls; `CallBuiltin(fn, argc)`;
`MakeArray/MakeMap/MakeRecord/MakeEnum/MakeClosure`; field/index/payload
accessors; `TryStart/TryEnd` for exception scopes.

### Chunks

A [`Chunk`](../src/compiler/bytecode.rs:77) is one compiled function:
`instructions: Rc<[Instruction]>`, `param_count`, and `spans: Rc<[(usize,
Span)]>`. The `Rc<[…]>` slices let a call `Frame` share a body with an O(1)
refcount bump instead of cloning it per call (so recursion is O(depth), not
O(depth × body size)). `chunks[0]` is always the top-level script.

### Codegen — `src/compiler/codegen.rs`

The [`Compiler`](../src/compiler/codegen.rs:19) compiles in two passes:

1. **Register function names** so forward calls resolve (reserve a chunk slot
   per `関数`).
2. **Compile bodies and the top-level script** into instructions.

Slot allocation is in [`Scopes`](../src/compiler/codegen.rs:63): `next_slot`
advances as bindings are declared, and a `watermarks` stack saved on `enter` /
restored on `exit` lets **sibling block scopes reuse the same slots** (phase 20c
— shrinks frame size). Shadowing is still safe: a same-scope re-declaration
reuses its slot, but a name that exists only in an *outer* scope gets a fresh
slot, so the new binding never corrupts the outer one. The VM grows a frame's
locals vector on demand, so there is no hard slot ceiling.

Notable lowering details:

- **Short-circuit `かつ`/`または`** are compiled with jumps, not as plain binops.
- **Loops** back-patch jump targets. `続ける` in `繰り返す`/`各` must land on the
  increment step (compiled *after* the body), so those continues are deferred and
  back-patched (`ContinueTarget::Deferred`); `間`'s continue target is known up
  front (`ContinueTarget::Known`). See the comment block at
  [codegen.rs:42](../src/compiler/codegen.rs:42).
- **`照合`** lowers to a chain of `TagEquals` + `JumpIfFalse`, reloading the
  subject from a local before each arm and extracting payloads with `GetPayload`.
- **Closures**: a free-variable analysis ([`free_vars`](../src/compiler/codegen.rs:872))
  finds enclosing locals a lambda references, pushes their current values, and
  `MakeClosure` bundles them; captures are seeded into the callee's locals right
  after the params, so the body reads them as ordinary `LoadLocal`s.

**Boundary hardening.** Bytecode fields are fixed width (`u16` for
constant/jump/chunk indices, `u8` for arg/payload/capture counts). `compile`
returns `Result<_, CompileError>`: `u8` sites are checked inline via
[`count_u8`](../src/compiler/codegen.rs:146) and `u16` structural limits are a
post-pass. Exceeding any limit yields "プログラムが大きすぎます" instead of a silent
wrap (unreachable for hand-written programs; guards against corruption).

---

## 7. The VM — `src/vm/`

| File | Responsibility |
|------|----------------|
| [`machine.rs`](../src/vm/machine.rs) | The `Vm`, the `step` dispatch loop, `run`/`run_repl_line`, HOF execution |
| [`frame.rs`](../src/vm/frame.rs) | `Frame` (ip, locals, shared instructions/spans), `TryHandler` |
| [`builtins.rs`](../src/vm/builtins.rs) | `call_builtin` — the non-HOF builtin implementations |
| [`value_ops.rs`](../src/vm/value_ops.rs) | `display_value`, comparisons, arithmetic helpers, sort, conversions |
| [`error.rs`](../src/vm/error.rs) | `RuntimeError` variants + Japanese `Display` |

### Execution model

The `Vm` holds the constant pool, all chunks, an operand `stack`, a `frames`
stack, a `try_stack`, the last error span, and the program args. [`step`](../src/vm/machine.rs:100)
fetches one instruction, increments `ip`, and dispatches. Falling off the end of
a chunk is a void return (pop the frame; if it was frame 0, halt).

- **Calls** push a `Frame` (seeded with args, then captures for closures) via
  [`push_frame`](../src/vm/machine.rs:789), which enforces
  `MAX_FRAME_DEPTH = 1024` → catchable `RuntimeError::StackOverflow`
  (`再帰が深すぎます`).
- **HOFs** (`マップ`/`絞り込み`/`畳み込み`/`どれか`/`すべて`/`数える`) and `引数`
  are handled directly in `step` (not `call_builtin`) because they need the frame
  machinery / VM state. They drive a callee to completion synchronously via
  [`call_function`](../src/vm/machine.rs:807).

### Error handling / try-catch

`TryStart` pushes a `TryHandler` recording the catch target, error slot, and the
stack length + frame depth at entry. On any `RuntimeError`, [`run`](../src/vm/machine.rs:675)
pops the nearest handler, **truncates frames first, then the stack** (order
matters), binds the error message string into the error slot, and jumps to the
catch target. With no handler, it records the failing instruction's span
([`current_error_span`](../src/vm/machine.rs:776)) and returns the error.

### Arithmetic & safety

Integer arithmetic is **checked** ([`arith`](../src/vm/value_ops.rs:156)):
overflow → `IntegerOverflow`. Division and modulo reject a zero divisor with
`DivisionByZero` for **both** `整数` and `小数` (`checked_div` also covers
`i64::MIN / -1`). The lone exception is **float modulo**: `a ％ 0.0` yields `NaN`
(IEEE) rather than erroring. `＋` is overloaded for string concatenation. The
non-HOF math builtins (`絶対値`/`累乗`/`余り`/…) likewise use checked arithmetic,
so an overflow surfaces as a catchable error rather than a panic.

### REPL specifics

[`run_repl_line`](../src/vm/machine.rs:703) appends a line's instructions to
frame 0 (rebuilding its `Rc` slice), shifts span checkpoints to absolute indices,
and runs from the append point. On an uncaught error it resets transient state
(drops in-progress frames, clears stack & try handlers, parks frame 0) while
keeping frame 0's persistent locals. The driver additionally snapshots the
`TypeChecker` and `Compiler` (both `Clone`) and rolls them back on any failure so
a bad line leaves no half-declared state.

---

## 8. Diagnostics — `src/diagnostic.rs`

`render(source, span, message)` and `render_warning(...)` produce the
`--> line:col` + source-snippet + caret format shared by compile-time and runtime
errors. Runtime errors gained source spans via the per-chunk span checkpoints, so
a division-by-zero deep in a function points at the failing statement, not the
call site. Granularity is statement-level: the span checkpoints are emitted
per statement, so even though `Expr::Call` now carries a span, sub-expression
precision (e.g. pointing at one operand) would still require spans on the rest of
the `Expr` variants — see roadmap 19a.

---

## 9. Testing

- Unit/integration tests live next to each module (`*/tests.rs`,
  `*/tests/*.rs`); ~583 tests cover lexer, parser, type checker, compiler, and VM
  behavior end to end.
- [`src/fuzz_tests.rs`](../src/fuzz_tests.rs) is a seeded, dependency-free
  property/fuzz harness driving ~25k random + hand-picked malformed inputs
  through lexer → parser → checker → compiler, asserting no panics.

```sh
cargo test          # everything
cargo fmt           # before committing
cargo check
```

---

## 10. Map of the source tree

```
src/
  main.rs              CLI entry, driver (run_source / REPL loop)
  diagnostic.rs        Japanese error rendering
  modules.rs           import resolution
  lints.rs             non-fatal lint pass
  fuzz_tests.rs        property/fuzz harness
  lexer.rs             tokenizer
  parser/
    ast.rs             HikariType, Expr, Stmt, MatchArm, BinOpKind
    parse.rs           recursive-descent parser
    display.rs         AST Display (debugging)
    error.rs           ParseError
  typechecker/
    checker.rs         statements, scopes, returns, loops
    exprs.rs           expression inference, call resolution
    generics.rs        parametric builtin signatures + unifier
    symbols.rs         builtin_sig, builtin_module, always_returns
    error.rs           TypeError
  compiler/
    codegen.rs         AST → bytecode
    bytecode.rs        Instruction, Chunk
    value.rs           Value
    builtins.rs        BuiltinFn enum + name mapping
    error.rs           CompileError
  vm/
    machine.rs         Vm, step loop, run, HOFs
    frame.rs           Frame, TryHandler
    builtins.rs        call_builtin
    value_ops.rs       display, comparisons, arithmetic, sort
    error.rs           RuntimeError
```
