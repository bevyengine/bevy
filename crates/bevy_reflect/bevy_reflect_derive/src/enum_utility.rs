use crate::{
    derive_data::{EnumVariantFields, ReflectEnum},
    utility::ident_or_index,
};
use proc_macro2::Ident;
use quote::{quote, ToTokens};

/// Contains all data needed to construct all variants within an enum.
pub(crate) struct EnumVariantConstructors {
    /// The names of each variant as a string.
    pub variant_names: Vec<String>,
    /// The stream of tokens that will construct each variant.
    pub variant_constructors: Vec<proc_macro2::TokenStream>,
}

/// Gets the constructors for all variants in the given enum.
pub(crate) fn get_variant_constructors(
    reflect_enum: &ReflectEnum,
    ref_value: &Ident,
    can_panic: bool,
) -> EnumVariantConstructors {
    let bevy_reflect_path = reflect_enum.meta().bevy_reflect_path();
    let variant_count = reflect_enum.variants().len();
    let mut variant_names = Vec::with_capacity(variant_count);
    let mut variant_constructors = Vec::with_capacity(variant_count);

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
            let field_ident = ident_or_index(field.data.ident.as_ref(), declar_index);
            let field_value = if field.attrs.ignore {
                quote! { Default::default() }
            } else {
                let error_repr = field.data.ident.as_ref().map_or_else(
                    || format!("at index {reflect_index}"),
                    |name| format!("`{name}`"),
                );
                let unwrapper = if can_panic {
                    let type_err_message = format!(
                        "the field {error_repr} should be of type `{}`",
                        field.data.ty.to_token_stream()
                    );
                    quote!(.expect(#type_err_message))
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
                let missing_field_err_message = format!("the field {error_repr} was not declared");
                let accessor = quote!(#field_accessor .expect(#missing_field_err_message));
                quote! {
                    #bevy_reflect_path::FromReflect::from_reflect(#ref_value #accessor)
                    #unwrapper
                }
            };
            quote! { #field_ident : #field_value }
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
