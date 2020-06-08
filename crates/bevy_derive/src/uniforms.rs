use crate::{
    attributes::get_field_attributes,
    modules::{get_modules, get_path},
};
use darling::FromMeta;
use inflector::Inflector;
use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{parse_macro_input, DeriveInput, Path};

#[derive(FromMeta, Debug, Default)]
struct UniformAttributeArgs {
    #[darling(default)]
    pub ignore: Option<bool>,
    #[darling(default)]
    pub shader_def: Option<bool>,
    #[darling(default)]
    pub buffer: Option<bool>,
}

#[derive(Default)]
struct UniformAttributes {
    pub ignore: bool,
    pub shader_def: bool,
    pub buffer: bool,
}

impl From<UniformAttributeArgs> for UniformAttributes {
    fn from(args: UniformAttributeArgs) -> Self {
        UniformAttributes {
            ignore: args.ignore.unwrap_or(false),
            shader_def: args.shader_def.unwrap_or(false),
            buffer: args.buffer.unwrap_or(false),
        }
    }
}

static UNIFORM_ATTRIBUTE_NAME: &'static str = "uniform";

pub fn derive_uniforms(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    let modules = get_modules(&ast);

    let bevy_render_path: Path = get_path(&modules.bevy_render);
    let bevy_core_path: Path = get_path(&modules.bevy_core);
    let bevy_asset_path: Path = get_path(&modules.bevy_asset);

    let field_attributes = get_field_attributes::<UniformAttributes, UniformAttributeArgs>(
        UNIFORM_ATTRIBUTE_NAME,
        &ast.data,
    );

    let struct_name = &ast.ident;

    let mut active_uniform_field_names = Vec::new();
    let mut active_uniform_field_name_strings = Vec::new();
    let mut uniform_name_strings = Vec::new();
    let mut texture_and_sampler_name_strings = Vec::new();
    let mut texture_and_sampler_name_idents = Vec::new();
    let mut field_infos = Vec::new();
    let mut get_field_bind_types = Vec::new();

    let mut shader_def_field_names = Vec::new();
    let mut shader_def_field_names_screaming_snake = Vec::new();

    for (f, attrs) in field_attributes.iter() {
        let field_name = f.ident.as_ref().unwrap().to_string();
        if !attrs.ignore {
            let active_uniform_field_name = &f.ident;
            active_uniform_field_names.push(&f.ident);
            active_uniform_field_name_strings.push(field_name.clone());
            let uniform = format!("{}_{}", struct_name, field_name);
            let texture = format!("{}", uniform);
            let sampler = format!("{}_sampler", uniform);
            uniform_name_strings.push(uniform.clone());
            texture_and_sampler_name_strings.push(texture.clone());
            texture_and_sampler_name_strings.push(sampler.clone());
            texture_and_sampler_name_idents.push(f.ident.clone());
            texture_and_sampler_name_idents.push(f.ident.clone());
            field_infos.push(quote!(#bevy_render_path::shader::FieldInfo {
                name: #field_name,
                uniform_name: #uniform,
                texture_name: #texture,
                sampler_name: #sampler,
            }));

            if attrs.buffer {
                get_field_bind_types.push(quote!({
                    let bind_type = self.#active_uniform_field_name.get_bind_type();
                    let size = if let Some(#bevy_render_path::shader::FieldBindType::Uniform { size }) = bind_type {
                        size
                    } else {
                        panic!("Uniform field was labeled as a 'buffer', but it does not have a compatible type.")
                    };
                    Some(#bevy_render_path::shader::FieldBindType::Buffer { size })
                }))
            } else {
                get_field_bind_types.push(quote!(self.#active_uniform_field_name.get_bind_type()))
            }
        }

        if attrs.shader_def {
            shader_def_field_names.push(&f.ident);
            shader_def_field_names_screaming_snake.push(field_name.to_screaming_snake_case())
        }
    }

    let struct_name_string = struct_name.to_string();
    let struct_name_uppercase = struct_name_string.to_uppercase();
    let field_infos_ident = format_ident!("{}_FIELD_INFO", struct_name_uppercase);

    TokenStream::from(quote! {
        static #field_infos_ident: &[#bevy_render_path::shader::FieldInfo] = &[
            #(#field_infos,)*
        ];

        impl #bevy_render_path::shader::Uniforms for #struct_name {
            fn get_field_infos() -> &'static [#bevy_render_path::shader::FieldInfo] {
                #field_infos_ident
            }

            fn get_field_bind_type(&self, name: &str) -> Option<#bevy_render_path::shader::FieldBindType> {
                None
            }

            fn get_uniform_texture(&self, name: &str) -> Option<#bevy_asset_path::Handle<#bevy_render_path::texture::Texture>> {
                None
            }

            fn write_uniform_bytes(&self, name: &str, buffer: &mut [u8]) {
                use #bevy_core_path::bytes::Bytes;
            }
            fn uniform_byte_len(&self, name: &str) -> usize {
                0
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
        }
    })
}

pub fn derive_uniform(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);

    let modules = get_modules(&ast);
    let bevy_asset_path = get_path(&modules.bevy_asset);
    let bevy_render_path = get_path(&modules.bevy_render);

    let generics = ast.generics;
    let (impl_generics, ty_generics, _where_clause) = generics.split_for_impl();

    let struct_name = &ast.ident;
    let struct_name_string = struct_name.to_string();

    TokenStream::from(quote! {
        impl #impl_generics #bevy_render_path::shader::Uniforms for #struct_name#ty_generics {
            fn get_field_infos() -> &'static [#bevy_render_path::shader::FieldInfo] {
                static FIELD_INFOS: &[#bevy_render_path::shader::FieldInfo] = &[
                    #bevy_render_path::shader::FieldInfo {
                       name: #struct_name_string,
                       uniform_name: #struct_name_string,
                       texture_name: #struct_name_string,
                       sampler_name: #struct_name_string,
                   }
                ];
                &FIELD_INFOS
            }

            fn get_field_bind_type(&self, name: &str) -> Option<#bevy_render_path::shader::FieldBindType> {
                None
            }

            fn write_uniform_bytes(&self, name: &str, buffer: &mut [u8]) {
            }
            fn uniform_byte_len(&self, name: &str) -> usize {
                0
            }

            fn get_uniform_texture(&self, name: &str) -> Option<#bevy_asset_path::Handle<#bevy_render_path::texture::Texture>> {
                None
            }

            // TODO: move this to field_info and add has_shader_def(&self, &str) -> bool
            // TODO: this will be very allocation heavy. find a way to either make this allocation free
            // or alternatively only run it when the shader_defs have changed
            fn get_shader_defs(&self) -> Option<Vec<String>> {
                None
            }
        }
    })
}
