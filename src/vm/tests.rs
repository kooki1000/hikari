use super::error::RuntimeError;
use super::frame::INITIAL_LOCALS;
use super::*;
use crate::compiler::{Compiler, Instruction, Value};
use crate::lexer::Lexer;
use crate::parser::Parser;

fn run(src: &str) -> Option<Value> {
    let ast = Parser::new(Lexer::new(src).tokenize()).parse().unwrap();
    let mut compiler = Compiler::new();
    let script = compiler.compile(&ast);
    Vm::with_chunks(compiler.constants, compiler.chunks, script)
        .run()
        .unwrap()
}

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
    let script = compiler.compile(&ast);
    let result = Vm::with_chunks(compiler.constants, compiler.chunks, script).run();
    assert_eq!(result, Err(RuntimeError::DivisionByZero));
}

#[test]
fn test_vm_float_division_by_zero_returns_error() {
    let ast = Parser::new(Lexer::new("小数 結果 ＝ １．０ ／ ０．０；").tokenize())
        .parse()
        .unwrap();
    let mut compiler = Compiler::new();
    let script = compiler.compile(&ast);
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
    let script = compiler.compile(&ast);
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

#[test]
fn test_vm_array_literal_and_index() {
    let result = run("整数列 数字 ＝ 【１、２、３】；返す 数字【１】；");
    assert_eq!(result, Some(Value::Int(2)));
}

#[test]
fn test_vm_index_assign_mutates_array() {
    let result = run("整数列 数字 ＝ 【１、２、３】；数字【０】＝ ９；返す 数字【０】；");
    assert_eq!(result, Some(Value::Int(9)));
}

#[test]
fn test_vm_array_aliasing_reference_semantics() {
    // Assigning Ａ to Ｂ shares the same underlying Rc<RefCell<>>, so
    // mutating through Ｂ must be visible through Ａ.
    let src = "整数列 Ａ ＝ 【１、２、３】；整数列 Ｂ ＝ Ａ；Ｂ【０】＝ ９９；返す Ａ【０】；";
    let result = run(src);
    assert_eq!(result, Some(Value::Int(99)));
}

#[test]
fn test_vm_index_out_of_bounds_returns_error() {
    let ast = Parser::new(Lexer::new("整数列 数字 ＝ 【１、２】；返す 数字【５】；").tokenize())
        .parse()
        .unwrap();
    let mut compiler = Compiler::new();
    let script = compiler.compile(&ast);
    let result = Vm::with_chunks(compiler.constants, compiler.chunks, script).run();
    assert_eq!(
        result,
        Err(RuntimeError::IndexOutOfBounds { index: 5, len: 2 })
    );
}

#[test]
fn test_vm_for_range_sums_to_ten() {
    // 繰り返す カウンタ ＝ ０ から ５ ならば ｛ 合計 ＝ 合計 ＋ カウンタ； ｝
    let src = "整数 合計 ＝ ０；繰り返す カウンタ ＝ ０ から ５ ならば ｛ 合計 ＝ 合計 ＋ カウンタ； ｝返す 合計；";
    let result = run(src);
    assert_eq!(result, Some(Value::Int(1 + 2 + 3 + 4))); // 0..5, excludes 5
}

#[test]
fn test_vm_for_each_sums_array_elements() {
    let src = "整数列 数字 ＝ 【１、２、３】；整数 合計 ＝ ０；各 要素 ： 数字 ならば ｛ 合計 ＝ 合計 ＋ 要素； ｝返す 合計；";
    let result = run(src);
    assert_eq!(result, Some(Value::Int(6)));
}

#[test]
fn test_vm_nested_for_each_loops_no_slot_collision() {
    let src = "整数列 Ａ ＝ 【１、２】；整数列 Ｂ ＝ 【１０、２０、３０】；整数 合計 ＝ ０；各 外側 ： Ａ ならば ｛ 各 内側 ： Ｂ ならば ｛ 合計 ＝ 合計 ＋ 内側； ｝ ｝返す 合計；";
    let result = run(src);
    // Outer loop runs twice; inner sum (10+20+30=60) accumulates each time.
    assert_eq!(result, Some(Value::Int(120)));
}

#[test]
fn test_vm_sequential_for_each_loops_no_slot_collision() {
    let src = "整数列 Ａ ＝ 【１、２】；整数列 Ｂ ＝ 【１０、２０】；整数 合計 ＝ ０；各 要素 ： Ａ ならば ｛ 合計 ＝ 合計 ＋ 要素； ｝各 要素 ： Ｂ ならば ｛ 合計 ＝ 合計 ＋ 要素； ｝返す 合計；";
    let result = run(src);
    assert_eq!(result, Some(Value::Int(33)));
}

#[test]
fn test_vm_print_array() {
    let result = run("印刷（【１、２、３】）；");
    assert_eq!(result, None);
}

#[test]
fn test_vm_if_body_var_does_not_shadow_outer_slot() {
    let src = "整数 Ｎ ＝ １０；もし 真 ならば ｛ 整数 Ｎ ＝ ５； ｝返す Ｎ；";
    let result = run(src);
    assert_eq!(result, Some(Value::Int(10)));
}

#[test]
fn test_vm_while_body_var_does_not_leak_into_outer_slot() {
    let src = "整数 Ｎ ＝ １０；整数 カウンタ ＝ ０；間 カウンタ ＜ ３ ならば ｛ 整数 Ｎ ＝ ９９；カウンタ ＝ カウンタ ＋ １； ｝返す Ｎ；";
    let result = run(src);
    assert_eq!(result, Some(Value::Int(10)));
}

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
    let result1 = vm.run_repl_line(instrs1).unwrap();
    assert_eq!(result1, None);

    let ast2 = Parser::new(Lexer::new("値；").tokenize()).parse().unwrap();
    let instrs2 = compiler.compile(&ast2);
    vm.sync_program(compiler.constants.clone(), compiler.chunks.clone());
    let result2 = vm.run_repl_line(instrs2).unwrap();
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
    let result1 = vm.run_repl_line(instrs1).unwrap();
    assert_eq!(result1, Some(Value::Int(1)));

    let ast2 = Parser::new(Lexer::new("印刷（２）；").tokenize())
        .parse()
        .unwrap();
    let instrs2 = compiler.compile(&ast2);
    vm.sync_program(compiler.constants.clone(), compiler.chunks.clone());
    let result2 = vm.run_repl_line(instrs2);
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
    let result = vm.run_repl_line(instrs).unwrap();
    assert_eq!(result, Some(Value::Int(2)));
}

