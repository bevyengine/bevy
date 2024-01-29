//! This example shows how irradiance volumes affect the indirect lighting of
//! objects in a scene.
//!
//! The controls are as follows:
//!
//! * Space toggles the irradiance volume on and off.
//!
//! * Enter toggles the camera rotation on and off.
//!
//! * Tab switches the object between a plain sphere and a running fox.
//!
//! * Clicking anywhere moves the object.

use bevy::math::{uvec3, vec3};
use bevy::pbr::irradiance_volume::IrradianceVolume;
use bevy::prelude::shape::UVSphere;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;

// Rotation speed in radians per frame.
const ROTATION_SPEED: f32 = 0.005;

const FOX_SCALE: f32 = 0.05;
const SPHERE_SCALE: f32 = 2.0;

const IRRADIANCE_VOLUME_INTENSITY: f32 = 150.0;

const VOXEL_GIZMO_RADIUS: f32 = 0.1;

static DISABLE_IRRADIANCE_VOLUME_HELP_TEXT: &str = "Space: Disable the irradiance volume";
static ENABLE_IRRADIANCE_VOLUME_HELP_TEXT: &str = "Space: Enable the irradiance volume";

static HIDE_GIZMO_HELP_TEXT: &str = "Backspace: Hide the voxels";
static SHOW_GIZMO_HELP_TEXT: &str = "Backspace: Show the voxels";

static STOP_ROTATION_HELP_TEXT: &str = "Enter: Stop rotation";
static START_ROTATION_HELP_TEXT: &str = "Enter: Start rotation";

static SWITCH_TO_FOX_HELP_TEXT: &str = "Tab: Switch to a skinned mesh";
static SWITCH_TO_SPHERE_HELP_TEXT: &str = "Tab: Switch to a plain sphere mesh";

static CLICK_TO_MOVE_HELP_TEXT: &str = "Left click: Move the object";

static GIZMO_COLOR: Color = Color::YELLOW;

// The mode the application is in.
#[derive(Resource)]
struct AppStatus {
    // Whether the user wants the irradiance volume to be applied.
    irradiance_volume_present: bool,
    // Whether the user wants the unskinned sphere mesh or the skinned fox mesh.
    model: ExampleModel,
    // Whether the user has requested the scene to rotate.
    rotating: bool,
    // Whether the user has requested the voxels gizmo to be displayed.
    voxels_gizmo_visible: bool,
}

// Which model the user wants to display.
#[derive(Clone, Copy, PartialEq)]
enum ExampleModel {
    // The plain sphere.
    Sphere,
    // The fox, which is skinned.
    Fox,
}

// Handles to all the assets used in this example.
#[derive(Resource)]
struct ExampleAssets {
    // The glTF scene containing the colored floor.
    main_scene: Handle<Scene>,
    // The 3D texture containing the irradiance volume.
    irradiance_volume: Handle<Image>,
    // The plain sphere mesh.
    sphere: Handle<Mesh>,
    // The material used for the sphere.
    sphere_material: Handle<StandardMaterial>,
    // The glTF scene containing the animated fox.
    fox: Handle<Scene>,
    // The animation that the fox will play.
    fox_animation: Handle<AnimationClip>,
}

// The sphere and fox both have this component.
#[derive(Component)]
struct MainObject;

fn main() {
    // Create the example app.
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Bevy Irradiance Volumes Example".into(),
                ..default()
            }),
            ..default()
        }))
        .init_resource::<AppStatus>()
        .init_resource::<ExampleAssets>()
        .add_systems(Startup, setup)
        .add_systems(Update, rotate_camera)
        .add_systems(Update, play_animations)
        .add_systems(
            Update,
            handle_mouse_clicks
                .after(rotate_camera)
                .after(play_animations),
        )
        .add_systems(
            Update,
            change_main_object
                .after(rotate_camera)
                .after(play_animations),
        )
        .add_systems(
            Update,
            toggle_irradiance_volumes
                .after(rotate_camera)
                .after(play_animations),
        )
        .add_systems(
            Update,
            toggle_gizmos.after(rotate_camera).after(play_animations),
        )
        .add_systems(
            Update,
            toggle_rotation.after(rotate_camera).after(play_animations),
        )
        .add_systems(
            Update,
            draw_gizmos
                .after(handle_mouse_clicks)
                .after(change_main_object)
                .after(toggle_irradiance_volumes)
                .after(toggle_gizmos)
                .after(toggle_rotation),
        )
        .add_systems(
            Update,
            update_text
                .after(handle_mouse_clicks)
                .after(change_main_object)
                .after(toggle_irradiance_volumes)
                .after(toggle_gizmos)
                .after(toggle_rotation),
        )
        .run();
}

