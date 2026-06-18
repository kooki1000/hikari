use super::*;

#[test]
fn test_vm_supports_more_than_initial_locals() {
    // A script that declares more local slots than INITIAL_LOCALS must
    // grow the frame's slot vector on demand instead of panicking.
    let count = INITIAL_LOCALS + 50;
    let mut src = String::new();
    for i in 0..count {
        src.push_str(&format!("整数 ｖ{} ＝ ０；", i));
    }
    src.push_str(&format!("返す ｖ{}；", count - 1));
    assert_eq!(run(&src), Some(Value::Int(0)));
}

#[test]
fn test_vm_push_constant() {
    let constants = vec![Value::Int(42)];
    let instructions = vec![Instruction::LoadConst(0), Instruction::Return];
    let result = Vm::new(constants, instructions).run().unwrap();
    assert_eq!(result, Some(Value::Int(42)));
}

#[test]
fn test_vm_store_and_load_local() {
    let result = run("整数 年齢 ＝ ２０；返す 年齢；");
    assert_eq!(result, Some(Value::Int(20)));
}

#[test]
fn test_vm_addition() {
    let result = run("整数 結果 ＝ ３ ＋ ４；返す 結果；");
    assert_eq!(result, Some(Value::Int(7)));
}

#[test]
fn test_vm_operator_precedence() {
    let result = run("整数 結果 ＝ ２ ＋ ３ ＊ ４；返す 結果；");
    assert_eq!(result, Some(Value::Int(14)));
}

#[test]
fn test_vm_function_body_via_call() {
    // 関数 加算一（整数 Ａ）ー＞ 整数 ｛ 返す Ａ ＋ １； ｝
    // 返す 加算一（９）；  →  10
    let src = "関数 加算一（整数 Ａ）ー＞ 整数 ｛ 返す Ａ ＋ １； ｝返す 加算一（９）；";
    assert_eq!(run(src), Some(Value::Int(10)));
}

#[test]
fn test_vm_call_function() {
    // 関数 二倍（整数 Ａ）ー＞ 整数 ｛ 返す Ａ ＊ ２； ｝
    // 返す 二倍（５）；  →  10
    let src = "関数 二倍（整数 Ａ）ー＞ 整数 ｛ 返す Ａ ＊ ２； ｝返す 二倍（５）；";
    assert_eq!(run(src), Some(Value::Int(10)));
}

#[test]
fn test_vm_print_integer() {
    // 印刷（４２）；  — should not panic and produces no return value
    let result = run("印刷（４２）；");
    assert_eq!(result, None);
}

#[test]
fn test_vm_print_variable() {
    // 整数 Ａ ＝ ７；  印刷（Ａ）；
    let result = run("整数 Ａ ＝ ７；印刷（Ａ）；");
    assert_eq!(result, None);
}

#[test]
fn test_vm_print_then_return() {
    // 印刷（１）；  返す ２；  — prints 1, returns 2
    let result = run("印刷（１）；返す ２；");
    assert_eq!(result, Some(Value::Int(2)));
}

#[test]
fn test_vm_bool_literal_as_if_condition() {
    // 真偽 フラグ ＝ 真；もし フラグ ならば ｛ 返す １； ｝ 違えば ｛ 返す ０； ｝
    let result =
        run("真偽 フラグ ＝ 真；もし フラグ ならば ｛ 返す １； ｝ 違えば ｛ 返す ０； ｝");
    assert_eq!(result, Some(Value::Int(1)));
}

#[test]
fn test_vm_if_true_branch() {
    // もし １ ＝＝ １ ならば ｛ 返す １０； ｝
    // Condition is true → returns 10
    let result = run("もし １ ＝＝ １ ならば ｛ 返す １０； ｝");
    assert_eq!(result, Some(Value::Int(10)));
}

#[test]
fn test_vm_if_false_branch_skipped() {
    // もし １ ＝＝ ２ ならば ｛ 返す １０； ｝ 返す ０；
    // Condition is false → skips then_body, returns 0
    let result = run("もし １ ＝＝ ２ ならば ｛ 返す １０； ｝返す ０；");
    assert_eq!(result, Some(Value::Int(0)));
}

#[test]
fn test_vm_if_else() {
    // もし １ ＝＝ ２ ならば ｛ 返す １； ｝ 違えば ｛ 返す ２； ｝
    // Condition false → else branch → returns 2
    let result = run("もし １ ＝＝ ２ ならば ｛ 返す １； ｝ 違えば ｛ 返す ２； ｝");
    assert_eq!(result, Some(Value::Int(2)));
}

#[test]
fn test_vm_comparison_lt_gt() {
    // 整数 Ａ ＝ ３；  もし Ａ ＜ ５ ならば ｛ 返す １； ｝ 違えば ｛ 返す ０； ｝
    let result = run("整数 Ａ ＝ ３；もし Ａ ＜ ５ ならば ｛ 返す １； ｝ 違えば ｛ 返す ０； ｝");
    assert_eq!(result, Some(Value::Int(1)));
}

#[test]
fn test_vm_call_with_expression_arg() {
    // 関数 二倍（整数 Ａ）ー＞ 整数 ｛ 返す Ａ ＊ ２； ｝
    // 返す 二倍（３ ＋ ４）；  →  14
    let src = "関数 二倍（整数 Ａ）ー＞ 整数 ｛ 返す Ａ ＊ ２； ｝返す 二倍（３ ＋ ４）；";
    assert_eq!(run(src), Some(Value::Int(14)));
}

