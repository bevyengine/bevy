mod main_pass_2d;
mod main_pass_3d;
mod main_pass_driver;

pub use main_pass_2d::*;
pub use main_pass_3d::*;
pub use main_pass_driver::*;

use bevy_app::{App, Plugin};
use bevy_asset::Handle;
use bevy_core::FloatOrd;
use bevy_ecs::{
    prelude::*,
    system::{lifetimeless::SRes, SystemParamItem},
};
use bevy_render2::{
    camera::{ActiveCameras, CameraPlugin},
    color::Color,
    render_graph::{EmptyNode, RenderGraph, RenderGraphError, SlotInfo, SlotType},
    render_phase::{
        sort_phase_system, DrawFunctionId, DrawFunctions, PhaseItem, RenderCommand, RenderPhase,
        TrackedRenderPass,
    },
    render_resource::*,
    renderer::RenderDevice,
    texture::{BevyDefault, Image, TextureCache},
    view::{ExtractedView, ExtractedWindows},
    RenderApp, RenderStage, RenderWorld,
};
use bevy_window::Windows;

/// Resource that configures the clear color
#[derive(Clone, Debug)]
pub struct ClearColor(pub Color);

impl Default for ClearColor {
    fn default() -> Self {
        Self(Color::rgb(0.4, 0.4, 0.4))
    }
}

/// Resource for configuring Multi-Sampled Anti-Aliasing sample count
#[derive(Clone)]
pub struct Msaa {
    pub samples: u32,
}

impl Default for Msaa {
    fn default() -> Self {
        Self { samples: 1 }
    }
}

pub type ExtractedMsaa = Msaa;

// Plugins that contribute to the RenderGraph should use the following label conventions:
// 1. Graph modules should have a NAME, input module, and node module (where relevant)
// 2. The "top level" graph is the plugin module root. Just add things like `pub mod node` directly under the plugin module
// 3. "sub graph" modules should be nested beneath their parent graph module

pub mod node {
    pub const MAIN_PASS_DEPENDENCIES: &str = "main_pass_dependencies";
    pub const MAIN_PASS_DRIVER: &str = "main_pass_driver";
    pub const VIEW: &str = "view";
}

pub mod draw_2d_graph {
    pub const NAME: &str = "draw_2d";
    pub mod input {
        pub const VIEW_ENTITY: &str = "view_entity";
        pub const SAMPLED_COLOR_ATTACHMENT: &str = "sampled_color_attachment";
        pub const RENDER_TARGET: &str = "render_target";
    }
    pub mod node {
        pub const MAIN_PASS: &str = "main_pass";
    }
}

pub mod draw_3d_graph {
    pub const NAME: &str = "draw_3d";
    pub mod input {
        pub const VIEW_ENTITY: &str = "view_entity";
        pub const SAMPLED_COLOR_ATTACHMENT: &str = "sampled_color_attachment";
        pub const RENDER_TARGET: &str = "render_target";
        pub const DEPTH: &str = "depth";
    }
    pub mod node {
        pub const MAIN_PASS: &str = "main_pass";
    }
}

#[derive(Default)]
pub struct CorePipelinePlugin;

