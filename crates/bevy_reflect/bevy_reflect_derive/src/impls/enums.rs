use crate::derive_data::{EnumVariantFields, ReflectEnum, StructField};
use crate::impls::impl_typed;
use proc_macro::TokenStream;
use proc_macro2::{Ident, Span};
use quote::{quote, ToTokens};

pub(crate) fn impl_enum(reflect_enum: &ReflectEnum) -> TokenStream {
    let bevy_reflect_path = reflect_enum.meta().bevy_reflect_path();
    let enum_name = reflect_enum.meta().type_name();

    let ref_name = Ident::new("__name_param", Span::call_site());
    let ref_index = Ident::new("__index_param", Span::call_site());
    let ref_value = Ident::new("__value_param", Span::call_site());

    let EnumImpls {
        variant_info,
        enum_field,
        enum_field_at,
        enum_index_of,
        enum_name_at,
        enum_field_len,
        enum_variant_name,
        enum_variant_type,
        enum_apply,
    } = generate_impls(reflect_enum, &ref_index, &ref_name, &ref_value);

    let hash_fn = reflect_enum
        .meta()
        .traits()
        .get_hash_impl(bevy_reflect_path);
    let partial_eq_fn = reflect_enum
        .meta()
        .traits()
        .get_partial_eq_impl(bevy_reflect_path)
        .unwrap_or_else(|| {
            quote! {
                fn reflect_partial_eq(&self, value: &dyn #bevy_reflect_path::Reflect) -> Option<bool> {
                    #bevy_reflect_path::enum_partial_eq(self, value)
                }
            }
        });

    let typed_impl = impl_typed(
        enum_name,
        reflect_enum.meta().generics(),
        quote! {
            let variants = [#(#variant_info),*];
            let info = #bevy_reflect_path::EnumInfo::new::<Self>(&variants);
            #bevy_reflect_path::TypeInfo::Enum(info)
        },
        bevy_reflect_path,
    );

    let get_type_registration_impl = reflect_enum.meta().get_type_registration();
    let (impl_generics, ty_generics, where_clause) =
        reflect_enum.meta().generics().split_for_impl();

    TokenStream::from(quote! {
        #get_type_registration_impl

        #typed_impl

        impl #impl_generics #bevy_reflect_path::Enum for #enum_name #ty_generics #where_clause {
            fn field(&self, #ref_name: &str) -> Option<&dyn #bevy_reflect_path::Reflect> {
                 match self {
                    #(#enum_field,)*
                    _ => None,
                }
            }

            fn field_at(&self, #ref_index: usize) -> Option<&dyn #bevy_reflect_path::Reflect> {
                match self {
                    #(#enum_field_at,)*
                    _ => None,
                }
            }

            fn field_mut(&mut self, #ref_name: &str) -> Option<&mut dyn #bevy_reflect_path::Reflect> {
                 match self {
                    #(#enum_field,)*
                    _ => None,
                }
            }

            fn field_at_mut(&mut self, #ref_index: usize) -> Option<&mut dyn #bevy_reflect_path::Reflect> {
                match self {
                    #(#enum_field_at,)*
                    _ => None,
                }
            }

            fn index_of(&self, #ref_name: &str) -> Option<usize> {
                 match self {
                    #(#enum_index_of,)*
                    _ => None,
                }
            }

            fn name_at(&self, #ref_index: usize) -> Option<&str> {
                 match self {
                    #(#enum_name_at,)*
                    _ => None,
                }
            }

            fn iter_fields(&self) -> #bevy_reflect_path::VariantFieldIter {
                VariantFieldIter::new(self)
            }

            #[inline]
            fn field_len(&self) -> usize {
                 match self {
                    #(#enum_field_len,)*
                    _ => 0,
                }
            }

            #[inline]
            fn variant_name(&self) -> &str {
                 match self {
                    #(#enum_variant_name,)*
                    _ => unreachable!(),
                }
            }

            #[inline]
            fn variant_type(&self) -> #bevy_reflect_path::VariantType {
                 match self {
                    #(#enum_variant_type,)*
                    _ => unreachable!(),
                }
            }

            fn clone_dynamic(&self) -> #bevy_reflect_path::DynamicEnum {
                #bevy_reflect_path::DynamicEnum::from_ref::<Self>(self)
            }
        }

        impl #impl_generics #bevy_reflect_path::Reflect for #enum_name #ty_generics #where_clause {
            #[inline]
            fn type_name(&self) -> &str {
                std::any::type_name::<Self>()
            }

            #[inline]
            fn get_type_info(&self) -> &'static #bevy_reflect_path::TypeInfo {
                <Self as #bevy_reflect_path::Typed>::type_info()
            }

            #[inline]
            fn into_any(self: Box<Self>) -> Box<dyn std::any::Any> {
                self
            }

            #[inline]
            fn as_any(&self) -> &dyn std::any::Any {
                self
            }

            #[inline]
            fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
                self
            }

            #[inline]
            fn as_reflect(&self) -> &dyn #bevy_reflect_path::Reflect {
                self
            }

            #[inline]
            fn as_reflect_mut(&mut self) -> &mut dyn #bevy_reflect_path::Reflect {
                self
            }

            #[inline]
            fn clone_value(&self) -> Box<dyn #bevy_reflect_path::Reflect> {
                Box::new(#bevy_reflect_path::Enum::clone_dynamic(self))
            }

            #[inline]
            fn set(&mut self, #ref_value: Box<dyn #bevy_reflect_path::Reflect>) -> Result<(), Box<dyn #bevy_reflect_path::Reflect>> {
                *self = #ref_value.take()?;
                Ok(())
            }

            #[inline]
            fn apply(&mut self, #ref_value: &dyn #bevy_reflect_path::Reflect) {
                if let #bevy_reflect_path::ReflectRef::Enum(#ref_value) = #ref_value.reflect_ref() {
                    if #bevy_reflect_path::Enum::variant_name(self) == #ref_value.variant_name() {
                        // Same variant -> just update fields
                        match #ref_value.variant_type() {
                            #bevy_reflect_path::VariantType::Struct => {
                                for (index, field) in #ref_value.iter_fields().enumerate() {
                                    let name = #ref_value.name_at(index).unwrap();
                                    #bevy_reflect_path::Enum::field_mut(self, name).map(|v| v.apply(field));
                                }
                            }
                            #bevy_reflect_path::VariantType::Tuple => {
                                for (index, field) in #ref_value.iter_fields().enumerate() {
                                    #bevy_reflect_path::Enum::field_at_mut(self, index).map(|v| v.apply(field));
                                }
                            }
                            _ => {}
                        }
                    } else {
                        // New variant -> perform a switch
                        match #ref_value.variant_name() {
                            #(#enum_apply,)*
                            _ => panic!("Enum is not a {}.", std::any::type_name::<Self>()),
                        }
                    }
                }
            }

            fn reflect_ref(&self) -> #bevy_reflect_path::ReflectRef {
                #bevy_reflect_path::ReflectRef::Enum(self)
            }

            fn reflect_mut(&mut self) -> #bevy_reflect_path::ReflectMut {
                #bevy_reflect_path::ReflectMut::Enum(self)
            }

            #hash_fn

            #partial_eq_fn
        }
    })
}

