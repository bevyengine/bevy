use bevy_macro_utils::BevyManifest;
use inflector::Inflector;
use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, Data, DataStruct, DeriveInput, Fields, Path};

static SHADER_DEF_ATTRIBUTE_NAME: &str = "shader_def";

pub fn derive_shader_defs(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    let bevy_render_path: Path = BevyManifest::default().get_path(crate::modules::BEVY_RENDER);

    let fields = match &ast.data {
        Data::Struct(DataStruct {
            fields: Fields::Named(fields),
            ..
        }) => &fields.named,
        _ => panic!("Expected a struct with named fields."),
    };

    let shader_def_idents = fields
        .iter()
        .filter(|f| {
            f.attrs
                .iter()
                .any(|a| *a.path.get_ident().as_ref().unwrap() == SHADER_DEF_ATTRIBUTE_NAME)
        })
        .map(|f| f.ident.as_ref().unwrap())
        .collect::<Vec<&syn::Ident>>();
    let struct_name = &ast.ident;
    let struct_name_pascal_case = ast.ident.to_string().to_pascal_case();
    let shader_defs = shader_def_idents
        .iter()
        .map(|i| format!("{}_{}", struct_name_pascal_case, i).to_uppercase());

    let shader_defs_len = shader_defs.len();
    let shader_def_indices = 0..shader_defs_len;

    let generics = ast.generics;
    let (impl_generics, ty_generics, _where_clause) = generics.split_for_impl();

    TokenStream::from(quote! {
        impl #impl_generics #bevy_render_path::shader::ShaderDefs for #struct_name#ty_generics {
            fn shader_defs_len(&self) -> usize {
                #shader_defs_len
            }

            fn get_shader_def(&self, index: usize) -> Option<&str> {
                use #bevy_render_path::shader::ShaderDef;
                match index {
                    #(#shader_def_indices => if self.#shader_def_idents.is_defined() {
                        Some(#shader_defs)
                    } else {
                        None
                    },)*
                    _ => None,
                }
            }

            fn iter_shader_defs(&self) -> #bevy_render_path::shader::ShaderDefIterator {
                #bevy_render_path::shader::ShaderDefIterator::new(self)
            }
        }
    })
}
