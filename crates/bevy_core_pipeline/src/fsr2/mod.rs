use crate::{
    core_3d::MainPass3dTexture,
    prelude::Camera3d,
    prepass::{DepthPrepass, VelocityPrepass, ViewPrepassTextures},
};
use bevy_app::{App, Plugin};
use bevy_ecs::{
    prelude::{Bundle, Component, Entity},
    query::{QueryState, With},
    schedule::IntoSystemDescriptor,
    system::{Commands, Query, Res, Resource},
    world::World,
};
use bevy_math::UVec2;
use bevy_render::{
    prelude::{Camera, Projection},
    render_graph::{Node, NodeRunError, RenderGraph, RenderGraphContext, SlotInfo, SlotType},
    renderer::{RenderAdapter, RenderContext, RenderDevice},
    texture::CachedTexture,
    view::{Msaa, ViewTarget},
    Extract, RenderApp, RenderStage,
};
use bevy_time::Time;
use bevy_winit::WinitWindows;
use fsr2_wgpu::{
    Fsr2Context, Fsr2Exposure, Fsr2InitializationFlags, Fsr2ReactiveMask, Fsr2RenderParameters,
    Fsr2Texture,
};
use std::sync::Mutex;

pub use fsr2_wgpu::Fsr2QualityMode;

pub struct Fsr2Plugin {
    pub hdr: bool,
}

impl Plugin for Fsr2Plugin {
    fn build(&self, app: &mut App) {
        if app.get_sub_app_mut(RenderApp).is_err() {
            return;
        }

        app.insert_resource(Msaa { samples: 1 });

        let max_resolution = max_monitor_size(app);

        let mut initialization_flags = Fsr2InitializationFlags::AUTO_EXPOSURE
            | Fsr2InitializationFlags::INFINITE_DEPTH
            | Fsr2InitializationFlags::INVERTED_DEPTH;
        if self.hdr {
            initialization_flags |= Fsr2InitializationFlags::HIGH_DYNAMIC_RANGE;
        }

        let render_app = app.get_sub_app_mut(RenderApp).unwrap();

        let fsr2_node = Fsr2Node::new(&mut render_app.world);
        let mut graph = render_app.world.resource_mut::<RenderGraph>();
        let draw_3d_graph = graph
            .get_sub_graph_mut(crate::core_3d::graph::NAME)
            .unwrap();
        draw_3d_graph.add_node(crate::core_3d::graph::node::FSR2, fsr2_node);
        draw_3d_graph.add_slot_edge(
            draw_3d_graph.input_node().id,
            crate::core_3d::graph::input::VIEW_ENTITY,
            crate::core_3d::graph::node::FSR2,
            Fsr2Node::IN_VIEW,
        );
        // MAIN_PASS -> FSR2 -> BLOOM / TONEMAPPING
        draw_3d_graph.add_node_edge(
            crate::core_3d::graph::node::MAIN_PASS,
            crate::core_3d::graph::node::FSR2,
        );
        draw_3d_graph.add_node_edge(
            crate::core_3d::graph::node::FSR2,
            crate::core_3d::graph::node::BLOOM,
        );
        draw_3d_graph.add_node_edge(
            crate::core_3d::graph::node::FSR2,
            crate::core_3d::graph::node::TONEMAPPING,
        );

        let fsr2_context = Fsr2Context::new(
            render_app.world.resource::<RenderDevice>().wgpu_device(),
            max_resolution,
            max_resolution,
            initialization_flags,
        );

        render_app
            .insert_resource(Fsr2ContextWrapper {
                context: Mutex::new(fsr2_context),
                hdr: self.hdr,
            })
            .add_system_to_stage(RenderStage::Extract, extract_fsr2_settings)
            .add_system_to_stage(RenderStage::Prepare, prepare_render_resolution.at_start());
    }
}

#[derive(Resource)] // TODO: Remove resource, make this a per-view component (using Arc to extract?)
struct Fsr2ContextWrapper {
    context: Mutex<Fsr2Context>,
    hdr: bool,
}

#[derive(Bundle)]
pub struct Fsr2Bundle {
    pub settings: Fsr2Settings,
    pub depth_prepass: DepthPrepass,
    pub velocity_prepass: VelocityPrepass,
}

#[derive(Component, Clone)]
pub struct Fsr2Settings {
    pub quality_mode: Fsr2QualityMode,
    pub sharpness: f32,
    pub reset: bool,
}