struct EnumImpls {
    variant_info: Vec<proc_macro2::TokenStream>,
    enum_field: Vec<proc_macro2::TokenStream>,
    enum_field_at: Vec<proc_macro2::TokenStream>,
    enum_index_of: Vec<proc_macro2::TokenStream>,
    enum_name_at: Vec<proc_macro2::TokenStream>,
    enum_field_len: Vec<proc_macro2::TokenStream>,
    enum_variant_name: Vec<proc_macro2::TokenStream>,
    enum_variant_type: Vec<proc_macro2::TokenStream>,
    enum_apply: Vec<proc_macro2::TokenStream>,
}

fn filter_active(field: &StructField) -> bool {
    !field.attrs.ignore
}

fn underscores(count: usize) -> proc_macro2::TokenStream {
    let mut output = quote! {};
    for _ in 0..count {
        output = quote! {
            #output _,
        }
    }
    output
}

fn generate_impls(
    reflect_enum: &ReflectEnum,
    ref_index: &Ident,
    ref_name: &Ident,
    ref_value: &Ident,
) -> EnumImpls {
    let bevy_reflect_path = reflect_enum.meta().bevy_reflect_path();

    let mut variant_info: Vec<proc_macro2::TokenStream> = Vec::new();
    let mut enum_field: Vec<proc_macro2::TokenStream> = Vec::new();
    let mut enum_field_at: Vec<proc_macro2::TokenStream> = Vec::new();
    let mut enum_index_of: Vec<proc_macro2::TokenStream> = Vec::new();
    let mut enum_name_at: Vec<proc_macro2::TokenStream> = Vec::new();
    let mut enum_field_len: Vec<proc_macro2::TokenStream> = Vec::new();
    let mut enum_variant_name: Vec<proc_macro2::TokenStream> = Vec::new();
    let mut enum_variant_type: Vec<proc_macro2::TokenStream> = Vec::new();
    let mut enum_apply: Vec<proc_macro2::TokenStream> = Vec::new();

    for variant in reflect_enum.active_variants() {
        let ident = &variant.data.ident;
        let name = ident.to_string();
        let unit = reflect_enum.get_unit(ident);

        // TODO: Filter by active fields
        match &variant.fields {
            EnumVariantFields::Unit => {
                variant_info.push(quote! {
                    #bevy_reflect_path::VariantInfo::Unit(
                        #bevy_reflect_path::UnitVariantInfo::new(#name)
                    )
                });
                enum_variant_name.push(quote! {
                    #unit => #name
                });
                enum_variant_type.push(quote! {
                    #unit => #bevy_reflect_path::VariantType::Unit
                });
                enum_apply.push(quote! {
                    #name => {
                        *self = #unit;
                    }
                });
            }
            EnumVariantFields::Unnamed(fields) => {
                let mut field_info = Vec::new();
                let mut variant_apply = Vec::new();
                for (field_idx, field) in fields.iter().enumerate() {
                    let empties = underscores(field_idx);
                    enum_field_at.push(quote! {
                        #unit( #empties value, .. ) if #ref_index == #field_idx => Some(value)
                    });

                    let field_ty = &field.data.ty;
                    field_info.push(quote! {
                        #bevy_reflect_path::UnnamedField::new::<#field_ty>(#field_idx)
                    });

                    let expect_field = format!("field at index `{}` should exist", field_idx);
                    let expect_type = format!(
                        "field at index `{}` should be of type `{}`",
                        field_idx,
                        field_ty.to_token_stream().to_string()
                    );
                    variant_apply.push(quote! {
                        #ref_value
                            .field_at(#field_idx)
                            .expect(#expect_field)
                            .clone_value()
                            .take::<#field_ty>()
                            .expect(#expect_type)
                    });
                }

                let field_len = fields.len();
                enum_field_len.push(quote! {
                    #unit(..) => #field_len
                });
                enum_variant_name.push(quote! {
                    #unit(..) => #name
                });
                enum_variant_type.push(quote! {
                    #unit(..) => #bevy_reflect_path::VariantType::Tuple
                });
                enum_apply.push(quote! {
                    #name => {
                        *self = #unit( #(#variant_apply),* );
                    }
                });
                variant_info.push(quote! {
                    #bevy_reflect_path::VariantInfo::Tuple(
                        #bevy_reflect_path::TupleVariantInfo::new(#name, &[
                            #(#field_info),*
                        ])
                    )
                });
            }
            EnumVariantFields::Named(fields) => {
                let mut field_info = Vec::new();
                let mut variant_apply = Vec::new();
                for (field_idx, field) in fields.iter().enumerate() {
                    let field_ident = field.data.ident.as_ref().unwrap();
                    let field_name = field_ident.to_string();
                    enum_field.push(quote! {
                        #unit{ #field_ident, .. } if #ref_name == #field_name => Some(#field_ident)
                    });
                    enum_field_at.push(quote! {
                        #unit{ #field_ident, .. } if #ref_index == #field_idx => Some(#field_ident)
                    });
                    enum_index_of.push(quote! {
                        #unit{ #field_ident, .. } if #ref_name == #field_name => Some(#field_idx)
                    });
                    enum_name_at.push(quote! {
                        #unit{ #field_ident, .. } if #ref_index == #field_idx => Some(#field_name)
                    });

                    let field_ty = &field.data.ty;
                    field_info.push(quote! {
                        #bevy_reflect_path::NamedField::new::<#field_ty, _>(#field_name)
                    });

                    let expect_field = format!("field with name `{}` should exist", field_name);
                    let expect_type = format!(
                        "field with name `{}` should be of type `{}`",
                        field_name,
                        field_ty.to_token_stream().to_string()
                    );
                    variant_apply.push(quote! {
                        #field_ident: #ref_value
                            .field(#field_name)
                            .expect(#expect_field)
                            .clone_value()
                            .take::<#field_ty>()
                            .expect(#expect_type)
                    });
                }

                let field_len = fields.len();
                enum_field_len.push(quote! {
                    #unit{..} => #field_len
                });
                enum_variant_name.push(quote! {
                    #unit{..} => #name
                });
                enum_variant_type.push(quote! {
                    #unit{..} => #bevy_reflect_path::VariantType::Struct
                });
                enum_apply.push(quote! {
                    #name => {
                        *self = #unit{ #(#variant_apply),* };
                    }
                });
                variant_info.push(quote! {
                    #bevy_reflect_path::VariantInfo::Struct(
                        #bevy_reflect_path::StructVariantInfo::new(#name, &[
                            #(#field_info),*
                        ])
                    )
                });
            }
        }
    }

    EnumImpls {
        variant_info,
        enum_field,
        enum_field_at,
        enum_index_of,
        enum_name_at,
        enum_field_len,
        enum_variant_name,
        enum_apply,
        enum_variant_type,
    }
}
