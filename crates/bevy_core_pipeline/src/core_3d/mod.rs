mod camera_3d;
mod main_pass_3d_node;

pub mod graph {
    pub const NAME: &str = "core_3d";
    pub mod input {
        pub const VIEW_ENTITY: &str = "view_entity";
    }
    pub mod node {
        pub const MAIN_PASS: &str = "main_pass";
    }
}

pub use camera_3d::*;
pub use main_pass_3d_node::*;

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
    RenderApp, RenderStage,
};
use bevy_utils::{FloatOrd, HashMap};

pub struct Core3dPlugin;

impl Plugin for Core3dPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<Camera3d>()
            .add_plugin(ExtractComponentPlugin::<Camera3d>::default());

        let render_app = match app.get_sub_app_mut(RenderApp) {
            Ok(render_app) => render_app,
            Err(_) => return,
        };

        render_app
            .init_resource::<DrawFunctions<Opaque3d>>()
            .init_resource::<DrawFunctions<AlphaMask3d>>()
            .init_resource::<DrawFunctions<HashedAlpha3d>>()
            .init_resource::<DrawFunctions<Transparent3d>>()
            .add_system_to_stage(RenderStage::Extract, extract_core_3d_camera_phases)
            .add_system_to_stage(RenderStage::Prepare, prepare_core_3d_depth_textures)
            .add_system_to_stage(RenderStage::PhaseSort, sort_phase_system::<Opaque3d>)
            .add_system_to_stage(RenderStage::PhaseSort, sort_phase_system::<AlphaMask3d>)
            .add_system_to_stage(RenderStage::PhaseSort, sort_phase_system::<HashedAlpha3d>)
            .add_system_to_stage(RenderStage::PhaseSort, sort_phase_system::<Transparent3d>);

        let pass_node_3d = MainPass3dNode::new(&mut render_app.world);
        let mut graph = render_app.world.resource_mut::<RenderGraph>();

        let mut draw_3d_graph = RenderGraph::default();
        draw_3d_graph.add_node(graph::node::MAIN_PASS, pass_node_3d);
        let input_node_id = draw_3d_graph.set_input(vec![SlotInfo::new(
            graph::input::VIEW_ENTITY,
            SlotType::Entity,
        )]);
        draw_3d_graph
            .add_slot_edge(
                input_node_id,
                graph::input::VIEW_ENTITY,
                graph::node::MAIN_PASS,
                MainPass3dNode::IN_VIEW,
            )
            .unwrap();
        graph.add_sub_graph(graph::NAME, draw_3d_graph);
    }
}

/// Object that is rendered in `main_opaque_pass_3d`.
pub struct Opaque3d {
    /// For front-to-back sorting.
    pub distance: f32,
    /// ID of pipeline specialized for `AlphaMode::Opaque` material and mesh.
    pub pipeline: CachedRenderPipelineId,
    /// [Entity] associated with rendered object.
    pub entity: Entity,
    /// [DrawFunctionId] from [DrawFunctions<Opaque3d>] for `SpecializedMaterial`.
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

impl CachedRenderPipelinePhaseItem for Opaque3d {
    #[inline]
    fn cached_pipeline(&self) -> CachedRenderPipelineId {
        self.pipeline
    }
}

/// Object that is rendered in `main_alpha_mask_pass_3d`.
pub struct AlphaMask3d {
    /// For front-to-back sorting.
    pub distance: f32,
    /// ID of pipeline specialized for `AlphaMode::Mask(f32)` material and mesh.
    pub pipeline: CachedRenderPipelineId,
    /// [Entity] associated with rendered object.
    pub entity: Entity,
    /// [DrawFunctionId] from [DrawFunctions<AlphaMask3d>] for `SpecializedMaterial`.
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

impl CachedRenderPipelinePhaseItem for AlphaMask3d {
    #[inline]
    fn cached_pipeline(&self) -> CachedRenderPipelineId {
        self.pipeline
    }
}

/// Object that is rendered in `main_hashed_alpha_phase_3d`.
pub struct HashedAlpha3d {
    /// For front-to-back sorting.
    pub distance: f32,
    /// ID of pipeline specialized for `AlphaMode::Hashed` material and mesh.
    pub pipeline: CachedRenderPipelineId,
    /// [Entity] associated with rendered object.
    pub entity: Entity,
    /// [DrawFunctionId] from [DrawFunctions<AlphaMask3d>] for `SpecializedMaterial`.
    pub draw_function: DrawFunctionId,
}

impl PhaseItem for HashedAlpha3d {
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

impl EntityPhaseItem for HashedAlpha3d {
    #[inline]
    fn entity(&self) -> Entity {
        self.entity
    }
}

impl CachedRenderPipelinePhaseItem for HashedAlpha3d {
    #[inline]
    fn cached_pipeline(&self) -> CachedRenderPipelineId {
        self.pipeline
    }
}

/// Object that is rendered in `main_transparent_pass_3d`.
pub struct Transparent3d {
    /// For back-to-front sorting.
    pub distance: f32,
    /// ID of pipeline specialized for `AlphaMode::Blend` material and mesh.
    pub pipeline: CachedRenderPipelineId,
    /// [Entity] associated with rendered object.
    pub entity: Entity,
    /// [DrawFunctionId] from [DrawFunctions<Transparent3d>] for `SpecializedMaterial`.
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

impl CachedRenderPipelinePhaseItem for Transparent3d {
    #[inline]
    fn cached_pipeline(&self) -> CachedRenderPipelineId {
        self.pipeline
    }
}

pub fn extract_core_3d_camera_phases(
    mut commands: Commands,
    cameras_3d: Query<(Entity, &Camera), With<Camera3d>>,
) {
    for (entity, camera) in cameras_3d.iter() {
        if camera.is_active {
            commands.get_or_spawn(entity).insert_bundle((
                RenderPhase::<Opaque3d>::default(),
                RenderPhase::<AlphaMask3d>::default(),
                RenderPhase::<HashedAlpha3d>::default(),
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
            With<RenderPhase<HashedAlpha3d>>,
            With<RenderPhase<Transparent3d>>,
        ),
    >,
) {
    let mut textures = HashMap::default();
    for (entity, camera) in views_3d.iter() {
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
