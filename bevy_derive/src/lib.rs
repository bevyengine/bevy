extern crate proc_macro;

use darling::FromMeta;
use inflector::Inflector;
use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{parse_macro_input, Data, DataStruct, DeriveInput, Field, Fields, Ident, Path, Type};

#[derive(FromMeta, Debug, Default)]
struct EntityArchetypeAttributeArgs {
    #[darling(default)]
    pub tag: Option<bool>,
}

#[derive(FromMeta, Debug)]
struct ModuleAttributeArgs {
    #[darling(default)]
    pub bevy_render: Option<String>,
    #[darling(default)]
    pub bevy_asset: Option<String>,
    #[darling(default)]
    pub bevy_core: Option<String>,
    #[darling(default)]
    pub bevy_app: Option<String>,
    #[darling(default)]
    pub legion: Option<String>,

    /// If true, it will use the meta "bevy" crate for dependencies by default (ex: bevy:app). If this is set to false, the individual bevy crates
    /// will be used (ex: "bevy_app"). Defaults to "true"
    #[darling(default)]
    pub meta: bool,
}

struct Modules {
    pub bevy_render: String,
    pub bevy_asset: String,
    pub bevy_core: String,
    pub bevy_app: String,
    pub legion: String,
}

impl Modules {
    pub fn meta() -> Modules {
        Modules {
            bevy_asset: "bevy::asset".to_string(),
            bevy_render: "bevy::render".to_string(),
            bevy_core: "bevy::core".to_string(),
            bevy_app: "bevy::app".to_string(),
            legion: "bevy".to_string(),
        }
    }

    pub fn external() -> Modules {
        Modules {
            bevy_asset: "bevy_asset".to_string(),
            bevy_render: "bevy_render".to_string(),
            bevy_core: "bevy_core".to_string(),
            bevy_app: "bevy_app".to_string(),
            legion: "legion".to_string(),
        }
    }
}

impl Default for ModuleAttributeArgs {
    fn default() -> Self {
        ModuleAttributeArgs {
            bevy_asset: None,
            bevy_render: None,
            bevy_core: None,
            bevy_app: None,
            legion: None,
            meta: true,
        }
    }
}

static MODULE_ATTRIBUTE_NAME: &'static str = "module";

fn get_modules(ast: &DeriveInput) -> Modules {
    let module_attribute_args = ast
        .attrs
        .iter()
        .find(|a| a.path.get_ident().as_ref().unwrap().to_string() == MODULE_ATTRIBUTE_NAME)
        .map(|a| {
            ModuleAttributeArgs::from_meta(&a.parse_meta().unwrap())
                .unwrap_or_else(|_err| ModuleAttributeArgs::default())
        });
    if let Some(module_attribute_args) = module_attribute_args {
        let mut modules = if module_attribute_args.meta {
            Modules::meta()
        } else {
            Modules::external()
        };

        if let Some(path) = module_attribute_args.bevy_asset {
            modules.bevy_asset = path;
        }

        if let Some(path) = module_attribute_args.bevy_render {
            modules.bevy_render = path;
        }

        if let Some(path) = module_attribute_args.bevy_core {
            modules.bevy_core = path;
        }

        if let Some(path) = module_attribute_args.bevy_app {
            modules.bevy_app = path;
        }

        modules
    } else {
        Modules::meta()
    }
}

fn get_path(path_str: &str) -> Path {
    syn::parse(path_str.parse::<TokenStream>().unwrap()).unwrap()
}

