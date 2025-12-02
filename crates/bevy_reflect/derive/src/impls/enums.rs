use crate::{
    derive_data::{EnumVariantFields, ReflectEnum, StructField},
    enum_utility::{EnumVariantOutputData, TryApplyVariantBuilder, VariantBuilder},
    impls::{common_partial_reflect_methods, impl_full_reflect, impl_type_path, impl_typed},
};
use bevy_macro_utils::fq_std::{FQOption, FQResult};
use proc_macro2::{Ident, Span};
use quote::quote;
use syn::{Fields, Path};

pub(crate) fn impl_enum(reflect_enum: &ReflectEnum) -> proc_macro2::TokenStream {
    let bevy_reflect_path = reflect_enum.meta().bevy_reflect_path();
    let enum_path = reflect_enum.meta().type_path();
    let is_remote = reflect_enum.meta().is_remote_wrapper();

    // For `match self` expressions where self is a reference
    let match_this = if is_remote {
        quote!(&self.0)
    } else {
        quote!(self)
    };
    // For `match self` expressions where self is a mutable reference
    let match_this_mut = if is_remote {
        quote!(&mut self.0)
    } else {
        quote!(self)
    };
    // For `*self` assignments
    let deref_this = if is_remote {
        quote!(self.0)
    } else {
        quote!(*self)
    };

    let ref_name = Ident::new("__name_param", Span::call_site());
    let ref_index = Ident::new("__index_param", Span::call_site());
    let ref_value = Ident::new("__value_param", Span::call_site());

    let EnumImpls {
        enum_field,
        enum_field_mut,
        enum_field_at,
        enum_field_at_mut,
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

    let where_clause_options = reflect_enum.where_clause_options();
    let typed_impl = impl_typed(&where_clause_options, reflect_enum.to_info_tokens());

    let type_path_impl = impl_type_path(reflect_enum.meta());
    let full_reflect_impl = impl_full_reflect(&where_clause_options);
    let common_methods = common_partial_reflect_methods(
        reflect_enum.meta(),
        || Some(quote!(#bevy_reflect_path::enum_partial_eq)),
        || Some(quote!(#bevy_reflect_path::enum_hash)),
    );
    let clone_fn = reflect_enum.get_clone_impl();

    #[cfg(not(feature = "functions"))]
    let function_impls = None::<proc_macro2::TokenStream>;
    #[cfg(feature = "functions")]
    let function_impls = crate::impls::impl_function_traits(&where_clause_options);

    let get_type_registration_impl = reflect_enum.get_type_registration(&where_clause_options);

    let (impl_generics, ty_generics, where_clause) =
        reflect_enum.meta().type_path().generics().split_for_impl();

    #[cfg(not(feature = "auto_register"))]
    let auto_register = None::<proc_macro2::TokenStream>;
    #[cfg(feature = "auto_register")]
    let auto_register = crate::impls::reflect_auto_registration(reflect_enum.meta());

    let where_reflect_clause = where_clause_options.extend_where_clause(where_clause);

    quote! {
        #get_type_registration_impl

        #typed_impl

        #type_path_impl

        #full_reflect_impl

        #function_impls

        #auto_register

        impl #impl_generics #bevy_reflect_path::Enum for #enum_path #ty_generics #where_reflect_clause {
            fn field(&self, #ref_name: &str) -> #FQOption<&dyn #bevy_reflect_path::PartialReflect> {
                 match #match_this {
                    #(#enum_field,)*
                    _ => #FQOption::None,
                }
            }

            fn field_at(&self, #ref_index: usize) -> #FQOption<&dyn #bevy_reflect_path::PartialReflect> {
                match #match_this {
                    #(#enum_field_at,)*
                    _ => #FQOption::None,
                }
            }

            fn field_mut(&mut self, #ref_name: &str) -> #FQOption<&mut dyn #bevy_reflect_path::PartialReflect> {
                 match #match_this_mut {
                    #(#enum_field_mut,)*
                    _ => #FQOption::None,
                }
            }

            fn field_at_mut(&mut self, #ref_index: usize) -> #FQOption<&mut dyn #bevy_reflect_path::PartialReflect> {
                match #match_this_mut {
                    #(#enum_field_at_mut,)*
                    _ => #FQOption::None,
                }
            }

            fn index_of(&self, #ref_name: &str) -> #FQOption<usize> {
                 match #match_this {
                    #(#enum_index_of,)*
                    _ => #FQOption::None,
                }
            }

            fn name_at(&self, #ref_index: usize) -> #FQOption<&str> {
                 match #match_this {
                    #(#enum_name_at,)*
                    _ => #FQOption::None,
                }
            }

            fn iter_fields(&self) -> #bevy_reflect_path::VariantFieldIter {
                #bevy_reflect_path::VariantFieldIter::new(self)
            }

            #[inline]
            fn field_len(&self) -> usize {
                 match #match_this {
                    #(#enum_field_len,)*
                    _ => 0,
                }
            }

            #[inline]
            fn variant_name(&self) -> &str {
                 match #match_this {
                    #(#enum_variant_name,)*
                    _ => unreachable!(),
                }
            }

            #[inline]
            fn variant_index(&self) -> usize {
                 match #match_this {
                    #(#enum_variant_index,)*
                    _ => unreachable!(),
                }
            }

            #[inline]
            fn variant_type(&self) -> #bevy_reflect_path::VariantType {
                 match #match_this {
                    #(#enum_variant_type,)*
                    _ => unreachable!(),
                }
            }

            fn to_dynamic_enum(&self) -> #bevy_reflect_path::DynamicEnum {
                #bevy_reflect_path::DynamicEnum::from_ref::<Self>(self)
            }
        }

        impl #impl_generics #bevy_reflect_path::PartialReflect for #enum_path #ty_generics #where_reflect_clause {
            #[inline]
            fn get_represented_type_info(&self) -> #FQOption<&'static #bevy_reflect_path::TypeInfo> {
                #FQOption::Some(<Self as #bevy_reflect_path::Typed>::type_info())
            }

            #[inline]
            fn try_apply(
                &mut self,
                #ref_value: &dyn #bevy_reflect_path::PartialReflect
            ) -> #FQResult<(), #bevy_reflect_path::ApplyError>  {
                if let #bevy_reflect_path::ReflectRef::Enum(#ref_value) =
                    #bevy_reflect_path::PartialReflect::reflect_ref(#ref_value) {
                    if #bevy_reflect_path::Enum::variant_name(self) == #bevy_reflect_path::Enum::variant_name(#ref_value) {
                        // Same variant -> just update fields
                        match #bevy_reflect_path::Enum::variant_type(#ref_value) {
                            #bevy_reflect_path::VariantType::Struct => {
                                for field in #bevy_reflect_path::Enum::iter_fields(#ref_value) {
                                    let name = field.name().unwrap();
                                    if let #FQOption::Some(v) = #bevy_reflect_path::Enum::field_mut(self, name) {
                                       #bevy_reflect_path::PartialReflect::try_apply(v, field.value())?;
                                    }
                                }
                            }
                            #bevy_reflect_path::VariantType::Tuple => {
                                for (index, field) in ::core::iter::Iterator::enumerate(#bevy_reflect_path::Enum::iter_fields(#ref_value)) {
                                    if let #FQOption::Some(v) = #bevy_reflect_path::Enum::field_at_mut(self, index) {
                                        #bevy_reflect_path::PartialReflect::try_apply(v, field.value())?;
                                    }
                                }
                            }
                            _ => {}
                        }
                    } else {
                        // New variant -> perform a switch
                        match #bevy_reflect_path::Enum::variant_name(#ref_value) {
                            #(#variant_names => {
                                #deref_this = #variant_constructors
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
                            from_kind: #bevy_reflect_path::PartialReflect::reflect_kind(#ref_value),
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

            fn reflect_owned(self: #bevy_reflect_path::__macro_exports::alloc_utils::Box<Self>) -> #bevy_reflect_path::ReflectOwned {
                #bevy_reflect_path::ReflectOwned::Enum(self)
            }

            #common_methods

            #clone_fn
        }
    }
}

