use super::*;

#[test]
fn test_vm_try_catch_success_skips_catch_body() {
    let src =
        "整数 結果 ＝ ０；試す ｛ 結果 ＝ １； ｝ 失敗 失敗内容 ｛ 結果 ＝ ２； ｝返す 結果；";
    let result = run(src);
    assert_eq!(result, Some(Value::Int(1)));
}

#[test]
fn test_vm_try_catch_catches_division_by_zero_and_continues() {
    let src = "整数 結果 ＝ ０；試す ｛ 結果 ＝ １ ／ ０； ｝ 失敗 失敗内容 ｛ 結果 ＝ ９； ｝返す 結果；";
    let result = run(src);
    assert_eq!(result, Some(Value::Int(9)));
}

#[test]
fn test_vm_try_catch_binds_error_message() {
    let src = "整数 結果 ＝ ０；試す ｛ 結果 ＝ １ ／ ０； ｝ 失敗 失敗内容 ｛ 結果 ＝ 文字数（失敗内容）； ｝返す 結果；";
    let result = run(src);
    match result {
        Some(Value::Int(n)) => assert!(n > 0),
        other => panic!("expected a non-empty error message length, got {:?}", other),
    }
}

// ── 11d: runtime error source spans ─────────────────────────────────

#[test]
fn test_vm_runtime_error_reports_statement_line() {
    // The division-by-zero is on line 3; the recorded span must point there.
    let src = "整数 ａ ＝ １０；\n整数 ｂ ＝ ０；\n整数 ｃ ＝ ａ ／ ｂ；\n返す ｃ；";
    assert_eq!(run_error_line(src), Some(3));
}

#[test]
fn test_vm_runtime_error_inside_function_points_into_function_body() {
    // The error happens inside 取得's body (line 2), reached via a call on
    // line 5. The span should point at the failing statement, not the call.
    let src = "関数 取得（整数列 ｘｓ）ー＞ 整数 ｛\n返す ｘｓ【９】；\n｝\n整数列 ａ ＝ 【１】；\n返す 取得（ａ）；";
    assert_eq!(run_error_line(src), Some(2));
}

#[test]
fn test_vm_unbounded_recursion_raises_stack_overflow() {
    // Infinite recursion must surface a clean StackOverflow rather than
    // growing the frame vector until the process is killed.
    let src = "関数 ループ（整数 ｎ）ー＞ 整数 ｛ 返す ループ（ｎ ＋ １）； ｝返す ループ（０）；";
    assert_eq!(run_result(src), Err(RuntimeError::StackOverflow));
}

#[test]
fn test_vm_deep_recursion_can_be_caught() {
    // A StackOverflow raised deep in recursion is an ordinary runtime error,
    // so try/catch must be able to recover from it.
    let src = "関数 ループ（整数 ｎ）ー＞ 整数 ｛ 返す ループ（ｎ ＋ １）； ｝整数 結果 ＝ ０；試す ｛ 結果 ＝ ループ（０）； ｝ 失敗 ｅ ｛ 結果 ＝ ９９； ｝返す 結果；";
    assert_eq!(run(src), Some(Value::Int(99)));
}

#[test]
fn test_vm_try_catch_unwinds_nested_function_call() {
    // Error occurs inside a function call made from within try_body, so
    // unwinding must pop the callee's Frame, not just truncate the stack.
    let src = "関数 割る（整数 Ａ、整数 Ｂ）ー＞ 整数 ｛ 返す Ａ ／ Ｂ； ｝整数 結果 ＝ ０；試す ｛ 結果 ＝ 割る（１０、０）； ｝ 失敗 失敗内容 ｛ 結果 ＝ ７； ｝整数 後 ＝ 割る（２０、４）；返す 結果 ＋ 後；";
    let result = run(src);
    // Catch sets 結果＝7; a fresh, unrelated call to 割る after the
    // try/catch must still work correctly, proving frames/stack were
    // left in a valid, non-corrupted state.
    assert_eq!(result, Some(Value::Int(7 + 5)));
}

