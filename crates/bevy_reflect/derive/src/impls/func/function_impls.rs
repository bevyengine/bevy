use crate::{
    impls::func::{
        from_arg::impl_from_arg, get_ownership::impl_get_ownership, into_return::impl_into_return,
    },
    where_clause_options::WhereClauseOptions,
};
use quote::quote;

pub(crate) fn impl_function_traits(
    where_clause_options: &WhereClauseOptions,
) -> proc_macro2::TokenStream {
    let get_ownership = impl_get_ownership(where_clause_options);
    let from_arg = impl_from_arg(where_clause_options);
    let into_return = impl_into_return(where_clause_options);

    quote! {
        #get_ownership

        #from_arg

        #into_return
    }
}
