use crate::modules::{get_modules, get_path};
use darling::FromMeta;
use inflector::Inflector;
use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{parse_macro_input, Data, DataStruct, DeriveInput, Field, Fields, Path};

#[derive(FromMeta, Debug, Default)]
struct VertexAttributeArgs {
    #[darling(default)]
    pub ignore: Option<bool>,
    #[darling(default)]
    pub instance: Option<bool>,
}

#[derive(Default)]
struct VertexAttributes {
    pub ignore: bool,
    pub instance: bool,
}

impl From<VertexAttributeArgs> for VertexAttributes {
    fn from(args: VertexAttributeArgs) -> Self {
        VertexAttributes {
            ignore: args.ignore.unwrap_or(false),
            instance: args.instance.unwrap_or(false),
        }
    }
}

static UNIFORM_ATTRIBUTE_NAME: &'static str = "uniform";

pub fn derive_as_vertex_buffer_descriptor(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    let modules = get_modules(&ast);

    let bevy_render_path: Path = get_path(&modules.bevy_render);

    let fields = match &ast.data {
        Data::Struct(DataStruct {
            fields: Fields::Named(fields),
            ..
        }) => &fields.named,
        _ => panic!("expected a struct with named fields"),
    };

    let field_attributes = fields
        .iter()
        .map(|f| {
            (
                f,
                f.attrs
                    .iter()
                    .find(|a| {
                        a.path.get_ident().as_ref().unwrap().to_string() == UNIFORM_ATTRIBUTE_NAME
                    })
                    .map(|a| {
                        VertexAttributes::from(
                            VertexAttributeArgs::from_meta(&a.parse_meta().unwrap())
                                .unwrap_or_else(|_err| VertexAttributeArgs::default()),
                        )
                    })
                    .unwrap_or_else(|| VertexAttributes::default()),
            )
        })
        .collect::<Vec<(&Field, VertexAttributes)>>();

    let struct_name = &ast.ident;

    let mut vertex_buffer_field_names_pascal = Vec::new();
    let mut vertex_buffer_field_types = Vec::new();
    for (f, attrs) in field_attributes.iter() {
        if attrs.ignore {
            continue;
        }

        vertex_buffer_field_types.push(&f.ty);
        let pascal_field = f.ident.as_ref().unwrap().to_string().to_pascal_case();
        vertex_buffer_field_names_pascal.push(if attrs.instance {
            format!("I_{}_{}", struct_name, pascal_field)
        } else {
            format!("{}_{}", struct_name, pascal_field)
        });
    }

    let struct_name_string = struct_name.to_string();
    let struct_name_uppercase = struct_name_string.to_uppercase();
    let vertex_buffer_descriptor_ident =
        format_ident!("{}_VERTEX_BUFFER_DESCRIPTOR", struct_name_uppercase);

    TokenStream::from(quote! {
        static #vertex_buffer_descriptor_ident: #bevy_render_path::once_cell::sync::Lazy<#bevy_render_path::pipeline::VertexBufferDescriptor> =
            #bevy_render_path::once_cell::sync::Lazy::new(|| {
                use #bevy_render_path::pipeline::{VertexFormat, AsVertexFormats, VertexAttributeDescriptor};

                let mut vertex_formats: Vec<(&str,&[VertexFormat])>  = vec![
                    #((#vertex_buffer_field_names_pascal, <#vertex_buffer_field_types>::as_vertex_formats()),)*
                ];

                let mut shader_location = 0;
                let mut offset = 0;
                let vertex_attribute_descriptors = vertex_formats.drain(..).map(|(name, formats)| {
                    formats.iter().enumerate().map(|(i, format)| {
                        let size = format.get_size();
                        let formatted_name = if formats.len() > 1 {
                            format!("{}_{}", name, i)
                        } else {
                            format!("{}", name)
                        };
                        let descriptor = VertexAttributeDescriptor {
                            name: formatted_name.into(),
                            offset,
                            format: *format,
                            shader_location,
                        };
                        offset += size;
                        shader_location += 1;
                        descriptor
                    }).collect::<Vec<VertexAttributeDescriptor>>()
                }).flatten().collect::<Vec<VertexAttributeDescriptor>>();

                #bevy_render_path::pipeline::VertexBufferDescriptor {
                    attributes: vertex_attribute_descriptors,
                    name: #struct_name_string.into(),
                    step_mode: #bevy_render_path::pipeline::InputStepMode::Instance,
                    stride: offset,
                }
            });

        impl #bevy_render_path::pipeline::AsVertexBufferDescriptor for #struct_name {
            fn as_vertex_buffer_descriptor() -> &'static #bevy_render_path::pipeline::VertexBufferDescriptor {
                &#vertex_buffer_descriptor_ident
            }
        }
    })
}