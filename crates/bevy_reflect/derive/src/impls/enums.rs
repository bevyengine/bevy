use crate::derive_data::{EnumVariantFields, ReflectEnum, StructField};
use crate::enum_utility::{EnumVariantOutputData, TryApplyVariantBuilder, VariantBuilder};
use crate::impls::{impl_type_path, impl_typed};
use bevy_macro_utils::fq_std::{FQAny, FQBox, FQOption, FQResult};
use proc_macro2::{Ident, Span};
use quote::quote;
use syn::Fields;

pub(crate) fn impl_enum(reflect_enum: &ReflectEnum) -> proc_macro2::TokenStream {
    let bevy_reflect_path = reflect_enum.meta().bevy_reflect_path();
    let enum_path = reflect_enum.meta().type_path();

    let ref_name = Ident::new("__name_param", Span::call_site());
    let ref_index = Ident::new("__index_param", Span::call_site());
    let ref_value = Ident::new("__value_param", Span::call_site());

    let where_clause_options = reflect_enum.where_clause_options();

    let EnumImpls {
        enum_field,
        enum_field_at,
        enum_index_of,
        enum_name_at,
        enum_field_len,
        enum_variant_name,
        enum_variant_index,
        enum_variant_type,
    } = generate_impls(reflect_enum, &ref_index, &ref_name);

    let EnumVariantOutputData {
        variant_names,
        variant_constructors,
        ..
    } = TryApplyVariantBuilder::new(reflect_enum).build(&ref_value);

    let hash_fn = reflect_enum
        .meta()
        .attrs()
        .get_hash_impl(bevy_reflect_path)
        .unwrap_or_else(|| {
            quote! {
                fn reflect_hash(&self) -> #FQOption<u64> {
                    #bevy_reflect_path::enum_hash(self)
                }
            }
        });
    let debug_fn = reflect_enum.meta().attrs().get_debug_impl();
    let partial_eq_fn = reflect_enum
        .meta()
        .attrs()
        .get_partial_eq_impl(bevy_reflect_path)
        .unwrap_or_else(|| {
            quote! {
                fn reflect_partial_eq(&self, value: &dyn #bevy_reflect_path::Reflect) -> #FQOption<bool> {
                    #bevy_reflect_path::enum_partial_eq(self, value)
                }
            }
        });

    let typed_impl = impl_typed(
        reflect_enum.meta(),
        &where_clause_options,
        reflect_enum.to_info_tokens(),
    );

    let type_path_impl = impl_type_path(reflect_enum.meta());

    let get_type_registration_impl = reflect_enum.get_type_registration(&where_clause_options);

    let (impl_generics, ty_generics, where_clause) =
        reflect_enum.meta().type_path().generics().split_for_impl();

    let where_reflect_clause = where_clause_options.extend_where_clause(where_clause);

    quote! {
        #get_type_registration_impl

        #typed_impl

        #type_path_impl

        impl #impl_generics #bevy_reflect_path::Enum for #enum_path #ty_generics #where_reflect_clause {
            fn field(&self, #ref_name: &str) -> #FQOption<&dyn #bevy_reflect_path::Reflect> {
                 match self {
                    #(#enum_field,)*
                    _ => #FQOption::None,
                }
            }

            fn field_at(&self, #ref_index: usize) -> #FQOption<&dyn #bevy_reflect_path::Reflect> {
                match self {
                    #(#enum_field_at,)*
                    _ => #FQOption::None,
                }
            }

            fn field_mut(&mut self, #ref_name: &str) -> #FQOption<&mut dyn #bevy_reflect_path::Reflect> {
                 match self {
                    #(#enum_field,)*
                    _ => #FQOption::None,
                }
            }

            fn field_at_mut(&mut self, #ref_index: usize) -> #FQOption<&mut dyn #bevy_reflect_path::Reflect> {
                match self {
                    #(#enum_field_at,)*
                    _ => #FQOption::None,
                }
            }

            fn index_of(&self, #ref_name: &str) -> #FQOption<usize> {
                 match self {
                    #(#enum_index_of,)*
                    _ => #FQOption::None,
                }
            }

            fn name_at(&self, #ref_index: usize) -> #FQOption<&str> {
                 match self {
                    #(#enum_name_at,)*
                    _ => #FQOption::None,
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
            fn variant_index(&self) -> usize {
                 match self {
                    #(#enum_variant_index,)*
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

        impl #impl_generics #bevy_reflect_path::Reflect for #enum_path #ty_generics #where_reflect_clause {
            #[inline]
            fn get_represented_type_info(&self) -> #FQOption<&'static #bevy_reflect_path::TypeInfo> {
                #FQOption::Some(<Self as #bevy_reflect_path::Typed>::type_info())
            }

            #[inline]
            fn into_any(self: #FQBox<Self>) -> #FQBox<dyn #FQAny> {
                self
            }

            #[inline]
            fn as_any(&self) -> &dyn #FQAny {
                self
            }

            #[inline]
            fn as_any_mut(&mut self) -> &mut dyn #FQAny {
                self
            }

            #[inline]
            fn into_reflect(self: #FQBox<Self>) -> #FQBox<dyn #bevy_reflect_path::Reflect> {
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
            fn clone_value(&self) -> #FQBox<dyn #bevy_reflect_path::Reflect> {
                #FQBox::new(#bevy_reflect_path::Enum::clone_dynamic(self))
            }

            #[inline]
            fn set(&mut self, #ref_value: #FQBox<dyn #bevy_reflect_path::Reflect>) -> #FQResult<(), #FQBox<dyn #bevy_reflect_path::Reflect>> {
                *self = <dyn #bevy_reflect_path::Reflect>::take(#ref_value)?;
                #FQResult::Ok(())
            }

            #[inline]
            fn try_apply(&mut self, #ref_value: &dyn #bevy_reflect_path::Reflect) -> #FQResult<(), #bevy_reflect_path::ApplyError>  {
                if let #bevy_reflect_path::ReflectRef::Enum(#ref_value) = #bevy_reflect_path::Reflect::reflect_ref(#ref_value) {
                    if #bevy_reflect_path::Enum::variant_name(self) == #bevy_reflect_path::Enum::variant_name(#ref_value) {
                        // Same variant -> just update fields
                        match #bevy_reflect_path::Enum::variant_type(#ref_value) {
                            #bevy_reflect_path::VariantType::Struct => {
                                for field in #bevy_reflect_path::Enum::iter_fields(#ref_value) {
                                    let name = field.name().unwrap();
                                    if let #FQOption::Some(v) = #bevy_reflect_path::Enum::field_mut(self, name) {
                                       #bevy_reflect_path::Reflect::try_apply(v, field.value())?;
                                    }
                                }
                            }
                            #bevy_reflect_path::VariantType::Tuple => {
                                for (index, field) in ::core::iter::Iterator::enumerate(#bevy_reflect_path::Enum::iter_fields(#ref_value)) {
                                    if let #FQOption::Some(v) = #bevy_reflect_path::Enum::field_at_mut(self, index) {
                                        #bevy_reflect_path::Reflect::try_apply(v, field.value())?;
                                    }
                                }
                            }
                            _ => {}
                        }
                    } else {
                        // New variant -> perform a switch
                        match #bevy_reflect_path::Enum::variant_name(#ref_value) {
                            #(#variant_names => {
                                *self = #variant_constructors
                            })*
                            name => {
                                return #FQResult::Err(
                                    #bevy_reflect_path::ApplyError::UnknownVariant {
                                        enum_name: ::core::convert::Into::into(#bevy_reflect_path::DynamicTypePath::reflect_type_path(self)),
                                        variant_name: ::core::convert::Into::into(name),
                                    }
                                );
                            }
                        }
                    }
                } else {
                    return #FQResult::Err(
                        #bevy_reflect_path::ApplyError::MismatchedKinds {
                            from_kind: #bevy_reflect_path::Reflect::reflect_kind(#ref_value),
                            to_kind: #bevy_reflect_path::ReflectKind::Enum,
                        }
                    );
                }
                #FQResult::Ok(())
            }

            fn reflect_kind(&self) -> #bevy_reflect_path::ReflectKind {
                #bevy_reflect_path::ReflectKind::Enum
            }

            fn reflect_ref(&self) -> #bevy_reflect_path::ReflectRef {
                #bevy_reflect_path::ReflectRef::Enum(self)
            }

            fn reflect_mut(&mut self) -> #bevy_reflect_path::ReflectMut {
                #bevy_reflect_path::ReflectMut::Enum(self)
            }

            fn reflect_owned(self: #FQBox<Self>) -> #bevy_reflect_path::ReflectOwned {
                #bevy_reflect_path::ReflectOwned::Enum(self)
            }

            #hash_fn

            #partial_eq_fn

            #debug_fn
        }
    }
}

