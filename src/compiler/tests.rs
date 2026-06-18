use super::*;
use crate::lexer::Lexer;
use crate::parser::Parser;

fn compile(src: &str) -> (Vec<Instruction>, Vec<Value>) {
    let ast = Parser::new(Lexer::new(src).tokenize()).parse().unwrap();
    let mut c = Compiler::new();
    let instrs = c.compile(&ast).unwrap();
    (instrs, c.constants)
}

#[test]
fn test_compile_integer_literal() {
    let (instrs, constants) = compile("整数 年齢 ＝ ２０；");
    assert_eq!(instrs[0], Instruction::LoadConst(0));
    assert_eq!(instrs[1], Instruction::StoreLocal(0));
    assert_eq!(constants[0], Value::Int(20));
}

#[test]
fn test_compile_bool_literal() {
    let (instrs, constants) = compile("真偽 フラグ ＝ 真；");
    assert_eq!(instrs[0], Instruction::LoadConst(0));
    assert_eq!(instrs[1], Instruction::StoreLocal(0));
    assert_eq!(constants[0], Value::Bool(true));
}

#[test]
fn test_compile_binary_add() {
    let (instrs, constants) = compile("整数 結果 ＝ １ ＋ ２；");
    assert_eq!(instrs[0], Instruction::LoadConst(0));
    assert_eq!(instrs[1], Instruction::LoadConst(1));
    assert_eq!(instrs[2], Instruction::Add);
    assert_eq!(instrs[3], Instruction::StoreLocal(0));
    assert_eq!(constants, vec![Value::Int(1), Value::Int(2)]);
}

#[test]
fn test_compile_constant_deduplication() {
    let (instrs, constants) = compile("整数 Ａ ＝ ５；整数 Ｂ ＝ ５；");
    assert_eq!(constants, vec![Value::Int(5)]);
    assert_eq!(instrs[0], Instruction::LoadConst(0));
    assert_eq!(instrs[2], Instruction::LoadConst(0));
}

#[test]
fn test_compile_load_local() {
    let (instrs, _) = compile("整数 Ａ ＝ １０；整数 Ｂ ＝ Ａ；");
    assert_eq!(instrs[2], Instruction::LoadLocal(0));
    assert_eq!(instrs[3], Instruction::StoreLocal(1));
}

#[test]
fn test_compile_return() {
    let src = "関数 計算（整数 Ａ）ー＞ 整数 ｛ 返す Ａ ＋ １； ｝";
    let ast = Parser::new(Lexer::new(src).tokenize()).parse().unwrap();
    let mut c = Compiler::new();
    c.compile(&ast).unwrap();
    // The function chunk (index 0) must end with Return.
    assert_eq!(
        c.chunks[0].instructions.last().unwrap(),
        &Instruction::Return
    );
}

#[test]
fn test_compile_while_loop() {
    // 整数 カウンタ ＝ ０；間 カウンタ ＜ ３ ならば ｛ 印刷（カウンタ）； ｝
    let src = "整数 カウンタ ＝ ０；間 カウンタ ＜ ３ ならば ｛ 印刷（カウンタ）； ｝";
    let (instrs, _) = compile(src);
    // layout: LoadConst(0), StoreLocal(0),        ← var decl
    //         [loop_start=2] LoadLocal(0), LoadConst(1), LessThan,  ← condition
    //         JumpIfFalse(after),                  ← idx 5
    //         LoadLocal(0), Print,                 ← body
    //         Jump(2),                             ← back-edge
    //         [after=9]
    assert!(matches!(instrs[5], Instruction::JumpIfFalse(9)));
    assert!(matches!(instrs[8], Instruction::Jump(2)));
}

#[test]
fn test_compile_reassignment_reuses_slot() {
    let (instrs, _) = compile("整数 年齢 ＝ ２０；年齢 ＝ ３０；");
    assert_eq!(instrs[1], Instruction::StoreLocal(0));
    assert_eq!(instrs[3], Instruction::StoreLocal(0));
}

