//! Order Independent Transparency (OIT) for 3d rendering. See [`OrderIndependentTransparencyPlugin`] for more details.

use bevy_app::prelude::*;
use bevy_camera::{Camera, Camera3d};
use bevy_ecs::{component::*, lifecycle::ComponentHook, prelude::*};
use bevy_math::UVec2;
use bevy_platform::collections::HashSet;
use bevy_platform::time::Instant;
use bevy_reflect::{std_traits::ReflectDefault, Reflect};
use bevy_render::{
    camera::ExtractedCamera,
    extract_component::{ExtractComponent, ExtractComponentPlugin},
    render_graph::{RenderGraphExt, ViewNodeRunner},
    render_resource::{BufferUsages, BufferVec, DynamicUniformBuffer, ShaderType, TextureUsages},
    renderer::{RenderDevice, RenderQueue},
    view::Msaa,
    Render, RenderApp, RenderStartup, RenderSystems,
};
use bevy_shader::load_shader_library;
use bevy_window::PrimaryWindow;
use resolve::{
    node::{OitResolveNode, OitResolvePass},
    OitResolvePlugin,
};
use tracing::{trace, warn};

use crate::core_3d::graph::{Core3d, Node3d};

/// Module that defines the necessary systems to resolve the OIT buffer and render it to the screen.
pub mod resolve;

/// Used to identify which camera will use OIT to render transparent meshes
/// and to configure OIT.
// TODO consider supporting multiple OIT techniques like WBOIT, Moment Based OIT,
// depth peeling, stochastic transparency, ray tracing etc.
// This should probably be done by adding an enum to this component.
// We use the same struct to pass on the settings to the drawing shader.
#[derive(Clone, Copy, ExtractComponent, Reflect, ShaderType)]
#[reflect(Clone, Default)]
pub struct OrderIndependentTransparencySettings {
    /// Controls how many layers will be used to compute the blending.
    /// The more layers you use the more memory it will use but it will also give better results.
    /// 8 is generally recommended, going above 32 is probably not worth it in the vast majority of cases
    pub layer_count: i32,
    /// Threshold for which fragments will be added to the blending layers.
    /// This can be tweaked to optimize quality / layers count. Higher values will
    /// allow lower number of layers and a better performance, compromising quality.
    pub alpha_threshold: f32,
}

impl Default for OrderIndependentTransparencySettings {
    fn default() -> Self {
        Self {
            layer_count: 8,
            alpha_threshold: 0.0,
        }
    }
}

// OrderIndependentTransparencySettings is also a Component. We explicitly implement the trait so
// we can hook on_add to issue a warning in case `layer_count` is seemingly too high.
impl Component for OrderIndependentTransparencySettings {
    const STORAGE_TYPE: StorageType = StorageType::SparseSet;
    type Mutability = Mutable;

    fn on_add() -> Option<ComponentHook> {
        Some(|world, context| {
            if let Some(value) = world.get::<OrderIndependentTransparencySettings>(context.entity)
                && value.layer_count > 32
            {
                warn!("{}OrderIndependentTransparencySettings layer_count set to {} might be too high.",
                        context.caller.map(|location|format!("{location}: ")).unwrap_or_default(),
                        value.layer_count
                    );
            }
        })
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
/// The first pass writes the depth and color of all the fragments to a big buffer.
/// The buffer contains N layers for each pixel, where N can be set with [`OrderIndependentTransparencySettings::layer_count`].
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
        .add_systems(Update, check_msaa)
        .add_systems(Last, configure_depth_texture_usages);

        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        render_app
            .add_systems(RenderStartup, init_oit_buffers)
            .add_systems(
                Render,
                prepare_oit_buffers.in_set(RenderSystems::PrepareResources),
            );

        render_app
            .add_render_graph_node::<ViewNodeRunner<OitResolveNode>>(Core3d, OitResolvePass)
            .add_render_graph_edges(
                Core3d,
                (
                    Node3d::MainTransparentPass,
                    OitResolvePass,
                    Node3d::EndMainPass,
                ),
            );
    }
}

// WARN This should only happen for cameras with the [`OrderIndependentTransparencySettings`] component
// but when multiple cameras are present on the same window
// bevy reuses the same depth texture so we need to set this on all cameras with the same render target.
fn configure_depth_texture_usages(
    p: Query<Entity, With<PrimaryWindow>>,
    cameras: Query<(&Camera, Has<OrderIndependentTransparencySettings>)>,
    mut new_cameras: Query<(&mut Camera3d, &Camera), Added<Camera3d>>,
) {
    if new_cameras.is_empty() {
        return;
    }

    // Find all the render target that potentially uses OIT
    let primary_window = p.single().ok();
    let mut render_target_has_oit = <HashSet<_>>::default();
    for (camera, has_oit) in &cameras {
        if has_oit {
            render_target_has_oit.insert(camera.target.normalize(primary_window));
        }
    }

    // Update the depth texture usage for cameras with a render target that has OIT
    for (mut camera_3d, camera) in &mut new_cameras {
        if render_target_has_oit.contains(&camera.target.normalize(primary_window)) {
            let mut usages = TextureUsages::from(camera_3d.depth_texture_usages);
            usages |= TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING;
            camera_3d.depth_texture_usages = usages.into();
        }
    }
}

fn check_msaa(cameras: Query<&Msaa, With<OrderIndependentTransparencySettings>>) {
    for msaa in &cameras {
        if msaa.samples() > 1 {
            panic!("MSAA is not supported when using OrderIndependentTransparency");
        }
    }
}

/// Holds the buffers that contain the data of all OIT layers.
/// We use one big buffer for the entire app. Each camera will reuse it so it will
/// always be the size of the biggest OIT enabled camera.
#[derive(Resource)]
pub struct OitBuffers {
    /// The OIT layers containing depth and color for each fragments.
    /// This is essentially used as a 3d array where xy is the screen coordinate and z is
    /// the list of fragments rendered with OIT.
    pub layers: BufferVec<UVec2>,
    /// Buffer containing the index of the last layer that was written for each fragment.
    pub layer_ids: BufferVec<i32>,
    pub settings: DynamicUniformBuffer<OrderIndependentTransparencySettings>,
}

pub fn init_oit_buffers(
    mut commands: Commands,
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
) {
    // initialize buffers with something so there's a valid binding

    let mut layers = BufferVec::new(BufferUsages::COPY_DST | BufferUsages::STORAGE);
    layers.set_label(Some("oit_layers"));
    layers.reserve(1, &render_device);
    layers.write_buffer(&render_device, &render_queue);

    let mut layer_ids = BufferVec::new(BufferUsages::COPY_DST | BufferUsages::STORAGE);
    layer_ids.set_label(Some("oit_layer_ids"));
    layer_ids.reserve(1, &render_device);
    layer_ids.write_buffer(&render_device, &render_queue);

    let mut settings = DynamicUniformBuffer::default();
    settings.set_label(Some("oit_settings"));

    commands.insert_resource(OitBuffers {
        layers,
        layer_ids,
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
            buffers.layers.capacity() * size_of::<UVec2>() / 1024 / 1024,
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
            buffers.layer_ids.capacity() * size_of::<UVec2>() / 1024 / 1024,
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
