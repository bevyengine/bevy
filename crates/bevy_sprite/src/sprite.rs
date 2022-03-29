use bevy_ecs::component::Component;
use bevy_math::Vec2;
use bevy_reflect::Reflect;
use bevy_render::color::Color;

#[derive(Component, Debug, Default, Clone, Reflect)]
#[repr(C)]
pub struct Sprite {
    /// The sprite's color tint
    pub color: Color,
    /// Flip the sprite along the X axis
    pub flip_x: bool,
    /// Flip the sprite along the Y axis
    pub flip_y: bool,
    /// An optional custom size for the sprite that will be used when rendering, instead of the size
    /// of the sprite's image
    pub custom_size: Option<Vec2>,
}
