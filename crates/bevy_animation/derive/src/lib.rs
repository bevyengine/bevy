//! Derive macros for `bevy_animation`.

use bevy_macro_utils::BevyManifest;
use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput};

/// Used to derive `AnimationEvent` for a type.
#[proc_macro_derive(AnimationEvent)]
pub fn derive_animation_event(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    let name = ast.ident;
    let manifest = BevyManifest::default();
    let bevy_animation_path = manifest.get_path("bevy_animation");
    let bevy_ecs_path = manifest.get_path("bevy_ecs");
    let animation_event_path = quote! { #bevy_animation_path::animation_event };
    let (impl_generics, ty_generics, where_clause) = ast.generics.split_for_impl();
    // TODO: This could derive Event as well.
    quote! {
        impl #impl_generics #animation_event_path::AnimationEvent for #name #ty_generics #where_clause {
            fn trigger(&self, _time: f32, _weight: f32, entity: #bevy_ecs_path::entity::Entity, world: &mut #bevy_ecs_path::world::World) {
                world.entity_mut(entity).trigger(Clone::clone(self));
            }
        }
    }
    .into()
}
