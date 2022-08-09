use crate::impls::impl_typed;
use crate::ReflectStruct;
use proc_macro::TokenStream;
use quote::quote;
use syn::{Index, Member};

/// Implements `Struct`, `GetTypeRegistration`, and `Reflect` for the given derive data.
pub(crate) fn impl_struct(reflect_struct: &ReflectStruct) -> TokenStream {
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
                fn reflect_partial_eq(&self, value: &dyn #bevy_reflect_path::Reflect) -> Option<bool> {
                    #bevy_reflect_path::struct_partial_eq(self, value)
                }
            }
        });

    let typed_impl = impl_typed(
        struct_name,
        reflect_struct.meta().generics(),
        quote! {
           let fields = [
                #(#bevy_reflect_path::NamedField::new::<#field_types, _>(#field_names),)*
            ];
            let info = #bevy_reflect_path::StructInfo::new::<Self>(&fields);
            #bevy_reflect_path::TypeInfo::Struct(info)
        },
        bevy_reflect_path,
    );

    let get_type_registration_impl = reflect_struct.meta().get_type_registration();
    let (impl_generics, ty_generics, where_clause) =
        reflect_struct.meta().generics().split_for_impl();

    TokenStream::from(quote! {
        #get_type_registration_impl

        #typed_impl

        impl #impl_generics #bevy_reflect_path::Struct for #struct_name #ty_generics #where_clause {
            fn field(&self, name: &str) -> Option<&dyn #bevy_reflect_path::Reflect> {
                match name {
                    #(#field_names => Some(&self.#field_idents),)*
                    _ => None,
                }
            }

            fn field_mut(&mut self, name: &str) -> Option<&mut dyn #bevy_reflect_path::Reflect> {
                match name {
                    #(#field_names => Some(&mut self.#field_idents),)*
                    _ => None,
                }
            }

            fn field_at(&self, index: usize) -> Option<&dyn #bevy_reflect_path::Reflect> {
                match index {
                    #(#field_indices => Some(&self.#field_idents),)*
                    _ => None,
                }
            }

            fn field_at_mut(&mut self, index: usize) -> Option<&mut dyn #bevy_reflect_path::Reflect> {
                match index {
                    #(#field_indices => Some(&mut self.#field_idents),)*
                    _ => None,
                }
            }

            fn name_at(&self, index: usize) -> Option<&str> {
                match index {
                    #(#field_indices => Some(#field_names),)*
                    _ => None,
                }
            }

            fn field_len(&self) -> usize {
                #field_count
            }

            fn iter_fields(&self) -> #bevy_reflect_path::FieldIter {
                #bevy_reflect_path::FieldIter::new(self)
            }

            fn clone_dynamic(&self) -> #bevy_reflect_path::DynamicStruct {
                let mut dynamic = #bevy_reflect_path::DynamicStruct::default();
                dynamic.set_name(self.type_name().to_string());
                #(dynamic.insert_boxed(#field_names, self.#field_idents.clone_value());)*
                dynamic
            }
        }

        impl #impl_generics #bevy_reflect_path::Reflect for #struct_name #ty_generics #where_clause {
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
                Box::new(#bevy_reflect_path::Struct::clone_dynamic(self))
            }
            #[inline]
            fn set(&mut self, value: Box<dyn #bevy_reflect_path::Reflect>) -> Result<(), Box<dyn #bevy_reflect_path::Reflect>> {
                *self = value.take()?;
                Ok(())
            }

            #[inline]
            fn apply(&mut self, value: &dyn #bevy_reflect_path::Reflect) {
                if let #bevy_reflect_path::ReflectRef::Struct(struct_value) = value.reflect_ref() {
                    for (i, value) in struct_value.iter_fields().enumerate() {
                        let name = struct_value.name_at(i).unwrap();
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

            #hash_fn

            #partial_eq_fn

            #debug_fn
        }
    })
}
