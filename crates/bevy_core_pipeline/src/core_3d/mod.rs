mod alpha_mask_pass_3d_node;
mod camera_3d;
mod opaque_pass_3d_node;
mod transparent_pass_3d_node;

pub mod graph {
    pub const NAME: &str = "core_3d";
    pub mod input {
        pub const VIEW_ENTITY: &str = "view_entity";
    }
    pub mod node {
        pub const OPAQUE_PASS: &str = "opaque_pass";
        pub const ALPHA_MASK_PASS: &str = "alpha_mask_pass";
        pub const TRANSPARENT_PASS: &str = "transparent_pass";
    }
}

use std::cmp::Reverse;

pub use alpha_mask_pass_3d_node::*;
pub use camera_3d::*;
pub use opaque_pass_3d_node::*;
pub use transparent_pass_3d_node::*;

use bevy_app::{App, Plugin};
use bevy_ecs::prelude::*;
use bevy_render::{
    camera::{Camera, ExtractedCamera},
    extract_component::ExtractComponentPlugin,
    prelude::Msaa,
    render_graph::{RenderGraph, SlotInfo, SlotType},
    render_phase::{
        sort_phase_system, CachedRenderPipelinePhaseItem, DrawFunctionId, DrawFunctions,
        EntityPhaseItem, PhaseItem, RenderPhase,
    },
    render_resource::{
        CachedRenderPipelineId, Extent3d, TextureDescriptor, TextureDimension, TextureFormat,
        TextureUsages,
    },
    renderer::RenderDevice,
    texture::TextureCache,
    view::ViewDepthTexture,
    Extract, RenderApp, RenderStage,
};
use bevy_utils::{FloatOrd, HashMap};

pub struct Core3dPlugin;

impl Plugin for Core3dPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<Camera3d>()
            .register_type::<Camera3dDepthLoadOp>()
            .add_plugin(ExtractComponentPlugin::<Camera3d>::default());

        let render_app = match app.get_sub_app_mut(RenderApp) {
            Ok(render_app) => render_app,
            Err(_) => return,
        };

        render_app
            .init_resource::<DrawFunctions<Opaque3d>>()
            .init_resource::<DrawFunctions<AlphaMask3d>>()
            .init_resource::<DrawFunctions<Transparent3d>>()
            .add_system_to_stage(RenderStage::Extract, extract_core_3d_camera_phases)
            .add_system_to_stage(RenderStage::Prepare, prepare_core_3d_depth_textures)
            .add_system_to_stage(RenderStage::PhaseSort, sort_phase_system::<Opaque3d>)
            .add_system_to_stage(RenderStage::PhaseSort, sort_phase_system::<AlphaMask3d>)
            .add_system_to_stage(RenderStage::PhaseSort, sort_phase_system::<Transparent3d>);

        // Init 3d sub-graph
        let mut draw_3d_graph = RenderGraph::default();
        let view_node_id = draw_3d_graph.set_input(vec![SlotInfo::new(
            graph::input::VIEW_ENTITY,
            SlotType::Entity,
        )]);

        // Build opaque pass
        let opaque_pass_3d = OpaquePass3dNode::new(&mut render_app.world);
        draw_3d_graph.add_node(graph::node::OPAQUE_PASS, opaque_pass_3d);
        draw_3d_graph
            .add_slot_edge(
                view_node_id,
                graph::input::VIEW_ENTITY,
                graph::node::OPAQUE_PASS,
                OpaquePass3dNode::IN_VIEW,
            )
            .unwrap();

        // Build alpha mask pass
        let alpha_mask_pass_3d = AlphaMaskPass3dNode::new(&mut render_app.world);
        draw_3d_graph.add_node(graph::node::ALPHA_MASK_PASS, alpha_mask_pass_3d);
        draw_3d_graph
            .add_slot_edge(
                view_node_id,
                graph::input::VIEW_ENTITY,
                graph::node::ALPHA_MASK_PASS,
                OpaquePass3dNode::IN_VIEW,
            )
            .unwrap();
        draw_3d_graph
            .add_node_edge(graph::node::OPAQUE_PASS, graph::node::ALPHA_MASK_PASS)
            .unwrap();

        // Build transparent pass
        let transparent_pass_3d = TransparentPass3dNode::new(&mut render_app.world);
        draw_3d_graph.add_node(graph::node::TRANSPARENT_PASS, transparent_pass_3d);
        draw_3d_graph
            .add_slot_edge(
                view_node_id,
                graph::input::VIEW_ENTITY,
                graph::node::TRANSPARENT_PASS,
                OpaquePass3dNode::IN_VIEW,
            )
            .unwrap();
        draw_3d_graph
            .add_node_edge(graph::node::ALPHA_MASK_PASS, graph::node::TRANSPARENT_PASS)
            .unwrap();

        // Add 3d graph to main render graph
        let mut graph = render_app.world.resource_mut::<RenderGraph>();
        graph.add_sub_graph(graph::NAME, draw_3d_graph);
    }
}

pub struct Opaque3d {
    pub distance: f32,
    pub pipeline: CachedRenderPipelineId,
    pub entity: Entity,
    pub draw_function: DrawFunctionId,
}