#[test]
fn test_vm_division_by_zero_returns_error() {
    // 整数 結果 ＝ １ ／ ０；
    let ast = Parser::new(Lexer::new("整数 結果 ＝ １ ／ ０；").tokenize())
        .parse()
        .unwrap();
    let mut compiler = Compiler::new();
    let script = compiler.compile(&ast).unwrap();
    let result = Vm::with_chunks(compiler.constants, compiler.chunks, script).run();
    assert_eq!(result, Err(RuntimeError::DivisionByZero));
}

#[test]
fn test_vm_float_division_by_zero_returns_error() {
    let ast = Parser::new(Lexer::new("小数 結果 ＝ １．０ ／ ０．０；").tokenize())
        .parse()
        .unwrap();
    let mut compiler = Compiler::new();
    let script = compiler.compile(&ast).unwrap();
    let result = Vm::with_chunks(compiler.constants, compiler.chunks, script).run();
    assert_eq!(result, Err(RuntimeError::DivisionByZero));
}

#[test]
fn test_vm_reassignment() {
    // 整数 年齢 ＝ ２０；年齢 ＝ ３０；返す 年齢；
    let result = run("整数 年齢 ＝ ２０；年齢 ＝ ３０；返す 年齢；");
    assert_eq!(result, Some(Value::Int(30)));
}

#[test]
fn test_vm_multi_param_call() {
    // 関数 加算（整数 Ａ、整数 Ｂ）ー＞ 整数 ｛ 返す Ａ ＋ Ｂ； ｝
    // 返す 加算（３、４）；  →  7
    let src = "関数 加算（整数 Ａ、整数 Ｂ）ー＞ 整数 ｛ 返す Ａ ＋ Ｂ； ｝返す 加算（３、４）；";
    assert_eq!(run(src), Some(Value::Int(7)));
}

#[test]
fn test_vm_unary_minus() {
    let result = run("整数 結果 ＝ ー５；返す 結果；");
    assert_eq!(result, Some(Value::Int(-5)));
}

#[test]
fn test_vm_unary_minus_in_expression() {
    let result = run("整数 結果 ＝ １０ ＋ ー３；返す 結果；");
    assert_eq!(result, Some(Value::Int(7)));
}

#[test]
fn test_vm_logical_and() {
    assert_eq!(run("返す 真 かつ 偽；"), Some(Value::Bool(false)));
    assert_eq!(run("返す 真 かつ 真；"), Some(Value::Bool(true)));
}

#[test]
fn test_vm_logical_or() {
    assert_eq!(run("返す 真 または 偽；"), Some(Value::Bool(true)));
    assert_eq!(run("返す 偽 または 偽；"), Some(Value::Bool(false)));
}

#[test]
fn test_vm_logical_not() {
    assert_eq!(run("返す 否定 真；"), Some(Value::Bool(false)));
    assert_eq!(run("返す 否定 偽；"), Some(Value::Bool(true)));
}

#[test]
fn test_vm_additional_comparison_operators() {
    assert_eq!(run("返す ３ ≦ ３；"), Some(Value::Bool(true)));
    assert_eq!(run("返す ５ ≧ １０；"), Some(Value::Bool(false)));
    assert_eq!(run("返す １ ≠ ２；"), Some(Value::Bool(true)));
}

#[test]
fn test_vm_string_concatenation() {
    let result = run("文字列 結果 ＝ 「あ」 ＋ 「い」；返す 結果；");
    assert_eq!(result, Some(Value::Str("あい".to_string())));
}

#[test]
fn test_vm_builtin_strlen() {
    let result = run("返す 文字数（「こんにちは」）；");
    assert_eq!(result, Some(Value::Int(5)));
}

#[test]
fn test_vm_builtin_parse_int() {
    let result = run("返す 整数化（「４２」）；");
    assert_eq!(result, Some(Value::Int(42)));
}

#[test]
fn test_vm_builtin_parse_float() {
    let result = run("返す 小数化（「３．５」）；");
    assert_eq!(result, Some(Value::Float(3.5)));
}

#[test]
fn test_vm_builtin_to_str_int() {
    let result = run("返す 文字列化（４２）；");
    assert_eq!(result, Some(Value::Str("42".to_string())));
}

#[test]
fn test_vm_builtin_to_str_float() {
    let result = run("返す 文字列化（３．５）；");
    assert_eq!(result, Some(Value::Str("3.5".to_string())));
}

#[test]
fn test_vm_builtin_to_str_bool() {
    let result = run("返す 文字列化（真）；");
    assert_eq!(result, Some(Value::Str("真".to_string())));
}

#[test]
fn test_vm_builtin_parse_int_invalid_returns_error() {
    let ast = Parser::new(Lexer::new("返す 整数化（「abc」）；").tokenize())
        .parse()
        .unwrap();
    let mut compiler = Compiler::new();
    let script = compiler.compile(&ast).unwrap();
    let result = Vm::with_chunks(compiler.constants, compiler.chunks, script).run();
    assert!(matches!(result, Err(RuntimeError::InvalidConversion(_))));
}

#[test]
fn test_vm_uninitialized_local_returns_error() {
    // A hand-built program that reads local slot 0 without ever storing
    // into it first; this can't be produced by the compiler from valid
    // Hikari source (every declared variable is stored immediately), so
    // the instruction stream is constructed directly to exercise the
    // VM's own guard against reading an uninitialized local.
    let instructions = vec![Instruction::LoadLocal(0), Instruction::Return];
    let result = Vm::new(vec![], instructions).run();
    assert_eq!(result, Err(RuntimeError::UninitializedLocal(0)));
}