// Spawns all the scene objects.
fn setup(
    mut commands: Commands,
    assets: Res<ExampleAssets>,
    app_status: Res<AppStatus>,
    asset_server: Res<AssetServer>,
) {
    spawn_main_scene(&mut commands, &assets);
    spawn_camera(&mut commands);
    spawn_irradiance_volume(&mut commands, &assets);
    spawn_light(&mut commands);
    spawn_sphere(&mut commands, &assets);
    spawn_fox(&mut commands, &assets);
    spawn_text(&mut commands, &app_status, &asset_server);
}

fn spawn_main_scene(commands: &mut Commands, assets: &ExampleAssets) {
    commands.spawn(SceneBundle {
        scene: assets.main_scene.clone(),
        ..SceneBundle::default()
    });
}

fn spawn_camera(commands: &mut Commands) {
    commands.spawn(Camera3dBundle {
        transform: Transform::from_xyz(-10.012, 4.8605, 13.281).looking_at(Vec3::ZERO, Vec3::Y),
        ..default()
    });
}

fn spawn_irradiance_volume(commands: &mut Commands, assets: &ExampleAssets) {
    commands
        .spawn(SpatialBundle {
            transform: Transform::from_matrix(Mat4::from_cols_array_2d(&[
                [-42.317566, 0.0, 0.0, 0.0],
                [0.0, 0.0, 44.601563, 0.0],
                [0.0, 16.73776, 0.0, 0.0],
                [0.0, 6.544792, 0.0, 1.0],
            ])),
            ..SpatialBundle::default()
        })
        .insert(IrradianceVolume {
            voxels: assets.irradiance_volume.clone(),
            intensity: IRRADIANCE_VOLUME_INTENSITY,
        })
        .insert(LightProbe);
}

fn spawn_light(commands: &mut Commands) {
    commands.spawn(PointLightBundle {
        point_light: PointLight {
            intensity: 250000.0,
            shadows_enabled: true,
            ..default()
        },
        transform: Transform::from_xyz(4.0762, 5.9039, 1.0055),
        ..default()
    });
}

fn spawn_sphere(commands: &mut Commands, assets: &ExampleAssets) {
    commands
        .spawn(PbrBundle {
            mesh: assets.sphere.clone(),
            material: assets.sphere_material.clone(),
            transform: Transform::from_xyz(0.0, SPHERE_SCALE, 0.0)
                .with_scale(Vec3::splat(SPHERE_SCALE)),
            ..default()
        })
        .insert(MainObject);
}

fn spawn_fox(commands: &mut Commands, assets: &ExampleAssets) {
    commands
        .spawn(SceneBundle {
            scene: assets.fox.clone(),
            visibility: Visibility::Hidden,
            transform: Transform::from_scale(Vec3::splat(FOX_SCALE)),
            ..default()
        })
        .insert(MainObject);
}

fn spawn_text(commands: &mut Commands, app_status: &AppStatus, asset_server: &AssetServer) {
    commands.spawn(
        TextBundle {
            text: app_status.create_text(asset_server),
            ..TextBundle::default()
        }
        .with_style(Style {
            position_type: PositionType::Absolute,
            bottom: Val::Px(10.0),
            left: Val::Px(10.0),
            ..default()
        }),
    );
}

// A system that updates the help text.
fn update_text(
    mut text_query: Query<&mut Text>,
    app_status: Res<AppStatus>,
    asset_server: Res<AssetServer>,
) {
    for mut text in text_query.iter_mut() {
        *text = app_status.create_text(&asset_server);
    }
}

