use syn::Path;

/// This formats the given `path` in standard Rust format.
///
/// ## Why does this exist?
///
/// [`Path`] does not include a `to_string()` function or a [`Debug`] impl. Hacks like
/// `path.to_token_stream().to_string()` exist, but they produce ugly spaces between the `::` separators.
pub fn path_to_string(path: &Path) -> String {
    path.segments
        .iter()
        .map(|seg| seg.ident.to_string())
        .collect::<Vec<String>>()
        .join("::")
}
