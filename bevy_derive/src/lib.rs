use proc_macro::TokenStream;
use syn::{parse_macro_input, DeriveInput, Data, DataStruct, Fields};
use quote::quote;

#[proc_macro_derive(EntityArchetype)]
pub fn derive_entity_archetype(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    let fields = match &ast.data {
        Data::Struct(DataStruct { fields: Fields::Named(fields), .. }) => &fields.named,
        _ => panic!("expected a struct with named fields"),
    };

    let struct_name = &ast.ident;
    let field_name = fields.iter().map(|field| &field.ident);

    TokenStream::from(quote! {
        impl EntityArchetype for #struct_name {
            fn insert(self, world: &mut World) -> Entity {
                *world.insert((), vec![(
                    #(self.#field_name),*
                )]).first().unwrap()
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
