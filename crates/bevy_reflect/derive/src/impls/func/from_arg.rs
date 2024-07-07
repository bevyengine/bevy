use crate::derive_data::ReflectMeta;
use crate::utility::WhereClauseOptions;
use bevy_macro_utils::fq_std::FQResult;
use quote::quote;

pub(crate) fn impl_from_arg(
    meta: &ReflectMeta,
    where_clause_options: &WhereClauseOptions,
) -> proc_macro2::TokenStream {
    let bevy_reflect = meta.bevy_reflect_path();
    let type_path = meta.type_path();

    let (impl_generics, ty_generics, where_clause) = type_path.generics().split_for_impl();
    let where_reflect_clause = where_clause_options.extend_where_clause(where_clause);

    quote! {
        impl #impl_generics #bevy_reflect::func::args::FromArg for #type_path #ty_generics #where_reflect_clause {
            type Item<'from_arg> = #type_path #ty_generics;
            fn from_arg<'from_arg>(
                arg: #bevy_reflect::func::args::Arg<'from_arg>,
                info: &#bevy_reflect::func::args::ArgInfo,
            ) -> #FQResult<Self::Item<'from_arg>, #bevy_reflect::func::args::ArgError> {
                arg.take_owned(info)
            }
        }

        impl #impl_generics #bevy_reflect::func::args::FromArg for &'static #type_path #ty_generics #where_reflect_clause {
            type Item<'from_arg> = &'from_arg #type_path #ty_generics;
            fn from_arg<'from_arg>(
                arg: #bevy_reflect::func::args::Arg<'from_arg>,
                info: &#bevy_reflect::func::args::ArgInfo,
            ) -> #FQResult<Self::Item<'from_arg>, #bevy_reflect::func::args::ArgError> {
                arg.take_ref(info)
            }
        }

        impl #impl_generics #bevy_reflect::func::args::FromArg for &'static mut #type_path #ty_generics #where_reflect_clause {
            type Item<'from_arg> = &'from_arg mut #type_path #ty_generics;
            fn from_arg<'from_arg>(
                arg: #bevy_reflect::func::args::Arg<'from_arg>,
                info: &#bevy_reflect::func::args::ArgInfo,
            ) -> #FQResult<Self::Item<'from_arg>, #bevy_reflect::func::args::ArgError> {
                arg.take_mut(info)
            }
        }
    }
}
