use crate::modules::{get_modules, get_path};
use proc_macro::TokenStream;
use quote::quote;
use syn::{parse::ParseStream, parse_macro_input, Data, DataStruct, DeriveInput, Field, Fields};

pub fn derive_animated_component(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);

    let fields = match &ast.data {
        Data::Struct(DataStruct {
            fields: Fields::Named(fields),
            ..
        }) => &fields.named,
        _ => panic!("Expected a struct with named fields."),
    };

    // Filter fields
    let fields = fields
        .iter()
        .filter(|field| {
            field
                .attrs
                .iter()
                .find(|a| *a.path.get_ident().as_ref().unwrap() == "animated")
                .map_or_else(
                    || true,
                    |a| {
                        syn::custom_keyword!(ignore);
                        a.parse_args_with(|input: ParseStream| {
                            if input.parse::<Option<ignore>>()?.is_some() {
                                Ok(false)
                            } else {
                                Ok(true)
                            }
                        })
                        .expect("Invalid 'animated' attribute format.")
                    },
                )
        })
        .collect::<Vec<&Field>>();

    let modules = get_modules(&ast.attrs);
    let bevy_animation = get_path(&modules.bevy_animation);
    let bevy_ecs = get_path(&modules.bevy_ecs);
    let bevy_asset = get_path(&modules.bevy_asset);

    let struct_name = &ast.ident;
    let properties = fields
        .iter()
        .map(|field| format!("{}.{}", struct_name, field.ident.as_ref().unwrap()));
    let field_types = fields.iter().map(|field| &field.ty);
    let fields = fields.iter().map(|field| field.ident.as_ref().unwrap());

    let generics = ast.generics;
    let (impl_generics, ty_generics, _where_clause) = generics.split_for_impl();

    TokenStream::from(quote! {
        impl #impl_generics #bevy_animation::AnimatedComponent for #struct_name #ty_generics {
            fn animator_update_system(
                clips: #bevy_ecs::Res<#bevy_asset::Assets<#bevy_animation::Clip>>,
                mut animator_blending: #bevy_ecs::Local<#bevy_animation::AnimatorBlending>,
                animators_query: #bevy_ecs::Query<& #bevy_animation::Animator>,
                component_query: #bevy_ecs::Query<&mut Self>,
            ) {
                // let __span = tracing::info_span!("animator_transform_update_system");
                // let __guard = __span.enter();

                let mut components = vec![];

                for animator in animators_query.iter() {
                    let mut blend_group = animator_blending.begin_blending();

                    components.clear();

                    // ? NOTE: Lazy get each component is worse than just fetching everything at once
                    // Pre-fetch all transforms to avoid calling get_mut multiple times
                    // SAFETY: each component will be updated one at the time and this function
                    // currently has the mutability over the Transform type, so no race conditions
                    // are possible
                    unsafe {
                        for entity in animator.entities() {
                            components.push(
                                entity
                                    .map(|entity| component_query.get_unsafe(entity).ok())
                                    .flatten(),
                            );
                        }
                    }

                    for (_, layer, clip_handle, entities_map) in animator.animate() {
                        let w = layer.weight;
                        if w < 1.0e-8 {
                            continue;
                        }

                        if let Some(clip) = clips.get(clip_handle) {
                            let time = layer.time;

                            // SAFETY: Never a different thread will modify or access the same index as this one;
                            // Plus as a nice and crazy feature each property is grouped by name into their own cache line
                            // buckets, this way no cache line will be accessed by the same thread unless the same property
                            // is accessed by two different systems, which is possible but weird and will hit the performance a bit
                            let keyframes = unsafe { layer.keyframes_unsafe() };

                            #(
                                if let Some(curves) = clip
                                    .get(#properties)
                                    .map(|curve_untyped| curve_untyped.downcast_ref::<#field_types>())
                                    .flatten()
                                {
                                    for (entity_index, (curve_index, curve)) in curves.iter() {
                                        let entity_index = entities_map[entity_index as usize];
                                        if let Some(ref mut component) = components[entity_index as usize] {
                                            let (k, v) = curve.sample_indexed(keyframes[*curve_index], time);
                                            keyframes[*curve_index] = k;
                                            component.#fields.blend(&mut blend_group, v, w);
                                        }
                                    }
                                }
                            )*
                        }
                    }
                }

                // std::mem::drop(__guard);
            }
        }
    })
}
