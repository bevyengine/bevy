use crate::derive_data::{EnumVariantFields, ReflectEnum, StructField};
use crate::enum_utility::{get_variant_constructors, EnumVariantConstructors};
use crate::impls::impl_typed;
use proc_macro::TokenStream;
use proc_macro2::{Ident, Span};
use quote::quote;

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
    } = generate_impls(reflect_enum, &ref_index, &ref_name);

    let EnumVariantConstructors {
        variant_names,
        variant_constructors,
    } = get_variant_constructors(reflect_enum, &ref_value, true);

    let hash_fn = reflect_enum
        .meta()
        .traits()
        .get_hash_impl(bevy_reflect_path)
        .unwrap_or_else(|| {
            quote! {
                fn reflect_hash(&self) -> Option<u64> {
                    #bevy_reflect_path::enum_hash(self)
                }
            }
        });
    let debug_fn = reflect_enum.meta().traits().get_debug_impl();
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
                #bevy_reflect_path::VariantFieldIter::new(self)
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
                                for field in #ref_value.iter_fields() {
                                    let name = field.name().unwrap();
                                    #bevy_reflect_path::Enum::field_mut(self, name).map(|v| v.apply(field.value()));
                                }
                            }
                            #bevy_reflect_path::VariantType::Tuple => {
                                for (index, field) in #ref_value.iter_fields().enumerate() {
                                    #bevy_reflect_path::Enum::field_at_mut(self, index).map(|v| v.apply(field.value()));
                                }
                            }
                            _ => {}
                        }
                    } else {
                        // New variant -> perform a switch
                        match #ref_value.variant_name() {
                            #(#variant_names => {
                                *self = #variant_constructors
                            })*
                            name => panic!("variant with name `{}` does not exist on enum `{}`", name, std::any::type_name::<Self>()),
                        }
                    }
                } else {
                    panic!("`{}` is not an enum", #ref_value.type_name());
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

            #debug_fn
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
}

fn generate_impls(reflect_enum: &ReflectEnum, ref_index: &Ident, ref_name: &Ident) -> EnumImpls {
    let bevy_reflect_path = reflect_enum.meta().bevy_reflect_path();

    let mut variant_info = Vec::new();
    let mut enum_field = Vec::new();
    let mut enum_field_at = Vec::new();
    let mut enum_index_of = Vec::new();
    let mut enum_name_at = Vec::new();
    let mut enum_field_len = Vec::new();
    let mut enum_variant_name = Vec::new();
    let mut enum_variant_type = Vec::new();

    for variant in reflect_enum.active_variants() {
        let ident = &variant.data.ident;
        let name = ident.to_string();
        let unit = reflect_enum.get_unit(ident);

        fn for_fields(
            fields: &[StructField],
            mut generate_for_field: impl FnMut(usize, usize, &StructField) -> proc_macro2::TokenStream,
        ) -> (usize, Vec<proc_macro2::TokenStream>) {
            let mut constructor_argument = Vec::new();
            let mut reflect_idx = 0;
            for field in fields.iter() {
                if field.attrs.ignore {
                    // Ignored field
                    continue;
                }
                constructor_argument.push(generate_for_field(reflect_idx, field.index, field));
                reflect_idx += 1;
            }
            (reflect_idx, constructor_argument)
        }
        let mut add_fields_branch = |variant, info_type, arguments, field_len| {
            let variant = Ident::new(variant, Span::call_site());
            let info_type = Ident::new(info_type, Span::call_site());
            variant_info.push(quote! {
                #bevy_reflect_path::VariantInfo::#variant(
                    #bevy_reflect_path::#info_type::new(#arguments)
                )
            });
            enum_field_len.push(quote! {
                #unit{..} => #field_len
            });
            enum_variant_name.push(quote! {
                #unit{..} => #name
            });
            enum_variant_type.push(quote! {
                #unit{..} => #bevy_reflect_path::VariantType::#variant
            });
        };
        match &variant.fields {
            EnumVariantFields::Unit => {
                add_fields_branch("Unit", "UnitVariantInfo", quote!(#name), 0usize);
            }
            EnumVariantFields::Unnamed(fields) => {
                let (field_len, argument) = for_fields(fields, |reflect_idx, declar, field| {
                    let declar_field = syn::Index::from(declar);
                    enum_field_at.push(quote! {
                        #unit { #declar_field : value, .. } if #ref_index == #reflect_idx => Some(value)
                    });
                    let field_ty = &field.data.ty;
                    quote! { #bevy_reflect_path::UnnamedField::new::<#field_ty>(#reflect_idx) }
                });
                let arguments = quote!(#name, &[ #(#argument),* ]);
                add_fields_branch("Tuple", "TupleVariantInfo", arguments, field_len);
            }
            EnumVariantFields::Named(fields) => {
                let (field_len, argument) = for_fields(fields, |reflect_idx, _, field| {
                    let field_ident = field.data.ident.as_ref().unwrap();
                    let field_name = field_ident.to_string();
                    enum_field.push(quote! {
                        #unit{ #field_ident, .. } if #ref_name == #field_name => Some(#field_ident)
                    });
                    enum_field_at.push(quote! {
                        #unit{ #field_ident, .. } if #ref_index == #reflect_idx => Some(#field_ident)
                    });
                    enum_index_of.push(quote! {
                        #unit{ .. } if #ref_name == #field_name => Some(#reflect_idx)
                    });
                    enum_name_at.push(quote! {
                        #unit{ .. } if #ref_index == #reflect_idx => Some(#field_name)
                    });

                    let field_ty = &field.data.ty;
                    quote! { #bevy_reflect_path::NamedField::new::<#field_ty, _>(#field_name) }
                });
                let arguments = quote!(#name, &[ #(#argument),* ]);
                add_fields_branch("Struct", "StructVariantInfo", arguments, field_len);
            }
        };
    }

    EnumImpls {
        variant_info,
        enum_field,
        enum_field_at,
        enum_index_of,
        enum_name_at,
        enum_field_len,
        enum_variant_name,
        enum_variant_type,
    }
}