#[test]
fn test_compile_unary_minus() {
    let (instrs, _) = compile("整数 結果 ＝ ー５；");
    assert_eq!(instrs[0], Instruction::LoadConst(0));
    assert_eq!(instrs[1], Instruction::Negate);
    assert_eq!(instrs[2], Instruction::StoreLocal(0));
}

#[test]
fn test_compile_builtin_strlen_emits_call_builtin() {
    let (instrs, _) = compile("整数 結果 ＝ 文字数（「あ」）；");
    assert!(matches!(
        instrs[1],
        Instruction::CallBuiltin(BuiltinFn::Len, 1)
    ));
}

#[test]
fn test_compile_builtin_input_emits_zero_args() {
    let (instrs, _) = compile("文字列 結果 ＝ 入力（）；");
    assert!(matches!(
        instrs[0],
        Instruction::CallBuiltin(BuiltinFn::Input, 0)
    ));
}

#[test]
fn test_compile_builtin_to_str_emits_call_builtin() {
    let (instrs, _) = compile("文字列 結果 ＝ 文字列化（１）；");
    assert!(matches!(
        instrs[1],
        Instruction::CallBuiltin(BuiltinFn::ToStr, 1)
    ));
}

#[test]
fn test_compile_stdlib_builtin_emits_call_builtin() {
    let (instrs, _) = compile("取り込む 「数学」；整数 結果 ＝ 絶対値（ー５）；");
    assert!(matches!(
        instrs[2],
        Instruction::CallBuiltin(BuiltinFn::Abs, 1)
    ));
}

#[test]
fn test_compile_call_emits_correct_fn_idx() {
    // 関数 二倍（整数 Ａ）ー＞ 整数 ｛ 返す Ａ ＊ ２； ｝ 返す 二倍（５）；
    let src = "関数 二倍（整数 Ａ）ー＞ 整数 ｛ 返す Ａ ＊ ２； ｝返す 二倍（５）；";
    let ast = Parser::new(Lexer::new(src).tokenize()).parse().unwrap();
    let mut c = Compiler::new();
    let script = c.compile(&ast).unwrap();
    // Script: LoadConst(5), Call(0, 1), Return
    assert!(matches!(script[1], Instruction::Call(0, 1)));
}

#[test]
fn test_compile_array_literal_emits_make_array() {
    let (instrs, constants) = compile("整数列 数字 ＝ 【１、２、３】；");
    assert_eq!(instrs[0], Instruction::LoadConst(0));
    assert_eq!(instrs[1], Instruction::LoadConst(1));
    assert_eq!(instrs[2], Instruction::LoadConst(2));
    assert_eq!(instrs[3], Instruction::MakeArray(3));
    assert_eq!(instrs[4], Instruction::StoreLocal(0));
    assert_eq!(constants, vec![Value::Int(1), Value::Int(2), Value::Int(3)]);
}

#[test]
fn test_compile_index_expr_emits_get_index() {
    let (instrs, _) = compile("整数列 数字 ＝ 【１】；返す 数字【０】；");
    assert!(instrs.contains(&Instruction::GetIndex));
}

#[test]
fn test_compile_index_assign_emits_set_index() {
    let (instrs, _) = compile("整数列 数字 ＝ 【１】；数字【０】＝ ２；");
    assert!(instrs.contains(&Instruction::SetIndex));
}

#[test]
fn test_compile_for_range_loop() {
    let (instrs, _) = compile("繰り返す カウンタ ＝ ０ から ５ ならば ｛ 印刷（カウンタ）； ｝");
    assert!(instrs.iter().any(|i| matches!(i, Instruction::Jump(_))));
    assert!(
        instrs
            .iter()
            .any(|i| matches!(i, Instruction::JumpIfFalse(_)))
    );
}

#[test]
fn test_compile_for_each_loop_emits_array_len() {
    let src = "整数列 数字 ＝ 【１、２】；各 要素 ： 数字 ならば ｛ 印刷（要素）； ｝";
    let (instrs, _) = compile(src);
    assert!(instrs.contains(&Instruction::ArrayLen));
}

