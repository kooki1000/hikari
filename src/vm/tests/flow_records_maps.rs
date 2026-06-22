use super::*;

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

// ── 10a: closures (capture by value) ─────────────────────────────────

#[test]
fn test_vm_lambda_captures_enclosing_local() {
    let src = "整数 ｂ ＝ １０；関数＜（整数） ー＞ 整数＞ ｆ ＝ ｜ｎ：整数｜ ー＞ 整数 ｛ 返す ｎ ＋ ｂ； ｝；返す ｆ（５）；";
    assert_eq!(run(src), Some(Value::Int(15)));
}

#[test]
fn test_vm_closure_captures_by_value_snapshot() {
    // Reassigning the captured variable after the closure is created does not
    // change what the closure sees (capture-by-value).
    let src = "整数 ｃ ＝ １；関数＜（） ー＞ 整数＞ ｇ ＝ ｜｜ ー＞ 整数 ｛ 返す ｃ； ｝；ｃ ＝ ９９；返す ｇ（）；";
    assert_eq!(run(src), Some(Value::Int(1)));
}

#[test]
fn test_vm_closure_used_in_higher_order_function() {
    // A lambda passed to マップ captures an enclosing local.
    let src = "取り込む 「関数」；整数 base ＝ １００；整数列 xs ＝ 【１、２、３】；整数列 ys ＝ マップ（xs、｜ｎ：整数｜ ー＞ 整数 ｛ 返す ｎ ＋ base； ｝）；返す ys【２】；";
    assert_eq!(run(src), Some(Value::Int(103)));
}

#[test]
fn test_vm_nested_lambda_captures_outer_outer_variable() {
    let src = "整数 ａ ＝ ７；関数＜（） ー＞ 整数＞ ｏｕｔｅｒ ＝ ｜｜ ー＞ 整数 ｛ 関数＜（） ー＞ 整数＞ ｉｎｎｅｒ ＝ ｜｜ ー＞ 整数 ｛ 返す ａ； ｝；返す ｉｎｎｅｒ（）； ｝；返す ｏｕｔｅｒ（）；";
    assert_eq!(run(src), Some(Value::Int(7)));
}

#[test]
fn test_vm_closure_captures_array_by_reference() {
    // Arrays have reference semantics, so mutating the captured array through
    // the outer variable is visible inside the closure.
    let src = "取り込む 「配列」；整数列  xs ＝ 【１】；関数＜（） ー＞ 整数＞ ｆ ＝ ｜｜ ー＞ 整数 ｛ 返す 要素数（xs）； ｝；追加（xs、２）；返す ｆ（）；";
    assert_eq!(run(src), Some(Value::Int(2)));
}

#[test]
fn test_vm_multiple_closures_capture_independent_snapshots() {
    let src = "整数 ｘ ＝ ５；関数＜（） ー＞ 整数＞ ａ ＝ ｜｜ ー＞ 整数 ｛ 返す ｘ； ｝；ｘ ＝ ２０；関数＜（） ー＞ 整数＞ ｂ ＝ ｜｜ ー＞ 整数 ｛ 返す ｘ； ｝；返す ａ（） ＋ ｂ（）；";
    // a captured x=5, b captured x=20 → 25
    assert_eq!(run(src), Some(Value::Int(25)));
}

// ── 15a: 省略可＜T＞ — Option type ──────────────────────────────────────

#[test]
fn test_vm_option_aru_match_extracts_value() {
    let src = "省略可＜整数＞ ｖ ＝ 有る（４２）；\
               照合 ｖ ｛\
                 有る（ｎ） ならば ｛ 返す ｎ； ｝\
                 無し（） ならば ｛ 返す ０； ｝\
               ｝";
    assert_eq!(run(src), Some(Value::Int(42)));
}

#[test]
fn test_vm_option_nashi_match_takes_none_arm() {
    let src = "省略可＜整数＞ ｖ ＝ 無し（）；\
               照合 ｖ ｛\
                 有る（ｎ） ならば ｛ 返す ｎ； ｝\
                 無し（） ならば ｛ 返す ー１； ｝\
               ｝";
    assert_eq!(run(src), Some(Value::Int(-1)));
}

#[test]
fn test_vm_option_function_returning_aru() {
    let src = "関数 探す（整数 ｎ）ー＞省略可＜整数＞｛\
                 もし ｎ ＞ ０ ならば ｛ 返す 有る（ｎ）； ｝\
                 違えば ｛ 返す 無し（）； ｝\
               ｝\
               照合 探す（５） ｛\
                 有る（ｖ） ならば ｛ 返す ｖ； ｝\
                 無し（） ならば ｛ 返す ー１； ｝\
               ｝";
    assert_eq!(run(src), Some(Value::Int(5)));
}

#[test]
fn test_vm_option_function_returning_nashi() {
    let src = "関数 探す（整数 ｎ）ー＞省略可＜整数＞｛\
                 もし ｎ ＞ ０ ならば ｛ 返す 有る（ｎ）； ｝\
                 違えば ｛ 返す 無し（）； ｝\
               ｝\
               照合 探す（ー１） ｛\
                 有る（ｖ） ならば ｛ 返す ｖ； ｝\
                 無し（） ならば ｛ 返す ー１； ｝\
               ｝";
    assert_eq!(run(src), Some(Value::Int(-1)));
}

// ── 15b: 取得 and 位置可 ── safe access builtins ─────────────────────

