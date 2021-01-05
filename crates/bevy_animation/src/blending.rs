use bevy_asset::{Asset, Handle, HandleUntyped};
use bevy_math::prelude::*;
use bevy_render::color::Color;
use fnv::FnvBuildHasher;
use std::{collections::HashMap, mem::size_of};

use crate::lerping::Lerp;

// pub struct Bit<'a>(&'a mut Mask, Mask);

// impl<'a> Bit<'a> {
//     #[inline(always)]
//     pub fn set(&mut self) -> bool {
//         if (*self.0 & self.1) != 0 {
//             *self.0 |= self.1;
//             true
//         } else {
//             false
//         }
//     }
// }

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
struct Ptr(*const u8);

// SAFETY: The underlying pointer will never will be dereferenced,
// it's only use as an numerical value globally unique any attribute
unsafe impl Send for Ptr {}
unsafe impl Sync for Ptr {}

/// Mask size used to blend properties, each bit corresponds to a property
pub type Mask = u32;

/// Number of animated properties a type can hold
pub const MASK_LIMIT: usize = size_of::<Mask>() * 8;

#[derive(Default, Debug)]
pub struct AnimatorBlending {
    bits: Vec<Mask>,
    // ? NOTE: HashMap is used here to reduce memory waste, it's slower but other wise a lot of memory won't be used
    // TODO: Change to the hashbrown::raw::RawTable to use the pointers values as hashes it self
    /// Used for contest blend type
    weights: HashMap<Ptr, f32, FnvBuildHasher>,
}

impl AnimatorBlending {
    #[inline(always)]
    pub fn begin_blending(&mut self, entities: usize) -> AnimatorBlendGroup {
        self.bits.clear();
        self.bits.resize(entities, 0);
        self.weights.clear();
        AnimatorBlendGroup { blending: self }
    }
}

pub struct AnimatorBlendGroup<'a> {
    blending: &'a mut AnimatorBlending,
}

impl<'a> AnimatorBlendGroup<'a> {
    /// Blend using lerping
    #[inline(always)]
    pub fn blend_lerp<T: Lerp>(
        &mut self,
        entity_index: usize,
        bit_mask: Mask,
        attribute: &mut T,
        value: T,
        weight: f32,
    ) {
        let b = &mut self.blending.bits[entity_index];
        if *b & bit_mask != 0 {
            *attribute = Lerp::lerp(&*attribute, &value, weight);
        } else {
            *attribute = value;
        }
        *b |= bit_mask;
    }

    /// Contest blending, only the value with the highest weight value will remain
    #[inline(always)]
    pub fn blend_contest<T>(&mut self, attribute: &mut T, value: T, weight: f32) {
        let ptr = Ptr(attribute as *const _ as *const u8);
        let w = self.blending.weights.entry(ptr).or_insert(0.0);
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
///     fn blend(
///         &mut self,
///         entity_index: usize,
///         bit_mask: Mask,
///         blend_group: &mut AnimatorBlendGroup,
///         value: Self,
///         weight: f32,
///     ){
///         blend_group.blend_contest(self, value, weight);
///         // or (MyType needs to implement `Lerp`)
///         // blend_group.blend_lerp(entity_index, bit_mask, self, value, weight);
///     }
/// }
/// ```
pub trait Blend {
    fn blend(
        &mut self,
        entity_index: usize,
        bit_mask: Mask,
        blend_group: &mut AnimatorBlendGroup,
        value: Self,
        weight: f32,
    );
}

macro_rules! lerp {
    ($t:ty) => {
        impl Blend for $t {
            #[inline(always)]
            fn blend(
                &mut self,
                entity_index: usize,
                bit_mask: Mask,
                blend_group: &mut AnimatorBlendGroup,
                value: Self,
                weight: f32,
            ) {
                blend_group.blend_lerp(entity_index, bit_mask, self, value, weight);
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
    fn blend(
        &mut self,
        _: usize,
        _: Mask,
        blend_group: &mut AnimatorBlendGroup,
        value: Self,
        weight: f32,
    ) {
        blend_group.blend_contest(self, value, weight);
    }
}

impl Blend for HandleUntyped {
    #[inline(always)]
    fn blend(
        &mut self,
        _: usize,
        _: Mask,
        blend_group: &mut AnimatorBlendGroup,
        value: Self,
        weight: f32,
    ) {
        blend_group.blend_contest(self, value, weight);
    }
}

impl<T: Blend> Blend for Option<T> {
    fn blend(
        &mut self,
        entity_index: usize,
        bit_mask: Mask,
        blend_group: &mut AnimatorBlendGroup,
        value: Self,
        weight: f32,
    ) {
        let ptr = Ptr(self as *const _ as *const u8);

        match (self.is_some(), value.is_some()) {
            (true, true) => {
                // Blend by lerp but also add the entry for the conext blending
                self.as_mut().unwrap().blend(
                    entity_index,
                    bit_mask,
                    blend_group,
                    value.unwrap(),
                    weight,
                );

                // Make sure to also add an entry for contest blent to work
                let w = blend_group.blending.weights.entry(ptr).or_insert(0.0);
                if weight > *w {
                    *w = weight;
                }
            }
            (false, true) | (true, false) | (false, false) => {
                // Blend by context but also add an entry for the next blend_lerp if needed
                blend_group.blending.bits[entity_index] |= bit_mask;
                blend_group.blend_contest(self, value, weight);
            }
        }
    }
}

// TODO: std::ops::Range
