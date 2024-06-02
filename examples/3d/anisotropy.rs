//! Demonstrates anisotropy with the glTF sample barn lamp model.

use bevy::{
    color::palettes::css::WHITE, core_pipeline::Skybox, math::vec3, prelude::*, time::Stopwatch,
};

/// The initial position of the camera.
const CAMERA_INITIAL_POSITION: Vec3 = vec3(-0.4, 0.0, 0.0);

/// The current settings of the app, as chosen by the user.
#[derive(Resource)]
struct AppStatus {
    /// Which type of light is in the scene.
    light_mode: LightMode,
    /// Whether anisotropy is enabled.
    anisotropy_enabled: bool,
}

/// Which type of light we're using: a directional light, a point light, or an
/// environment map.
#[derive(Clone, Copy, PartialEq, Default)]
enum LightMode {
    /// A rotating directional light.
    #[default]
    Directional,
    /// A rotating point light.
    Point,
    /// An environment map (image-based lighting, including skybox).
    EnvironmentMap,
}

/// A component that stores the version of the material with anisotropy and the
/// version of the material without it.
///
/// This is placed on each mesh with a material. It exists so that the
/// appropriate system can replace the materials when the user presses Enter to
/// turn anisotropy on and off.
#[derive(Component)]
struct MaterialVariants {
    /// The version of the material in the glTF file, with anisotropy.
    anisotropic: Handle<StandardMaterial>,
    /// The version of the material with anisotropy removed.
    isotropic: Handle<StandardMaterial>,
}

/// The application entry point.
fn main() {
    App::new()
        .init_resource::<AppStatus>()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Bevy Anisotropy Example".into(),
                ..default()
            }),
            ..default()
        }))
        .add_systems(Startup, setup)
        .add_systems(Update, create_material_variants)
        .add_systems(Update, animate_light)
        .add_systems(Update, rotate_camera)
        .add_systems(Update, (handle_input, update_help_text).chain())
        .run();
}

/// Creates the initial scene.
fn setup(mut commands: Commands, asset_server: Res<AssetServer>, app_status: Res<AppStatus>) {
    commands.spawn(Camera3dBundle {
        transform: Transform::from_translation(CAMERA_INITIAL_POSITION)
            .looking_at(Vec3::ZERO, Vec3::Y),
        ..default()
    });

    spawn_directional_light(&mut commands);

    commands.spawn(SceneBundle {
        scene: asset_server.load("models/AnisotropyBarnLamp/AnisotropyBarnLamp.gltf#Scene0"),
        transform: Transform::from_xyz(0.0, 0.07, -0.13),
        ..default()
    });

    spawn_text(&mut commands, &asset_server, &app_status);
}

/// Spawns the help text.
fn spawn_text(commands: &mut Commands, asset_server: &AssetServer, app_status: &AppStatus) {
    commands.spawn(
        TextBundle {
            text: app_status.create_help_text(asset_server),
            ..default()
        }
        .with_style(Style {
            position_type: PositionType::Absolute,
            bottom: Val::Px(10.0),
            left: Val::Px(10.0),
            ..default()
        }),
    );
}

/// For each material, creates a version with the anisotropy removed.
///
/// This allows the user to press Enter to toggle anisotropy on and off.
fn create_material_variants(
    mut commands: Commands,
    mut materials: ResMut<Assets<StandardMaterial>>,
    new_meshes: Query<
        (Entity, &Handle<StandardMaterial>),
        (Added<Handle<StandardMaterial>>, Without<MaterialVariants>),
    >,
) {
    for (entity, anisotropic_material_handle) in new_meshes.iter() {
        let Some(anisotropic_material) = materials.get(anisotropic_material_handle).cloned() else {
            continue;
        };

        commands.entity(entity).insert(MaterialVariants {
            anisotropic: anisotropic_material_handle.clone(),
            isotropic: materials.add(StandardMaterial {
                anisotropy_texture: None,
                anisotropy_strength: 0.0,
                anisotropy_rotation: 0.0,
                ..anisotropic_material
            }),
        });
    }
}

/// A system that animates the light every frame, if there is one.
fn animate_light(
    mut lights: Query<&mut Transform, Or<(With<DirectionalLight>, With<PointLight>)>>,
    time: Res<Time>,
) {
    let now = time.elapsed_seconds();
    for mut transform in lights.iter_mut() {
        transform.translation = vec3(f32::cos(now), 1.0, f32::sin(now)) * vec3(3.0, 4.0, 3.0);
        transform.look_at(Vec3::ZERO, Vec3::Y);
    }
}

/// A system that rotates the camera if the environment map is enabled.
fn rotate_camera(
    mut camera: Query<&mut Transform, With<Camera>>,
    app_status: Res<AppStatus>,
    time: Res<Time>,
    mut stopwatch: Local<Stopwatch>,
) {
    if app_status.light_mode == LightMode::EnvironmentMap {
        stopwatch.tick(time.delta());
    }

    let now = stopwatch.elapsed_secs();
    for mut transform in camera.iter_mut() {
        *transform = Transform::from_translation(
            Quat::from_rotation_y(now).mul_vec3(CAMERA_INITIAL_POSITION),
        )
        .looking_at(Vec3::ZERO, Vec3::Y);
    }
}

