use bevy_app::prelude::*;
use bevy_asset::{load_internal_asset, Handle};
use bevy_ecs::prelude::*;
use bevy_math::UVec2;
use bevy_render::{
    camera::ExtractedCamera,
    extract_component::{ExtractComponent, ExtractComponentPlugin},
    render_graph::{RenderGraphApp, ViewNodeRunner},
    render_resource::{BufferUsages, BufferVec, DynamicUniformBuffer, Shader, TextureUsages},
    renderer::{RenderDevice, RenderQueue},
    view::Msaa,
    Render, RenderApp, RenderSet,
};
use bevy_utils::{tracing::trace, warn_once, Instant};
use resolve::{
    node::{OitResolveNode, OitResolvePass},
    OitResolvePlugin,
};

use crate::core_3d::{
    graph::{Core3d, Node3d},
    Camera3d,
};

/// Module that defines the necesasry systems to resolve the OIT buffer and render it to the screen
pub mod resolve;

/// Shader handle for the shader that draws the transparent meshes to the OIT layers buffer
pub const OIT_DRAW_SHADER_HANDLE: Handle<Shader> = Handle::weak_from_u128(4042527984320512);

/// Used to identify which camera will use OIT to render transparent meshes
/// Alos used to configure OIT
// TODO consider supporting multiple OIT techniques like WBOIT, Moment Based OIT,
// depth peeling, stochastic transparency, ray tracing etc.
// This should probably be done by adding an enum to this component
#[derive(Component, Clone, Copy, ExtractComponent)]
pub struct OrderIndependentTransparencySettings {
    pub layer_count: u8,
}

impl Default for OrderIndependentTransparencySettings {
    fn default() -> Self {
        Self { layer_count: 8 }
    }
}

/// Plugin needed to enable Order Independent Transparency
pub struct OrderIndependentTransparencyPlugin;
impl Plugin for OrderIndependentTransparencyPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        load_internal_asset!(
            app,
            OIT_DRAW_SHADER_HANDLE,
            "oit_draw.wgsl",
            Shader::from_wgsl
        );

        app.add_plugins((
            ExtractComponentPlugin::<OrderIndependentTransparencySettings>::default(),
            OitResolvePlugin,
        ))
        .add_systems(Update, check_msaa)
        .add_systems(Last, configure_depth_texture_usages);

        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        render_app.add_systems(
            Render,
            prepare_oit_buffers.in_set(RenderSet::PrepareResources),
        );

        render_app
            .add_render_graph_node::<ViewNodeRunner<OitResolveNode>>(Core3d, OitResolvePass)
            .add_render_graph_edges(Core3d, (Node3d::MainTransparentPass, OitResolvePass));
    }

    fn finish(&self, app: &mut bevy_app::App) {
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        render_app.init_resource::<OitBuffers>();
    }
}

// WARN This should only happen for cameras with the [`OrderIndependentTransparencySettings`]
// but when multiple cameras are present on the same window
// bevy reuses the same depth texture so we need to set this on all cameras.
fn configure_depth_texture_usages(mut new_cameras: Query<&mut Camera3d, Added<Camera3d>>) {
    for mut camera in &mut new_cameras {
        let mut usages = TextureUsages::from(camera.depth_texture_usages);
        usages |= TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING;
        camera.depth_texture_usages = usages.into();
    }
}

fn check_msaa(cameras: Query<&Msaa, With<OrderIndependentTransparencySettings>>) {
    for msaa in &cameras {
        if msaa.samples() > 1 {
            warn_once!(
                "MSAA should be disabled when using Order Independent Transparency. \
                It will cause some rendering issues on some platform. Consider using another AA method."
            );
        }
    }
}

/// Holds the buffers that contain the data of all OIT layers
/// We use one big buffer for the entire app. Each camaera will reuse it so it will
/// always be the size of the biggest OIT enabled camera.
#[derive(Resource)]
pub struct OitBuffers {
    /// The OIT layers containing depth and color for each fragments
    /// This is essentially used as a 3d array where xy is the screen coordinate and z is
    /// the list of fragments rendered with OIT
    pub layers: BufferVec<UVec2>,
    /// Buffer containing the index of the last layer that was written for each fragment
    pub layer_ids: BufferVec<i32>,
    pub layers_count_uniforms: DynamicUniformBuffer<i32>,
}

