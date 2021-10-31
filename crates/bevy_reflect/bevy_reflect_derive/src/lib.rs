extern crate proc_macro;

mod reflect_trait;
mod type_uuid;

use bevy_macro_utils::BevyManifest;
use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::quote;
use syn::{
    parenthesized,
    parse::{Parse, ParseStream},
    parse_macro_input,
    punctuated::Punctuated,
    token::{Comma, Paren, Where},
    Data, DataStruct, DeriveInput, Field, Fields, Generics, Ident, Index, Member, Meta, NestedMeta,
    Path,
};

#[derive(Default)]
struct PropAttributeArgs {
    pub ignore: Option<bool>,
}

#[derive(Clone)]
enum TraitImpl {
    NotImplemented,
    Implemented,
    Custom(Ident),
}

impl Default for TraitImpl {
    fn default() -> Self {
        Self::NotImplemented
    }
}

enum DeriveType {
    Struct,
    TupleStruct,
    UnitStruct,
    Value,
}

static REFLECT_ATTRIBUTE_NAME: &str = "reflect";
static REFLECT_VALUE_ATTRIBUTE_NAME: &str = "reflect_value";

#[proc_macro_derive(Reflect, attributes(reflect, reflect_value, module))]
pub fn derive_reflect(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    let unit_struct_punctuated = Punctuated::new();
    let (fields, mut derive_type) = match &ast.data {
        Data::Struct(DataStruct {
            fields: Fields::Named(fields),
            ..
        }) => (&fields.named, DeriveType::Struct),
        Data::Struct(DataStruct {
            fields: Fields::Unnamed(fields),
            ..
        }) => (&fields.unnamed, DeriveType::TupleStruct),
        Data::Struct(DataStruct {
            fields: Fields::Unit,
            ..
        }) => (&unit_struct_punctuated, DeriveType::UnitStruct),
        _ => (&unit_struct_punctuated, DeriveType::Value),
    };

    let fields_and_args = fields
        .iter()
        .enumerate()
        .map(|(i, f)| {
            (
                f,
                f.attrs
                    .iter()
                    .find(|a| *a.path.get_ident().as_ref().unwrap() == REFLECT_ATTRIBUTE_NAME)
                    .map(|a| {
                        syn::custom_keyword!(ignore);
                        let mut attribute_args = PropAttributeArgs { ignore: None };
                        a.parse_args_with(|input: ParseStream| {
                            if input.parse::<Option<ignore>>()?.is_some() {
                                attribute_args.ignore = Some(true);
                                return Ok(());
                            }
                            Ok(())
                        })
                        .expect("Invalid 'property' attribute format.");

                        attribute_args
                    }),
                i,
            )
        })
        .collect::<Vec<(&Field, Option<PropAttributeArgs>, usize)>>();
    let active_fields = fields_and_args
        .iter()
        .filter(|(_field, attrs, _i)| {
            attrs.is_none()
                || match attrs.as_ref().unwrap().ignore {
                    Some(ignore) => !ignore,
                    None => true,
                }
        })
        .map(|(f, _attr, i)| (*f, *i))
        .collect::<Vec<(&Field, usize)>>();

    let bevy_reflect_path = BevyManifest::default().get_path("bevy_reflect");
    let type_name = &ast.ident;

    let mut reflect_attrs = ReflectAttrs::default();
    for attribute in ast.attrs.iter().filter_map(|attr| attr.parse_meta().ok()) {
        let meta_list = if let Meta::List(meta_list) = attribute {
            meta_list
        } else {
            continue;
        };

        if let Some(ident) = meta_list.path.get_ident() {
            if ident == REFLECT_ATTRIBUTE_NAME {
                reflect_attrs = ReflectAttrs::from_nested_metas(&meta_list.nested);
            } else if ident == REFLECT_VALUE_ATTRIBUTE_NAME {
                derive_type = DeriveType::Value;
                reflect_attrs = ReflectAttrs::from_nested_metas(&meta_list.nested);
            }
        }
    }

    let registration_data = &reflect_attrs.data;
    let get_type_registration_impl = impl_get_type_registration(
        type_name,
        &bevy_reflect_path,
        registration_data,
        &ast.generics,
    );

    match derive_type {
        DeriveType::Struct | DeriveType::UnitStruct => impl_struct(
            type_name,
            &ast.generics,
            get_type_registration_impl,
            &bevy_reflect_path,
            &reflect_attrs,
            &active_fields,
        ),
        DeriveType::TupleStruct => impl_tuple_struct(
            type_name,
            &ast.generics,
            get_type_registration_impl,
            &bevy_reflect_path,
            &reflect_attrs,
            &active_fields,
        ),
        DeriveType::Value => impl_value(
            type_name,
            &ast.generics,
            get_type_registration_impl,
            &bevy_reflect_path,
            &reflect_attrs,
        ),
    }
}

