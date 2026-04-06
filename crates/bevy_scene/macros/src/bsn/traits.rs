use proc_macro2::TokenStream;
use syn::parse::Parse;

use crate::bsn::codegen::BsnCodegenCtx;

pub trait BsnTokenStream: Parse {
    fn to_tokens(&self, ctx: &mut BsnCodegenCtx) -> TokenStream;
}
