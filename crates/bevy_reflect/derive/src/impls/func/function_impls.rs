use crate::derive_data::ReflectMeta;
use crate::impls::func::from_arg::impl_from_arg;
use crate::impls::func::get_ownership::impl_get_ownership;
use crate::impls::func::into_return::impl_into_return;
use crate::utility::WhereClauseOptions;
use quote::quote;

pub(crate) fn impl_function_traits(
    meta: &ReflectMeta,
    where_clause_options: &WhereClauseOptions,
) -> proc_macro2::TokenStream {
    let get_ownership = impl_get_ownership(meta, where_clause_options);
    let from_arg = impl_from_arg(meta, where_clause_options);
    let into_return = impl_into_return(meta, where_clause_options);

    quote! {
        #get_ownership

        #from_arg

        #into_return
    }
}
