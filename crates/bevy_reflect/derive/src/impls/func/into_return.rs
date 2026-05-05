use crate::where_clause_options::WhereClauseOptions;
use quote::quote;

pub(crate) fn impl_into_return(
    where_clause_options: &WhereClauseOptions,
) -> proc_macro2::TokenStream {
    let meta = where_clause_options.meta();
    let bevy_reflect = meta.bevy_reflect_path();
    let type_path = meta.type_path();

    let (impl_generics, ty_generics, where_clause) = type_path.generics().split_for_impl();
    let where_reflect_clause = where_clause_options.extend_where_clause(where_clause);

    quote! {
        impl #impl_generics #bevy_reflect::func::IntoReturn for #type_path #ty_generics #where_reflect_clause {
            fn into_return<'into_return>(self) -> #bevy_reflect::func::Return<'into_return>
                where Self: 'into_return
            {
                #bevy_reflect::func::Return::Owned(#bevy_reflect::__macro_exports::alloc_utils::Box::new(self))
            }
        }
    }
}