#[test]
fn test_vm_safe_map_get_hit_returns_aru() {
    let src = "取り込む 「辞書」；\
               辞書＜文字列、整数＞ ｍ ＝ ｛「ａ」：１、「ｂ」：２｝；\
               照合 取得（ｍ、「ａ」） ｛\
                 有る（ｖ） ならば ｛ 返す ｖ； ｝\
                 無し（） ならば ｛ 返す ０； ｝\
               ｝";
    assert_eq!(run(src), Some(Value::Int(1)));
}

#[test]
fn test_vm_safe_map_get_miss_returns_nashi() {
    let src = "取り込む 「辞書」；\
               辞書＜文字列、整数＞ ｍ ＝ ｛「ａ」：１｝；\
               照合 取得（ｍ、「ｚ」） ｛\
                 有る（ｖ） ならば ｛ 返す ｖ； ｝\
                 無し（） ならば ｛ 返す ０； ｝\
               ｝";
    assert_eq!(run(src), Some(Value::Int(0)));
}

#[test]
fn test_vm_safe_array_get_in_bounds_returns_aru() {
    let src = "取り込む 「配列」；\
               整数列 ａ ＝ 【１０、２０、３０】；\
               照合 取得（ａ、１） ｛\
                 有る（ｖ） ならば ｛ 返す ｖ； ｝\
                 無し（） ならば ｛ 返す ０； ｝\
               ｝";
    assert_eq!(run(src), Some(Value::Int(20)));
}

#[test]
fn test_vm_safe_array_get_out_of_bounds_returns_nashi() {
    let src = "取り込む 「配列」；\
               整数列 ａ ＝ 【１、２、３】；\
               照合 取得（ａ、９９） ｛\
                 有る（ｖ） ならば ｛ 返す ｖ； ｝\
                 無し（） ならば ｛ 返す ー１； ｝\
               ｝";
    assert_eq!(run(src), Some(Value::Int(-1)));
}

#[test]
fn test_vm_safe_pos_found_returns_aru() {
    let src = "取り込む 「配列」；\
               整数列 ａ ＝ 【１０、２０、３０】；\
               照合 位置可（ａ、２０） ｛\
                 有る（ｉ） ならば ｛ 返す ｉ； ｝\
                 無し（） ならば ｛ 返す ー１； ｝\
               ｝";
    assert_eq!(run(src), Some(Value::Int(1)));
}

#[test]
fn test_vm_safe_pos_not_found_returns_nashi() {
    let src = "取り込む 「配列」；\
               整数列 ａ ＝ 【１、２、３】；\
               照合 位置可（ａ、９９） ｛\
                 有る（ｉ） ならば ｛ 返す ｉ； ｝\
                 無し（） ならば ｛ 返す ー１； ｝\
               ｝";
    assert_eq!(run(src), Some(Value::Int(-1)));
}

// ── 16a: generic function declarations ───────────────────────────────

#[test]
fn test_vm_generic_identity_int() {
    let src = "関数＜Ｔ＞ 恒等（Ｔ ｘ）ー＞Ｔ｛ 返す ｘ； ｝返す 恒等（４２）；";
    assert_eq!(run(src), Some(Value::Int(42)));
}

#[test]
fn test_vm_generic_identity_string() {
    let src = "関数＜Ｔ＞ 恒等（Ｔ ｘ）ー＞Ｔ｛ 返す ｘ； ｝返す 恒等（「こんにちは」）；";
    assert_eq!(run(src), Some(Value::Str("こんにちは".to_string())));
}

#[test]
fn test_vm_generic_identity_bool() {
    let src = "関数＜Ｔ＞ 恒等（Ｔ ｘ）ー＞Ｔ｛ 返す ｘ； ｝返す 恒等（真）；";
    assert_eq!(run(src), Some(Value::Bool(true)));
}

#[test]
fn test_vm_generic_two_params_first() {
    let src = "関数＜Ａ、Ｂ＞ 第一（Ａ ａ、Ｂ ｂ）ー＞Ａ｛ 返す ａ； ｝返す 第一（１０、「文字」）；";
    assert_eq!(run(src), Some(Value::Int(10)));
}

#[test]
fn test_vm_generic_two_params_second() {
    let src = "関数＜Ａ、Ｂ＞ 第二（Ａ ａ、Ｂ ｂ）ー＞Ｂ｛ 返す ｂ； ｝返す 第二（１０、「文字」）；";
    assert_eq!(run(src), Some(Value::Str("文字".to_string())));
}

#[test]
fn test_vm_generic_called_multiple_times_with_different_types() {
    let src = "関数＜Ｔ＞ 恒等（Ｔ ｘ）ー＞Ｔ｛ 返す ｘ； ｝\
               整数 ａ ＝ 恒等（１）；\
               文字列 ｂ ＝ 恒等（「ｈｉ」）；\
               返す ａ；";
    assert_eq!(run(src), Some(Value::Int(1)));
}

#[test]
fn test_vm_generic_array_param_returns_element() {
    let src = "取り込む 「配列」；\
               関数＜Ｔ＞ 先頭（配列＜Ｔ＞ ｌ）ー＞Ｔ｛ 返す ｌ【０】； ｝\
               返す 先頭（【１０、２０、３０】）；";
    assert_eq!(run(src), Some(Value::Int(10)));
}

#[test]
fn test_vm_generic_void_return_runs_side_effect() {
    // Generic parameter, void return — runs side effect, no panic.
    let src = "関数＜Ｔ＞ 表示（Ｔ ｖ）ー＞無｛ 印刷（ｖ）； ｝表示（７）；";
    assert_eq!(run(src), None);
}