/// Handles requests from the user to change the lighting or toggle anisotropy.
fn handle_input(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    cameras: Query<Entity, With<Camera>>,
    lights: Query<Entity, Or<(With<DirectionalLight>, With<PointLight>)>>,
    mut meshes: Query<(&mut Handle<StandardMaterial>, &MaterialVariants)>,
    keyboard: Res<ButtonInput<KeyCode>>,
    mut app_status: ResMut<AppStatus>,
) {
    // If Space was pressed, change the lighting.
    if keyboard.just_pressed(KeyCode::Space) {
        match app_status.light_mode {
            LightMode::Directional => {
                // Switch to a point light. Despawn all existing lights and
                // create the light point.
                app_status.light_mode = LightMode::Point;
                for light in lights.iter() {
                    commands.entity(light).despawn();
                }
                spawn_point_light(&mut commands);
            }

            LightMode::Point => {
                // Switch to the environment map. Despawn all existing lights,
                // and create the skybox and environment map.
                app_status.light_mode = LightMode::EnvironmentMap;
                for light in lights.iter() {
                    commands.entity(light).despawn();
                }
                for camera in cameras.iter() {
                    add_skybox_and_environment_map(&mut commands, &asset_server, camera);
                }
            }

            LightMode::EnvironmentMap => {
                // Switch back to a directional light. Despawn the skybox and
                // environment map light, and recreate the directional light.
                app_status.light_mode = LightMode::Directional;
                for camera in cameras.iter() {
                    commands
                        .entity(camera)
                        .remove::<Skybox>()
                        .remove::<EnvironmentMapLight>();
                }
                spawn_directional_light(&mut commands);
            }
        }
    }

    // If Enter was pressed, toggle anisotropy on and off.
    if keyboard.just_pressed(KeyCode::Enter) {
        app_status.anisotropy_enabled = !app_status.anisotropy_enabled;

        // Go through each mesh and alter its material.
        for (mut material_handle, material_variants) in meshes.iter_mut() {
            *material_handle = if app_status.anisotropy_enabled {
                material_variants.anisotropic.clone()
            } else {
                material_variants.isotropic.clone()
            }
        }
    }
}

/// A system that updates the help text based on the current app status.
fn update_help_text(
    mut text_query: Query<&mut Text>,
    app_status: Res<AppStatus>,
    asset_server: Res<AssetServer>,
) {
    for mut text in text_query.iter_mut() {
        *text = app_status.create_help_text(&asset_server);
    }
}

/// Adds the skybox and environment map to the scene.
fn add_skybox_and_environment_map(
    commands: &mut Commands,
    asset_server: &AssetServer,
    entity: Entity,
) {
    commands
        .entity(entity)
        .insert(Skybox {
            brightness: 5000.0,
            image: asset_server.load("environment_maps/pisa_specular_rgb9e5_zstd.ktx2"),
        })
        .insert(EnvironmentMapLight {
            diffuse_map: asset_server.load("environment_maps/pisa_diffuse_rgb9e5_zstd.ktx2"),
            specular_map: asset_server.load("environment_maps/pisa_specular_rgb9e5_zstd.ktx2"),
            intensity: 2500.0,
        });
}

/// Spawns a rotating directional light.
fn spawn_directional_light(commands: &mut Commands) {
    commands.spawn(DirectionalLightBundle {
        directional_light: DirectionalLight {
            color: WHITE.into(),
            illuminance: 3000.0,
            ..default()
        },
        ..default()
    });
}

/// Spawns a rotating point light.
fn spawn_point_light(commands: &mut Commands) {
    commands.spawn(PointLightBundle {
        point_light: PointLight {
            color: WHITE.into(),
            intensity: 200000.0,
            ..default()
        },
        ..default()
    });
}

impl AppStatus {
    /// Creates the help text as appropriate for the current app status.
    fn create_help_text(&self, asset_server: &AssetServer) -> Text {
        // Choose the appropriate help text for the anisotropy toggle.
        let material_variant_help_text = if self.anisotropy_enabled {
            "Press Enter to disable anisotropy"
        } else {
            "Press Enter to enable anisotropy"
        };

        // Choose the appropriate help text for the light toggle.
        let light_help_text = match self.light_mode {
            LightMode::Directional => "Press Space to switch to a point light",
            LightMode::Point => "Press Space to switch to an environment map",
            LightMode::EnvironmentMap => "Press Space to switch to a directional light",
        };

        // Build the `Text` object.
        Text::from_section(
            format!("{}\n{}", material_variant_help_text, light_help_text),
            TextStyle {
                font: asset_server.load("fonts/FiraMono-Medium.ttf"),
                font_size: 24.0,
                ..default()
            },
        )
    }
}

impl Default for AppStatus {
    fn default() -> Self {
        Self {
            light_mode: default(),
            anisotropy_enabled: true,
        }
    }
}