impl FromWorld for OitBuffers {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();
        let render_queue = world.resource::<RenderQueue>();

        // initialize buffers with something so there's a valid binding

        let mut layers = BufferVec::new(BufferUsages::COPY_DST | BufferUsages::STORAGE);
        layers.set_label(Some("oit_layers"));
        layers.reserve(1, render_device);
        layers.write_buffer(render_device, render_queue);

        let mut layer_ids = BufferVec::new(BufferUsages::COPY_DST | BufferUsages::STORAGE);
        layer_ids.set_label(Some("oit_layer_ids"));
        layer_ids.reserve(1, render_device);
        layer_ids.write_buffer(render_device, render_queue);

        let mut layers_count_uniforms = DynamicUniformBuffer::default();
        layers_count_uniforms.set_label(Some("oit_layers_count"));

        Self {
            layers,
            layer_ids,
            layers_count_uniforms,
        }
    }
}

#[derive(Component)]
pub struct OitLayersCountOffset {
    pub offset: u32,
}

/// This creates or resizes the oit buffers for each camera
/// It will always create one big buffer that's as big as the biggest buffer needed
/// Cameras with smaller viewports or less layers will simply use the big buffer and ignore the rest
#[allow(clippy::type_complexity)]
pub fn prepare_oit_buffers(
    mut commands: Commands,
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    cameras: Query<
        (&ExtractedCamera, &OrderIndependentTransparencySettings),
        (
            Changed<ExtractedCamera>,
            Changed<OrderIndependentTransparencySettings>,
        ),
    >,
    camera_oit_uniforms: Query<(Entity, &OrderIndependentTransparencySettings)>,
    mut buffers: ResMut<OitBuffers>,
) {
    // Get the max buffer size for any OIT enabled camera
    let mut max_layer_ids_size = usize::MIN;
    let mut max_layers_size = usize::MIN;
    for (camera, settings) in &cameras {
        let Some(size) = camera.physical_target_size else {
            continue;
        };

        let layer_count = settings.layer_count as usize;
        let size = (size.x * size.y) as usize;
        max_layer_ids_size = max_layer_ids_size.max(size);
        max_layers_size = max_layers_size.max(size * layer_count);
    }

    // Create or update the layers buffer based on the max size
    if buffers.layers.capacity() < max_layers_size {
        let start = Instant::now();
        buffers.layers.reserve(max_layers_size, &render_device);
        let remaining = max_layers_size - buffers.layers.capacity();
        for _ in 0..remaining {
            buffers.layers.push(UVec2::ZERO);
        }
        buffers.layers.write_buffer(&render_device, &render_queue);
        trace!(
            "OIT layers buffer updated in {:.01}ms with total size {} MiB",
            start.elapsed().as_millis(),
            buffers.layers.capacity() * std::mem::size_of::<UVec2>() / 1024 / 1024,
        );
    }

    // Create or update the layer_ids buffer based on the max size
    if buffers.layer_ids.capacity() < max_layer_ids_size {
        let start = Instant::now();
        buffers
            .layer_ids
            .reserve(max_layer_ids_size, &render_device);
        let remaining = max_layer_ids_size - buffers.layer_ids.capacity();
        for _ in 0..remaining {
            buffers.layer_ids.push(0);
        }
        buffers
            .layer_ids
            .write_buffer(&render_device, &render_queue);
        trace!(
            "OIT layer ids buffer updated in {:.01}ms with total size {} MiB",
            start.elapsed().as_millis(),
            buffers.layer_ids.capacity() * std::mem::size_of::<UVec2>() / 1024 / 1024,
        );
    }

    if let Some(mut writer) = buffers.layers_count_uniforms.get_writer(
        camera_oit_uniforms.iter().len(),
        &render_device,
        &render_queue,
    ) {
        for (entity, settings) in &camera_oit_uniforms {
            let offset = writer.write(&(settings.layer_count as i32));
            commands
                .entity(entity)
                .insert(OitLayersCountOffset { offset });
        }
    }
}