#[test]
fn test_vm_replace_happy_path() {
    let result = run("取り込む 「文字列」；返す 置換（「あいう」、「い」、「え」）；");
    assert_eq!(result, Some(Value::Str("あえう".to_string())));
}

fn run_result(src: &str) -> Result<Option<Value>, RuntimeError> {
    let ast = Parser::new(Lexer::new(src).tokenize()).parse().unwrap();
    let mut compiler = Compiler::new();
    let script = compiler.compile(&ast);
    Vm::with_chunks(compiler.constants, compiler.chunks, script).run()
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

// ── 7a: modulo ───────────────────────────────────────────────────────

#[test]
fn test_vm_modulo_int() {
    assert_eq!(run("返す １０ ％ ３；"), Some(Value::Int(1)));
}

#[test]
fn test_vm_modulo_float() {
    assert_eq!(run("返す １０．０ ％ ３．０；"), Some(Value::Float(1.0)));
}

#[test]
fn test_vm_modulo_by_zero_returns_error() {
    assert_eq!(
        run_result("返す １０ ％ ０；"),
        Err(RuntimeError::DivisionByZero)
    );
}

#[test]
fn test_vm_fizzbuzz_with_modulo() {
    let src =
        "整数 Ｎ ＝ １５；もし Ｎ ％ １５ ＝＝ ０ ならば ｛ 返す ０； ｝違えば｛ 返す Ｎ； ｝";
    assert_eq!(run(src), Some(Value::Int(0)));
}

// ── 7b: array builtins ──────────────────────────────────────────────

#[test]
fn test_vm_array_len_builtin() {
    let src = "取り込む 「配列」；整数列 数字 ＝ 【１、２、３】；返す 要素数（数字）；";
    assert_eq!(run(src), Some(Value::Int(3)));
}

#[test]
fn test_vm_push_pop_round_trip() {
    let src = "取り込む 「配列」；整数列 数字 ＝ 新配列＜整数＞；追加（数字、１）；追加（数字、２）；整数 最後 ＝ 取り出す（数字）；もし 最後 ＝＝ ２ かつ 要素数（数字） ＝＝ １ ならば ｛ 返す １； ｝違えば｛ 返す ０； ｝";
    assert_eq!(run(src), Some(Value::Int(1)));
}

#[test]
fn test_vm_pop_empty_array_returns_error() {
    let src = "取り込む 「配列」；整数列 数字 ＝ 新配列＜整数＞；返す 取り出す（数字）；";
    assert_eq!(run_result(src), Err(RuntimeError::EmptyArray));
}

#[test]
fn test_vm_contains_array_found_and_not_found() {
    let src = "取り込む 「配列」；整数列 数字 ＝ 【１、２、３】；返す 含む配列（数字、２）；";
    assert_eq!(run(src), Some(Value::Bool(true)));

    let src = "取り込む 「配列」；整数列 数字 ＝ 【１、２、３】；返す 含む配列（数字、９）；";
    assert_eq!(run(src), Some(Value::Bool(false)));
}

#[test]
fn test_vm_index_of_found_and_not_found() {
    let src = "取り込む 「配列」；整数列 数字 ＝ 【１０、２０、３０】；返す 位置（数字、２０）；";
    assert_eq!(run(src), Some(Value::Int(1)));

    let src = "取り込む 「配列」；整数列 数字 ＝ 【１０、２０、３０】；返す 位置（数字、９９）；";
    assert_eq!(run(src), Some(Value::Int(-1)));
}

#[test]
fn test_vm_reverse_mutates_in_place_and_returns_array() {
    let src = "取り込む 「配列」；整数列 数字 ＝ 【１、２、３】；整数列 同じ ＝ 逆順（数字）；返す 数字【０】；";
    assert_eq!(run(src), Some(Value::Int(3)));
}

#[test]
fn test_vm_sort_int_array() {
    let src = "取り込む 「配列」；整数列 数字 ＝ 【３、１、２】；整列（数字）；返す 数字【０】；";
    assert_eq!(run(src), Some(Value::Int(1)));
}

#[test]
fn test_vm_sort_float_array() {
    let src = "取り込む 「配列」；小数列 数字 ＝ 【３．０、１．０、２．０】；整列（数字）；返す 数字【０】；";
    assert_eq!(run(src), Some(Value::Float(1.0)));
}

#[test]
fn test_vm_sort_string_array() {
    let src = "取り込む 「配列」；文字列列 文字 ＝ 【「う」、「あ」、「い」】；整列（文字）；返す 文字【０】；";
    assert_eq!(run(src), Some(Value::Str("あ".to_string())));
}

#[test]
fn test_vm_slice_returns_new_array_without_mutating_original() {
    let src = "取り込む 「配列」；整数列 数字 ＝ 【１、２、３、４】；整数列 部分 ＝ 部分列（数字、１、３）；もし 部分【０】 ＝＝ ２ かつ 部分【１】 ＝＝ ３ かつ 要素数（数字） ＝＝ ４ ならば ｛ 返す １； ｝違えば｛ 返す ０； ｝";
    assert_eq!(run(src), Some(Value::Int(1)));
}

#[test]
fn test_vm_slice_out_of_bounds_returns_error() {
    let src = "取り込む 「配列」；整数列 数字 ＝ 【１、２】；返す 部分列（数字、０、５）；";
    assert!(matches!(
        run_result(src),
        Err(RuntimeError::IndexOutOfBounds { .. })
    ));
}

#[test]
fn test_vm_new_array_then_build_and_check_len() {
    let src = "取り込む 「配列」；文字列列 単語 ＝ 新配列＜文字列＞；追加（単語、「あ」）；追加（単語、「い」）；返す 要素数（単語）；";
    assert_eq!(run(src), Some(Value::Int(2)));
}

// ── 7c: more math builtins ─────────────────────────────────────────

#[test]
fn test_vm_pow_int_and_float() {
    let src = "取り込む 「数学」；返す 累乗（２、１０）；";
    assert_eq!(run(src), Some(Value::Int(1024)));

    let src = "取り込む 「数学」；返す 累乗（２．０、０．５）；";
    match run(src) {
        Some(Value::Float(f)) => assert!((f - std::f64::consts::SQRT_2).abs() < 1e-9),
        other => panic!("expected Float, got {:?}", other),
    }
}

#[test]
fn test_vm_pow_negative_exponent_returns_error() {
    let src = "取り込む 「数学」；返す 累乗（２、ー１）；";
    assert!(matches!(
        run_result(src),
        Err(RuntimeError::InvalidConversion(_))
    ));
}

#[test]
fn test_vm_floor_ceil_round() {
    let src = "取り込む 「数学」；返す 切り捨て（３．７）；";
    assert_eq!(run(src), Some(Value::Int(3)));

    let src = "取り込む 「数学」；返す 切り上げ（３．２）；";
    assert_eq!(run(src), Some(Value::Int(4)));

    let src = "取り込む 「数学」；返す 四捨五入（３．５）；";
    assert_eq!(run(src), Some(Value::Int(4)));
}

#[test]
fn test_vm_remainder_function_form() {
    let src = "取り込む 「数学」；返す 余り（１０、３）；";
    assert_eq!(run(src), Some(Value::Int(1)));
}

#[test]
fn test_vm_remainder_by_zero_returns_error() {
    let src = "取り込む 「数学」；返す 余り（１０、０）；";
    assert_eq!(run_result(src), Err(RuntimeError::DivisionByZero));
}

// ── 8b: break / continue ─────────────────────────────────────────────

#[test]
fn test_vm_while_break_stops_early() {
    let src = "整数 カウンタ ＝ ０；間 カウンタ ＜ １０ ならば ｛ もし カウンタ ＝＝ ５ ならば ｛ 抜ける； ｝ カウンタ ＝ カウンタ ＋ １； ｝返す カウンタ；";
    assert_eq!(run(src), Some(Value::Int(5)));
}

#[test]
fn test_vm_while_continue_skips_even_numbers_sums_odds() {
    // 続ける skips adding on even numbers, but the loop variable still
    // increments and the loop still completes: 1+3+5+7+9 = 25.
    let src = "整数 合計 ＝ ０；整数 ｉ ＝ １；間 ｉ ≦ １０ ならば ｛ もし ｉ ％ ２ ＝＝ ０ ならば ｛ ｉ ＝ ｉ ＋ １；続ける； ｝ 合計 ＝ 合計 ＋ ｉ；ｉ ＝ ｉ ＋ １； ｝返す 合計；";
    assert_eq!(run(src), Some(Value::Int(25)));
}

#[test]
fn test_vm_for_range_break_stops_early() {
    let src = "整数 合計 ＝ ０；繰り返す ｉ ＝ ０ から １０ ならば ｛ もし ｉ ＝＝ ５ ならば ｛ 抜ける； ｝ 合計 ＝ 合計 ＋ １； ｝返す 合計；";
    assert_eq!(run(src), Some(Value::Int(5)));
}

#[test]
fn test_vm_for_range_continue_skips_even_increments_correctly() {
    // Proves the increment step still runs on a 続ける'd iteration:
    // summing 0..10 while skipping evens gives 1+3+5+7+9 = 25.
    let src = "整数 合計 ＝ ０；繰り返す ｉ ＝ ０ から １０ ならば ｛ もし ｉ ％ ２ ＝＝ ０ ならば ｛ 続ける； ｝ 合計 ＝ 合計 ＋ ｉ； ｝返す 合計；";
    assert_eq!(run(src), Some(Value::Int(25)));
}

#[test]
fn test_vm_for_each_break_stops_early() {
    let src = "整数列 数字 ＝ 【１、２、３、４、５】；整数 合計 ＝ ０；各 値 ： 数字 ならば ｛ もし 値 ＝＝ ３ ならば ｛ 抜ける； ｝ 合計 ＝ 合計 ＋ 値； ｝返す 合計；";
    assert_eq!(run(src), Some(Value::Int(3))); // 1 + 2
}

#[test]
fn test_vm_for_each_continue_skips_even_increments_correctly() {
    let src = "整数列 数字 ＝ 【１、２、３、４、５、６、７、８、９、１０】；整数 合計 ＝ ０；各 値 ： 数字 ならば ｛ もし 値 ％ ２ ＝＝ ０ ならば ｛ 続ける； ｝ 合計 ＝ 合計 ＋ 値； ｝返す 合計；";
    assert_eq!(run(src), Some(Value::Int(25)));
}

#[test]
fn test_vm_break_inside_nested_if_inside_loop() {
    let src = "整数 カウンタ ＝ ０；間 真 ならば ｛ もし 真 ならば ｛ もし カウンタ ＝＝ ３ ならば ｛ 抜ける； ｝ ｝ カウンタ ＝ カウンタ ＋ １； ｝返す カウンタ；";
    assert_eq!(run(src), Some(Value::Int(3)));
}

#[test]
fn test_vm_outer_loop_break_unaffected_by_inner_loop_break() {
    // Inner loop's 抜ける only exits the inner loop (after 2 inner runs
    // each outer iteration); outer loop runs its own 3 iterations.
    let src = "整数 外回数 ＝ ０；整数 内合計 ＝ ０；繰り返す 外 ＝ ０ から ３ ならば ｛ 外回数 ＝ 外回数 ＋ １；繰り返す 内 ＝ ０ から １０ ならば ｛ もし 内 ＝＝ ２ ならば ｛ 抜ける； ｝ 内合計 ＝ 内合計 ＋ １； ｝ ｝返す 外回数 ＊ １００ ＋ 内合計；";
    // Outer runs 3 times; inner runs 2 iterations (0,1) each time → 6.
    assert_eq!(run(src), Some(Value::Int(306)));
}

// ── 8c: bare return / void semantics ──────────────────────────────────

#[test]
fn test_vm_bare_return_in_void_function_keeps_stack_balanced() {
    let src =
        "関数 何もしない（）ー＞ 無 ｛ 返す； ｝何もしない（）；整数 結果 ＝ ４２；返す 結果；";
    assert_eq!(run(src), Some(Value::Int(42)));
}

// ── 9a: records ───────────────────────────────────────────────────────

#[test]
fn test_vm_record_construct_and_read_field() {
    let src = "型 点 ｛ 整数 ｘ； 整数 ｙ； ｝点 ｐ ＝ 点 ｛ ｘ：１、ｙ：２ ｝；返す ｐ：：ｘ ＋ ｐ：：ｙ；";
    assert_eq!(run(src), Some(Value::Int(3)));
}

#[test]
fn test_vm_record_field_assign_mutates() {
    let src = "型 点 ｛ 整数 ｘ； ｝点 ｐ ＝ 点 ｛ ｘ：１ ｝；ｐ：：ｘ ＝ ９９；返す ｐ：：ｘ；";
    assert_eq!(run(src), Some(Value::Int(99)));
}

#[test]
fn test_vm_record_aliasing_reference_semantics() {
    // Assigning Ａ to Ｂ shares the same underlying Rc<RefCell<>>, so
    // mutating a field through Ｂ must be visible through Ａ, mirroring
    // array aliasing.
    let src = "型 点 ｛ 整数 ｘ； ｝点 Ａ ＝ 点 ｛ ｘ：１ ｝；点 Ｂ ＝ Ａ；Ｂ：：ｘ ＝ ９９；返す Ａ：：ｘ；";
    assert_eq!(run(src), Some(Value::Int(99)));
}

#[test]
fn test_vm_record_as_function_param_and_return() {
    let src = "型 点 ｛ 整数 ｘ； 整数 ｙ； ｝関数 ずらす（点 Ｐ）ー＞ 点 ｛ Ｐ：：ｘ ＝ Ｐ：：ｘ ＋ １；返す Ｐ； ｝点 ａ ＝ 点 ｛ ｘ：１、ｙ：２ ｝；点 ｂ ＝ ずらす（ａ）；返す ｂ：：ｘ；";
    assert_eq!(run(src), Some(Value::Int(2)));
}

#[test]
fn test_vm_record_with_array_field() {
    let src =
        "型 箱 ｛ 整数列 数字； ｝箱 ｂ ＝ 箱 ｛ 数字：【１、２、３】 ｝；返す ｂ：：数字【１】；";
    assert_eq!(run(src), Some(Value::Int(2)));
}

// ── 9b: enums and pattern matching ──────────────────────────────────

#[test]
fn test_vm_construct_and_print_payload_and_payloadless_variants() {
    let src = "構造 結果 ｛ 成功（整数）、 異常 ｝印刷（成功（１２３））；印刷（異常（））；";
    let result = run(src);
    assert_eq!(result, None);
}

#[test]
fn test_vm_match_dispatches_to_correct_arm_for_each_variant() {
    let src = "構造 信号 ｛ 赤、 黄、 青 ｝関数 名前（信号 値）ー＞ 整数 ｛ 照合 値 ｛ 赤（） ならば ｛ 返す １； ｝ 黄（） ならば ｛ 返す ２； ｝ 青（） ならば ｛ 返す ３； ｝ ｝返す ０； ｝返す 名前（赤（）） ＊ １００ ＋ 名前（黄（）） ＊ １０ ＋ 名前（青（））；";
    assert_eq!(run(src), Some(Value::Int(123)));
}

#[test]
fn test_vm_match_binder_receives_correct_payload_values_in_order() {
    let src = "構造 結果 ｛ 点（整数、整数） ｝結果 値 ＝ 点（３、４）；照合 値 ｛ 点（ｘ、ｙ） ならば ｛ 返す ｘ ＊ １０ ＋ ｙ； ｝ ｝";
    assert_eq!(run(src), Some(Value::Int(34)));
}

#[test]
fn test_vm_non_exhaustive_match_rejected_at_typecheck_time() {
    let src = "構造 信号 ｛ 赤、 青 ｝信号 値 ＝ 赤（）；照合 値 ｛ 赤（） ならば ｛ ｝ ｝";
    let ast = Parser::new(Lexer::new(src).tokenize()).parse().unwrap();
    let err = crate::typechecker::TypeChecker::new().check(&ast);
    assert!(err.is_err());
}

#[test]
fn test_vm_build_sort_print_array_program() {
    let src = "取り込む 「配列」；整数列 数字 ＝ 新配列＜整数＞；追加（数字、５）；追加（数字、１）；追加（数字、３）；整列（数字）；返す 数字；";
    let result = run(src);
    match result {
        Some(Value::Array(a)) => {
            let v: Vec<i64> = a
                .borrow()
                .iter()
                .map(|x| match x {
                    Value::Int(n) => *n,
                    _ => panic!("expected Int"),
                })
                .collect();
            assert_eq!(v, vec![1, 3, 5]);
        }
        other => panic!("expected Array, got {:?}", other),
    }
}

// ── 9c: maps ──────────────────────────────────────────────────────────

#[test]
fn test_vm_map_literal_creation_and_lookup() {
    let src = "取り込む 「辞書」；辞書＜文字列、整数＞ スコア ＝ ｛ 「アリス」：１００、「ボブ」：８５ ｝；返す スコア【「アリス」】；";
    assert_eq!(run(src), Some(Value::Int(100)));
}

#[test]
fn test_vm_map_insert_and_contains() {
    let src = "取り込む 「辞書」；辞書＜文字列、整数＞ m ＝ ｛ 「あ」：１ ｝；m【「い」】 ＝ ２；真偽 結果 ＝ 含む（m、「い」）；返す 結果；";
    assert_eq!(run(src), Some(Value::Bool(true)));
}

#[test]
fn test_vm_map_keys_and_values_builtins() {
    // 鍵一覧 returns an array of string keys sorted alphabetically.
    let src = "取り込む 「辞書」；辞書＜文字列、整数＞ m ＝ ｛ 「い」：２、「あ」：１ ｝；文字列列 ks ＝ 鍵一覧（m）；返す ks【０】；";
    assert_eq!(run(src), Some(Value::Str("あ".to_string())));

    // 値一覧 returns values in key-sorted order.
    let src2 = "取り込む 「辞書」；辞書＜文字列、整数＞ m ＝ ｛ 「い」：２、「あ」：１ ｝；整数列 vs ＝ 値一覧（m）；返す vs【０】；";
    assert_eq!(run(src2), Some(Value::Int(1)));
}

#[test]
fn test_vm_map_delete_removes_key() {
    let src = "取り込む 「辞書」；辞書＜文字列、整数＞ m ＝ ｛ 「あ」：１、「い」：２ ｝；削除（m、「あ」）；返す 含む（m、「あ」）；";
    assert_eq!(run(src), Some(Value::Bool(false)));
}

#[test]
fn test_vm_map_missing_key_returns_error() {
    let src = "取り込む 「辞書」；辞書＜文字列、整数＞ m ＝ ｛ 「あ」：１ ｝；返す m【「い」】；";
    assert_eq!(
        run_result(src),
        Err(RuntimeError::KeyNotFound("い".to_string()))
    );
}

#[test]
fn test_vm_empty_map_literal_and_insert() {
    let src = "取り込む 「辞書」；辞書＜文字列、整数＞ m ＝ ｛｝；m【「キー」】 ＝ ４２；返す m【「キー」】；";
    assert_eq!(run(src), Some(Value::Int(42)));
}

#[test]
fn test_vm_lambda_creation_and_call() {
    // Lambda stored in var, then called through var.
    // The var decl ends with ；ー
    let src = "関数＜（整数） ー＞ 整数＞ f ＝ ｜ｎ：整数｜ ー＞ 整数 ｛ 返す ｎ ＊ ２； ｝；返す f（５）；";
    assert_eq!(run(src), Some(Value::Int(10)));
}

#[test]
fn test_vm_named_function_as_value() {
    // Named function used as a first-class value.
    let src = "関数 二倍（整数 ｎ）ー＞ 整数 ｛ 返す ｎ ＊ ２； ｝関数＜（整数） ー＞ 整数＞ f ＝ 二倍；返す f（７）；";
    assert_eq!(run(src), Some(Value::Int(14)));
}

#[test]
fn test_vm_map_array_with_named_function() {
    // マップ HOF with named function
    let src = "取り込む 「関数」；関数 二倍（整数 ｎ）ー＞ 整数 ｛ 返す ｎ ＊ ２； ｝整数列 nums ＝ 【１、２、３】；整数列 result ＝ マップ（nums、二倍）；返す result【２】；";
    assert_eq!(run(src), Some(Value::Int(6)));
}

#[test]
fn test_vm_filter_array_with_lambda() {
    // 絞り込み HOF with lambda predicate (lambda is an argument, no extra ；)
    let src = "取り込む 「関数」；取り込む 「配列」；整数列 nums ＝ 【１、２、３、４、５】；整数列 evens ＝ 絞り込み（nums、｜ｎ：整数｜ ー＞ 真偽 ｛ 返す ｎ ％ ２ ＝＝ ０； ｝）；返す 要素数（evens）；";
    assert_eq!(run(src), Some(Value::Int(2)));
}

#[test]
fn test_vm_fold_array() {
    // 畳み込み sum
    let src = "取り込む 「関数」；整数列 nums ＝ 【１、２、３、４、５】；整数 total ＝ 畳み込み（nums、０、｜acc：整数、ｎ：整数｜ ー＞ 整数 ｛ 返す acc ＋ ｎ； ｝）；返す total；";
    assert_eq!(run(src), Some(Value::Int(15)));
}