impl AppStatus {
    // Constructs the help text at the bottom of the screen based on the
    // application status.
    fn create_text(&self, asset_server: &AssetServer) -> Text {
        let irradiance_volume_help_text = if self.irradiance_volume_present {
            DISABLE_IRRADIANCE_VOLUME_HELP_TEXT
        } else {
            ENABLE_IRRADIANCE_VOLUME_HELP_TEXT
        };

        let voxels_gizmo_help_text = if self.voxels_gizmo_visible {
            HIDE_GIZMO_HELP_TEXT
        } else {
            SHOW_GIZMO_HELP_TEXT
        };

        let rotation_help_text = if self.rotating {
            STOP_ROTATION_HELP_TEXT
        } else {
            START_ROTATION_HELP_TEXT
        };

        let switch_mesh_help_text = match self.model {
            ExampleModel::Sphere => SWITCH_TO_FOX_HELP_TEXT,
            ExampleModel::Fox => SWITCH_TO_SPHERE_HELP_TEXT,
        };

        Text::from_section(
            format!(
                "{}\n{}\n{}\n{}\n{}",
                CLICK_TO_MOVE_HELP_TEXT,
                voxels_gizmo_help_text,
                irradiance_volume_help_text,
                rotation_help_text,
                switch_mesh_help_text
            ),
            TextStyle {
                font: asset_server.load("fonts/FiraMono-Medium.ttf"),
                font_size: 24.0,
                color: Color::ANTIQUE_WHITE,
            },
        )
    }
}

// Rotates the camera a bit every frame.
fn rotate_camera(
    mut camera_query: Query<&mut Transform, With<Camera3d>>,
    app_status: Res<AppStatus>,
) {
    if !app_status.rotating {
        return;
    }

    for mut transform in camera_query.iter_mut() {
        transform.translation = Vec2::from_angle(ROTATION_SPEED)
            .rotate(transform.translation.xz())
            .extend(transform.translation.y)
            .xzy();
        transform.look_at(Vec3::ZERO, Vec3::Y);
    }
}

// Toggles between the unskinned sphere model and the skinned fox model if the
// user requests it.
fn change_main_object(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut app_status: ResMut<AppStatus>,
    mut sphere_query: Query<
        &mut Visibility,
        (With<MainObject>, With<Handle<Mesh>>, Without<Handle<Scene>>),
    >,
    mut fox_query: Query<&mut Visibility, (With<MainObject>, With<Handle<Scene>>)>,
) {
    if !keyboard.just_pressed(KeyCode::Tab) {
        return;
    }
    let Some(mut sphere_visibility) = sphere_query.iter_mut().next() else {
        return;
    };
    let Some(mut fox_visibility) = fox_query.iter_mut().next() else {
        return;
    };

    match app_status.model {
        ExampleModel::Sphere => {
            *sphere_visibility = Visibility::Hidden;
            *fox_visibility = Visibility::Visible;
            app_status.model = ExampleModel::Fox;
        }
        ExampleModel::Fox => {
            *sphere_visibility = Visibility::Visible;
            *fox_visibility = Visibility::Hidden;
            app_status.model = ExampleModel::Sphere;
        }
    }
}

impl Default for AppStatus {
    fn default() -> Self {
        Self {
            irradiance_volume_present: true,
            rotating: true,
            model: ExampleModel::Sphere,
            voxels_gizmo_visible: false,
        }
    }
}

// Turns on and off the irradiance volume as requested by the user.
fn toggle_irradiance_volumes(
    mut commands: Commands,
    keyboard: Res<ButtonInput<KeyCode>>,
    light_probe_query: Query<Entity, With<LightProbe>>,
    mut app_status: ResMut<AppStatus>,
    assets: Res<ExampleAssets>,
) {
    if !keyboard.just_pressed(KeyCode::Space) {
        return;
    };

    let Some(light_probe) = light_probe_query.iter().next() else {
        return;
    };

    if app_status.irradiance_volume_present {
        commands.entity(light_probe).remove::<IrradianceVolume>();
        app_status.irradiance_volume_present = false;
    } else {
        commands.entity(light_probe).insert(IrradianceVolume {
            voxels: assets.irradiance_volume.clone(),
            intensity: IRRADIANCE_VOLUME_INTENSITY,
        });
        app_status.irradiance_volume_present = true;
    }
}

fn toggle_rotation(keyboard: Res<ButtonInput<KeyCode>>, mut app_status: ResMut<AppStatus>) {
    if keyboard.just_pressed(KeyCode::Enter) {
        app_status.rotating = !app_status.rotating;
    }
}

