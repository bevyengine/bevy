mod clear_pass;
mod clear_pass_driver;
mod main_pass_2d;
mod main_pass_3d;
mod main_pass_driver;

pub mod prelude {
    #[doc(hidden)]
    pub use crate::ClearColor;
}

pub use clear_pass::*;
pub use clear_pass_driver::*;
pub use main_pass_2d::*;
pub use main_pass_3d::*;
pub use main_pass_driver::*;

use bevy_app::{App, Plugin};
use bevy_core::FloatOrd;
use bevy_ecs::prelude::*;
use bevy_render::{
    camera::{ActiveCameras, CameraPlugin},
    color::Color,
    render_graph::{EmptyNode, RenderGraph, SlotInfo, SlotType},
    render_phase::{
        sort_phase_system, CachedPipelinePhaseItem, DrawFunctionId, DrawFunctions, EntityPhaseItem,
        PhaseItem, RenderPhase,
    },
    render_resource::*,
    renderer::RenderDevice,
    texture::TextureCache,
    view::{ExtractedView, Msaa, ViewDepthTexture},
    RenderApp, RenderStage, RenderWorld,
};

use crate::clear_pass::ClearPassNode;

/// Resource that configures the clear color
#[derive(Clone, Debug)]
pub struct ClearColor(pub Color);

impl Default for ClearColor {
    fn default() -> Self {
        Self(Color::rgb(0.4, 0.4, 0.4))
    }
}

// Plugins that contribute to the RenderGraph should use the following label conventions:
// 1. Graph modules should have a NAME, input module, and node module (where relevant)
// 2. The "top level" graph is the plugin module root. Just add things like `pub mod node` directly under the plugin module
// 3. "sub graph" modules should be nested beneath their parent graph module

pub mod node {
    pub const MAIN_PASS_DEPENDENCIES: &str = "main_pass_dependencies";
    pub const MAIN_PASS_DRIVER: &str = "main_pass_driver";
    pub const CLEAR_PASS_DRIVER: &str = "clear_pass_driver";
}

pub mod draw_2d_graph {
    pub const NAME: &str = "draw_2d";
    pub mod input {
        pub const VIEW_ENTITY: &str = "view_entity";
    }
    pub mod node {
        pub const MAIN_PASS: &str = "main_pass";
    }
}

pub mod draw_3d_graph {
    pub const NAME: &str = "draw_3d";
    pub mod input {
        pub const VIEW_ENTITY: &str = "view_entity";
    }
    pub mod node {
        pub const MAIN_PASS: &str = "main_pass";
    }
}

pub mod clear_graph {
    pub const NAME: &str = "clear";
    pub mod node {
        pub const CLEAR_PASS: &str = "clear_pass";
    }
}

#[derive(Default)]
pub struct CorePipelinePlugin;

