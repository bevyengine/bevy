//! Generic animation for any type that implements the `Reflect` trait
//!
//! This will never be as fast as the code gen approach but it will give
//! a nice way of provide animation to external types or even to types derived
//! from scrips!

use std::{any::TypeId, marker::PhantomData};

use bevy_asset::{prelude::*, Asset};
use bevy_ecs::prelude::*;
use bevy_math::prelude::*;
use bevy_reflect::prelude::*;

use crate::{blending::Blend, lerping::Lerp, Animator, AnimatorBlending, Clip};

#[derive(Default)]
struct AnimatorDescriptorInner {
    dynamic_properties: Vec<(String, TypeId, isize, u32)>,
}

// Defines how to animate the reflected type
pub struct AnimatorDescriptor<T> {
    inner: Option<AnimatorDescriptorInner>,
    animator_blending: AnimatorBlending,
    _marker: PhantomData<T>,
}

impl<T> Default for AnimatorDescriptor<T> {
    fn default() -> Self {
        Self {
            inner: None,
            animator_blending: AnimatorBlending::default(),
            _marker: PhantomData,
        }
    }
}

fn shorten_name(n: &str) -> &str {
    n.rsplit("::").nth(0).unwrap_or(n)
}

// fn animate<T: Lerp + Blend + 'static>(
//     prop_name: &str,
//     prop_type: &TypeId,
//     prop_offset: &usize,
//     prop_mask: &u32,
// ) {
//     if *prop_type == TypeId::of::<T>() {
//         if let Some(curves) = clip
//             .get(prop_name)
//             .map(|curve_untyped| curve_untyped.downcast_ref::<T>())
//             .flatten()
//         {
//             for (entity_index, (curve_index, curve)) in curves.iter() {
//                 let entity_index = entities_map[entity_index as usize] as usize;
//                 if let Some(ref mut component) = components[entity_index] {
//                     let kr = &mut keyframes[*curve_index];
//                     let (k, v) = curve.sample_indexed(*kr, time);
//                     *kr = k;

//                     let value = unsafe {
//                         &mut *(((&mut *component) as *mut _ as *mut u8).offset(*prop_offset)
//                             as *mut T)
//                     };

//                     value.blend(entity_index, *prop_mask, &mut blend_group, v, w);
//                 }
//             }
//         }
//         true
//     } else {
//         false
//     }
// }

pub fn animate_system<T: Struct + Component>(
    mut descriptor: Local<AnimatorDescriptor<T>>,
    clips: Res<Assets<Clip>>,
    animators_query: Query<&Animator>,
    // assets: Option<ResMut<Assets<T>>>,
    // handles_query: Query<&mut Handle<T>>,
    components_query: Query<&mut T>,
) {
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
                        .map(|entity| components_query.get_unsafe(entity).ok())
                        .flatten(),
                );
            }
        }

        if let Some(c) = components.iter().find_map(|c| c.as_ref()) {
            if descriptor.inner.is_none() {
                let mut inner = AnimatorDescriptorInner::default();

                // Lazy initialize the descriptor
                let short_name = shorten_name(c.type_name());
                let origin = (&**c) as *const _ as *const u8;
                for (i, value) in c.iter_fields().enumerate() {
                    let mut n = short_name.to_string();
                    n.push('.');
                    n.push_str(c.name_at(i).unwrap());
                    let ty = value.type_id();
                    // SAFETY: Is be less than size_of::<T>() so it won't be crazy high
                    let offset =
                        unsafe { (value.any() as *const _ as *const u8).offset_from(origin) };

                    inner
                        .dynamic_properties
                        .push((n, ty, offset, (1 << i) as u32));
                }

                descriptor.inner = Some(inner);
            }
        } else {
            // No components found skip
            continue;
        }

        let descriptor = &mut *descriptor;
        let descriptor_inner = descriptor.inner.as_ref().unwrap();

        let mut blend_group = descriptor
            .animator_blending
            .begin_blending(components.len());

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

                for (prop_name, prop_type, prop_offset, prop_mask) in
                    &descriptor_inner.dynamic_properties
                {
                    if *prop_type == TypeId::of::<Vec3>() {
                        if let Some(curves) = clip
                            .get(prop_name)
                            .map(|curve_untyped| curve_untyped.downcast_ref::<Vec3>())
                            .flatten()
                        {
                            for (entity_index, (curve_index, curve)) in curves.iter() {
                                let entity_index = entities_map[entity_index as usize] as usize;
                                if let Some(ref mut component) = components[entity_index] {
                                    let kr = &mut keyframes[*curve_index];
                                    let (k, v) = curve.sample_indexed(*kr, time);
                                    *kr = k;

                                    let value = unsafe {
                                        &mut *(((&mut **component) as *mut _ as *mut u8)
                                            .offset(*prop_offset)
                                            as *mut Vec3)
                                    };

                                    value.blend(entity_index, *prop_mask, &mut blend_group, v, w);
                                }
                            }
                        }
                        continue;
                    }

                    if *prop_type == TypeId::of::<Quat>() {
                        if let Some(curves) = clip
                            .get(prop_name)
                            .map(|curve_untyped| curve_untyped.downcast_ref::<Quat>())
                            .flatten()
                        {
                            for (entity_index, (curve_index, curve)) in curves.iter() {
                                let entity_index = entities_map[entity_index as usize] as usize;
                                if let Some(ref mut component) = components[entity_index] {
                                    let kr = &mut keyframes[*curve_index];
                                    let (k, v) = curve.sample_indexed(*kr, time);
                                    *kr = k;

                                    let value = unsafe {
                                        &mut *(((&mut **component) as *mut _ as *mut u8)
                                            .offset(*prop_offset)
                                            as *mut Quat)
                                    };

                                    value.blend(entity_index, *prop_mask, &mut blend_group, v, w);
                                }
                            }
                        }
                        continue;
                    }
                }
            }
        }
    }

    // std::mem::drop(__guard);
}