#[test]
fn test_vm_uncaught_error_still_propagates() {
    let ast = Parser::new(Lexer::new("整数 結果 ＝ １ ／ ０；").tokenize())
        .parse()
        .unwrap();
    let mut compiler = Compiler::new();
    let script = compiler.compile(&ast);
    let result = Vm::with_chunks(compiler.constants, compiler.chunks, script).run();
    assert_eq!(result, Err(RuntimeError::DivisionByZero));
}

#[test]
fn test_vm_nested_try_catch_inner_handler_catches_own_error() {
    // Inner 試す catches its own division-by-zero; the outer handler is
    // never triggered since the inner one already handled it.
    let src = "整数 結果 ＝ ０；試す ｛ 試す ｛ 結果 ＝ １ ／ ０； ｝ 失敗 失敗内容 ｛ 結果 ＝ １； ｝ ｝ 失敗 失敗内容 ｛ 結果 ＝ ２； ｝返す 結果；";
    let result = run(src);
    assert_eq!(result, Some(Value::Int(1)));
}

#[test]
fn test_vm_abs_int_and_float() {
    let result = run("取り込む 「数学」；返す 絶対値（ー５）；");
    assert_eq!(result, Some(Value::Int(5)));

    let result = run("取り込む 「数学」；返す 絶対値（ー５．５）；");
    assert_eq!(result, Some(Value::Float(5.5)));
}

#[test]
fn test_vm_sqrt_of_perfect_square() {
    let result = run("取り込む 「数学」；返す 平方根（９）；");
    assert_eq!(result, Some(Value::Float(3.0)));
}

#[test]
fn test_vm_sqrt_of_negative_returns_error() {
    let ast = Parser::new(
        Lexer::new("取り込む 「数学」；整数 結果 ＝ ー１；返す 平方根（結果）；").tokenize(),
    )
    .parse()
    .unwrap();
    let mut compiler = Compiler::new();
    let script = compiler.compile(&ast);
    let result = Vm::with_chunks(compiler.constants, compiler.chunks, script).run();
    assert!(matches!(result, Err(RuntimeError::InvalidConversion(_))));
}

#[test]
fn test_vm_random_within_bounds() {
    for _ in 0..200 {
        let result = run("取り込む 「数学」；返す 乱数（５、１０）；");
        match result {
            Some(Value::Int(n)) => assert!((5..=10).contains(&n)),
            other => panic!("expected Int, got {:?}", other),
        }
    }
}

#[test]
fn test_vm_random_invalid_range_returns_error() {
    let ast = Parser::new(Lexer::new("取り込む 「数学」；返す 乱数（１０、５）；").tokenize())
        .parse()
        .unwrap();
    let mut compiler = Compiler::new();
    let script = compiler.compile(&ast);
    let result = Vm::with_chunks(compiler.constants, compiler.chunks, script).run();
    assert!(matches!(result, Err(RuntimeError::InvalidConversion(_))));
}

#[test]
fn test_vm_max_min_happy_path() {
    let result = run("取り込む 「数学」；返す 最大（３、７）；");
    assert_eq!(result, Some(Value::Int(7)));

    let result = run("取り込む 「数学」；返す 最小（３、７）；");
    assert_eq!(result, Some(Value::Int(3)));
}

#[test]
fn test_vm_split_and_join_round_trip() {
    let result = run(
        "取り込む 「文字列」；文字列列 部分 ＝ 分割（「あ、い、う」、「、」）；返す 結合（部分、「、」）；",
    );
    assert_eq!(result, Some(Value::Str("あ、い、う".to_string())));
}

#[test]
fn test_vm_contains_true_and_false() {
    let result = run("取り込む 「文字列」；返す 含む（「あいう」、「い」）；");
    assert_eq!(result, Some(Value::Bool(true)));

    let result = run("取り込む 「文字列」；返す 含む（「あいう」、「え」）；");
    assert_eq!(result, Some(Value::Bool(false)));
}

