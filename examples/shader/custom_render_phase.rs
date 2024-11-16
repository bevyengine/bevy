use bevy::{
    core_pipeline::oit::OrderIndependentTransparencySettings, prelude::*, scene::SceneInstance,
    window::WindowResized,
};
use bevy_render::camera::Viewport;
use rand::Rng;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                resolution: Vec2::new(1920.0, 1080.0).into(),
                ..default()
            }),
            ..default()
        }))
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (exit, set_camera_viewports, rotate, replace_scene_materials),
        )
        .run();
}

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // circular base
    //commands.spawn((
    //    Mesh3d(meshes.add(Circle::new(4.0))),
    //    MeshMaterial3d(materials.add(Color::WHITE)),
    //    Transform::from_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2)),
    //));
    // light
    commands.spawn((
        PointLight {
            shadows_enabled: true,
            ..default()
        },
        Transform::from_xyz(4.0, 8.0, 4.0),
    ));

    commands.spawn((
        SceneRoot(
            asset_server
                .load(GltfAssetLabel::Scene(0).from_asset("models/reconstructed_engine.glb")),
        ),
        //Transform::from_scale(Vec3::splat(0.035)),
        Transform::from_scale(Vec3::splat(0.85)),
        Rotate,
        ReplaceStandardMaterial,
    ));

    let camera_pos =
        Transform::from_xyz(0.0, 5.0, 10.0).looking_at(Vec3::new(0.0, 0.5, 0.0), Vec3::Y);
    let text = ["OIT enabled", "OIT disabled"];
    for index in 0..2 {
        let mut camera = commands.spawn((
            Camera3d::default(),
            camera_pos.clone(),
            Camera {
                order: index as isize,
                ..default()
            },
            CameraPosition {
                pos: UVec2::new((index % 2) as u32, (index / 2) as u32),
            },
            Msaa::Off,
        ));
        if index == 0 {
            camera.insert(OrderIndependentTransparencySettings {
                layer_count: 32,
                //alpha_threshold: 0.05,
                ..default()
            });
        }
        let camera_id = camera.id();
        // Set up UI
        commands
            .spawn((
                TargetCamera(camera_id),
                Node {
                    width: Val::Percent(100.),
                    height: Val::Percent(100.),
                    ..default()
                },
            ))
            .with_children(|parent| {
                parent.spawn((
                    Text::new(text[index]),
                    TextFont {
                        font_size: 24.0,
                        ..default()
                    },
                    Node {
                        position_type: PositionType::Absolute,
                        top: Val::Px(12.),
                        left: Val::Px(12.),
                        ..default()
                    },
                ));
            });
    }
}

fn exit(input: Res<ButtonInput<KeyCode>>) {
    if input.just_pressed(KeyCode::KeyQ) {
        std::process::exit(0);
    }
}

#[derive(Component)]
struct CameraPosition {
    pos: UVec2,
}

fn set_camera_viewports(
    windows: Query<&Window>,
    mut resize_events: EventReader<WindowResized>,
    mut query: Query<(&CameraPosition, &mut Camera)>,
) {
    // We need to dynamically resize the camera's viewports whenever the window size changes
    // so then each camera always takes up half the screen.
    // A resize_event is sent when the window is first created, allowing us to reuse this system for initial setup.
    for resize_event in resize_events.read() {
        let window = windows.get(resize_event.window).unwrap();
        let size = UVec2::new(window.physical_size().x / 2, window.physical_size().y);

        for (camera_position, mut camera) in &mut query {
            camera.viewport = Some(Viewport {
                physical_position: camera_position.pos * size,
                physical_size: size,
                ..default()
            });
        }
    }
}
#[derive(Component)]
struct Rotate;

fn rotate(mut q: Query<&mut Transform, With<Rotate>>, time: Res<Time>) {
    for mut transform in &mut q {
        transform.rotate_axis(Dir3::Y, std::f32::consts::PI * time.delta_secs() * 0.25);
    }
}

#[derive(Component)]
struct ReplaceStandardMaterial;
fn replace_scene_materials(
    mut commands: Commands,
    unloaded_instances: Query<(Entity, &SceneInstance), With<ReplaceStandardMaterial>>,
    handles: Query<(Entity, &MeshMaterial3d<StandardMaterial>)>,
    mut pbr_materials: ResMut<Assets<StandardMaterial>>,
    scene_manager: Res<SceneSpawner>,
) {
    for (entity, instance) in unloaded_instances.iter() {
        println!("replace");
        if scene_manager.instance_is_ready(**instance) {
            commands.entity(entity).remove::<ReplaceStandardMaterial>();
        }
        let handles = handles.iter_many(scene_manager.iter_instance_entities(**instance));
        let mut rng = rand::thread_rng();
        for (_entity, material_handle) in handles {
            let Some(material) = pbr_materials.get_mut(material_handle) else {
                continue;
            };
            // TODO random colors
            //material.base_color.set_alpha(0.25);
            material.base_color = Color::srgba(rng.gen(), rng.gen(), rng.gen(), 0.25);
            material.alpha_mode = AlphaMode::Blend;
        }
    }
}
