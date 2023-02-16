//! This examples compares Tonemapping options

use bevy::{
    core_pipeline::tonemapping::{Tonemapping, TonemappingMethod},
    math::vec2,
    pbr::CascadeShadowConfigBuilder,
    prelude::*,
    reflect::TypeUuid,
    render::{
        render_resource::{
            AsBindGroup, Extent3d, SamplerDescriptor, ShaderRef, TextureDimension, TextureFormat,
        },
        texture::ImageSampler,
        view::ColorGrading,
    },
    utils::HashMap,
};
use std::f32::consts::PI;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugin(MaterialPlugin::<ColorGradientMaterial>::default())
        .insert_resource(CameraTransform(
            Transform::from_xyz(0.7, 0.7, 1.0).looking_at(Vec3::new(0.0, 0.3, 0.0), Vec3::Y),
        ))
        .insert_resource(CurrentScene(1))
        .add_startup_system(setup)
        .add_startup_system(setup_basic_scene)
        .add_startup_system(setup_color_gradient_scene)
        .add_startup_system(setup_image_viewer_scene)
        .add_system(update_image_viewer)
        .add_system(toggle_scene)
        .add_system(toggle_tonemapping_method)
        .add_system(update_color_grading_settings)
        .add_system(update_ui)
        .run();
}

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    camera_transform: Res<CameraTransform>,
) {
    // camera
    commands.spawn((
        Camera3dBundle {
            camera: Camera {
                hdr: true,
                ..default()
            },
            transform: camera_transform.0,
            ..default()
        },
        EnvironmentMapLight {
            diffuse_map: asset_server.load("environment_maps/pisa_diffuse_rgb9e5_zstd.ktx2"),
            specular_map: asset_server.load("environment_maps/pisa_specular_rgb9e5_zstd.ktx2"),
        },
    ));

    // ui
    commands.spawn(
        TextBundle::from_section(
            "",
            TextStyle {
                font: asset_server.load("fonts/FiraMono-Medium.ttf"),
                font_size: 18.0,
                color: Color::WHITE,
            },
        )
        .with_style(Style {
            position_type: PositionType::Absolute,
            position: UiRect {
                top: Val::Px(10.0),
                left: Val::Px(10.0),
                ..default()
            },
            ..default()
        }),
    );
}

