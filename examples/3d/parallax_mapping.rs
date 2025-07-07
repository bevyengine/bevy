//! A simple 3D scene with a spinning cube with a normal map and depth map to demonstrate parallax mapping.
//! Press left mouse button to cycle through different views.

use std::fmt;

use bevy::{image::ImageLoaderSettings, math::ops, prelude::*};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                spin,
                move_camera,
                update_parallax_depth_scale,
                update_parallax_layers,
                switch_method,
            ),
        )
        .run();
}

#[derive(Component)]
struct Spin {
    speed: f32,
}

/// The camera, used to move camera on click.
#[derive(Component)]
struct CameraController;

const DEPTH_CHANGE_RATE: f32 = 0.1;
const DEPTH_UPDATE_STEP: f32 = 0.03;
const MAX_DEPTH: f32 = 0.3;

struct TargetDepth(f32);
impl Default for TargetDepth {
    fn default() -> Self {
        TargetDepth(0.09)
    }
}
struct TargetLayers(f32);
impl Default for TargetLayers {
    fn default() -> Self {
        TargetLayers(5.0)
    }
}
struct CurrentMethod(ParallaxMappingMethod);
impl Default for CurrentMethod {
    fn default() -> Self {
        CurrentMethod(ParallaxMappingMethod::Relief { max_steps: 4 })
    }
}

impl fmt::Display for CurrentMethod {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.0 {
            ParallaxMappingMethod::Occlusion => write!(f, "Parallax Occlusion Mapping"),
            ParallaxMappingMethod::Relief { max_steps } => {
                write!(f, "Relief Mapping with {max_steps} steps")
            }
        }
    }
}

impl CurrentMethod {
    fn next_method(&mut self) {
        use ParallaxMappingMethod::*;
        self.0 = match self.0 {
            Occlusion => Relief { max_steps: 2 },
            Relief { max_steps } if max_steps < 3 => Relief { max_steps: 4 },
            Relief { max_steps } if max_steps < 5 => Relief { max_steps: 8 },
            Relief { .. } => Occlusion,
        }
    }
}

fn update_parallax_depth_scale(
    input: Res<ButtonInput<KeyCode>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut target_depth: Local<TargetDepth>,
    mut depth_update: Local<bool>,
    mut writer: TextUiWriter,
    text: Single<Entity, With<Text>>,
) {
    if input.just_pressed(KeyCode::Digit1) {
        target_depth.0 -= DEPTH_UPDATE_STEP;
        target_depth.0 = target_depth.0.max(0.0);
        *depth_update = true;
    }
    if input.just_pressed(KeyCode::Digit2) {
        target_depth.0 += DEPTH_UPDATE_STEP;
        target_depth.0 = target_depth.0.min(MAX_DEPTH);
        *depth_update = true;
    }
    if *depth_update {
        for (_, mat) in materials.iter_mut() {
            let current_depth = mat.parallax_depth_scale;
            let new_depth = current_depth.lerp(target_depth.0, DEPTH_CHANGE_RATE);
            mat.parallax_depth_scale = new_depth;
            *writer.text(*text, 1) = format!("Parallax depth scale: {new_depth:.5}\n");
            if (new_depth - current_depth).abs() <= 0.000000001 {
                *depth_update = false;
            }
        }
    }
}

fn switch_method(
    input: Res<ButtonInput<KeyCode>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    text: Single<Entity, With<Text>>,
    mut writer: TextUiWriter,
    mut current: Local<CurrentMethod>,
) {
    if input.just_pressed(KeyCode::Space) {
        current.next_method();
    } else {
        return;
    }
    let text_entity = *text;
    *writer.text(text_entity, 3) = format!("Method: {}\n", *current);

    for (_, mat) in materials.iter_mut() {
        mat.parallax_mapping_method = current.0;
    }
}

fn update_parallax_layers(
    input: Res<ButtonInput<KeyCode>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut target_layers: Local<TargetLayers>,
    text: Single<Entity, With<Text>>,
    mut writer: TextUiWriter,
) {
    if input.just_pressed(KeyCode::Digit3) {
        target_layers.0 -= 1.0;
        target_layers.0 = target_layers.0.max(0.0);
    } else if input.just_pressed(KeyCode::Digit4) {
        target_layers.0 += 1.0;
    } else {
        return;
    }
    let layer_count = ops::exp2(target_layers.0);
    let text_entity = *text;
    *writer.text(text_entity, 2) = format!("Layers: {layer_count:.0}\n");

    for (_, mat) in materials.iter_mut() {
        mat.max_parallax_layer_count = layer_count;
    }
}

fn spin(time: Res<Time>, mut query: Query<(&mut Transform, &Spin)>) {
    for (mut transform, spin) in query.iter_mut() {
        transform.rotate_local_y(spin.speed * time.delta_secs());
        transform.rotate_local_x(spin.speed * time.delta_secs());
        transform.rotate_local_z(-spin.speed * time.delta_secs());
    }
}

// Camera positions to cycle through when left-clicking.
const CAMERA_POSITIONS: &[Transform] = &[
    Transform {
        translation: Vec3::new(1.5, 1.5, 1.5),
        rotation: Quat::from_xyzw(-0.279, 0.364, 0.115, 0.880),
        scale: Vec3::ONE,
    },
    Transform {
        translation: Vec3::new(2.4, 0.0, 0.2),
        rotation: Quat::from_xyzw(0.094, 0.676, 0.116, 0.721),
        scale: Vec3::ONE,
    },
    Transform {
        translation: Vec3::new(2.4, 2.6, -4.3),
        rotation: Quat::from_xyzw(0.170, 0.908, 0.308, 0.225),
        scale: Vec3::ONE,
    },
    Transform {
        translation: Vec3::new(-1.0, 0.8, -1.2),
        rotation: Quat::from_xyzw(-0.004, 0.909, 0.247, -0.335),
        scale: Vec3::ONE,
    },
];

