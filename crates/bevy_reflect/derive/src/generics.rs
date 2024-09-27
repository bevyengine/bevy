use crate::derive_data::ReflectMeta;
use proc_macro2::TokenStream;
use quote::{quote, ToTokens};
use syn::punctuated::Punctuated;
use syn::{GenericParam, Path, Token};

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
                Some(generate_generic_info(
                    ident,
                    ident.to_string(),
                    false,
                    bevy_reflect_path,
                ))
            }
            GenericParam::Const(const_param) => {
                let ty = &const_param.ty;
                let name = const_param.ident.to_string();
                Some(generate_generic_info(ty, name, true, bevy_reflect_path))
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

fn generate_generic_info(
    ty: impl ToTokens,
    name: String,
    is_const: bool,
    bevy_reflect_path: &Path,
) -> TokenStream {
    quote! {
        #bevy_reflect_path::GenericInfo::new::<#ty>(
            ::alloc::borrow::Cow::Borrowed(#name),
            #is_const
        )
    }
}