fn setup_basic_scene(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
    asset_server: Res<AssetServer>,
) {
    // plane
    commands.spawn((
        PbrBundle {
            mesh: meshes.add(Mesh::from(shape::Plane {
                size: 5.0,
                ..default()
            })),
            material: materials.add(StandardMaterial {
                base_color: Color::rgb(0.3, 0.5, 0.3),
                perceptual_roughness: 0.5,
                ..default()
            }),
            ..default()
        },
        SceneNumber(1),
    ));

    // cubes
    let cube_material = materials.add(StandardMaterial {
        base_color_texture: Some(images.add(uv_debug_texture())),
        ..default()
    });
    for i in 0..5 {
        commands.spawn((
            PbrBundle {
                mesh: meshes.add(Mesh::from(shape::Cube { size: 0.25 })),
                material: cube_material.clone(),
                transform: Transform::from_xyz(i as f32 * 0.25 - 1.0, 0.125, -i as f32 * 0.5),
                ..default()
            },
            SceneNumber(1),
        ));
    }

    // spheres
    for i in 0..6 {
        let j = i % 3;
        let s_val = if i < 3 { 0.0 } else { 0.2 };
        let material = if j == 0 {
            materials.add(StandardMaterial {
                base_color: Color::rgb(1.0, s_val, s_val),
                perceptual_roughness: 0.089,
                metallic: 0.0,
                ..default()
            })
        } else if j == 1 {
            materials.add(StandardMaterial {
                base_color: Color::rgb(s_val, 1.0, s_val),
                perceptual_roughness: 0.089,
                metallic: 0.0,
                ..default()
            })
        } else {
            materials.add(StandardMaterial {
                base_color: Color::rgb(s_val, s_val, 1.0),
                perceptual_roughness: 0.089,
                metallic: 0.0,
                ..default()
            })
        };
        commands.spawn((
            PbrBundle {
                mesh: meshes.add(Mesh::from(shape::UVSphere {
                    radius: 0.125,
                    sectors: 128,
                    stacks: 128,
                })),
                material,
                transform: Transform::from_xyz(
                    j as f32 * 0.25 + if i < 3 { -0.15 } else { 0.15 },
                    0.125,
                    -j as f32 * 0.25 + if i < 3 { -0.15 } else { 0.15 },
                ),
                ..default()
            },
            SceneNumber(1),
        ));
    }

    // Flight Helmet
    commands.spawn((
        SceneBundle {
            scene: asset_server.load("models/FlightHelmet/FlightHelmet.gltf#Scene0"),
            transform: Transform::from_xyz(-0.5, 0.0, 0.25),
            ..default()
        },
        SceneNumber(1),
    ));

    // light
    commands.spawn((
        DirectionalLightBundle {
            directional_light: DirectionalLight {
                shadows_enabled: true,
                illuminance: 50000.0,
                ..default()
            },
            transform: Transform::from_rotation(Quat::from_euler(
                EulerRot::ZYX,
                0.0,
                PI * -0.15,
                PI * -0.15,
            )),
            cascade_shadow_config: CascadeShadowConfigBuilder {
                maximum_distance: 3.0,
                first_cascade_far_bound: 0.9,
                ..default()
            }
            .into(),
            ..default()
        },
        SceneNumber(1),
    ));
}

fn setup_color_gradient_scene(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorGradientMaterial>>,
    camera_transform: Res<CameraTransform>,
) {
    let mut transform = camera_transform.0;
    transform.translation += transform.forward();

    commands.spawn((
        MaterialMeshBundle {
            mesh: meshes.add(Mesh::from(shape::Quad {
                size: vec2(1.0, 1.0) * 0.7,
                flip: false,
            })),
            material: materials.add(ColorGradientMaterial {}),
            transform,
            visibility: Visibility::Hidden,
            ..default()
        },
        SceneNumber(2),
    ));
}

fn setup_image_viewer_scene(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    camera_transform: Res<CameraTransform>,
    asset_server: Res<AssetServer>,
) {
    let mut transform = camera_transform.0;
    transform.translation += transform.forward();

    // exr/hdr viewer (exr requires enabling bevy feature)
    commands.spawn((
        PbrBundle {
            mesh: meshes.add(Mesh::from(shape::Quad {
                size: vec2(1.0, 1.0),
                flip: false,
            })),
            material: materials.add(StandardMaterial {
                base_color_texture: None,
                unlit: true,
                ..default()
            }),
            transform,
            visibility: Visibility::Hidden,
            ..default()
        },
        SceneNumber(3),
        HDRViewer,
    ));

    commands
        .spawn((
            TextBundle::from_section(
                "Drag and drop an HDR or EXR file",
                TextStyle {
                    font: asset_server.load("fonts/FiraMono-Medium.ttf"),
                    font_size: 28.0,
                    color: Color::BLACK,
                },
            )
            .with_text_alignment(TextAlignment::Center)
            .with_style(Style {
                align_self: AlignSelf::Center,
                margin: UiRect::all(Val::Auto),
                ..default()
            }),
            SceneNumber(3),
        ))
        .insert(Visibility::Hidden);
}

// ----------------------------------------------------------------------------

