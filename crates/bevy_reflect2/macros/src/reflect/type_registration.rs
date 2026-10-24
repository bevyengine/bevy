use proc_macro2::TokenStream;
use quote::{quote, quote_spanned};
use syn::{spanned::Spanned, Generics, Path, Type};

use crate::reflect::type_data::TypeDataRegistration;

/// Returns the `TypeRegistration` impl for `target`. `dependencies` are the types registered by
/// `register_type_dependencies`, which must implement `GetTypeRegistration` given
/// `extended_where_clause`.
pub(crate) fn type_registration(
    target: &TokenStream,
    generics: &Generics,
    bevy_reflect: &Path,
    extended_where_clause: &TokenStream,
    type_data: &[TypeDataRegistration],
    dependencies: &[&Type],
) -> TokenStream {
    let type_data = type_data.iter().map(|data| {
        let reflect_path = data.reflect_path();
        let args = data.args();

        let args = if args.is_empty() {
            // Set the span so that we get pointed to the correct identifier even when there are no type data arguments
            let span = reflect_path.span();
            quote_spanned!(span => ())
        } else {
            quote!((#args))
        };

        let cfg_attrs = data.cfg_attrs();
        quote! {
            #(#cfg_attrs)*
            registration.register_type_data_with::<#reflect_path, Self, _>(#args);
        }
    });

    let (impl_generics, _, _) = generics.split_for_impl();
    quote! {
        impl #impl_generics #bevy_reflect::GetTypeRegistration for #target #extended_where_clause {
            fn get_type_registration() -> #bevy_reflect::TypeRegistration {
                let mut registration = #bevy_reflect::TypeRegistration::of::<Self>();
                #(#type_data)*
                registration
            }

            fn register_type_dependencies(registry: &mut #bevy_reflect::TypeRegistry) {
                #(registry.register::<#dependencies>();)*
            }
        }
    }
}