impl PhaseItem for Opaque3d {
    // NOTE: Values increase towards the camera. Front-to-back ordering for opaque means we need a descending sort.
    type SortKey = Reverse<FloatOrd>;

    #[inline]
    fn sort_key(&self) -> Self::SortKey {
        Reverse(FloatOrd(self.distance))
    }

    #[inline]
    fn draw_function(&self) -> DrawFunctionId {
        self.draw_function
    }

    #[inline]
    fn sort(items: &mut [Self]) {
        // Key negated to match reversed SortKey ordering
        radsort::sort_by_key(items, |item| -item.distance);
    }
}

impl EntityPhaseItem for Opaque3d {
    #[inline]
    fn entity(&self) -> Entity {
        self.entity
    }
}

impl CachedRenderPipelinePhaseItem for Opaque3d {
    #[inline]
    fn cached_pipeline(&self) -> CachedRenderPipelineId {
        self.pipeline
    }
}

pub struct AlphaMask3d {
    pub distance: f32,
    pub pipeline: CachedRenderPipelineId,
    pub entity: Entity,
    pub draw_function: DrawFunctionId,
}

impl PhaseItem for AlphaMask3d {
    // NOTE: Values increase towards the camera. Front-to-back ordering for alpha mask means we need a descending sort.
    type SortKey = Reverse<FloatOrd>;

    #[inline]
    fn sort_key(&self) -> Self::SortKey {
        Reverse(FloatOrd(self.distance))
    }

    #[inline]
    fn draw_function(&self) -> DrawFunctionId {
        self.draw_function
    }

    #[inline]
    fn sort(items: &mut [Self]) {
        // Key negated to match reversed SortKey ordering
        radsort::sort_by_key(items, |item| -item.distance);
    }
}

impl EntityPhaseItem for AlphaMask3d {
    #[inline]
    fn entity(&self) -> Entity {
        self.entity
    }
}

impl CachedRenderPipelinePhaseItem for AlphaMask3d {
    #[inline]
    fn cached_pipeline(&self) -> CachedRenderPipelineId {
        self.pipeline
    }
}

pub struct Transparent3d {
    pub distance: f32,
    pub pipeline: CachedRenderPipelineId,
    pub entity: Entity,
    pub draw_function: DrawFunctionId,
}

impl PhaseItem for Transparent3d {
    // NOTE: Values increase towards the camera. Back-to-front ordering for transparent means we need an ascending sort.
    type SortKey = FloatOrd;

    #[inline]
    fn sort_key(&self) -> Self::SortKey {
        FloatOrd(self.distance)
    }

    #[inline]
    fn draw_function(&self) -> DrawFunctionId {
        self.draw_function
    }

    #[inline]
    fn sort(items: &mut [Self]) {
        radsort::sort_by_key(items, |item| item.distance);
    }
}

impl EntityPhaseItem for Transparent3d {
    #[inline]
    fn entity(&self) -> Entity {
        self.entity
    }
}

impl CachedRenderPipelinePhaseItem for Transparent3d {
    #[inline]
    fn cached_pipeline(&self) -> CachedRenderPipelineId {
        self.pipeline
    }
}

pub fn extract_core_3d_camera_phases(
    mut commands: Commands,
    cameras_3d: Extract<Query<(Entity, &Camera), With<Camera3d>>>,
) {
    for (entity, camera) in &cameras_3d {
        if camera.is_active {
            commands.get_or_spawn(entity).insert((
                RenderPhase::<Opaque3d>::default(),
                RenderPhase::<AlphaMask3d>::default(),
                RenderPhase::<Transparent3d>::default(),
            ));
        }
    }
}

pub fn prepare_core_3d_depth_textures(
    mut commands: Commands,
    mut texture_cache: ResMut<TextureCache>,
    msaa: Res<Msaa>,
    render_device: Res<RenderDevice>,
    views_3d: Query<
        (Entity, &ExtractedCamera),
        (
            With<RenderPhase<Opaque3d>>,
            With<RenderPhase<AlphaMask3d>>,
            With<RenderPhase<Transparent3d>>,
        ),
    >,
) {
    let mut textures = HashMap::default();
    for (entity, camera) in &views_3d {
        if let Some(physical_target_size) = camera.physical_target_size {
            let cached_texture = textures
                .entry(camera.target.clone())
                .or_insert_with(|| {
                    texture_cache.get(
                        &render_device,
                        TextureDescriptor {
                            label: Some("view_depth_texture"),
                            size: Extent3d {
                                depth_or_array_layers: 1,
                                width: physical_target_size.x,
                                height: physical_target_size.y,
                            },
                            mip_level_count: 1,
                            sample_count: msaa.samples,
                            dimension: TextureDimension::D2,
                            format: TextureFormat::Depth32Float, /* PERF: vulkan docs recommend using 24
                                                                  * bit depth for better performance */
                            usage: TextureUsages::RENDER_ATTACHMENT,
                        },
                    )
                })
                .clone();
            commands.entity(entity).insert(ViewDepthTexture {
                texture: cached_texture.texture,
                view: cached_texture.default_view,
            });
        }
    }
}
