extern crate proc_macro;

use darling::FromMeta;
use inflector::Inflector;
use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{parse_macro_input, Data, DataStruct, DeriveInput, Field, Fields};

#[proc_macro_derive(EntityArchetype)]
pub fn derive_entity_archetype(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    let fields = match &ast.data {
        Data::Struct(DataStruct {
            fields: Fields::Named(fields),
            ..
        }) => &fields.named,
        _ => panic!("expected a struct with named fields"),
    };

    let struct_name = &ast.ident;
    let field_name = fields.iter().map(|field| &field.ident);

    TokenStream::from(quote! {
        impl EntityArchetype for #struct_name {
            fn insert(self, world: &mut World) -> Entity {
                *world.insert((), vec![(
                    #(self.#field_name,)*
                )]).first().unwrap()
            }
        }
    })
}
#[derive(FromMeta, Debug, Default)]
struct UniformAttributeArgs {
    #[darling(default)]
    pub ignore: Option<bool>,
    #[darling(default)]
    pub shader_def: Option<String>,
}

#[proc_macro_derive(Uniforms, attributes(uniform))]
pub fn derive_uniforms(input: TokenStream) -> TokenStream {
    const UNIFORM_ATTRIBUTE_NAME: &'static str = "uniform";
    let ast = parse_macro_input!(input as DeriveInput);

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

    let shader_defs = uniform_fields
        .iter()
        .filter(|(_f, attrs)| match attrs {
            Some(attrs) => attrs.shader_def.is_some(),
            None => false,
        })
        .map(|(f, attrs)| {
            // attrs is guaranteed to be set because we checked in filter
            let shader_def = attrs.as_ref().unwrap().shader_def.as_ref().unwrap();
            if shader_def.len() == 0 {
                f.ident.as_ref().unwrap().to_string()
            } else {
                shader_def.to_string()
            }
        })
        .collect::<Vec<String>>();

    let struct_name = &ast.ident;
    let struct_name_screaming_snake = struct_name.to_string().to_screaming_snake_case();
    let info_ident = format_ident!("{}_UNIFORM_INFO", struct_name_screaming_snake);
    let layout_ident = format_ident!("{}_UNIFORM_LAYOUTS", struct_name_screaming_snake);
    let layout_arrays = (0..active_uniform_fields.len()).map(|_| quote!(&[]));
    let uniform_name_uniform_info = active_uniform_fields
        .iter()
        .map(|field| format!("{}_{}", struct_name, field.ident.as_ref().unwrap()))
        .collect::<Vec<String>>();
    let get_uniform_bytes_field_name = active_uniform_fields.iter().map(|field| &field.ident);
    let get_uniform_bytes_uniform_name = uniform_name_uniform_info.clone();
    let get_uniform_info_uniform_name = uniform_name_uniform_info.clone();
    let get_uniform_info_array_refs =
        (0..active_uniform_fields.len()).map(|i| quote!(&#info_ident[#i]));

    TokenStream::from(quote! {
        const #info_ident: &[UniformInfo] = &[
            #(UniformInfo {
                name: #uniform_name_uniform_info,
                bind_type: BindType::Uniform {
                    dynamic: false,
                    // TODO: fill this in with properties
                    properties: Vec::new(),
                },
            },)*
        ];

        const #layout_ident: &[&[UniformPropertyType]] = &[
            #(#layout_arrays,)*
        ];

        impl AsUniforms for #struct_name {
            fn get_uniform_infos(&self) -> &[UniformInfo] {
                #info_ident
            }

            fn get_uniform_layouts(&self) -> &[&[UniformPropertyType]] {
                #layout_ident
            }

            fn get_uniform_bytes(&self, name: &str) -> Option<Vec<u8>> {
                match name {
                    #(#get_uniform_bytes_uniform_name => Some(self.#get_uniform_bytes_field_name.get_bytes()),)*
                    _ => None,
                }
            }
            fn get_uniform_info(&self, name: &str) -> Option<&UniformInfo> {
                match name {
                    #(#get_uniform_info_uniform_name => Some(#get_uniform_info_array_refs),)*
                    _ => None,
                }
            }

            fn get_shader_defs(&self) -> Vec<&'static str> {
                vec![
                    #(#shader_defs,)*
                ]
            }
        }
    })
}

#[proc_macro_derive(RegisterAppPlugin)]
pub fn derive_app_plugin(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    let struct_name = &ast.ident;

    TokenStream::from(quote! {
        #[no_mangle]
        pub extern "C" fn _create_plugin() -> *mut AppPlugin {
            // TODO: without this the assembly does nothing. why is that the case?
            print!("");
            // make sure the constructor is the correct type.
            let object = #struct_name {};
            let boxed = Box::new(object);
            Box::into_raw(boxed)
        }
    })
}