fn impl_struct(
    struct_name: &Ident,
    generics: &Generics,
    get_type_registration_impl: proc_macro2::TokenStream,
    bevy_reflect_path: &Path,
    reflect_attrs: &ReflectAttrs,
    active_fields: &[(&Field, usize)],
) -> TokenStream {
    let field_names = active_fields
        .iter()
        .map(|(field, index)| {
            field
                .ident
                .as_ref()
                .map(|i| i.to_string())
                .unwrap_or_else(|| index.to_string())
        })
        .collect::<Vec<String>>();
    let field_idents = active_fields
        .iter()
        .map(|(field, index)| {
            field
                .ident
                .as_ref()
                .map(|ident| Member::Named(ident.clone()))
                .unwrap_or_else(|| Member::Unnamed(Index::from(*index)))
        })
        .collect::<Vec<_>>();
    let field_count = active_fields.len();
    let field_indices = (0..field_count).collect::<Vec<usize>>();

    let hash_fn = reflect_attrs.get_hash_impl(bevy_reflect_path);
    let serialize_fn = reflect_attrs.get_serialize_impl(bevy_reflect_path);
    let partial_eq_fn = match reflect_attrs.reflect_partial_eq {
        TraitImpl::NotImplemented => quote! {
            use #bevy_reflect_path::Struct;
            #bevy_reflect_path::struct_partial_eq(self, value)
        },
        TraitImpl::Implemented | TraitImpl::Custom(_) => reflect_attrs.get_partial_eq_impl(),
    };

    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

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
            fn clone_value(&self) -> Box<dyn #bevy_reflect_path::Reflect> {
                use #bevy_reflect_path::Struct;
                Box::new(self.clone_dynamic())
            }
            #[inline]
            fn set(&mut self, value: Box<dyn #bevy_reflect_path::Reflect>) -> Result<(), Box<dyn #bevy_reflect_path::Reflect>> {
                *self = value.take()?;
                Ok(())
            }

            #[inline]
            fn apply(&mut self, value: &dyn #bevy_reflect_path::Reflect) {
                use #bevy_reflect_path::Struct;
                if let #bevy_reflect_path::ReflectRef::Struct(struct_value) = value.reflect_ref() {
                    for (i, value) in struct_value.iter_fields().enumerate() {
                        let name = struct_value.name_at(i).unwrap();
                        self.field_mut(name).map(|v| v.apply(value));
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

fn impl_tuple_struct(
    struct_name: &Ident,
    generics: &Generics,
    get_type_registration_impl: proc_macro2::TokenStream,
    bevy_reflect_path: &Path,
    reflect_attrs: &ReflectAttrs,
    active_fields: &[(&Field, usize)],
) -> TokenStream {
    let field_idents = active_fields
        .iter()
        .map(|(_field, index)| Member::Unnamed(Index::from(*index)))
        .collect::<Vec<_>>();
    let field_count = active_fields.len();
    let field_indices = (0..field_count).collect::<Vec<usize>>();

    let hash_fn = reflect_attrs.get_hash_impl(bevy_reflect_path);
    let serialize_fn = reflect_attrs.get_serialize_impl(bevy_reflect_path);
    let partial_eq_fn = match reflect_attrs.reflect_partial_eq {
        TraitImpl::NotImplemented => quote! {
            use #bevy_reflect_path::TupleStruct;
            #bevy_reflect_path::tuple_struct_partial_eq(self, value)
        },
        TraitImpl::Implemented | TraitImpl::Custom(_) => reflect_attrs.get_partial_eq_impl(),
    };

    let (impl_generics, ty_generics, _where_clause) = generics.split_for_impl();
    TokenStream::from(quote! {
        #get_type_registration_impl

        impl #impl_generics #bevy_reflect_path::TupleStruct for #struct_name #ty_generics {
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
        unsafe impl #impl_generics #bevy_reflect_path::Reflect for #struct_name #ty_generics {
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
                use #bevy_reflect_path::TupleStruct;
                Box::new(self.clone_dynamic())
            }
            #[inline]
            fn set(&mut self, value: Box<dyn #bevy_reflect_path::Reflect>) -> Result<(), Box<dyn #bevy_reflect_path::Reflect>> {
                *self = value.take()?;
                Ok(())
            }

            #[inline]
            fn apply(&mut self, value: &dyn #bevy_reflect_path::Reflect) {
                use #bevy_reflect_path::TupleStruct;
                if let #bevy_reflect_path::ReflectRef::TupleStruct(struct_value) = value.reflect_ref() {
                    for (i, value) in struct_value.iter_fields().enumerate() {
                        self.field_mut(i).map(|v| v.apply(value));
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

fn impl_value(
    type_name: &Ident,
    generics: &Generics,
    get_type_registration_impl: proc_macro2::TokenStream,
    bevy_reflect_path: &Path,
    reflect_attrs: &ReflectAttrs,
) -> TokenStream {
    let hash_fn = reflect_attrs.get_hash_impl(bevy_reflect_path);
    let partial_eq_fn = reflect_attrs.get_partial_eq_impl();
    let serialize_fn = reflect_attrs.get_serialize_impl(bevy_reflect_path);

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
struct ReflectDef {
    type_name: Ident,
    generics: Generics,
    attrs: Option<ReflectAttrs>,
}

impl Parse for ReflectDef {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let type_ident = input.parse::<Ident>()?;
        let generics = input.parse::<Generics>()?;
        let mut lookahead = input.lookahead1();
        let mut where_clause = None;
        if lookahead.peek(Where) {
            where_clause = Some(input.parse()?);
            lookahead = input.lookahead1();
        }

        let mut attrs = None;
        if lookahead.peek(Paren) {
            let content;
            parenthesized!(content in input);
            attrs = Some(content.parse::<ReflectAttrs>()?);
        }

        Ok(ReflectDef {
            type_name: type_ident,
            generics: Generics {
                where_clause,
                ..generics
            },
            attrs,
        })
    }
}

#[proc_macro]
pub fn impl_reflect_value(input: TokenStream) -> TokenStream {
    let reflect_value_def = parse_macro_input!(input as ReflectDef);

    let bevy_reflect_path = BevyManifest::default().get_path("bevy_reflect");
    let ty = &reflect_value_def.type_name;
    let reflect_attrs = reflect_value_def.attrs.unwrap_or_default();
    let registration_data = &reflect_attrs.data;
    let get_type_registration_impl = impl_get_type_registration(
        ty,
        &bevy_reflect_path,
        registration_data,
        &reflect_value_def.generics,
    );
    impl_value(
        ty,
        &reflect_value_def.generics,
        get_type_registration_impl,
        &bevy_reflect_path,
        &reflect_attrs,
    )
}

#[derive(Default)]
struct ReflectAttrs {
    reflect_hash: TraitImpl,
    reflect_partial_eq: TraitImpl,
    serialize: TraitImpl,
    data: Vec<Ident>,
}

impl ReflectAttrs {
    fn from_nested_metas(nested_metas: &Punctuated<NestedMeta, Comma>) -> Self {
        let mut attrs = ReflectAttrs::default();
        for nested_meta in nested_metas.iter() {
            match nested_meta {
                NestedMeta::Lit(_) => {}
                NestedMeta::Meta(meta) => match meta {
                    Meta::Path(path) => {
                        if let Some(segment) = path.segments.iter().next() {
                            let ident = segment.ident.to_string();
                            match ident.as_str() {
                                "PartialEq" => attrs.reflect_partial_eq = TraitImpl::Implemented,
                                "Hash" => attrs.reflect_hash = TraitImpl::Implemented,
                                "Serialize" => attrs.serialize = TraitImpl::Implemented,
                                _ => attrs.data.push(Ident::new(
                                    &format!("Reflect{}", segment.ident),
                                    Span::call_site(),
                                )),
                            }
                        }
                    }
                    Meta::List(list) => {
                        let ident = if let Some(segment) = list.path.segments.iter().next() {
                            segment.ident.to_string()
                        } else {
                            continue;
                        };

                        if let Some(list_nested) = list.nested.iter().next() {
                            match list_nested {
                                NestedMeta::Meta(list_nested_meta) => match list_nested_meta {
                                    Meta::Path(path) => {
                                        if let Some(segment) = path.segments.iter().next() {
                                            match ident.as_str() {
                                                "PartialEq" => {
                                                    attrs.reflect_partial_eq =
                                                        TraitImpl::Custom(segment.ident.clone())
                                                }
                                                "Hash" => {
                                                    attrs.reflect_hash =
                                                        TraitImpl::Custom(segment.ident.clone())
                                                }
                                                "Serialize" => {
                                                    attrs.serialize =
                                                        TraitImpl::Custom(segment.ident.clone())
                                                }
                                                _ => {}
                                            }
                                        }
                                    }
                                    Meta::List(_) => {}
                                    Meta::NameValue(_) => {}
                                },
                                NestedMeta::Lit(_) => {}
                            }
                        }
                    }
                    Meta::NameValue(_) => {}
                },
            }
        }

        attrs
    }

    fn get_hash_impl(&self, path: &Path) -> proc_macro2::TokenStream {
        match &self.reflect_hash {
            TraitImpl::Implemented => quote! {
                use std::hash::{Hash, Hasher};
                let mut hasher = #path::ReflectHasher::default();
                Hash::hash(&std::any::Any::type_id(self), &mut hasher);
                Hash::hash(self, &mut hasher);
                Some(hasher.finish())
            },
            TraitImpl::Custom(impl_fn) => quote! {
                Some(#impl_fn(self))
            },
            TraitImpl::NotImplemented => quote! {
                None
            },
        }
    }

    fn get_partial_eq_impl(&self) -> proc_macro2::TokenStream {
        match &self.reflect_partial_eq {
            TraitImpl::Implemented => quote! {
                let value = value.any();
                if let Some(value) = value.downcast_ref::<Self>() {
                    Some(std::cmp::PartialEq::eq(self, value))
                } else {
                    Some(false)
                }
            },
            TraitImpl::Custom(impl_fn) => quote! {
                Some(#impl_fn(self, value))
            },
            TraitImpl::NotImplemented => quote! {
                None
            },
        }
    }

    fn get_serialize_impl(&self, path: &Path) -> proc_macro2::TokenStream {
        match &self.serialize {
            TraitImpl::Implemented => quote! {
                Some(#path::serde::Serializable::Borrowed(self))
            },
            TraitImpl::Custom(impl_fn) => quote! {
                Some(#impl_fn(self))
            },
            TraitImpl::NotImplemented => quote! {
                None
            },
        }
    }
}

impl Parse for ReflectAttrs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let result = Punctuated::<NestedMeta, Comma>::parse_terminated(input)?;
        Ok(ReflectAttrs::from_nested_metas(&result))
    }
}

fn impl_get_type_registration(
    type_name: &Ident,
    bevy_reflect_path: &Path,
    registration_data: &[Ident],
    generics: &Generics,
) -> proc_macro2::TokenStream {
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
    quote! {
        #[allow(unused_mut)]
        impl #impl_generics #bevy_reflect_path::GetTypeRegistration for #type_name #ty_generics #where_clause {
            fn get_type_registration() -> #bevy_reflect_path::TypeRegistration {
                let mut registration = #bevy_reflect_path::TypeRegistration::of::<#type_name #ty_generics>();
                #(registration.insert::<#registration_data>(#bevy_reflect_path::FromType::<#type_name #ty_generics>::from_type());)*
                registration
            }
        }
    }
}

// From https://github.com/randomPoison/type-uuid
#[proc_macro_derive(TypeUuid, attributes(uuid))]
pub fn type_uuid_derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    type_uuid::type_uuid_derive(input)
}

#[proc_macro]
pub fn external_type_uuid(tokens: proc_macro::TokenStream) -> proc_macro::TokenStream {
    type_uuid::external_type_uuid(tokens)
}

#[proc_macro_attribute]
pub fn reflect_trait(args: TokenStream, input: TokenStream) -> TokenStream {
    reflect_trait::reflect_trait(args, input)
}
