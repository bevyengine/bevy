//! This example shows how to place reflection probes in the scene.
//!
//! Press Space to switch between no reflections, environment map reflections
//! (i.e. the skybox only, not the cubes), and a full reflection probe that
//! reflects the skybox and the cubes. Press Enter to pause rotation.
//!
//! Reflection probes don't work on WebGL 2 or WebGPU.

use bevy::core_pipeline::Skybox;
use bevy::prelude::*;

use std::{
    f32::consts::PI,
    fmt::{Display, Formatter, Result as FmtResult},
};

static STOP_ROTATION_HELP_TEXT: &str = "Press Enter to stop rotation";
static START_ROTATION_HELP_TEXT: &str = "Press Enter to start rotation";

static REFLECTION_MODE_HELP_TEXT: &str = "Press Space to switch reflection mode";

// The mode the application is in.
#[derive(Resource)]
struct AppStatus {
    // Which environment maps the user has requested to display.
    reflection_mode: ReflectionMode,
    // Whether the user has requested the scene to rotate.
    rotating: bool,
}

// Which environment maps the user has requested to display.
#[derive(Clone, Copy)]
enum ReflectionMode {
    // No environment maps are shown.
    None = 0,
    // Only a world environment map is shown.
    EnvironmentMap = 1,
    // Both a world environment map and a reflection probe are present. The
    // reflection probe is shown in the sphere.
    ReflectionProbe = 2,
}

// The various reflection maps.
#[derive(Resource)]
struct Cubemaps {
    // The blurry diffuse cubemap. This is used for both the world environment
    // map and the reflection probe. (In reality you wouldn't do this, but this
    // reduces complexity of this example a bit.)
    diffuse: Handle<Image>,

    // The specular cubemap that reflects the world, but not the cubes.
    specular_environment_map: Handle<Image>,

    // The specular cubemap that reflects both the world and the cubes.
    specular_reflection_probe: Handle<Image>,

    // The skybox cubemap image. This is almost the same as
    // `specular_environment_map`.
    skybox: Handle<Image>,
}

fn main() {
    // Create the app.
    App::new()
        .add_plugins(DefaultPlugins)
        .init_resource::<AppStatus>()
        .init_resource::<Cubemaps>()
        .add_systems(Startup, setup)
        .add_systems(PreUpdate, add_environment_map_to_camera)
        .add_systems(Update, change_reflection_type)
        .add_systems(Update, toggle_rotation)
        .add_systems(
            Update,
            rotate_camera
                .after(toggle_rotation)
                .after(change_reflection_type),
        )
        .add_systems(Update, update_text.after(rotate_camera))
        .run();
}

// Spawns all the scene objects.
fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
    app_status: Res<AppStatus>,
    cubemaps: Res<Cubemaps>,
) {
    spawn_scene(&mut commands, &asset_server);
    spawn_camera(&mut commands);
    spawn_sphere(&mut commands, &mut meshes, &mut materials);
    spawn_reflection_probe(&mut commands, &cubemaps);
    spawn_text(&mut commands, &app_status);
}

// Spawns the cubes, light, and camera.
fn spawn_scene(commands: &mut Commands, asset_server: &AssetServer) {
    commands.spawn(SceneBundle {
        scene: asset_server.load(GltfAssetLabel::Scene(0).from_asset("models/cubes/Cubes.glb")),
        ..SceneBundle::default()
    });
}

// Spawns the camera.
fn spawn_camera(commands: &mut Commands) {
    commands.spawn(Camera3dBundle {
        camera: Camera {
            hdr: true,
            ..default()
        },
        transform: Transform::from_xyz(-6.483, 0.325, 4.381).looking_at(Vec3::ZERO, Vec3::Y),
        ..default()
    });
}

// Creates the sphere mesh and spawns it.
fn spawn_sphere(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
) {
    // Create a sphere mesh.
    let sphere_mesh = meshes.add(Sphere::new(1.0).mesh().ico(7).unwrap());

    // Create a sphere.
    commands.spawn(PbrBundle {
        mesh: sphere_mesh.clone(),
        material: materials.add(StandardMaterial {
            base_color: Srgba::hex("#ffd891").unwrap().into(),
            metallic: 1.0,
            perceptual_roughness: 0.0,
            ..StandardMaterial::default()
        }),
        transform: Transform::default(),
        ..PbrBundle::default()
    });
}

// Spawns the reflection probe.
fn spawn_reflection_probe(commands: &mut Commands, cubemaps: &Cubemaps) {
    commands.spawn(ReflectionProbeBundle {
        spatial: SpatialBundle {
            // 2.0 because the sphere's radius is 1.0 and we want to fully enclose it.
            transform: Transform::from_scale(Vec3::splat(2.0)),
            ..SpatialBundle::default()
        },
        light_probe: LightProbe,
        environment_map: EnvironmentMapLight {
            diffuse_map: cubemaps.diffuse.clone(),
            specular_map: cubemaps.specular_reflection_probe.clone(),
            intensity: 5000.0,
        },
    });
}

// Spawns the help text.
fn spawn_text(commands: &mut Commands, app_status: &AppStatus) {
    // Create the text.
    commands.spawn(
        TextBundle {
            text: app_status.create_text(),
            ..default()
        }
        .with_style(Style {
            position_type: PositionType::Absolute,
            bottom: Val::Px(12.0),
            left: Val::Px(12.0),
            ..default()
        }),
    );
}

