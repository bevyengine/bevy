use bevy_macro_utils::fq_std::{FQAny, FQBox};
use quote::quote;

/// A helper function for quickly implementing the `Reflect::*_any` methods while taking remote types into account.
///
/// Specifically, this handles `Reflect::into_any`, `Reflect::as_any`, and `Reflect::as_any_mut`.
pub(crate) fn impl_reflect_any_methods(is_remote_wrapper: bool) -> proc_macro2::TokenStream {
    if is_remote_wrapper {
        quote! {
            #[inline]
            fn into_any(self: #FQBox<Self>) -> #FQBox<dyn #FQAny> {
                #FQBox::new(self.0)
            }

            #[inline]
            fn as_any(&self) -> &dyn #FQAny {
                &self.0
            }

            #[inline]
            fn as_any_mut(&mut self) -> &mut dyn #FQAny {
                &mut self.0
            }
        }
    } else {
        quote! {
            #[inline]
            fn into_any(self: #FQBox<Self>) -> #FQBox<dyn #FQAny> {
                self
            }

            #[inline]
            fn as_any(&self) -> &dyn #FQAny {
                self
            }

            #[inline]
            fn as_any_mut(&mut self) -> &mut dyn #FQAny {
                self
            }
        }
    }
}
