//! Validates single-pass multiview (stereo) rendering on hardware that supports it.
//!
//! Multiview renders N views in **one** pass, with the GPU broadcasting each draw to
//! every layer of an array render target and the shader selecting per-view data via
//! WGSL's `@builtin(view_index)`. It is the standard way to render stereo for VR.
//!
//! `wgpu` only exposes multiview on **Vulkan**, so this example is a no-op elsewhere.
//! Run it with the Vulkan backend forced:
//!
//! ```sh
//! WGPU_BACKEND=vulkan cargo run --example multiview
//! ```
//!
//! It renders a scene once through a camera carrying a two-entry [`Multiview`]
//! (a left and right eye, 64mm apart), reads both array layers back, and writes
//! `multiview_layer0.png` and `multiview_layer1.png` next to the working directory.
//!
//! **What a pass looks like:** both PNGs show the same scene, shifted horizontally
//! by the eye offset — i.e. real parallax. Near geometry should shift more than far
//! geometry.
//!
//! **What a failure looks like:** wgpu validation errors on startup; two *identical*
//! images (the view index isn't reaching the shader); or a black/garbage layer 1
//! (the array target isn't being broadcast to).

use bevy::{
    camera::{
        CameraProjection, ManualTextureViewHandle, Multiview, MultiviewSubview,
        PerspectiveProjection, RenderTarget,
    },
    prelude::*,
    render::{
        render_resource::{
            Buffer, BufferDescriptor, BufferUsages, CommandEncoderDescriptor, Extent3d, MapMode,
            PollType, TexelCopyBufferInfo, TexelCopyBufferLayout, TextureDescriptor,
            TextureDimension, TextureFormat, TextureUsages, TextureViewDescriptor,
            TextureViewDimension,
        },
        renderer::{RenderDevice, RenderQueue},
        texture::{ManualTextureView, ManualTextureViews},
        Extract, Render, RenderApp, RenderSystems,
    },
};
use crossbeam_channel::{Receiver, Sender};

/// Arbitrary unused handle for our manual render target.
const TARGET: ManualTextureViewHandle = ManualTextureViewHandle(42);

const WIDTH: u32 = 640;
const HEIGHT: u32 = 480;
/// Half of a 64mm interpupillary distance, in meters.
const HALF_IPD: f32 = 0.032;
/// Number of views (eyes) to render in the single pass.
const VIEW_COUNT: u32 = 2;
/// Give pipelines time to compile before capturing. Bevy specializes and
/// compiles render pipelines lazily, and skips draws whose pipeline isn't ready
/// yet, so capturing too early yields a cleared target and tells you nothing
/// about multiview. The PBR mesh pipelines are the slow ones here.
const CAPTURE_FRAME: u32 = 300;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(ReadbackPlugin)
        .add_systems(Startup, setup)
        .add_systems(Update, save_when_ready)
        .run();
}

/// The array texture the multiview camera renders into, plus the CPU-visible
/// buffer we copy it back through.
#[derive(Resource, Clone)]
struct MultiviewTarget {
    texture: bevy::render::render_resource::Texture,
    buffer: Buffer,
    padded_bytes_per_row: u32,
}

