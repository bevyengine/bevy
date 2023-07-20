mod extract;
mod pipeline;
mod prepare;
mod queue;
mod render_pass;

use bevy_app::prelude::*;
use bevy_asset::{load_internal_asset, Handle, HandleUntyped};
use bevy_core_pipeline::{core_2d::Camera2d, core_3d::Camera3d};
use bevy_ecs::prelude::*;
use bevy_reflect::TypeUuid;
use bevy_render::{
    render_graph::{RenderGraph, RunGraphOnViewNode},
    render_phase::{sort_phase_system, AddRenderCommand, DrawFunctions},
    render_resource::*,
    texture::Image,
    RenderApp, RenderSet,
};
use bevy_render::{ExtractSchedule, Render};
use bevy_utils::HashMap;
pub use extract::*;
pub use pipeline::*;
pub use prepare::*;
pub use queue::*;
pub use render_pass::*;

pub mod node {
    pub const UI_PASS_DRIVER: &str = "ui_pass_driver";
}

pub mod draw_ui_graph {
    pub const NAME: &str = "draw_ui";
    pub mod node {
        pub const UI_PASS: &str = "ui_pass";
    }
}

pub const UI_SHADER_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 13012847047162779583);

#[derive(Resource, Default)]
pub struct UiImageBindGroups {
    pub values: HashMap<Handle<Image>, BindGroup>,
}

#[derive(Resource)]
pub struct UiMeta {
    pub(crate) vertices: BufferVec<UiVertex>,
    pub(crate) view_bind_group: Option<BindGroup>,
}

impl Default for UiMeta {
    fn default() -> Self {
        Self {
            vertices: BufferVec::new(BufferUsages::VERTEX),
            view_bind_group: None,
        }
    }
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
pub enum RenderUiSystem {
    ExtractNode,
}

pub fn build_ui_render(app: &mut App) {
    load_internal_asset!(app, UI_SHADER_HANDLE, "ui.wgsl", Shader::from_wgsl);

    let render_app = match app.get_sub_app_mut(RenderApp) {
        Ok(render_app) => render_app,
        Err(_) => return,
    };

    render_app
        .init_resource::<SpecializedRenderPipelines<UiPipeline>>()
        .init_resource::<UiImageBindGroups>()
        .init_resource::<UiMeta>()
        .init_resource::<ExtractedUiNodes>()
        .init_resource::<DrawFunctions<TransparentUi>>()
        .add_render_command::<TransparentUi, DrawUi>()
        .add_systems(
            ExtractSchedule,
            (
                extract_default_ui_camera_view::<Camera2d>,
                extract_default_ui_camera_view::<Camera3d>,
                extract_uinodes.in_set(RenderUiSystem::ExtractNode),
                extract_atlas_uinodes.after(RenderUiSystem::ExtractNode),
                extract_uinode_borders.after(RenderUiSystem::ExtractNode),
                #[cfg(feature = "bevy_text")]
                extract_text_uinodes.after(RenderUiSystem::ExtractNode),
            ),
        )
        .add_systems(
            Render,
            (
                prepare_uinodes.in_set(RenderSet::Prepare),
                queue_uinodes.in_set(RenderSet::Queue),
                sort_phase_system::<TransparentUi>.in_set(RenderSet::PhaseSort),
            ),
        );

    // Render graph
    let ui_graph_2d = get_ui_graph(render_app);
    let ui_graph_3d = get_ui_graph(render_app);
    let mut graph = render_app.world.resource_mut::<RenderGraph>();

    if let Some(graph_2d) = graph.get_sub_graph_mut(bevy_core_pipeline::core_2d::graph::NAME) {
        graph_2d.add_sub_graph(draw_ui_graph::NAME, ui_graph_2d);
        graph_2d.add_node(
            draw_ui_graph::node::UI_PASS,
            RunGraphOnViewNode::new(draw_ui_graph::NAME),
        );
        graph_2d.add_node_edge(
            bevy_core_pipeline::core_2d::graph::node::MAIN_PASS,
            draw_ui_graph::node::UI_PASS,
        );
        graph_2d.add_node_edge(
            bevy_core_pipeline::core_2d::graph::node::END_MAIN_PASS_POST_PROCESSING,
            draw_ui_graph::node::UI_PASS,
        );
        graph_2d.add_node_edge(
            draw_ui_graph::node::UI_PASS,
            bevy_core_pipeline::core_2d::graph::node::UPSCALING,
        );
    }

    if let Some(graph_3d) = graph.get_sub_graph_mut(bevy_core_pipeline::core_3d::graph::NAME) {
        graph_3d.add_sub_graph(draw_ui_graph::NAME, ui_graph_3d);
        graph_3d.add_node(
            draw_ui_graph::node::UI_PASS,
            RunGraphOnViewNode::new(draw_ui_graph::NAME),
        );
        graph_3d.add_node_edge(
            bevy_core_pipeline::core_3d::graph::node::END_MAIN_PASS,
            draw_ui_graph::node::UI_PASS,
        );
        graph_3d.add_node_edge(
            bevy_core_pipeline::core_3d::graph::node::END_MAIN_PASS_POST_PROCESSING,
            draw_ui_graph::node::UI_PASS,
        );
        graph_3d.add_node_edge(
            draw_ui_graph::node::UI_PASS,
            bevy_core_pipeline::core_3d::graph::node::UPSCALING,
        );
    }
}

fn get_ui_graph(render_app: &mut App) -> RenderGraph {
    let ui_pass_node = UiPassNode::new(&mut render_app.world);
    let mut ui_graph = RenderGraph::default();
    ui_graph.add_node(draw_ui_graph::node::UI_PASS, ui_pass_node);
    ui_graph
}
