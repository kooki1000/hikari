use super::*;

#[test]
fn test_typecheck_math_builtins_after_import() {
    let src = "取り込む 「数学」；整数 結果 ＝ 絶対値（ー５）；";
    let ast = parse(src);
    assert!(TypeChecker::new().check(&ast).is_ok());

    let src = "取り込む 「数学」；小数 結果 ＝ 平方根（９）；";
    let ast = parse(src);
    assert!(TypeChecker::new().check(&ast).is_ok());

    let src = "取り込む 「数学」；整数 結果 ＝ 乱数（１、１０）；";
    let ast = parse(src);
    assert!(TypeChecker::new().check(&ast).is_ok());

    let src = "取り込む 「数学」；整数 結果 ＝ 最大（１、２）；";
    let ast = parse(src);
    assert!(TypeChecker::new().check(&ast).is_ok());

    let src = "取り込む 「数学」；小数 結果 ＝ 最小（１．０、２．０）；";
    let ast = parse(src);
    assert!(TypeChecker::new().check(&ast).is_ok());
}

#[test]
fn test_typecheck_string_builtins_after_import() {
    let src = "取り込む 「文字列」；文字列列 結果 ＝ 分割（「あ、い」、「、」）；";
    let ast = parse(src);
    assert!(TypeChecker::new().check(&ast).is_ok());

    let src = "取り込む 「文字列」；文字列 結果 ＝ 結合（【「あ」、「い」】、「、」）；";
    let ast = parse(src);
    assert!(TypeChecker::new().check(&ast).is_ok());

    let src = "取り込む 「文字列」；真偽 結果 ＝ 含む（「あいう」、「い」）；";
    let ast = parse(src);
    assert!(TypeChecker::new().check(&ast).is_ok());

    let src = "取り込む 「文字列」；文字列 結果 ＝ 置換（「あいう」、「い」、「え」）；";
    let ast = parse(src);
    assert!(TypeChecker::new().check(&ast).is_ok());
}

#[test]
fn test_typecheck_stdlib_builtin_without_import_fails() {
    let src = "整数 結果 ＝ 絶対値（ー５）；";
    let ast = parse(src);
    let err = TypeChecker::new().check(&ast).unwrap_err();
    assert!(matches!(
        err,
        TypeError::ModuleNotImported { module, .. } if module == "数学"
    ));

    let src = "真偽 結果 ＝ 含む（「あ」、「い」）；";
    let ast = parse(src);
    let err = TypeChecker::new().check(&ast).unwrap_err();
    assert!(matches!(
        err,
        TypeError::ModuleNotImported { module, .. } if module == "文字列"
    ));
}

#[test]
fn test_typecheck_abs_sqrt_polymorphic_mismatch() {
    let src = "取り込む 「数学」；整数 結果 ＝ 絶対値（「文字」）；";
    let ast = parse(src);
    let err = TypeChecker::new().check(&ast).unwrap_err();
    assert!(matches!(err, TypeError::ArgTypeMismatch { .. }));

    let src = "取り込む 「数学」；小数 結果 ＝ 平方根（「文字」）；";
    let ast = parse(src);
    let err = TypeChecker::new().check(&ast).unwrap_err();
    assert!(matches!(err, TypeError::ArgTypeMismatch { .. }));
}

#[test]
fn test_typecheck_max_min_polymorphic_mismatch() {
    let src = "取り込む 「数学」；整数 結果 ＝ 最大（１、「あ」）；";
    let ast = parse(src);
    let err = TypeChecker::new().check(&ast).unwrap_err();
    assert!(matches!(err, TypeError::ArgTypeMismatch { .. }));

    let src = "取り込む 「数学」；文字列 結果 ＝ 最小（「あ」、「い」）；";
    let ast = parse(src);
    let err = TypeChecker::new().check(&ast).unwrap_err();
    assert!(matches!(err, TypeError::ArgTypeMismatch { .. }));
}