#[allow(clippy::too_many_arguments)]
fn update_image_viewer(
    image_mesh: Query<(&Handle<StandardMaterial>, &Handle<Mesh>), With<HDRViewer>>,
    text: Query<Entity, (With<Text>, With<SceneNumber>)>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>,
    images: Res<Assets<Image>>,
    mut drop_events: EventReader<FileDragAndDrop>,
    mut drop_hovered: Local<bool>,
    asset_server: Res<AssetServer>,
    mut image_events: EventReader<AssetEvent<Image>>,
    mut commands: Commands,
) {
    let mut new_image: Option<Handle<Image>> = None;

    for event in drop_events.iter() {
        match event {
            FileDragAndDrop::DroppedFile { path_buf, .. } => {
                new_image = Some(asset_server.load(path_buf.to_string_lossy().to_string()));
                *drop_hovered = false;
            }
            FileDragAndDrop::HoveredFile { .. } => *drop_hovered = true,
            FileDragAndDrop::HoveredFileCancelled { .. } => *drop_hovered = false,
        }
    }

    for (mat_h, mesh_h) in &image_mesh {
        if let Some(mat) = materials.get_mut(mat_h) {
            if let Some(ref new_image) = new_image {
                mat.base_color_texture = Some(new_image.clone());

                commands.entity(text.single()).despawn();
            }

            for event in image_events.iter() {
                let image_changed_h = match event {
                    AssetEvent::Created { handle } | AssetEvent::Modified { handle } => handle,
                    _ => continue,
                };
                if let Some(base_color_texture) = mat.base_color_texture.clone() {
                    if image_changed_h == &base_color_texture {
                        if let Some(image_changed) = images.get(image_changed_h) {
                            let size = image_changed.size().normalize_or_zero() * 1.4;
                            // Resize Mesh
                            let quad = Mesh::from(shape::Quad::new(size));
                            let _ = meshes.set(mesh_h, quad);
                        }
                    }
                }
            }
        }
    }
}

fn toggle_scene(
    keys: Res<Input<KeyCode>>,
    mut query: Query<(&mut Visibility, &SceneNumber)>,
    mut current_scene: ResMut<CurrentScene>,
) {
    let mut pressed = None;
    if keys.just_pressed(KeyCode::Key1) {
        pressed = Some(1);
    } else if keys.just_pressed(KeyCode::Key2) {
        pressed = Some(2);
    } else if keys.just_pressed(KeyCode::Key3) {
        pressed = Some(3);
    }

    if let Some(pressed) = pressed {
        current_scene.0 = pressed;

        for (mut visibility, scene) in query.iter_mut() {
            if scene.0 == pressed {
                *visibility = Visibility::Visible;
            } else {
                *visibility = Visibility::Hidden;
            }
        }
    }
}

fn toggle_tonemapping_method(keys: Res<Input<KeyCode>>, mut query: Query<&mut Tonemapping>) {
    let Tonemapping::Enabled { method, .. } = &mut *query.single_mut() else { unreachable!() };

    if keys.just_pressed(KeyCode::Q) {
        *method = TonemappingMethod::None;
    } else if keys.just_pressed(KeyCode::W) {
        *method = TonemappingMethod::Reinhard;
    } else if keys.just_pressed(KeyCode::E) {
        *method = TonemappingMethod::ReinhardLuminance;
    } else if keys.just_pressed(KeyCode::R) {
        *method = TonemappingMethod::Aces;
    } else if keys.just_pressed(KeyCode::T) {
        *method = TonemappingMethod::AgX;
    } else if keys.just_pressed(KeyCode::Y) {
        *method = TonemappingMethod::SomewhatBoringDisplayTransform;
    } else if keys.just_pressed(KeyCode::U) {
        *method = TonemappingMethod::TonyMcMapface;
    } else if keys.just_pressed(KeyCode::I) {
        *method = TonemappingMethod::BlenderFilmic;
    }
}

