use std::{borrow::Cow, str::Lines};

use regex::Regex;

static RE_COMMENT: once_cell::sync::Lazy<Regex> =
    once_cell::sync::Lazy::new(|| Regex::new(r"(//|/\*|\*/)").unwrap());

pub struct CommentReplaceIter<'a> {
    lines: &'a mut Lines<'a>,
    block_depth: usize,
}

impl<'a> Iterator for CommentReplaceIter<'a> {
    type Item = Cow<'a, str>;

    fn next(&mut self) -> Option<Self::Item> {
        let line_in = self.lines.next()?;
        let mut markers = RE_COMMENT
            .captures_iter(line_in)
            .map(|cap| cap.get(0).unwrap())
            .peekable();

        // fast path
        if self.block_depth == 0 && markers.peek().is_none() {
            return Some(Cow::Borrowed(line_in));
        }

        let mut output = String::new();
        let mut section_start = 0;

        loop {
            let mut next_marker = markers.next();
            let mut section_end = next_marker.map(|m| m.start()).unwrap_or(line_in.len());

            // skip partial tokens
            while next_marker.is_some() && section_start > section_end {
                next_marker = markers.next();
                section_end = next_marker.map(|m| m.start()).unwrap_or(line_in.len());
            }

            if self.block_depth == 0 {
                output.push_str(&line_in[section_start..section_end]);
            } else {
                output.extend(std::iter::repeat(' ').take(section_end - section_start));
            }

            match next_marker {
                None => return Some(Cow::Owned(output)),
                Some(marker) => {
                    match marker.as_str() {
                        "//" => {
                            // the specs (https://www.w3.org/TR/WGSL/#comment, https://registry.khronos.org/OpenGL/specs/gl/GLSLangSpec.4.60.pdf @ 3.4) state that
                            // whichever comment-type starts first should cancel parsing of the other type
                            if self.block_depth == 0 {
                                output.extend(
                                    std::iter::repeat(' ').take(line_in.len() - marker.start()),
                                );
                                return Some(Cow::Owned(output));
                            }
                        }
                        "/*" => {
                            self.block_depth += 1;
                        }
                        "*/" => {
                            self.block_depth = self.block_depth.saturating_sub(1);
                        }
                        _ => unreachable!(),
                    }
                    output.extend(std::iter::repeat(' ').take(marker.as_str().len()));
                    section_start = marker.end();
                }
            }
        }
    }
}

pub trait CommentReplaceExt<'a> {
    /// replace WGSL and GLSL comments with whitespace characters
    fn replace_comments(&'a mut self) -> CommentReplaceIter;
}

impl<'a> CommentReplaceExt<'a> for Lines<'a> {
    fn replace_comments(&'a mut self) -> CommentReplaceIter {
        CommentReplaceIter {
            lines: self,
            block_depth: 0,
        }
    }
}

#[test]
fn comment_test() {
    const INPUT: &str = r"
not commented
// line commented
not commented
/* block commented on a line */
not commented
// line comment with a /* block comment unterminated
not commented
/* block comment
   spanning lines */
not commented
/* block comment
   spanning lines and with // line comments
   even with a // line commented terminator */
not commented
";

    assert_eq!(
        INPUT
            .lines()
            .replace_comments()
            .zip(INPUT.lines())
            .find(|(line, original)| {
                (line != "not commented" && !line.chars().all(|c| c == ' '))
                    || line.len() != original.len()
            }),
        None
    );

    const PARTIAL_TESTS: [(&str, &str); 4] = [
        (
            "1.0 /* block comment with a partial line comment on the end *// 2.0",
            "1.0                                                           / 2.0",
        ),
        (
            "1.0 /* block comment with a partial block comment on the end */* 2.0",
            "1.0                                                            * 2.0",
        ),
        (
            "1.0 /* block comment 1 *//* block comment 2 */ * 2.0",
            "1.0                                            * 2.0",
        ),
        (
            "1.0 /* block comment with real line comment after */// line comment",
            "1.0                                                                ",
        ),
    ];

    for &(input, expected) in PARTIAL_TESTS.iter() {
        let mut nasty_processed = input.lines();
        let nasty_processed = nasty_processed.replace_comments().next().unwrap();
        assert_eq!(&nasty_processed, expected);
    }
}