#[test]
fn test_typecheck_arithmetic_on_bool_is_rejected() {
    // 真 ＋ 偽 has matching operand types but ＋ is undefined for 真偽.
    let ast = parse("真偽 結果 ＝ 真 ＋ 偽；");
    let err = TypeChecker::new().check(&ast).unwrap_err();
    assert!(matches!(err, TypeError::BinOpMismatch { .. }));
}

#[test]
fn test_typecheck_subtraction_on_strings_is_rejected() {
    // ＋ concatenates strings, but ー/＊/／ are numbers-only.
    let ast = parse("文字列 結果 ＝ 「あ」 ー 「い」；");
    let err = TypeChecker::new().check(&ast).unwrap_err();
    assert!(matches!(err, TypeError::BinOpMismatch { .. }));
}

#[test]
fn test_typecheck_ordering_on_strings_is_rejected() {
    // ＜/＞/≦/≧ are only defined for numbers.
    let ast = parse("真偽 結果 ＝ 「あ」 ＜ 「い」；");
    let err = TypeChecker::new().check(&ast).unwrap_err();
    assert!(matches!(err, TypeError::BinOpMismatch { .. }));
}

#[test]
fn test_typecheck_equality_on_strings_is_allowed() {
    // ＝＝/≠ remain valid for any two values of the same type.
    let ast = parse("真偽 結果 ＝ 「あ」 ＝＝ 「い」；");
    assert!(TypeChecker::new().check(&ast).is_ok());
}

#[test]
fn test_typecheck_string_concatenation_still_allowed() {
    let ast = parse("文字列 結果 ＝ 「あ」 ＋ 「い」；");
    assert!(TypeChecker::new().check(&ast).is_ok());
}

// ── 7a: modulo ───────────────────────────────────────────────────────

#[test]
fn test_typecheck_modulo_numeric_only() {
    let ast = parse("整数 結果 ＝ １０ ％ ３；");
    assert!(TypeChecker::new().check(&ast).is_ok());

    let ast = parse("文字列 結果 ＝ 「あ」 ％ 「い」；");
    let err = TypeChecker::new().check(&ast).unwrap_err();
    assert!(matches!(err, TypeError::BinOpMismatch { .. }));
}

// ── 7b: array builtins ──────────────────────────────────────────────

#[test]
fn test_typecheck_array_builtins_require_import() {
    let ast = parse("整数列 数字 ＝ 【１】；整数 結果 ＝ 要素数（数字）；");
    let err = TypeChecker::new().check(&ast).unwrap_err();
    assert!(matches!(
        err,
        TypeError::ModuleNotImported { module, .. } if module == "配列"
    ));
}

#[test]
fn test_typecheck_array_len_happy_and_mismatch() {
    let ast = parse("取り込む 「配列」；整数列 数字 ＝ 【１】；整数 結果 ＝ 要素数（数字）；");
    assert!(TypeChecker::new().check(&ast).is_ok());

    let ast = parse("取り込む 「配列」；整数 結果 ＝ 要素数（５）；");
    let err = TypeChecker::new().check(&ast).unwrap_err();
    assert!(matches!(err, TypeError::ArgTypeMismatch { .. }));
}

#[test]
fn test_typecheck_push_happy_and_mismatch() {
    let ast = parse("取り込む 「配列」；整数列 数字 ＝ 【１】；追加（数字、２）；");
    assert!(TypeChecker::new().check(&ast).is_ok());

    let ast = parse("取り込む 「配列」；整数列 数字 ＝ 【１】；追加（数字、「あ」）；");
    let err = TypeChecker::new().check(&ast).unwrap_err();
    assert!(matches!(err, TypeError::ArgTypeMismatch { .. }));
}