impl Plugin for CorePipelinePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ClearColor>();

        let render_app = app.sub_app(RenderApp);
        render_app
            .init_resource::<DrawFunctions<Transparent2d>>()
            .init_resource::<DrawFunctions<Opaque3d>>()
            .init_resource::<DrawFunctions<AlphaMask3d>>()
            .init_resource::<DrawFunctions<Transparent3d>>()
            .add_system_to_stage(RenderStage::Extract, extract_clear_color)
            .add_system_to_stage(RenderStage::Extract, extract_core_pipeline_camera_phases)
            .add_system_to_stage(RenderStage::Prepare, prepare_core_views_system)
            .add_system_to_stage(RenderStage::PhaseSort, sort_phase_system::<Transparent2d>)
            .add_system_to_stage(RenderStage::PhaseSort, sort_phase_system::<Opaque3d>)
            .add_system_to_stage(RenderStage::PhaseSort, sort_phase_system::<AlphaMask3d>)
            .add_system_to_stage(RenderStage::PhaseSort, sort_phase_system::<Transparent3d>);

        let clear_pass_node = ClearPassNode::new(&mut render_app.world);
        let pass_node_2d = MainPass2dNode::new(&mut render_app.world);
        let pass_node_3d = MainPass3dNode::new(&mut render_app.world);
        let mut graph = render_app.world.get_resource_mut::<RenderGraph>().unwrap();

        let mut draw_2d_graph = RenderGraph::default();
        draw_2d_graph.add_node(draw_2d_graph::node::MAIN_PASS, pass_node_2d);
        let input_node_id = draw_2d_graph.set_input(vec![SlotInfo::new(
            draw_2d_graph::input::VIEW_ENTITY,
            SlotType::Entity,
        )]);
        draw_2d_graph
            .add_slot_edge(
                input_node_id,
                draw_2d_graph::input::VIEW_ENTITY,
                draw_2d_graph::node::MAIN_PASS,
                MainPass2dNode::IN_VIEW,
            )
            .unwrap();
        graph.add_sub_graph(draw_2d_graph::NAME, draw_2d_graph);

        let mut draw_3d_graph = RenderGraph::default();
        draw_3d_graph.add_node(draw_3d_graph::node::MAIN_PASS, pass_node_3d);
        let input_node_id = draw_3d_graph.set_input(vec![SlotInfo::new(
            draw_3d_graph::input::VIEW_ENTITY,
            SlotType::Entity,
        )]);
        draw_3d_graph
            .add_slot_edge(
                input_node_id,
                draw_3d_graph::input::VIEW_ENTITY,
                draw_3d_graph::node::MAIN_PASS,
                MainPass3dNode::IN_VIEW,
            )
            .unwrap();
        graph.add_sub_graph(draw_3d_graph::NAME, draw_3d_graph);

        let mut clear_graph = RenderGraph::default();
        clear_graph.add_node(clear_graph::node::CLEAR_PASS, clear_pass_node);
        graph.add_sub_graph(clear_graph::NAME, clear_graph);

        graph.add_node(node::MAIN_PASS_DEPENDENCIES, EmptyNode);
        graph.add_node(node::MAIN_PASS_DRIVER, MainPassDriverNode);
        graph
            .add_node_edge(node::MAIN_PASS_DEPENDENCIES, node::MAIN_PASS_DRIVER)
            .unwrap();
        graph.add_node(node::CLEAR_PASS_DRIVER, ClearPassDriverNode);
        graph
            .add_node_edge(node::CLEAR_PASS_DRIVER, node::MAIN_PASS_DRIVER)
            .unwrap();
    }
}

pub struct Transparent2d {
    pub sort_key: FloatOrd,
    pub entity: Entity,
    pub pipeline: CachedPipelineId,
    pub draw_function: DrawFunctionId,
}

impl PhaseItem for Transparent2d {
    type SortKey = FloatOrd;

    #[inline]
    fn sort_key(&self) -> Self::SortKey {
        self.sort_key
    }

    #[inline]
    fn draw_function(&self) -> DrawFunctionId {
        self.draw_function
    }
}

pub struct Opaque3d {
    pub distance: f32,
    pub pipeline: CachedPipelineId,
    pub entity: Entity,
    pub draw_function: DrawFunctionId,
}

impl PhaseItem for Opaque3d {
    type SortKey = FloatOrd;

    #[inline]
    fn sort_key(&self) -> Self::SortKey {
        FloatOrd(self.distance)
    }

    #[inline]
    fn draw_function(&self) -> DrawFunctionId {
        self.draw_function
    }
}

impl EntityPhaseItem for Opaque3d {
    #[inline]
    fn entity(&self) -> Entity {
        self.entity
    }
}

impl CachedPipelinePhaseItem for Opaque3d {
    #[inline]
    fn cached_pipeline(&self) -> CachedPipelineId {
        self.pipeline
    }
}

