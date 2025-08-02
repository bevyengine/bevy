use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{parse_macro_input, Data, DeriveInput};

pub(crate) fn derive_variant_defaults(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    let type_ident = &ast.ident;
    let (impl_generics, type_generics, where_clause) = &ast.generics.split_for_impl();
    let Data::Enum(data_enum) = &ast.data else {
        panic!("Can only derive VariantDefaults for enums");
    };

    let mut variant_defaults = Vec::new();
    for variant in &data_enum.variants {
        let variant_ident = &variant.ident;
        let variant_name_lower = variant_ident.to_string().to_lowercase();
        let variant_default_name = format_ident!("default_{}", variant_name_lower);
        match &variant.fields {
            syn::Fields::Named(fields_named) => {
                let fields = fields_named.named.iter().map(|f| &f.ident);
                variant_defaults.push(quote! {
                    pub fn #variant_default_name() -> Self {
                        Self::#variant_ident {
                            #(#fields: Default::default(),)*
                        }
                    }
                })
            }
            syn::Fields::Unnamed(fields_unnamed) => {
                let fields = fields_unnamed
                    .unnamed
                    .iter()
                    .map(|_| quote! {Default::default()});
                variant_defaults.push(quote! {
                    pub fn #variant_default_name() -> Self {
                        Self::#variant_ident(
                            #(#fields,)*
                        )
                    }
                })
            }
            syn::Fields::Unit => variant_defaults.push(quote! {
                pub fn #variant_default_name() -> Self {
                    Self::#variant_ident
                }
            }),
        }
    }

    TokenStream::from(quote! {
        impl #impl_generics #type_ident #type_generics #where_clause {
            #(#variant_defaults)*
        }
    })
}