#[test]
fn test_typecheck_pop_happy_and_mismatch() {
    let ast = parse("取り込む 「配列」；整数列 数字 ＝ 【１】；整数 結果 ＝ 取り出す（数字）；");
    assert!(TypeChecker::new().check(&ast).is_ok());

    let ast = parse("取り込む 「配列」；整数 結果 ＝ 取り出す（５）；");
    let err = TypeChecker::new().check(&ast).unwrap_err();
    assert!(matches!(err, TypeError::ArgTypeMismatch { .. }));
}

#[test]
fn test_typecheck_contains_array_and_index_of_happy_and_mismatch() {
    let ast =
        parse("取り込む 「配列」；整数列 数字 ＝ 【１】；真偽 結果 ＝ 含む配列（数字、１）；");
    assert!(TypeChecker::new().check(&ast).is_ok());

    let ast = parse("取り込む 「配列」；整数列 数字 ＝ 【１】；整数 結果 ＝ 位置（数字、１）；");
    assert!(TypeChecker::new().check(&ast).is_ok());

    let ast =
        parse("取り込む 「配列」；整数列 数字 ＝ 【１】；真偽 結果 ＝ 含む配列（数字、「あ」）；");
    let err = TypeChecker::new().check(&ast).unwrap_err();
    assert!(matches!(err, TypeError::ArgTypeMismatch { .. }));
}

#[test]
fn test_typecheck_reverse_happy_and_mismatch() {
    let ast = parse("取り込む 「配列」；整数列 数字 ＝ 【１】；整数列 結果 ＝ 逆順（数字）；");
    assert!(TypeChecker::new().check(&ast).is_ok());

    let ast = parse("取り込む 「配列」；整数 結果 ＝ 逆順（５）；");
    let err = TypeChecker::new().check(&ast).unwrap_err();
    assert!(matches!(err, TypeError::ArgTypeMismatch { .. }));
}

#[test]
fn test_typecheck_sort_happy_and_rejects_bool_array() {
    let ast = parse("取り込む 「配列」；整数列 数字 ＝ 【１】；整列（数字）；");
    assert!(TypeChecker::new().check(&ast).is_ok());

    let ast = parse("取り込む 「配列」；真偽列 旗 ＝ 【真】；整列（旗）；");
    let err = TypeChecker::new().check(&ast).unwrap_err();
    assert!(matches!(err, TypeError::ArgTypeMismatch { .. }));
}

#[test]
fn test_typecheck_slice_happy_and_mismatch() {
    let ast = parse(
        "取り込む 「配列」；整数列 数字 ＝ 【１、２】；整数列 結果 ＝ 部分列（数字、０、１）；",
    );
    assert!(TypeChecker::new().check(&ast).is_ok());

    let ast = parse(
        "取り込む 「配列」；整数列 数字 ＝ 【１】；整数列 結果 ＝ 部分列（数字、「あ」、１）；",
    );
    let err = TypeChecker::new().check(&ast).unwrap_err();
    assert!(matches!(err, TypeError::ArgTypeMismatch { .. }));
}

#[test]
fn test_typecheck_new_array_expr() {
    let ast = parse("整数列 数字 ＝ 新配列＜整数＞；");
    assert!(TypeChecker::new().check(&ast).is_ok());

    let ast = parse("小数列 数字 ＝ 新配列＜小数＞；");
    assert!(TypeChecker::new().check(&ast).is_ok());

    let ast = parse("文字列列 文字 ＝ 新配列＜文字列＞；");
    assert!(TypeChecker::new().check(&ast).is_ok());

    let ast = parse("真偽列 旗 ＝ 新配列＜真偽＞；");
    assert!(TypeChecker::new().check(&ast).is_ok());
}

// ── 7c: more math builtins ─────────────────────────────────────────

#[test]
fn test_typecheck_pow_happy_and_mismatch() {
    let ast = parse("取り込む 「数学」；整数 結果 ＝ 累乗（２、３）；");
    assert!(TypeChecker::new().check(&ast).is_ok());

    let ast = parse("取り込む 「数学」；整数 結果 ＝ 累乗（２、「あ」）；");
    let err = TypeChecker::new().check(&ast).unwrap_err();
    assert!(matches!(err, TypeError::ArgTypeMismatch { .. }));
}

