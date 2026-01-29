//! Order Independent Transparency (OIT) for 3d rendering. See [`OrderIndependentTransparencyPlugin`] for more details.

use bevy_app::prelude::*;
use bevy_camera::Camera3d;
use bevy_ecs::{component::*, prelude::*};
use bevy_math::UVec2;
use bevy_platform::time::Instant;
use bevy_reflect::{std_traits::ReflectDefault, Reflect};
use bevy_render::{
    camera::ExtractedCamera,
    extract_component::{ExtractComponent, ExtractComponentPlugin},
    render_resource::{
        BufferUsages, DynamicUniformBuffer, ShaderType, TextureUsages, UninitBufferVec,
    },
    renderer::{RenderDevice, RenderQueue},
    view::Msaa,
    Render, RenderApp, RenderStartup, RenderSystems,
};
use bevy_shader::load_shader_library;
use resolve::OitResolvePlugin;
use tracing::trace;

use crate::{
    core_3d::main_transparent_pass_3d,
    oit::resolve::node::oit_resolve,
    schedule::{Core3d, Core3dSystems},
};

/// Module that defines the necessary systems to resolve the OIT buffer and render it to the screen.
pub mod resolve;

/// Used to identify which camera will use OIT to render transparent meshes
/// and to configure OIT.
// TODO consider supporting multiple OIT techniques like WBOIT, Moment Based OIT,
// depth peeling, stochastic transparency, ray tracing etc.
// This should probably be done by adding an enum to this component.
// We use the same struct to pass on the settings to the drawing shader.
#[derive(Clone, Copy, ExtractComponent, Reflect, ShaderType, Component)]
#[reflect(Clone, Default)]
pub struct OrderIndependentTransparencySettings {
    /// Controls how many fragments will be exactly sorted.
    /// If the scene has more fragments than this, they will be merged approximately.
    /// More sorted fragments is more accurate but will be slower.
    pub sorted_fragment_max_count: u32,
    /// The average fragments per pixel stored in the buffer. This should be bigger enough otherwise the fragments will be discarded.
    /// Higher values increase memory usage.
    pub fragments_per_pixel_average: f32,
    /// Threshold for which fragments will be added to the blending layers.
    /// This can be tweaked to optimize quality / layers count. Higher values will
    /// allow lower number of layers and a better performance, compromising quality.
    pub alpha_threshold: f32,
}

impl Default for OrderIndependentTransparencySettings {
    fn default() -> Self {
        Self {
            sorted_fragment_max_count: 8,
            fragments_per_pixel_average: 4.0,
            alpha_threshold: 0.0,
        }
    }
}

/// A plugin that adds support for Order Independent Transparency (OIT).
/// This can correctly render some scenes that would otherwise have artifacts due to alpha blending, but uses more memory.
///
/// To enable OIT for a camera you need to add the [`OrderIndependentTransparencySettings`] component to it.
///
/// If you want to use OIT for your custom material you need to call `oit_draw(position, color)` in your fragment shader.
/// You also need to make sure that your fragment shader doesn't output any colors.
///
/// # Implementation details
/// This implementation uses 2 passes.
///
/// The first pass constructs a linked list which stores depth and color of all fragments in a big buffer.
/// The linked list capacity can be set with [`OrderIndependentTransparencySettings::fragments_per_pixel_average`].
/// This pass is essentially a forward pass.
///
/// The second pass is a single fullscreen triangle pass that sorts all the fragments then blends them together
/// and outputs the result to the screen.
pub struct OrderIndependentTransparencyPlugin;
impl Plugin for OrderIndependentTransparencyPlugin {
    fn build(&self, app: &mut App) {
        load_shader_library!(app, "oit_draw.wgsl");

        app.add_plugins((
            ExtractComponentPlugin::<OrderIndependentTransparencySettings>::default(),
            OitResolvePlugin,
        ))
        .add_systems(Update, check_msaa);

        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        render_app
            .add_systems(RenderStartup, init_oit_buffers)
            .add_systems(
                Render,
                (
                    configure_camera_depth_usages.in_set(RenderSystems::ManageViews),
                    prepare_oit_buffers.in_set(RenderSystems::PrepareResources),
                ),
            );

        render_app.add_systems(
            Core3d,
            oit_resolve
                .after(main_transparent_pass_3d)
                .in_set(Core3dSystems::MainPass),
        );
    }
}

