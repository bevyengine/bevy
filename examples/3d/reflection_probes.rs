//! This example shows how to place reflection probes in the scene.
//!
//! Press Space to cycle through the reflection modes:
//!
//! 1. A pre-generated [`EnvironmentMapLight`] acting as a reflection probe, with both the skybox and cubes
//! 2. A runtime-generated [`GeneratedEnvironmentMapLight`] acting as a reflection probe with just the skybox
//! 3. A pre-generated [`EnvironmentMapLight`] with just the skybox
//!
//! Press Enter to pause or resume rotation.
//!
//! Reflection probes don't work on WebGL 2 or WebGPU.

use bevy::{
    camera::Exposure,
    core_pipeline::{tonemapping::Tonemapping, Skybox},
    pbr::generate::generate_environment_map_light,
    prelude::*,
    render::{render_resource::TextureUsages, view::Hdr},
};

use std::{
    f32::consts::PI,
    fmt::{Display, Formatter, Result as FmtResult},
};

static STOP_ROTATION_HELP_TEXT: &str = "Press Enter to stop rotation";
static START_ROTATION_HELP_TEXT: &str = "Press Enter to start rotation";

static REFLECTION_MODE_HELP_TEXT: &str = "Press Space to switch reflection mode";

const ENV_MAP_INTENSITY: f32 = 5000.0;

// The mode the application is in.
#[derive(Resource)]
struct AppStatus {
    // Which environment maps the user has requested to display.
    reflection_mode: ReflectionMode,
    // Whether the user has requested the scene to rotate.
    rotating: bool,
    // The current roughness of the central sphere
    sphere_roughness: f32,
}

// Which environment maps the user has requested to display.
#[derive(Clone, Copy, PartialEq)]
enum ReflectionMode {
    // Only a world environment map is shown.
    EnvironmentMap = 0,
    // Both a world environment map and a reflection probe are present. The
    // reflection probe is shown in the sphere.
    ReflectionProbe = 1,
    // A generated environment map is shown.
    GeneratedEnvironmentMap = 2,
}

// The various reflection maps.
#[derive(Resource)]
struct Cubemaps {
    // The blurry diffuse cubemap that reflects the world, but not the cubes.
    diffuse_environment_map: Handle<Image>,

    // The specular cubemap mip chain that reflects the world, but not the cubes.
    specular_environment_map: Handle<Image>,

    // The specular cubemap mip chain that reflects both the world and the cubes.
    specular_reflection_probe: Handle<Image>,
}

fn main() {
    // Create the app.
    App::new()
        .add_plugins(DefaultPlugins)
        .init_resource::<AppStatus>()
        .init_resource::<Cubemaps>()
        .add_systems(Startup, setup)
        .add_systems(PreUpdate, add_environment_map_to_camera)
        .add_systems(
            Update,
            change_reflection_type.before(generate_environment_map_light),
        )
        .add_systems(Update, toggle_rotation)
        .add_systems(Update, change_sphere_roughness)
        .add_systems(
            Update,
            rotate_camera
                .after(toggle_rotation)
                .after(change_reflection_type),
        )
        .add_systems(Update, update_text.after(rotate_camera))
        .add_systems(Update, setup_environment_map_usage)
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
    spawn_camera(&mut commands);
    spawn_sphere(&mut commands, &mut meshes, &mut materials, &app_status);
    spawn_reflection_probe(&mut commands, &cubemaps);
    spawn_scene(&mut commands, &asset_server);
    spawn_text(&mut commands, &app_status);
}

// Spawns the cubes, light, and camera.
fn spawn_scene(commands: &mut Commands, asset_server: &AssetServer) {
    commands.spawn((
        SceneRoot(asset_server.load(GltfAssetLabel::Scene(0).from_asset("models/cubes/Cubes.glb"))),
        CubesScene,
    ));
}

