use std::mem::size_of;

use bevy_asset::{Asset, Handle, HandleUntyped};
use bevy_math::prelude::*;
use bevy_render::color::Color;

use crate::interpolate::{
    utils::{Scale2, Scale3},
    Lerp,
};

/// Mask size used to blend properties, each bit corresponds to a single property
pub type Mask = u32;

/// Number of animated properties a type can hold
pub const MASK_LIMIT: usize = size_of::<Mask>() * 8;

#[derive(Default, Debug)]
pub struct AnimatorBlending {
    bits: Vec<Mask>,
    weight_slots: usize,
    weights: Vec<f32>,
}

impl AnimatorBlending {
    #[inline]
    pub fn begin_blending(&mut self, entities: usize, weight_slots: usize) -> AnimatorBlendGroup {
        self.bits.clear();
        self.bits.resize(entities, 0);

        self.weight_slots = weight_slots;
        self.weights.clear();
        self.weights.resize(entities * weight_slots, f32::MIN);

        AnimatorBlendGroup { blending: self }
    }
}

pub struct AnimatorBlendGroup<'a> {
    blending: &'a mut AnimatorBlending,
}

impl<'a> AnimatorBlendGroup<'a> {
    #[inline]
    pub fn blend<T>(
        &mut self,
        entity_index: usize,
        bit_mask: Mask,
        attribute: &mut T,
        value: T,
        weight: f32,
        function: impl Fn(&T, &T, f32) -> T,
    ) {
        let b = &mut self.blending.bits[entity_index];
        if *b & bit_mask != 0 {
            *attribute = (function)(&*attribute, &value, weight);
        } else {
            *attribute = value;
        }
        *b |= bit_mask;
    }

    /// Contest blending, only the value with the highest weight value will remain
    #[inline]
    pub fn blend_contest<T>(
        &mut self,
        entity_index: usize,
        slot: usize,
        attribute: &mut T,
        value: T,
        weight: f32,
    ) {
        let w = &mut self.blending.weights[entity_index * slot];
        if weight > *w {
            *w = weight;
            *attribute = value;
        }
    }
}

///////////////////////////////////////////////////////////////////////////////

// ? NOTE: Implements the 'Additive 1' mode described here
// ? https://github.com/nfrechette/acl/blob/develop/docs/additive_clips.md

#[inline]
fn additive_blend_bool(a: &bool, b: &bool, _: f32) -> bool {
    // TODO: Review
    *a || *b
}

#[inline]
fn additive_blend_f32(a: &f32, b: &f32, w: f32) -> f32 {
    *a + (*b * w)
}

#[inline]
fn additive_blend_vec2(a: &Vec2, b: &Vec2, w: f32) -> Vec2 {
    *a + (*b * w)
}

#[inline]
fn additive_blend_vec3(a: &Vec3, b: &Vec3, w: f32) -> Vec3 {
    *a + (*b * w)
}

#[inline]
fn additive_blend_vec4(a: &Vec4, b: &Vec4, w: f32) -> Vec4 {
    *a + (*b * w)
}

#[inline]
fn additive_blend_quat(a: &Quat, b: &Quat, w: f32) -> Quat {
    // TODO: Review
    (*b * w) * *a
}

#[inline]
fn additive_blend_scale2(a: &Scale2, b: &Scale2, w: f32) -> Scale2 {
    Scale2((Vec2::one() + (b.0 * w)) * a.0)
}

#[inline]
fn additive_blend_scale3(a: &Scale3, b: &Scale3, w: f32) -> Scale3 {
    Scale3((Vec3::one() + (b.0 * w)) * a.0)
}

#[inline]
fn additive_blend_color(a: &Color, b: &Color, w: f32) -> Color {
    (Vec4::from(*a) + (Vec4::from(*b) * w)).into()
}

///////////////////////////////////////////////////////////////////////////////

