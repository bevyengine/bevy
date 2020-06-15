use crate::{
    attributes::{get_attributes, get_field_attributes},
    modules::{get_modules, get_path},
};
use darling::FromMeta;
use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{parse_macro_input, DeriveInput, Path};

#[derive(FromMeta, Debug, Default)]
struct RenderResourceAttributeArgs {
    #[darling(default)]
    pub ignore: Option<bool>,
    #[darling(default)]
    pub buffer: Option<bool>,
    #[darling(default)]
    pub from_self: Option<bool>,
}

#[derive(Default)]
struct RenderResourceAttributes {
    pub ignore: bool,
    pub buffer: bool,
    pub from_self: bool,
}

impl From<RenderResourceAttributeArgs> for RenderResourceAttributes {
    fn from(args: RenderResourceAttributeArgs) -> Self {
        Self {
            ignore: args.ignore.unwrap_or(false),
            buffer: args.buffer.unwrap_or(false),
            from_self: args.from_self.unwrap_or(false),
        }
    }
}

static RENDER_RESOURCE_ATTRIBUTE_NAME: &'static str = "render_resources";

pub fn derive_render_resources(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    let modules = get_modules(&ast);

    let bevy_render_path: Path = get_path(&modules.bevy_render);
    let attributes = get_attributes::<RenderResourceAttributes, RenderResourceAttributeArgs>(
        RENDER_RESOURCE_ATTRIBUTE_NAME,
        &ast.attrs,
    );
    let struct_name = &ast.ident;
    let struct_name_string = struct_name.to_string();

    if attributes.from_self {
        TokenStream::from(quote! {
            impl #bevy_render_path::render_resource::RenderResources for #struct_name {
                fn render_resources_len(&self) -> usize {
                    1
                }

                fn get_render_resource(&self, index: usize) -> Option<&dyn #bevy_render_path::render_resource::RenderResource> {
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

                fn iter_render_resources(&self) -> #bevy_render_path::render_resource::RenderResourceIterator {
                    #bevy_render_path::render_resource::RenderResourceIterator::new(self)
                }
            }
        })
    } else {
        let field_attributes = get_field_attributes::<
            RenderResourceAttributes,
            RenderResourceAttributeArgs,
        >(RENDER_RESOURCE_ATTRIBUTE_NAME, &ast.data);

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
                render_resource_hints.push(
                    quote! {Some(#bevy_render_path::render_resource::RenderResourceHints::BUFFER)},
                )
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

            static #render_resource_hints_ident: &[Option<#bevy_render_path::render_resource::RenderResourceHints>] = &[
                #(#render_resource_hints,)*
            ];

            impl #bevy_render_path::render_resource::RenderResources for #struct_name {
                fn render_resources_len(&self) -> usize {
                    #render_resource_count
                }

                fn get_render_resource(&self, index: usize) -> Option<&dyn #bevy_render_path::render_resource::RenderResource> {
                    match index {
                        #(#render_resource_indices => Some(&self.#render_resource_fields),)*
                        _ => None,
                    }
                }

                fn get_render_resource_name(&self, index: usize) -> Option<&str> {
                    Some(#render_resource_names_ident[index])
                }

                fn get_render_resource_hints(&self, index: usize) -> Option<#bevy_render_path::render_resource::RenderResourceHints> {
                    #render_resource_hints_ident[index].clone()
                }

                fn iter_render_resources(&self) -> #bevy_render_path::render_resource::RenderResourceIterator {
                    #bevy_render_path::render_resource::RenderResourceIterator::new(self)
                }
            }
        })
    }
}
