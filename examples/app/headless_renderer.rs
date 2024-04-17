//! This example illustrates how to make headless renderer

mod frame_capture {
    pub mod image_copy {
        use bevy::prelude::*;
        use bevy::render::{
            render_asset::RenderAssets,
            render_graph::{self, NodeRunError, RenderGraph, RenderGraphContext},
            render_resource::{
                Buffer, BufferDescriptor, BufferUsages, CommandEncoderDescriptor, Extent3d,
                ImageCopyBuffer, ImageDataLayout, Maintain, MapMode,
            },
            renderer::{RenderContext, RenderDevice, RenderQueue},
            Extract, RenderApp,
        };
        use std::sync::{
            atomic::{AtomicBool, Ordering},
            Arc,
        };

        pub fn receive_images(
            image_copiers: Query<&ImageCopier>,
            mut images: ResMut<Assets<Image>>,
            render_device: Res<RenderDevice>,
        ) {
            for image_copier in image_copiers.iter() {
                if !image_copier.enabled() {
                    continue;
                }
                // Derived from: https://sotrh.github.io/learn-wgpu/showcase/windowless/#a-triangle-without-a-window
                // We need to scope the mapping variables so that we can
                // unmap the buffer
                futures_lite::future::block_on(async {
                    let buffer_slice = image_copier.buffer.slice(..);

                    // NOTE: We have to create the mapping THEN device.poll() before await
                    // the future. Otherwise the application will freeze.
                    let (tx, rx) = async_channel::bounded(1);
                    buffer_slice.map_async(MapMode::Read, move |result| {
                        tx.send_blocking(result).unwrap();
                    });
                    render_device.poll(Maintain::Wait);
                    rx.recv().await.unwrap().unwrap();
                    if let Some(image) = images.get_mut(&image_copier.dst_image) {
                        image.data = buffer_slice.get_mapped_range().to_vec();
                    }

                    image_copier.buffer.unmap();
                });
            }
        }

        #[derive(Debug, PartialEq, Eq, Clone, Hash, bevy::render::render_graph::RenderLabel)]
        pub struct ImageCopy;

        pub struct ImageCopyPlugin;
        impl Plugin for ImageCopyPlugin {
            fn build(&self, app: &mut App) {
                let render_app = app
                    .add_systems(Update, receive_images)
                    .sub_app_mut(RenderApp);

                render_app.add_systems(ExtractSchedule, image_copy_extract);

                let mut graph = render_app
                    .world_mut()
                    .get_resource_mut::<RenderGraph>()
                    .unwrap();

                graph.add_node(ImageCopy, ImageCopyDriver);

                graph.add_node_edge(ImageCopy, bevy::render::graph::CameraDriverLabel);
            }
        }

        #[derive(Clone, Default, Resource, Deref, DerefMut)]
        pub struct ImageCopiers(pub Vec<ImageCopier>);

        #[derive(Clone, Component)]
        pub struct ImageCopier {
            buffer: Buffer,
            enabled: Arc<AtomicBool>,
            src_image: Handle<Image>,
            dst_image: Handle<Image>,
        }

        impl ImageCopier {
            pub fn new(
                src_image: Handle<Image>,
                dst_image: Handle<Image>,
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
                    dst_image,
                    enabled: Arc::new(AtomicBool::new(true)),
                }
            }

            pub fn enabled(&self) -> bool {
                self.enabled.load(Ordering::Relaxed)
            }
        }

        pub fn image_copy_extract(
            mut commands: Commands,
            image_copiers: Extract<Query<&ImageCopier>>,
        ) {
            commands.insert_resource(ImageCopiers(
                image_copiers.iter().cloned().collect::<Vec<ImageCopier>>(),
            ));
        }

        #[derive(Default)]
        pub struct ImageCopyDriver;

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

                    let padded_bytes_per_row = RenderDevice::align_copy_bytes_per_row(
                        (src_image.size.x as usize / block_dimensions.0 as usize)
                            * block_size as usize,
                    );

                    let texture_extent = Extent3d {
                        width: src_image.size.x as u32,
                        height: src_image.size.y as u32,
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
    }
    pub mod scene {
        use std::path::PathBuf;

        use bevy::{
            app::AppExit,
            prelude::*,
            render::{
                camera::RenderTarget,
                render_resource::{
                    Extent3d, TextureDescriptor, TextureDimension, TextureFormat, TextureUsages,
                },
                renderer::RenderDevice,
            },
        };

        use super::image_copy::ImageCopier;

        #[derive(Component, Default)]
        pub struct CaptureCamera;

        #[derive(Component, Deref, DerefMut)]
        struct ImageToSave(Handle<Image>);

        pub struct CaptureFramePlugin;
        impl Plugin for CaptureFramePlugin {
            fn build(&self, app: &mut App) {
                println!("Adding CaptureFramePlugin");
                app.add_systems(PostUpdate, update);
            }
        }

        #[derive(Debug, Default, Resource, Event)]
        pub struct SceneController {
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

        #[derive(Debug, Default)]
        pub enum SceneState {
            #[default]
            BuildScene,
            Render(u32),
        }

