extern crate proc_macro;

mod modules;

use modules::{get_modules, get_path};
use proc_macro::TokenStream;
use proc_macro_crate::crate_name;
use quote::quote;
use syn::{
    parse::{Parse, ParseStream},
    parse_macro_input,
    punctuated::Punctuated,
    token::{Comma, Where},
    Data, DataStruct, DeriveInput, Field, Fields, Generics, Ident, Index, Member,
};

#[derive(Default)]
struct PropAttributeArgs {
    pub ignore: Option<bool>,
}

static PROP_ATTRIBUTE_NAME: &str = "property";

#[proc_macro_derive(Properties, attributes(property, module))]
pub fn derive_properties(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    let unit_struct_punctuated = Punctuated::new();
    let fields = match &ast.data {
        Data::Struct(DataStruct {
            fields: Fields::Named(fields),
            ..
        }) => &fields.named,
        Data::Struct(DataStruct {
            fields: Fields::Unnamed(fields),
            ..
        }) => &fields.unnamed,
        Data::Struct(DataStruct {
            fields: Fields::Unit,
            ..
        }) => &unit_struct_punctuated,
        _ => panic!("expected a struct with named fields"),
    };
    let fields_and_args = fields
        .iter()
        .enumerate()
        .map(|(i, f)| {
            (
                f,
                f.attrs
                    .iter()
                    .find(|a| *a.path.get_ident().as_ref().unwrap() == PROP_ATTRIBUTE_NAME)
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
                        .expect("invalid 'property' attribute format");

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

    let modules = get_modules();
    let bevy_property_path = get_path(&modules.bevy_property);

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

    let generics = ast.generics;
    let (impl_generics, ty_generics, _where_clause) = generics.split_for_impl();

    let struct_name = &ast.ident;

    TokenStream::from(quote! {
        impl #impl_generics #bevy_property_path::Properties for #struct_name#ty_generics {
            fn prop(&self, name: &str) -> Option<&dyn #bevy_property_path::Property> {
                match name {
                    #(#field_names => Some(&self.#field_idents),)*
                    _ => None,
                }
            }

            fn prop_mut(&mut self, name: &str) -> Option<&mut dyn #bevy_property_path::Property> {
                match name {
                    #(#field_names => Some(&mut self.#field_idents),)*
                    _ => None,
                }
            }

            fn prop_with_index(&self, index: usize) -> Option<&dyn #bevy_property_path::Property> {
                match index {
                    #(#field_indices => Some(&self.#field_idents),)*
                    _ => None,
                }
            }

            fn prop_with_index_mut(&mut self, index: usize) -> Option<&mut dyn #bevy_property_path::Property> {
                match index {
                    #(#field_indices => Some(&mut self.#field_idents),)*
                    _ => None,
                }
            }

            fn prop_name(&self, index: usize) -> Option<&str> {
                match index {
                    #(#field_indices => Some(#field_names),)*
                    _ => None,
                }
            }

            fn prop_len(&self) -> usize {
                #field_count
            }

            fn iter_props(&self) -> #bevy_property_path::PropertyIter {
                #bevy_property_path::PropertyIter::new(self)
            }
        }

        impl #impl_generics #bevy_property_path::DeserializeProperty for #struct_name#ty_generics {
            fn deserialize(
                deserializer: &mut dyn #bevy_property_path::erased_serde::Deserializer,
                property_type_registry: &#bevy_property_path::PropertyTypeRegistry) ->
                    Result<Box<dyn #bevy_property_path::Property>, #bevy_property_path::erased_serde::Error> {
                    use #bevy_property_path::serde::de::DeserializeSeed;
                    let dynamic_properties_deserializer = #bevy_property_path::property_serde::DynamicPropertiesDeserializer::new(property_type_registry);
                    let dynamic_properties: #bevy_property_path::DynamicProperties = dynamic_properties_deserializer.deserialize(deserializer)?;
                    Ok(Box::new(dynamic_properties))
            }
        }

        impl #impl_generics #bevy_property_path::Property for #struct_name#ty_generics {
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
            fn clone_prop(&self) -> Box<dyn #bevy_property_path::Property> {
                Box::new(self.to_dynamic())
            }
            #[inline]
            fn set(&mut self, value: &dyn #bevy_property_path::Property) {
                // TODO: type check
                self.apply(value);
            }

            #[inline]
            fn apply(&mut self, value: &dyn #bevy_property_path::Property) {
                if let Some(properties) = value.as_properties() {
                    if properties.property_type() != self.property_type() {
                        panic!(
                            "Properties type mismatch. This type is {:?} but the applied type is {:?}",
                            self.property_type(),
                            properties.property_type()
                        );
                    }
                    for (i, prop) in properties.iter_props().enumerate() {
                        let name = properties.prop_name(i).unwrap();
                        self.prop_mut(name).map(|p| p.apply(prop));
                    }
                } else {
                    panic!("attempted to apply non-Properties type to Properties type");
                }
            }

            #[inline]
            fn as_properties(&self) -> Option<&dyn #bevy_property_path::Properties> {
                Some(self)
            }

            fn serializable<'a>(&'a self, registry: &'a #bevy_property_path::PropertyTypeRegistry) -> #bevy_property_path::property_serde::Serializable<'a> {
                #bevy_property_path::property_serde::Serializable::Owned(Box::new(#bevy_property_path::property_serde::MapSerializer::new(self, registry)))
            }

            fn property_type(&self) -> #bevy_property_path::PropertyType {
                #bevy_property_path::PropertyType::Map
            }
        }
    })
}