#[proc_macro_derive(EntityArchetype, attributes(tag, module))]
pub fn derive_entity_archetype(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    let fields = match &ast.data {
        Data::Struct(DataStruct {
            fields: Fields::Named(fields),
            ..
        }) => &fields.named,
        _ => panic!("expected a struct with named fields"),
    };

    let modules = get_modules(&ast);
    let bevy_app_path = get_path(&modules.bevy_app);
    let legion_path = get_path(&modules.legion);

    let tag_fields = fields
        .iter()
        .filter(|f| {
            f.attrs
                .iter()
                .find(|a| a.path.get_ident().as_ref().unwrap().to_string() == "tag")
                .is_some()
        })
        .map(|field| field.ident.as_ref().unwrap())
        .collect::<Vec<&Ident>>();

    let component_fields = fields
        .iter()
        .filter(|f| {
            f.attrs
                .iter()
                .find(|a| a.path.get_ident().as_ref().unwrap().to_string() == "tag")
                .is_none()
        })
        .map(|field| field.ident.as_ref().unwrap())
        .collect::<Vec<&Ident>>();

    let generics = ast.generics;
    let (impl_generics, ty_generics, _where_clause) = generics.split_for_impl();

    let struct_name = &ast.ident;

    TokenStream::from(quote! {
        impl #impl_generics #bevy_app_path::EntityArchetype for #struct_name#ty_generics {
            fn insert(self, world: &mut #legion_path::prelude::World) -> #legion_path::prelude::Entity {
                *world.insert((#(self.#tag_fields,)*),
                    vec![(
                        #(self.#component_fields,)*
                    )
                ]).first().unwrap()
            }

            fn insert_command_buffer(self, command_buffer: &mut #legion_path::prelude::CommandBuffer) -> #legion_path::prelude::Entity {
                *command_buffer.insert((#(self.#tag_fields,)*),
                    vec![(
                        #(self.#component_fields,)*
                    )
                ]).first().unwrap()
            }
        }
    })
}

// TODO: ensure shader_def and instance/vertex are mutually exclusive
#[derive(FromMeta, Debug, Default)]
struct UniformAttributeArgs {
    #[darling(default)]
    pub ignore: Option<bool>,
    #[darling(default)]
    pub shader_def: Option<bool>,
    #[darling(default)]
    pub instance: Option<bool>,
    #[darling(default)]
    pub vertex: Option<bool>,
    #[darling(default)]
    pub bevy_render_path: Option<String>,
    #[darling(default)]
    pub bevy_asset_path: Option<String>,
    #[darling(default)]
    pub bevy_core_path: Option<String>,
}

static UNIFORM_ATTRIBUTE_NAME: &'static str = "uniform";
#[proc_macro_derive(Uniforms, attributes(uniform, module))]
pub fn derive_uniforms(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    let modules = get_modules(&ast);

    let bevy_render_path: Path = get_path(&modules.bevy_render);
    let bevy_core_path: Path = get_path(&modules.bevy_core);
    let bevy_asset_path: Path = get_path(&modules.bevy_asset);

    let fields = match &ast.data {
        Data::Struct(DataStruct {
            fields: Fields::Named(fields),
            ..
        }) => &fields.named,
        _ => panic!("expected a struct with named fields"),
    };

    let uniform_fields = fields
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
                        UniformAttributeArgs::from_meta(&a.parse_meta().unwrap())
                            .unwrap_or_else(|_err| UniformAttributeArgs::default())
                    }),
            )
        })
        .collect::<Vec<(&Field, Option<UniformAttributeArgs>)>>();

    let active_uniform_fields = uniform_fields
        .iter()
        .filter(|(_field, attrs)| {
            attrs.is_none()
                || match attrs.as_ref().unwrap().ignore {
                    Some(ignore) => !ignore,
                    None => true,
                }
        })
        .map(|(f, _attr)| *f)
        .collect::<Vec<&Field>>();

    let shader_def_fields = uniform_fields
        .iter()
        .filter(|(_field, attrs)| match attrs {
            Some(attrs) => match attrs.shader_def {
                Some(shader_def) => shader_def,
                None => false,
            },
            None => false,
        })
        .map(|(f, _attr)| *f)
        .collect::<Vec<&Field>>();

    let shader_def_field_names = shader_def_fields.iter().map(|field| &field.ident);
    let shader_def_field_names_screaming_snake = shader_def_fields.iter().map(|field| {
        field
            .ident
            .as_ref()
            .unwrap()
            .to_string()
            .to_screaming_snake_case()
    });

    let struct_name = &ast.ident;
    let struct_name_string = struct_name.to_string();
    let struct_name_uppercase = struct_name.to_string().to_uppercase();
    let field_infos_ident = format_ident!("{}_FIELD_INFO", struct_name_uppercase);
    let vertex_buffer_descriptor_ident =
        format_ident!("{}_VERTEX_BUFFER_DESCRIPTOR", struct_name_uppercase);

    let active_uniform_field_names = active_uniform_fields
        .iter()
        .map(|field| &field.ident)
        .collect::<Vec<_>>();

    let active_uniform_field_name_strings = active_uniform_fields
        .iter()
        .map(|field| field.ident.as_ref().unwrap().to_string())
        .collect::<Vec<String>>();

    let vertex_buffer_fields = uniform_fields
        .iter()
        .map(|(field, attrs)| {
            (
                field,
                match attrs {
                    Some(attrs) => (
                        (match attrs.instance {
                            Some(instance) => instance,
                            None => false,
                        }),
                        (match attrs.vertex {
                            Some(vertex) => vertex,
                            None => false,
                        }),
                    ),
                    None => (false, false),
                },
            )
        })
        .filter(|(_f, (instance, vertex))| *instance || *vertex);

    let vertex_buffer_field_types = vertex_buffer_fields
        .clone()
        .map(|(f, _)| &f.ty)
        .collect::<Vec<&Type>>();

    let vertex_buffer_field_names_pascal = vertex_buffer_fields
        .map(|(f, (instance, _vertex))| {
            let pascal_field = f.ident.as_ref().unwrap().to_string().to_pascal_case();
            if instance {
                format!("I_{}_{}", struct_name, pascal_field)
            } else {
                format!("{}_{}", struct_name, pascal_field)
            }
        })
        .collect::<Vec<String>>();

    let mut uniform_name_strings = Vec::new();
    let mut texture_and_sampler_name_strings = Vec::new();
    let mut texture_and_sampler_name_idents = Vec::new();
    let field_infos = uniform_fields
        .iter()
        .filter(|(_field, attrs)| {
            attrs.is_none()
                || match attrs.as_ref().unwrap().ignore {
                    Some(ignore) => !ignore,
                    None => true,
                }
        })
        .map(|(f, attrs)| {
            let field_name = f.ident.as_ref().unwrap().to_string();
            let uniform = format!("{}_{}", struct_name, field_name);
            let texture = format!("{}", uniform);
            let sampler = format!("{}_sampler", uniform);
            uniform_name_strings.push(uniform.clone());
            texture_and_sampler_name_strings.push(texture.clone());
            texture_and_sampler_name_strings.push(sampler.clone());
            texture_and_sampler_name_idents.push(f.ident.clone());
            texture_and_sampler_name_idents.push(f.ident.clone());
            let is_instanceable = match attrs {
                Some(attrs) => match attrs.instance {
                    Some(instance) => instance,
                    None => false,
                },
                None => false,
            };
            quote!(#bevy_render_path::shader::FieldInfo {
                name: #field_name,
                uniform_name: #uniform,
                texture_name: #texture,
                sampler_name: #sampler,
                is_instanceable: #is_instanceable,
            })
        });

    TokenStream::from(quote! {
        static #field_infos_ident: &[#bevy_render_path::shader::FieldInfo] = &[
            #(#field_infos,)*
        ];

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
                            name: formatted_name,
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
                    name: #struct_name_string.to_string(),
                    step_mode: #bevy_render_path::pipeline::InputStepMode::Instance,
                    stride: offset,
                }
            });

        impl #bevy_render_path::shader::AsUniforms for #struct_name {
            fn get_field_infos() -> &'static [#bevy_render_path::shader::FieldInfo] {
                #field_infos_ident
            }

            fn get_field_bind_type(&self, name: &str) -> Option<#bevy_render_path::shader::FieldBindType> {
                use #bevy_render_path::shader::AsFieldBindType;
                match name {
                    #(#active_uniform_field_name_strings => self.#active_uniform_field_names.get_field_bind_type(),)*
                    _ => None,
                }
            }

            fn get_uniform_bytes(&self, name: &str) -> Option<Vec<u8>> {
                use #bevy_core_path::bytes::GetBytes;
                match name {
                    #(#uniform_name_strings => Some(self.#active_uniform_field_names.get_bytes()),)*
                    _ => None,
                }
            }

            fn get_uniform_bytes_ref(&self, name: &str) -> Option<&[u8]> {
                use #bevy_core_path::bytes::GetBytes;
                match name {
                    #(#uniform_name_strings => self.#active_uniform_field_names.get_bytes_ref(),)*
                    _ => None,
                }
            }

            fn get_uniform_texture(&self, name: &str) -> Option<#bevy_asset_path::Handle<#bevy_render_path::texture::Texture>> {
                use #bevy_render_path::shader::GetTexture;
                match name {
                    #(#texture_and_sampler_name_strings => self.#texture_and_sampler_name_idents.get_texture(),)*
                    _ => None,
                }
            }

            // TODO: move this to field_info and add has_shader_def(&self, &str) -> bool
            // TODO: this will be very allocation heavy. find a way to either make this allocation free
            // or alternatively only run it when the shader_defs have changed
            fn get_shader_defs(&self) -> Option<Vec<String>> {
                use #bevy_render_path::shader::ShaderDefSuffixProvider;
                let mut potential_shader_defs: Vec<(&'static str, Option<&'static str>)> = vec![
                    #((#shader_def_field_names_screaming_snake, self.#shader_def_field_names.get_shader_def()),)*
                ];

                Some(potential_shader_defs.drain(..)
                    .filter(|(f, shader_def)| shader_def.is_some())
                    .map(|(f, shader_def)| format!("{}_{}{}", #struct_name_uppercase, f, shader_def.unwrap()))
                    .collect::<Vec<String>>())
            }

            fn get_vertex_buffer_descriptor() -> Option<&'static #bevy_render_path::pipeline::VertexBufferDescriptor> {
                if #vertex_buffer_descriptor_ident.attributes.len() == 0 {
                    None
                } else {
                    Some(&#vertex_buffer_descriptor_ident)
                }
            }
        }
    })
}

#[proc_macro_derive(DynamicAppPlugin)]
pub fn derive_app_plugin(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    let struct_name = &ast.ident;

    TokenStream::from(quote! {
        #[no_mangle]
        pub extern "C" fn _create_plugin() -> *mut bevy::app::plugin::AppPlugin {
            // TODO: without this the assembly does nothing. why is that the case?
            print!("");
            // make sure the constructor is the correct type.
            let object = #struct_name {};
            let boxed = Box::new(object);
            Box::into_raw(boxed)
        }
    })
}