// Adds a world environment map to the camera. This separate system is needed because the camera is
// managed by the scene spawner, as it's part of the glTF file with the cubes, so we have to add
// the environment map after the fact.
fn add_environment_map_to_camera(
    mut commands: Commands,
    query: Query<Entity, Added<Camera3d>>,
    cubemaps: Res<Cubemaps>,
) {
    for camera_entity in query.iter() {
        commands
            .entity(camera_entity)
            .insert(create_camera_environment_map_light(&cubemaps))
            .insert(Skybox {
                image: cubemaps.skybox.clone(),
                brightness: 5000.0,
            });
    }
}

// A system that handles switching between different reflection modes.
fn change_reflection_type(
    mut commands: Commands,
    light_probe_query: Query<Entity, With<LightProbe>>,
    camera_query: Query<Entity, With<Camera3d>>,
    keyboard: Res<ButtonInput<KeyCode>>,
    mut app_status: ResMut<AppStatus>,
    cubemaps: Res<Cubemaps>,
) {
    // Only do anything if space was pressed.
    if !keyboard.just_pressed(KeyCode::Space) {
        return;
    }

    // Switch reflection mode.
    app_status.reflection_mode =
        ReflectionMode::try_from((app_status.reflection_mode as u32 + 1) % 3).unwrap();

    // Add or remove the light probe.
    for light_probe in light_probe_query.iter() {
        commands.entity(light_probe).despawn();
    }
    match app_status.reflection_mode {
        ReflectionMode::None | ReflectionMode::EnvironmentMap => {}
        ReflectionMode::ReflectionProbe => spawn_reflection_probe(&mut commands, &cubemaps),
    }

    // Add or remove the environment map from the camera.
    for camera in camera_query.iter() {
        match app_status.reflection_mode {
            ReflectionMode::None => {
                commands.entity(camera).remove::<EnvironmentMapLight>();
            }
            ReflectionMode::EnvironmentMap | ReflectionMode::ReflectionProbe => {
                commands
                    .entity(camera)
                    .insert(create_camera_environment_map_light(&cubemaps));
            }
        }
    }
}

// A system that handles enabling and disabling rotation.
fn toggle_rotation(keyboard: Res<ButtonInput<KeyCode>>, mut app_status: ResMut<AppStatus>) {
    if keyboard.just_pressed(KeyCode::Enter) {
        app_status.rotating = !app_status.rotating;
    }
}

// A system that updates the help text.
fn update_text(mut text_query: Query<&mut Text>, app_status: Res<AppStatus>) {
    for mut text in text_query.iter_mut() {
        *text = app_status.create_text();
    }
}

impl TryFrom<u32> for ReflectionMode {
    type Error = ();

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(ReflectionMode::None),
            1 => Ok(ReflectionMode::EnvironmentMap),
            2 => Ok(ReflectionMode::ReflectionProbe),
            _ => Err(()),
        }
    }
}

impl Display for ReflectionMode {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> FmtResult {
        let text = match *self {
            ReflectionMode::None => "No reflections",
            ReflectionMode::EnvironmentMap => "Environment map",
            ReflectionMode::ReflectionProbe => "Reflection probe",
        };
        formatter.write_str(text)
    }
}

impl AppStatus {
    // Constructs the help text at the bottom of the screen based on the
    // application status.
    fn create_text(&self) -> Text {
        let rotation_help_text = if self.rotating {
            STOP_ROTATION_HELP_TEXT
        } else {
            START_ROTATION_HELP_TEXT
        };

        Text::from_section(
            format!(
                "{}\n{}\n{}",
                self.reflection_mode, rotation_help_text, REFLECTION_MODE_HELP_TEXT
            ),
            TextStyle::default(),
        )
    }
}

// Creates the world environment map light, used as a fallback if no reflection
// probe is applicable to a mesh.
fn create_camera_environment_map_light(cubemaps: &Cubemaps) -> EnvironmentMapLight {
    EnvironmentMapLight {
        diffuse_map: cubemaps.diffuse.clone(),
        specular_map: cubemaps.specular_environment_map.clone(),
        intensity: 5000.0,
    }
}

// Rotates the camera a bit every frame.
fn rotate_camera(
    time: Res<Time>,
    mut camera_query: Query<&mut Transform, With<Camera3d>>,
    app_status: Res<AppStatus>,
) {
    if !app_status.rotating {
        return;
    }

    for mut transform in camera_query.iter_mut() {
        transform.translation = Vec2::from_angle(time.delta_seconds() * PI / 5.0)
            .rotate(transform.translation.xz())
            .extend(transform.translation.y)
            .xzy();
        transform.look_at(Vec3::ZERO, Vec3::Y);
    }
}

// Loads the cubemaps from the assets directory.
impl FromWorld for Cubemaps {
    fn from_world(world: &mut World) -> Self {
        // Just use the specular map for the skybox since it's not too blurry.
        // In reality you wouldn't do this--you'd use a real skybox texture--but
        // reusing the textures like this saves space in the Bevy repository.
        let specular_map = world.load_asset("environment_maps/pisa_specular_rgb9e5_zstd.ktx2");

        Cubemaps {
            diffuse: world.load_asset("environment_maps/pisa_diffuse_rgb9e5_zstd.ktx2"),
            specular_reflection_probe: world
                .load_asset("environment_maps/cubes_reflection_probe_specular_rgb9e5_zstd.ktx2"),
            specular_environment_map: specular_map.clone(),
            skybox: specular_map,
        }
    }
}

impl Default for AppStatus {
    fn default() -> Self {
        Self {
            reflection_mode: ReflectionMode::ReflectionProbe,
            rotating: true,
        }
    }
}
