use crate::container_attributes::ReflectTraits;
use proc_macro::TokenStream;
use proc_macro2::{Ident, Span};
use quote::quote;
use syn::{Fields, Generics, Path, Variant};

fn tuple_field_name(i: usize) -> String {
    format!("t{}", i)
}

fn tuple_field_ident(i: usize) -> Ident {
    Ident::new(tuple_field_name(i).as_str(), Span::call_site())
}

pub(crate) fn impl_enum(
    enum_name: &Ident,
    generics: &Generics,
    get_type_registration_impl: proc_macro2::TokenStream,
    bevy_reflect_path: &Path,
    traits: &ReflectTraits,
    active_variants: &[(&Variant, usize)],
) -> TokenStream {
    let mut variant_indices = Vec::new();
    let mut struct_wrappers = Vec::new();
    let mut tuple_wrappers = Vec::new();
    let mut variant_names = Vec::new();
    let mut variant_idents = Vec::new();
    let mut reflect_variants = Vec::new();
    let mut reflect_variants_mut = Vec::new();
    let mut variant_with_fields_idents = Vec::new();
    let mut variant_without_fields_idents = Vec::new();
    for (variant, variant_index) in active_variants.iter() {
        let variant_ident = {
            let ident = &variant.ident;
            quote!(#enum_name::#ident)
        };
        let variant_name = variant_ident.to_string();
        let variant_without_fields_ident = {
            match &variant.fields {
                Fields::Named(_struct_fields) => {
                    quote!(#variant_ident {..})
                }
                Fields::Unnamed(tuple) => {
                    let tuple_fields = &tuple.unnamed;
                    if tuple_fields.len() == 1 {
                        quote!(#variant_ident (_))
                    } else {
                        quote!(#variant_ident (..))
                    }
                }
                Fields::Unit => {
                    quote!(#variant_ident)
                }
            }
        };
        let variant_with_fields_ident = {
            match &variant.fields {
                Fields::Named(struct_fields) => {
                    let field_idents = struct_fields
                        .named
                        .iter()
                        .map(|field| field.ident.as_ref().unwrap())
                        .collect::<Vec<_>>();
                    quote!(#variant_ident {#(#field_idents,)*})
                }
                Fields::Unnamed(tuple_fields) => {
                    let field_idents = (0..tuple_fields.unnamed.len())
                        .map(|i| tuple_field_ident(i))
                        .collect::<Vec<_>>();
                    if tuple_fields.unnamed.len() == 1 {
                        quote!(#variant_ident (new_type))
                    } else {
                        quote!(#variant_ident (#(#field_idents,)*))
                    }
                }
                Fields::Unit => {
                    quote!(#variant_ident)
                }
            }
        };
        let wrapper_ident = if let Fields::Named(_) | Fields::Unnamed(_) = &variant.fields {
            Ident::new(
                format!("{}{}Wrapper", enum_name, variant.ident).as_str(),
                Span::call_site(),
            )
        } else {
            Ident::new("unused", Span::call_site())
        };
        let wrapper_name = match &variant.fields {
            Fields::Named(struct_fields) => quote!(#struct_fields).to_string(),
            Fields::Unnamed(tuple_fields) => quote!(#tuple_fields).to_string(),
            Fields::Unit => "unused".to_string(),
        };
        let reflect_variant = {
            match &variant.fields {
                Fields::Named(_struct_fields) => {
                    quote!({
                        let wrapper_ref = unsafe { std::mem::transmute::< &Self, &#wrapper_ident >(self) };
                        #bevy_reflect_path::EnumVariant::Struct(wrapper_ref as &dyn #bevy_reflect_path::Struct)
                    })
                }
                Fields::Unnamed(tuple_fields) => {
                    if tuple_fields.unnamed.len() == 1 {
                        quote!(#bevy_reflect_path::EnumVariant::NewType(new_type as &dyn #bevy_reflect_path::Reflect))
                    } else {
                        quote!({
                            let wrapper_ref = unsafe { std::mem::transmute::< &Self, &#wrapper_ident >(self) };
                            #bevy_reflect_path::EnumVariant::Tuple(wrapper_ref as &dyn #bevy_reflect_path::Tuple)
                        })
                    }
                }
                Fields::Unit => {
                    quote!(#bevy_reflect_path::EnumVariant::Unit)
                }
            }
        };
        let reflect_variant_mut = {
            match &variant.fields {
                Fields::Named(_struct_fields) => {
                    quote!({
                        let wrapper_ref = unsafe { std::mem::transmute::< &mut Self, &mut #wrapper_ident >(self) };
                        #bevy_reflect_path::EnumVariantMut::Struct(wrapper_ref as &mut dyn #bevy_reflect_path::Struct)
                    })
                }
                Fields::Unnamed(tuple) => {
                    let tuple_fields = &tuple.unnamed;
                    if tuple_fields.len() == 1 {
                        quote!(#bevy_reflect_path::EnumVariantMut::NewType(new_type as &mut dyn #bevy_reflect_path::Reflect))
                    } else {
                        quote!({
                            let wrapper_ref = unsafe { std::mem::transmute::< &mut Self, &mut #wrapper_ident >(self) };
                            #bevy_reflect_path::EnumVariantMut::Tuple(wrapper_ref as &mut dyn #bevy_reflect_path::Tuple)
                        })
                    }
                }
                Fields::Unit => {
                    quote!(#bevy_reflect_path::EnumVariantMut::Unit)
                }
            }
        };
        match &variant.fields {
            Fields::Named(struct_fields) => {
                struct_wrappers.push((
                    wrapper_ident,
                    wrapper_name,
                    variant_index,
                    variant_with_fields_ident.clone(),
                    struct_fields.clone(),
                ));
            }
            Fields::Unnamed(tuple_fields) => {
                if tuple_fields.unnamed.len() > 1 {
                    tuple_wrappers.push((
                        wrapper_ident,
                        wrapper_name,
                        variant_index,
                        variant_with_fields_ident.clone(),
                        tuple_fields.clone(),
                    ));
                }
            }
            Fields::Unit => {}
        }
        variant_indices.push(variant_index);
        variant_names.push(variant_name);
        variant_idents.push(variant_ident);
        reflect_variants.push(reflect_variant);
        reflect_variants_mut.push(reflect_variant_mut);
        variant_with_fields_idents.push(variant_with_fields_ident);
        variant_without_fields_idents.push(variant_without_fields_ident);
    }
    let hash_fn = traits.get_hash_impl(bevy_reflect_path);
    let partial_eq_fn = traits.get_partial_eq_impl(bevy_reflect_path).unwrap_or_else(|| {
        quote! {
            fn reflect_partial_eq(&self, value: &dyn #bevy_reflect_path::Reflect) -> Option<bool> {
                #bevy_reflect_path::enum_partial_eq(self, value)
            }
        }
    });

    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    let mut token_stream = TokenStream::from(quote! {
        #get_type_registration_impl

        impl #impl_generics #bevy_reflect_path::Enum for #enum_name #ty_generics #where_clause {
            fn variant(&self) -> #bevy_reflect_path::EnumVariant<'_> {
                match self {
                    #(#variant_with_fields_idents => #reflect_variants,)*
                }
            }

            fn variant_mut(&mut self) -> #bevy_reflect_path::EnumVariantMut<'_> {
                match self {
                    #(#variant_with_fields_idents => #reflect_variants_mut,)*
                }
            }

            fn variant_info(&self) -> #bevy_reflect_path::VariantInfo<'_> {
                let index = match self {
                    #(#variant_without_fields_idents => #variant_indices,)*
                };
                #bevy_reflect_path::VariantInfo {
                    index,
                    name: self.get_index_name(index).unwrap(),
                }
            }

            fn get_index_name(&self, index: usize) -> Option<&'_ str> {
                match index {
                    #(#variant_indices => Some(#variant_names),)*
                    _ => None,
                }
            }

            fn get_index_from_name(&self, name: &str) -> Option<usize> {
                match name {
                    #(#variant_names => Some(#variant_indices),)*
                    _ => None,
                }
            }

            fn iter_variants_info(&self) -> #bevy_reflect_path::VariantInfoIter<'_> {
                #bevy_reflect_path::VariantInfoIter::new(self)
            }
        }

        impl #impl_generics #bevy_reflect_path::Reflect for #enum_name #ty_generics #where_clause {
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
            fn clone_value(&self) -> Box<dyn #bevy_reflect_path::Reflect> {
                use #bevy_reflect_path::Enum;
                Box::new(self.clone()) // FIXME: should be clone_dynamic, so that Clone is not a required bound
            }
            #[inline]
            fn set(&mut self, value: Box<dyn #bevy_reflect_path::Reflect>) -> Result<(), Box<dyn #bevy_reflect_path::Reflect>> {
                *self = value.take()?;
                Ok(())
            }

            #[inline]
            fn apply(&mut self, value: &dyn #bevy_reflect_path::Reflect) {
                use #bevy_reflect_path::Enum;
                let value = value.any();
                if let Some(value) = value.downcast_ref::<Self>() {
                    *self = value.clone(); //FIXME: should apply the variant instead
                } else {
                    panic!("Attempted to apply non-enum type to enum type.");
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

            fn as_reflect(&self) -> &dyn #bevy_reflect_path::Reflect {
                self
            }

            fn as_reflect_mut(&mut self) -> &mut dyn #bevy_reflect_path::Reflect {
                self
            }
        }
    });
    for (wrapper_ident, wrapper_name, variant_index, variant_with_fields_ident, fields) in struct_wrappers
    {
        let mut field_names = Vec::new();
        let mut field_idents = Vec::new();
        let mut field_indices = Vec::new();
        for (i, field) in fields.named.iter().enumerate() {
            field_names.push(field.ident.as_ref().unwrap().to_string());
            field_idents.push(field.ident.clone());
            field_indices.push(i);
        }
        let fields_len = field_indices.len();
        let mut match_fields = quote!();
        for (i, _variant_ident) in variant_idents.iter().enumerate() {
            if i == *variant_index {
                match_fields.extend(quote!(
                    #variant_with_fields_ident => (#(#field_idents,)*),
                ));
            } else {
                match_fields.extend(quote!(
                    #variant_with_fields_ident => unreachable!(),
                ));
            }
        }
        let match_fields_mut = quote!(let (#(#field_idents,)*) = match &mut self.0 {
            #match_fields
        };);
        let match_fields = quote!(let (#(#field_idents,)*) = match &self.0 {
            #match_fields
        };);
        token_stream.extend(TokenStream::from(quote! {
            #[repr(transparent)]
            pub struct #wrapper_ident(enum_name);
            impl #bevy_reflect_path::Reflect for #wrapper_ident {
                fn type_name(&self) -> &str {
                    #wrapper_name
                }

                fn any(&self) -> &dyn std::any::Any {
                    self.0.any()
                }

                fn any_mut(&mut self) -> &mut dyn std::any::Any {
                    self.0.any_mut()
                }

                fn apply(&mut self, value: &dyn #bevy_reflect_path::Reflect) {
                    self.0.apply(value);
                }

                fn set(&mut self, value: Box<dyn #bevy_reflect_path::Reflect>) -> Result<(), Box<dyn #bevy_reflect_path::Reflect>> {
                    self.0.set(value)
                }

                fn reflect_ref(&self) -> #bevy_reflect_path::ReflectRef {
                    #bevy_reflect_path::ReflectRef::Struct(self)
                }

                fn reflect_mut(&mut self) -> #bevy_reflect_path::ReflectMut {
                    #bevy_reflect_path::ReflectMut::Struct(self)
                }

                fn clone_value(&self) -> Box<dyn #bevy_reflect_path::Reflect> {
                    self.0.clone_value()
                }

                fn reflect_hash(&self) -> Option<u64> {
                    self.0.reflect_hash()
                }

                fn reflect_partial_eq(&self, value: &dyn #bevy_reflect_path::Reflect) -> Option<bool> {
                    self.0.reflect_partial_eq(value)
                }

                fn serializable(&self) -> Option<#bevy_reflect_path::serde::Serializable> {
                    self.0.serializable()
                }

                fn as_reflect(&self) -> &dyn #bevy_reflect_path::Reflect {
                    self
                }

                fn as_reflect_mut(&mut self) -> &mut dyn #bevy_reflect_path::Reflect {
                    self
                }
            }
            impl #bevy_reflect_path::Struct for #wrapper_ident {
                fn field(&self, name: &str) -> Option<&dyn #bevy_reflect_path::Reflect> {
                    #match_fields
                    match name {
                        #(#field_names => Some(#field_idents),)*
                        _ => None,
                    }
                }

                fn field_mut(&mut self, name: &str) -> Option<&mut dyn #bevy_reflect_path::Reflect> {
                    #match_fields_mut
                    match name {
                        #(#field_names => Some(#field_idents),)*
                        _ => None,
                    }
                }

                fn field_at(&self, index: usize) -> Option<&dyn #bevy_reflect_path::Reflect> {
                    #match_fields
                    match index {
                        #(#field_indices => Some(#field_idents),)*
                        _ => None,
                    }
                }

                fn field_at_mut(&mut self, index: usize) -> Option<&mut dyn #bevy_reflect_path::Reflect> {
                    #match_fields_mut
                    match index {
                        #(#field_indices => Some(#field_idents),)*
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
                    #fields_len
                }

                fn iter_fields(&self) -> #bevy_reflect_path::FieldIter {
                    #bevy_reflect_path::FieldIter::new(self)
                }

                fn clone_dynamic(&self) -> #bevy_reflect_path::DynamicStruct {
                    #match_fields
                    let mut dynamic = #bevy_reflect_path::DynamicStruct::default();
                    dynamic.set_name(self.type_name().to_string());
                    #(dynamic.insert_boxed(#field_names, #field_idents.clone_value());)*
                    dynamic
                }
            }
        }));
    }
    for (wrapper_ident, wrapper_name, variant_index, variant_with_fields_ident, fields) in tuple_wrappers
    {
        let mut field_names = Vec::new();
        let mut field_idents = Vec::new();
        let mut field_indices = Vec::new();
        for (index, _field) in fields.unnamed.iter().enumerate() {
            field_names.push(tuple_field_name(index));
            field_idents.push(tuple_field_ident(index));
            field_indices.push(index);
        }
        let fields_len = field_indices.len();
        let mut match_fields = quote!();
        for (i, _variant_ident) in variant_idents.iter().enumerate() {
            if i == *variant_index {
                match_fields.extend(quote!(
                    #variant_with_fields_ident => (#(#field_idents,)*),
                ));
            } else {
                match_fields.extend(quote!(
                    #variant_with_fields_ident => unreachable!(),
                ));
            }
        }
        let match_fields_mut = quote!(let (#(#field_idents,)*) = match &mut self.0 {
            #match_fields
        };);
        let match_fields = quote!(let (#(#field_idents,)*) = match &self.0 {
            #match_fields
        };);
        token_stream.extend(TokenStream::from(quote! {
            #[repr(transparent)]
            pub struct #wrapper_ident(enum_name);
            impl #bevy_reflect_path::Reflect for #wrapper_ident {
                fn type_name(&self) -> &str {
                    #wrapper_name
                }

                fn into_any(self: Box<Self>) -> Box<dyn std::any::Any> {
                    self
                }

                fn as_any(&self) -> &dyn std::any::Any {
                    self
                }

                fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
                    self
                }

                fn apply(&mut self, value: &dyn #bevy_reflect_path::Reflect) {
                    self.0.apply(value);
                }

                fn set(&mut self, value: Box<dyn #bevy_reflect_path::Reflect>) -> Result<(), Box<dyn #bevy_reflect_path::Reflect>> {
                    self.0.set(value)
                }

                fn reflect_ref(&self) -> #bevy_reflect_path::ReflectRef {
                    #bevy_reflect_path::ReflectRef::Tuple(self)
                }

                fn reflect_mut(&mut self) -> #bevy_reflect_path::ReflectMut {
                    #bevy_reflect_path::ReflectMut::Tuple(self)
                }

                fn clone_value(&self) -> Box<dyn #bevy_reflect_path::Reflect> {
                    self.0.clone_value()
                }

                fn reflect_hash(&self) -> Option<u64> {
                    self.0.reflect_hash()
                }

                fn reflect_partial_eq(&self, value: &dyn #bevy_reflect_path::Reflect) -> Option<bool> {
                    self.0.reflect_partial_eq(value)
                }

                fn serializable(&self) -> Option<#bevy_reflect_path::serde::Serializable> {
                    self.0.serializable()
                }

                fn as_reflect(&self) -> &dyn #bevy_reflect_path::Reflect {
                    self
                }

                fn as_reflect_mut(&mut self) -> &mut dyn #bevy_reflect_path::Reflect {
                    self
                }
            }
            impl #bevy_reflect_path::Tuple for #wrapper_ident {
                fn field(&self, index: usize) -> Option<&dyn #bevy_reflect_path::Reflect> {
                    #match_fields
                    match index {
                        #(#field_indices => Some(#field_idents),)*
                        _ => None,
                    }
                }

                fn field_mut(&mut self, index: usize) -> Option<&mut dyn #bevy_reflect_path::Reflect> {
                    #match_fields_mut
                    match index {
                        #(#field_indices => Some(#field_idents),)*
                        _ => None,
                    }
                }

                fn field_len(&self) -> usize {
                    #fields_len
                }

                fn iter_fields(&self) -> #bevy_reflect_path::TupleFieldIter {
                    #bevy_reflect_path::TupleFieldIter::new(self)
                }

                fn clone_dynamic(&self) -> #bevy_reflect_path::DynamicTuple {
                    #match_fields
                    let mut dynamic = #bevy_reflect_path::DynamicTuple::default();
                    #(dynamic.insert_boxed(#field_idents.clone_value());)*
                    dynamic
                }
            }
        }));
    }
    token_stream
}
