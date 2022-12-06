//! This module contains unit structs that should be used inside `quote!` and `spanned_quote!` using the variable interpolation syntax in place of their equivalent structs and traits present in `std`.
//
//! To create hygienic proc macros, all the names must be its fully qualified form. These unit structs help us to not specify the fully qualified name every single time.
//!
//! # Example
//! Instead of writing this:
//! ```ignore
//! quote!(
//!     fn get_id() -> Option<i32> {
//!         Some(0)
//!     }
//! )
//! ```
//! Or this:
//! ```ignore
//! quote!(
//!     fn get_id() -> ::core::option::Option<i32> {
//!         ::core::option::Option::Some(0)
//!     }
//! )
//! ```
//! We should write this:
//! ```ignore
//! use crate::fq_std::FQOption;
//!
//! quote!(
//!     fn get_id() -> #FQOption<i32> {
//!         #FQOption::Some(0)
//!     }
//! )
//! ```

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
