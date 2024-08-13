use crate::impls::{common_partial_reflect_methods, impl_full_reflect, impl_type_path, impl_typed};
use crate::struct_utility::FieldAccessors;
use crate::ReflectStruct;
use bevy_macro_utils::fq_std::{FQBox, FQCow, FQDefault, FQOption, FQResult};
use quote::{quote, ToTokens};

/// Implements `Struct`, `GetTypeRegistration`, and `Reflect` for the given derive data.
pub(crate) fn impl_struct(reflect_struct: &ReflectStruct) -> proc_macro2::TokenStream {
    let fqoption = FQOption.into_token_stream();
    let fqresult = FQResult.into_token_stream();

    let bevy_reflect_path = reflect_struct.meta().bevy_reflect_path();
    let struct_path = reflect_struct.meta().type_path();

    let field_names = reflect_struct
        .active_fields()
        .map(|field| {
            field
                .data
                .ident
                .as_ref()
                .map(ToString::to_string)
                .unwrap_or_else(|| field.declaration_index.to_string())
        })
        .collect::<Vec<String>>();

    let FieldAccessors {
        fields_ref,
        fields_mut,
        field_indices,
        field_count,
        ..
    } = FieldAccessors::new(reflect_struct);

    let where_clause_options = reflect_struct.where_clause_options();
    let typed_impl = impl_typed(
        reflect_struct.meta(),
        &where_clause_options,
        reflect_struct.to_info_tokens(false),
    );

    let type_path_impl = impl_type_path(reflect_struct.meta());
    let full_reflect_impl = impl_full_reflect(reflect_struct.meta(), &where_clause_options);
    let common_methods = common_partial_reflect_methods(
        reflect_struct.meta(),
        || Some(quote!(#bevy_reflect_path::struct_partial_eq)),
        || None,
    );

    #[cfg(not(feature = "functions"))]
    let function_impls = None::<proc_macro2::TokenStream>;
    #[cfg(feature = "functions")]
    let function_impls =
        crate::impls::impl_function_traits(reflect_struct.meta(), &where_clause_options);

    let get_type_registration_impl = reflect_struct.get_type_registration(&where_clause_options);

    let (impl_generics, ty_generics, where_clause) = reflect_struct
        .meta()
        .type_path()
        .generics()
        .split_for_impl();

    let where_reflect_clause = where_clause_options.extend_where_clause(where_clause);

    quote! {
        #get_type_registration_impl

        #typed_impl

        #type_path_impl

        #full_reflect_impl

        #function_impls

        impl #impl_generics #bevy_reflect_path::Struct for #struct_path #ty_generics #where_reflect_clause {
            fn field(&self, name: &str) -> #FQResult<&dyn #bevy_reflect_path::PartialReflect, #bevy_reflect_path::error::ReflectFieldError> {
                match name {
                    #(#field_names => #fqresult::Ok(#fields_ref),)*
                    _ => #FQResult::Err(#bevy_reflect_path::error::ReflectFieldError::DoesNotExist {
                        field: #bevy_reflect_path::FieldId::Named(::std::convert::Into::into(name.to_string())),
                        container_type_path: #FQCow::Borrowed(<Self as #bevy_reflect_path::TypePath>::type_path()),
                    }),
                }
            }

            fn field_mut(&mut self, name: &str) -> #FQResult<&mut dyn #bevy_reflect_path::PartialReflect, #bevy_reflect_path::error::ReflectFieldError> {
                match name {
                    #(#field_names => #fields_mut,)*
                    _ => #FQResult::Err(#bevy_reflect_path::error::ReflectFieldError::DoesNotExist {
                        field: #bevy_reflect_path::FieldId::Named(::std::convert::Into::into(name.to_string())),
                        container_type_path: #FQCow::Borrowed(<Self as #bevy_reflect_path::TypePath>::type_path()),
                    }),
                }
            }

            fn field_at(&self, index: usize) -> #FQResult<&dyn #bevy_reflect_path::PartialReflect, #bevy_reflect_path::error::ReflectFieldError> {
                match index {
                    #(#field_indices => #fqresult::Ok(#fields_ref),)*
                    _ => #FQResult::Err(#bevy_reflect_path::error::ReflectFieldError::DoesNotExist {
                        field: #bevy_reflect_path::FieldId::Unnamed(index),
                        container_type_path: #FQCow::Borrowed(<Self as #bevy_reflect_path::TypePath>::type_path()),
                    }),
                }
            }

            fn field_at_mut(&mut self, index: usize) -> #FQResult<&mut dyn #bevy_reflect_path::PartialReflect, #bevy_reflect_path::error::ReflectFieldError> {
                match index {
                    #(#field_indices => #fields_mut,)*
                    _ => #FQResult::Err(#bevy_reflect_path::error::ReflectFieldError::DoesNotExist {
                        field: #bevy_reflect_path::FieldId::Unnamed(index),
                        container_type_path: #FQCow::Borrowed(<Self as #bevy_reflect_path::TypePath>::type_path()),
                    }),
                }
            }

            fn name_at(&self, index: usize) -> #FQOption<&str> {
                match index {
                    #(#field_indices => #fqoption::Some(#field_names),)*
                    _ => #FQOption::None,
                }
            }

            fn field_len(&self) -> usize {
                #field_count
            }

            fn iter_fields(&self) -> #bevy_reflect_path::FieldIter {
                #bevy_reflect_path::FieldIter::new(self)
            }

            fn clone_dynamic(&self) -> #bevy_reflect_path::DynamicStruct {
                let mut dynamic: #bevy_reflect_path::DynamicStruct = #FQDefault::default();
                dynamic.set_represented_type(#bevy_reflect_path::PartialReflect::get_represented_type_info(self));
                #(dynamic.insert_boxed(#field_names, #bevy_reflect_path::PartialReflect::clone_value(#fields_ref));)*
                dynamic
            }
        }

        impl #impl_generics #bevy_reflect_path::PartialReflect for #struct_path #ty_generics #where_reflect_clause {
            #[inline]
            fn get_represented_type_info(&self) -> #FQOption<&'static #bevy_reflect_path::TypeInfo> {
                #FQOption::Some(<Self as #bevy_reflect_path::Typed>::type_info())
            }

            #[inline]
            fn clone_value(&self) -> #FQBox<dyn #bevy_reflect_path::PartialReflect> {
                #FQBox::new(#bevy_reflect_path::Struct::clone_dynamic(self))
            }

            #[inline]
            fn try_apply(
                &mut self,
                value: &dyn #bevy_reflect_path::PartialReflect
            ) -> #FQResult<(), #bevy_reflect_path::ApplyError> {
                if let #bevy_reflect_path::ReflectRef::Struct(struct_value)
                    = #bevy_reflect_path::PartialReflect::reflect_ref(value) {
                    for (i, value) in ::core::iter::Iterator::enumerate(#bevy_reflect_path::Struct::iter_fields(struct_value)) {
                        let name = #bevy_reflect_path::Struct::name_at(struct_value, i).unwrap();
                        if let #FQResult::Ok(v) = #bevy_reflect_path::Struct::field_mut(self, name) {
                           #bevy_reflect_path::PartialReflect::try_apply(v, value)?;
                        }
                    }
                } else {
                    return #FQResult::Err(
                        #bevy_reflect_path::ApplyError::MismatchedKinds {
                            from_kind: #bevy_reflect_path::PartialReflect::reflect_kind(value),
                            to_kind: #bevy_reflect_path::ReflectKind::Struct
                        }
                    );
                }
                #FQResult::Ok(())
            }
            #[inline]
            fn reflect_kind(&self) -> #bevy_reflect_path::ReflectKind {
                #bevy_reflect_path::ReflectKind::Struct
            }
            #[inline]
            fn reflect_ref(&self) -> #bevy_reflect_path::ReflectRef {
                #bevy_reflect_path::ReflectRef::Struct(self)
            }
            #[inline]
            fn reflect_mut(&mut self) -> #bevy_reflect_path::ReflectMut {
                #bevy_reflect_path::ReflectMut::Struct(self)
            }
            #[inline]
            fn reflect_owned(self: #FQBox<Self>) -> #bevy_reflect_path::ReflectOwned {
                #bevy_reflect_path::ReflectOwned::Struct(self)
            }

            #common_methods
        }
    }
}
