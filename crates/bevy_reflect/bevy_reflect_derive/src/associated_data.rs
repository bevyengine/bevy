use crate::field_attributes::{DefaultBehavior, ReflectFieldAttr, ReflectIgnoreBehavior};
use crate::utility::ident_or_index;
use bevy_macro_utils::fq_std::{FQBox, FQDefault};
use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, ToTokens};
use syn::{parse_quote, Field, ItemFn, Path};

/// Associated data to generate alongside derived trait implementations.
///
/// It's important these are generated within the context of an [_unnamed const_]
/// in order to avoid conflicts and keep the macro hygenic.
///
/// [_unnamed const_]: https://doc.rust-lang.org/stable/reference/items/constant-items.html#unnamed-constant
#[derive(Clone)]
pub(crate) struct AssociatedData {
    default_fn: Option<ItemFn>,
}

impl AssociatedData {
    /// Generates a new `AssociatedData` for a given field.
    pub fn new(
        field: &Field,
        index: usize,
        attrs: &ReflectFieldAttr,
        qualifier: &Ident,
        bevy_reflect_path: &Path,
    ) -> Self {
        let field_ident = ident_or_index(field.ident.as_ref(), index);
        let field_ty = &field.ty;

        let default_fn = match attrs.ignore {
            ReflectIgnoreBehavior::IgnoreSerialization => {
                let ident = format_ident!("get_default__{}__{}", qualifier, field_ident);
                match &attrs.default {
                    DefaultBehavior::Required | DefaultBehavior::Default => Some(parse_quote! {
                        #[allow(non_snake_case)]
                        fn #ident() -> #FQBox<dyn #bevy_reflect_path::Reflect> {
                            #FQBox::new(<#field_ty as #FQDefault>::default())
                        }
                    }),
                    DefaultBehavior::Func(func) => Some(parse_quote! {
                        #[allow(non_snake_case)]
                        fn #ident() -> #FQBox<dyn #bevy_reflect_path::Reflect> {
                            #FQBox::new(#func() as #field_ty)
                        }
                    }),
                }
            }
            _ => None,
        };

        Self { default_fn }
    }

    /// Returns the function used to generate a default instance of a field.
    ///
    /// Returns `None` if the field does not have or need such a function.
    pub fn default_fn(&self) -> Option<&ItemFn> {
        self.default_fn.as_ref()
    }
}

impl ToTokens for AssociatedData {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        self.default_fn.to_tokens(tokens);
    }
}