#[test]
fn test_typecheck_floor_ceil_round_happy_and_reject_int() {
    let ast = parse("取り込む 「数学」；整数 結果 ＝ 切り捨て（３．５）；");
    assert!(TypeChecker::new().check(&ast).is_ok());

    let ast = parse("取り込む 「数学」；整数 結果 ＝ 切り上げ（３．５）；");
    assert!(TypeChecker::new().check(&ast).is_ok());

    let ast = parse("取り込む 「数学」；整数 結果 ＝ 四捨五入（３．５）；");
    assert!(TypeChecker::new().check(&ast).is_ok());

    let ast = parse("取り込む 「数学」；整数 結果 ＝ 切り捨て（３）；");
    let err = TypeChecker::new().check(&ast).unwrap_err();
    assert!(matches!(err, TypeError::ArgTypeMismatch { .. }));
}

#[test]
fn test_typecheck_remainder_function_form() {
    let ast = parse("取り込む 「数学」；整数 結果 ＝ 余り（１０、３）；");
    assert!(TypeChecker::new().check(&ast).is_ok());

    let ast = parse("取り込む 「数学」；整数 結果 ＝ 余り（１０、「あ」）；");
    let err = TypeChecker::new().check(&ast).unwrap_err();
    assert!(matches!(err, TypeError::ArgTypeMismatch { .. }));
}

#[test]
fn test_typecheck_math_builtins_without_import_fails() {
    let ast = parse("整数 結果 ＝ 累乗（２、３）；");
    let err = TypeChecker::new().check(&ast).unwrap_err();
    assert!(matches!(
        err,
        TypeError::ModuleNotImported { module, .. } if module == "数学"
    ));
}

// ── 11a: I/O module (入出力) ─────────────────────────────────────────

#[test]
fn test_typecheck_io_builtins_after_import() {
    let src = "取り込む 「入出力」；文字列 内容 ＝ ファイル読む（「a.txt」）；ファイル書く（「b.txt」、内容）；印字（内容）；";
    let ast = parse(src);
    assert!(TypeChecker::new().check(&ast).is_ok());
}

#[test]
fn test_typecheck_io_builtins_without_import_fails() {
    let ast = parse("文字列 内容 ＝ ファイル読む（「a.txt」）；");
    let err = TypeChecker::new().check(&ast).unwrap_err();
    assert!(matches!(
        err,
        TypeError::ModuleNotImported { module, .. } if module == "入出力"
    ));
}

#[test]
fn test_typecheck_file_write_result_is_void() {
    // ファイル書く returns 無, so its result cannot be used as a value (here,
    // bound to a typed var): a 無 value in value position is a TypeError.
    let ast = parse("取り込む 「入出力」；整数 x ＝ ファイル書く（「a.txt」、「データ」）；");
    let err = TypeChecker::new().check(&ast).unwrap_err();
    assert!(matches!(err, TypeError::VoidValueUsed { .. }));
}

// ── 11c: environment module (環境) ───────────────────────────────────

#[test]
fn test_typecheck_env_builtins_after_import() {
    let src = "取り込む 「環境」；文字列列 ａ ＝ 引数（）；文字列 ｐ ＝ 環境変数（「PATH」）；";
    let ast = parse(src);
    assert!(TypeChecker::new().check(&ast).is_ok());
}

#[test]
fn test_typecheck_program_args_without_import_fails() {
    let ast = parse("文字列列 ａ ＝ 引数（）；");
    let err = TypeChecker::new().check(&ast).unwrap_err();
    assert!(matches!(
        err,
        TypeError::ModuleNotImported { module, .. } if module == "環境"
    ));
}

