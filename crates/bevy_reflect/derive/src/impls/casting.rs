use crate::derive_data::ReflectMeta;
use crate::where_clause_options::WhereClauseOptions;
use proc_macro2::TokenStream;
use quote::quote;

/// Generates impls for the `CastPartialReflect` and `CastReflect` traits.
pub(crate) fn impl_casting_traits(
    meta: &ReflectMeta,
    where_clause_options: &WhereClauseOptions,
) -> TokenStream {
    let bevy_reflect_path = meta.bevy_reflect_path();
    let type_path = meta.type_path();
    let (impl_generics, ty_generics, where_clause) = type_path.generics().split_for_impl();
    let where_reflect_clause = where_clause_options.extend_where_clause(where_clause);

    quote! {
        impl #impl_generics #bevy_reflect_path::cast::CastPartialReflect for #type_path #ty_generics #where_reflect_clause {
            #[inline]
            fn as_partial_reflect(&self) -> &dyn #bevy_reflect_path::PartialReflect {
                self
            }

            #[inline]
            fn as_partial_reflect_mut(&mut self) -> &mut dyn #bevy_reflect_path::PartialReflect {
                self
            }

            #[inline]
            fn into_partial_reflect(self: #bevy_reflect_path::__macro_exports::alloc_utils::Box<Self>) -> #bevy_reflect_path::__macro_exports::alloc_utils::Box<dyn #bevy_reflect_path::PartialReflect> {
                self
            }
        }

        impl #impl_generics #bevy_reflect_path::cast::CastReflect for #type_path #ty_generics #where_reflect_clause {
            #[inline]
            fn as_reflect(&self) -> &dyn #bevy_reflect_path::Reflect {
                self
            }

            #[inline]
            fn as_reflect_mut(&mut self) -> &mut dyn #bevy_reflect_path::Reflect {
                self
            }

            #[inline]
            fn into_reflect(self: #bevy_reflect_path::__macro_exports::alloc_utils::Box<Self>) -> #bevy_reflect_path::__macro_exports::alloc_utils::Box<dyn #bevy_reflect_path::Reflect> {
                self
            }
        }
    }
}