struct EnumImpls {
    enum_field: Vec<proc_macro2::TokenStream>,
    enum_field_at: Vec<proc_macro2::TokenStream>,
    enum_index_of: Vec<proc_macro2::TokenStream>,
    enum_name_at: Vec<proc_macro2::TokenStream>,
    enum_field_len: Vec<proc_macro2::TokenStream>,
    enum_variant_name: Vec<proc_macro2::TokenStream>,
    enum_variant_index: Vec<proc_macro2::TokenStream>,
    enum_variant_type: Vec<proc_macro2::TokenStream>,
}

fn generate_impls(reflect_enum: &ReflectEnum, ref_index: &Ident, ref_name: &Ident) -> EnumImpls {
    let bevy_reflect_path = reflect_enum.meta().bevy_reflect_path();

    let mut enum_field = Vec::new();
    let mut enum_field_at = Vec::new();
    let mut enum_index_of = Vec::new();
    let mut enum_name_at = Vec::new();
    let mut enum_field_len = Vec::new();
    let mut enum_variant_name = Vec::new();
    let mut enum_variant_index = Vec::new();
    let mut enum_variant_type = Vec::new();

    for (variant_index, variant) in reflect_enum.variants().iter().enumerate() {
        let ident = &variant.data.ident;
        let name = ident.to_string();
        let unit = reflect_enum.get_unit(ident);

        let variant_type_ident = match variant.data.fields {
            Fields::Unit => Ident::new("Unit", Span::call_site()),
            Fields::Unnamed(..) => Ident::new("Tuple", Span::call_site()),
            Fields::Named(..) => Ident::new("Struct", Span::call_site()),
        };

        enum_variant_name.push(quote! {
            #unit{..} => #name
        });
        enum_variant_index.push(quote! {
            #unit{..} => #variant_index
        });
        enum_variant_type.push(quote! {
            #unit{..} => #bevy_reflect_path::VariantType::#variant_type_ident
        });

        fn process_fields(
            fields: &[StructField],
            mut f: impl FnMut(&StructField) + Sized,
        ) -> usize {
            let mut field_len = 0;
            for field in fields.iter() {
                if field.attrs.ignore.is_ignored() {
                    // Ignored field
                    continue;
                };

                f(field);

                field_len += 1;
            }

            field_len
        }

        match &variant.fields {
            EnumVariantFields::Unit => {
                let field_len = process_fields(&[], |_| {});

                enum_field_len.push(quote! {
                    #unit{..} => #field_len
                });
            }
            EnumVariantFields::Unnamed(fields) => {
                let field_len = process_fields(fields, |field: &StructField| {
                    let reflection_index = field
                        .reflection_index
                        .expect("reflection index should exist for active field");

                    let declare_field = syn::Index::from(field.declaration_index);
                    enum_field_at.push(quote! {
                        #unit { #declare_field : value, .. } if #ref_index == #reflection_index => #FQOption::Some(value)
                    });
                });

                enum_field_len.push(quote! {
                    #unit{..} => #field_len
                });
            }
            EnumVariantFields::Named(fields) => {
                let field_len = process_fields(fields, |field: &StructField| {
                    let field_ident = field.data.ident.as_ref().unwrap();
                    let field_name = field_ident.to_string();
                    let reflection_index = field
                        .reflection_index
                        .expect("reflection index should exist for active field");

                    enum_field.push(quote! {
                        #unit{ #field_ident, .. } if #ref_name == #field_name => #FQOption::Some(#field_ident)
                    });
                    enum_field_at.push(quote! {
                        #unit{ #field_ident, .. } if #ref_index == #reflection_index => #FQOption::Some(#field_ident)
                    });
                    enum_index_of.push(quote! {
                        #unit{ .. } if #ref_name == #field_name => #FQOption::Some(#reflection_index)
                    });
                    enum_name_at.push(quote! {
                        #unit{ .. } if #ref_index == #reflection_index => #FQOption::Some(#field_name)
                    });
                });

                enum_field_len.push(quote! {
                    #unit{..} => #field_len
                });
            }
        };
    }

    EnumImpls {
        enum_field,
        enum_field_at,
        enum_index_of,
        enum_name_at,
        enum_field_len,
        enum_variant_name,
        enum_variant_index,
        enum_variant_type,
    }
}
