use bevy_macro_utils::BevyManifest;
use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput};

pub fn derive_animation_event(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    let manifest = BevyManifest::shared();
    let bevy_ecs = manifest.get_path("bevy_ecs");
    let bevy_animation = manifest.get_path("bevy_animation");

    let generics = ast.generics;
    let (impl_generics, type_generics, where_clause) = generics.split_for_impl();

    let struct_name = &ast.ident;

    quote! {
        impl #impl_generics #bevy_ecs::event::Event for #struct_name #type_generics #where_clause {
            type Trigger<'a> = #bevy_animation::AnimationEventTrigger;
        }

        impl #impl_generics #bevy_animation::AnimationEvent for #struct_name #type_generics #where_clause {
        }
    }
    .into()
}