/// Instructs on how blend multiple layers togethers based on their value,
/// normally there is two types of blend, lerp and contest blending, the first
/// is used by floats, vectors and quaternions, while the second keeps only the value
/// with higher weight and is used for booleans and asset handles.
///
/// Here's how to correctly implement it:
/// ```rust,ignore
/// struct MyType;
///
/// impl Blend for MyType {
///     #[inline]
///     fn blend(
///         &mut self,
///         entity_index: usize,
///         bit_mask: Mask,
///         blend_group: &mut AnimatorBlendGroup,
///         value: Self,
///         weight: f32,
///         additive: bool,
///     ){
///         blend_group.blend_contest(self, value, weight);
///         // or (MyType needs to implement `Lerp`)
///         // blend_group.blend(entity_index, bit_mask, self, value, weight, blend_function);
///     }
/// }
/// ```
pub trait Blend: Sized {
    fn requires_weight_slot() -> bool;

    fn blend(
        &mut self,
        entity_index: usize,
        bit_mask: Mask,
        slot: usize,
        blend_group: &mut AnimatorBlendGroup,
        value: Self,
        weight: f32,
        additive: bool,
    );
}

macro_rules! impl_blend {
    ($t:ty, $normal:path, $add:path) => {
        impl Blend for $t {
            fn requires_weight_slot() -> bool {
                false
            }

            #[inline]
            fn blend(
                &mut self,
                entity_index: usize,
                bit_mask: Mask,
                _: usize,
                blend_group: &mut AnimatorBlendGroup,
                value: Self,
                weight: f32,
                additive: bool,
            ) {
                if additive {
                    blend_group.blend(entity_index, bit_mask, self, value, weight, $add);
                } else {
                    blend_group.blend(entity_index, bit_mask, self, value, weight, $normal);
                }
            }
        }
    };
}

impl_blend!(bool, Lerp::lerp, additive_blend_bool);
impl_blend!(f32, Lerp::lerp, additive_blend_f32);
impl_blend!(Vec2, Lerp::lerp, additive_blend_vec2);
impl_blend!(Vec3, Lerp::lerp, additive_blend_vec3);
impl_blend!(Vec4, Lerp::lerp, additive_blend_vec4);
impl_blend!(Quat, Lerp::lerp, additive_blend_quat);
impl_blend!(Scale2, Lerp::lerp, additive_blend_scale2);
impl_blend!(Scale3, Lerp::lerp, additive_blend_scale3);
impl_blend!(Color, Lerp::lerp, additive_blend_color);

///////////////////////////////////////////////////////////////////////////////

impl<T: Asset + 'static> Blend for Handle<T> {
    fn requires_weight_slot() -> bool {
        true
    }

    #[inline]
    fn blend(
        &mut self,
        entity_index: usize,
        _: Mask,
        slot: usize,
        blend_group: &mut AnimatorBlendGroup,
        value: Self,
        weight: f32,
        _: bool,
    ) {
        blend_group.blend_contest(entity_index, slot, self, value, weight);
    }
}

impl Blend for HandleUntyped {
    fn requires_weight_slot() -> bool {
        true
    }

    #[inline]
    fn blend(
        &mut self,
        entity_index: usize,
        _: Mask,
        slot: usize,
        blend_group: &mut AnimatorBlendGroup,
        value: Self,
        weight: f32,
        _: bool,
    ) {
        blend_group.blend_contest(entity_index, slot, self, value, weight);
    }
}

impl<T: Blend> Blend for Option<T> {
    fn requires_weight_slot() -> bool {
        true
    }

    fn blend(
        &mut self,
        entity_index: usize,
        bit_mask: Mask,
        slot: usize,
        blend_group: &mut AnimatorBlendGroup,
        value: Self,
        weight: f32,
        additive: bool,
    ) {
        match (self.is_some(), value.is_some()) {
            (true, true) => {
                // Blend by lerp but also add the entry for the conext blending
                self.as_mut().unwrap().blend(
                    entity_index,
                    bit_mask,
                    slot,
                    blend_group,
                    value.unwrap(),
                    weight,
                    additive,
                );

                // Make sure to also add an entry for contest blent to work
                let w = &mut blend_group.blending.weights[entity_index * slot];
                if weight > *w {
                    *w = weight;
                }
            }
            (false, true) | (true, false) | (false, false) => {
                // Blend by context but also add an entry for the next blend_lerp if needed
                blend_group.blending.bits[entity_index] |= bit_mask;
                blend_group.blend_contest(entity_index, slot, self, value, weight);
            }
        }
    }
}