fn setup(
    mut commands: Commands,
    mut manual_views: ResMut<ManualTextureViews>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    render_device: Res<RenderDevice>,
) {
    // A 2-layer array texture: one layer per eye. This is the shape an OpenXR
    // runtime hands you for a stereo swapchain image.
    let texture = render_device.create_texture(&TextureDescriptor {
        label: Some("multiview_target"),
        size: Extent3d {
            width: WIDTH,
            height: HEIGHT,
            depth_or_array_layers: VIEW_COUNT,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: TextureDimension::D2,
        format: TextureFormat::Rgba8UnormSrgb,
        usage: TextureUsages::RENDER_ATTACHMENT
            | TextureUsages::TEXTURE_BINDING
            | TextureUsages::COPY_SRC,
        view_formats: &[],
    });

    // The view Bevy renders through must be `D2Array` so the pass can address
    // every layer.
    let texture_view = texture.create_view(&TextureViewDescriptor {
        label: Some("multiview_target_view"),
        dimension: Some(TextureViewDimension::D2Array),
        ..default()
    });

    manual_views.insert(
        TARGET,
        ManualTextureView {
            texture_view,
            size: UVec2::new(WIDTH, HEIGHT),
            view_format: TextureFormat::Rgba8UnormSrgb,
        },
    );

    // Readback buffer. `copy_texture_to_buffer` needs each row padded to
    // `COPY_BYTES_PER_ROW_ALIGNMENT`, and we copy both layers in one go.
    let padded_bytes_per_row = RenderDevice::align_copy_bytes_per_row(WIDTH as usize * 4) as u32;
    let buffer = render_device.create_buffer(&BufferDescriptor {
        label: Some("multiview_readback"),
        size: padded_bytes_per_row as u64 * HEIGHT as u64 * VIEW_COUNT as u64,
        usage: BufferUsages::MAP_READ | BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });

    commands.insert_resource(MultiviewTarget {
        texture,
        buffer,
        padded_bytes_per_row,
    });

    // Per-eye projections. A real runtime supplies asymmetric per-eye
    // projections; a symmetric one is fine for checking parallax.
    //
    // `aspect_ratio` has to be set explicitly. Bevy normally overwrites it from
    // the viewport in `camera_system`, but that only touches the camera's own
    // `Projection` — a projection built standalone for a subview never gets
    // that treatment, so the default of 1.0 would render a square frustum into
    // a 4:3 target and stretch both eyes horizontally.
    let projection = PerspectiveProjection {
        aspect_ratio: WIDTH as f32 / HEIGHT as f32,
        ..default()
    }
    .get_clip_from_view();

    let eye = |x: f32| MultiviewSubview {
        view_from_camera: Transform::from_xyz(x, 0.0, 0.0),
        clip_from_view: projection,
    };

    commands.spawn((
        Camera3d::default(),
        Camera::default(),
        // Multiview and MSAA don't combine: WGSL has no
        // `texture_depth_multisampled_2d_array`, so the depth binding stays
        // single-layer under MSAA and per-eye depth reads would be wrong.
        // A real XR runtime hands you a single-sampled swapchain image anyway.
        Msaa::Off,
        RenderTarget::TextureView(TARGET),
        // The camera transform is the "head" pose; each subview offsets from it.
        Transform::from_xyz(0.0, 1.0, 4.0).looking_at(Vec3::new(0.0, 0.5, 0.0), Vec3::Y),
        Multiview {
            views: vec![eye(-HALF_IPD), eye(HALF_IPD)],
        },
    ));

    // A near object and a far object, so the parallax difference between the two
    // layers is obvious rather than subtle.
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(0.4, 0.4, 0.4))),
        MeshMaterial3d(materials.add(Color::srgb(0.9, 0.2, 0.2))),
        Transform::from_xyz(0.0, 0.6, 2.6),
    ));
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(1.0, 1.0, 1.0))),
        MeshMaterial3d(materials.add(Color::srgb(0.2, 0.4, 0.9))),
        Transform::from_xyz(0.0, 0.5, -3.0),
    ));
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(20.0, 20.0))),
        MeshMaterial3d(materials.add(Color::srgb(0.75, 0.75, 0.75))),
    ));
    commands.spawn((
        DirectionalLight {
            shadow_maps_enabled: true,
            ..default()
        },
        Transform::from_xyz(4.0, 8.0, 4.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
}

// --- readback plumbing -------------------------------------------------------

#[derive(Resource, Deref)]
struct MainWorldReceiver(Receiver<Vec<u8>>);

#[derive(Resource, Deref)]
struct RenderWorldSender(Sender<Vec<u8>>);

struct ReadbackPlugin;

impl Plugin for ReadbackPlugin {
    fn build(&self, app: &mut App) {
        let (sender, receiver) = crossbeam_channel::unbounded();
        app.insert_resource(MainWorldReceiver(receiver));

        let render_app = app.sub_app_mut(RenderApp);
        render_app
            .insert_resource(RenderWorldSender(sender))
            .add_systems(ExtractSchedule, extract_target)
            .add_systems(Render, copy_and_send.after(RenderSystems::Render));
    }
}

fn extract_target(mut commands: Commands, target: Extract<Option<Res<MultiviewTarget>>>) {
    if let Some(target) = target.as_ref() {
        commands.insert_resource((*target).clone());
    }
}

/// Copies every layer of the array target into the readback buffer, maps it, and
/// ships the bytes to the main world.
fn copy_and_send(
    target: Option<Res<MultiviewTarget>>,
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    sender: Res<RenderWorldSender>,
) {
    let Some(target) = target else {
        return;
    };

    let mut encoder = render_device.create_command_encoder(&CommandEncoderDescriptor::default());
    encoder.copy_texture_to_buffer(
        target.texture.as_image_copy(),
        TexelCopyBufferInfo {
            buffer: &target.buffer,
            layout: TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(target.padded_bytes_per_row),
                // Required when copying more than one array layer.
                rows_per_image: Some(HEIGHT),
            },
        },
        Extent3d {
            width: WIDTH,
            height: HEIGHT,
            depth_or_array_layers: VIEW_COUNT,
        },
    );
    render_queue.submit(std::iter::once(encoder.finish()));

    let slice = target.buffer.slice(..);
    let (tx, rx) = crossbeam_channel::bounded(1);
    slice.map_async(MapMode::Read, move |result| {
        let _ = tx.send(result);
    });
    // Block until the copy lands; this example values simplicity over throughput.
    if render_device.poll(PollType::wait_indefinitely()).is_err() {
        return;
    }
    if rx.recv().is_err() {
        return;
    }

    let Ok(view) = slice.get_mapped_range() else {
        return;
    };
    let _ = sender.send(view.to_vec());
    drop(view);
    target.buffer.unmap();
}

/// Waits for the scene to settle, then splits the readback into one image per
/// eye and writes them to disk.
fn save_when_ready(
    receiver: Res<MainWorldReceiver>,
    mut frames: Local<u32>,
    mut done: Local<bool>,
    mut exit: MessageWriter<AppExit>,
) {
    // Drain whatever has arrived; keep only the most recent frame.
    let mut latest = None;
    while let Ok(bytes) = receiver.try_recv() {
        latest = Some(bytes);
    }

    *frames += 1;
    if *done || *frames < CAPTURE_FRAME {
        return;
    }
    let Some(bytes) = latest else {
        return;
    };

    let padded = RenderDevice::align_copy_bytes_per_row(WIDTH as usize * 4);
    let layer_stride = padded * HEIGHT as usize;

    for layer in 0..VIEW_COUNT as usize {
        // Strip the row padding wgpu required for the copy.
        let mut pixels = Vec::with_capacity((WIDTH * HEIGHT * 4) as usize);
        for row in 0..HEIGHT as usize {
            let start = layer * layer_stride + row * padded;
            pixels.extend_from_slice(&bytes[start..start + WIDTH as usize * 4]);
        }

        let image = Image::new(
            Extent3d {
                width: WIDTH,
                height: HEIGHT,
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            pixels,
            TextureFormat::Rgba8UnormSrgb,
            bevy::asset::RenderAssetUsages::default(),
        );

        let path = format!("multiview_layer{layer}.png");
        match image.try_into_dynamic() {
            Ok(dynamic) => match dynamic.save(&path) {
                Ok(()) => info!("wrote {path}"),
                Err(err) => error!("failed to write {path}: {err}"),
            },
            Err(err) => error!("failed to convert layer {layer}: {err:?}"),
        }
    }

    info!("compare multiview_layer0.png and multiview_layer1.png — they should differ by parallax");
    *done = true;
    exit.write(AppExit::Success);
}