impl Plugin for CorePipelinePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ClearColor>();
        let msaa = if let Some(msaa) = app.world.get_resource::<Msaa>() {
            msaa.clone()
        } else {
            let msaa = Msaa { samples: 1 };
            app.world.insert_resource(msaa.clone());
            msaa
        };

        let render_app = app.sub_app(RenderApp);
        render_app
            .init_resource::<DrawFunctions<Transparent2d>>()
            .init_resource::<DrawFunctions<Transparent3d>>()
            .insert_resource::<ExtractedMsaa>(msaa.clone())
            .add_system_to_stage(RenderStage::Extract, extract_clear_color)
            .add_system_to_stage(RenderStage::Extract, extract_msaa)
            .add_system_to_stage(RenderStage::Extract, extract_core_pipeline_camera_phases)
            .add_system_to_stage(RenderStage::Prepare, prepare_windows_msaa)
            .add_system_to_stage(RenderStage::Prepare, prepare_core_views_system)
            .add_system_to_stage(RenderStage::PhaseSort, sort_phase_system::<Transparent2d>)
            .add_system_to_stage(RenderStage::PhaseSort, sort_phase_system::<Transparent3d>);

        let pass_node_2d = MainPass2dNode::new(&mut render_app.world);
        let pass_node_3d = MainPass3dNode::new(&mut render_app.world);
        let mut graph = render_app.world.get_resource_mut::<RenderGraph>().unwrap();

        let mut draw_2d_graph = RenderGraph::default();
        draw_2d_graph.add_node(draw_2d_graph::node::MAIN_PASS, pass_node_2d);
        let input_node_id = draw_2d_graph.set_input(vec![
            SlotInfo::new(draw_2d_graph::input::VIEW_ENTITY, SlotType::Entity),
            SlotInfo::new(
                draw_2d_graph::input::SAMPLED_COLOR_ATTACHMENT,
                SlotType::TextureView,
            ),
            SlotInfo::new(draw_2d_graph::input::RENDER_TARGET, SlotType::TextureView),
        ]);
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
        let input_node_id = draw_3d_graph.set_input(vec![
            SlotInfo::new(draw_3d_graph::input::VIEW_ENTITY, SlotType::Entity),
            SlotInfo::new(
                draw_3d_graph::input::SAMPLED_COLOR_ATTACHMENT,
                SlotType::TextureView,
            ),
            SlotInfo::new(draw_3d_graph::input::RENDER_TARGET, SlotType::TextureView),
            SlotInfo::new(draw_3d_graph::input::DEPTH, SlotType::TextureView),
        ]);
        draw_3d_graph
            .add_slot_edge(
                input_node_id,
                draw_3d_graph::input::VIEW_ENTITY,
                draw_3d_graph::node::MAIN_PASS,
                MainPass3dNode::IN_VIEW,
            )
            .unwrap();
        draw_3d_graph
            .add_slot_edge(
                input_node_id,
                draw_3d_graph::input::DEPTH,
                draw_3d_graph::node::MAIN_PASS,
                MainPass3dNode::IN_DEPTH,
            )
            .unwrap();
        graph.add_sub_graph(draw_3d_graph::NAME, draw_3d_graph);

        configure_graph_msaa(msaa.samples, &mut *graph);

        graph.add_node(node::MAIN_PASS_DEPENDENCIES, EmptyNode);
        graph.add_node(node::MAIN_PASS_DRIVER, MainPassDriverNode);
        graph
            .add_node_edge(node::MAIN_PASS_DEPENDENCIES, node::MAIN_PASS_DRIVER)
            .unwrap();
    }
}

pub struct Transparent2d {
    pub sort_key: Handle<Image>,
    pub entity: Entity,
    pub pipeline: CachedPipelineId,
    pub draw_function: DrawFunctionId,
}

impl PhaseItem for Transparent2d {
    type SortKey = Handle<Image>;

    #[inline]
    fn sort_key(&self) -> Self::SortKey {
        self.sort_key.clone_weak()
    }

    #[inline]
    fn draw_function(&self) -> DrawFunctionId {
        self.draw_function
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

pub struct SetItemPipeline;
impl RenderCommand<Transparent3d> for SetItemPipeline {
    type Param = SRes<RenderPipelineCache>;
    #[inline]
    fn render<'w>(
        _view: Entity,
        item: &Transparent3d,
        pipeline_cache: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) {
        let pipeline = pipeline_cache
            .into_inner()
            .get_state(item.pipeline)
            .unwrap();
        pass.set_render_pipeline(&pipeline);
    }
}

impl RenderCommand<Transparent2d> for SetItemPipeline {
    type Param = SRes<RenderPipelineCache>;
    #[inline]
    fn render<'w>(
        _view: Entity,
        item: &Transparent2d,
        pipeline_cache: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) {
        let pipeline = pipeline_cache
            .into_inner()
            .get_state(item.pipeline)
            .unwrap();
        pass.set_render_pipeline(&pipeline);
    }
}

pub struct ViewDepthTexture {
    pub texture: Texture,
    pub view: TextureView,
}

