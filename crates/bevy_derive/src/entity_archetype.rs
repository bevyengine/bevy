use crate::modules::{get_modules, get_path};
use darling::FromMeta;
use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, Data, DataStruct, DeriveInput, Fields, Ident};

#[derive(FromMeta, Debug, Default)]
struct EntityArchetypeAttributeArgs {
    #[darling(default)]
    pub tag: Option<bool>,
}

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
