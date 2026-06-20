# Hikari Known Issues

This document records bugs and limitations found in a comprehensive code review
of the implementation. Each entry has a severity, a reproduction (where
practical), the root cause with a file reference, and a suggested fix. Items are
ordered by impact.

Status legend: 🔴 open bug · 🟡 latent / hard-to-trigger · 🔵 known limitation
(by design today, but surprising)

---

## 1. 🔴 Exhaustive `照合` is not recognized as a returning path

**Severity:** High (sound but spurious rejection of valid programs).

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

## 2. 🔵 Imported files lose their own imports and type declarations

**Severity:** Medium (breaks encapsulation of library files).

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

## 3. 🔵 `小数` whole numbers print identically to `整数`

**Severity:** Low (display only; misleading output).

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

## 4. 🟡 `絶対値` of `i64::MIN` wraps instead of erroring

**Severity:** Latent (hard to trigger; inconsistent with the rest of arithmetic).

[`call_builtin` / `BuiltinFn::Abs`](../src/vm/builtins.rs:75) uses
`n.wrapping_abs()`, which returns a **negative** `i64::MIN` for `i64::MIN` rather
than raising `IntegerOverflow`. Every other integer operation in the VM uses
checked arithmetic, so this is the odd one out. It is hard to reach because the
lexer rejects the `i64::MIN` literal (its magnitude overflows `i64`) and any
arithmetic that would produce it overflows first — but it is reachable in
principle.

**Suggested fix.** `n.checked_abs().map(Value::Int).ok_or(RuntimeError::IntegerOverflow)`.

---

## 5. 🟡 Mixed-type sort comparators silently fall back to "equal"

**Severity:** Latent (currently unreachable; defensive concern only).

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