fn configure_camera_depth_usages(
    mut cameras: Query<
        &mut Camera3d,
        (
            Changed<Camera3d>,
            With<OrderIndependentTransparencySettings>,
        ),
    >,
) {
    for mut camera in &mut cameras {
        camera.depth_texture_usages.0 |= TextureUsages::TEXTURE_BINDING.bits();
    }
}

fn check_msaa(cameras: Query<&Msaa, With<OrderIndependentTransparencySettings>>) {
    for msaa in &cameras {
        if msaa.samples() > 1 {
            panic!("MSAA is not supported when using OrderIndependentTransparency");
        }
    }
}

#[derive(Clone, Copy, ShaderType)]
pub struct OitFragmentNode {
    pub color: u32,
    pub depth_alpha: u32,
    pub next: u32,
}

/// Holds the buffers that contain the data of all OIT layers.
/// We use one big buffer for the entire app. Each camera will reuse it so it will
/// always be the size of the biggest OIT enabled camera.
#[derive(Resource)]
pub struct OitBuffers {
    pub settings: DynamicUniformBuffer<OrderIndependentTransparencySettings>,
    /// The OIT buffers containing color, depth and linked next node for each fragments.
    /// This is essentially used as a 3d array where xy is the screen coordinate and z is
    /// the list of fragments rendered with OIT.
    pub nodes: UninitBufferVec<OitFragmentNode>,
    pub heads: UninitBufferVec<u32>,
    pub atomic_counter: UninitBufferVec<u32>,
}

pub fn init_oit_buffers(mut commands: Commands, render_device: Res<RenderDevice>) {
    // initialize buffers with something so there's a valid binding

    let mut nodes = UninitBufferVec::new(BufferUsages::COPY_DST | BufferUsages::STORAGE);
    nodes.set_label(Some("oit_nodes"));
    nodes.reserve(1, &render_device);

    let mut heads = UninitBufferVec::new(BufferUsages::COPY_DST | BufferUsages::STORAGE);
    heads.set_label(Some("oit_heads"));
    heads.reserve(1, &render_device);

    let mut atomic_counter = UninitBufferVec::new(BufferUsages::COPY_DST | BufferUsages::STORAGE);
    atomic_counter.set_label(Some("oit_atomic_counter"));
    atomic_counter.reserve(1, &render_device);

    let mut settings = DynamicUniformBuffer::default();
    settings.set_label(Some("oit_settings"));

    commands.insert_resource(OitBuffers {
        nodes,
        heads,
        atomic_counter,
        settings,
    });
}

#[derive(Component)]
pub struct OrderIndependentTransparencySettingsOffset {
    pub offset: u32,
}

/// This creates or resizes the oit buffers for each camera.
/// It will always create one big buffer that's as big as the biggest buffer needed.
/// Cameras with smaller viewports or less layers will simply use the big buffer and ignore the rest.
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
    let mut max_size = UVec2::new(0, 0);
    let mut fragments_per_pixel_average = 0f32;
    for (camera, settings) in &cameras {
        let Some(size) = camera.physical_target_size else {
            continue;
        };
        max_size = max_size.max(size);
        fragments_per_pixel_average =
            fragments_per_pixel_average.max(settings.fragments_per_pixel_average);
    }

    // Create or update the heads buffer based on the max size
    let heads_size = (max_size.x * max_size.y) as usize;
    if buffers.heads.capacity() < heads_size {
        let start = Instant::now();
        buffers.heads.reserve(heads_size, &render_device);
        trace!(
            "OIT heads buffer updated in {:.01}ms with total size {} MiB",
            start.elapsed().as_millis(),
            buffers.heads.capacity() * size_of::<u32>() / 1024 / 1024,
        );
    }

    // Create or update the nodes buffer based on the max size
    let nodes_size = ((max_size.x * max_size.y) as f32 * fragments_per_pixel_average) as usize;
    if buffers.nodes.capacity() < nodes_size {
        let start = Instant::now();
        buffers.nodes.reserve(nodes_size, &render_device);
        trace!(
            "OIT nodes buffer updated in {:.01}ms with total size {} MiB",
            start.elapsed().as_millis(),
            buffers.nodes.capacity() * size_of::<OitFragmentNode>() / 1024 / 1024,
        );
    }

    if let Some(mut writer) = buffers.settings.get_writer(
        camera_oit_uniforms.iter().len(),
        &render_device,
        &render_queue,
    ) {
        for (entity, settings) in &camera_oit_uniforms {
            let offset = writer.write(settings);
            commands
                .entity(entity)
                .insert(OrderIndependentTransparencySettingsOffset { offset });
        }
    }
}
