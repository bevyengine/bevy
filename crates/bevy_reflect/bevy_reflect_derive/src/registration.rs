//! Contains code related specifically to Bevy's type registration.

use bevy_macro_utils::BevyManifest;
use proc_macro::TokenStream;
use proc_macro2::Ident;
use quote::{format_ident, quote};
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

    let bevy_reflect_path = BevyManifest::default().get_path("bevy_reflect");
    let registry_ident = format_ident!("registry");

    let empty = Punctuated::<Type, Token![,]>::default();
    let registrations = input.types().map(|ty| {
        let type_data = input.type_data().unwrap_or_else(|| empty.iter());
        let trait_type = input.traits().unwrap_or_else(|| empty.iter());
        quote! {{
            // Get or create registration for type
            let type_registration = match #registry_ident.get_mut(::std::any::TypeId::of::<#ty>()) {
                Some(registration) => registration,
                None => {
                    #registry_ident.register::<#ty>();
                    #registry_ident.get_mut(::std::any::TypeId::of::<#ty>()).unwrap()
                }
            };
            // Register all possible trait casts
            #(
                if let Some(cast_fn) = #bevy_reflect_path::maybe_trait_cast!(#ty, #trait_type) {
                    type_registration.register_trait_cast::<dyn #trait_type>(cast_fn);
                }
            )*
            // Register all possible type data
            #(
                if let Some(data) = #bevy_reflect_path::maybe_type_data!(#ty, #type_data) {
                    type_registration.insert(data);
                }
            )*
        }}
    });

    TokenStream::from(quote! {
        pub fn register_types(#registry_ident: &mut #bevy_reflect_path::TypeRegistry) {
            #(#registrations)*
        }
    })
}

/// Maps to the following invocation:
///
/// ```ignore
/// use bevy_reflect_derive::{register_all, reflect_trait};
///
/// trait MyTrait {}
/// struct MyType {}
/// #[reflect_trait]
/// trait MyData {}
///
/// register_all! {
///     types: [MyType],
///     traits: [MyTrait],
///     data: [ReflectMyData],
/// }
/// ```
///
/// > Note: The order of the `traits`, `data`, and `types` fields does not matter. Additionally,
/// > the commas (separating and trailing) may be omitted entirely.
///
/// The only required field in this macro is the `types` field. All others can be omitted if
/// desired.
struct RegisterAllData {
    type_list: Punctuated<Type, Token![,]>,
    trait_list: Option<Punctuated<Type, Token![,]>>,
    data_list: Option<Punctuated<Type, Token![,]>>,
}

impl RegisterAllData {
    /// Returns an iterator over the types to register.
    fn types(&self) -> Iter<Type> {
        self.type_list.iter()
    }

    /// Returns an iterator over the traits to register.
    fn traits(&self) -> Option<Iter<Type>> {
        self.trait_list.as_ref().map(|list| list.iter())
    }

    /// Returns an iterator over the type data to register.
    fn type_data(&self) -> Option<Iter<Type>> {
        self.data_list.as_ref().map(|list| list.iter())
    }

    /// Parse a list of types.
    ///
    /// This is the portion _after_ the the respective keyword and consumes: `: [ Foo, Bar, Baz ]`
    fn parse_list(input: &mut ParseStream) -> syn::Result<Punctuated<Type, Token![,]>> {
        input.parse::<Token![:]>()?;
        let list;
        bracketed!(list in input);
        let parsed = list.parse_terminated(Type::parse)?;
        // Parse optional trailing comma
        input.parse::<Option<Token![,]>>()?;
        Ok(parsed)
    }
}

impl Parse for RegisterAllData {
    fn parse(mut input: ParseStream) -> syn::Result<Self> {
        let mut trait_list = None;
        let mut type_list = None;
        let mut data_list = None;

        while !input.is_empty() {
            let lookahead = input.lookahead1();
            if lookahead.peek(kw::traits) {
                input.parse::<kw::traits>()?;
                trait_list = Some(Self::parse_list(&mut input)?);
            } else if lookahead.peek(kw::types) {
                input.parse::<kw::types>()?;
                type_list = Some(Self::parse_list(&mut input)?);
            } else if lookahead.peek(kw::data) {
                input.parse::<kw::data>()?;
                data_list = Some(Self::parse_list(&mut input)?);
            } else {
                return Err(syn::Error::new(
                    input.span(),
                    "expected either 'traits', 'types', or 'data' field",
                ));
            }
        }

        if let Some(type_list) = type_list {
            Ok(Self {
                trait_list,
                type_list,
                data_list,
            })
        } else {
            Err(syn::Error::new(
                input.span(),
                "missing required field 'types'",
            ))
        }
    }
}

mod kw {
    syn::custom_keyword!(traits);
    syn::custom_keyword!(types);
    syn::custom_keyword!(data);
}
