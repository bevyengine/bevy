use proc_macro2::TokenStream;
use quote::{quote, ToTokens};

pub(crate) struct FQAny;
pub(crate) struct FQBox;
pub(crate) struct FQClone;
pub(crate) struct FQDefault;
pub(crate) struct FQOption;
pub(crate) struct FQResult;

impl ToTokens for FQAny {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        quote!(::core::any::Any).to_tokens(tokens);
    }
}

impl ToTokens for FQBox {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        quote!(::std::boxed::Box).to_tokens(tokens);
    }
}

impl ToTokens for FQClone {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        quote!(::core::clone::Clone).to_tokens(tokens);
    }
}

impl ToTokens for FQDefault {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        quote!(::core::default::Default).to_tokens(tokens);
    }
}

impl ToTokens for FQOption {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        quote!(::core::option::Option).to_tokens(tokens);
    }
}

impl ToTokens for FQResult {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        quote!(::core::result::Result).to_tokens(tokens);
    }
}