fn update_color_grading_settings(
    keys: Res<Input<KeyCode>>,
    time: Res<Time>,
    mut color_grading: Query<&mut ColorGrading>,
) {
    let mut color_grading = color_grading.single_mut();
    let dt = time.delta_seconds();

    if keys.pressed(KeyCode::S) {
        color_grading.exposure -= dt;
    }
    if keys.pressed(KeyCode::A) {
        color_grading.exposure += dt;
    }

    if keys.pressed(KeyCode::F) {
        color_grading.gamma -= dt;
    }
    if keys.pressed(KeyCode::D) {
        color_grading.gamma += dt;
    }

    if keys.pressed(KeyCode::X) {
        color_grading.pre_saturation -= dt;
    }
    if keys.pressed(KeyCode::Z) {
        color_grading.pre_saturation += dt;
    }

    if keys.pressed(KeyCode::V) {
        color_grading.post_saturation -= dt;
    }
    if keys.pressed(KeyCode::C) {
        color_grading.post_saturation += dt;
    }
}

fn update_ui(
    mut text: Query<&mut Text, Without<SceneNumber>>,
    settings: Query<(&Tonemapping, &ColorGrading)>,
    current_scene: Res<CurrentScene>,
) {
    let mut text = text.single_mut();
    let text = &mut text.sections[0].value;
    text.clear();
    let (&Tonemapping::Enabled { method, .. }, color_grading) = settings.single() else { unreachable!() };

    text.push_str("Scene: ");
    if current_scene.0 == 1 {
        text.push_str("(*1*)");
    } else {
        text.push_str("(1)");
    }
    if current_scene.0 == 2 {
        text.push_str("(*2*)");
    } else {
        text.push_str("(2)");
    }
    if current_scene.0 == 3 {
        text.push_str("(*3*)");
    } else {
        text.push_str("(3)");
    }

    text.push_str("\n\nTonemapping Method:\n");
    if method == TonemappingMethod::None {
        text.push_str("(Q) *Disabled*\n");
    } else {
        text.push_str("(Q) Disabled\n");
    }
    if method == TonemappingMethod::Reinhard {
        text.push_str("(W) *Reinhard*\n");
    } else {
        text.push_str("(W) Reinhard\n");
    }
    if method == TonemappingMethod::ReinhardLuminance {
        text.push_str("(E) *ReinhardLuminance*\n");
    } else {
        text.push_str("(E) ReinhardLuminance\n");
    }
    if method == TonemappingMethod::Aces {
        text.push_str("(R) *ACES*\n");
    } else {
        text.push_str("(R) ACES\n");
    }
    if method == TonemappingMethod::AgX {
        text.push_str("(T) *AgX*\n");
    } else {
        text.push_str("(T) AgX\n");
    }
    if method == TonemappingMethod::SomewhatBoringDisplayTransform {
        text.push_str("(Y) *SomewhatBoringDisplayTransform*\n");
    } else {
        text.push_str("(Y) SomewhatBoringDisplayTransform\n");
    }
    if method == TonemappingMethod::TonyMcMapface {
        text.push_str("(U) *TonyMcMapface*\n");
    } else {
        text.push_str("(U) TonyMcMapface\n");
    }
    if method == TonemappingMethod::BlenderFilmic {
        text.push_str("(I) *BlenderFilmic*");
    } else {
        text.push_str("(I) BlenderFilmic");
    }

    text.push_str("\n\nColor Grading:\n");
    text.push_str(&format!("(A/S) Exposure: {}\n", color_grading.exposure));
    text.push_str(&format!("(D/F) Gamma: {}\n", color_grading.gamma));
    text.push_str(&format!(
        "(Z/X) PreSaturation: {}\n",
        color_grading.pre_saturation
    ));
    text.push_str(&format!(
        "(C/V) PostSaturation: {}",
        color_grading.post_saturation
    ));
}

// ----------------------------------------------------------------------------

// pub struct PerMethodSettings {
//     pub settings: HashMap<TonemappingMethod, ColorGrading>,
// }

// impl Default for PerMethodSettings {
//     fn default() -> Self {
//         let mut settings = HashMap::new();