#[test]
fn test_compile_nested_for_each_unique_synthetic_slots() {
    let src = "整数列 Ａ ＝ 【１】；整数列 Ｂ ＝ 【２】；各 要素 ： Ａ ならば ｛ 各 内側 ： Ｂ ならば ｛ 印刷（内側）； ｝ ｝";
    let (instrs, _) = compile(src);
    let array_len_count = instrs
        .iter()
        .filter(|i| matches!(i, Instruction::ArrayLen))
        .count();
    assert_eq!(array_len_count, 2);
}

#[test]
fn test_compile_if_body_shadowing_gets_distinct_slot() {
    let (instrs, _) =
        compile("整数 Ｎ ＝ １０；もし 真 ならば ｛ 整数 Ｎ ＝ ５； ｝整数 結果 ＝ Ｎ；");
    // LoadConst(0)=10, StoreLocal(0)=outer Ｎ
    assert_eq!(instrs[0], Instruction::LoadConst(0));
    assert_eq!(instrs[1], Instruction::StoreLocal(0));
    // Inner Ｎ inside the if-block must get a distinct slot (1), not slot 0.
    assert!(
        instrs
            .iter()
            .any(|i| matches!(i, Instruction::StoreLocal(1)))
    );
    // Final read of outer Ｎ after the if-block must load slot 0, not 1.
    assert!(instrs.contains(&Instruction::LoadLocal(0)));
    assert!(!instrs.contains(&Instruction::LoadLocal(1)));
}

#[test]
fn test_compile_repl_persists_script_slots_across_calls() {
    let ast1 = Parser::new(Lexer::new("整数 値 ＝ １０；").tokenize())
        .parse()
        .unwrap();
    let mut c = Compiler::new();
    let instrs1 = c.compile(&ast1).unwrap();
    assert_eq!(instrs1[1], Instruction::StoreLocal(0));

    let ast2 = Parser::new(Lexer::new("印刷（値）；").tokenize())
        .parse()
        .unwrap();
    let instrs2 = c.compile(&ast2).unwrap();
    assert_eq!(instrs2[0], Instruction::LoadLocal(0));
}

#[test]
fn test_compile_try_catch_emits_try_start_end_and_jump() {
    // layout: [0] TryStart(catch_target, error_slot)
    //         [1] LoadConst(0)=1, [2] Print     ← try_body
    //         [3] TryEnd
    //         [4] Jump(after_catch)
    //         [catch_target=5] LoadConst(1), [6] Print   ← catch_body
    //         [after_catch=7]
    let src = "試す ｛ 印刷（１）； ｝ 失敗 失敗内容 ｛ 印刷（失敗内容）； ｝";
    let (instrs, _) = compile(src);
    assert!(matches!(instrs[0], Instruction::TryStart(5, 0)));
    assert_eq!(instrs[3], Instruction::TryEnd);
    assert!(matches!(instrs[4], Instruction::Jump(7)));
    assert_eq!(instrs.len(), 7);
}

#[test]
fn test_compile_modulo_emits_mod_instruction() {
    let (instrs, _) = compile("整数 結果 ＝ １０ ％ ３；");
    assert!(instrs.contains(&Instruction::Mod));
}

#[test]
fn test_compile_array_len_builtin_emits_call_builtin() {
    let src = "取り込む 「配列」；整数列 数字 ＝ 【１】；整数 結果 ＝ 要素数（数字）；";
    let (instrs, _) = compile(src);
    assert!(instrs.contains(&Instruction::CallBuiltin(BuiltinFn::ArrayLen, 1)));
}

#[test]
fn test_compile_new_array_emits_make_array_zero() {
    let (instrs, _) = compile("整数列 数字 ＝ 新配列＜整数＞；");
    assert_eq!(instrs[0], Instruction::MakeArray(0));
    assert_eq!(instrs[1], Instruction::StoreLocal(0));
}

#[test]
fn test_compile_pow_builtin_emits_call_builtin() {
    let src = "取り込む 「数学」；整数 結果 ＝ 累乗（２、３）；";
    let (instrs, _) = compile(src);
    assert!(instrs.contains(&Instruction::CallBuiltin(BuiltinFn::Pow, 2)));
}

// ── 8b: break / continue ─────────────────────────────────────────────

