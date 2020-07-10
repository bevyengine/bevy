use crate::modules::{get_modules, get_path};
use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, Data, DataStruct, DeriveInput, Fields, Ident};

pub fn derive_component_set(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    let fields = match &ast.data {
        Data::Struct(DataStruct {
            fields: Fields::Named(fields),
            ..
        }) => &fields.named,
        _ => panic!("expected a struct with named fields"),
    };

    let modules = get_modules();
    let bevy_app_path = get_path(&modules.bevy_app);
    let legion_path = get_path(&modules.legion);

    let component_fields = fields
        .iter()
        .map(|field| field.ident.as_ref().unwrap())
        .collect::<Vec<&Ident>>();

    let generics = ast.generics;
    let (impl_generics, ty_generics, _where_clause) = generics.split_for_impl();

    let struct_name = &ast.ident;

    TokenStream::from(quote! {
        impl #impl_generics #bevy_app_path::ComponentSet for #struct_name#ty_generics {
            fn insert(self, world: &mut #legion_path::prelude::World) -> #legion_path::prelude::Entity {
                *world.insert((),
                    vec![(
                        #(self.#component_fields,)*
                    )
                ]).first().unwrap()
            }

            fn insert_command_buffer(self, command_buffer: &mut #legion_path::prelude::CommandBuffer) -> #legion_path::prelude::Entity {
                *command_buffer.insert((),
                    vec![(
                        #(self.#component_fields,)*
                    )
                ]).first().unwrap()
            }
        }
    })
}
