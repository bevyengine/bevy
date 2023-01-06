use crate::fq_std::{FQAny, FQBox, FQDefault, FQOption, FQResult};
use crate::impls::impl_typed;
use crate::ReflectStruct;
use proc_macro::TokenStream;
use quote::{quote, ToTokens};
use syn::{Index, Member};

/// Implements `Struct`, `GetTypeRegistration`, and `Reflect` for the given derive data.
pub(crate) fn impl_struct(reflect_struct: &ReflectStruct) -> TokenStream {
    let fqoption = FQOption.into_token_stream();

    let bevy_reflect_path = reflect_struct.meta().bevy_reflect_path();
    let struct_name = reflect_struct.meta().type_name();

    let field_names = reflect_struct
        .active_fields()
        .map(|field| {
            field
                .data
                .ident
                .as_ref()
                .map(|i| i.to_string())
                .unwrap_or_else(|| field.index.to_string())
        })
        .collect::<Vec<String>>();
    let field_idents = reflect_struct
        .active_fields()
        .map(|field| {
            field
                .data
                .ident
                .as_ref()
                .map(|ident| Member::Named(ident.clone()))
                .unwrap_or_else(|| Member::Unnamed(Index::from(field.index)))
        })
        .collect::<Vec<_>>();
    let field_types = reflect_struct.active_types();
    let field_count = field_idents.len();
    let field_indices = (0..field_count).collect::<Vec<usize>>();

    let hash_fn = reflect_struct
        .meta()
        .traits()
        .get_hash_impl(bevy_reflect_path);
    let debug_fn = reflect_struct.meta().traits().get_debug_impl();
    let partial_eq_fn = reflect_struct.meta()
        .traits()
        .get_partial_eq_impl(bevy_reflect_path)
        .unwrap_or_else(|| {
            quote! {
                fn reflect_partial_eq(&self, value: &dyn #bevy_reflect_path::Reflect) -> #FQOption<bool> {
                    #bevy_reflect_path::struct_partial_eq(self, value)
                }
            }
        });

    #[cfg(feature = "documentation")]
    let field_generator = {
        let docs = reflect_struct
            .active_fields()
            .map(|field| quote::ToTokens::to_token_stream(&field.doc));
        quote! {
            #(#bevy_reflect_path::NamedField::new::<#field_types>(#field_names).with_docs(#docs) ,)*
        }
    };

    #[cfg(not(feature = "documentation"))]
    let field_generator = {
        quote! {
            #(#bevy_reflect_path::NamedField::new::<#field_types>(#field_names) ,)*
        }
    };

    let string_name = struct_name.to_string();

    #[cfg(feature = "documentation")]
    let info_generator = {
        let doc = reflect_struct.meta().doc();
        quote! {
            #bevy_reflect_path::StructInfo::new::<Self>(#string_name, &fields).with_docs(#doc)
        }
    };

    #[cfg(not(feature = "documentation"))]
    let info_generator = {
        quote! {
            #bevy_reflect_path::StructInfo::new::<Self>(#string_name, &fields)
        }
    };

    let typed_impl = impl_typed(
        struct_name,
        reflect_struct.meta().generics(),
        quote! {
            let fields = [#field_generator];
            let info = #info_generator;
            #bevy_reflect_path::TypeInfo::Struct(info)
        },
        bevy_reflect_path,
    );

    let get_type_registration_impl = reflect_struct.get_type_registration();
    let (impl_generics, ty_generics, where_clause) =
        reflect_struct.meta().generics().split_for_impl();

    TokenStream::from(quote! {
        #get_type_registration_impl

        #typed_impl

        impl #impl_generics #bevy_reflect_path::Struct for #struct_name #ty_generics #where_clause {
            fn field(&self, name: &str) -> #FQOption<&dyn #bevy_reflect_path::Reflect> {
                match name {
                    #(#field_names => #fqoption::Some(&self.#field_idents),)*
                    _ => #FQOption::None,
                }
            }

            fn field_mut(&mut self, name: &str) -> #FQOption<&mut dyn #bevy_reflect_path::Reflect> {
                match name {
                    #(#field_names => #fqoption::Some(&mut self.#field_idents),)*
                    _ => #FQOption::None,
                }
            }

            fn field_at(&self, index: usize) -> #FQOption<&dyn #bevy_reflect_path::Reflect> {
                match index {
                    #(#field_indices => #fqoption::Some(&self.#field_idents),)*
                    _ => #FQOption::None,
                }
            }

            fn field_at_mut(&mut self, index: usize) -> #FQOption<&mut dyn #bevy_reflect_path::Reflect> {
                match index {
                    #(#field_indices => #fqoption::Some(&mut self.#field_idents),)*
                    _ => #FQOption::None,
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
                dynamic.set_name(::std::string::ToString::to_string(#bevy_reflect_path::Reflect::type_name(self)));
                #(dynamic.insert_boxed(#field_names, #bevy_reflect_path::Reflect::clone_value(&self.#field_idents));)*
                dynamic
            }
        }

        impl #impl_generics #bevy_reflect_path::Reflect for #struct_name #ty_generics #where_clause {
            #[inline]
            fn type_name(&self) -> &str {
                ::core::any::type_name::<Self>()
            }

            #[inline]
            fn get_type_info(&self) -> &'static #bevy_reflect_path::TypeInfo {
                <Self as #bevy_reflect_path::Typed>::type_info()
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
                #FQBox::new(#bevy_reflect_path::Struct::clone_dynamic(self))
            }

            #[inline]
            fn set(&mut self, value: #FQBox<dyn #bevy_reflect_path::Reflect>) -> #FQResult<(), #FQBox<dyn #bevy_reflect_path::Reflect>> {
                *self = <dyn #bevy_reflect_path::Reflect>::take(value)?;
                #FQResult::Ok(())
            }

            #[inline]
            fn apply(&mut self, value: &dyn #bevy_reflect_path::Reflect) {
                if let #bevy_reflect_path::ReflectRef::Struct(struct_value) = #bevy_reflect_path::Reflect::reflect_ref(value) {
                    for (i, value) in ::core::iter::Iterator::enumerate(#bevy_reflect_path::Struct::iter_fields(struct_value)) {
                        let name = #bevy_reflect_path::Struct::name_at(struct_value, i).unwrap();
                        #bevy_reflect_path::Struct::field_mut(self, name).map(|v| v.apply(value));
                    }
                } else {
                    panic!("Attempted to apply non-struct type to struct type.");
                }
            }

            fn reflect_ref(&self) -> #bevy_reflect_path::ReflectRef {
                #bevy_reflect_path::ReflectRef::Struct(self)
            }

            fn reflect_mut(&mut self) -> #bevy_reflect_path::ReflectMut {
                #bevy_reflect_path::ReflectMut::Struct(self)
            }

            fn reflect_owned(self: #FQBox<Self>) -> #bevy_reflect_path::ReflectOwned {
                #bevy_reflect_path::ReflectOwned::Struct(self)
            }

            #hash_fn

            #partial_eq_fn

            #debug_fn
        }
    })
}
