use crate::container_attributes::ReflectTraits;
use crate::ReflectDeriveData;
use proc_macro::TokenStream;
use proc_macro2::Ident;
use quote::quote;
use syn::{Generics, Index, Member, Path};

/// Implements `Struct`, `GetTypeRegistration`, and `Reflect` for the given derive data.
pub(crate) fn impl_struct(derive_data: &ReflectDeriveData) -> TokenStream {
    let bevy_reflect_path = derive_data.bevy_reflect_path();
    let struct_name = derive_data.type_name();

    let field_names = derive_data
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
    let field_idents = derive_data
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
    let field_count = field_idents.len();
    let field_indices = (0..field_count).collect::<Vec<usize>>();

    let hash_fn = derive_data
        .traits()
        .get_hash_impl(bevy_reflect_path)
        .unwrap_or_else(|| quote!(None));
    let serialize_fn = derive_data
        .traits()
        .get_serialize_impl(bevy_reflect_path)
        .unwrap_or_else(|| quote!(None));
    let partial_eq_fn = derive_data
        .traits()
        .get_partial_eq_impl()
        .unwrap_or_else(|| {
            quote! {
                #bevy_reflect_path::struct_partial_eq(self, value)
            }
        });

    let get_type_registration_impl = derive_data.get_type_registration();
    let (impl_generics, ty_generics, where_clause) = derive_data.generics().split_for_impl();

    TokenStream::from(quote! {
        #get_type_registration_impl

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

        // SAFE: any and any_mut both return self
        unsafe impl #impl_generics #bevy_reflect_path::Reflect for #struct_name #ty_generics #where_clause {
            #[inline]
            fn type_name(&self) -> &str {
                std::any::type_name::<Self>()
            }

            #[inline]
            fn any(&self) -> &dyn std::any::Any {
                self
            }
            #[inline]
            fn any_mut(&mut self) -> &mut dyn std::any::Any {
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

            fn serializable(&self) -> Option<#bevy_reflect_path::serde::Serializable> {
                #serialize_fn
            }

            fn reflect_hash(&self) -> Option<u64> {
                #hash_fn
            }

            fn reflect_partial_eq(&self, value: &dyn #bevy_reflect_path::Reflect) -> Option<bool> {
                #partial_eq_fn
            }
        }
    })
}

/// Implements `TupleStruct`, `GetTypeRegistration`, and `Reflect` for the given derive data.
pub(crate) fn impl_tuple_struct(derive_data: &ReflectDeriveData) -> TokenStream {
    let bevy_reflect_path = derive_data.bevy_reflect_path();
    let struct_name = derive_data.type_name();
    let get_type_registration_impl = derive_data.get_type_registration();

    let field_idents = derive_data
        .active_fields()
        .map(|field| Member::Unnamed(Index::from(field.index)))
        .collect::<Vec<_>>();
    let field_count = field_idents.len();
    let field_indices = (0..field_count).collect::<Vec<usize>>();

    let hash_fn = derive_data
        .traits()
        .get_hash_impl(bevy_reflect_path)
        .unwrap_or_else(|| quote!(None));
    let serialize_fn = derive_data
        .traits()
        .get_serialize_impl(bevy_reflect_path)
        .unwrap_or_else(|| quote!(None));
    let partial_eq_fn = derive_data
        .traits()
        .get_partial_eq_impl()
        .unwrap_or_else(|| {
            quote! {
                #bevy_reflect_path::tuple_struct_partial_eq(self, value)
            }
        });

    let (impl_generics, ty_generics, where_clause) = derive_data.generics().split_for_impl();
    TokenStream::from(quote! {
        #get_type_registration_impl

        impl #impl_generics #bevy_reflect_path::TupleStruct for #struct_name #ty_generics #where_clause {
            fn field(&self, index: usize) -> Option<&dyn #bevy_reflect_path::Reflect> {
                match index {
                    #(#field_indices => Some(&self.#field_idents),)*
                    _ => None,
                }
            }

            fn field_mut(&mut self, index: usize) -> Option<&mut dyn #bevy_reflect_path::Reflect> {
                match index {
                    #(#field_indices => Some(&mut self.#field_idents),)*
                    _ => None,
                }
            }

            fn field_len(&self) -> usize {
                #field_count
            }

            fn iter_fields(&self) -> #bevy_reflect_path::TupleStructFieldIter {
                #bevy_reflect_path::TupleStructFieldIter::new(self)
            }

            fn clone_dynamic(&self) -> #bevy_reflect_path::DynamicTupleStruct {
                let mut dynamic = #bevy_reflect_path::DynamicTupleStruct::default();
                dynamic.set_name(self.type_name().to_string());
                #(dynamic.insert_boxed(self.#field_idents.clone_value());)*
                dynamic
            }
        }

        // SAFE: any and any_mut both return self
        unsafe impl #impl_generics #bevy_reflect_path::Reflect for #struct_name #ty_generics #where_clause {
            #[inline]
            fn type_name(&self) -> &str {
                std::any::type_name::<Self>()
            }

            #[inline]
            fn any(&self) -> &dyn std::any::Any {
                self
            }
            #[inline]
            fn any_mut(&mut self) -> &mut dyn std::any::Any {
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
                Box::new(#bevy_reflect_path::TupleStruct::clone_dynamic(self))
            }
            #[inline]
            fn set(&mut self, value: Box<dyn #bevy_reflect_path::Reflect>) -> Result<(), Box<dyn #bevy_reflect_path::Reflect>> {
                *self = value.take()?;
                Ok(())
            }

            #[inline]
            fn apply(&mut self, value: &dyn #bevy_reflect_path::Reflect) {
                if let #bevy_reflect_path::ReflectRef::TupleStruct(struct_value) = value.reflect_ref() {
                    for (i, value) in struct_value.iter_fields().enumerate() {
                        #bevy_reflect_path::TupleStruct::field_mut(self, i).map(|v| v.apply(value));
                    }
                } else {
                    panic!("Attempted to apply non-TupleStruct type to TupleStruct type.");
                }
            }

            fn reflect_ref(&self) -> #bevy_reflect_path::ReflectRef {
                #bevy_reflect_path::ReflectRef::TupleStruct(self)
            }

            fn reflect_mut(&mut self) -> #bevy_reflect_path::ReflectMut {
                #bevy_reflect_path::ReflectMut::TupleStruct(self)
            }

            fn serializable(&self) -> Option<#bevy_reflect_path::serde::Serializable> {
                #serialize_fn
            }

            fn reflect_hash(&self) -> Option<u64> {
                #hash_fn
            }

            fn reflect_partial_eq(&self, value: &dyn #bevy_reflect_path::Reflect) -> Option<bool> {
                #partial_eq_fn
            }
        }
    })
}

