use crate::modules::{get_modules, get_path};
use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{
    parse::ParseStream, parse_macro_input, punctuated::Punctuated, Data, DataStruct, DeriveInput,
    Field, Fields, Path,
};

#[derive(Default)]
struct RenderResourceFieldAttributes {
    pub ignore: bool,
    pub buffer: bool,
}

#[derive(Default)]
struct RenderResourceAttributes {
    pub from_self: bool,
}

static RENDER_RESOURCE_ATTRIBUTE_NAME: &str = "render_resources";

pub fn derive_render_resources(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    let modules = get_modules(&ast.attrs);

    let bevy_render_path: Path = get_path(&modules.bevy_render);
    let attributes = ast
        .attrs
        .iter()
        .find(|a| *a.path.get_ident().as_ref().unwrap() == RENDER_RESOURCE_ATTRIBUTE_NAME)
        .map_or_else(RenderResourceAttributes::default, |a| {
            syn::custom_keyword!(from_self);
            let mut attributes = RenderResourceAttributes::default();
            a.parse_args_with(|input: ParseStream| {
                if input.parse::<Option<from_self>>()?.is_some() {
                    attributes.from_self = true;
                }
                Ok(())
            })
            .expect("Invalid 'render_resources' attribute format.");

            attributes
        });
    let struct_name = &ast.ident;
    let struct_name_string = struct_name.to_string();

    if attributes.from_self {
        TokenStream::from(quote! {
            impl #bevy_render_path::renderer::RenderResources for #struct_name {
                fn render_resources_len(&self) -> usize {
                    1
                }

                fn get_render_resource(&self, index: usize) -> Option<&dyn #bevy_render_path::renderer::RenderResource> {
                    if index == 0 {
                        Some(self)
                    } else {
                        None
                    }
                }

                fn get_render_resource_name(&self, index: usize) -> Option<&str> {
                    if index == 0 {
                        Some(#struct_name_string)
                    } else {
                        None
                    }
                }

                fn iter(&self) -> #bevy_render_path::renderer::RenderResourceIterator {
                    #bevy_render_path::renderer::RenderResourceIterator::new(self)
                }
            }
        })
    } else {
        let empty = Punctuated::new();

        let fields = match &ast.data {
            Data::Struct(DataStruct {
                fields: Fields::Named(fields),
                ..
            }) => &fields.named,
            Data::Struct(DataStruct {
                fields: Fields::Unit,
                ..
            }) => &empty,
            _ => panic!("Expected a struct with named fields."),
        };
        let field_attributes = fields
            .iter()
            .map(|field| {
                (
                    field,
                    field
                        .attrs
                        .iter()
                        .find(|a| {
                            *a.path.get_ident().as_ref().unwrap() == RENDER_RESOURCE_ATTRIBUTE_NAME
                        })
                        .map_or_else(RenderResourceFieldAttributes::default, |a| {
                            syn::custom_keyword!(ignore);
                            syn::custom_keyword!(buffer);
                            let mut attributes = RenderResourceFieldAttributes::default();
                            a.parse_args_with(|input: ParseStream| {
                                if input.parse::<Option<ignore>>()?.is_some() {
                                    attributes.ignore = true;
                                } else if input.parse::<Option<buffer>>()?.is_some() {
                                    attributes.buffer = true;
                                }
                                Ok(())
                            })
                            .expect("Invalid 'render_resources' attribute format.");

                            attributes
                        }),
                )
            })
            .collect::<Vec<(&Field, RenderResourceFieldAttributes)>>();
        let mut render_resource_names = Vec::new();
        let mut render_resource_fields = Vec::new();
        let mut render_resource_hints = Vec::new();
        for (field, attrs) in field_attributes.iter() {
            if attrs.ignore {
                continue;
            }

            let field_ident = field.ident.as_ref().unwrap();
            let field_name = field_ident.to_string();
            render_resource_fields.push(field_ident);
            render_resource_names.push(format!("{}_{}", struct_name, field_name));
            if attrs.buffer {
                render_resource_hints
                    .push(quote! {Some(#bevy_render_path::renderer::RenderResourceHints::BUFFER)})
            } else {
                render_resource_hints.push(quote! {None})
            }
        }

        let render_resource_count = render_resource_names.len();
        let render_resource_indices = 0..render_resource_count;

        let struct_name_uppercase = struct_name_string.to_uppercase();
        let render_resource_names_ident =
            format_ident!("{}_RENDER_RESOURCE_NAMES", struct_name_uppercase);
        let render_resource_hints_ident =
            format_ident!("{}_RENDER_RESOURCE_HINTS", struct_name_uppercase);

        TokenStream::from(quote! {
            static #render_resource_names_ident: &[&str] = &[
                #(#render_resource_names,)*
            ];

            static #render_resource_hints_ident: &[Option<#bevy_render_path::renderer::RenderResourceHints>] = &[
                #(#render_resource_hints,)*
            ];

            impl #bevy_render_path::renderer::RenderResources for #struct_name {
                fn render_resources_len(&self) -> usize {
                    #render_resource_count
                }

                fn get_render_resource(&self, index: usize) -> Option<&dyn #bevy_render_path::renderer::RenderResource> {
                    match index {
                        #(#render_resource_indices => Some(&self.#render_resource_fields),)*
                        _ => None,
                    }
                }

                fn get_render_resource_name(&self, index: usize) -> Option<&str> {
                    #render_resource_names_ident.get(index).copied()
                }

                fn get_render_resource_hints(&self, index: usize) -> Option<#bevy_render_path::renderer::RenderResourceHints> {
                    #render_resource_hints_ident.get(index).and_then(|o| *o)
                }

                fn iter(&self) -> #bevy_render_path::renderer::RenderResourceIterator {
                    #bevy_render_path::renderer::RenderResourceIterator::new(self)
                }
            }
        })
    }
}