//         settings.insert(TonemappingMethod::None, ColorGrading::default());
//         settings.insert(TonemappingMethod::Reinhard, ColorGrading::default());
//         settings.insert(
//             TonemappingMethod::ReinhardLuminance,
//             ColorGrading::default(),
//         );
//         settings.insert(TonemappingMethod::Aces, ColorGrading::default());
//         settings.insert(TonemappingMethod::AgX, ColorGrading::default());
//         settings.insert(
//             TonemappingMethod::SomewhatBoringDisplayTransform,
//             ColorGrading::default(),
//         );
//         settings.insert(TonemappingMethod::TonyMcMapface, ColorGrading::default());
//         settings.insert(TonemappingMethod::BlenderFilmic, ColorGrading::default());

//         Self { settings }
//     }
// }

// impl PerMethodSettings {
//     fn matched() -> Self {
//         // Settings to somewhat match the tone mappers, especially in exposure, for this specific scene.
//         let mut settings = HashMap::new();

//         settings.insert(TonemappingMethod::None, ColorGrading::default());
//         settings.insert(
//             TonemappingMethod::Reinhard,
//             ColorGrading {
//                 exposure: 0.5,
//                 ..default()
//             },
//         );
//         settings.insert(
//             TonemappingMethod::ReinhardLuminance,
//             ColorGrading {
//                 exposure: 0.5,
//                 ..default()
//             },
//         );
//         settings.insert(
//             TonemappingMethod::Aces,
//             ColorGrading {
//                 exposure: -0.3,
//                 ..default()
//             },
//         );
//         settings.insert(
//             TonemappingMethod::AgX,
//             ColorGrading {
//                 exposure: -0.2,
//                 gamma: 1.0,
//                 pre_saturation: 1.1,
//                 post_saturation: 1.1,
//             },
//         );
//         settings.insert(
//             TonemappingMethod::SomewhatBoringDisplayTransform,
//             ColorGrading {
//                 exposure: 0.0,
//                 ..default()
//             },
//         );
//         settings.insert(
//             TonemappingMethod::TonyMcMapface,
//             ColorGrading {
//                 exposure: 0.0,
//                 ..default()
//             },
//         );
//         settings.insert(
//             TonemappingMethod::BlenderFilmic,
//             ColorGrading {
//                 exposure: 0.0,
//                 ..default()
//             },
//         );

//         Self { settings }
//     }
// }

/// Creates a colorful test pattern
fn uv_debug_texture() -> Image {
    const TEXTURE_SIZE: usize = 8;

    let mut palette: [u8; 32] = [
        255, 102, 159, 255, 255, 159, 102, 255, 236, 255, 102, 255, 121, 255, 102, 255, 102, 255,
        198, 255, 102, 198, 255, 255, 121, 102, 255, 255, 236, 102, 255, 255,
    ];

    let mut texture_data = [0; TEXTURE_SIZE * TEXTURE_SIZE * 4];
    for y in 0..TEXTURE_SIZE {
        let offset = TEXTURE_SIZE * y * 4;
        texture_data[offset..(offset + TEXTURE_SIZE * 4)].copy_from_slice(&palette);
        palette.rotate_right(4);
    }

    let mut img = Image::new_fill(
        Extent3d {
            width: TEXTURE_SIZE as u32,
            height: TEXTURE_SIZE as u32,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &texture_data,
        TextureFormat::Rgba8UnormSrgb,
    );
    img.sampler_descriptor = ImageSampler::Descriptor(SamplerDescriptor::default());
    img
}

impl Material for ColorGradientMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/tonemapping_test_patterns.wgsl".into()
    }
}

#[derive(AsBindGroup, Debug, Clone, TypeUuid)]
#[uuid = "117f64fe-6844-1822-8926-e3ed372291c8"]
pub struct ColorGradientMaterial {}

#[derive(Resource)]
struct CameraTransform(Transform);

#[derive(Resource)]
struct CurrentScene(u32);

#[derive(Component)]
struct SceneNumber(u32);

#[derive(Component)]
struct HDRViewer;