// Handles clicks on the plane that reposition the object.
fn handle_mouse_clicks(
    buttons: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    cameras: Query<(&Camera, &GlobalTransform)>,
    mut main_objects: Query<&mut Transform, With<MainObject>>,
) {
    if !buttons.pressed(MouseButton::Left) {
        return;
    }
    let Some(mouse_position) = windows
        .iter()
        .next()
        .and_then(|window| window.cursor_position())
    else {
        return;
    };
    let Some((camera, camera_transform)) = cameras.iter().next() else {
        return;
    };

    // Figure out where the user clicked on the plane.
    let Some(ray) = camera.viewport_to_world(camera_transform, mouse_position) else {
        return;
    };
    let Some(ray_distance) = ray.intersect_plane(Vec3::ZERO, Plane3d::new(Vec3::Y)) else {
        return;
    };
    let plane_intersection = ray.origin + ray.direction.normalize() * ray_distance;

    // Move all the main objeccts.
    for mut transform in main_objects.iter_mut() {
        transform.translation = vec3(
            plane_intersection.x,
            transform.translation.y,
            plane_intersection.z,
        );
    }
}

impl FromWorld for ExampleAssets {
    fn from_world(world: &mut World) -> Self {
        // Load all the assets.
        let asset_server = world.resource::<AssetServer>();
        let fox = asset_server.load("models/animated/Fox.glb#Scene0");
        let main_scene =
            asset_server.load("models/IrradianceVolumeExample/IrradianceVolumeExample.glb#Scene0");
        let irradiance_volume = asset_server.load::<Image>("irradiance_volumes/Example.vxgi.ktx2");
        let fox_animation =
            asset_server.load::<AnimationClip>("models/animated/Fox.glb#Animation1");

        let mut mesh_assets = world.resource_mut::<Assets<Mesh>>();
        let sphere = mesh_assets.add(UVSphere::default());

        let mut standard_material_assets = world.resource_mut::<Assets<StandardMaterial>>();
        let main_material = standard_material_assets.add(Color::SILVER);

        ExampleAssets {
            sphere,
            fox,
            sphere_material: main_material,
            main_scene,
            irradiance_volume,
            fox_animation,
        }
    }
}

// Plays the animation on the fox.
fn play_animations(assets: Res<ExampleAssets>, mut players: Query<&mut AnimationPlayer>) {
    for mut player in players.iter_mut() {
        // This will safely do nothing if the animation is already playing.
        player.play(assets.fox_animation.clone()).repeat();
    }
}

fn draw_gizmos(
    mut gizmos: Gizmos,
    irradiance_volume_query: Query<(&GlobalTransform, &IrradianceVolume)>,
    camera_query: Query<&GlobalTransform, With<Camera>>,
    images: Res<Assets<Image>>,
    app_status: Res<AppStatus>,
) {
    if !app_status.voxels_gizmo_visible {
        return;
    }

    let Some(camera_pos) = camera_query.iter().map(|transform| transform.translation()).next() else { return };

    for (transform, irradiance_volume) in irradiance_volume_query.iter() {
        gizmos.cuboid(*transform, GIZMO_COLOR);

        let Some(image) = images.get(&irradiance_volume.voxels) else { continue };
        let resolution = image.texture_descriptor.size;
        let scale = vec3(1.0 / resolution.width as f32, 1.0 / resolution.height as f32, 1.0 / resolution.depth_or_array_layers as f32);

        for z in 0..resolution.depth_or_array_layers {
            for y in 0..resolution.height {
                for x in 0..resolution.width {
                    let uvw = (uvec3(x, y, z).as_vec3() + 0.5) * scale - 0.5;
                    let pos = transform.transform_point(uvw);
                    let Ok(normal) = Direction3d::new(camera_pos - pos) else { continue };
                    gizmos.circle(pos, normal, VOXEL_GIZMO_RADIUS, GIZMO_COLOR);
                }
            }
        }
    }
}

fn toggle_gizmos(keyboard: Res<ButtonInput<KeyCode>>, mut app_status: ResMut<AppStatus>) {
    if keyboard.just_pressed(KeyCode::Backspace) {
        app_status.voxels_gizmo_visible = !app_status.voxels_gizmo_visible;
    }
}
