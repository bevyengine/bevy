//! Generic animation for any type that implements the `Reflect` trait
//!
//! This will never be as fast as the code gen approach but it will give
//! a nice way of provide animation to external types or even to types derived
//! from scrips!

use std::{any::TypeId, marker::PhantomData};

use bevy_asset::prelude::*;
use bevy_ecs::prelude::*;
use bevy_math::prelude::*;
use bevy_reflect::prelude::*;
use tracing::warn;

use crate::{
    blending::AnimatorBlendGroup, blending::Blend, lerping::Lerp, Animator, AnimatorBlending, Clip,
};

type AnimateFn = fn(
    components: &mut dyn Components,
    entities_map: &[u16],
    blend_group: &mut AnimatorBlendGroup,
    keyframes: &mut [u16],
    clip: &Clip,
    time: f32,
    w: f32,
    prop_name: &str,
    prop_offset: isize,
    prop_mask: u32,
);

/// Register how to animate custom types
pub struct AnimatorRegistry {
    ver: usize,
    animate_functions: Vec<(TypeId, AnimateFn)>,
}

impl Default for AnimatorRegistry {
    fn default() -> Self {
        Self {
            ver: 0,
            // Basic types
            animate_functions: vec![
                (TypeId::of::<bool>(), animate::<bool>),
                (TypeId::of::<f32>(), animate::<f32>),
                (TypeId::of::<Vec2>(), animate::<Vec2>),
                (TypeId::of::<Vec3>(), animate::<Vec3>),
                (TypeId::of::<Vec4>(), animate::<Vec4>),
                (TypeId::of::<Quat>(), animate::<Quat>),
                // (TypeId::of::<Color>(), animate::<Color>),
                // (TypeId::of::<HandleUntyped>(), animate::<HandleUntyped>),
                // (TypeId::of::<Handle<Mesh>>(), animate::<Handle<Mesh>>),
                // TODO: How to handle generic types like Handle<T> or Option<T>
            ],
        }
    }
}

impl AnimatorRegistry {
    pub fn registry<T: Lerp + Blend + Clone + 'static>(&mut self) {
        let ty = TypeId::of::<T>();
        if self
            .animate_functions
            .iter()
            .position(|(other, _)| *other == ty)
            .is_some()
        {
            panic!("type '{}' already registered");
        }

        self.ver = self.ver.wrapping_add(1);
        self.animate_functions.push((ty, animate::<T>))
    }

    /// Returns `usize::MAX` if the type wasn't registered,
    ///
    /// **NOTE** The index ranges from `0` to `usize::MAX - 1`,
    /// which makes usize::MAX an invalid index thats why
    /// this function doesn't return `Option<usize>`
    pub fn index_of(&self, ty: TypeId) -> usize {
        self.animate_functions
            .iter()
            .position(|(other, _)| *other == ty)
            .unwrap_or(usize::MAX)
    }
}

/// Helper trait to fetch components pointers
trait Components {
    fn get_mut(&mut self, index: usize) -> Option<*mut u8>;
}

impl<'a, 's, T: Send + Sync + 'static> Components for &'s mut [Option<Mut<'a, T>>] {
    #[inline(always)]
    fn get_mut(&mut self, index: usize) -> Option<*mut u8> {
        // NOTE: Will trigger the `Changed` event to the right types
        self[index].as_mut().map(|c| &mut **c as *mut T as *mut u8)
    }
}

fn animate<T: Lerp + Blend + Clone + 'static>(
    components: &mut dyn Components,
    entities_map: &[u16],
    blend_group: &mut AnimatorBlendGroup,
    keyframes: &mut [u16],
    clip: &Clip,
    time: f32,
    w: f32,
    prop_name: &str,
    prop_offset: isize,
    prop_mask: u32,
) {
    if let Some(curves) = clip
        .get(prop_name)
        .map(|curve_untyped| curve_untyped.downcast_ref::<T>())
        .flatten()
    {
        for (entity_index, (curve_index, curve)) in curves.iter() {
            let entity_index = entities_map[entity_index as usize] as usize;
            if let Some(ref mut component) = components.get_mut(entity_index) {
                let kr = &mut keyframes[*curve_index];
                let (k, v) = curve.sample_indexed(*kr, time);
                *kr = k;

                // SAFETY: prop_offset is
                let value = unsafe { &mut *(component.offset(prop_offset) as *mut T) };
                value.blend(entity_index, prop_mask, blend_group, v, w);
            }
        }
    }
}

struct AnimatorDescriptorInner {
    ver: usize,
    dynamic_properties: Vec<(String, TypeId, usize, isize, u32)>,
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

pub fn animate_system<T: Struct + Send + Sync + 'static>(
    registry: Res<AnimatorRegistry>,
    mut descriptor: Local<AnimatorDescriptor<T>>,
    clips: Res<Assets<Clip>>,
    animators_query: Query<&Animator>,
    // assets: Option<ResMut<Assets<T>>>,
    // handles_query: Query<&mut Handle<T>>,
    components_query: Query<&mut T>,
) {
    let __span = tracing::info_span!("animator_transform_update_system");
    let __guard = __span.enter();

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
                let mut inner = AnimatorDescriptorInner {
                    ver: registry.ver,
                    dynamic_properties: vec![],
                };

                // Lazy initialize the descriptor
                let short_name = shorten_name(c.type_name());
                let origin = (&**c) as *const _ as *const u8;
                for (i, value) in c.iter_fields().enumerate() {
                    let mut name = short_name.to_string();
                    name.push('.');
                    name.push_str(c.name_at(i).unwrap());
                    let ty = value.type_id();

                    // Find the animate function for the given type
                    let index = registry.index_of(ty);
                    if index == usize::MAX {
                        warn!("missing animation function for `{}`", &name);
                    }

                    // SAFETY: Is be less than size_of::<T>() so it won't be crazy high
                    let offset =
                        unsafe { (value.any() as *const _ as *const u8).offset_from(origin) };

                    inner
                        .dynamic_properties
                        .push((name, ty, index, offset, (1 << i) as u32));

                    // TODO: Expand type if possible
                }

                descriptor.inner = Some(inner);
            }
        } else {
            // No components found skip
            continue;
        }

        let descriptor = &mut *descriptor;
        let descriptor_inner = descriptor.inner.as_mut().unwrap();

        if descriptor_inner.ver != registry.ver {
            // TODO: Lookup missing animate functions because `AnimatorRegistry` changed
        }

        let mut blend_group = descriptor
            .animator_blending
            .begin_blending(components.len());

        // Alias `T`
        let mut components: Box<dyn Components> = Box::new(&mut components[..]);

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

                for (prop_name, _, prop_index, prop_offset, prop_mask) in
                    &descriptor_inner.dynamic_properties
                {
                    let i = *prop_index;
                    if i == usize::MAX {
                        // Missing animation function
                        continue;
                    }

                    (registry.animate_functions[i].1)(
                        &mut *components,
                        entities_map,
                        &mut blend_group,
                        keyframes,
                        clip,
                        time,
                        w,
                        prop_name,
                        *prop_offset,
                        *prop_mask,
                    )
                }
            }
        }
    }

    std::mem::drop(__guard);
}
