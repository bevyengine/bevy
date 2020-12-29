//! Generic animation for any type that implements the `Reflect` trait

use std::{any::type_name, any::TypeId, marker::PhantomData, num::NonZeroUsize};

use bevy_asset::prelude::*;
use bevy_ecs::prelude::*;
use bevy_math::prelude::*;
use bevy_reflect::prelude::*;
use tracing::warn;

use crate::{
    blending::AnimatorBlendGroup, blending::Blend, help::shorten_name, lerping::Lerp, Animator,
    AnimatorBlending, Clip,
};

// TODO: How to deal with generic types like Handle<T> or Option<T>
// TODO: Animate assets
// TODO: Make sure properties are within component size bounds
// TODO: Accept also `TupleStruct` and `Value`
// TODO: Expand types like Vec2, Vec3, Vec4 and Color
// ! FIXME: Vec2, Vec3, Vec4 doesn't implement bevy_reflect::Struct so they can't be auto expanded
// TODO: Implement safe property path like `Transforms::props().translation().x()` using only const, (Property validation is working)

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
pub struct AnimatorPropertyRegistry {
    ver: NonZeroUsize,
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
                // (TypeId::of::<Color>(), animate::<Color>),
                // (TypeId::of::<HandleUntyped>(), animate::<HandleUntyped>),
                // (TypeId::of::<Handle<Mesh>>(), animate::<Handle<Mesh>>),
                // TODO: How to handle generic types like Handle<T> or Option<T>
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

impl<T> AnimatorDescriptor<T> {
    #[inline(always)]
    pub fn properties(&self) -> impl Iterator<Item = (&str, TypeId)> {
        self.inner.as_ref().into_iter().flat_map(|inner| {
            inner
                .dynamic_properties
                .iter()
                .map(|(name, type_id, _, _, _)| (name.as_str(), *type_id))
        })
    }
}

impl<T: Struct> AnimatorDescriptor<T> {
    pub fn from_component(component: &T) -> Self {
        let mut inner = AnimatorDescriptorInner {
            ver: 0,
            dynamic_properties: vec![],
        };

        // Lazy initialize the descriptor
        let short_name = shorten_name(component.type_name());
        let origin = component.any() as *const _ as *const u8;
        for (i, value) in component.iter_fields().enumerate() {
            let component_name = component.name_at(i).unwrap();
            // NOTE: Pre calculate all the space needed to make in a single allocation (untested)
            let mut prop_name = String::with_capacity(short_name.len() + 1 + component_name.len());
            prop_name.push_str(&short_name);
            prop_name.push('.');
            prop_name.push_str(component_name);

            let prop_type = value.type_id();

            // SAFETY: Is be less than size_of::<T>() so it won't be crazy high
            let prop_offset = unsafe { (value.any() as *const _ as *const u8).offset_from(origin) };

            let prop_mask = (1 << i) as u32;

            inner.dynamic_properties.push((
                prop_name,
                prop_type,
                usize::MAX,
                prop_offset,
                prop_mask,
            ));
        }

        Self {
            inner: Some(inner),
            ..Default::default()
        }
    }
}

pub fn animate_component_system<T: Struct + Send + Sync + 'static>(
    registry: Res<AnimatorPropertyRegistry>,
    mut descriptor: ResMut<AnimatorDescriptor<T>>,
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

        let descriptor = &mut *descriptor;
        let descriptor_inner = descriptor.inner.as_mut().unwrap();

        if descriptor_inner.ver != registry.ver.get() {
            // Lookup missing animate functions because `AnimatorRegistry` changed
            for (prop_name, prop_type, prop_index, _, _) in &mut descriptor_inner.dynamic_properties
            {
                let mut index = *prop_index;
                if index == usize::MAX {
                    index = registry.index_of(*prop_type);
                    if index == usize::MAX {
                        warn!("missing animation function for `{}`", &prop_name);
                    }
                    *prop_index = index;
                }
            }
            descriptor_inner.ver = registry.ver.get();
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