#[test]
fn test_typecheck_env_var_returns_string() {
    // 環境変数 returns 文字列; binding it to an 整数 must fail.
    let ast = parse("取り込む 「環境」；整数 x ＝ 環境変数（「PATH」）；");
    let err = TypeChecker::new().check(&ast).unwrap_err();
    assert!(matches!(
        err,
        TypeError::VarDeclMismatch {
            got: HikariType::String,
            ..
        }
    ));
}

// ── 10b: generic builtin signatures (parametric polymorphism) ────────

#[test]
fn test_typecheck_generic_length_over_any_element_type() {
    // 要素数 works on 配列＜Ｔ＞ for any Ｔ.
    for decl in [
        "整数列 ａ ＝ 【１】",
        "文字列列 ａ ＝ 【「x」】",
        "真偽列 ａ ＝ 【真】",
    ] {
        let src = format!("取り込む 「配列」；{}；整数 ｎ ＝ 要素数（ａ）；", decl);
        assert!(TypeChecker::new().check(&parse(&src)).is_ok(), "{}", decl);
    }
}

#[test]
fn test_typecheck_generic_pop_returns_element_type() {
    // 取り出す（配列＜Ｔ＞）→ Ｔ: popping a 文字列列 yields a 文字列.
    let src = "取り込む 「配列」；文字列列 ａ ＝ 【「x」】；文字列 ｓ ＝ 取り出す（ａ）；";
    assert!(TypeChecker::new().check(&parse(src)).is_ok());
    // Binding the popped 文字列 to an 整数 must fail.
    let src = "取り込む 「配列」；文字列列 ａ ＝ 【「x」】；整数 ｓ ＝ 取り出す（ａ）；";
    assert!(matches!(
        TypeChecker::new().check(&parse(src)).unwrap_err(),
        TypeError::VarDeclMismatch { .. }
    ));
}

#[test]
fn test_typecheck_generic_map_transforms_element_type() {
    // マップ（配列＜Ｔ＞、Ｔ→Ｕ）→ 配列＜Ｕ＞: 整数列 → 文字列列.
    let src = "取り込む 「関数」；整数列 ｎ ＝ 【１】；文字列列 ｓ ＝ マップ（ｎ、｜ｘ：整数｜ ー＞ 文字列 ｛ 返す 「a」； ｝）；";
    assert!(TypeChecker::new().check(&parse(src)).is_ok());
}

#[test]
fn test_typecheck_generic_unify_conflict_is_rejected() {
    // 追加（整数列、文字列）: the element variable Ｔ is bound to 整数 by the
    // array, so a 文字列 second argument conflicts.
    let src = "取り込む 「配列」；整数列 ａ ＝ 【１】；追加（ａ、「x」）；";
    let err = TypeChecker::new().check(&parse(src)).unwrap_err();
    assert!(matches!(
        err,
        TypeError::ArgTypeMismatch {
            param: HikariType::Int,
            got: HikariType::String,
            ..
        }
    ));
}

#[test]
fn test_typecheck_generic_non_array_argument_is_rejected() {
    let src = "取り込む 「配列」；整数 ｎ ＝ 要素数（５）；";
    let err = TypeChecker::new().check(&parse(src)).unwrap_err();
    assert!(matches!(
        err,
        TypeError::ArgTypeMismatch {
            got: HikariType::Int,
            ..
        }
    ));
}

#[test]
fn test_typecheck_generic_fold_threads_accumulator_type() {
    // 畳み込み（整数列、文字列、（文字列、整数）→文字列）→ 文字列.
    let src = "取り込む 「関数」；整数列 ｎ ＝ 【１】；文字列 ｓ ＝ 畳み込み（ｎ、「」、｜ａ：文字列、ｘ：整数｜ ー＞ 文字列 ｛ 返す ａ； ｝）；";
    assert!(TypeChecker::new().check(&parse(src)).is_ok());
}

// ── Phase 23b: dedup, chunk, fold-right, string ops ───────────────────

