use bevy::math::vec3;
use bevy::pbr::irradiance_volume::IrradianceVolume;
use bevy::prelude::shape::UVSphere;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;

// Rotation speed in radians per frame.
const ROTATION_SPEED: f32 = 0.005;

const FOX_SCALE: f32 = 0.05;
const SPHERE_SCALE: f32 = 2.0;

const IRRADIANCE_VOLUME_INTENSITY: f32 = 150.0;

static DISABLE_IRRADIANCE_VOLUME_HELP_TEXT: &str = "Press Space to disable the irradiance volume";
static ENABLE_IRRADIANCE_VOLUME_HELP_TEXT: &str = "Press Space to enable the irradiance volume";

static STOP_ROTATION_HELP_TEXT: &str = "Press Enter to stop rotation";
static START_ROTATION_HELP_TEXT: &str = "Press Enter to start rotation";

static SWITCH_TO_FOX_HELP_TEXT: &str = "Press Tab to switch to a skinned mesh";
static SWITCH_TO_SPHERE_HELP_TEXT: &str = "Press Tab to switch to a plain sphere mesh";

static CLICK_TO_MOVE_HELP_TEXT: &str = "Click to move the object";

// The mode the application is in.
#[derive(Resource)]
struct AppStatus {
    irradiance_volume_present: bool,
    fox_present: bool,
    // Whether the user has requested the scene to rotate.
    rotating: bool,
}

#[derive(Resource)]
struct ExampleAssets {
    sphere: Handle<Mesh>,
    fox: Handle<Scene>,
    main_material: Handle<StandardMaterial>,
    main_scene: Handle<Scene>,
    irradiance_volume: Handle<Image>,
    fox_animation: Handle<AnimationClip>,
}

#[derive(Component)]
struct MainObject;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
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
            toggle_rotation.after(rotate_camera).after(play_animations),
        )
        .add_systems(
            Update,
            update_text
                .after(handle_mouse_clicks)
                .after(change_main_object)
                .after(toggle_irradiance_volumes)
                .after(toggle_rotation),
        )
        .run();
}

fn setup(
    mut commands: Commands,
    assets: Res<ExampleAssets>,
    app_status: Res<AppStatus>,
    asset_server: Res<AssetServer>,
) {
    commands.spawn(SceneBundle {
        scene: assets.main_scene.clone(),
        ..SceneBundle::default()
    });

    commands.spawn(Camera3dBundle {
        transform: Transform::from_xyz(-10.012, 4.8605, 13.281).looking_at(Vec3::ZERO, Vec3::Y),
        ..default()
    });

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

    commands.spawn(PointLightBundle {
        point_light: PointLight {
            intensity: 250000.0,
            shadows_enabled: true,
            ..default()
        },
        transform: Transform::from_xyz(4.0762, 5.9039, 1.0055),
        ..default()
    });

    commands
        .spawn(PbrBundle {
            mesh: assets.sphere.clone(),
            material: assets.main_material.clone(),
            transform: Transform::from_xyz(0.0, SPHERE_SCALE, 0.0)
                .with_scale(Vec3::splat(SPHERE_SCALE)),
            ..default()
        })
        .insert(MainObject);

    commands
        .spawn(SceneBundle {
            scene: assets.fox.clone(),
            visibility: Visibility::Hidden,
            transform: Transform::from_scale(Vec3::splat(FOX_SCALE)),
            ..default()
        })
        .insert(MainObject);

    commands.spawn(
        TextBundle {
            text: app_status.create_text(&asset_server),
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

        let rotation_help_text = if self.rotating {
            STOP_ROTATION_HELP_TEXT
        } else {
            START_ROTATION_HELP_TEXT
        };

        let switch_mesh_help_text = if self.fox_present {
            SWITCH_TO_SPHERE_HELP_TEXT
        } else {
            SWITCH_TO_FOX_HELP_TEXT
        };

        Text::from_section(
            format!(
                "{}\n{}\n{}\n{}",
                CLICK_TO_MOVE_HELP_TEXT,
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

    if !app_status.fox_present {
        *sphere_visibility = Visibility::Hidden;
        *fox_visibility = Visibility::Visible;
        app_status.fox_present = true;
    } else {
        *sphere_visibility = Visibility::Visible;
        *fox_visibility = Visibility::Hidden;
        app_status.fox_present = false;
    }
}

impl Default for AppStatus {
    fn default() -> Self {
        Self {
            irradiance_volume_present: true,
            fox_present: false,
            rotating: true,
        }
    }
}

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

    let Some(ray) = camera.viewport_to_world(camera_transform, mouse_position) else {
        return;
    };
    let Some(ray_distance) = ray.intersect_plane(Vec3::ZERO, Plane3d::new(Vec3::Y)) else {
        return;
    };
    let plane_intersection = ray.origin + ray.direction.normalize() * ray_distance;

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
            main_material,
            main_scene,
            irradiance_volume,
            fox_animation,
        }
    }
}

fn play_animations(assets: Res<ExampleAssets>, mut players: Query<&mut AnimationPlayer>) {
    for mut player in players.iter_mut() {
        player.play(assets.fox_animation.clone()).repeat();
    }
}