pub fn extract_clear_color(clear_color: Res<ClearColor>, mut render_world: ResMut<RenderWorld>) {
    // If the clear color has changed
    if clear_color.is_changed() {
        // Update the clear color resource in the render world
        render_world.insert_resource(clear_color.clone())
    }
}

pub fn extract_msaa(msaa: Res<Msaa>, windows: Res<Windows>, mut render_world: ResMut<RenderWorld>) {
    // NOTE: windows.is_changed() handles cases where a window was resized
    if msaa.is_changed() || windows.is_changed() {
        render_world.insert_resource::<ExtractedMsaa>(msaa.clone())
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
            commands
                .get_or_spawn(entity)
                .insert(RenderPhase::<Transparent3d>::default());
        }
    }
}

pub fn prepare_core_views_system(
    mut commands: Commands,
    mut texture_cache: ResMut<TextureCache>,
    msaa: Res<ExtractedMsaa>,
    render_device: Res<RenderDevice>,
    views: Query<(Entity, &ExtractedView), With<RenderPhase<Transparent3d>>>,
) {
    for (entity, view) in views.iter() {
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

pub fn prepare_windows_msaa(
    msaa: Res<ExtractedMsaa>,
    mut windows: ResMut<ExtractedWindows>,
    mut texture_cache: ResMut<TextureCache>,
    render_device: Res<RenderDevice>,
    mut render_graph: ResMut<RenderGraph>,
) {
    // NOTE: msaa.is_changed() is true also when the app world Windows is changed
    //       and this handles window resizing
    if msaa.is_added() || msaa.is_changed() {
        for window in windows.windows.values_mut() {
            if msaa.samples > 1 {
                let cached_texture = texture_cache.get(
                    &render_device,
                    TextureDescriptor {
                        label: Some("sampled_color_attachment_texture"),
                        size: Extent3d {
                            width: window.physical_width,
                            height: window.physical_height,
                            depth_or_array_layers: 1,
                        },
                        mip_level_count: 1,
                        sample_count: msaa.samples,
                        dimension: TextureDimension::D2,
                        format: TextureFormat::bevy_default(),
                        usage: TextureUsages::RENDER_ATTACHMENT,
                    },
                );
                let texture_view = cached_texture.texture.create_view(&TextureViewDescriptor {
                    label: Some("sampled_color_attachment"),
                    format: None,
                    dimension: Some(TextureViewDimension::D2),
                    aspect: TextureAspect::All,
                    base_mip_level: 0,
                    mip_level_count: None,
                    base_array_layer: 0,
                    array_layer_count: None,
                });
                window.sampled_color_attachment_texture = Some(cached_texture.texture);
                window.sampled_color_attachment = Some(texture_view);
            }

            configure_graph_msaa(msaa.samples, &mut *render_graph);
        }
    }
}

pub fn configure_graph_msaa(msaa_samples: u32, render_graph: &mut RenderGraph) {
    if msaa_samples > 1 {
        // Scope to drop the mutable borrow
        {
            let draw_2d_graph = render_graph.get_sub_graph_mut(draw_2d_graph::NAME).unwrap();
            // Remove msaa_samples == 1 edges
            // NOTE: Allowing EdgeDoesNotExist as it's basically saying there is nothing to do.
            let input_node_id = draw_2d_graph.input_node().unwrap().id;
            match draw_2d_graph.remove_slot_edge(
                input_node_id,
                draw_2d_graph::input::RENDER_TARGET,
                draw_2d_graph::node::MAIN_PASS,
                MainPass2dNode::IN_COLOR_ATTACHMENT,
            ) {
                Ok(_) | Err(RenderGraphError::EdgeDoesNotExist(_)) => {}
                Err(e) => panic!("{:?}", e),
            }
            // NOTE: This is not used but satisfies the graph connections
            match draw_2d_graph.remove_slot_edge(
                input_node_id,
                draw_2d_graph::input::SAMPLED_COLOR_ATTACHMENT,
                draw_2d_graph::node::MAIN_PASS,
                MainPass2dNode::IN_COLOR_RESOLVE_TARGET,
            ) {
                Ok(_) | Err(RenderGraphError::EdgeDoesNotExist(_)) => {}
                Err(e) => panic!("{:?}", e),
            }
            // Add msaa_samples > 1 edges
            // NOTE: Allowing EdgeAlreadyExists as it's basically saying there is nothing to do.
            match draw_2d_graph.add_slot_edge(
                input_node_id,
                draw_2d_graph::input::SAMPLED_COLOR_ATTACHMENT,
                draw_2d_graph::node::MAIN_PASS,
                MainPass2dNode::IN_COLOR_ATTACHMENT,
            ) {
                Ok(_) | Err(RenderGraphError::EdgeAlreadyExists(_)) => {}
                Err(e) => panic!("{:?}", e),
            }
            match draw_2d_graph.add_slot_edge(
                input_node_id,
                draw_2d_graph::input::RENDER_TARGET,
                draw_2d_graph::node::MAIN_PASS,
                MainPass2dNode::IN_COLOR_RESOLVE_TARGET,
            ) {
                Ok(_) | Err(RenderGraphError::EdgeAlreadyExists(_)) => {}
                Err(e) => panic!("{:?}", e),
            }
        }

        // Scope to drop the mutable borrow
        {
            let draw_3d_graph = render_graph.get_sub_graph_mut(draw_3d_graph::NAME).unwrap();
            // Remove msaa_samples == 1 edges
            // NOTE: Allowing EdgeDoesNotExist as it's basically saying there is nothing to do.
            let input_node_id = draw_3d_graph.input_node().unwrap().id;
            match draw_3d_graph.remove_slot_edge(
                input_node_id,
                draw_3d_graph::input::RENDER_TARGET,
                draw_3d_graph::node::MAIN_PASS,
                MainPass3dNode::IN_COLOR_ATTACHMENT,
            ) {
                Ok(_) | Err(RenderGraphError::EdgeDoesNotExist(_)) => {}
                Err(e) => panic!("{:?}", e),
            }
            // NOTE: This is not used but satisfies the graph connections
            match draw_3d_graph.remove_slot_edge(
                input_node_id,
                draw_3d_graph::input::SAMPLED_COLOR_ATTACHMENT,
                draw_3d_graph::node::MAIN_PASS,
                MainPass3dNode::IN_COLOR_RESOLVE_TARGET,
            ) {
                Ok(_) | Err(RenderGraphError::EdgeDoesNotExist(_)) => {}
                Err(e) => panic!("{:?}", e),
            }
            // Add msaa_samples > 1 edges
            // NOTE: Allowing EdgeAlreadyExists as it's basically saying there is nothing to do.
            match draw_3d_graph.add_slot_edge(
                input_node_id,
                draw_3d_graph::input::SAMPLED_COLOR_ATTACHMENT,
                draw_3d_graph::node::MAIN_PASS,
                MainPass3dNode::IN_COLOR_ATTACHMENT,
            ) {
                Ok(_) | Err(RenderGraphError::EdgeAlreadyExists(_)) => {}
                Err(e) => panic!("{:?}", e),
            }
            match draw_3d_graph.add_slot_edge(
                input_node_id,
                draw_3d_graph::input::RENDER_TARGET,
                draw_3d_graph::node::MAIN_PASS,
                MainPass3dNode::IN_COLOR_RESOLVE_TARGET,
            ) {
                Ok(_) | Err(RenderGraphError::EdgeAlreadyExists(_)) => {}
                Err(e) => panic!("{:?}", e),
            }
        }
    } else {
        // Scope to drop the mutable borrow
        {
            let draw_2d_graph = render_graph.get_sub_graph_mut(draw_2d_graph::NAME).unwrap();
            // Remove msaa_samples == 1 edges
            // NOTE: Allowing EdgeDoesNotExist as it's basically saying there is nothing to do.
            let input_node_id = draw_2d_graph.input_node().unwrap().id;
            match draw_2d_graph.remove_slot_edge(
                input_node_id,
                draw_2d_graph::input::SAMPLED_COLOR_ATTACHMENT,
                draw_2d_graph::node::MAIN_PASS,
                MainPass2dNode::IN_COLOR_ATTACHMENT,
            ) {
                Ok(_) | Err(RenderGraphError::EdgeDoesNotExist(_)) => {}
                Err(e) => panic!("{:?}", e),
            }
            match draw_2d_graph.remove_slot_edge(
                input_node_id,
                draw_2d_graph::input::RENDER_TARGET,
                draw_2d_graph::node::MAIN_PASS,
                MainPass2dNode::IN_COLOR_RESOLVE_TARGET,
            ) {
                Ok(_) | Err(RenderGraphError::EdgeDoesNotExist(_)) => {}
                Err(e) => panic!("{:?}", e),
            }
            // Add msaa_samples == 1 edges
            // NOTE: Allowing EdgeAlreadyExists as it's basically saying there is nothing to do.
            match draw_2d_graph.add_slot_edge(
                input_node_id,
                draw_2d_graph::input::RENDER_TARGET,
                draw_2d_graph::node::MAIN_PASS,
                MainPass2dNode::IN_COLOR_ATTACHMENT,
            ) {
                Ok(_) | Err(RenderGraphError::EdgeAlreadyExists(_)) => {}
                Err(e) => panic!("{:?}", e),
            }
            // NOTE: This is not used but satisfies the graph connections
            match draw_2d_graph.add_slot_edge(
                input_node_id,
                draw_2d_graph::input::SAMPLED_COLOR_ATTACHMENT,
                draw_2d_graph::node::MAIN_PASS,
                MainPass2dNode::IN_COLOR_RESOLVE_TARGET,
            ) {
                Ok(_) | Err(RenderGraphError::EdgeAlreadyExists(_)) => {}
                Err(e) => panic!("{:?}", e),
            }
        }

        // Scope to drop the mutable borrow
        {
            let draw_3d_graph = render_graph.get_sub_graph_mut(draw_3d_graph::NAME).unwrap();
            // Remove msaa_samples == 1 edges
            // NOTE: Allowing EdgeDoesNotExist as it's basically saying there is nothing to do.
            let input_node_id = draw_3d_graph.input_node().unwrap().id;
            match draw_3d_graph.remove_slot_edge(
                input_node_id,
                draw_3d_graph::input::SAMPLED_COLOR_ATTACHMENT,
                draw_3d_graph::node::MAIN_PASS,
                MainPass3dNode::IN_COLOR_ATTACHMENT,
            ) {
                Ok(_) | Err(RenderGraphError::EdgeDoesNotExist(_)) => {}
                Err(e) => panic!("{:?}", e),
            }
            match draw_3d_graph.remove_slot_edge(
                input_node_id,
                draw_3d_graph::input::RENDER_TARGET,
                draw_3d_graph::node::MAIN_PASS,
                MainPass3dNode::IN_COLOR_RESOLVE_TARGET,
            ) {
                Ok(_) | Err(RenderGraphError::EdgeDoesNotExist(_)) => {}
                Err(e) => panic!("{:?}", e),
            }
            // Add msaa_samples == 1 edges
            // NOTE: Allowing EdgeAlreadyExists as it's basically saying there is nothing to do.
            match draw_3d_graph.add_slot_edge(
                input_node_id,
                draw_3d_graph::input::RENDER_TARGET,
                draw_3d_graph::node::MAIN_PASS,
                MainPass3dNode::IN_COLOR_ATTACHMENT,
            ) {
                Ok(_) | Err(RenderGraphError::EdgeAlreadyExists(_)) => {}
                Err(e) => panic!("{:?}", e),
            }
            // NOTE: This is not used but satisfies the graph connections
            match draw_3d_graph.add_slot_edge(
                input_node_id,
                draw_3d_graph::input::SAMPLED_COLOR_ATTACHMENT,
                draw_3d_graph::node::MAIN_PASS,
                MainPass3dNode::IN_COLOR_RESOLVE_TARGET,
            ) {
                Ok(_) | Err(RenderGraphError::EdgeAlreadyExists(_)) => {}
                Err(e) => panic!("{:?}", e),
            }
        }
    }
}
