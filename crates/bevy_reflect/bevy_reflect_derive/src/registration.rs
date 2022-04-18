//! Contains code related specifically to Bevy's type registration.

use bevy_macro_utils::BevyManifest;
use proc_macro::TokenStream;
use proc_macro2::Ident;
use quote::quote;
use syn::parse::{Parse, ParseStream};
use syn::punctuated::{Iter, Punctuated};
use syn::{bracketed, parse_macro_input, Token, Type};
use syn::{Generics, Path};

/// Creates the `GetTypeRegistration` impl for the given type data.
pub(crate) fn impl_get_type_registration(
    type_name: &Ident,
    bevy_reflect_path: &Path,
    registration_data: &[Ident],
    generics: &Generics,
) -> proc_macro2::TokenStream {
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
    quote! {
        #[allow(unused_mut)]
        impl #impl_generics #bevy_reflect_path::GetTypeRegistration for #type_name #ty_generics #where_clause {
            fn get_type_registration() -> #bevy_reflect_path::TypeRegistration {
                let mut registration = #bevy_reflect_path::TypeRegistration::of::<#type_name #ty_generics>();
                #(registration.insert::<#registration_data>(#bevy_reflect_path::FromType::<#type_name #ty_generics>::from_type());)*
                registration
            }
        }
    }
}

pub fn register_all_internal(item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as RegisterAllData);

    let registration_params = input.types().map(|ty| {
        let trait_type = input.traits();
        quote! {
            #ty, #(#trait_type),*
        }
    });

    let bevy_reflect_path = BevyManifest::default().get_path("bevy_reflect");

    TokenStream::from(quote! {
        pub fn register_types(registry: &mut #bevy_reflect_path::TypeRegistry) {
            #(#bevy_reflect_path::register_type!(registry, #registration_params));*
        }
    })
}

/// Maps to the following invocation:
///
/// ```
/// use bevy_reflect_derive::register_all;
///
/// trait MyTrait {}
/// struct MyType {}
///
/// register_all! {
///     traits: [MyTrait],
///     types: [MyType],
/// }
/// ```
///
/// > Note: The order of the `traits` and `types` fields does not matter. Additionally,
/// > the commas (separating and trailing) may be omitted entirely.
struct RegisterAllData {
    trait_list: Punctuated<Type, Token![,]>,
    type_list: Punctuated<Type, Token![,]>,
}

impl RegisterAllData {
    /// Returns an iterator over the types to register.
    fn types(&self) -> Iter<Type> {
        self.type_list.iter()
    }

    /// Returns an iterator over the traits to register.
    fn traits(&self) -> Iter<Type> {
        self.trait_list.iter()
    }

    /// Parse a list of types.
    ///
    /// This is the portion _after_ the the respective keyword and consumes: `: [ Foo, Bar, Baz ]`
    fn parse_list(input: &mut ParseStream) -> syn::Result<Punctuated<Type, Token![,]>> {
        input.parse::<Token![:]>()?;
        let list;
        bracketed!(list in input);
        list.parse_terminated(Type::parse)
    }
}

impl Parse for RegisterAllData {
    fn parse(mut input: ParseStream) -> syn::Result<Self> {
        let trait_list;
        let type_list;

        let lookahead = input.lookahead1();
        if lookahead.peek(kw::traits) {
            // Parse `traits` then `types`
            input.parse::<kw::traits>()?;
            trait_list = Self::parse_list(&mut input)?;
            // Optional separating comma
            input.parse::<Option<Token![,]>>()?;
            input.parse::<kw::types>()?;
            type_list = Self::parse_list(&mut input)?;
            // Optional trailing comma
            input.parse::<Option<Token![,]>>()?;
        } else if lookahead.peek(kw::types) {
            // Parse `types` then `traits`
            input.parse::<kw::types>()?;
            type_list = Self::parse_list(&mut input)?;
            // Optional separating comma
            input.parse::<Option<Token![,]>>()?;
            input.parse::<kw::traits>()?;
            trait_list = Self::parse_list(&mut input)?;
            // Optional trailing comma
            input.parse::<Option<Token![,]>>()?;
        } else {
            return Err(syn::Error::new(
                input.span(),
                "expected either 'traits' or 'types' field",
            ));
        }

        Ok(Self {
            trait_list,
            type_list,
        })
    }
}

mod kw {
    syn::custom_keyword!(traits);
    syn::custom_keyword!(types);
}
