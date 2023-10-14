//! This example shows how to place reflection probes in the scene.

use bevy::math::Vec3A;
use bevy::prelude::*;
use bevy_internal::core_pipeline::Skybox;

use std::fmt::{Display, Formatter, Result as FmtResult};

const ROTATION_SPEED: f32 = 0.005;

static HELP_TEXT: &str = "Press space to switch reflection mode";

#[derive(Clone, Copy, PartialEq, Resource, Default)]
enum ReflectionMode {
    None = 0,
    EnvironmentMap = 1,
    #[default]
    ReflectionProbe = 2,
}

#[derive(Clone, Copy, Resource, Default)]
struct CameraAngle(f32);

fn main() {
    App::new()
        .init_resource::<ReflectionMode>()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_systems(PreUpdate, add_environment_map_to_camera)
        .add_systems(Update, change_reflection_type)
        .add_systems(Update, rotate_camera)
        .run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
    reflection_mode: Res<ReflectionMode>,
) {
    // Spawn the cubes, light, and camera.
    commands.spawn(SceneBundle {
        scene: asset_server.load("models/cubes/Cubes.glb#Scene0"),
        ..SceneBundle::default()
    });

    // Create a sphere mesh.
    let sphere_mesh = meshes.add(
        Mesh::try_from(shape::Icosphere {
            radius: 1.0,
            subdivisions: 7,
        })
        .unwrap(),
    );

    // Create a sphere.
    commands.spawn(PbrBundle {
        mesh: sphere_mesh.clone(),
        material: materials.add(StandardMaterial {
            base_color: Color::hex("#ffd891").unwrap(),
            metallic: 1.0,
            perceptual_roughness: 0.0,
            ..StandardMaterial::default()
        }),
        transform: Transform::default(),
        ..PbrBundle::default()
    });

    // Create the reflection probe.
    create_reflection_probe(&mut commands, &asset_server);

    // Create the text.
    commands.spawn(
        TextBundle {
            text: create_text(*reflection_mode, &asset_server),
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

fn create_reflection_probe(commands: &mut Commands, asset_server: &AssetServer) {
    commands.spawn((
        SpatialBundle {
            transform: Transform::IDENTITY,
            ..SpatialBundle::default()
        },
        LightProbe {
            half_extents: Vec3A::splat(1.1),
        },
        EnvironmentMapLight {
            diffuse_map: asset_server.load("environment_maps/pisa_diffuse_rgb9e5_zstd.ktx2"),
            specular_map: asset_server
                .load("environment_maps/cubes_reflection_probe_specular.ktx2"),
        },
    ));
}

fn add_environment_map_to_camera(
    mut commands: Commands,
    query: Query<Entity, Added<Camera3d>>,
    asset_server: Res<AssetServer>,
) {
    for camera_entity in query.iter() {
        commands
            .entity(camera_entity)
            .insert(create_camera_environment_map_light(&asset_server))
            .insert(Skybox(asset_server.load("textures/pisa_cubemap.ktx2")));
    }
}

fn change_reflection_type(
    mut commands: Commands,
    light_probe_query: Query<Entity, With<LightProbe>>,
    camera_query: Query<Entity, With<Camera3d>>,
    mut text_query: Query<&mut Text>,
    keyboard: Res<Input<KeyCode>>,
    mut reflection_mode: ResMut<ReflectionMode>,
    asset_server: Res<AssetServer>,
) {
    // Only do anything if space was pressed.
    if !keyboard.just_pressed(KeyCode::Space) {
        return;
    }

    // Switch reflection mode.
    *reflection_mode = ReflectionMode::try_from((*reflection_mode as u32 + 1) % 3).unwrap();

    // Add or remove the light probe.
    for light_probe in light_probe_query.iter() {
        commands.entity(light_probe).despawn();
    }
    match *reflection_mode {
        ReflectionMode::None | ReflectionMode::EnvironmentMap => {}
        ReflectionMode::ReflectionProbe => create_reflection_probe(&mut commands, &asset_server),
    }

    // Add or remove the environment map from the camera.
    for camera in camera_query.iter() {
        match *reflection_mode {
            ReflectionMode::None => {
                commands.entity(camera).remove::<EnvironmentMapLight>();
            }
            ReflectionMode::EnvironmentMap | ReflectionMode::ReflectionProbe => {
                commands
                    .entity(camera)
                    .insert(create_camera_environment_map_light(&asset_server));
            }
        }
    }

    // Update text.
    for mut text in text_query.iter_mut() {
        *text = create_text(*reflection_mode, &asset_server);
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

fn create_text(reflection_mode: ReflectionMode, asset_server: &AssetServer) -> Text {
    Text::from_section(
        format!("{}\n{}", reflection_mode, HELP_TEXT),
        TextStyle {
            font: asset_server.load("fonts/FiraMono-Medium.ttf"),
            font_size: 24.0,
            color: Color::ANTIQUE_WHITE,
        },
    )
}

fn create_camera_environment_map_light(asset_server: &AssetServer) -> EnvironmentMapLight {
    EnvironmentMapLight {
        diffuse_map: asset_server.load("environment_maps/pisa_diffuse_rgb9e5_zstd.ktx2"),
        specular_map: asset_server.load("environment_maps/pisa_specular_rgb9e5_zstd.ktx2"),
    }
}

fn rotate_camera(mut camera_query: Query<&mut Transform, With<Camera3d>>) {
    for mut transform in camera_query.iter_mut() {
        transform.translation = Vec2::from_angle(ROTATION_SPEED)
            .rotate(transform.translation.xz())
            .extend(transform.translation.y)
            .xzy();
        transform.look_at(Vec3::ZERO, Vec3::Y);
    }
}
