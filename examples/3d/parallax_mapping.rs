//! A simple 3D scene with a spinning cube with a normal map and depth map to demonstrate parallax mapping.
//! Press left mouse button to cycle through different views.

use std::fmt;

use bevy::{prelude::*, render::render_resource::TextureFormat, window::close_on_esc};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .insert_resource(Normal(None))
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                spin,
                update_normal,
                move_camera,
                update_parallax_depth_scale,
                update_parallax_layers,
                switch_method,
                close_on_esc,
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
    input: Res<Input<KeyCode>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut target_depth: Local<TargetDepth>,
    mut depth_update: Local<bool>,
    mut text: Query<&mut Text>,
) {
    if input.just_pressed(KeyCode::Key1) {
        target_depth.0 -= DEPTH_UPDATE_STEP;
        target_depth.0 = target_depth.0.max(0.0);
        *depth_update = true;
    }
    if input.just_pressed(KeyCode::Key2) {
        target_depth.0 += DEPTH_UPDATE_STEP;
        target_depth.0 = target_depth.0.min(MAX_DEPTH);
        *depth_update = true;
    }
    if *depth_update {
        let mut text = text.single_mut();
        for (_, mat) in materials.iter_mut() {
            let current_depth = mat.parallax_depth_scale;
            let new_depth =
                current_depth * (1.0 - DEPTH_CHANGE_RATE) + (target_depth.0 * DEPTH_CHANGE_RATE);
            mat.parallax_depth_scale = new_depth;
            text.sections[0].value = format!("Parallax depth scale: {new_depth:.5}\n");
            if (new_depth - current_depth).abs() <= 0.000000001 {
                *depth_update = false;
            }
        }
    }
}

fn switch_method(
    input: Res<Input<KeyCode>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut text: Query<&mut Text>,
    mut current: Local<CurrentMethod>,
) {
    if input.just_pressed(KeyCode::Space) {
        current.next_method();
    } else {
        return;
    }
    let mut text = text.single_mut();
    text.sections[2].value = format!("Method: {}\n", *current);

    for (_, mat) in materials.iter_mut() {
        mat.parallax_mapping_method = current.0;
    }
}

fn update_parallax_layers(
    input: Res<Input<KeyCode>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut target_layers: Local<TargetLayers>,
    mut text: Query<&mut Text>,
) {
    if input.just_pressed(KeyCode::Key3) {
        target_layers.0 -= 1.0;
        target_layers.0 = target_layers.0.max(0.0);
    } else if input.just_pressed(KeyCode::Key4) {
        target_layers.0 += 1.0;
    } else {
        return;
    }
    let layer_count = target_layers.0.exp2();
    let mut text = text.single_mut();
    text.sections[1].value = format!("Layers: {layer_count:.0}\n");

    for (_, mat) in materials.iter_mut() {
        mat.max_parallax_layer_count = layer_count;
    }
}

