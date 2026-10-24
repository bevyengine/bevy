use crate::generics::generate_generics;
use crate::{
    impls::{common_reflect_trait_methods, impl_reflect_casts, impl_type_path, impl_typed},
    where_clause_options::WhereClauseOptions,
    ReflectMeta,
};
use quote::quote;

/// Implements `GetTypeRegistration` and `Reflect` for the given type data.
pub(crate) fn impl_opaque(meta: &ReflectMeta) -> proc_macro2::TokenStream {
    let bevy_reflect_path = meta.bevy_reflect_path();
    let type_path = meta.type_path();

    #[cfg(feature = "reflect_documentation")]
    let with_docs = {
        let doc = quote::ToTokens::to_token_stream(meta.doc());
        Some(quote!(.with_docs(#doc)))
    };
    #[cfg(not(feature = "reflect_documentation"))]
    let with_docs: Option<proc_macro2::TokenStream> = None;

    let where_clause_options = WhereClauseOptions::new(meta);
    let mut info = quote! {
        #bevy_reflect_path::OpaqueInfo::new::<Self>() #with_docs
    };
    if let Some(generics) = generate_generics(meta) {
        info.extend(quote! {
            .with_generics(#generics)
        });
    }
    let typed_impl = impl_typed(
        &where_clause_options,
        quote! {
            let info = #info;
            #bevy_reflect_path::TypeInfo::Opaque(#info)
        },
    );

    let type_path_impl = impl_type_path(meta);
    let reflect_casts_impl = impl_reflect_casts(meta);
    let common_methods = common_reflect_trait_methods(meta, || None, || None, || None);
    let clone_fn = meta.attrs().get_clone_impl(bevy_reflect_path);

    #[cfg(not(feature = "functions"))]
    let function_impls = None::<proc_macro2::TokenStream>;
    #[cfg(feature = "functions")]
    let function_impls = crate::impls::impl_function_traits(&where_clause_options);

    #[cfg(not(feature = "auto_register"))]
    let auto_register = None::<proc_macro2::TokenStream>;
    #[cfg(feature = "auto_register")]
    let auto_register = crate::impls::reflect_auto_registration(meta);

    let (impl_generics, ty_generics, where_clause) = type_path.generics().split_for_impl();
    let where_reflect_clause = where_clause_options.extend_where_clause(where_clause);
    let get_type_registration_impl = meta.get_type_registration(&where_clause_options);

    quote! {
        #get_type_registration_impl

        #type_path_impl

        #typed_impl

        #function_impls

        #auto_register

        impl #impl_generics #bevy_reflect_path::Reflect for #type_path #ty_generics #where_reflect_clause  {
            #[inline]
            fn reflect_kind(&self) -> #bevy_reflect_path::ReflectKind {
                #bevy_reflect_path::ReflectKind::Opaque
            }

            #[inline]
            fn reflect_ref(&self) -> #bevy_reflect_path::ReflectRef {
                #bevy_reflect_path::ReflectRef::Opaque(self)
            }

            #[inline]
            fn reflect_mut(&mut self) -> #bevy_reflect_path::ReflectMut {
                #bevy_reflect_path::ReflectMut::Opaque(self)
            }

            #[inline]
            fn reflect_owned(self: #bevy_reflect_path::__macro_exports::alloc_utils::Box<Self>) -> #bevy_reflect_path::ReflectOwned {
                #bevy_reflect_path::ReflectOwned::Opaque(self)
            }

            #common_methods

            #clone_fn

            #reflect_casts_impl
        }
    }
}
