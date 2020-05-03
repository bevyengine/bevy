pub mod entity;
pub mod light;
pub mod material;
pub mod nodes;
pub mod pipelines;

mod forward_pbr_render_graph;
pub use forward_pbr_render_graph::*;

use bevy_app::{stage, AppBuilder, AppPlugin};
use bevy_asset::AssetStorage;
use bevy_render::{render_graph::RenderGraph, shader};
use legion::prelude::IntoSystem;
use material::StandardMaterial;

/// NOTE: this isn't PBR yet. consider this name "aspirational" :)
#[derive(Default)]
pub struct PbrPlugin;

impl AppPlugin for PbrPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.add_resource(AssetStorage::<StandardMaterial>::new())
            .add_system_to_stage(
                stage::POST_UPDATE,
                shader::asset_handle_shader_def_system::<StandardMaterial>.system(),
            );
        let resources = app.resources();
        let mut render_graph = resources.get_mut::<RenderGraph>().unwrap();
        render_graph.add_pbr_graph(resources);
    }
}
