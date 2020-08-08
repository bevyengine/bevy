pub mod entity;
pub mod light;
pub mod material;
pub mod nodes;
pub mod pipelines;
pub mod render_graph;

pub mod prelude {
    pub use crate::{entity::*, light::Light, material::StandardMaterial};
}

use bevy_app::prelude::*;
use bevy_asset::AddAsset;
use bevy_ecs::IntoQuerySystem;
use bevy_render::{render_graph::RenderGraph, shader};
use bevy_type_registry::RegisterType;
use light::Light;
use material::StandardMaterial;
use render_graph::ForwardPbrRenderGraphBuilder;

/// NOTE: this isn't PBR yet. consider this name "aspirational" :)
#[derive(Default)]
pub struct PbrPlugin;

impl Plugin for PbrPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.add_asset::<StandardMaterial>()
            .register_component::<Light>()
            .add_system_to_stage(
                stage::POST_UPDATE,
                shader::asset_shader_defs_system::<StandardMaterial>.system(),
            );
        let resources = app.resources();
        let mut render_graph = resources.get_mut::<RenderGraph>().unwrap();
        render_graph.add_pbr_graph(resources);
    }
}
