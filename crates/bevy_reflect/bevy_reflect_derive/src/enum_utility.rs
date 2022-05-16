use crate::derive_data::{EnumVariantFields, ReflectEnum};
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
    let mut variant_names: Vec<String> = Vec::new();
    let mut variant_constructors: Vec<proc_macro2::TokenStream> = Vec::new();

    for variant in reflect_enum.active_variants() {
        let ident = &variant.data.ident;
        let name = ident.to_string();
        let unit = reflect_enum.get_unit(ident);

        match &variant.fields {
            EnumVariantFields::Unit => {
                variant_constructors.push(quote! {
                    #unit
                });
            }
            EnumVariantFields::Unnamed(fields) => {
                let mut variant_apply = Vec::new();
                let mut field_idx: usize = 0;
                for field in fields.iter() {
                    if field.attrs.ignore {
                        // Ignored field -> use default value
                        variant_apply.push(quote! {
                            Default::default()
                        });
                        continue;
                    }

                    let field_ty = &field.data.ty;
                    let expect_field = format!("field at index `{}` should exist", field_idx);
                    let expect_type = format!(
                        "field at index `{}` should be of type `{}`",
                        field_idx,
                        field_ty.to_token_stream().to_string()
                    );

                    let unwrapper = if can_panic {
                        quote!(.expect(#expect_type))
                    } else {
                        quote!(?)
                    };

                    variant_apply.push(quote! {
                        #bevy_reflect_path::FromReflect::from_reflect(
                            #ref_value
                                .field_at(#field_idx)
                                .expect(#expect_field)
                        )
                        #unwrapper
                    });

                    field_idx += 1;
                }

                variant_constructors.push(quote! {
                    #unit( #(#variant_apply),* )
                });
            }
            EnumVariantFields::Named(fields) => {
                let mut variant_apply = Vec::new();
                for field in fields.iter() {
                    let field_ident = field.data.ident.as_ref().unwrap();

                    if field.attrs.ignore {
                        // Ignored field -> use default value
                        variant_apply.push(quote! {
                            #field_ident: Default::default()
                        });
                        continue;
                    }

                    let field_name = field_ident.to_string();
                    let field_ty = &field.data.ty;
                    let expect_field = format!("field with name `{}` should exist", field_name);
                    let expect_type = format!(
                        "field with name `{}` should be of type `{}`",
                        field_name,
                        field_ty.to_token_stream().to_string()
                    );

                    let unwrapper = if can_panic {
                        quote!(.expect(#expect_type))
                    } else {
                        quote!(?)
                    };

                    variant_apply.push(quote! {
                        #field_ident: #bevy_reflect_path::FromReflect::from_reflect(
                            #ref_value
                                .field(#field_name)
                                .expect(#expect_field)
                            )
                            #unwrapper
                    });
                }

                variant_constructors.push(quote! {
                    #unit{ #(#variant_apply),* }
                });
            }
        }

        variant_names.push(name);
    }

    EnumVariantConstructors {
        variant_names,
        variant_constructors,
    }
}
