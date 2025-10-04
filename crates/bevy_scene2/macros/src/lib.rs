mod bsn;

use proc_macro::TokenStream;

#[proc_macro]
pub fn bsn(input: TokenStream) -> TokenStream {
    crate::bsn::bsn(input)
}

#[proc_macro]
pub fn bsn_list(input: TokenStream) -> TokenStream {
    crate::bsn::bsn_list(input)
}
