//! Generic animation for any type that implements the `Reflect` trait

use std::{any::type_name, any::TypeId, marker::PhantomData, mem::size_of, num::NonZeroUsize};

use bevy_asset::{prelude::*, Asset};
use bevy_ecs::prelude::*;
use bevy_math::prelude::*;
use bevy_reflect::prelude::*;
use bevy_render::prelude::Color;
use tracing::warn;

use crate::{
    animator::{Animator, Clip},
    blending::{AnimatorBlendGroup, AnimatorBlending, Blend, Mask, MASK_LIMIT},
    help::shorten_name,
    lerping::Lerp,
};

// ? NOTE: Generic types like `Option<T>` must be specialized and registered with `register_animated_property_type`
// ? in order to be animated; `Handle<T>` and `Option<Handle<T>>` are registered automatically upon registering a
// ? animated asset `T` with `register_animated_asset`

// TODO: Add tracing spans
// TODO: Accept also `TupleStruct` and `Value`
// TODO: Expand types like Vec2, Vec3, Vec4 and Color
// ! FIXME: Vec2, Vec3, Vec4 doesn't implement bevy_reflect::Struct so they can't be auto expanded

type AnimateFn = unsafe fn(
    get_mut: &mut dyn FnMut(usize) -> Option<*mut u8>,
    entities_map: &[u16],
    blend_group: &mut AnimatorBlendGroup,
    keyframes: &mut [u16],
    clip: &Clip,
    time: f32,
    w: f32,
    prop: &Property,
);

/// Register how to animate custom types
pub struct AnimatorPropertyRegistry {
    ver: NonZeroUsize,
    // TODO: Use a hashmap?
    animate_functions: Vec<(TypeId, AnimateFn)>,
}

impl Default for AnimatorPropertyRegistry {
    fn default() -> Self {
        Self {
            ver: NonZeroUsize::new(1).unwrap(),
            // Basic types
            animate_functions: vec![
                (TypeId::of::<bool>(), animate::<bool>),
                (TypeId::of::<f32>(), animate::<f32>),
                (TypeId::of::<Vec2>(), animate::<Vec2>),
                (TypeId::of::<Vec3>(), animate::<Vec3>),
                (TypeId::of::<Vec4>(), animate::<Vec4>),
                (TypeId::of::<Quat>(), animate::<Quat>),
                (TypeId::of::<Color>(), animate::<Color>),
            ],
        }
    }
}

impl AnimatorPropertyRegistry {
    pub fn register<T: Lerp + Blend + Clone + 'static>(&mut self) {
        let ty = TypeId::of::<T>();
        if self
            .animate_functions
            .iter()
            .position(|(other, _)| *other == ty)
            .is_some()
        {
            panic!("type '{}' already registered", type_name::<T>());
        }

        let ver = self.ver.get().wrapping_add(1);
        self.ver = if ver == 0 {
            NonZeroUsize::new(1)
        } else {
            NonZeroUsize::new(ver)
        }
        .unwrap();

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

/// Use need to guarantee that `Property` is valid and is owned by same type
/// of object returned by `get_mut` function
unsafe fn animate<T>(
    get_mut: &mut dyn FnMut(usize) -> Option<*mut u8>,
    entities_map: &[u16],
    blend_group: &mut AnimatorBlendGroup,
    keyframes: &mut [u16],
    clip: &Clip,
    time: f32,
    w: f32,
    prop: &Property,
) where
    T: Lerp + Blend + Clone + 'static,
{
    assert_eq!(TypeId::of::<T>(), prop.type_id);

    if let Some(curves) = clip
        .get(&prop.path)
        .map(|curve_untyped| curve_untyped.downcast_ref::<T>())
        .flatten()
    {
        for (entity_index, (curve_index, curve)) in curves.iter() {
            let entity_index = entities_map[entity_index as usize] as usize;
            if let Some(component) = (get_mut)(entity_index) {
                let kr = &mut keyframes[*curve_index];
                let (k, v) = curve.sample_indexed(*kr, time);
                *kr = k;

                // Unsafe portion
                let value = &mut *(component.offset(prop.offset) as *mut T);
                value.blend(entity_index, prop.mask, blend_group, v, w);
            }
        }
    }
}

struct Property {
    path: String,
    type_id: TypeId,
    index: usize,
    offset: isize,
    mask: Mask,
}

// Defines how to animate an type
pub(crate) struct AnimatorDescriptor<T> {
    ver: usize,
    short_name: String,
    dynamic_properties: Vec<Property>,
    animator_blending: AnimatorBlending,
    _marker: PhantomData<T>,
}

impl<T> Default for AnimatorDescriptor<T> {
    fn default() -> Self {
        Self {
            ver: 0,
            short_name: String::new(),
            dynamic_properties: vec![],
            animator_blending: AnimatorBlending::default(),
            _marker: PhantomData,
        }
    }
}

impl<T> AnimatorDescriptor<T> {
    #[inline(always)]
    pub fn properties(&self) -> impl Iterator<Item = (&str, TypeId)> {
        self.dynamic_properties
            .iter()
            .map(|Property { path, type_id, .. }| (path.as_str(), *type_id))
    }
}

impl<T: Struct> AnimatorDescriptor<T> {
    pub fn from_component(component: &T) -> Self {
        let base_path = shorten_name(component.type_name());
        let mut descriptor = Self::build(component, &base_path);
        descriptor.short_name = base_path;
        descriptor
    }

