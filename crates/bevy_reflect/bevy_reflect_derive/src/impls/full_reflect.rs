use quote::quote;

use crate::{
    derive_data::ReflectMeta,
    utility::{extend_where_clause, WhereClauseOptions},
};
use bevy_macro_utils::fq_std::{FQAny, FQBox, FQResult};

pub(crate) fn impl_full_reflect(
    meta: &ReflectMeta,
    where_clause_options: &WhereClauseOptions,
) -> proc_macro2::TokenStream {
    let type_path = meta.type_path();
    let bevy_reflect_path = meta.bevy_reflect_path();

    let (impl_generics, ty_generics, where_clause) = type_path.generics().split_for_impl();

    let where_reflect_clause = extend_where_clause(where_clause, where_clause_options);

    quote! {
        impl #impl_generics #bevy_reflect_path::Reflect for #type_path #ty_generics #where_reflect_clause {
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

            #[inline]
            fn into_reflect(self: #FQBox<Self>) -> #FQBox<dyn #bevy_reflect_path::Reflect> {
                self
            }

            #[inline]
            fn as_reflect(&self) -> &dyn #bevy_reflect_path::Reflect {
                self
            }

            #[inline]
            fn as_reflect_mut(&mut self) -> &mut dyn #bevy_reflect_path::Reflect {
                self
            }

            #[inline]
            fn set(&mut self, value: #FQBox<dyn #bevy_reflect_path::Reflect>) -> #FQResult<(), #FQBox<dyn #bevy_reflect_path::Reflect>> {
                *self = <dyn #bevy_reflect_path::Reflect>::take(value)?;
                #FQResult::Ok(())
            }
        }
    }
}
