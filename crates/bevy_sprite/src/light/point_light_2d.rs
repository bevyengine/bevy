use bevy_color::Color;
use bevy_ecs::component::Component;
use bevy_math::*;
use bevy_reflect::Reflect;

/// A 2D point light for lighting 2D sprites.
#[derive(Component, Clone, Copy, Debug, Reflect)]
pub struct PointLight2D {
    // The color of this light source
    pub color: Color,

    // Amount of light in lumens emitted by this source in all directions
    pub intensity: f32,

    // Affects the size of specular highlights created by this light
    pub radius: f32,

    // This affects how light will fade with distance
    pub falloff: FalloffType,
}

#[derive(Debug, Clone, Copy, Reflect, PartialEq)]
pub enum FalloffType {
    Linear,
    Exponential,
}

impl Default for PointLight2D {
    fn default() -> Self {
        Self {
            color: Color::WHITE,
            intensity: 1.0,
            radius: 300.0,
            falloff: FalloffType::Linear,
        }
    }
}