// Spawns the camera.
fn spawn_camera(commands: &mut Commands) {
    commands.spawn((
        Camera3d::default(),
        Hdr,
        Exposure { ev100: 11.0 },
        Tonemapping::AcesFitted,
        Transform::from_xyz(-3.883, 0.325, 2.781).looking_at(Vec3::ZERO, Vec3::Y),
    ));
}

// Creates the sphere mesh and spawns it.
fn spawn_sphere(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    app_status: &AppStatus,
) {
    // Create a sphere mesh.
    let sphere_mesh = meshes.add(Sphere::new(1.0).mesh().ico(7).unwrap());

    // Create a sphere.
    commands.spawn((
        Mesh3d(sphere_mesh.clone()),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Srgba::hex("#ffffff").unwrap().into(),
            metallic: 1.0,
            perceptual_roughness: app_status.sphere_roughness,
            ..StandardMaterial::default()
        })),
        SphereMaterial,
    ));
}

// Spawns the reflection probe.
fn spawn_reflection_probe(commands: &mut Commands, cubemaps: &Cubemaps) {
    commands.spawn((
        LightProbe,
        EnvironmentMapLight {
            diffuse_map: cubemaps.diffuse_environment_map.clone(),
            specular_map: cubemaps.specular_reflection_probe.clone(),
            intensity: ENV_MAP_INTENSITY,
            ..default()
        },
        // 2.0 because the sphere's radius is 1.0 and we want to fully enclose it.
        Transform::from_scale(Vec3::splat(2.0)),
    ));
}

// Spawns the help text.
fn spawn_text(commands: &mut Commands, app_status: &AppStatus) {
    // Create the text.
    commands.spawn((
        app_status.create_text(),
        Node {
            position_type: PositionType::Absolute,
            bottom: px(12),
            left: px(12),
            ..default()
        },
    ));
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
                image: cubemaps.specular_environment_map.clone(),
                brightness: ENV_MAP_INTENSITY,
                ..default()
            });
    }
}

