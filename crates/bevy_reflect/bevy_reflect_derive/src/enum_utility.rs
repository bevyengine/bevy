use crate::fq_std::{FQBox, FQDefault};
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

    for variant in reflect_enum.variants() {
        let ident = &variant.data.ident;
        let name = ident.to_string();
        let variant_constructor = reflect_enum.get_unit(ident);

        let fields = match &variant.fields {
            EnumVariantFields::Unit => &[],
            EnumVariantFields::Named(fields) | EnumVariantFields::Unnamed(fields) => {
                fields.as_slice()
            }
        };
        let mut reflect_index: usize = 0;
        let constructor_fields = fields.iter().enumerate().map(|(declar_index, field)| {
            let field_ident = ident_or_index(field.data.ident.as_ref(), declar_index);
            let field_value = if field.attrs.ignore.is_ignored() {
                quote! { #FQDefault::default() }
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
                    match &field.data.ident {
                        Some(ident) => {
                            let name = ident.to_string();
                            quote!(.map_err(|err| #bevy_reflect_path::FromReflectError::NamedFieldError {
                                from_type: #bevy_reflect_path::Reflect::get_type_info(#ref_value),
                                from_kind: #bevy_reflect_path::Reflect::reflect_kind(#ref_value),
                                to_type: <Self as #bevy_reflect_path::Typed>::type_info(),
                                field: #name,
                                source: #FQBox::new(err),
                            })?)
                        },
                        None => quote!(.map_err(|err| #bevy_reflect_path::FromReflectError::UnnamedFieldError {
                            from_type: #bevy_reflect_path::Reflect::get_type_info(#ref_value),
                            from_kind: #bevy_reflect_path::Reflect::reflect_kind(#ref_value),
                            to_type: <Self as #bevy_reflect_path::Typed>::type_info(),
                            index: #reflect_index,
                            source: #FQBox::new(err),
                        })?)
                    }
                };
                let field_accessor = match &field.data.ident {
                    Some(ident) => {
                        let name = ident.to_string();
                        if can_panic {
                            quote!(.field(#name))
                        } else {
                            quote!(.field(#name)
                                   .ok_or_else(|| #bevy_reflect_path::FromReflectError::MissingNamedField {
                                       from_type: #bevy_reflect_path::Reflect::get_type_info(#ref_value),
                                       from_kind: #bevy_reflect_path::Reflect::reflect_kind(#ref_value),
                                       to_type: <Self as #bevy_reflect_path::Typed>::type_info(),
                                       field: #name,
                                   })
                            )
                        }
                    }
                    None => if can_panic {
                        quote!(.field_at(#reflect_index))
                    } else {
                        quote!(.field_at(#reflect_index)
                               .ok_or_else(|| #bevy_reflect_path::FromReflectError::MissingUnnamedField {
                                   from_type: #bevy_reflect_path::Reflect::get_type_info(#ref_value),
                                   from_kind: #bevy_reflect_path::Reflect::reflect_kind(#ref_value),
                                   to_type: <Self as #bevy_reflect_path::Typed>::type_info(),
                                   index: #reflect_index,
                               })
                        )
                    }
                };
                reflect_index += 1;
                let missing_field_err_message = format!("the field {error_repr} was not declared");
                let accessor = if can_panic {
                    quote!(#field_accessor .expect(#missing_field_err_message))
                } else {
                    quote!(#field_accessor?)
                };
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
