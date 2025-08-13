use crate::derive_data::ReflectMeta;
use proc_macro2::TokenStream;
use quote::quote;
use syn::punctuated::Punctuated;
use syn::{GenericParam, Token};

/// Creates a `TokenStream` for generating an expression that creates a `Generics` instance.
///
/// Returns `None` if `Generics` cannot or should not be generated.
pub(crate) fn generate_generics(meta: &ReflectMeta) -> Option<TokenStream> {
    if !meta.attrs().type_path_attrs().should_auto_derive() {
        // Cannot verify that all generic parameters implement `TypePath`
        return None;
    }

    let bevy_reflect_path = meta.bevy_reflect_path();

    let generics = meta
        .type_path()
        .generics()
        .params
        .iter()
        .filter_map(|param| match param {
            GenericParam::Type(ty_param) => {
                let ident = &ty_param.ident;
                let name = ident.to_string();
                let with_default = ty_param
                    .default
                    .as_ref()
                    .map(|default_ty| quote!(.with_default::<#default_ty>()));

                Some(quote! {
                    #bevy_reflect_path::GenericInfo::Type(
                        #bevy_reflect_path::TypeParamInfo::new::<#ident>(
                            #bevy_reflect_path::__macro_exports::alloc_utils::Cow::Borrowed(#name),
                        )
                        #with_default
                    )
                })
            }
            GenericParam::Const(const_param) => {
                let ty = &const_param.ty;
                let name = const_param.ident.to_string();
                let with_default = const_param.default.as_ref().map(|default| {
                    // We add the `as #ty` to ensure that the correct type is inferred.
                    quote!(.with_default(#default as #ty))
                });

                Some(quote! {
                    #[allow(
                        clippy::unnecessary_cast,
                        reason = "reflection requires an explicit type hint for const generics"
                    )]
                    #bevy_reflect_path::GenericInfo::Const(
                        #bevy_reflect_path::ConstParamInfo::new::<#ty>(
                            #bevy_reflect_path::__macro_exports::alloc_utils::Cow::Borrowed(#name),
                        )
                        #with_default
                    )
                })
            }
            GenericParam::Lifetime(_) => None,
        })
        .collect::<Punctuated<_, Token![,]>>();

    if generics.is_empty() {
        // No generics to generate
        return None;
    }

    Some(quote!(#bevy_reflect_path::Generics::from_iter([ #generics ])))
}