// A system that handles switching between different reflection modes.
fn change_reflection_type(
    mut commands: Commands,
    light_probe_query: Query<Entity, With<LightProbe>>,
    cubes_scene_query: Query<Entity, With<CubesScene>>,
    camera_query: Query<Entity, With<Camera3d>>,
    keyboard: Res<ButtonInput<KeyCode>>,
    mut app_status: ResMut<AppStatus>,
    cubemaps: Res<Cubemaps>,
    asset_server: Res<AssetServer>,
) {
    // Only do anything if space was pressed.
    if !keyboard.just_pressed(KeyCode::Space) {
        return;
    }

    // Advance to the next reflection mode.
    app_status.reflection_mode =
        ReflectionMode::try_from((app_status.reflection_mode as u32 + 1) % 3).unwrap();

    // Remove light probes
    for light_probe in light_probe_query.iter() {
        commands.entity(light_probe).despawn();
    }
    // Remove existing cube scenes
    for scene_entity in cubes_scene_query.iter() {
        commands.entity(scene_entity).despawn();
    }
    match app_status.reflection_mode {
        ReflectionMode::EnvironmentMap | ReflectionMode::GeneratedEnvironmentMap => {}
        ReflectionMode::ReflectionProbe => {
            spawn_reflection_probe(&mut commands, &cubemaps);
            spawn_scene(&mut commands, &asset_server);
        }
    }

    // Update the environment-map components on the camera entity/entities
    for camera in camera_query.iter() {
        // Remove any existing environment-map components
        commands
            .entity(camera)
            .remove::<(EnvironmentMapLight, GeneratedEnvironmentMapLight)>();

        match app_status.reflection_mode {
            // A baked or reflection-probe environment map
            ReflectionMode::EnvironmentMap | ReflectionMode::ReflectionProbe => {
                commands
                    .entity(camera)
                    .insert(create_camera_environment_map_light(&cubemaps));
            }

            // GPU-filtered environment map generated at runtime
            ReflectionMode::GeneratedEnvironmentMap => {
                commands
                    .entity(camera)
                    .insert(GeneratedEnvironmentMapLight {
                        environment_map: cubemaps.specular_environment_map.clone(),
                        intensity: ENV_MAP_INTENSITY,
                        ..default()
                    });
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
            0 => Ok(ReflectionMode::EnvironmentMap),
            1 => Ok(ReflectionMode::ReflectionProbe),
            2 => Ok(ReflectionMode::GeneratedEnvironmentMap),
            _ => Err(()),
        }
    }
}

impl Display for ReflectionMode {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> FmtResult {
        let text = match *self {
            ReflectionMode::EnvironmentMap => "Environment map",
            ReflectionMode::ReflectionProbe => "Reflection probe",
            ReflectionMode::GeneratedEnvironmentMap => "Generated environment map",
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

        format!(
            "{}\n{}\nRoughness: {:.2}\n{}\nUp/Down arrows to change roughness",
            self.reflection_mode,
            rotation_help_text,
            self.sphere_roughness,
            REFLECTION_MODE_HELP_TEXT
        )
        .into()
    }
}

// Creates the world environment map light, used as a fallback if no reflection
// probe is applicable to a mesh.
fn create_camera_environment_map_light(cubemaps: &Cubemaps) -> EnvironmentMapLight {
    EnvironmentMapLight {
        diffuse_map: cubemaps.diffuse_environment_map.clone(),
        specular_map: cubemaps.specular_environment_map.clone(),
        intensity: ENV_MAP_INTENSITY,
        ..default()
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
        transform.translation = Vec2::from_angle(time.delta_secs() * PI / 5.0)
            .rotate(transform.translation.xz())
            .extend(transform.translation.y)
            .xzy();
        transform.look_at(Vec3::ZERO, Vec3::Y);
    }
}

// Loads the cubemaps from the assets directory.
impl FromWorld for Cubemaps {
    fn from_world(world: &mut World) -> Self {
        Cubemaps {
            diffuse_environment_map: world
                .load_asset("environment_maps/pisa_diffuse_rgb9e5_zstd.ktx2"),
            specular_environment_map: world
                .load_asset("environment_maps/pisa_specular_rgb9e5_zstd.ktx2"),
            specular_reflection_probe: world
                .load_asset("environment_maps/cubes_reflection_probe_specular_rgb9e5_zstd.ktx2"),
        }
    }
}

fn setup_environment_map_usage(cubemaps: Res<Cubemaps>, mut images: ResMut<Assets<Image>>) {
    if let Some(image) = images.get_mut(&cubemaps.specular_environment_map)
        && !image
            .texture_descriptor
            .usage
            .contains(TextureUsages::COPY_SRC)
    {
        image.texture_descriptor.usage |= TextureUsages::COPY_SRC;
    }
}

impl Default for AppStatus {
    fn default() -> Self {
        Self {
            reflection_mode: ReflectionMode::ReflectionProbe,
            rotating: false,
            sphere_roughness: 0.2,
        }
    }
}

#[derive(Component)]
struct SphereMaterial;

#[derive(Component)]
struct CubesScene;

// A system that changes the sphere's roughness with up/down arrow keys
fn change_sphere_roughness(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut app_status: ResMut<AppStatus>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    sphere_query: Query<&MeshMaterial3d<StandardMaterial>, With<SphereMaterial>>,
) {
    let roughness_delta = if keyboard.pressed(KeyCode::ArrowUp) {
        0.01 // Decrease roughness
    } else if keyboard.pressed(KeyCode::ArrowDown) {
        -0.01 // Increase roughness
    } else {
        0.0 // No change
    };

    if roughness_delta != 0.0 {
        // Update the app status
        app_status.sphere_roughness =
            (app_status.sphere_roughness + roughness_delta).clamp(0.0, 1.0);

        // Update the sphere material
        for material_handle in sphere_query.iter() {
            if let Some(material) = materials.get_mut(&material_handle.0) {
                material.perceptual_roughness = app_status.sphere_roughness;
            }
        }
    }
}