#[test]
fn test_compile_while_break_jumps_to_after_loop() {
    // layout: [0] LoadConst(0)=真  [loop_start=1] ... condition is the
    // same constant reload, [jif] JumpIfFalse(after), body: Break, then
    // back-edge Jump(loop_start), [after]
    let src = "間 真 ならば ｛ 抜ける； ｝";
    let (instrs, _) = compile(src);
    let after_loop = instrs.len() as u16;
    let break_jump = instrs
        .iter()
        .find(|i| matches!(i, Instruction::Jump(n) if *n == after_loop));
    assert!(break_jump.is_some());
}

#[test]
fn test_compile_while_continue_jumps_to_loop_start() {
    let src = "間 真 ならば ｛ 続ける； ｝";
    let (instrs, _) = compile(src);
    // loop_start is index 0 (condition re-check starts the loop).
    assert!(instrs.contains(&Instruction::Jump(0)));
}

// ── 9a: records ───────────────────────────────────────────────────────

#[test]
fn test_compile_record_lit_emits_make_record_in_source_order() {
    let src = "型 点 ｛ 整数 ｘ； 整数 ｙ； ｝点 ｐ ＝ 点 ｛ ｙ：２、ｘ：１ ｝；";
    let (instrs, _) = compile(src);
    assert!(matches!(
        &instrs[2],
        Instruction::MakeRecord(names) if names == &vec!["ｙ".to_string(), "ｘ".to_string()]
    ));
}

#[test]
fn test_compile_field_access_emits_get_field() {
    let src = "型 点 ｛ 整数 ｘ； ｝点 ｐ ＝ 点 ｛ ｘ：１ ｝；返す ｐ：：ｘ；";
    let (instrs, _) = compile(src);
    assert!(instrs.contains(&Instruction::GetField("ｘ".to_string())));
}

#[test]
fn test_compile_field_assign_emits_set_field() {
    let src = "型 点 ｛ 整数 ｘ； ｝点 ｐ ＝ 点 ｛ ｘ：１ ｝；ｐ：：ｘ ＝ ９；";
    let (instrs, _) = compile(src);
    assert!(instrs.contains(&Instruction::SetField("ｘ".to_string())));
}

#[test]
fn test_compile_type_decl_emits_no_instructions() {
    let src = "型 点 ｛ 整数 ｘ； ｝";
    let (instrs, _) = compile(src);
    assert!(instrs.is_empty());
}

// ── 9b: enums and pattern matching ──────────────────────────────────

#[test]
fn test_compile_enum_decl_emits_no_instructions() {
    let src = "構造 結果 ｛ 成功（整数） ｝";
    let (instrs, _) = compile(src);
    assert!(instrs.is_empty());
}

#[test]
fn test_compile_variant_construction_emits_make_enum() {
    let src = "構造 結果 ｛ 成功（整数） ｝結果 値 ＝ 成功（１）；";
    let (instrs, _) = compile(src);
    assert!(matches!(
        &instrs[1],
        Instruction::MakeEnum(enum_name, variant, 1)
        if enum_name == "結果" && variant == "成功"
    ));
}

#[test]
fn test_compile_zero_payload_variant_construction_emits_make_enum_zero() {
    let src = "構造 信号 ｛ 赤 ｝信号 値 ＝ 赤（）；";
    let (instrs, _) = compile(src);
    assert!(matches!(
        &instrs[0],
        Instruction::MakeEnum(enum_name, variant, 0)
        if enum_name == "信号" && variant == "赤"
    ));
}

