pub mod render_graph;

mod entity;
mod light;
mod material;

use bevy_ecs::IntoSystem;
pub use entity::*;
pub use light::*;
pub use material::*;

pub mod prelude {
    pub use crate::{entity::*, light::Light, material::StandardMaterial};
}

use bevy_app::prelude::*;
use bevy_asset::{AddAsset, Assets, Handle};
use bevy_reflect::RegisterTypeBuilder;
use bevy_render::{prelude::Color, render_graph::RenderGraph, shader};
use material::StandardMaterial;
use render_graph::add_pbr_graph;

/// NOTE: this isn't PBR yet. consider this name "aspirational" :)
#[derive(Default)]
pub struct PbrPlugin;

impl Plugin for PbrPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.add_asset::<StandardMaterial>()
            .register_type::<Light>()
            .add_system_to_stage(
                stage::POST_UPDATE,
                shader::asset_shader_defs_system::<StandardMaterial>.system(),
            )
            .init_resource::<AmbientLight>();
        let resources = app.resources();
        let mut render_graph = resources.get_mut::<RenderGraph>().unwrap();
        add_pbr_graph(&mut render_graph, resources);

        // add default StandardMaterial
        let mut materials = app
            .resources()
            .get_mut::<Assets<StandardMaterial>>()
            .unwrap();
        materials.set_untracked(
            Handle::<StandardMaterial>::default(),
            StandardMaterial {
                albedo: Color::PINK,
                shaded: false,
                albedo_texture: None,
            },
        );
    }
}
