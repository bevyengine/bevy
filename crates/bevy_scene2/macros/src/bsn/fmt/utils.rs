// TODO encapsulate/clean these up where it makes sense.
// Maybe use a config or rustfmt.toml

use proc_macro2::TokenStream;
use quote::ToTokens;

pub(crate) fn indent(base: usize, level: usize) -> String {
    " ".repeat(base + (level * 4))
}

pub(crate) fn fmt_quote<T: ToTokens>(item: &T) -> String {
    fmt_tokens(&item.to_token_stream())
}

pub(crate) fn fmt_tokens(tokens: &TokenStream) -> String {
    tokens
        .to_string()
        .replace("( ", "(")
        .replace(" )", ")")
        .replace(" (", "(")
        .replace(" :: ", "::")
        .replace(" ,", ",")
        .replace(" ()", "()")
        .replace("! ", "!")
        .replace(" !(", "!(")
        .replace(" ;", ";")
        .replace("new (", "new(")
}

pub(crate) fn fmt_list_with<T, F>(
    items: &[T],
    base: usize,
    level: usize,
    separator: &str,
    mut format_item: F,
) -> String
where
    F: FnMut(&T, usize, usize) -> String,
{
    items.iter().fold(String::new(), |mut out, item| {
        out.push_str(&indent(base, level));
        out.push_str(&format_item(item, base, level));
        out.push_str(separator);
        out
    })
}

/*
* TODO comments are ignored for now since we are using `syn` but we could still extract them and
* format them separately
pub fn extract_comments(source: &str, start: usize, end: usize) -> Vec<String> {
    if start >= end || end > source.len() {
        return Vec::new();
    }

    source[start..end]
        .lines()
        .map(|l| l.trim())
        .filter(|l| l.starts_with("//"))
        .map(|l| l.to_string())
        .collect()
}
*/

fn fallback(tokens: &TokenStream, base: usize, level: usize) -> String {
    let raw = fmt_tokens(tokens);
    format!("{}{}", indent(base, level), raw)
}

/// Formats standard Rust code (expressions, closures) by wrapping them in a dummy
/// AST, formatting with prettyplease, and then stripping the wrapper.
pub(crate) fn fmt_rust_expr(tokens: &TokenStream, base: usize, level: usize) -> String {
    let wrapped = quote::quote! {
        fn __dummy() {
            #tokens
        }
    };

    let Ok(file) = syn::parse2::<syn::File>(wrapped) else {
        return fallback(tokens, base, level);
    };

    let formatted = prettyplease::unparse(&file);
    let mut lines: Vec<&str> = formatted.lines().collect();
    if lines.len() < 2 {
        return fallback(tokens, base, level);
    }

    // Remove `fn __dummy() {` and `}`
    lines.remove(0);
    lines.pop();
    lines
        .iter()
        .enumerate()
        .fold(String::new(), |mut out, (i, line)| {
            if i > 0 {
                out.push('\n');
            }

            // prettyplease indents the body by 4 spaces. Strip that.
            let trimmed = line.strip_prefix("    ").unwrap_or(line);
            if !trimmed.is_empty() {
                out.push_str(&indent(base, level));
                out.push_str(trimmed);
            }

            out
        })
}

pub fn col_to_offset(source: &str, target_line: usize, target_col: usize) -> usize {
    let mut line = 1;
    let mut col = 0;

    for (i, c) in source.char_indices() {
        if line == target_line && col == target_col {
            return i;
        }

        if c == '\n' {
            line += 1;
            col = 0;
        } else {
            col += 1;
        }
    }

    source.len()
}

pub fn offset_to_col(source: &str, offset: usize) -> (usize, usize) {
    let mut line = 1;
    let mut col = 0;

    for (i, c) in source.char_indices() {
        if i >= offset {
            break;
        }

        if c == '\n' {
            line += 1;
            col = 0;
        } else {
            col += 1;
        }
    }

    (line, col)
}