#[test]
fn test_vm_repl_persists_locals_across_lines() {
    let mut compiler = Compiler::new();
    let mut vm = Vm::with_chunks(Vec::new(), Vec::new(), Vec::new());

    let ast1 = Parser::new(Lexer::new("整数 値 ＝ １０；").tokenize())
        .parse()
        .unwrap();
    let instrs1 = compiler.compile(&ast1);
    vm.sync_program(compiler.constants.clone(), compiler.chunks.clone());
    let result1 = vm
        .run_repl_line(instrs1, compiler.script_spans.clone())
        .unwrap();
    assert_eq!(result1, None);

    let ast2 = Parser::new(Lexer::new("値；").tokenize()).parse().unwrap();
    let instrs2 = compiler.compile(&ast2);
    vm.sync_program(compiler.constants.clone(), compiler.chunks.clone());
    let result2 = vm
        .run_repl_line(instrs2, compiler.script_spans.clone())
        .unwrap();
    assert_eq!(result2, Some(Value::Int(10)));
}

#[test]
fn test_vm_repl_line_with_explicit_return_resets_frame_without_panicking() {
    let mut compiler = Compiler::new();
    let mut vm = Vm::with_chunks(Vec::new(), Vec::new(), Vec::new());

    let ast1 = Parser::new(Lexer::new("返す １；").tokenize())
        .parse()
        .unwrap();
    let instrs1 = compiler.compile(&ast1);
    vm.sync_program(compiler.constants.clone(), compiler.chunks.clone());
    let result1 = vm
        .run_repl_line(instrs1, compiler.script_spans.clone())
        .unwrap();
    assert_eq!(result1, Some(Value::Int(1)));

    let ast2 = Parser::new(Lexer::new("印刷（２）；").tokenize())
        .parse()
        .unwrap();
    let instrs2 = compiler.compile(&ast2);
    vm.sync_program(compiler.constants.clone(), compiler.chunks.clone());
    let result2 = vm.run_repl_line(instrs2, compiler.script_spans.clone());
    assert!(result2.is_ok());
}

#[test]
fn test_vm_repl_line_bare_expression_surfaces_value() {
    let mut compiler = Compiler::new();
    let mut vm = Vm::with_chunks(Vec::new(), Vec::new(), Vec::new());

    let ast = Parser::new(Lexer::new("１ ＋ １；").tokenize())
        .parse()
        .unwrap();
    let instrs = compiler.compile(&ast);
    vm.sync_program(compiler.constants.clone(), compiler.chunks.clone());
    let result = vm
        .run_repl_line(instrs, compiler.script_spans.clone())
        .unwrap();
    assert_eq!(result, Some(Value::Int(2)));
}

#[test]
fn test_vm_replace_happy_path() {
    let result = run("取り込む 「文字列」；返す 置換（「あいう」、「い」、「え」）；");
    assert_eq!(result, Some(Value::Str("あえう".to_string())));
}

#[test]
fn test_vm_void_function_call_does_not_halt_program() {
    // A 無-returning function falls off the end of its body without an
    // explicit 返す; the caller must resume rather than the whole program
    // terminating. 表示 prints its arg, then the script returns 99.
    let src = "関数 表示（整数 Ａ）ー＞ 無 ｛ 印刷（Ａ）； ｝表示（５）；返す ９９；";
    assert_eq!(run_result(src).unwrap(), Some(Value::Int(99)));
}

#[test]
fn test_vm_integer_addition_overflow_is_runtime_error() {
    let src = "整数 Ｘ ＝ ９２２３３７２０３６８５４７７５８０７ ＋ １；返す Ｘ；";
    assert_eq!(run_result(src), Err(RuntimeError::IntegerOverflow));
}

#[test]
fn test_vm_integer_multiplication_overflow_is_runtime_error() {
    let src = "整数 Ｘ ＝ ９２２３３７２０３６８５４７７５８０７ ＊ ２；返す Ｘ；";
    assert_eq!(run_result(src), Err(RuntimeError::IntegerOverflow));
}
