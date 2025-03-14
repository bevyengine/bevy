use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, parse_quote, Data, DeriveInput, Fields, Path, Type};

pub(super) fn derive_map_entities(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = input.ident.clone();

    let ty = &input.ident;
    let component_trait: Path = parse_quote!(::bevy_ecs::component::Component);

    todo!()
}

// Function to generate code to map fields for a struct
fn map_struct_fields(fields: &Fields) -> proc_macro2::TokenStream {
    let mut map_entities = vec![];

    for (i, field) in fields.iter().enumerate() {
        // let map_field = if is_primitive(&field.ty) {
        //     quote! {}
        // } else if let Some(field_name) = &field.ident {
        //     // Named field (struct)
        //     quote! {
        //         self.#field_name.auto_map_entities(entity_mapper);
        //     }
        // } else {
        //     // Unnamed field (tuple-like struct or enum variant)
        //     let idx = syn::Index::from(i);
        //     quote! {
        //         self.#idx.auto_map_entities(entity_mapper);
        //     }
        // };

        // map_entities.push(map_field);
        println!("field: {field:#?}");
    }
    todo!()

    // quote! {
    //     #(#map_entities)*
    // }
}