/// Implements `GetTypeRegistration` and `Reflect` for the given type data.
pub(crate) fn impl_value(
    type_name: &Ident,
    generics: &Generics,
    get_type_registration_impl: proc_macro2::TokenStream,
    bevy_reflect_path: &Path,
    reflect_attrs: &ReflectTraits,
) -> TokenStream {
    let hash_fn = reflect_attrs
        .get_hash_impl(bevy_reflect_path)
        .unwrap_or_else(|| quote!(None));
    let partial_eq_fn = reflect_attrs
        .get_partial_eq_impl()
        .unwrap_or_else(|| quote!(None));
    let serialize_fn = reflect_attrs
        .get_serialize_impl(bevy_reflect_path)
        .unwrap_or_else(|| quote!(None));

    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
    TokenStream::from(quote! {
        #get_type_registration_impl

        // SAFE: any and any_mut both return self
        unsafe impl #impl_generics #bevy_reflect_path::Reflect for #type_name #ty_generics #where_clause  {
            #[inline]
            fn type_name(&self) -> &str {
                std::any::type_name::<Self>()
            }

            #[inline]
            fn any(&self) -> &dyn std::any::Any {
                self
            }

            #[inline]
            fn any_mut(&mut self) -> &mut dyn std::any::Any {
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
                Box::new(self.clone())
            }

            #[inline]
            fn apply(&mut self, value: &dyn #bevy_reflect_path::Reflect) {
                let value = value.any();
                if let Some(value) = value.downcast_ref::<Self>() {
                    *self = value.clone();
                } else {
                    panic!("Value is not {}.", std::any::type_name::<Self>());
                }
            }

            #[inline]
            fn set(&mut self, value: Box<dyn #bevy_reflect_path::Reflect>) -> Result<(), Box<dyn #bevy_reflect_path::Reflect>> {
                *self = value.take()?;
                Ok(())
            }

            fn reflect_ref(&self) -> #bevy_reflect_path::ReflectRef {
                #bevy_reflect_path::ReflectRef::Value(self)
            }

            fn reflect_mut(&mut self) -> #bevy_reflect_path::ReflectMut {
                #bevy_reflect_path::ReflectMut::Value(self)
            }

            fn reflect_hash(&self) -> Option<u64> {
                #hash_fn
            }

            fn reflect_partial_eq(&self, value: &dyn #bevy_reflect_path::Reflect) -> Option<bool> {
                #partial_eq_fn
            }

            fn serializable(&self) -> Option<#bevy_reflect_path::serde::Serializable> {
                #serialize_fn
            }
        }
    })
}
