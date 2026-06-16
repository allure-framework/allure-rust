//! Utilities for turning Rust doc attributes into Allure descriptions.

use proc_macro::{Delimiter, TokenTree};

/// Extracts a normalized markdown description from leading Rust doc attributes.
pub(crate) fn description(tokens: &[TokenTree]) -> Option<String> {
    let mut lines = Vec::new();
    let mut idx = 0;

    while idx + 1 < tokens.len() {
        if let Some(line) = doc_attr_at(tokens, idx) {
            lines.push(line);
            idx += 2;
            continue;
        }

        idx += 1;
    }

    description_from_lines(lines)
}

fn description_from_lines(lines: Vec<String>) -> Option<String> {
    let mut lines = lines
        .into_iter()
        .map(|line| normalize_line(&line))
        .collect::<Vec<_>>();

    while lines.first().is_some_and(|line| line.is_empty()) {
        lines.remove(0);
    }
    while lines.last().is_some_and(|line| line.is_empty()) {
        lines.pop();
    }

    if lines.is_empty() {
        None
    } else {
        Some(lines.join("\n"))
    }
}

fn doc_attr_at(tokens: &[TokenTree], index: usize) -> Option<String> {
    let Some(TokenTree::Punct(pound)) = tokens.get(index) else {
        return None;
    };
    if pound.as_char() != '#' {
        return None;
    }

    let Some(TokenTree::Group(group)) = tokens.get(index + 1) else {
        return None;
    };
    if group.delimiter() != Delimiter::Bracket {
        return None;
    }

    let attr_tokens: Vec<TokenTree> = group.stream().into_iter().collect();
    match attr_tokens.as_slice() {
        [TokenTree::Ident(name), TokenTree::Punct(eq), TokenTree::Literal(value)]
            if name.to_string() == "doc" && eq.as_char() == '=' =>
        {
            parse_string_literal(&value.to_string())
        }
        _ => None,
    }
}

fn normalize_line(line: &str) -> String {
    line.strip_prefix(' ').unwrap_or(line).to_string()
}

fn parse_string_literal(raw: &str) -> Option<String> {
    if raw.starts_with('"') && raw.ends_with('"') {
        parse_cooked_string_literal(raw)
    } else if raw.starts_with('r') {
        parse_raw_string_literal(raw)
    } else {
        None
    }
}

fn parse_raw_string_literal(raw: &str) -> Option<String> {
    let mut chars = raw.char_indices();
    let (_, first) = chars.next()?;
    if first != 'r' {
        return None;
    }

    let mut hashes = 0;
    let mut content_start = None;
    for (idx, ch) in chars.by_ref() {
        match ch {
            '#' if content_start.is_none() => hashes += 1,
            '"' if content_start.is_none() => {
                content_start = Some(idx + ch.len_utf8());
                break;
            }
            _ => return None,
        }
    }

    let content_start = content_start?;
    let suffix = format!("\"{}", "#".repeat(hashes));
    if !raw.ends_with(&suffix) || raw.len() < content_start + suffix.len() {
        return None;
    }
    let content_end = raw.len() - suffix.len();
    Some(raw[content_start..content_end].to_string())
}

fn parse_cooked_string_literal(raw: &str) -> Option<String> {
    let mut output = String::new();
    let mut chars = raw[1..raw.len() - 1].chars().peekable();

    while let Some(ch) = chars.next() {
        if ch != '\\' {
            output.push(ch);
            continue;
        }

        match chars.next()? {
            '"' => output.push('"'),
            '\\' => output.push('\\'),
            'n' => output.push('\n'),
            'r' => output.push('\r'),
            't' => output.push('\t'),
            '0' => output.push('\0'),
            'x' => {
                let high = chars.next()?;
                let low = chars.next()?;
                let value = u8::from_str_radix(&format!("{high}{low}"), 16).ok()?;
                output.push(value as char);
            }
            'u' => {
                if chars.next()? != '{' {
                    return None;
                }
                let mut value = String::new();
                let mut closed = false;
                for ch in chars.by_ref() {
                    if ch == '}' {
                        closed = true;
                        break;
                    }
                    if ch != '_' {
                        value.push(ch);
                    }
                }
                if !closed {
                    return None;
                }
                let value = u32::from_str_radix(&value, 16).ok()?;
                output.push(char::from_u32(value)?);
            }
            '\n' => skip_string_continuation_whitespace(&mut chars),
            '\r' => {
                if chars.peek().is_some_and(|ch| *ch == '\n') {
                    chars.next();
                }
                skip_string_continuation_whitespace(&mut chars);
            }
            escaped => output.push(escaped),
        }
    }

    Some(output)
}