#[proc_macro_derive(Property)]
pub fn derive_property(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    let modules = get_modules();
    let bevy_property_path = get_path(&modules.bevy_property);

    let generics = ast.generics;
    let (impl_generics, ty_generics, _where_clause) = generics.split_for_impl();

    let struct_name = &ast.ident;

    TokenStream::from(quote! {
        impl #impl_generics #bevy_property_path::Property for #struct_name#ty_generics  {
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
            fn clone_prop(&self) -> Box<dyn #bevy_property_path::Property> {
                Box::new(self.clone())
            }

            #[inline]
            fn apply(&mut self, value: &dyn #bevy_property_path::Property) {
                self.set(value);
            }

            #[inline]
            fn set(&mut self, value: &dyn #bevy_property_path::Property) {
                let value = value.any();
                if let Some(prop) = value.downcast_ref::<Self>() {
                    *self = prop.clone();
                } else {
                    panic!("prop value is not {}", std::any::type_name::<Self>());
                }
            }

            #[inline]
            fn serializable<'a>(&'a self, registry: &'a #bevy_property_path::PropertyTypeRegistry) -> #bevy_property_path::property_serde::Serializable<'a> {
                #bevy_property_path::property_serde::Serializable::Owned(Box::new(#bevy_property_path::property_serde::PropertyValueSerializer::new(self, registry)))
            }

            fn property_type(&self) -> #bevy_property_path::PropertyType {
                #bevy_property_path::PropertyType::Value
            }
        }

        impl #impl_generics #bevy_property_path::DeserializeProperty for #struct_name#ty_generics  {
            fn deserialize(
                deserializer: &mut dyn #bevy_property_path::erased_serde::Deserializer,
                property_type_registry: &#bevy_property_path::PropertyTypeRegistry) ->
                    Result<Box<dyn #bevy_property_path::Property>, #bevy_property_path::erased_serde::Error> {
                    let property = <#struct_name#ty_generics as #bevy_property_path::serde::Deserialize>::deserialize(deserializer)?;
                    Ok(Box::new(property))
            }
       }
    })
}

struct PropertyDef {
    type_name: Ident,
    generics: Generics,
    serialize_fn: Option<Ident>,
    deserialize_fn: Option<Ident>,
}

impl Parse for PropertyDef {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let type_ident = input.parse::<Ident>()?;
        let generics = input.parse::<Generics>()?;
        let mut lookahead = input.lookahead1();
        let mut where_clause = None;
        if lookahead.peek(Where) {
            where_clause = Some(input.parse()?);
            lookahead = input.lookahead1();
        }

        let mut serialize_fn = None;
        if lookahead.peek(Comma) {
            input.parse::<Comma>()?;
            serialize_fn = Some(input.parse::<Ident>()?);
            lookahead = input.lookahead1();
        }

        let mut deserialize_fn = None;
        if lookahead.peek(Comma) {
            input.parse::<Comma>()?;
            deserialize_fn = Some(input.parse::<Ident>()?);
        }

        Ok(PropertyDef {
            type_name: type_ident,
            generics: Generics {
                where_clause,
                ..generics
            },
            serialize_fn,
            deserialize_fn,
        })
    }
}

#[proc_macro]
pub fn impl_property(input: TokenStream) -> TokenStream {
    let property_def = parse_macro_input!(input as PropertyDef);

    let bevy_property_path = get_path(if crate_name("bevy").is_ok() {
        "bevy::property"
    } else if crate_name("bevy_property").is_ok() {
        "bevy_property"
    } else {
        "crate"
    });

    let (impl_generics, ty_generics, where_clause) = property_def.generics.split_for_impl();
    let ty = &property_def.type_name;
    let serialize_fn = if let Some(serialize_fn) = property_def.serialize_fn {
        quote! { #serialize_fn(self) }
    } else {
        quote! {
            #bevy_property_path::property_serde::Serializable::Owned(Box::new(#bevy_property_path::property_serde::PropertyValueSerializer::new(self, registry)))
        }
    };
    let deserialize_fn = if let Some(deserialize_fn) = property_def.deserialize_fn {
        quote! { #deserialize_fn(deserializer, property_type_registry) }
    } else {
        quote! {
            let property = <#ty#ty_generics as #bevy_property_path::serde::Deserialize>::deserialize(deserializer)?;
            Ok(Box::new(property))
        }
    };

    TokenStream::from(quote! {
        impl #impl_generics #bevy_property_path::Property for #ty#ty_generics #where_clause  {
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
            fn clone_prop(&self) -> Box<dyn #bevy_property_path::Property> {
                Box::new(self.clone())
            }

            #[inline]
            fn apply(&mut self, value: &dyn #bevy_property_path::Property) {
                self.set(value);
            }

            #[inline]
            fn set(&mut self, value: &dyn #bevy_property_path::Property) {
                let value = value.any();
                if let Some(prop) = value.downcast_ref::<Self>() {
                    *self = prop.clone();
                } else {
                    panic!("prop value is not {}", std::any::type_name::<Self>());
                }
            }

            #[inline]
            fn serializable<'a>(&'a self, registry: &'a #bevy_property_path::PropertyTypeRegistry) -> #bevy_property_path::property_serde::Serializable<'a> {
                #serialize_fn
            }

            fn property_type(&self) -> #bevy_property_path::PropertyType {
                #bevy_property_path::PropertyType::Value
            }
        }

        impl #impl_generics #bevy_property_path::DeserializeProperty for #ty#ty_generics #where_clause  {
            fn deserialize(
                deserializer: &mut dyn #bevy_property_path::erased_serde::Deserializer,
                property_type_registry: &#bevy_property_path::PropertyTypeRegistry) ->
                    Result<Box<dyn #bevy_property_path::Property>, #bevy_property_path::erased_serde::Error> {
                    #deserialize_fn
            }
       }
    })
}
