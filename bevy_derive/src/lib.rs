extern crate proc_macro;

use inflector::Inflector;
use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{parse_macro_input, Data, DataStruct, DeriveInput, Fields};

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

#[proc_macro_derive(Uniforms)]
pub fn derive_uniforms(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    let fields = match &ast.data {
        Data::Struct(DataStruct {
            fields: Fields::Named(fields),
            ..
        }) => &fields.named,
        _ => panic!("expected a struct with named fields"),
    };

    let struct_name = &ast.ident;
    let struct_name_screaming_snake = struct_name.to_string().to_screaming_snake_case();
    let info_ident = format_ident!("{}_UNIFORM_INFO", struct_name_screaming_snake);
    let layout_ident = format_ident!("{}_UNIFORM_LAYOUTS", struct_name_screaming_snake);
    let layout_arrays = (0..fields.len()).map(|_| quote!(&[]));
    let uniform_name_uniform_info = fields
        .iter()
        .map(|field| format!("{}_{}", struct_name, field.ident.as_ref().unwrap()))
        .collect::<Vec<String>>();
    let get_uniform_bytes_field_name = fields.iter().map(|field| &field.ident);
    let get_uniform_bytes_uniform_name = uniform_name_uniform_info.clone();
    let get_uniform_info_uniform_name = uniform_name_uniform_info.clone();
    let get_uniform_info_array_refs = (0..fields.len()).map(|i| quote!(&#info_ident[#i]));

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