fn skip_string_continuation_whitespace<I>(chars: &mut std::iter::Peekable<I>)
where
    I: Iterator<Item = char>,
{
    while chars
        .peek()
        .is_some_and(|ch| matches!(*ch, ' ' | '\t' | '\n' | '\r'))
    {
        chars.next();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use allure_rust_commons::{self as allure, TestOptions};

    fn doc_lines(lines: &[&str]) -> Vec<String> {
        lines.iter().map(|line| (*line).to_string()).collect()
    }

    #[track_caller]
    fn run_doc_parser_test(name: &str, description: &str, body: impl FnOnce()) {
        allure::test_with(
            TestOptions::new(name)
                .with_full_name(format!("allure_test_macros::doc_comments::tests::{name}"))
                .with_description(description)
                .with_source(
                    std::panic::Location::caller().file(),
                    env!("CARGO_MANIFEST_DIR"),
                    module_path!(),
                ),
            || allure::step("verify doc string parser behavior", body),
        );
    }

    #[test]
    fn description_from_lines_removes_standard_doc_comment_padding() {
        run_doc_parser_test(
            "description_from_lines_removes_standard_doc_comment_padding",
            "Verifies standard rustdoc leading spaces are removed from collected doc lines.",
            || {
                assert_eq!(
                    description_from_lines(doc_lines(&[" First line", " Second line"])).as_deref(),
                    Some("First line\nSecond line")
                );
            },
        );
    }

    #[test]
    fn description_from_lines_trims_blank_edges() {
        run_doc_parser_test(
            "description_from_lines_trims_blank_edges",
            "Verifies leading and trailing blank doc comment lines are omitted.",
            || {
                assert_eq!(
                    description_from_lines(doc_lines(&[
                        "",
                        " ",
                        " First line",
                        " Second line",
                        "",
                    ])),
                    Some("First line\nSecond line".to_string())
                );
            },
        );
    }

    #[test]
    fn description_from_lines_returns_none_for_blank_only_lines() {
        run_doc_parser_test(
            "description_from_lines_returns_none_for_blank_only_lines",
            "Verifies blank-only doc comments do not create an empty Allure description.",
            || {
                assert_eq!(description_from_lines(doc_lines(&["", " ", ""])), None);
            },
        );
    }

    #[test]
    fn description_from_lines_preserves_extra_indentation() {
        run_doc_parser_test(
            "description_from_lines_preserves_extra_indentation",
            "Verifies indentation beyond the standard rustdoc padding is preserved.",
            || {
                assert_eq!(
                    description_from_lines(doc_lines(&[
                        " Heading",
                        "  indented block",
                        "    nested block",
                    ]))
                    .as_deref(),
                    Some("Heading\n indented block\n   nested block")
                );
            },
        );
    }

    #[test]
    fn description_from_lines_preserves_interior_blank_lines() {
        run_doc_parser_test(
            "description_from_lines_preserves_interior_blank_lines",
            "Verifies blank lines inside a doc comment remain in the Allure description.",
            || {
                assert_eq!(
                    description_from_lines(doc_lines(&[" First line", " ", " Second line"]))
                        .as_deref(),
                    Some("First line\n\nSecond line")
                );
            },
        );
    }

    #[test]
    fn description_from_lines_keeps_unpadded_lines() {
        run_doc_parser_test(
            "description_from_lines_keeps_unpadded_lines",
            "Verifies doc lines without standard rustdoc padding are left unchanged.",
            || {
                assert_eq!(
                    description_from_lines(doc_lines(&["First line", " Second line"])).as_deref(),
                    Some("First line\nSecond line")
                );
            },
        );
    }

    #[test]
    fn parse_string_literal_decodes_cooked_string_escapes() {
        run_doc_parser_test(
            "parse_string_literal_decodes_cooked_string_escapes",
            "Verifies cooked doc string literals decode common Rust escape sequences.",
            || {
                assert_eq!(
                    parse_string_literal("\"line\\n\\\"quoted\\\"\\\\slash\\t\"").as_deref(),
                    Some("line\n\"quoted\"\\slash\t")
                );
            },
        );
    }

    #[test]
    fn parse_string_literal_decodes_hex_and_unicode_escapes() {
        run_doc_parser_test(
            "parse_string_literal_decodes_hex_and_unicode_escapes",
            "Verifies cooked doc string literals decode hex and Unicode escapes.",
            || {
                assert_eq!(
                    parse_string_literal("\"\\x41\\u{42}\"").as_deref(),
                    Some("AB")
                );
            },
        );
    }

    #[test]
    fn parse_string_literal_decodes_control_escapes() {
        run_doc_parser_test(
            "parse_string_literal_decodes_control_escapes",
            "Verifies cooked doc string literals decode carriage return and nul escapes.",
            || {
                assert_eq!(
                    parse_string_literal("\"a\\r\\0b\""),
                    Some("a\r\0b".to_string())
                );
            },
        );
    }

    #[test]
    fn parse_string_literal_decodes_unicode_escape_with_underscores() {
        run_doc_parser_test(
            "parse_string_literal_decodes_unicode_escape_with_underscores",
            "Verifies cooked Unicode escapes may contain Rust-style underscore separators.",
            || {
                assert_eq!(parse_string_literal("\"\\u{2_1}\"").as_deref(), Some("!"));
            },
        );
    }

    #[test]
    fn parse_string_literal_skips_cooked_string_continuation_whitespace() {
        run_doc_parser_test(
            "parse_string_literal_skips_cooked_string_continuation_whitespace",
            "Verifies Rust string continuation whitespace is omitted from cooked doc strings.",
            || {
                let literal = "\"first\\
        second\"";

                assert_eq!(
                    parse_string_literal(literal).as_deref(),
                    Some("firstsecond")
                );
            },
        );
    }

    #[test]
    fn parse_string_literal_preserves_raw_string_content() {
        run_doc_parser_test(
            "parse_string_literal_preserves_raw_string_content",
            "Verifies raw doc string literals preserve backslashes and quote-like content.",
            || {
                let literal = r##"r#"raw \n "# text"#"##;

                assert_eq!(
                    parse_string_literal(literal).as_deref(),
                    Some(r##"raw \n "# text"##)
                );
            },
        );
    }

    #[test]
    fn parse_string_literal_rejects_unsupported_or_invalid_literals() {
        run_doc_parser_test(
            "parse_string_literal_rejects_unsupported_or_invalid_literals",
            "Verifies unsupported or malformed doc string literals are ignored.",
            || {
                assert_eq!(parse_string_literal("b\"bytes\""), None);
                assert_eq!(parse_string_literal("\"\\xGG\""), None);
                assert_eq!(parse_string_literal("\"\\u{41\""), None);
                assert_eq!(parse_string_literal(r##"r#"missing terminator"##), None);
            },
        );
    }
}
