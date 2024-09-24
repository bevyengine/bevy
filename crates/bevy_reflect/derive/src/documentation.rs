//! Contains code related to documentation reflection (requires the `documentation` feature).

use bevy_macro_utils::fq_std::FQOption;
use proc_macro2::TokenStream;
use quote::{quote, ToTokens};
use syn::{Attribute, Expr, ExprLit, Lit, Meta};

/// A struct used to represent a type's documentation, if any.
///
/// When converted to a [`TokenStream`], this will output an `Option<String>`
/// containing the collection of doc comments.
#[derive(Default, Clone)]
pub(crate) struct Documentation {
    docs: Vec<String>,
}

impl Documentation {
    /// Create a new [`Documentation`] from a type's attributes.
    ///
    /// This will collect all `#[doc = "..."]` attributes, including the ones generated via `///` and `//!`.
    pub fn from_attributes<'a>(attributes: impl IntoIterator<Item = &'a Attribute>) -> Self {
        let docs = attributes
            .into_iter()
            .filter_map(|attr| match &attr.meta {
                Meta::NameValue(pair) if pair.path.is_ident("doc") => {
                    if let Expr::Lit(ExprLit {
                        lit: Lit::Str(lit), ..
                    }) = &pair.value
                    {
                        Some(lit.value())
                    } else {
                        None
                    }
                }
                _ => None,
            })
            .collect();

        Self { docs }
    }

    /// The full docstring, if any.
    pub fn doc_string(&self) -> Option<String> {
        if self.docs.is_empty() {
            return None;
        }

        let len = self.docs.len();
        Some(
            self.docs
                .iter()
                .enumerate()
                .map(|(index, doc)| {
                    if index < len - 1 {
                        format!("{doc}\n")
                    } else {
                        doc.to_owned()
                    }
                })
                .collect(),
        )
    }

    /// Push a new docstring to the collection
    pub fn push(&mut self, doc: String) {
        self.docs.push(doc);
    }
}

impl ToTokens for Documentation {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        if let Some(doc) = self.doc_string() {
            quote!(#FQOption::Some(#doc)).to_tokens(tokens);
        } else {
            quote!(#FQOption::None).to_tokens(tokens);
        }
    }
}