pub struct AlphaMask3d {
    pub distance: f32,
    pub pipeline: CachedPipelineId,
    pub entity: Entity,
    pub draw_function: DrawFunctionId,
}

impl PhaseItem for AlphaMask3d {
    type SortKey = FloatOrd;

    #[inline]
    fn sort_key(&self) -> Self::SortKey {
        FloatOrd(self.distance)
    }

    #[inline]
    fn draw_function(&self) -> DrawFunctionId {
        self.draw_function
    }
}

impl EntityPhaseItem for AlphaMask3d {
    #[inline]
    fn entity(&self) -> Entity {
        self.entity
    }
}

impl CachedPipelinePhaseItem for AlphaMask3d {
    #[inline]
    fn cached_pipeline(&self) -> CachedPipelineId {
        self.pipeline
    }
}

pub struct Transparent3d {
    pub distance: f32,
    pub pipeline: CachedPipelineId,
    pub entity: Entity,
    pub draw_function: DrawFunctionId,
}

impl PhaseItem for Transparent3d {
    type SortKey = FloatOrd;

    #[inline]
    fn sort_key(&self) -> Self::SortKey {
        FloatOrd(self.distance)
    }

    #[inline]
    fn draw_function(&self) -> DrawFunctionId {
        self.draw_function
    }
}

impl EntityPhaseItem for Transparent3d {
    #[inline]
    fn entity(&self) -> Entity {
        self.entity
    }
}

impl CachedPipelinePhaseItem for Transparent3d {
    #[inline]
    fn cached_pipeline(&self) -> CachedPipelineId {
        self.pipeline
    }
}

pub fn extract_clear_color(clear_color: Res<ClearColor>, mut render_world: ResMut<RenderWorld>) {
    // If the clear color has changed
    if clear_color.is_changed() {
        // Update the clear color resource in the render world
        render_world.insert_resource(clear_color.clone())
    }
}

pub fn extract_core_pipeline_camera_phases(
    mut commands: Commands,
    active_cameras: Res<ActiveCameras>,
) {
    if let Some(camera_2d) = active_cameras.get(CameraPlugin::CAMERA_2D) {
        if let Some(entity) = camera_2d.entity {
            commands
                .get_or_spawn(entity)
                .insert(RenderPhase::<Transparent2d>::default());
        }
    }
    if let Some(camera_3d) = active_cameras.get(CameraPlugin::CAMERA_3D) {
        if let Some(entity) = camera_3d.entity {
            commands.get_or_spawn(entity).insert_bundle((
                RenderPhase::<Opaque3d>::default(),
                RenderPhase::<AlphaMask3d>::default(),
                RenderPhase::<Transparent3d>::default(),
            ));
        }
    }
}

pub fn prepare_core_views_system(
    mut commands: Commands,
    mut texture_cache: ResMut<TextureCache>,
    msaa: Res<Msaa>,
    render_device: Res<RenderDevice>,
    views_3d: Query<
        (Entity, &ExtractedView),
        (
            With<RenderPhase<Opaque3d>>,
            With<RenderPhase<AlphaMask3d>>,
            With<RenderPhase<Transparent3d>>,
        ),
    >,
) {
    for (entity, view) in views_3d.iter() {
        let cached_texture = texture_cache.get(
            &render_device,
            TextureDescriptor {
                label: Some("view_depth_texture"),
                size: Extent3d {
                    depth_or_array_layers: 1,
                    width: view.width as u32,
                    height: view.height as u32,
                },
                mip_level_count: 1,
                sample_count: msaa.samples,
                dimension: TextureDimension::D2,
                format: TextureFormat::Depth32Float, /* PERF: vulkan docs recommend using 24
                                                      * bit depth for better performance */
                usage: TextureUsages::RENDER_ATTACHMENT,
            },
        );
        commands.entity(entity).insert(ViewDepthTexture {
            texture: cached_texture.texture,
            view: cached_texture.default_view,
        });
    }
}