        pub fn setup_render_target(
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
            let mut render_target_image = Image {
                texture_descriptor: TextureDescriptor {
                    label: None,
                    size,
                    dimension: TextureDimension::D2,
                    format: TextureFormat::Rgba8UnormSrgb,
                    mip_level_count: 1,
                    sample_count: 1,
                    usage: TextureUsages::COPY_SRC
                        | TextureUsages::COPY_DST
                        | TextureUsages::TEXTURE_BINDING
                        | TextureUsages::RENDER_ATTACHMENT,
                    view_formats: &[],
                },
                ..Default::default()
            };
            render_target_image.resize(size);
            let render_target_image_handle = images.add(render_target_image);

            // This is the texture that will be copied to.
            let mut cpu_image = Image {
                texture_descriptor: TextureDescriptor {
                    label: None,
                    size,
                    dimension: TextureDimension::D2,
                    format: TextureFormat::Rgba8UnormSrgb,
                    mip_level_count: 1,
                    sample_count: 1,
                    usage: TextureUsages::COPY_DST | TextureUsages::TEXTURE_BINDING,
                    view_formats: &[],
                },
                ..Default::default()
            };
            cpu_image.resize(size);
            let cpu_image_handle = images.add(cpu_image);

            commands.spawn(ImageCopier::new(
                render_target_image_handle.clone(),
                cpu_image_handle.clone(),
                size,
                render_device,
            ));

            commands.spawn(ImageToSave(cpu_image_handle));

            scene_controller.state = SceneState::Render(pre_roll_frames);
            scene_controller.name = scene_name;
            RenderTarget::Image(render_target_image_handle)
        }

        fn update(
            images_to_save: Query<&ImageToSave>,
            mut images: ResMut<Assets<Image>>,
            mut scene_controller: ResMut<SceneController>,
            mut app_exit_writer: EventWriter<AppExit>,
        ) {
            if let SceneState::Render(n) = scene_controller.state {
                if n < 1 {
                    use rand::Rng;
                    let mut rng = rand::thread_rng();
                    for image in images_to_save.iter() {
                        let img_bytes = images.get_mut(image.id()).unwrap();

                        let img = match img_bytes.clone().try_into_dynamic() {
                            Ok(img) => img.to_rgba8(),
                            Err(e) => panic!("Failed to create image buffer {e:?}"),
                        };

                        let images_dir =
                            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("test_images");
                        print!("Saving image to: {:?}\n", images_dir);
                        std::fs::create_dir_all(&images_dir).unwrap();

                        let number = rng.gen::<u128>();
                        let image_path = images_dir.join(format!("{number:032X}.png"));
                        if let Err(e) = img.save(image_path) {
                            panic!("Failed to save image: {}", e);
                        };
                    }
                    if scene_controller.single_image {
                        app_exit_writer.send(AppExit);
                    }
                } else {
                    scene_controller.state = SceneState::Render(n - 1);
                }
            }
        }
    }
}

struct AppConfig {
    width: u32,
    height: u32,
    single_image: bool,
}

use bevy::{
    app::ScheduleRunnerPlugin, core_pipeline::tonemapping::Tonemapping, prelude::*,
    render::renderer::RenderDevice,
};

fn main() {
    let mut app = App::new();

    let config = AppConfig {
        width: 1920,
        height: 1080,
        single_image: true,
    };

    // setup frame capture
    app.insert_resource(frame_capture::scene::SceneController::new(
        config.width,
        config.height,
        config.single_image,
    ));
    app.insert_resource(ClearColor(Color::srgb_u8(0, 0, 0)));

    app.add_plugins(
        DefaultPlugins
            .set(ImagePlugin::default_nearest())
            .set(WindowPlugin {
                primary_window: None,
                exit_condition: bevy::window::ExitCondition::DontExit,
                close_when_requested: false,
            })
            .build()
            // avoid panic, caused by using buffer by main world and render world at same time:
            // thread '<unnamed>' panicked at /path/to/.cargo/registry/src/index.crates.io-6f17d22bba15001f/wgpu-0.19.3/src/backend/wgpu_core.rs:2225:30:
            // Error in Queue::submit: Validation Error
            //
            // Caused by:
            //     Buffer Id(0,1,your_backend_type) is still mapped
            .disable::<bevy::render::pipelined_rendering::PipelinedRenderingPlugin>(),
    );

    app.add_plugins(frame_capture::image_copy::ImageCopyPlugin);

    // headless frame capture
    app.add_plugins(frame_capture::scene::CaptureFramePlugin);

    app.add_plugins(ScheduleRunnerPlugin::run_loop(
        std::time::Duration::from_secs_f64(1.0 / 60.0),
    ));

    app.init_resource::<frame_capture::scene::SceneController>();
    app.add_event::<frame_capture::scene::SceneController>();

    app.add_systems(Startup, setup);

    app.run();
}

fn setup(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut scene_controller: ResMut<frame_capture::scene::SceneController>,
    render_device: Res<RenderDevice>,
) {
    let render_target = frame_capture::scene::setup_render_target(
        &mut commands,
        &mut images,
        &render_device,
        &mut scene_controller,
        15,
        String::from("main_scene"),
    );

    // Scene is empty, but you can add any mesh to generate non black box picture

    commands.spawn(Camera3dBundle {
        transform: Transform::from_xyz(0.0, 6., 12.0).looking_at(Vec3::new(0., 1., 0.), Vec3::Y),
        tonemapping: Tonemapping::None,
        camera: Camera {
            target: render_target,
            ..default()
        },
        ..default()
    });
}
