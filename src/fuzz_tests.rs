//! Dependency-free property / fuzz tests for the front end.
//!
//! The goal is to catch the next class of *panics on malformed input* before
//! users do: the lexer, parser, type checker, and compiler must always return
//! a value or an error for ANY input — never panic, index out of bounds, or
//! `unwrap` a `None`. We feed thousands of pseudo-random and hand-picked
//! malformed strings through the pipeline; a panic anywhere fails the test.
//!
//! A small seeded xorshift RNG keeps the corpus deterministic and reproducible
//! without pulling in a fuzzing dependency. The parser caps nesting depth
//! (`MAX_DEPTH`) so even hostile deeply-nested input is rejected with a clean
//! error rather than overflowing the stack — the corpus exercises that path
//! explicitly.

use crate::compiler::Compiler;
use crate::lexer::Lexer;
use crate::parser::Parser;
use crate::typechecker::TypeChecker;

struct XorShift(u64);

impl XorShift {
    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }

    fn below(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }
}

// A grab-bag of characters the lexer and parser actually care about: full-width
// digits/letters, keyword kanji/kana, every operator and bracket, plus some
// ASCII (which is mostly invalid) and whitespace to exercise rejection paths.
const ALPHABET: &[char] = &[
    '０', '１', '２', '９', '．', 'Ａ', 'Ｂ', 'Ｎ', 'ｎ', 'ｘ', 'も', 'し', 'な', 'ら', 'ば', '違',
    'え', '間', '繰', 'り', '返', 'す', '各', '関', '数', '整', '小', '文', '字', '列', '真', '偽',
    '無', '印', '刷', '試', '失', '敗', '構', '造', '照', '合', '型', '取', '込', '抜', '続', 'け',
    'る', '＋', 'ー', '＊', '／', '％', '＝', '＜', '＞', '≦', '≧', '≠', '、', '；', '：', '（',
    '）', '｛', '｝', '【', '】', '「', '」', '｜', '＃', '　', ' ', '\n', 'a', 'z', '#', '1', '@',
];

fn random_source(rng: &mut XorShift) -> String {
    let len = rng.below(48);
    (0..len)
        .map(|_| ALPHABET[rng.below(ALPHABET.len())])
        .collect()
}

// Run the full front end. Any panic here fails the test; a clean Err is fine.
// We stop before the VM (a random program could loop forever or flood output).
fn drive(source: &str) {
    let tokens = Lexer::new(source).tokenize();
    if let Ok(ast) = Parser::new(tokens).parse() {
        if TypeChecker::new().check(&ast).is_ok() {
            let _ = Compiler::new().compile(&ast);
        }
    }
}

#[test]
fn fuzz_pipeline_never_panics_on_random_input() {
    // Several fixed seeds for a broad but reproducible corpus.
    for seed in [0x1234_5678, 0x9E37_79B9, 0xDEAD_BEEF, 0x0BAD_F00D, 0x1] {
        let mut rng = XorShift(seed);
        for _ in 0..5000 {
            drive(&random_source(&mut rng));
        }
    }
}

#[test]
fn fuzz_pipeline_never_panics_on_handpicked_malformed_input() {
    let corpus = [
        "",
        " ",
        "　",
        "\n\n\n",
        "＃",
        "＃ comment with no newline",
        "「",                                           // unterminated string
        "「あ",                                         // unterminated string with content
        "【",                                           // unterminated array literal
        "【１、",                                       // trailing comma, unterminated
        "（（（（（",                                   // unbalanced opening parens
        "）））",                                       // stray closers
        "｛｛｛",                                       // unbalanced braces
        "整数",                                         // type with no name
        "整数 ＝",                                      // missing name, dangling assign
        "整数 ａ ＝",                                   // dangling assign with no value
        "整数 ａ ＝ ；",                                // empty value
        "＋＋＋",                                       // lone operators
        "ー",                                           // lone unary minus / arrow piece
        "ー＞",                                         // bare arrow
        "もし",                                         // keyword with nothing after
        "もし ならば ｛",                               // truncated if
        "関数",                                         // keyword only
        "関数 ｆ（",                                    // truncated fn
        "照合 ｘ ｛",                                   // truncated match
        "構造 ｛ ｝",                                   // empty enum
        "型 Ｔ ｛",                                     // truncated record
        "９９９９９９９９９９９９９９９９９９９９９９", // huge number (overflow path)
        "整数 ａ ＝ ９９９９９９９９９９９９９９９９９９；",
        "印刷（",                     // truncated call
        "印刷（、）",                 // empty arg with comma
        "ａ【",                       // truncated index
        "ａ：：",                     // dangling field access
        "｜｜ ー＞",                  // truncated lambda
        "辞書＜",                     // truncated map type
        "繰り返す から ならば ｛ ｝", // malformed for
        "各 ： ならば ｛ ｝",         // malformed for-each
        "試す 失敗 ｛ ｝",            // malformed try
        "\u{0}\u{1}\u{7f}",           // control characters
    ];
    for src in corpus {
        drive(src);
    }

    // Deeply nested input must be rejected (depth limit), never overflow.
    drive(&format!(
        "整数 ａ ＝ {}１{}；",
        "（".repeat(10000),
        "）".repeat(10000)
    ));
    drive(&"もし 真 ならば ｛".repeat(10000));
    drive(&"【".repeat(10000));
}
