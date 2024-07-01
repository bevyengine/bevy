//! Demonstrates the clearcoat PBR feature.
//!
//! Clearcoat is a separate material layer that represents a thin translucent
//! layer over a material. Examples include (from the Filament spec [1]) car paint,
//! soda cans, and lacquered wood.
//!
//! In glTF, clearcoat is supported via the `KHR_materials_clearcoat` [2]
//! extension. This extension is well supported by tools; in particular,
//! Blender's glTF exporter maps the clearcoat feature of its Principled BSDF
//! node to this extension, allowing it to appear in Bevy.
//!
//! This Bevy example is inspired by the corresponding three.js example [3].
//!
//! [1]: https://google.github.io/filament/Filament.html#materialsystem/clearcoatmodel
//!
//! [2]: https://github.com/KhronosGroup/glTF/blob/main/extensions/2.0/Khronos/KHR_materials_clearcoat/README.md
//!
//! [3]: https://threejs.org/examples/webgl_materials_physical_clearcoat.html

use std::f32::consts::PI;

use bevy::{
    color::palettes::css::{BLUE, GOLD, WHITE},
    core_pipeline::{tonemapping::Tonemapping::AcesFitted, Skybox},
    math::vec3,
    pbr::{CascadeShadowConfig, Cascades, CascadesVisibleEntities},
    prelude::*,
    render::{primitives::CascadesFrusta, texture::ImageLoaderSettings},
};

/// The size of each sphere.
const SPHERE_SCALE: f32 = 0.9;

/// The speed at which the spheres rotate, in radians per second.
const SPHERE_ROTATION_SPEED: f32 = 0.8;

/// Which type of light we're using: a point light or a directional light.
#[derive(Clone, Copy, PartialEq, Resource, Default)]
enum LightMode {
    #[default]
    Point,
    Directional,
}

/// Tags the example spheres.
#[derive(Component)]
struct ExampleSphere;

/// Entry point.
pub fn main() {
    App::new()
        .init_resource::<LightMode>()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_systems(Update, animate_light)
        .add_systems(Update, animate_spheres)
        .add_systems(Update, (handle_input, update_help_text).chain())
        .run();
}

/// Initializes the scene.
fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
    light_mode: Res<LightMode>,
) {
    let sphere = create_sphere_mesh(&mut meshes);
    spawn_car_paint_sphere(&mut commands, &mut materials, &asset_server, &sphere);
    spawn_coated_glass_bubble_sphere(&mut commands, &mut materials, &sphere);
    spawn_golf_ball(&mut commands, &asset_server);
    spawn_scratched_gold_ball(&mut commands, &mut materials, &asset_server, &sphere);

    spawn_light(&mut commands);
    spawn_camera(&mut commands, &asset_server);
    spawn_text(&mut commands, &light_mode);
}

/// Generates a sphere.
fn create_sphere_mesh(meshes: &mut Assets<Mesh>) -> Handle<Mesh> {
    // We're going to use normal maps, so make sure we've generated tangents, or
    // else the normal maps won't show up.

    let mut sphere_mesh = Sphere::new(1.0).mesh().build();
    sphere_mesh
        .generate_tangents()
        .expect("Failed to generate tangents");
    meshes.add(sphere_mesh)
}

/// Spawn a regular object with a clearcoat layer. This looks like car paint.
fn spawn_car_paint_sphere(
    commands: &mut Commands,
    materials: &mut Assets<StandardMaterial>,
    asset_server: &AssetServer,
    sphere: &Handle<Mesh>,
) {
    commands
        .spawn(PbrBundle {
            mesh: sphere.clone(),
            material: materials.add(StandardMaterial {
                clearcoat: 1.0,
                clearcoat_perceptual_roughness: 0.1,
                normal_map_texture: Some(asset_server.load_with_settings(
                    "textures/BlueNoise-Normal.png",
                    |settings: &mut ImageLoaderSettings| settings.is_srgb = false,
                )),
                metallic: 0.9,
                perceptual_roughness: 0.5,
                base_color: BLUE.into(),
                ..default()
            }),
            transform: Transform::from_xyz(-1.0, 1.0, 0.0).with_scale(Vec3::splat(SPHERE_SCALE)),
            ..default()
        })
        .insert(ExampleSphere);
}

/// Spawn a semitransparent object with a clearcoat layer.
fn spawn_coated_glass_bubble_sphere(
    commands: &mut Commands,
    materials: &mut Assets<StandardMaterial>,
    sphere: &Handle<Mesh>,
) {
    commands
        .spawn(PbrBundle {
            mesh: sphere.clone(),
            material: materials.add(StandardMaterial {
                clearcoat: 1.0,
                clearcoat_perceptual_roughness: 0.1,
                metallic: 0.5,
                perceptual_roughness: 0.1,
                base_color: Color::srgba(0.9, 0.9, 0.9, 0.3),
                alpha_mode: AlphaMode::Blend,
                ..default()
            }),
            transform: Transform::from_xyz(-1.0, -1.0, 0.0).with_scale(Vec3::splat(SPHERE_SCALE)),
            ..default()
        })
        .insert(ExampleSphere);
}

/// Spawns an object with both a clearcoat normal map (a scratched varnish) and
/// a main layer normal map (the golf ball pattern).
///
/// This object is in glTF format, using the `KHR_materials_clearcoat`
/// extension.
fn spawn_golf_ball(commands: &mut Commands, asset_server: &AssetServer) {
    commands
        .spawn(SceneBundle {
            scene: asset_server
                .load(GltfAssetLabel::Scene(0).from_asset("models/GolfBall/GolfBall.glb")),
            transform: Transform::from_xyz(1.0, 1.0, 0.0).with_scale(Vec3::splat(SPHERE_SCALE)),
            ..default()
        })
        .insert(ExampleSphere);
}

