use std::{borrow::Cow, str::Lines};

use regex::Regex;

// outside of blocks and quotes, change state on //, /* or "
static RE_NONE: once_cell::sync::Lazy<Regex> =
    once_cell::sync::Lazy::new(|| Regex::new(r#"(//|/\*|\")"#).unwrap());
// in blocks, change on /* and */
static RE_BLOCK: once_cell::sync::Lazy<Regex> =
    once_cell::sync::Lazy::new(|| Regex::new(r"(/\*|\*/)").unwrap());
// in quotes, change only on "
static RE_QUOTE: once_cell::sync::Lazy<Regex> =
    once_cell::sync::Lazy::new(|| Regex::new(r#"\""#).unwrap());

#[derive(PartialEq, Eq)]
enum CommentState {
    None,
    Block(usize),
    Quote,
}

pub struct CommentReplaceIter<'a> {
    lines: &'a mut Lines<'a>,
    state: CommentState,
}

impl<'a> Iterator for CommentReplaceIter<'a> {
    type Item = Cow<'a, str>;

    fn next(&mut self) -> Option<Self::Item> {
        let line_in = self.lines.next()?;

        // fast path
        if self.state == CommentState::None && !RE_NONE.is_match(line_in) {
            return Some(Cow::Borrowed(line_in));
        }

        let mut output = String::new();
        let mut section_start = 0;

        loop {
            let marker = match self.state {
                CommentState::None => &RE_NONE,
                CommentState::Block(_) => &RE_BLOCK,
                CommentState::Quote => &RE_QUOTE,
            }
            .find(&line_in[section_start..]);

            let section_end = marker
                .map(|m| section_start + m.start())
                .unwrap_or(line_in.len());

            if let CommentState::Block(_) = self.state {
                output.extend(std::iter::repeat(' ').take(section_end - section_start));
            } else {
                output.push_str(&line_in[section_start..section_end]);
            }

            match marker {
                None => return Some(Cow::Owned(output)),
                Some(marker) => {
                    match marker.as_str() {
                        // only possible in None state
                        "//" => {
                            output.extend(
                                std::iter::repeat(' ')
                                    .take(line_in.len() - marker.start() - section_start),
                            );
                            return Some(Cow::Owned(output));
                        }
                        // only possible in None or Block state
                        "/*" => {
                            self.state = match self.state {
                                CommentState::None => CommentState::Block(1),
                                CommentState::Block(n) => CommentState::Block(n + 1),
                                _ => unreachable!(),
                            };
                            output.push_str("  ");
                        }
                        // only possible in Block state
                        "*/" => {
                            self.state = match self.state {
                                CommentState::Block(1) => CommentState::None,
                                CommentState::Block(n) => CommentState::Block(n - 1),
                                _ => unreachable!(),
                            };
                            output.push_str("  ");
                        }
                        // only possible in None or Quote state
                        "\"" => {
                            self.state = match self.state {
                                CommentState::None => CommentState::Quote,
                                CommentState::Quote => CommentState::None,
                                _ => unreachable!(),
                            };
                            output.push('"');
                        }
                        _ => unreachable!(),
                    }
                    section_start += marker.end();
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
            state: CommentState::None,
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

    const PARTIAL_TESTS: [(&str, &str); 11] = [
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
        ("*/", "*/"),
        (
            r#"#import "embedded://file.wgsl""#,
            r#"#import "embedded://file.wgsl""#,
        ),
        (
            r#"// #import "embedded://file.wgsl""#,
            r#"                                 "#,
        ),
        (
            r#"/* #import "embedded://file.wgsl" */"#,
            r#"                                    "#,
        ),
        (
            r#"/* #import "embedded:*/file.wgsl" */"#,
            r#"                       file.wgsl" */"#,
        ),
        (
            r#"#import "embedded://file.wgsl" // comment"#,
            r#"#import "embedded://file.wgsl"           "#,
        ),
        (
            r#"#import "embedded:/* */ /* /**/* / / /// * / //*/*/ / */*file.wgsl""#,
            r#"#import "embedded:/* */ /* /**/* / / /// * / //*/*/ / */*file.wgsl""#,
        ),
    ];

    for &(input, expected) in PARTIAL_TESTS.iter() {
        let mut nasty_processed = input.lines();
        let nasty_processed = nasty_processed.replace_comments().next().unwrap();
        assert_eq!(&nasty_processed, expected);
    }
}

#[test]
fn multiline_comment_test() {
    let test_cases = [
        (
            // Basic test
            r"/*
hoho
*/",
            r"  
    
  ",
        ),
        (
            // Testing the commenting-out of multiline comments
            r"///*
hehe
//*/",
            r"    
hehe
    ",
        ),
        (
            // Testing the commenting-out of single-line comments
            r"/* // */ code goes here /*
Still a comment // */
/* dummy */",
            r"         code goes here   
                     
           ",
        ),
        (
            // A comment with a nested multiline comment
            // Notice how the "//" inside the multiline comment doesn't take effect
            r"/*
//*
*/commented
*/not commented",
            r"  
   
           
  not commented",
        ),
    ];

    for &(input, expected) in test_cases.iter() {
        for (output_line, expected_line) in input.lines().replace_comments().zip(expected.lines()) {
            assert_eq!(output_line.as_ref(), expected_line);
        }
    }
}

#[test]
fn test_comment_becomes_spaces() {
    let test_cases = [("let a/**/b =3u;", "let a    b =3u;")];
    for &(input, expected) in test_cases.iter() {
        for (output_line, expected_line) in input.lines().replace_comments().zip(expected.lines()) {
            assert_eq!(output_line.as_ref(), expected_line);
        }
    }
}
