# Hikari Known Issues

This document records bugs and limitations found in comprehensive code reviews
of the implementation. Each entry has a severity, a reproduction (where
practical), the root cause with a file reference, and a suggested fix. Items are
ordered by impact.

Status legend: 🔴 open bug · 🟡 latent / hard-to-trigger · 🔵 known limitation
(by design today, but surprising) · ✅ resolved (kept for history).

**Current status:** issues #1–#5 (the first review) are all **resolved** — they
map to roadmap Phase 14/18 and are verified in the code. Issue **#6** (formatter
deletes comments) is the only one still **open**; its fix is planned (see roadmap
Phase 21a). A separate second review also fixed three runtime bugs not listed
here (`余り`/`経過` overflow panics and the empty-`総和` float-zero soundness hole,
merged in PR #49).

---

## 1. ✅ Exhaustive `照合` is not recognized as a returning path

**Status:** ✅ Resolved — [`always_returns`](../src/typechecker/symbols.rs:226)
now has the `Stmt::Match` arm. **Severity:** was High (sound but spurious
rejection of valid programs).

A non-`無` function whose last statement is an **exhaustive** `照合` where every
arm returns is wrongly rejected with `MissingReturn`:

```
構造 色 ｛ 赤、 青 ｝
関数 番号（色 ｃ）ー＞整数｛
  照合 ｃ ｛
    赤（）ならば ｛ 返す １； ｝
    青（）ならば ｛ 返す ２； ｝
  ｝
｝
```
```
エラー: 関数「番号」のすべての実行経路が値を返すとは限りません。
```

**Root cause.** [`always_returns`](../src/typechecker/symbols.rs:100) handles
`返す`, `もし/違えば`, and `試す/失敗`, but has no arm for `Stmt::Match`, so it
falls through to `false`.

**Why the fix is safe.** Match exhaustiveness is *already* proven by the type
checker ([checker.rs](../src/typechecker/checker.rs:580)) before this runs. So a
match whose every arm `always_returns` genuinely returns on every path.

**Suggested fix.** Add to `always_returns`:

```rust
Some(Stmt::Match { arms, .. }) => arms.iter().all(|a| always_returns(&a.body)),
```

The workaround today is a dead `返す` after the match.

---

## 2. ✅ Imported files lose their own imports and type declarations

**Status:** ✅ Resolved — [`resolve_imports`](../src/modules.rs:68) now splices an
imported file's top-level `取り込む`/`型`/`構造` alongside its `関数` (and the
namespaced-import path in Phase 18 mangles them per module). **Severity:** was
Medium (broke encapsulation of library files).

A function imported from another `.hkr` file fails to typecheck if it uses a
gated stdlib builtin (or a record/enum) that the *library* file imported/declared:

```
# lib.hkr
取り込む 「数学」；
関数 二乗（整数 ｎ）ー＞整数｛ 返す 累乗（ｎ、 ２）； ｝

# main.hkr
取り込む 「lib.hkr」；
印刷（二乗（４））；
```
```
エラー: 「累乗」を使うには「取り込む 「数学」；」が必要です。
  --> 2:18   ← points at the call site in main.hkr, not the real usage in lib.hkr
```

