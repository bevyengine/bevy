//! This module contains unit structs that should be used inside `quote!` and `spanned_quote!`
//! using the variable interpolation syntax in place of their equivalent structs and traits
//! present in `std`.
//!
//! To create hygienic proc macros, all the names must be its fully qualified form. These
//! unit structs help us to not specify the fully qualified name every single time.
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

/// Fully Qualified (FQ) short name for [`::core::any::Any`]
pub struct FQAny;
/// Fully Qualified (FQ) short name for [`::std::boxed::Box`]
pub struct FQBox;
/// Fully Qualified (FQ) short name for [`::core::clone::Clone`]
pub struct FQClone;
/// Fully Qualified (FQ) short name for [`::core::default::Default`]
pub struct FQDefault;
/// Fully Qualified (FQ) short name for [`::core::option::Option`]
pub struct FQOption;
/// Fully Qualified (FQ) short name for [`::core::result::Result`]
pub struct FQResult;
/// Fully Qualified (FQ) short name for [`::core::marker::Send`]
pub struct FQSend;
/// Fully Qualified (FQ) short name for [`::core::marker::Sync`]
pub struct FQSync;

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

impl ToTokens for FQSend {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        quote!(::core::marker::Send).to_tokens(tokens);
    }
}

impl ToTokens for FQSync {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        quote!(::core::marker::Sync).to_tokens(tokens);
    }
}
