//! This example illustrates how to make headless renderer
//! derived from: <https://sotrh.github.io/learn-wgpu/showcase/windowless/#a-triangle-without-a-window>
//! It follows this steps:
//! 1. Render from camera to gpu-image render target
//! 2. Copy from gpu image to buffer using `ImageCopyDriver` node in `RenderGraph`
//! 3. Copy from buffer to channel using `receive_image_from_buffer` after `RenderSet::Render`
//! 4. Save from channel to random named file using `scene::update` at `PostUpdate` in `MainWorld`
//! 5. Exit if `single_image` setting is set

use bevy::{
    app::{AppExit, ScheduleRunnerPlugin},
    core_pipeline::tonemapping::Tonemapping,
    prelude::*,
    render::{
        camera::RenderTarget,
        render_asset::{RenderAssetUsages, RenderAssets},
        render_graph::{self, NodeRunError, RenderGraph, RenderGraphContext, RenderLabel},
        render_resource::{
            Buffer, BufferDescriptor, BufferUsages, CommandEncoderDescriptor, Extent3d,
            ImageCopyBuffer, ImageDataLayout, Maintain, MapMode, TextureDimension, TextureFormat,
            TextureUsages,
        },
        renderer::{RenderContext, RenderDevice, RenderQueue},
        texture::{BevyDefault, TextureFormatPixelInfo},
        Extract, Render, RenderApp, RenderSet,
    },
};
use crossbeam_channel::{Receiver, Sender};
use std::{
    ops::{Deref, DerefMut},
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Duration,
};

// To communicate between the main world and the render world we need a channel.
// Since the main world and render world run in parallel, there will always be a frame of latency
// between the data sent from the render world and the data received in the main world
//
// frame n => render world sends data through the channel at the end of the frame
// frame n + 1 => main world receives the data
//
// Receiver and Sender are kept in resources because there is single camera and single target
// That's why there is single images role, if you want to differentiate images
// from different cameras, you should keep Receiver in ImageCopier and Sender in ImageToSave
// or send some id with data

/// This will receive asynchronously any data sent from the render world
#[derive(Resource, Deref)]
struct MainWorldReceiver(Receiver<Vec<u8>>);

/// This will send asynchronously any data to the main world
#[derive(Resource, Deref)]
struct RenderWorldSender(Sender<Vec<u8>>);

// Parameters of resulting image
struct AppConfig {
    width: u32,
    height: u32,
    single_image: bool,
}

fn main() {
    let config = AppConfig {
        width: 1920,
        height: 1080,
        single_image: true,
    };

    // setup frame capture
    App::new()
        .insert_resource(SceneController::new(
            config.width,
            config.height,
            config.single_image,
        ))
        .insert_resource(ClearColor(Color::srgb_u8(0, 0, 0)))
        .add_plugins(
            DefaultPlugins
                .set(ImagePlugin::default_nearest())
                // Do not create a window on startup.
                .set(WindowPlugin {
                    primary_window: None,
                    exit_condition: bevy::window::ExitCondition::DontExit,
                    close_when_requested: false,
                }),
        )
        .add_plugins(ImageCopyPlugin)
        // headless frame capture
        .add_plugins(CaptureFramePlugin)
        .add_plugins(ScheduleRunnerPlugin::run_loop(
            // Run 60 times per second.
            Duration::from_secs_f64(1.0 / 60.0),
        ))
        .init_resource::<SceneController>()
        .add_systems(Startup, setup)
        .run();
}

/// Capture image settings and state
#[derive(Debug, Default, Resource)]
struct SceneController {
    state: SceneState,
    name: String,
    width: u32,
    height: u32,
    single_image: bool,
}

impl SceneController {
    pub fn new(width: u32, height: u32, single_image: bool) -> SceneController {
        SceneController {
            state: SceneState::BuildScene,
            name: String::from(""),
            width,
            height,
            single_image,
        }
    }
}

