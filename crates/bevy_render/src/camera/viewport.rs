use bevy_math::Vec2;
use bevy_reflect::{Reflect, ReflectComponent};

#[derive(Debug, PartialEq, Clone, Reflect)]
#[reflect(Component)]
pub struct Viewport {
    pub name: Option<String>,
    pub origin: Vec2,
    pub size: Vec2,
    pub scale_factor: f64,
}

impl Viewport {
    pub fn physical_origin(&self) -> Vec2 {
        (self.origin.as_f64() * self.scale_factor).as_f32()
    }

    pub fn physical_size(&self) -> Vec2 {
        (self.size.as_f64() * self.scale_factor).as_f32()
    }
}

impl Default for Viewport {
    fn default() -> Self {
        Self {
            name: None,
            origin: Vec2::zero(),
            size: Vec2::one(),
            scale_factor: 1.0,
        }
    }
}
