//! This example shows how to make an externally driven headless renderer,
//! pumping the update loop manually.
use bevy::{
    app::SubApps,
    asset::RenderAssetUsages,
    camera::RenderTarget,
    core_pipeline::tonemapping::Tonemapping,
    diagnostic::FrameCount,
    image::Image,
    prelude::*,
    render::{
        render_resource::{Extent3d, PollType, TextureDimension, TextureFormat, TextureUsages},
        renderer::RenderDevice,
        view::screenshot::{save_to_disk, Screenshot},
        RenderPlugin,
    },
    window::ExitCondition,
    winit::WinitPlugin,
};

fn main() {
    let mut bw = BevyWrapper::new();

    let target = bw.new_render_target(500, 500);
    let camera = bw.spawn_camera(target.clone());
    for i in 0..10 {
        // Schedule a screenshot for this frame
        bw.screenshot(target.clone(), i);
        // Pump the update loop once
        bw.update();
    }
    // Loop a couple times more to let screenshot gpu readback and then write to disk
    bw.update();
    bw.update();
}

struct BevyWrapper(SubApps);

impl BevyWrapper {
    fn new() -> Self {
        let render_plugin = RenderPlugin {
            // Make sure all shaders are loaded for the first frame
            synchronous_pipeline_compilation: true,
            ..default()
        };
        // We don't have any windows, but the WindowPlugin is still needed
        // because a lot of bevy expects it to be there. Just configure it
        // to not have any windows and not exit automatically.
        let window_plugin = WindowPlugin {
            primary_window: None,
            exit_condition: ExitCondition::DontExit,
            ..default()
        };

        let mut app = App::new();
        app.add_plugins(
            DefaultPlugins
                .set(window_plugin)
                .set(render_plugin)
                .disable::<WinitPlugin>(),
        )
        .add_systems(Startup, spawn_test_scene)
        .add_systems(Update, update_camera);

        // We yeet the schedule runner and never call app.run(),
        // so we have to finish and clean up ourselves
        app.finish();
        app.cleanup();

        // We grab the sub apps cus we dont want the runner, as we'll
        // be pumping the update loop ourselves manually.
        Self(std::mem::take(app.sub_apps_mut()))
    }

    fn new_render_target(&mut self, width: u32, height: u32) -> RenderTarget {
        let mut target = Image::new_uninit(
            Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            TextureFormat::Rgba8UnormSrgb,
            RenderAssetUsages::RENDER_WORLD,
        );
        // We're going to render to this image, mark it as such
        target.texture_descriptor.usage |= TextureUsages::RENDER_ATTACHMENT;
        self.0
            .main
            .world_mut()
            .resource_mut::<Assets<Image>>()
            .add(target)
            .into()
    }

    fn spawn_camera(&mut self, target: RenderTarget) -> Entity {
        self.0
            .main
            .world_mut()
            .spawn((Camera3d::default(), target, Transform::IDENTITY))
            .id()
    }

    fn update(&mut self) {
        self.0.update();
        // Wait for frame to finish rendering by wait polling the device
        self.0
            .main
            .world()
            .resource::<RenderDevice>()
            .wgpu_device()
            .poll(PollType::Wait {
                submission_index: None,
                timeout: None,
            })
            .unwrap();
    }

    fn screenshot(&mut self, target: RenderTarget, i: u32) {
        self.0
            .main
            .world_mut()
            .spawn(Screenshot::image(target.as_image().unwrap().clone()))
            .observe(save_to_disk(format!("test_images/screenshot{i}.png")));
    }
}

fn spawn_test_scene(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.spawn((
        Mesh3d(meshes.add(Circle::new(4.0))),
        MeshMaterial3d(materials.add(Color::WHITE)),
        Transform::from_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2)),
    ));
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(2.0, 2.0, 2.0))),
        MeshMaterial3d(materials.add(Color::srgb_u8(124, 144, 255))),
        Transform::from_xyz(0.0, 1.0, 0.0),
    ));
    commands.spawn((
        PointLight {
            shadow_maps_enabled: true,
            ..default()
        },
        Transform::from_xyz(4.0, 8.0, 4.0),
    ));
}

fn update_camera(mut camera: Query<&mut Transform, With<Camera>>, frame_count: Res<FrameCount>) {
    for mut t in camera.iter_mut() {
        let (s, c) = ops::sin_cos(frame_count.0 as f32 * 0.3);
        *t = Transform::from_xyz(s * 10.0, 4.5, c * 10.0).looking_at(Vec3::ZERO, Vec3::Y);
    }
}