/// Capture image state
#[derive(Debug, Default)]
enum SceneState {
    #[default]
    // State before any rendering
    BuildScene,
    // Rendering state, stores the number of frames remaining before saving the image
    Render(u32),
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
    mut scene_controller: ResMut<SceneController>,
    render_device: Res<RenderDevice>,
) {
    let render_target = setup_render_target(
        &mut commands,
        &mut images,
        &render_device,
        &mut scene_controller,
        // pre_roll_frames should be big enough for full scene render,
        // but the bigger it is, the longer example will run.
        // To visualize stages of scene rendering change this param to 0
        // and change AppConfig::single_image to false in main
        // Stages are:
        // 1. Transparent image
        // 2. Few black box images
        // 3. Fully rendered scene images
        // Exact number depends on device speed, device load and scene size
        40,
        "main_scene".into(),
    );

    // Scene example for non black box picture
    // circular base
    commands.spawn(PbrBundle {
        mesh: meshes.add(Circle::new(4.0)),
        material: materials.add(Color::WHITE),
        transform: Transform::from_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2)),
        ..default()
    });
    // cube
    commands.spawn(PbrBundle {
        mesh: meshes.add(Cuboid::new(1.0, 1.0, 1.0)),
        material: materials.add(Color::srgb_u8(124, 144, 255)),
        transform: Transform::from_xyz(0.0, 0.5, 0.0),
        ..default()
    });
    // light
    commands.spawn(PointLightBundle {
        point_light: PointLight {
            shadows_enabled: true,
            ..default()
        },
        transform: Transform::from_xyz(4.0, 8.0, 4.0),
        ..default()
    });

    commands.spawn(Camera3dBundle {
        transform: Transform::from_xyz(-2.5, 4.5, 9.0).looking_at(Vec3::ZERO, Vec3::Y),
        tonemapping: Tonemapping::None,
        camera: Camera {
            // render to image
            target: render_target,
            ..default()
        },
        ..default()
    });
}

/// Plugin for Render world part of work
pub struct ImageCopyPlugin;
impl Plugin for ImageCopyPlugin {
    fn build(&self, app: &mut App) {
        let (s, r) = crossbeam_channel::unbounded();

        let render_app = app
            .insert_resource(MainWorldReceiver(r))
            .sub_app_mut(RenderApp);

        let mut graph = render_app.world_mut().resource_mut::<RenderGraph>();
        graph.add_node(ImageCopy, ImageCopyDriver);
        graph.add_node_edge(bevy::render::graph::CameraDriverLabel, ImageCopy);

        render_app
            .insert_resource(RenderWorldSender(s))
            // Make ImageCopiers accessible in RenderWorld system and plugin
            .add_systems(ExtractSchedule, image_copy_extract)
            // Receives image data from buffer to channel
            // so we need to run it after the render graph is done
            .add_systems(Render, receive_image_from_buffer.after(RenderSet::Render));
    }
}

/// Setups render target and cpu image for saving, changes scene state into render mode
fn setup_render_target(
    commands: &mut Commands,
    images: &mut ResMut<Assets<Image>>,
    render_device: &Res<RenderDevice>,
    scene_controller: &mut ResMut<SceneController>,
    pre_roll_frames: u32,
    scene_name: String,
) -> RenderTarget {
    let size = Extent3d {
        width: scene_controller.width,
        height: scene_controller.height,
        ..Default::default()
    };

    // This is the texture that will be rendered to.
    let mut render_target_image = Image::new_fill(
        size,
        TextureDimension::D2,
        &[0; 4],
        TextureFormat::bevy_default(),
        RenderAssetUsages::default(),
    );
    render_target_image.texture_descriptor.usage |=
        TextureUsages::COPY_SRC | TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING;
    let render_target_image_handle = images.add(render_target_image);

    // This is the texture that will be copied to.
    let cpu_image = Image::new_fill(
        size,
        TextureDimension::D2,
        &[0; 4],
        TextureFormat::bevy_default(),
        RenderAssetUsages::default(),
    );
    let cpu_image_handle = images.add(cpu_image);

    commands.spawn(ImageCopier::new(
        render_target_image_handle.clone(),
        size,
        render_device,
    ));

    commands.spawn(ImageToSave(cpu_image_handle));

    scene_controller.state = SceneState::Render(pre_roll_frames);
    scene_controller.name = scene_name;
    RenderTarget::Image(render_target_image_handle)
}

/// Setups image saver
pub struct CaptureFramePlugin;
impl Plugin for CaptureFramePlugin {
    fn build(&self, app: &mut App) {
        info!("Adding CaptureFramePlugin");
        app.add_systems(PostUpdate, update);
    }
}

/// `ImageCopier` aggregator in `RenderWorld`
#[derive(Clone, Default, Resource, Deref, DerefMut)]
struct ImageCopiers(pub Vec<ImageCopier>);

/// Used by `ImageCopyDriver` for copying from render target to buffer
#[derive(Clone, Component)]
struct ImageCopier {
    buffer: Buffer,
    enabled: Arc<AtomicBool>,
    src_image: Handle<Image>,
}

