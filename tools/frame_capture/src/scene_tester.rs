use std::{path::PathBuf, time::Duration};

use bevy::{
    app::{AppExit, ScheduleRunnerPlugin, ScheduleRunnerSettings},
    log::LogPlugin,
    prelude::*,
    render::{camera::RenderTarget, renderer::RenderDevice},
    winit::WinitPlugin,
};
use wgpu::{Extent3d, TextureDescriptor, TextureDimension, TextureFormat, TextureUsages};

use image::io::Reader;

use crate::image_copy::{ImageCopier, ImageCopyPlugin};

#[derive(Component, Default)]
pub struct CaptureCamera;

#[derive(Component, Deref, DerefMut)]
struct ImageToSave(Handle<Image>);

pub struct SceneTesterPlugin;
impl Plugin for SceneTesterPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(
            DefaultPlugins
                .build()
                .disable::<WinitPlugin>()
                // multiple separate app runs with LogPlugin result in this error:
                // thread 'main' panicked at 'called `Result::unwrap()` on an `Err` value: SetLoggerError(())', crates\bevy_log\src\lib.rs:118:27
                .disable::<LogPlugin>()
                .set(WindowPlugin {
                    add_primary_window: false,
                    exit_on_all_closed: false,
                    close_when_requested: true,
                    ..default()
                }),
        )
        .insert_resource(ScheduleRunnerSettings::run_loop(Duration::from_secs_f64(
            1.0 / 60.0, //Don't run faster than 60fps
        )))
        .init_resource::<SceneController>()
        .add_plugin(ScheduleRunnerPlugin)
        .add_plugin(ImageCopyPlugin)
        .add_event::<SceneController>()
        .add_system_to_stage(CoreStage::PostUpdate, update);
    }
}

#[derive(Debug, Resource)]
pub struct SceneController {
    state: SceneState,
    name: String,
    create_images: bool,
    width: u32,
    height: u32,
}

impl SceneController {
    pub fn new(create_images: bool) -> SceneController {
        SceneController {
            state: SceneState::BuildScene,
            name: String::from(""),
            create_images,
            width: 512,
            height: 512,
        }
    }
}

impl Default for SceneController {
    fn default() -> SceneController {
        SceneController {
            state: SceneState::BuildScene,
            name: String::from(""),
            create_images: false,
            width: 512,
            height: 512,
        }
    }
}

#[derive(Debug)]
pub enum SceneState {
    BuildScene,
    Render(u32),
}

pub fn setup_test(
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
        if n > 0 {
            scene_controller.state = SceneState::Render(n - 1);
        } else {
            for image in images_to_save.iter() {
                let data = &images.get_mut(image).unwrap().data;

                let images_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("test_images");
                let image_path = images_path.join(format!("{}.png", scene_controller.name));

                // Create test image file
                if scene_controller.create_images {
                    image::save_buffer(
                        image_path,
                        data,
                        scene_controller.width,
                        scene_controller.height,
                        image::ColorType::Rgba8,
                    )
                    .unwrap();
                } else {
                    // Test against existing image
                    match Reader::open(&image_path) {
                        Ok(file) => {
                            let existing_image = file.decode().unwrap();
                            if data != existing_image.as_flat_samples_u8().unwrap().samples {
                                panic!(
                                    "{} failed, {:?} does not match",
                                    scene_controller.name, image_path
                                )
                            }
                        }
                        Err(_) => {
                            panic!(
                                "{} failed, could not find file {:?}",
                                scene_controller.name, image_path
                            )
                        }
                    }
                }
            }
            app_exit_writer.send(AppExit);
        }
    }
}
