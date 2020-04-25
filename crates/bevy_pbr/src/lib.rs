pub mod entity;
pub mod light;
pub mod material;
pub mod nodes;
pub mod passes;
pub mod pipelines;

mod forward_pbr_render_graph;
pub use forward_pbr_render_graph::*;

use bevy_app::{AppBuilder, AppPlugin, stage};
use bevy_asset::AssetStorage;
use material::StandardMaterial;
use bevy_render::{render_graph::RenderGraph, shader};

#[derive(Default)]
pub struct PbrPlugin;

// NOTE: this isn't PBR yet. consider this name "aspirational" :)
impl AppPlugin for PbrPlugin {
    fn build(&self, app: &mut AppBuilder) {
        // asset_batchers.batch_types2::<Mesh, StandardMaterial>();
        app.add_resource(AssetStorage::<StandardMaterial>::new())
            .add_system_to_stage(
                stage::POST_UPDATE,
                shader::asset_handle_shader_def_system::<StandardMaterial>(),
            )
            .add_system_to_stage(
                stage::POST_UPDATE,
                shader::asset_handle_batcher_system::<StandardMaterial>(),
            );
        let resources = app.resources();
        let mut render_graph = resources.get_mut::<RenderGraph>().unwrap();
        render_graph.add_pbr_graph(resources);
    }
}
