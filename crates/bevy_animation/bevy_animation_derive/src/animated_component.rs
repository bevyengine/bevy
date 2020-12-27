use crate::{
    animated_properties::{query_prop, query_prop_nested},
    help::parse_animate_options,
    modules::{get_modules, get_path},
};
use proc_macro::TokenStream;
use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::quote;
use std::collections::{HashMap, HashSet};
use syn::{
    parse::{Error as ParseError, Parse, ParseStream, Result as ParserResult},
    parse_macro_input, parse_quote,
    punctuated::Punctuated,
    token::Comma,
    Data, DataStruct, DeriveInput, Field, Fields, Ident, Type,
};

fn animate_property(
    property_bit_masks: &mut HashSet<Ident>,
    property_map: &mut HashMap<String, usize>,
    property_name: String,
    ident: &Ident,
    nested_fields: Option<&Ident>,
    ty: &Type,
) -> TokenStream2 {
    let mut c = ident.to_string();
    for n in nested_fields {
        c.push('_');
        c.push_str(&n.to_string());
    }
    c = c.to_uppercase();
    let c = Ident::new(&c, Span::call_site());
    property_bit_masks.insert(c.clone());

    let default_property_index = property_map.len();
    let property_index = property_map
        .entry(property_name)
        .or_insert(default_property_index);

    let nested_fields = nested_fields.into_iter();

    quote! {
        if let Some(curves) = clip
            .get(Self::PROPERTIES[#property_index])
            .map(|curve_untyped| curve_untyped.downcast_ref::<#ty>())
            .flatten()
        {
            for (entity_index, (curve_index, curve)) in curves.iter() {
                let entity_index = entities_map[entity_index as usize] as usize;
                if let Some(ref mut component) = components[entity_index] {
                    let kr = &mut keyframes[*curve_index];
                    let (k, v) = curve.sample_indexed(*kr, time);
                    *kr = k;
                    component.#ident #(. #nested_fields)* .blend(entity_index, #c, &mut blend_group, v, w);
                }
            }
        }
    }
}

fn animate_property_extended<'a>(
    property_bit_masks: &mut HashSet<Ident>,
    property_map: &mut HashMap<String, usize>,
    property_name: String,
    ident: &Ident,
    nested_fields: impl Iterator<Item = &'a Field>,
    ty: &Type,
) -> TokenStream2 {
    let nested_fields = nested_fields
        .map(|field| field.ident.as_ref().unwrap())
        .collect::<Vec<_>>();

    let bit_masks = nested_fields
        .iter()
        .map(|field_inner| {
            let mut c = ident.to_string();
            c.push('_');
            c.push_str(&field_inner.to_string());
            c = c.to_uppercase();
            let c = Ident::new(&c, Span::call_site());
            c
        })
        .collect::<Vec<_>>();

    for c in &bit_masks {
        property_bit_masks.insert(c.clone());
    }

    let default_property_index = property_map.len();
    let property_index = property_map
        .entry(property_name)
        .or_insert(default_property_index);

    quote! {
        if let Some(curves) = clip
            .get(Self::PROPERTIES[#property_index])
            .map(|curve_untyped| curve_untyped.downcast_ref::<#ty>())
            .flatten()
        {
            for (entity_index, (curve_index, curve)) in curves.iter() {
                let entity_index = entities_map[entity_index as usize] as usize;
                if let Some(ref mut component) = components[entity_index] {
                    let kr = &mut keyframes[*curve_index];
                    let (k, v) = curve.sample_indexed(*kr, time);
                    *kr = k;
                    #(component.#ident.#nested_fields.blend(entity_index, #bit_masks, &mut blend_group, v.#nested_fields, w);)*
                }
            }
        }
    }
}

pub fn derive_animated_component(input: TokenStream) -> TokenStream {
    let derive_input = parse_macro_input!(input as DeriveInput);
    let struct_name = &derive_input.ident;

    // Modules namespaces
    let modules = get_modules(&derive_input.attrs);
    let bevy_animation = get_path(&modules.bevy_animation);
    let bevy_ecs = get_path(&modules.bevy_ecs);
    let bevy_asset = get_path(&modules.bevy_asset);

    let fields = match &derive_input.data {
        Data::Struct(DataStruct {
            fields: Fields::Named(fields),
            ..
        }) => &fields.named,
        _ => panic!("Expected a struct with named fields."),
    };

    // Animation blocks for each animated property
    let mut properties_blocks = vec![];
    let mut property_bit_masks = HashSet::default();
    let mut property_map = HashMap::default();

    // Filter fields
    for field in fields.iter() {
        let ident = field.ident.as_ref().unwrap();
        let ty = &field.ty;
        let field_options = field
            .attrs
            .iter()
            .find(|a| *a.path.get_ident().as_ref().unwrap() == "animated")
            .map(|attr| parse_animate_options(attr).expect("Invalid 'animated' attribute format."));

        if let Some(options) = field_options {
            // Have some custom options attached
            let nested_fields = options.fields.unwrap_or_else(Default::default);
            let nested = nested_fields
                .iter()
                .map(|nested_field| {
                    let nested_ident = nested_field.ident.as_ref().unwrap();
                    animate_property(
                        &mut property_bit_masks,
                        &mut property_map,
                        format!("{}.{}.{}", struct_name, ident, nested_ident),
                        ident,
                        Some(nested_ident),
                        &nested_field.ty,
                    )
                })
                .collect::<Vec<_>>();

            if options.ignore {
                if !nested_fields.is_empty() {
                    // Only expanded properties
                    properties_blocks.push(quote! { #( #nested )* });
                }
                continue;
            } else {
                if !nested_fields.is_empty() {
                    // With expanded attributes
                    let root = animate_property_extended(
                        &mut property_bit_masks,
                        &mut property_map,
                        format!("{}.{}", struct_name, ident),
                        ident,
                        nested_fields.iter(),
                        &ty,
                    );
                    properties_blocks.push(quote! { #root else {  #( #nested )* } });
                    continue;
                }
                // Default behavior
            }
        }

        // Default
        let root = animate_property(
            &mut property_bit_masks,
            &mut property_map,
            format!("{}.{}", struct_name, ident),
            ident,
            None,
            ty,
        );
        properties_blocks.push(root);
    }

    let generics = derive_input.generics;
    let (impl_generics, ty_generics, _where_clause) = generics.split_for_impl();

    let (constants_values, constants_idents): (Vec<u32>, Vec<&Ident>) = property_bit_masks
        .iter()
        .enumerate()
        .map(|(i, n)| {
            if i >= 32 {
                panic!("more than 32 animated fields");
            }

            ((1 << i) as u32, n)
        })
        .unzip();

    let mut properties_names = vec![];
    properties_names.resize_with(property_map.len(), || String::new());
    for (k, i) in property_map {
        properties_names[i] = k;
    }

    TokenStream::from(quote! {

        // #(
        //     pub struct #property_ident;

        //     impl #property_ident {
        //         #(
        //             pub const fn #property_field_ident(&self) -> #bevy_animation::Prop<#property_ty, #property_nested> {
        //                 #bevy_animation::Prop::borrowed( #struct_name::PROPERTIES[#property_index] )
        //             }
        //         )*
        //     }
        // )*

        // #(
        //     impl std::ops::Deref for #bevy_animation::Prop<#property_ty, #property_nested> {
        //         type Target = #property_nested;

        //         #[inline(always)]
        //         fn deref(&self) -> &Self::Target {
        //             &#property_nested
        //         }
        //     }
        // )*

        impl #impl_generics #bevy_animation::AnimatedProperties for #struct_name #ty_generics {
            type Props = #root_properties;

            const PROPERTIES: &'static [&'static str] = &[ #( #properties_names, )* ];

            #[inline(always)]
            fn props() -> Self::Props {
                #root_properties
            }
        }

        impl #impl_generics #bevy_animation::AnimatedComponent for #struct_name #ty_generics {
            fn animator_update_system(
                clips: #bevy_ecs::Res<#bevy_asset::Assets<#bevy_animation::Clip>>,
                mut animator_blending: #bevy_ecs::Local<#bevy_animation::AnimatorBlending>,
                animators_query: #bevy_ecs::Query<& #bevy_animation::Animator>,
                component_query: #bevy_ecs::Query<&mut Self>,
            ) {

                #(const #constants_idents: u32 = #constants_values;)*

                // TODO: add tracing span
                // let __span = tracing::info_span!("animator_transform_update_system");
                // let __guard = __span.enter();

                let mut components = vec![];

                for animator in animators_query.iter() {

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

                    let mut blend_group = animator_blending.begin_blending(components.len());

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

                            #(#properties_blocks)*
                        }
                    }
                }

                // std::mem::drop(__guard);
            }
        }
    })
}
