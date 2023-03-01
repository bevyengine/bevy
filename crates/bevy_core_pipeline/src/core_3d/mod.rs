mod camera_3d;
mod main_pass_3d_node;

pub mod graph {
    pub const NAME: &str = "core_3d";
    pub mod input {
        pub const VIEW_ENTITY: &str = "view_entity";
    }
    pub mod node {
        pub const PREPASS: &str = "prepass";
        pub const MAIN_PASS: &str = "main_pass";
        pub const BLOOM: &str = "bloom";
        pub const TONEMAPPING: &str = "tonemapping";
        pub const FXAA: &str = "fxaa";
        pub const UPSCALING: &str = "upscaling";
        pub const END_MAIN_PASS_POST_PROCESSING: &str = "end_main_pass_post_processing";
    }
}

use std::cmp::Reverse;

pub use camera_3d::*;
pub use main_pass_3d_node::*;

use bevy_app::{App, IntoSystemAppConfig, Plugin};
use bevy_ecs::prelude::*;
use bevy_render::{
    camera::{Camera, ExtractedCamera},
    extract_component::ExtractComponentPlugin,
    prelude::Msaa,
    render_graph::{EmptyNode, RenderGraph, SlotInfo, SlotType},
    render_phase::{
        sort_phase_system, CachedRenderPipelinePhaseItem, DrawFunctionId, DrawFunctions, PhaseItem,
        RenderPhase,
    },
    render_resource::{
        CachedRenderPipelineId, Extent3d, TextureDescriptor, TextureDimension, TextureFormat,
        TextureUsages,
    },
    renderer::RenderDevice,
    texture::TextureCache,
    view::ViewDepthTexture,
    Extract, ExtractSchedule, RenderApp, RenderSet,
};
use bevy_utils::{FloatOrd, HashMap};

use crate::{
    prepass::{node::PrepassNode, DepthPrepass},
    tonemapping::TonemappingNode,
    upscaling::UpscalingNode,
};

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
            .add_system(extract_core_3d_camera_phases.in_schedule(ExtractSchedule))
            .add_system(
                prepare_core_3d_depth_textures
                    .in_set(RenderSet::Prepare)
                    .after(bevy_render::view::prepare_windows),
            )
            .add_system(sort_phase_system::<Opaque3d>.in_set(RenderSet::PhaseSort))
            .add_system(sort_phase_system::<AlphaMask3d>.in_set(RenderSet::PhaseSort))
            .add_system(sort_phase_system::<Transparent3d>.in_set(RenderSet::PhaseSort));

        let prepass_node = PrepassNode::new(&mut render_app.world);
        let pass_node_3d = MainPass3dNode::new(&mut render_app.world);
        let tonemapping = TonemappingNode::new(&mut render_app.world);
        let upscaling = UpscalingNode::new(&mut render_app.world);
        let mut graph = render_app.world.resource_mut::<RenderGraph>();

        let mut draw_3d_graph = RenderGraph::default();
        draw_3d_graph.add_node(graph::node::PREPASS, prepass_node);
        draw_3d_graph.add_node(graph::node::MAIN_PASS, pass_node_3d);
        draw_3d_graph.add_node(graph::node::TONEMAPPING, tonemapping);
        draw_3d_graph.add_node(graph::node::END_MAIN_PASS_POST_PROCESSING, EmptyNode);
        draw_3d_graph.add_node(graph::node::UPSCALING, upscaling);

        let input_node_id = draw_3d_graph.set_input(vec![SlotInfo::new(
            graph::input::VIEW_ENTITY,
            SlotType::Entity,
        )]);
        draw_3d_graph.add_slot_edge(
            input_node_id,
            graph::input::VIEW_ENTITY,
            graph::node::PREPASS,
            PrepassNode::IN_VIEW,
        );
        draw_3d_graph.add_slot_edge(
            input_node_id,
            graph::input::VIEW_ENTITY,
            graph::node::MAIN_PASS,
            MainPass3dNode::IN_VIEW,
        );
        draw_3d_graph.add_slot_edge(
            input_node_id,
            graph::input::VIEW_ENTITY,
            graph::node::TONEMAPPING,
            TonemappingNode::IN_VIEW,
        );
        draw_3d_graph.add_slot_edge(
            input_node_id,
            graph::input::VIEW_ENTITY,
            graph::node::UPSCALING,
            UpscalingNode::IN_VIEW,
        );
        draw_3d_graph.add_node_edge(graph::node::PREPASS, graph::node::MAIN_PASS);
        draw_3d_graph.add_node_edge(graph::node::MAIN_PASS, graph::node::TONEMAPPING);
        draw_3d_graph.add_node_edge(
            graph::node::TONEMAPPING,
            graph::node::END_MAIN_PASS_POST_PROCESSING,
        );
        draw_3d_graph.add_node_edge(
            graph::node::END_MAIN_PASS_POST_PROCESSING,
            graph::node::UPSCALING,
        );
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
    fn entity(&self) -> Entity {
        self.entity
    }

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
    fn entity(&self) -> Entity {
        self.entity
    }

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
    fn entity(&self) -> Entity {
        self.entity
    }

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
        (Entity, &ExtractedCamera, Option<&DepthPrepass>),
        (
            With<RenderPhase<Opaque3d>>,
            With<RenderPhase<AlphaMask3d>>,
            With<RenderPhase<Transparent3d>>,
        ),
    >,
) {
    let mut textures = HashMap::default();
    for (entity, camera, depth_prepass) in &views_3d {
        let Some(physical_target_size) = camera.physical_target_size else {
            continue;
        };

        let cached_texture = textures
            .entry(camera.target.clone())
            .or_insert_with(|| {
                // Default usage required to write to the depth texture
                let mut usage = TextureUsages::RENDER_ATTACHMENT;
                if depth_prepass.is_some() {
                    // Required to read the output of the prepass
                    usage |= TextureUsages::COPY_SRC;
                }

                // The size of the depth texture
                let size = Extent3d {
                    depth_or_array_layers: 1,
                    width: physical_target_size.x,
                    height: physical_target_size.y,
                };

                let descriptor = TextureDescriptor {
                    label: Some("view_depth_texture"),
                    size,
                    mip_level_count: 1,
                    sample_count: msaa.samples(),
                    dimension: TextureDimension::D2,
                    // PERF: vulkan docs recommend using 24 bit depth for better performance
                    format: TextureFormat::Depth32Float,
                    usage,
                    view_formats: &[],
                };

                texture_cache.get(&render_device, descriptor)
            })
            .clone();

        commands.entity(entity).insert(ViewDepthTexture {
            texture: cached_texture.texture,
            view: cached_texture.default_view,
        });
    }
}