fn spin(time: Res<Time>, mut query: Query<(&mut Transform, &Spin)>) {
    for (mut transform, spin) in query.iter_mut() {
        transform.rotate_local_y(spin.speed * time.delta_seconds());
        transform.rotate_local_x(spin.speed * time.delta_seconds());
        transform.rotate_local_z(-spin.speed * time.delta_seconds());
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
    mut camera: Query<&mut Transform, With<CameraController>>,
    mut current_view: Local<usize>,
    button: Res<Input<MouseButton>>,
) {
    let mut camera = camera.single_mut();
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
    mut normal: ResMut<Normal>,
    asset_server: Res<AssetServer>,
) {
    // The normal map. Note that to generate it in the GIMP image editor, you should
    // open the depth map, and do Filters → Generic → Normal Map
    // You should enable the "flip X" checkbox.
    let normal_handle = asset_server.load("textures/parallax_example/cube_normal.png");
    normal.0 = Some(normal_handle);

    // Camera
    commands.spawn((
        Camera3dBundle {
            transform: Transform::from_xyz(1.5, 1.5, 1.5).looking_at(Vec3::ZERO, Vec3::Y),
            ..default()
        },
        CameraController,
    ));

    // light
    commands
        .spawn(PointLightBundle {
            transform: Transform::from_xyz(1.8, 0.7, -1.1),
            point_light: PointLight {
                intensity: 226.0,
                shadows_enabled: true,
                ..default()
            },
            ..default()
        })
        .with_children(|commands| {
            // represent the light source as a sphere
            let mesh = meshes.add(
                shape::Icosphere {
                    radius: 0.05,
                    subdivisions: 3,
                }
                .try_into()
                .unwrap(),
            );
            commands.spawn(PbrBundle { mesh, ..default() });
        });

    // Plane
    commands.spawn(PbrBundle {
        mesh: meshes.add(
            shape::Plane {
                size: 10.0,
                subdivisions: 0,
            }
            .into(),
        ),
        material: materials.add(StandardMaterial {
            // standard material derived from dark green, but
            // with roughness and reflectance set.
            perceptual_roughness: 0.45,
            reflectance: 0.18,
            ..Color::rgb_u8(0, 80, 0).into()
        }),
        transform: Transform::from_xyz(0.0, -1.0, 0.0),
        ..default()
    });

    let mut cube: Mesh = shape::Cube { size: 1.0 }.into();

    // NOTE: for normal maps and depth maps to work, the mesh
    // needs tangents generated.
    cube.generate_tangents().unwrap();

    let parallax_depth_scale = TargetDepth::default().0;
    let max_parallax_layer_count = TargetLayers::default().0.exp2();
    let parallax_mapping_method = CurrentMethod::default();
    let parallax_material = materials.add(StandardMaterial {
        perceptual_roughness: 0.4,
        base_color_texture: Some(asset_server.load("textures/parallax_example/cube_color.png")),
        normal_map_texture: normal.0.clone(),
        // The depth map is a greyscale texture where black is the highest level and
        // white the lowest.
        depth_map: Some(asset_server.load("textures/parallax_example/cube_depth.png")),
        parallax_depth_scale,
        parallax_mapping_method: parallax_mapping_method.0,
        max_parallax_layer_count,
        ..default()
    });
    commands.spawn((
        PbrBundle {
            mesh: meshes.add(cube),
            material: parallax_material.clone_weak(),
            ..default()
        },
        Spin { speed: 0.3 },
    ));

    let mut background_cube: Mesh = shape::Cube { size: 40.0 }.into();
    background_cube.generate_tangents().unwrap();
    let background_cube = meshes.add(background_cube);

    let background_cube_bundle = |translation| {
        (
            PbrBundle {
                transform: Transform::from_translation(translation),
                mesh: background_cube.clone(),
                material: parallax_material.clone(),
                ..default()
            },
            Spin { speed: -0.1 },
        )
    };
    commands.spawn(background_cube_bundle(Vec3::new(45., 0., 0.)));
    commands.spawn(background_cube_bundle(Vec3::new(-45., 0., 0.)));
    commands.spawn(background_cube_bundle(Vec3::new(0., 0., 45.)));
    commands.spawn(background_cube_bundle(Vec3::new(0., 0., -45.)));

    let style = TextStyle {
        font_size: 20.0,
        ..default()
    };

    // example instructions
    commands.spawn(
        TextBundle::from_sections(vec![
            TextSection::new(
                format!("Parallax depth scale: {parallax_depth_scale:.5}\n"),
                style.clone(),
            ),
            TextSection::new(
                format!("Layers: {max_parallax_layer_count:.0}\n"),
                style.clone(),
            ),
            TextSection::new(format!("{parallax_mapping_method}\n"), style.clone()),
            TextSection::new("\n\n", style.clone()),
            TextSection::new("Controls:\n", style.clone()),
            TextSection::new("Left click - Change view angle\n", style.clone()),
            TextSection::new(
                "1/2 - Decrease/Increase parallax depth scale\n",
                style.clone(),
            ),
            TextSection::new("3/4 - Decrease/Increase layer count\n", style.clone()),
            TextSection::new("Space - Switch parallaxing algorithm\n", style),
        ])
        .with_style(Style {
            position_type: PositionType::Absolute,
            top: Val::Px(12.0),
            left: Val::Px(12.0),
            ..default()
        }),
    );
}

/// Store handle of the normal to later modify its format in [`update_normal`].
#[derive(Resource)]
struct Normal(Option<Handle<Image>>);

/// Work around the default bevy image loader.
///
/// The bevy image loader used by `AssetServer` always loads images in
/// `Srgb` mode, which is usually what it should do,
/// but is incompatible with normal maps.
///
/// Normal maps require a texture in linear color space,
/// so we overwrite the format of the normal map we loaded through `AssetServer`
/// in this system.
///
/// Note that this method of conversion is a last resort workaround. You should
/// get your normal maps from a 3d model file, like gltf.
///
/// In this system, we wait until the image is loaded, immediately
/// change its format and never run the logic afterward.
fn update_normal(
    mut already_ran: Local<bool>,
    mut images: ResMut<Assets<Image>>,
    normal: Res<Normal>,
) {
    if *already_ran {
        return;
    }
    if let Some(normal) = normal.0.as_ref() {
        if let Some(image) = images.get_mut(normal) {
            image.texture_descriptor.format = TextureFormat::Rgba8Unorm;
            *already_ran = true;
        }
    }
}
