use bevy_math::Vec2;
use bevy_reflect::{Reflect, ReflectDeserialize, TypeUuid};
use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Clone, TypeUuid, Reflect)]
#[uuid = "7233c597-ccfa-411f-bd59-9af349432ada"]
#[repr(C)]
pub struct Sprite {
    pub size: Vec2,
    pub flip_x: bool,
    pub flip_y: bool,
    pub resize_mode: SpriteResizeMode,
}

/// Determines how `Sprite` resize should be handled
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Reflect)]
#[reflect_value(PartialEq, Serialize, Deserialize)]
pub enum SpriteResizeMode {
    Manual,
    Automatic,
}

impl Default for SpriteResizeMode {
    fn default() -> Self {
        SpriteResizeMode::Automatic
    }
}

impl Sprite {
    /// Creates new `Sprite` with `SpriteResizeMode::Manual` value for `resize_mode`
    pub fn new(size: Vec2) -> Self {
        Self {
            size,
            resize_mode: SpriteResizeMode::Manual,
            flip_x: false,
            flip_y: false,
        }
    }
}
