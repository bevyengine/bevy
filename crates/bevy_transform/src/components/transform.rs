use crate::math::Mat4;
use bevy_property::Properties;
use std::fmt;

#[derive(Debug, PartialEq, Clone, Copy, Properties)]
pub struct Transform {
    pub value: Mat4,
    #[property(ignore)]
    pub sync: bool, // NOTE: this is hopefully a temporary measure to allow setting the transform directly.
                    // ideally setting the transform automatically propagates back to position / translation / rotation,
                    // but right now they are always considered the source of truth
}

impl Transform {
    #[inline(always)]
    pub fn identity() -> Self {
        Transform {
            value: Mat4::identity(),
            sync: true,
        }
    }

    #[inline(always)]
    pub fn new(value: Mat4) -> Self {
        Transform { value, sync: true }
    }

    /// This creates a new `LocalToWorld` transform with the `sync` field set to `false`.
    /// While `sync` is false, position, rotation, and scale components will not be synced to the transform.
    /// This is helpful if you want to manually set the transform to a value (ex: Mat4::face_toward)  
    #[inline(always)]
    pub fn new_sync_disabled(value: Mat4) -> Self {
        Transform { value, sync: false }
    }
}

impl Default for Transform {
    fn default() -> Self {
        Self::identity()
    }
}

impl fmt::Display for Transform {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.value)
    }
}
