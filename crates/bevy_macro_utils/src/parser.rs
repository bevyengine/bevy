use syn::{
    parse::{Parse, ParseStream, Peek},
    punctuated::Punctuated,
};

/// Returns a [`syn::parse::Parser`] which parses a stream of zero or more occurrences of `T`
/// separated by punctuation of type `P`, with optional trailing punctuation.
///
/// This is functionally the same as [`Punctuated::parse_terminated`],
/// but accepts a closure rather than a function pointer.
pub fn terminated_parser<T, P, F: FnMut(ParseStream) -> syn::Result<T>>(
    terminator: P,
    mut parser: F,
) -> impl FnOnce(ParseStream) -> syn::Result<Punctuated<T, P::Token>>
where
    P: Peek,
    P::Token: Parse,
{
    let _ = terminator;
    move |stream: ParseStream| {
        let mut punctuated = Punctuated::new();

        loop {
            if stream.is_empty() {
                break;
            }
            let value = parser(stream)?;
            punctuated.push_value(value);
            if stream.is_empty() {
                break;
            }
            let punct = stream.parse()?;
            punctuated.push_punct(punct);
        }

        Ok(punctuated)
    }
}
