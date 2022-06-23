use crate::derive_data::{EnumVariantFields, ReflectEnum};
use proc_macro2::{Ident, TokenStream};
use quote::{quote, ToTokens};
use syn::Index;

/// Contains all data needed to construct all variants within an enum.
pub(crate) struct EnumVariantConstructors {
    /// The names of each variant as a string.
    pub variant_names: Vec<String>,
    /// The stream of tokens that will construct each variant.
    pub variant_constructors: Vec<proc_macro2::TokenStream>,
}

fn field_indentifier(i: usize, ident: Option<&Ident>) -> TokenStream {
    let tuple_accessor = Index::from(i);
    match ident {
        Some(ident) => quote!(#ident :),
        None => quote!(#tuple_accessor :),
    }
}
/// Gets the constructors for all variants in the given enum.
pub(crate) fn get_variant_constructors(
    reflect_enum: &ReflectEnum,
    ref_value: &Ident,
    can_panic: bool,
) -> EnumVariantConstructors {
    let bevy_reflect_path = reflect_enum.meta().bevy_reflect_path();
    let variant_count = reflect_enum.variants().len();
    let mut variant_names: Vec<String> = Vec::with_capacity(variant_count);
    let mut variant_constructors: Vec<proc_macro2::TokenStream> = Vec::with_capacity(variant_count);

    for variant in reflect_enum.active_variants() {
        let ident = &variant.data.ident;
        let name = ident.to_string();
        let variant_constructor = reflect_enum.get_unit(ident);

        let fields = match &variant.fields {
            EnumVariantFields::Unit => &[],
            EnumVariantFields::Named(fields) => fields.as_slice(),
            EnumVariantFields::Unnamed(fields) => fields.as_slice(),
        };
        let mut reflect_index: usize = 0;
        let constructor_fields = fields.iter().enumerate().map(|(declar_index, field)| {
            let field_ident = field_indentifier(declar_index, field.data.ident.as_ref());
            let field_value = if field.attrs.ignore {
                quote! { Default::default() }
            } else {
                let error_repr = match (&field.data.ident, reflect_index) {
                    (None, 0) => "1st".to_owned(),
                    (None, 1) => "2nd".to_owned(),
                    (None, 2) => "3rd".to_owned(),
                    // Assuming we have less than 21 fields
                    (None, n) => format!("{}th", n + 1),
                    (Some(name), _) => format!("`{name}`"),
                };
                let unwrapper = if can_panic {
                    let expect_type = format!(
                        "the {error_repr} field should be of type `{}`",
                        field.data.ty.to_token_stream()
                    );
                    quote!(.expect(#expect_type))
                } else {
                    quote!(?)
                };
                let field_accessor = match &field.data.ident {
                    Some(ident) => {
                        let name = ident.to_string();
                        quote!(.field(#name))
                    }
                    None => quote!(.field_at(#reflect_index)),
                };
                reflect_index += 1;
                let expect_field = format!("the {error_repr} field was not declared");
                let accessor = quote!(#field_accessor .expect(#expect_field));
                quote! {
                    #bevy_reflect_path::FromReflect::from_reflect(#ref_value #accessor)
                    #unwrapper
                }
            };
            quote! { #field_ident #field_value }
        });
        variant_constructors.push(quote! {
            #variant_constructor { #( #constructor_fields ),* }
        });
        variant_names.push(name);
    }

    EnumVariantConstructors {
        variant_names,
        variant_constructors,
    }
}