**Root cause.** [`resolve_imports`](../src/modules.rs:64) splices **only
`FnDecl`** statements from an imported file; its `取り込む`, `型`, and `構造`
statements are discarded. The misleading error location compounds it (the spliced
function body is rendered against `main.hkr`'s source text).

**Impact.** The importer must re-import every stdlib module its dependencies use,
and imported functions cannot reference types declared in their own file.

**Suggested fixes (pick per design intent):**
- Splice imported `Import`/`TypeDecl`/`EnumDecl` (top-level) as well, so library
  files are self-contained; or
- At minimum, document the limitation and emit a clearer diagnostic that names
  the imported file, rather than pointing into `main.hkr`.

This is currently *partly* by design (README says "only top-level `関数`
declarations are merged"), but the gated-builtin failure is surprising enough to
treat as a bug.

---

## 3. ✅ `小数` whole numbers print identically to `整数`

**Status:** ✅ Resolved — [`display_value`](../src/vm/value_ops.rs:5) renders an
integral finite `小数` as `１．０` (`format!("{f:.1}")`). **Severity:** was Low
(display only; misleading output).

```
印刷（１．０）；        ＃ prints "1"
印刷（１．０ ＋ ２．０）；  ＃ prints "3"
```

Although `整数` and `小数` are distinct types, an integral float is
indistinguishable from an integer in output.

**Root cause.** [`display_value`](../src/vm/value_ops.rs:8) uses
`f.to_string()`, and Rust formats `1.0_f64` as `"1"`.

**Suggested fix.** Append `.0` for finite floats with no fractional part:

```rust
Value::Float(f) => {
    if f.is_finite() && f.fract() == 0.0 { format!("{f:.1}") } else { f.to_string() }
}
```

(Choose a convention for `NaN`/`inf` display at the same time.)

---

## 4. ✅ `絶対値` of `i64::MIN` wraps instead of erroring

**Status:** ✅ Resolved — [`BuiltinFn::Abs`](../src/vm/builtins.rs:75) now uses
`checked_abs → IntegerOverflow`. **Severity:** was Latent (hard to trigger;
inconsistent with the rest of arithmetic).

[`call_builtin` / `BuiltinFn::Abs`](../src/vm/builtins.rs:75) uses
`n.wrapping_abs()`, which returns a **negative** `i64::MIN` for `i64::MIN` rather
than raising `IntegerOverflow`. Every other integer operation in the VM uses
checked arithmetic, so this is the odd one out. It is hard to reach because the
lexer rejects the `i64::MIN` literal (its magnitude overflows `i64`) and any
arithmetic that would produce it overflows first — but it is reachable in
principle.

**Suggested fix.** `n.checked_abs().map(Value::Int).ok_or(RuntimeError::IntegerOverflow)`.

---

## 5. ✅ Mixed-type sort comparators silently fall back to "equal"

**Status:** ✅ Resolved — [`sort_values`](../src/vm/value_ops.rs:148) and
[`MapKeys`](../src/vm/builtins.rs:288) now use `unreachable!` for the mixed-type
case (the remaining `partial_cmp` fallback is the legitimate float-vs-`NaN`
case, not a type mismatch). **Severity:** was Latent (unreachable; defensive
concern only).

[`sort_values`](../src/vm/value_ops.rs:130) and the key sort in
[`BuiltinFn::MapKeys`](../src/vm/builtins.rs:278) use comparators with a
`_ => Ordering::Equal` fallback. If an array ever contained mixed `整数`/`小数`
(or other mixed types), the sort would silently produce a wrong (unstable)
ordering instead of erroring.

This cannot happen today: the type checker guarantees arrays are homogeneous, and
`整列` accepts only `整数`/`小数`/`文字列` element types. The risk is purely that a
future change to those invariants would surface as a silent mis-sort rather than
a loud failure.

**Suggested fix.** Make the fallback `unreachable!()` (documenting the invariant)
or return a `TypeMismatch`, rather than `Equal`.

---

## 6. 🔴 The formatter (`整形`) silently deletes all comments

**Severity:** High (data loss — `整形 -i` permanently destroys comments in place).

```
＃ This is an important comment
整数 ｘ ＝ ４２；   ＃ inline comment
印刷（ｘ）；
```
after `hikari 整形` (or, destructively, `hikari 整形 -i`) becomes:
```
整数 ｘ ＝ ４２；
印刷（ｘ）；
```

Both the standalone and the trailing comment are gone. Blank lines between
statements are dropped the same way.

**Root cause.** Comments are consumed and discarded in
[`Lexer::skip_whitespace`](../src/lexer.rs:135): the `＃` branch scans to
end-of-line and throws the text away, so comments never become tokens and are
absent from the AST. The formatter ([formatter.rs](../src/formatter.rs)) renders
from the AST, so it has nothing to emit. It is not a formatter-logic error — the
information is dropped two layers earlier.

**Planned fix — comment- and blank-line-preserving formatter.**

Design: keep a *side channel* of comments so the parser and token stream stay
untouched (the parser keeps receiving a clean, comment-free stream), then
interleave comments into the formatter's output by source position. The `整形`
path ([main.rs](../src/main.rs:60)) only does parse → format on a single file
(no import resolution, no typecheck), so comment line/col map directly onto the
file being formatted, and every `Stmt` already carries a `Span` to anchor against.

1. **`src/lexer.rs`** — add `Comment { line, col, text }` and a
   `comments: Vec<Comment>` field on `Lexer`, populated in `skip_whitespace`'s
   `＃` branch instead of discarding. Add `into_comments()`. `tokenize()`'s
   signature is unchanged, so the parser and every other caller are untouched.
2. **`src/formatter.rs`** — add `format_stmts_with_comments(stmts, comments)`;
   keep `format_stmts` as a thin wrapper passing `&[]`. Thread a
   `Formatter { comments, next, out }` cursor through `format_stmt` /
   `format_match_arm`. At each statement-emitting level (top level and every
   block body): flush leading comments with `line < stmt.start_line` (own line,
   current indent); splice trailing comments with `line == stmt.start_line`
   before the line's `\n` (attached to the header line for block statements);
   and emit a single blank line where consecutive statements have a source-line
   gap. Flush any remaining comments at the end (trailing file comments).
3. **`src/main.rs`** — in the `整形` branch, keep the lexer alive and pass
   `lexer.into_comments()` into the new formatter entry point.

Scope decisions:
- **Preserved:** own-line comments (correct indent/order), trailing comments on a
  statement's source line, and blank lines between statements.
- **Not preserved (documented limitation):** comments embedded *inside* an
  expression or argument list are relocated to the nearest statement boundary.
  Full in-place fidelity there would require attaching comments to individual AST
  nodes (parser changes); deliberately out of scope.

This also removes the `整形 -i` data-loss as a side effect. Until it lands, treat
`整形 -i` on comment-bearing files as destructive.

---

## Verified correct (checked during the review)

These were examined and behave correctly — listed so future reviewers don't
re-investigate:

- Integer overflow, division/modulo by zero, and `i64::MIN / -1` are all checked
  and produce catchable runtime errors.
- Recursion is bounded by `MAX_FRAME_DEPTH` (catchable `StackOverflow`); the
  parser is bounded by `MAX_DEPTH` (`TooDeeplyNested`).
- Errors raised inside HOF callbacks (`マップ`/`絞り込み`/`畳み込み`) propagate to
  and are caught by an enclosing `試す/失敗`.
- Closures capture primitives by value (snapshot) and reference types by shared
  `Rc`; nested lambdas compose.
- REPL transactionality: a line failing at parse/type/compile/runtime leaves no
  half-applied checker, compiler, or VM state.
- The `含む` String-vs-Map overload is not falsely gated through
  `builtin_module` (its gating is handled inline).
