use super::*;

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
    let script = compiler.compile(&ast).unwrap();
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
