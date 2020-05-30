mod color_material;
pub mod entity;
mod rect;
mod render;
mod sprite;

pub use color_material::*;
pub use rect::*;
pub use render::*;
pub use sprite::*;

use bevy_app::{stage, AppBuilder, AppPlugin};
use bevy_asset::{AddAsset, Assets, Handle};
use bevy_render::{
    mesh::{shape::Quad, Mesh},
    render_graph::RenderGraph,
    shader::asset_shader_def_system,
};
use glam::Vec2;
use legion::prelude::IntoSystem;
use sprite::sprite_system;

#[derive(Default)]
pub struct SpritePlugin;

pub const QUAD_HANDLE: Handle<Mesh> = Handle::from_u128(142404619811301375266013514540294236421);

impl AppPlugin for SpritePlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.add_asset::<ColorMaterial>()
            .add_system_to_stage(stage::POST_UPDATE, sprite_system())
            .add_system_to_stage(
                stage::POST_UPDATE,
                asset_shader_def_system::<ColorMaterial>.system(),
            );

        let resources = app.resources();
        let mut render_graph = resources.get_mut::<RenderGraph>().unwrap();
        render_graph.add_sprite_graph(resources);

        let mut meshes = resources.get_mut::<Assets<Mesh>>().unwrap();
        meshes.set(
            QUAD_HANDLE,
            Mesh::from(Quad {
                size: Vec2::new(1.0, 1.0),
            }),
        );

        let mut color_materials = resources.get_mut::<Assets<ColorMaterial>>().unwrap();
        color_materials.add_default(ColorMaterial::default());
    }
}
