use bevy_derive::{Deref, DerefMut};
use bevy_reflect::{prelude::ReflectDefault, Reflect};
use core::hash::Hash;

/// The location of a material (either bindless or non-bindless) within the
/// slabs.
#[derive(Clone, Copy, Debug, Default, Reflect)]
#[reflect(Clone, Default)]
pub struct MaterialBindingId {
    /// The index of the bind group (slab) where the GPU data is located.
    pub group: MaterialBindGroupIndex,
    /// The slot within that bind group.
    ///
    /// Non-bindless materials will always have a slot of 0.
    pub slot: MaterialBindGroupSlot,
}

/// The index of each material bind group.
///
/// In bindless mode, each bind group contains multiple materials. In
/// non-bindless mode, each bind group contains only one material.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Reflect, Deref, DerefMut)]
#[reflect(Default, Clone, PartialEq, Hash)]
pub struct MaterialBindGroupIndex(pub u32);

impl From<u32> for MaterialBindGroupIndex {
    fn from(value: u32) -> Self {
        MaterialBindGroupIndex(value)
    }
}

/// The index of the slot containing material data within each material bind
/// group.
///
/// In bindless mode, this slot is needed to locate the material data in each
/// bind group, since multiple materials are packed into a single slab. In
/// non-bindless mode, this slot is always 0.
#[derive(Clone, Copy, Debug, Default, PartialEq, Reflect, Deref, DerefMut)]
#[reflect(Default, Clone, PartialEq)]
pub struct MaterialBindGroupSlot(pub u32);

impl From<u32> for MaterialBindGroupSlot {
    fn from(value: u32) -> Self {
        MaterialBindGroupSlot(value)
    }
}

impl From<MaterialBindGroupSlot> for u32 {
    fn from(value: MaterialBindGroupSlot) -> Self {
        value.0
    }
}