#[test]
fn test_compile_match_two_arms_emits_tag_equals_and_correct_jump_targets() {
    let src = "構造 信号 ｛ 赤、 青 ｝信号 値 ＝ 赤（）；照合 値 ｛ 赤（） ならば ｛ 印刷（１）； ｝ 青（） ならば ｛ 印刷（２）； ｝ ｝";
    let (instrs, _) = compile(src);

    let tag_equals_count = instrs
        .iter()
        .filter(|i| matches!(i, Instruction::TagEquals(_)))
        .count();
    assert_eq!(tag_equals_count, 2);
    assert!(matches!(&instrs[3], Instruction::TagEquals(v) if v == "赤"));

    let after_match = instrs.len() as u16;
    // The last arm's JumpIfFalse must land exactly at after_match.
    let last_jif = instrs
        .iter()
        .rev()
        .find(|i| matches!(i, Instruction::JumpIfFalse(_)))
        .unwrap();
    assert_eq!(last_jif, &Instruction::JumpIfFalse(after_match));

    // Every arm's trailing Jump (skip-to-end) must also land at after_match.
    let unconditional_jumps: Vec<&Instruction> = instrs
        .iter()
        .filter(|i| matches!(i, Instruction::Jump(n) if *n == after_match))
        .collect();
    assert_eq!(unconditional_jumps.len(), 2);
}

#[test]
fn test_compile_for_range_continue_targets_increment_not_loop_start() {
    let src = "繰り返す ｉ ＝ ０ から ５ ならば ｛ 続ける； ｝";
    let (instrs, _) = compile(src);
    // The continue's Jump target must be the increment step's LoadLocal,
    // not loop_start (index 1, where the condition re-check begins).
    let continue_jump_target = instrs.iter().find_map(|i| match i {
        Instruction::Jump(n) if *n != 1 => Some(*n),
        _ => None,
    });
    assert!(continue_jump_target.is_some());
    let target = continue_jump_target.unwrap() as usize;
    assert!(matches!(instrs[target], Instruction::LoadLocal(_)));
}

// ── 11b: multi-value 印刷 ─────────────────────────────────────────────

#[test]
fn test_compile_print_single_value() {
    let (instrs, _) = compile("印刷（４２）；");
    assert_eq!(instrs[0], Instruction::LoadConst(0));
    assert_eq!(instrs[1], Instruction::PrintLine(1));
}

#[test]
fn test_compile_print_multiple_values() {
    let (instrs, _) = compile("印刷（１、２、３）；");
    assert_eq!(instrs[0], Instruction::LoadConst(0));
    assert_eq!(instrs[1], Instruction::LoadConst(1));
    assert_eq!(instrs[2], Instruction::LoadConst(2));
    assert_eq!(instrs[3], Instruction::PrintLine(3));
}

#[test]
fn test_compile_print_no_values() {
    let (instrs, _) = compile("印刷（）；");
    assert_eq!(instrs[0], Instruction::PrintLine(0));
}

// ── 12: fixed-width-field boundary hardening ─────────────────────────

// Convert a non-negative integer to its full-width (ZenKaku) digit form,
// which is what the Hikari lexer accepts.
fn fw(n: usize) -> String {
    n.to_string()
        .chars()
        .map(|c| char::from_u32(c as u32 - '0' as u32 + '０' as u32).unwrap())
        .collect()
}

fn compile_err(src: &str) -> CompileError {
    let ast = Parser::new(Lexer::new(src).tokenize()).parse().unwrap();
    Compiler::new().compile(&ast).unwrap_err()
}

#[test]
fn test_compile_too_many_arguments_is_rejected() {
    // A builtin call with 256 args overflows the u8 arg-count field.
    let args = (0..256).map(fw).collect::<Vec<_>>().join("、");
    let src = format!("要素数（{}）；", args);
    assert_eq!(compile_err(&src), CompileError::TooManyArguments(256));
}

#[test]
fn test_compile_255_arguments_is_accepted() {
    // The boundary value itself must still compile.
    let args = (0..255).map(fw).collect::<Vec<_>>().join("、");
    let src = format!("要素数（{}）；", args);
    let ast = Parser::new(Lexer::new(&src).tokenize()).parse().unwrap();
    assert!(Compiler::new().compile(&ast).is_ok());
}

#[test]
fn test_compile_oversized_chunk_is_rejected() {
    // >65,535 instructions in one chunk overflows the u16 jump/offset fields.
    // 32,768 prints compile to ~65,536 instructions; the repeated literal
    // dedupes to a single constant, so this stays cheap to build/compile.
    let src = "印刷（１）；".repeat(32768);
    assert!(matches!(compile_err(&src), CompileError::ChunkTooLarge(_)));
}
