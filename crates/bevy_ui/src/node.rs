use bevy_math::Vec2;
use bevy_render::renderer::RenderResources;

#[derive(Debug, Clone, Default, RenderResources)]
pub struct Node {
    pub size: Vec2,
}
