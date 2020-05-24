extern crate proc_macro;

mod modules;

use modules::{get_modules, get_path};
use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, Data, DataStruct, DeriveInput, Fields};

#[proc_macro_derive(Properties, attributes(prop, module))]
pub fn derive_properties(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    let fields = match &ast.data {
        Data::Struct(DataStruct {
            fields: Fields::Named(fields),
            ..
        }) => &fields.named,
        _ => panic!("expected a struct with named fields"),
    };

    let modules = get_modules(&ast);
    let bevy_property_path = get_path(&modules.bevy_property);

    let field_names = fields
        .iter()
        .map(|field| field.ident.as_ref().unwrap().to_string())
        .collect::<Vec<String>>();
    let field_idents = fields
        .iter()
        .map(|field| field.ident.as_ref().unwrap())
        .collect::<Vec<_>>();
    let field_count = fields.len();
    let field_indices = (0..field_count).collect::<Vec<usize>>();

    let generics = ast.generics;
    let (impl_generics, ty_generics, _where_clause) = generics.split_for_impl();

    let struct_name = &ast.ident;

    TokenStream::from(quote! {
        impl #impl_generics #bevy_property_path::Properties for #struct_name#ty_generics {
            fn type_name(&self) -> &str {
                std::any::type_name::<Self>()
            }
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

        impl #impl_generics #bevy_property_path::serde::ser::Serialize for #struct_name#ty_generics {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: #bevy_property_path::serde::ser::Serializer,
            {
                use #bevy_property_path::serde::ser::SerializeMap;
                let mut state = serializer.serialize_map(Some(self.prop_len()))?;
                state.serialize_entry("type", self.type_name())?;
                for (name, prop) in self.iter_props() {
                    state.serialize_entry(name, prop)?;
                }
                state.end()
            }
        }

        impl #impl_generics #bevy_property_path::Property for #struct_name#ty_generics {
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
                    for (name, prop) in properties.iter_props() {
                        self.prop_mut(name).map(|p| p.apply(prop));
                    }
                } else {
                    panic!("attempted to apply non-Properties type to Properties type");
                }
            }
        }

        impl #impl_generics #bevy_property_path::AsProperties for #struct_name#ty_generics {
            fn as_properties(&self) -> Option<&dyn #bevy_property_path::Properties> {
                Some(self)
            }
        }
    })
}