fn extract_fsr2_settings(
    mut commands: Commands,
    fsr2_context: Res<Fsr2ContextWrapper>,
    query: Extract<
        Query<
            (Entity, &Fsr2Settings, &Camera, &Projection),
            (With<Camera3d>, With<DepthPrepass>, With<VelocityPrepass>),
        >,
    >,
) {
    for (entity, fsr2_settings, camera, camera_projection) in &query {
        let perspective_projection = matches!(camera_projection, Projection::Perspective(_));
        if perspective_projection && camera.hdr == fsr2_context.hdr {
            commands.get_or_spawn(entity).insert(fsr2_settings.clone());
        }
    }
}

fn prepare_render_resolution(
    fsr2_context: Res<Fsr2ContextWrapper>,
    mut query: Query<(&mut Camera3d, &Fsr2Settings)>,
) {
    for (mut camera, fsr2_settings) in &mut query {
        camera.render_resolution = Some(
            fsr2_context
                .context
                .lock()
                .unwrap()
                .get_suggested_input_resolution(fsr2_settings.quality_mode),
        );
    }
}

struct Fsr2Node {
    view_query: QueryState<(
        &'static Fsr2Settings,
        &'static Camera3d,
        &'static Projection,
        &'static ViewTarget,
        &'static MainPass3dTexture,
        &'static ViewPrepassTextures,
    )>,
}

impl Fsr2Node {
    const IN_VIEW: &'static str = "view";

    fn new(world: &mut World) -> Self {
        Self {
            view_query: QueryState::new(world),
        }
    }
}

impl Node for Fsr2Node {
    fn input(&self) -> Vec<SlotInfo> {
        vec![SlotInfo::new(Self::IN_VIEW, SlotType::Entity)]
    }

    fn update(&mut self, world: &mut World) {
        self.view_query.update_archetypes(world);
    }

    fn run(
        &self,
        graph: &mut RenderGraphContext,
        render_context: &mut RenderContext,
        world: &World,
    ) -> Result<(), NodeRunError> {
        #[cfg(feature = "trace")]
        let _fsr2_span = info_span!("fsr2").entered();

        let view_entity = graph.get_input_entity(Self::IN_VIEW)?;
        let time = world.resource::<Time>();
        let render_adapter = world.resource::<RenderAdapter>();
        let mut fsr2_context = world
            .resource::<Fsr2ContextWrapper>()
            .context
            .lock()
            .unwrap();
        let Ok((
            fsr2_settings,
            camera_3d,
            Projection::Perspective(camera_projection),
            view_target,
            main_pass_3d_texture,
            prepass_textures
        )) = self.view_query.get_manual(world, view_entity) else { return Ok(()) };

        let render_resolution = camera_3d.render_resolution.unwrap();
        fsr2_context.render(Fsr2RenderParameters {
            color: fsr2_texture(&main_pass_3d_texture.texture),
            depth: fsr2_texture(prepass_textures.depth.as_ref().unwrap()),
            motion_vectors: fsr2_texture(prepass_textures.velocity.as_ref().unwrap()),
            motion_vector_scale: Some(render_resolution.as_vec2()),
            exposure: Fsr2Exposure::AutoExposure,
            reactive_mask: Fsr2ReactiveMask::NoMask, // TODO: Auto
            transparency_and_composition_mask: None,
            output: fsr2_texture(view_target.main_texture()),
            input_resolution: render_resolution,
            sharpness: fsr2_settings.sharpness,
            frame_delta_time: time.delta(),
            reset: fsr2_settings.reset,
            camera_near: camera_projection.near,
            camera_far: None,
            camera_fov_angle_vertical: camera_projection.fov,
            jitter_offset: todo!("jitter offset"),
            adapter: render_adapter,
            command_encoder: &mut render_context.command_encoder,
        });

        Ok(())
    }
}

fn fsr2_texture(texture: &CachedTexture) -> Fsr2Texture {
    Fsr2Texture {
        texture: &texture.texture,
        view: &texture.default_view,
    }
}

fn max_monitor_size(app: &App) -> UVec2 {
    let mut max_resolution = UVec2::ZERO;
    for monitor in app
        .world
        .get_non_send_resource::<WinitWindows>()
        .unwrap()
        .windows
        .values()
        .next()
        .unwrap()
        .available_monitors()
    {
        let monitor_size = monitor.size();
        max_resolution.x = max_resolution.x.max(monitor_size.width);
        max_resolution.y = max_resolution.y.max(monitor_size.height);
    }
    max_resolution
}
