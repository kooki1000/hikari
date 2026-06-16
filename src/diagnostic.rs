use crate::lexer::Span;

/// Render a rustc-style diagnostic pointing at `span` within `source`,
/// prefixed by `message`.
pub fn render(source: &str, span: Span, message: &str) -> String {
    let line_text = source
        .lines()
        .nth(span.line.saturating_sub(1))
        .unwrap_or("");
    let line_num_str = span.line.to_string();
    let gutter_width = line_num_str.len();
    let gutter = " ".repeat(gutter_width);

    let underline_offset = span.col.saturating_sub(1);
    let underline_len = span.len.max(1);

    format!(
        "エラー: {message}\n{gutter} --> {line}:{col}\n{gutter} |\n{line_num} | {line_text}\n{gutter} | {pointer_pad}{underline}",
        message = message,
        gutter = gutter,
        line = span.line,
        col = span.col,
        line_num = line_num_str,
        line_text = line_text,
        pointer_pad = " ".repeat(underline_offset),
        underline = "^".repeat(underline_len),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_points_to_correct_line() {
        let source = "整数 年齢 ＝ ２０\n印刷（年齢）；\n返す ０；";
        let span = Span {
            line: 2,
            col: 1,
            len: 2,
        };
        let out = render(source, span, "テストエラー");
        assert!(out.contains("--> 2:1"));
        assert!(out.contains("印刷（年齢）；"));
    }

    #[test]
    fn test_render_underline_position() {
        let source = "ＡＢＣＤＥ";
        let span = Span {
            line: 1,
            col: 3,
            len: 2,
        };
        let out = render(source, span, "下線テスト");
        let lines: Vec<&str> = out.lines().collect();
        let underline_line = lines.last().unwrap();
        // gutter is "1 | " (4 chars) then 2 spaces then "^^"
        assert!(underline_line.ends_with("^^"));
        assert_eq!(underline_line.matches('^').count(), 2);
    }

    #[test]
    fn test_render_includes_message() {
        let source = "Ａ";
        let span = Span {
            line: 1,
            col: 1,
            len: 1,
        };
        let out = render(source, span, "これはメッセージです");
        assert!(out.starts_with("エラー: これはメッセージです"));
    }
}