    pub fn from_asset(asset: &T) -> Self {
        let mut base_path = shorten_name(asset.type_name());
        base_path.insert_str(0, "Handle<");
        base_path.push('>');
        let mut descriptor = Self::build(asset, &base_path);
        descriptor.short_name = base_path;
        descriptor
    }

    fn build(instance: &T, base_path: &str) -> Self {
        let mut dynamic_properties = vec![];

        // Lazy initialize the descriptor
        let origin = instance.any() as *const _ as *const u8;
        for (i, value) in instance.iter_fields().enumerate() {
            if i >= MASK_LIMIT {
                panic!(
                    "`{}` reached the limit of {} animated properties",
                    base_path, MASK_LIMIT
                );
            }

            let field_name = instance.name_at(i).unwrap();
            // NOTE: Pre calculate all the space needed to make in a single allocation (untested)
            let mut path = String::with_capacity(base_path.len() + 1 + field_name.len());
            path.push_str(&base_path);
            path.push('.');
            path.push_str(field_name);

            let type_id = value.type_id();

            // SAFETY: Is less than size_of::<T>() witch is expected to be very low compared to isize::MAX;
            // the resulted offset is also guaranteed to be inside the component
            let offset = unsafe {
                let ptr = value.any() as *const _ as *const u8;
                assert!(
                    ptr >= origin && ptr < origin.add(size_of::<T>()),
                    "property '{}' offset isn't within struct bounds [0; {}]",
                    path,
                    size_of::<T>()
                );
                ptr.offset_from(origin)
            };

            let mask = 1 << i;
            let index = usize::MAX;

            dynamic_properties.push(Property {
                path,
                type_id,
                index,
                offset,
                mask,
            });
        }

        Self {
            dynamic_properties,
            ..Default::default()
        }
    }
}

///////////////////////////////////////////////////////////////////////////////

pub(crate) fn animate_component_system<T: Component>(
    registry: Res<AnimatorPropertyRegistry>,
    mut descriptor: ResMut<AnimatorDescriptor<T>>,
    clips: Res<Assets<Clip>>,
    animators_query: Query<&Animator>,
    components_query: Query<&mut T>,
) {
    let __span = tracing::info_span!("animator_transform_update_system");
    let __guard = __span.enter();

    // TODO: Find a way to reuse these arrays in between system executions
    let mut cached_components = vec![];

    let descriptor = &mut *descriptor;
    if descriptor.ver != registry.ver.get() {
        // Lookup missing animate functions because `AnimatorRegistry` changed
        for prop in &mut descriptor.dynamic_properties {
            let mut i = prop.index;
            if i == usize::MAX {
                i = registry.index_of(prop.type_id);
                if i == usize::MAX {
                    warn!("missing animation function for `{}`", &prop.path);
                }
                prop.index = i;
            }
        }
        descriptor.ver = registry.ver.get();
    }

    for animator in animators_query.iter() {
        cached_components.clear();

        // ? NOTE: Lazy get each component is worse than just fetching everything at once
        // Pre-fetch all transforms to avoid calling get_mut multiple times
        // SAFETY: each component will be updated one at the time and this function
        // currently has the mutability over the Transform type, so no race conditions
        // are possible
        unsafe {
            for entity in animator.entities() {
                cached_components.push(
                    entity
                        .map(|entity| components_query.get_unsafe(entity).ok())
                        .flatten(),
                );
            }
        }

        let mut blend_group = descriptor
            .animator_blending
            .begin_blending(cached_components.len());

        let mut get_mut_components = |index: usize| -> Option<*mut u8> {
            // NOTE: Will trigger the `Changed` event to the right types
            cached_components[index]
                .as_mut()
                .map(|c| &mut **c as *mut T as *mut u8)
        };

        for (_, layer, clip_handle, entities_map) in animator.animate() {
            let w = layer.weight;
            if w < 1.0e-8 {
                continue;
            }

            // NOTE: It's most likely that two different layers play different clips
            if let Some(clip) = clips.get(clip_handle) {
                let time = layer.time;

                // SAFETY: Never a different thread will modify or access the same index as this one;
                // Plus as a nice and crazy feature each property is grouped by name into their own cache line
                // buckets, this way no cache line will be accessed by the same thread unless the same property
                // is accessed by two different systems, which is possible but weird and will hit the performance a bit
                let keyframes = unsafe { layer.keyframes_unsafe() };

                for prop in &descriptor.dynamic_properties {
                    let i = prop.index;
                    if i == usize::MAX {
                        // Missing animation function
                        continue;
                    }

                    // SAFETY: Components type matches with the ones returned by `get_mut_components`
                    // and each property is generates internally by `AnimatorDescriptor<T>` which was
                    // it's own checks to guarantee that they are valid
                    unsafe {
                        (registry.animate_functions[i].1)(
                            &mut get_mut_components,
                            entities_map,
                            &mut blend_group,
                            keyframes,
                            clip,
                            time,
                            w,
                            &prop,
                        )
                    }
                }
            }
        }
    }

    std::mem::drop(__guard);
}

///////////////////////////////////////////////////////////////////////////////

pub(crate) fn animate_asset_system<T: Asset>(
    registry: Res<AnimatorPropertyRegistry>,
    mut descriptor: ResMut<AnimatorDescriptor<T>>,
    clips: Res<Assets<Clip>>,
    animators_query: Query<&Animator>,
    mut assets: ResMut<Assets<T>>,
    components_query: Query<&mut Handle<T>>,
) {
    // let __span = tracing::info_span!("animator_transform_update_system");
    // let __guard = __span.enter();

    // TODO: Find a way to reuse these arrays in between system executions
    let mut cached_components = vec![];
    // NOTE: Cache assets to avoid multiple hash table queries
    let mut cached_assets = vec![];
    // NOTE: Clips are cached here because we need to loop for the animators layers twice
    let mut cached_clips = vec![];

    let descriptor = &mut *descriptor;
    if descriptor.ver != registry.ver.get() {
        // Lookup missing animate functions because `AnimatorRegistry` changed
        for prop in &mut descriptor.dynamic_properties {
            let mut i = prop.index;
            if i == usize::MAX {
                i = registry.index_of(prop.type_id);
                if i == usize::MAX {
                    warn!("missing animation function for `{}`", &prop.path);
                }
                prop.index = i;
            }
        }
        descriptor.ver = registry.ver.get();
    }

    for animator in animators_query.iter() {
        cached_clips.clear();
        cached_clips.resize_with(animator.clips().len(), || None);

        cached_components.clear();

        // ? NOTE: Lazy get each component is worse than just fetching everything at once
        // Pre-fetch all transforms to avoid calling get_mut multiple times
        // SAFETY: each component will be updated one at the time and this function
        // currently has the mutability over the Transform type, so no race conditions
        // are possible
        unsafe {
            for entity in animator.entities() {
                cached_components.push(
                    entity
                        .map(|entity| components_query.get_unsafe(entity).ok())
                        .flatten(),
                );
            }
        }

        cached_assets.clear();
        cached_assets.resize_with(animator.entities().len(), || None);

        let mut blend_group = descriptor
            .animator_blending
            .begin_blending(cached_components.len());

        // Animate asset handle

        for (_, layer, clip_handle, entities_map) in animator.animate() {
            let w = layer.weight;
            if w < 1.0e-8 {
                continue;
            }

            // NOTE: `animator.animate()` only yield layers with valid clip indexes
            let clip = &mut cached_clips[layer.clip];
            if clip.is_none() {
                *clip = clips.get(clip_handle);
            }

            if let Some(clip) = *clip {
                let time = layer.time;

                // SAFETY: Never a different thread will modify or access the same index as this one;
                // Plus as a nice and crazy feature each property is grouped by name into their own cache line
                // buckets, this way no cache line will be accessed by the same thread unless the same property
                // is accessed by two different systems, which is possible but weird and will hit the performance a bit
                let keyframes = unsafe { layer.keyframes_unsafe() };

                // Handle property is always the first one
                let prop_name = &descriptor.short_name;

                if descriptor.dynamic_properties.len() == MASK_LIMIT {
                    panic!(
                        "`{}` reached the limit of {} animated properties",
                        prop_name, MASK_LIMIT
                    );
                }
                let prop_mask = 1 << descriptor.dynamic_properties.len();

                // ? NOTE: Maybe a bit incontinent to have this `animate` function inlined here,
                // ? but it will make the code more safe and also faster
                if let Some(curves) = clip
                    .get(prop_name)
                    .map(|curve_untyped| curve_untyped.downcast_ref::<Handle<T>>())
                    .flatten()
                {
                    for (entity_index, (curve_index, curve)) in curves.iter() {
                        let entity_index = entities_map[entity_index as usize] as usize;
                        if let Some(ref mut component) = cached_components[entity_index] {
                            let kr = &mut keyframes[*curve_index];
                            let (k, v) = curve.sample_indexed(*kr, time);
                            *kr = k;

                            let value = &mut **component;
                            value.blend(entity_index, prop_mask, &mut blend_group, v, w);
                        }
                    }
                }
            }
        }

        // ? NOTE Only after all asset handles had been settled that, the assets properties can be animated
        // Animate asset properties

        if descriptor.dynamic_properties.len() == 0 {
            // This assets have no animated properties
            continue;
        }

        let mut get_mut_components = |index: usize| -> Option<*mut u8> {
            // NOTE: Will trigger the `Changed` event to the right assets

            let asset = &mut cached_assets[index];
            if asset.is_none() {
                *asset = cached_components[index]
                    .as_ref()
                    .and_then(|c| assets.get_mut(&**c))
                    .map(|a| a as *mut T as *mut u8)
            }

            *asset
        };

        // TODO: `animator.animate()` doesn't come for free, might want to cache some info
        for (_, layer, _, entities_map) in animator.animate() {
            let w = layer.weight;
            if w < 1.0e-8 {
                continue;
            }

            // NOTE: `animator.animate()` only yield layers with valid clip indexes
            if let Some(clip) = cached_clips[layer.clip] {
                let time = layer.time;

                // SAFETY: Never a different thread will modify or access the same index as this one;
                // Plus as a nice and crazy feature each property is grouped by name into their own cache line
                // buckets, this way no cache line will be accessed by the same thread unless the same property
                // is accessed by two different systems, which is possible but weird and will hit the performance a bit
                let keyframes = unsafe { layer.keyframes_unsafe() };

                for prop in &descriptor.dynamic_properties {
                    let i = prop.index;
                    if i == usize::MAX {
                        // Missing animation function
                        continue;
                    }

                    // SAFETY: Components type matches with the ones returned by `get_mut_components`
                    // and each property is generates internally by `AnimatorDescriptor<T>` which was
                    // it's own checks to guarantee that they are valid
                    unsafe {
                        (registry.animate_functions[i].1)(
                            &mut get_mut_components,
                            entities_map,
                            &mut blend_group,
                            keyframes,
                            clip,
                            time,
                            w,
                            &prop,
                        )
                    }
                }
            }
        }
    }

    // std::mem::drop(__guard);
}