fn move_camera(
    mut camera: Single<&mut Transform, With<CameraController>>,
    mut current_view: Local<usize>,
    button: Res<ButtonInput<MouseButton>>,
) {
    if button.just_pressed(MouseButton::Left) {
        *current_view = (*current_view + 1) % CAMERA_POSITIONS.len();
    }
    let target = CAMERA_POSITIONS[*current_view];
    camera.translation = camera.translation.lerp(target.translation, 0.2);
    camera.rotation = camera.rotation.slerp(target.rotation, 0.2);
}

fn setup(
    mut commands: Commands,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>,
    asset_server: Res<AssetServer>,
) {
    // The normal map. Note that to generate it in the GIMP image editor, you should
    // open the depth map, and do Filters → Generic → Normal Map
    // You should enable the "flip X" checkbox.
    let normal_handle = asset_server.load_with_settings(
        "textures/parallax_example/cube_normal.png",
        // The normal map texture is in linear color space. Lighting won't look correct
        // if `is_srgb` is `true`, which is the default.
        |settings: &mut ImageLoaderSettings| settings.is_srgb = false,
    );

    // Camera
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(1.5, 1.5, 1.5).looking_at(Vec3::ZERO, Vec3::Y),
        CameraController,
    ));

    // light
    commands
        .spawn((
            PointLight {
                shadows_enabled: true,
                ..default()
            },
            Transform::from_xyz(2.0, 1.0, -1.1),
        ))
        .with_children(|commands| {
            // represent the light source as a sphere
            let mesh = meshes.add(Sphere::new(0.05).mesh().ico(3).unwrap());
            commands.spawn((Mesh3d(mesh), MeshMaterial3d(materials.add(Color::WHITE))));
        });

    // Plane
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(10.0, 10.0))),
        MeshMaterial3d(materials.add(StandardMaterial {
            // standard material derived from dark green, but
            // with roughness and reflectance set.
            perceptual_roughness: 0.45,
            reflectance: 0.18,
            ..Color::srgb_u8(0, 80, 0).into()
        })),
        Transform::from_xyz(0.0, -1.0, 0.0),
    ));

    let parallax_depth_scale = TargetDepth::default().0;
    let max_parallax_layer_count = ops::exp2(TargetLayers::default().0);
    let parallax_mapping_method = CurrentMethod::default();
    let parallax_material = materials.add(StandardMaterial {
        perceptual_roughness: 0.4,
        base_color_texture: Some(asset_server.load("textures/parallax_example/cube_color.png")),
        normal_map_texture: Some(normal_handle),
        // The depth map is a grayscale texture where black is the highest level and
        // white the lowest.
        depth_map: Some(asset_server.load("textures/parallax_example/cube_depth.png")),
        parallax_depth_scale,
        parallax_mapping_method: parallax_mapping_method.0,
        max_parallax_layer_count,
        ..default()
    });
    commands.spawn((
        Mesh3d(
            meshes.add(
                // NOTE: for normal maps and depth maps to work, the mesh
                // needs tangents generated.
                Mesh::from(Cuboid::default())
                    .with_generated_tangents()
                    .unwrap(),
            ),
        ),
        MeshMaterial3d(parallax_material.clone()),
        Spin { speed: 0.3 },
    ));

    let background_cube = meshes.add(
        Mesh::from(Cuboid::new(40.0, 40.0, 40.0))
            .with_generated_tangents()
            .unwrap(),
    );

    let background_cube_bundle = |translation| {
        (
            Mesh3d(background_cube.clone()),
            MeshMaterial3d(parallax_material.clone()),
            Transform::from_translation(translation),
            Spin { speed: -0.1 },
        )
    };
    commands.spawn(background_cube_bundle(Vec3::new(45., 0., 0.)));
    commands.spawn(background_cube_bundle(Vec3::new(-45., 0., 0.)));
    commands.spawn(background_cube_bundle(Vec3::new(0., 0., 45.)));
    commands.spawn(background_cube_bundle(Vec3::new(0., 0., -45.)));

    // example instructions
    commands
        .spawn((
            Text::default(),
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(12.0),
                left: Val::Px(12.0),
                ..default()
            },
        ))
        .with_children(|p| {
            p.spawn(TextSpan(format!(
                "Parallax depth scale: {parallax_depth_scale:.5}\n"
            )));
            p.spawn(TextSpan(format!("Layers: {max_parallax_layer_count:.0}\n")));
            p.spawn(TextSpan(format!("{parallax_mapping_method}\n")));
            p.spawn(TextSpan::new("\n\n"));
            p.spawn(TextSpan::new("Controls:\n"));
            p.spawn(TextSpan::new("Left click - Change view angle\n"));
            p.spawn(TextSpan::new(
                "1/2 - Decrease/Increase parallax depth scale\n",
            ));
            p.spawn(TextSpan::new("3/4 - Decrease/Increase layer count\n"));
            p.spawn(TextSpan::new("Space - Switch parallaxing algorithm\n"));
        });
}