#[test]
fn test_typecheck_dedup_ok() {
    let src = "取り込む 「配列」；整数列 ａ ＝ 【１、２、１】；整数列 ｂ ＝ 重複除去（ａ）；";
    assert!(TypeChecker::new().check(&parse(src)).is_ok());
}

#[test]
fn test_typecheck_chunk_ok() {
    let src =
        "取り込む 「配列」；整数列 ａ ＝ 【１、２、３】；配列＜整数列＞ ｂ ＝ 分割列（ａ、２）；";
    assert!(TypeChecker::new().check(&parse(src)).is_ok());
}

#[test]
fn test_typecheck_fold_right_ok() {
    let src = "取り込む 「関数」；整数列 ａ ＝ 【１、２、３】；整数 ｒ ＝ 畳み込み右（ａ、０、｜ｘ：整数、ｙ：整数｜ ー＞ 整数 ｛ 返す ｘ ＋ ｙ； ｝）；";
    assert!(TypeChecker::new().check(&parse(src)).is_ok());
}

#[test]
fn test_typecheck_pad_left_ok() {
    let src = r#"取り込む 「文字列」；文字列 ｓ ＝ 左詰め（「hi」、５）；"#;
    assert!(TypeChecker::new().check(&parse(src)).is_ok());
}

#[test]
fn test_typecheck_pad_right_ok() {
    let src = r#"取り込む 「文字列」；文字列 ｓ ＝ 右詰め（「hi」、５）；"#;
    assert!(TypeChecker::new().check(&parse(src)).is_ok());
}

#[test]
fn test_typecheck_format_radix_ok() {
    let src = r#"取り込む 「文字列」；文字列 ｓ ＝ 基数変換（２５５、１６）；"#;
    assert!(TypeChecker::new().check(&parse(src)).is_ok());
}

// ── Phase 23d: I/O builtins ───────────────────────────────────────────

#[test]
fn test_typecheck_print_stderr_ok() {
    let src = r#"取り込む 「入出力」；エラー印刷（「エラー」）；"#;
    assert!(TypeChecker::new().check(&parse(src)).is_ok());
}

#[test]
fn test_typecheck_print_stderr_no_newline_ok() {
    let src = r#"取り込む 「入出力」；エラー印字（「エラー」）；"#;
    assert!(TypeChecker::new().check(&parse(src)).is_ok());
}

#[test]
fn test_typecheck_exit_ok() {
    let src = r#"取り込む 「入出力」；終了（０）；"#;
    assert!(TypeChecker::new().check(&parse(src)).is_ok());
}

#[test]
fn test_typecheck_read_all_input_ok() {
    let src = r#"取り込む 「入出力」；文字列列 行列 ＝ すべて入力（）；"#;
    assert!(TypeChecker::new().check(&parse(src)).is_ok());
}

#[test]
fn test_typecheck_dedup_requires_array_module() {
    let src = "整数列 ａ ＝ 【１、２】；整数列 ｂ ＝ 重複除去（ａ）；";
    let err = TypeChecker::new().check(&parse(src)).unwrap_err();
    assert!(matches!(err, TypeError::ModuleNotImported { .. }));
}

#[test]
fn test_typecheck_exit_requires_io_module() {
    let src = "終了（０）；";
    let err = TypeChecker::new().check(&parse(src)).unwrap_err();
    assert!(matches!(err, TypeError::ModuleNotImported { .. }));
}

// ── Phase 24b: assertions ─────────────────────────────────────────────

#[test]
fn test_typecheck_assert_ungated_ok() {
    let src = "確認（１ ＝＝ １）；";
    assert!(TypeChecker::new().check(&parse(src)).is_ok());
}

#[test]
fn test_typecheck_assert_requires_bool_arg() {
    let src = "確認（５）；";
    let err = TypeChecker::new().check(&parse(src)).unwrap_err();
    assert!(matches!(err, TypeError::ArgTypeMismatch { .. }));
}
