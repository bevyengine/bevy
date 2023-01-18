#![forbid(unsafe_code)]
#![warn(missing_docs)]

use proc_macro::TokenStream;

mod iterable_enum;
mod paths;

#[proc_macro_derive(IterableEnum)]
pub fn iterable_enum_derive(input: TokenStream) -> TokenStream {
    iterable_enum::parse_iterable_enum_derive(input)
}