struct EnumImpls {
    enum_field: Vec<proc_macro2::TokenStream>,
    enum_field_mut: Vec<proc_macro2::TokenStream>,
    enum_field_at: Vec<proc_macro2::TokenStream>,
    enum_field_at_mut: Vec<proc_macro2::TokenStream>,
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
    let mut enum_field_mut = Vec::new();
    let mut enum_field_at = Vec::new();
    let mut enum_field_at_mut = Vec::new();
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

        /// Process the field value to account for remote types.
        ///
        /// If the field is a remote type, then the value will be transmuted accordingly.
        fn process_field_value(
            ident: &Ident,
            field: &StructField,
            is_mutable: bool,
            bevy_reflect_path: &Path,
        ) -> proc_macro2::TokenStream {
            let method = if is_mutable {
                quote!(as_wrapper_mut)
            } else {
                quote!(as_wrapper)
            };

            field
                .attrs
                .remote
                .as_ref()
                .map(|ty| quote!(<#ty as #bevy_reflect_path::ReflectRemote>::#method(#ident)))
                .unwrap_or_else(|| quote!(#ident))
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

                    let __value = Ident::new("__value", Span::call_site());
                    let value_ref = process_field_value(&__value, field, false, bevy_reflect_path);
                    let value_mut = process_field_value(&__value, field, true, bevy_reflect_path);

                    enum_field_at.push(quote! {
                        #unit { #declare_field : #__value, .. } if #ref_index == #reflection_index => #FQOption::Some(#value_ref)
                    });
                    enum_field_at_mut.push(quote! {
                        #unit { #declare_field : #__value, .. } if #ref_index == #reflection_index => #FQOption::Some(#value_mut)
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

                    let __value = Ident::new("__value", Span::call_site());
                    let value_ref = process_field_value(&__value, field, false, bevy_reflect_path);
                    let value_mut = process_field_value(&__value, field, true, bevy_reflect_path);

                    enum_field.push(quote! {
                        #unit{ #field_ident: #__value, .. } if #ref_name == #field_name => #FQOption::Some(#value_ref)
                    });
                    enum_field_mut.push(quote! {
                        #unit{ #field_ident: #__value, .. } if #ref_name == #field_name => #FQOption::Some(#value_mut)
                    });
                    enum_field_at.push(quote! {
                        #unit{ #field_ident: #__value, .. } if #ref_index == #reflection_index => #FQOption::Some(#value_ref)
                    });
                    enum_field_at_mut.push(quote! {
                        #unit{ #field_ident: #__value, .. } if #ref_index == #reflection_index => #FQOption::Some(#value_mut)
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
        enum_field_mut,
        enum_field_at,
        enum_field_at_mut,
        enum_index_of,
        enum_name_at,
        enum_field_len,
        enum_variant_name,
        enum_variant_index,
        enum_variant_type,
    }
}
