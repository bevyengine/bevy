use bevy_asset::{Asset, Handle, HandleUntyped};
use bevy_math::prelude::*;
use bevy_render::color::Color;
use fnv::FnvBuildHasher;
use std::collections::{HashMap, HashSet};

use crate::lerping::Lerp;

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
struct Ptr(*const u8);

// SAFETY: Store pointers to each attribute to be updated, a clip can't have two pointers
// with the same value. Each clip per Animator will be updated sequentially
unsafe impl Send for Ptr {}
unsafe impl Sync for Ptr {}

#[derive(Default, Debug)]
pub struct AnimatorBlending {
    // ? NOTE: FnvHasher performed better over the PtrHasher (ptr value as hash) and AHash
    blend: HashSet<Ptr, FnvBuildHasher>,
    /// Used for contest blend type
    blend_memory: HashMap<Ptr, f32, FnvBuildHasher>,
}

impl AnimatorBlending {
    #[inline(always)]
    pub fn begin_blending(&mut self) -> AnimatorBlendGroup {
        self.blend.clear();
        self.blend_memory.clear();
        AnimatorBlendGroup { blending: self }
    }
}

pub struct AnimatorBlendGroup<'a> {
    blending: &'a mut AnimatorBlending,
}

impl<'a> AnimatorBlendGroup<'a> {
    /// Blend using lerping
    pub fn blend_lerp<T: Lerp>(&mut self, attribute: &mut T, value: T, weight: f32) {
        let ptr = Ptr(attribute as *const _ as *const u8);
        if self.blending.blend.contains(&ptr) {
            *attribute = Lerp::lerp(&*attribute, &value, weight);
        } else {
            self.blending.blend.insert(ptr);
            *attribute = value;
        }
    }

    /// Contest blending, only the value with the highest weight value will remain
    pub fn blend_contest<T>(&mut self, attribute: &mut T, value: T, weight: f32) {
        let ptr = Ptr(attribute as *const _ as *const u8);
        let w = self.blending.blend_memory.entry(ptr).or_insert(0.0);
        if weight > *w {
            *w = weight;
            *attribute = value;
        }
    }
}

/// Instructs on how blend multiple layers togethers based on their value,
/// normally there is two types of blend, lerp and contest blending, the first
/// is used by floats, vectors and quaternions, while the second keeps only the value
/// with higher weight and is used for booleans and asset handles.
///
/// Here's how to correctly implement it:
/// ```rust
/// struct MyType;
///
/// impl Blend for MyType {
///     #[inline(always)]
///     fn blend(&mut self, blend_group: &mut AnimatorBlendGroup, value: Self, weight: f32) {
///         blend_group.blend_contest(self, value, weight);
///         // or
///         // blend_group.blend_lerp(self, value, weight);
///     }
/// }
/// ```
pub trait Blend {
    fn blend(&mut self, blend_group: &mut AnimatorBlendGroup, value: Self, weight: f32);
}

macro_rules! lerp {
    ($t:ty) => {
        impl Blend for $t {
            #[inline(always)]
            fn blend(&mut self, blend_group: &mut AnimatorBlendGroup, value: Self, weight: f32) {
                blend_group.blend_lerp(self, value, weight);
            }
        }
    };
}

lerp!(bool);
lerp!(f32);
lerp!(Vec2);
lerp!(Vec3);
lerp!(Vec4);
lerp!(Quat);
lerp!(Color);

impl<T: Asset + 'static> Blend for Handle<T> {
    #[inline(always)]
    fn blend(&mut self, blend_group: &mut AnimatorBlendGroup, value: Self, weight: f32) {
        blend_group.blend_contest(self, value, weight);
    }
}

impl Blend for HandleUntyped {
    #[inline(always)]
    fn blend(&mut self, blend_group: &mut AnimatorBlendGroup, value: Self, weight: f32) {
        blend_group.blend_contest(self, value, weight);
    }
}
