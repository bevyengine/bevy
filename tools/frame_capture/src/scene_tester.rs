use std::time::Duration;

use bevy::{
    app::{AppExit, ScheduleRunnerPlugin, ScheduleRunnerSettings},
    prelude::*,
    render::{camera::RenderTarget, renderer::RenderDevice},
    window::{ModifiesWindows, WindowSettings},
    winit::WinitPlugin,
};
use wgpu::{Extent3d, TextureDescriptor, TextureDimension, TextureFormat, TextureUsages};

use crate::image_copy::{ImageCopier, ImageCopyPlugin};

#[derive(Component, Default)]
pub struct CaptureCamera;

#[derive(Component, Deref, DerefMut)]
struct ImageToSave(Handle<Image>);

fn modifies_windows() {}

pub struct SceneName(pub String);

pub struct SceneTesterPlugin;
impl Plugin for SceneTesterPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(WindowSettings {
            add_primary_window: false,
            exit_on_all_closed: false,
            close_when_requested: true,
        })
        .add_plugins_with(DefaultPlugins, |group| {
            group.disable::<WinitPlugin>();
            // The render and camera plugin requires the Windows resource and events to exist.
            // So we can't just disable the window plugin with: group.disable::<WindowPlugin>();
            group
        })
        .add_system_to_stage(
            CoreStage::PostUpdate,
            modifies_windows.label(ModifiesWindows), // Cursed
        )
        .insert_resource(ScheduleRunnerSettings::run_loop(Duration::from_secs_f64(
            1.0 / 60.0, //Don't run faster than 60fps
        )))
        .insert_resource(SceneController(SceneState::BuildScene))
        .add_plugin(ScheduleRunnerPlugin)
        .add_plugin(ImageCopyPlugin)
        .add_event::<SceneController>()
        .add_startup_system(setup)
        .add_system_to_stage(CoreStage::PostUpdate, update);
    }
}

#[derive(Debug)]
pub struct SceneController(pub SceneState);

#[derive(Debug)]
pub enum SceneState {
    BuildScene,
    Render(u32),
}

fn setup(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    render_device: Res<RenderDevice>,
) {
    let size = Extent3d {
        width: 512,
        height: 512,
        ..Default::default()
    };

    // This is the texture that will be rendered to.
    let mut render_target_image = Image {
        texture_descriptor: TextureDescriptor {
            label: None,
            size,
            dimension: TextureDimension::D2,
            format: TextureFormat::Bgra8UnormSrgb,
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
            format: TextureFormat::Bgra8UnormSrgb,
            mip_level_count: 1,
            sample_count: 1,
            usage: TextureUsages::COPY_DST | TextureUsages::TEXTURE_BINDING,
        },
        ..Default::default()
    };
    cpu_image.resize(size);
    let cpu_image_handle = images.add(cpu_image);

    commands.spawn().insert(ImageCopier::new(
        render_target_image_handle.clone(),
        cpu_image_handle.clone(),
        size,
        &render_device,
    ));

    commands
        .spawn()
        .insert(ImageToSave(cpu_image_handle.clone()));

    commands.spawn_bundle(Camera3dBundle {
        transform: Transform::from_xyz(1.0, 1.0, 1.0).looking_at(Vec3::new(0.0, 0.0, 0.0), Vec3::Y),
        camera: Camera {
            target: RenderTarget::Image(render_target_image_handle),
            ..default()
        },
        ..default()
    });
}

fn update(
    images_to_save: Query<&ImageToSave>,
    mut images: ResMut<Assets<Image>>,
    mut scene_controller: ResMut<SceneController>,
    mut app_exit_writer: EventWriter<AppExit>,
    scene_name: Res<SceneName>,
) {
    if let SceneState::Render(n) = scene_controller.0 {
        if n > 0 {
            scene_controller.0 = SceneState::Render(n - 1)
        } else {
            for image in images_to_save.iter() {
                dbg!(&scene_controller);
                //convert to rgba
                let data = &mut images.get_mut(image).unwrap().data;
                for src in data.chunks_exact_mut(4) {
                    let r = src[2];
                    let g = src[1];
                    let b = src[0];
                    let a = src[3];
                    src[0] = r;
                    src[1] = g;
                    src[2] = b;
                    src[3] = a;
                }

                image::save_buffer(
                    &format!("./../scene{}.png", scene_name.0),
                    &data,
                    512,
                    512,
                    image::ColorType::Rgba8,
                )
                .unwrap();
            }
            app_exit_writer.send(AppExit);
        }
    }
}