/// Spawns an object with only a clearcoat normal map (a scratch pattern) and no
/// main layer normal map.
fn spawn_scratched_gold_ball(
    commands: &mut Commands,
    materials: &mut Assets<StandardMaterial>,
    asset_server: &AssetServer,
    sphere: &Handle<Mesh>,
) {
    commands
        .spawn(PbrBundle {
            mesh: sphere.clone(),
            material: materials.add(StandardMaterial {
                clearcoat: 1.0,
                clearcoat_perceptual_roughness: 0.3,
                clearcoat_normal_texture: Some(asset_server.load_with_settings(
                    "textures/ScratchedGold-Normal.png",
                    |settings: &mut ImageLoaderSettings| settings.is_srgb = false,
                )),
                metallic: 0.9,
                perceptual_roughness: 0.1,
                base_color: GOLD.into(),
                ..default()
            }),
            transform: Transform::from_xyz(1.0, -1.0, 0.0).with_scale(Vec3::splat(SPHERE_SCALE)),
            ..default()
        })
        .insert(ExampleSphere);
}

/// Spawns a light.
fn spawn_light(commands: &mut Commands) {
    // Add the cascades objects used by the `DirectionalLightBundle`, since the
    // user can toggle between a point light and a directional light.
    commands
        .spawn(PointLightBundle {
            point_light: PointLight {
                color: WHITE.into(),
                intensity: 100000.0,
                ..default()
            },
            ..default()
        })
        .insert(CascadesFrusta::default())
        .insert(Cascades::default())
        .insert(CascadeShadowConfig::default())
        .insert(CascadesVisibleEntities::default());
}

/// Spawns a camera with associated skybox and environment map.
fn spawn_camera(commands: &mut Commands, asset_server: &AssetServer) {
    commands
        .spawn(Camera3dBundle {
            camera: Camera {
                hdr: true,
                ..default()
            },
            projection: Projection::Perspective(PerspectiveProjection {
                fov: 27.0 / 180.0 * PI,
                ..default()
            }),
            transform: Transform::from_xyz(0.0, 0.0, 10.0),
            tonemapping: AcesFitted,
            ..default()
        })
        .insert(Skybox {
            brightness: 5000.0,
            image: asset_server.load("environment_maps/pisa_specular_rgb9e5_zstd.ktx2"),
        })
        .insert(EnvironmentMapLight {
            diffuse_map: asset_server.load("environment_maps/pisa_diffuse_rgb9e5_zstd.ktx2"),
            specular_map: asset_server.load("environment_maps/pisa_specular_rgb9e5_zstd.ktx2"),
            intensity: 2000.0,
        });
}

/// Spawns the help text.
fn spawn_text(commands: &mut Commands, light_mode: &LightMode) {
    commands.spawn(
        TextBundle {
            text: light_mode.create_help_text(),
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

/// Moves the light around.
fn animate_light(
    mut lights: Query<&mut Transform, Or<(With<PointLight>, With<DirectionalLight>)>>,
    time: Res<Time>,
) {
    let now = time.elapsed_seconds();
    for mut transform in lights.iter_mut() {
        transform.translation = vec3(
            f32::sin(now * 1.4),
            f32::cos(now * 1.0),
            f32::cos(now * 0.6),
        ) * vec3(3.0, 4.0, 3.0);
        transform.look_at(Vec3::ZERO, Vec3::Y);
    }
}

/// Rotates the spheres.
fn animate_spheres(mut spheres: Query<&mut Transform, With<ExampleSphere>>, time: Res<Time>) {
    let now = time.elapsed_seconds();
    for mut transform in spheres.iter_mut() {
        transform.rotation = Quat::from_rotation_y(SPHERE_ROTATION_SPEED * now);
    }
}

/// Handles the user pressing Space to change the type of light from point to
/// directional and vice versa.
fn handle_input(
    mut commands: Commands,
    mut light_query: Query<Entity, Or<(With<PointLight>, With<DirectionalLight>)>>,
    keyboard: Res<ButtonInput<KeyCode>>,
    mut light_mode: ResMut<LightMode>,
) {
    if !keyboard.just_pressed(KeyCode::Space) {
        return;
    }

    for light in light_query.iter_mut() {
        match *light_mode {
            LightMode::Point => {
                *light_mode = LightMode::Directional;
                commands
                    .entity(light)
                    .remove::<PointLight>()
                    .insert(create_directional_light());
            }
            LightMode::Directional => {
                *light_mode = LightMode::Point;
                commands
                    .entity(light)
                    .remove::<DirectionalLight>()
                    .insert(create_point_light());
            }
        }
    }
}

/// Updates the help text at the bottom of the screen.
fn update_help_text(mut text_query: Query<&mut Text>, light_mode: Res<LightMode>) {
    for mut text in text_query.iter_mut() {
        *text = light_mode.create_help_text();
    }
}

/// Creates or recreates the moving point light.
fn create_point_light() -> PointLight {
    PointLight {
        color: WHITE.into(),
        intensity: 100000.0,
        ..default()
    }
}

/// Creates or recreates the moving directional light.
fn create_directional_light() -> DirectionalLight {
    DirectionalLight {
        color: WHITE.into(),
        illuminance: 1000.0,
        ..default()
    }
}

impl LightMode {
    /// Creates the help text at the bottom of the screen.
    fn create_help_text(&self) -> Text {
        let help_text = match *self {
            LightMode::Point => "Press Space to switch to a directional light",
            LightMode::Directional => "Press Space to switch to a point light",
        };

        Text::from_section(help_text, TextStyle::default())
    }
}