impl ImageCopier {
    pub fn new(
        src_image: Handle<Image>,
        size: Extent3d,
        render_device: &RenderDevice,
    ) -> ImageCopier {
        let padded_bytes_per_row =
            RenderDevice::align_copy_bytes_per_row((size.width) as usize) * 4;

        let cpu_buffer = render_device.create_buffer(&BufferDescriptor {
            label: None,
            size: padded_bytes_per_row as u64 * size.height as u64,
            usage: BufferUsages::MAP_READ | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        ImageCopier {
            buffer: cpu_buffer,
            src_image,
            enabled: Arc::new(AtomicBool::new(true)),
        }
    }

    pub fn enabled(&self) -> bool {
        self.enabled.load(Ordering::Relaxed)
    }
}

/// Extracting `ImageCopier`s into render world, because `ImageCopyDriver` accesses them
fn image_copy_extract(mut commands: Commands, image_copiers: Extract<Query<&ImageCopier>>) {
    commands.insert_resource(ImageCopiers(
        image_copiers.iter().cloned().collect::<Vec<ImageCopier>>(),
    ));
}

/// `RenderGraph` label for `ImageCopyDriver`
#[derive(Debug, PartialEq, Eq, Clone, Hash, RenderLabel)]
struct ImageCopy;

/// `RenderGraph` node
#[derive(Default)]
struct ImageCopyDriver;

// Copies image content from render target to buffer
impl render_graph::Node for ImageCopyDriver {
    fn run(
        &self,
        _graph: &mut RenderGraphContext,
        render_context: &mut RenderContext,
        world: &World,
    ) -> Result<(), NodeRunError> {
        let image_copiers = world.get_resource::<ImageCopiers>().unwrap();
        let gpu_images = world
            .get_resource::<RenderAssets<bevy::render::texture::GpuImage>>()
            .unwrap();

        for image_copier in image_copiers.iter() {
            if !image_copier.enabled() {
                continue;
            }

            let src_image = gpu_images.get(&image_copier.src_image).unwrap();

            let mut encoder = render_context
                .render_device()
                .create_command_encoder(&CommandEncoderDescriptor::default());

            let block_dimensions = src_image.texture_format.block_dimensions();
            let block_size = src_image.texture_format.block_copy_size(None).unwrap();

            // Calculating correct size of image row because
            // copy_texture_to_buffer can copy image only by rows aligned wgpu::COPY_BYTES_PER_ROW_ALIGNMENT
            // That's why image in buffer can be little bit wider
            // This should be taken into account at copy from buffer stage
            let padded_bytes_per_row = RenderDevice::align_copy_bytes_per_row(
                (src_image.size.x as usize / block_dimensions.0 as usize) * block_size as usize,
            );

            let texture_extent = Extent3d {
                width: src_image.size.x,
                height: src_image.size.y,
                depth_or_array_layers: 1,
            };

            encoder.copy_texture_to_buffer(
                src_image.texture.as_image_copy(),
                ImageCopyBuffer {
                    buffer: &image_copier.buffer,
                    layout: ImageDataLayout {
                        offset: 0,
                        bytes_per_row: Some(
                            std::num::NonZeroU32::new(padded_bytes_per_row as u32)
                                .unwrap()
                                .into(),
                        ),
                        rows_per_image: None,
                    },
                },
                texture_extent,
            );

            let render_queue = world.get_resource::<RenderQueue>().unwrap();
            render_queue.submit(std::iter::once(encoder.finish()));
        }

        Ok(())
    }
}

/// runs in render world after Render stage to send image from buffer via channel (receiver is in main world)
fn receive_image_from_buffer(
    image_copiers: Res<ImageCopiers>,
    render_device: Res<RenderDevice>,
    sender: Res<RenderWorldSender>,
) {
    for image_copier in image_copiers.0.iter() {
        if !image_copier.enabled() {
            continue;
        }

        // Finally time to get our data back from the gpu.
        // First we get a buffer slice which represents a chunk of the buffer (which we
        // can't access yet).
        // We want the whole thing so use unbounded range.
        let buffer_slice = image_copier.buffer.slice(..);

        // Now things get complicated. WebGPU, for safety reasons, only allows either the GPU
        // or CPU to access a buffer's contents at a time. We need to "map" the buffer which means
        // flipping ownership of the buffer over to the CPU and making access legal. We do this
        // with `BufferSlice::map_async`.
        //
        // The problem is that map_async is not an async function so we can't await it. What
        // we need to do instead is pass in a closure that will be executed when the slice is
        // either mapped or the mapping has failed.
        //
        // The problem with this is that we don't have a reliable way to wait in the main
        // code for the buffer to be mapped and even worse, calling get_mapped_range or
        // get_mapped_range_mut prematurely will cause a panic, not return an error.
        //
        // Using channels solves this as awaiting the receiving of a message from
        // the passed closure will force the outside code to wait. It also doesn't hurt
        // if the closure finishes before the outside code catches up as the message is
        // buffered and receiving will just pick that up.
        //
        // It may also be worth noting that although on native, the usage of asynchronous
        // channels is wholly unnecessary, for the sake of portability to WASM
        // we'll use async channels that work on both native and WASM.

        let (s, r) = crossbeam_channel::bounded(1);

        // Maps the buffer so it can be read on the cpu
        buffer_slice.map_async(MapMode::Read, move |r| match r {
            // This will execute once the gpu is ready, so after the call to poll()
            Ok(r) => s.send(r).expect("Failed to send map update"),
            Err(err) => panic!("Failed to map buffer {err}"),
        });

        // In order for the mapping to be completed, one of three things must happen.
        // One of those can be calling `Device::poll`. This isn't necessary on the web as devices
        // are polled automatically but natively, we need to make sure this happens manually.
        // `Maintain::Wait` will cause the thread to wait on native but not on WebGpu.

        // This blocks until the gpu is done executing everything
        render_device.poll(Maintain::wait()).panic_on_timeout();

        // This blocks until the buffer is mapped
        r.recv().expect("Failed to receive the map_async message");

        // This could fail on app exit, if Main world clears resources (including receiver) while Render world still renders
        let _ = sender.send(buffer_slice.get_mapped_range().to_vec());

        // We need to make sure all `BufferView`'s are dropped before we do what we're about
        // to do.
        // Unmap so that we can copy to the staging buffer in the next iteration.
        image_copier.buffer.unmap();
    }
}

/// CPU-side image for saving
#[derive(Component, Deref, DerefMut)]
struct ImageToSave(Handle<Image>);

// Takes from channel image content sent from render world and saves it to disk
fn update(
    images_to_save: Query<&ImageToSave>,
    receiver: Res<MainWorldReceiver>,
    mut images: ResMut<Assets<Image>>,
    mut scene_controller: ResMut<SceneController>,
    mut app_exit_writer: EventWriter<AppExit>,
    mut file_number: Local<u32>,
) {
    if let SceneState::Render(n) = scene_controller.state {
        if n < 1 {
            // We don't want to block the main world on this,
            // so we use try_recv which attempts to receive without blocking
            let mut image_data = Vec::new();
            while let Ok(data) = receiver.try_recv() {
                // image generation could be faster than saving to fs,
                // that's why use only last of them
                image_data = data;
            }
            if !image_data.is_empty() {
                for image in images_to_save.iter() {
                    // Fill correct data from channel to image
                    let img_bytes = images.get_mut(image.id()).unwrap();

                    // We need to ensure that this works regardless of the image dimensions
                    // If the image became wider when copying from the texture to the buffer,
                    // then the data is reduced to its original size when copying from the buffer to the image.
                    let row_bytes = img_bytes.width() as usize
                        * img_bytes.texture_descriptor.format.pixel_size();
                    let aligned_row_bytes = RenderDevice::align_copy_bytes_per_row(row_bytes);
                    if row_bytes == aligned_row_bytes {
                        img_bytes.data.clone_from(&image_data);
                    } else {
                        // shrink data to original image size
                        img_bytes.data = image_data
                            .chunks(aligned_row_bytes)
                            .take(img_bytes.height() as usize)
                            .flat_map(|row| &row[..row_bytes.min(row.len())])
                            .cloned()
                            .collect();
                    }

                    // Create RGBA Image Buffer
                    let img = match img_bytes.clone().try_into_dynamic() {
                        Ok(img) => img.to_rgba8(),
                        Err(e) => panic!("Failed to create image buffer {e:?}"),
                    };

                    // Prepare directory for images, test_images in bevy folder is used here for example
                    // You should choose the path depending on your needs
                    let images_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("test_images");
                    info!("Saving image to: {images_dir:?}");
                    std::fs::create_dir_all(&images_dir).unwrap();

                    // Choose filename starting from 000.png
                    let image_path = images_dir.join(format!("{:03}.png", file_number.deref()));
                    *file_number.deref_mut() += 1;

                    // Finally saving image to file, this heavy blocking operation is kept here
                    // for example simplicity, but in real app you should move it to a separate task
                    if let Err(e) = img.save(image_path) {
                        panic!("Failed to save image: {}", e);
                    };
                }
                if scene_controller.single_image {
                    app_exit_writer.send(AppExit::Success);
                }
            }
        } else {
            // clears channel for skipped frames
            while receiver.try_recv().is_ok() {}
            scene_controller.state = SceneState::Render(n - 1);
        }
    }
}
