use bevy_asset::Handle;
use bevy_ecs::{component::Component, entity::Entity, reflect::ReflectComponent, world::FromWorld};
use bevy_math::{Rect, Vec2};
use bevy_reflect::{std_traits::ReflectDefault, Reflect};
use bevy_render::texture::Image;

use crate::Anchor;

#[derive(Component, Debug, Default, Clone, Reflect)]
#[reflect(Component, Default)]
#[repr(C)]
pub struct Mask {
    /// The `Image` used to occlude `Masked` `Sprite`s.
    /// If the `Image` is not grayscale, the red channel will be used.
    /// Samples of 0 completely occlude the `Sprite`, samples of 1 have
    /// no effect on the `Sprite`, and in between reduces the alpha
    /// proportionally.
    pub image: Handle<Image>,
    /// If set, samples from `image` less than `threshold` will be clamped to 0,
    /// greater will be clamped to 1.
    pub threshold: Option<f32>,

    /// Flip the mask along the `X` axis
    pub flip_x: bool,
    /// Flip the mask along the `Y` axis
    pub flip_y: bool,
    /// An optional custom size for the mask that will be used when rendering, instead of the size
    /// of the mask's image
    pub custom_size: Option<Vec2>,
    /// An optional rectangle representing the region of the mask's image to render, instead of
    /// masking as the full image.
    pub rect: Option<Rect>,
    /// [`Anchor`] point of the mask in the world
    pub anchor: Anchor,
}

#[derive(Component, Debug, Clone, Reflect)]
#[reflect(Component)]
#[repr(C)]
pub struct Masked {
    pub mask: Entity,
}

impl FromWorld for Masked {
    fn from_world(_world: &mut bevy_ecs::world::World) -> Self {
        Self {
            mask: Entity::PLACEHOLDER,
        }
    }
}
