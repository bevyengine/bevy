//! This examples compares Tonemapping options

use bevy::{
    core_pipeline::tonemapping::Tonemapping,
    math::vec2,
    pbr::CascadeShadowConfigBuilder,
    prelude::*,
    reflect::{TypePath, TypeUuid},
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
        .add_plugins((
            DefaultPlugins,
            MaterialPlugin::<ColorGradientMaterial>::default(),
        ))
        .insert_resource(CameraTransform(
            Transform::from_xyz(0.7, 0.7, 1.0).looking_at(Vec3::new(0.0, 0.3, 0.0), Vec3::Y),
        ))
        .init_resource::<PerMethodSettings>()
        .insert_resource(CurrentScene(1))
        .insert_resource(SelectedParameter { value: 0, max: 4 })
        .add_systems(
            Startup,
            (
                setup,
                setup_basic_scene,
                setup_color_gradient_scene,
                setup_image_viewer_scene,
            ),
        )
        .add_systems(
            Update,
            (
                update_image_viewer,
                toggle_scene,
                toggle_tonemapping_method,
                update_color_grading_settings,
                update_ui,
            ),
        )
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
                font_size: 18.0,
                color: Color::WHITE,
                ..default()
            },
        )
        .with_style(Style {
            position_type: PositionType::Absolute,
            top: Val::Px(10.0),
            left: Val::Px(10.0),
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

    let cube_mesh = meshes.add(Mesh::from(shape::Cube { size: 0.25 }));
    for i in 0..5 {
        commands.spawn((
            PbrBundle {
                mesh: cube_mesh.clone(),
                material: cube_material.clone(),
                transform: Transform::from_xyz(i as f32 * 0.25 - 1.0, 0.125, -i as f32 * 0.5),
                ..default()
            },
            SceneNumber(1),
        ));
    }

    // spheres
    let sphere_mesh = meshes.add(Mesh::from(shape::UVSphere {
        radius: 0.125,
        ..default()
    }));
    for i in 0..6 {
        let j = i % 3;
        let s_val = if i < 3 { 0.0 } else { 0.2 };
        let material = if j == 0 {
            materials.add(StandardMaterial {
                base_color: Color::rgb(s_val, s_val, 1.0),
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
                base_color: Color::rgb(1.0, s_val, s_val),
                perceptual_roughness: 0.089,
                metallic: 0.0,
                ..default()
            })
        };
        commands.spawn((
            PbrBundle {
                mesh: sphere_mesh.clone(),
                material,
                transform: Transform::from_xyz(
                    j as f32 * 0.25 + if i < 3 { -0.15 } else { 0.15 } - 0.4,
                    0.125,
                    -j as f32 * 0.25 + if i < 3 { -0.15 } else { 0.15 } + 0.4,
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
            transform: Transform::from_xyz(0.5, 0.0, -0.5)
                .with_rotation(Quat::from_rotation_y(-0.15 * PI)),
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
                    font_size: 36.0,
                    color: Color::BLACK,
                    ..default()
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

    for event in drop_events.read() {
        match event {
            FileDragAndDrop::DroppedFile { path_buf, .. } => {
                new_image = Some(asset_server.load(path_buf.to_string_lossy().to_string()));
                *drop_hovered = false;
            }
            FileDragAndDrop::HoveredFile { .. } => *drop_hovered = true,
            FileDragAndDrop::HoveredFileCanceled { .. } => *drop_hovered = false,
        }
    }

    for (mat_h, mesh_h) in &image_mesh {
        if let Some(mat) = materials.get_mut(mat_h) {
            if let Some(ref new_image) = new_image {
                mat.base_color_texture = Some(new_image.clone());

                if let Ok(text_entity) = text.get_single() {
                    commands.entity(text_entity).despawn();
                }
            }

            for event in image_events.read() {
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
    if keys.just_pressed(KeyCode::Q) {
        pressed = Some(1);
    } else if keys.just_pressed(KeyCode::W) {
        pressed = Some(2);
    } else if keys.just_pressed(KeyCode::E) {
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

fn toggle_tonemapping_method(
    keys: Res<Input<KeyCode>>,
    mut tonemapping: Query<&mut Tonemapping>,
    mut color_grading: Query<&mut ColorGrading>,
    per_method_settings: Res<PerMethodSettings>,
) {
    let mut method = tonemapping.single_mut();
    let mut color_grading = color_grading.single_mut();

    if keys.just_pressed(KeyCode::Key1) {
        *method = Tonemapping::None;
    } else if keys.just_pressed(KeyCode::Key2) {
        *method = Tonemapping::Reinhard;
    } else if keys.just_pressed(KeyCode::Key3) {
        *method = Tonemapping::ReinhardLuminance;
    } else if keys.just_pressed(KeyCode::Key4) {
        *method = Tonemapping::AcesFitted;
    } else if keys.just_pressed(KeyCode::Key5) {
        *method = Tonemapping::AgX;
    } else if keys.just_pressed(KeyCode::Key6) {
        *method = Tonemapping::SomewhatBoringDisplayTransform;
    } else if keys.just_pressed(KeyCode::Key7) {
        *method = Tonemapping::TonyMcMapface;
    } else if keys.just_pressed(KeyCode::Key8) {
        *method = Tonemapping::BlenderFilmic;
    }

    *color_grading = *per_method_settings
        .settings
        .get::<Tonemapping>(&method)
        .unwrap();
}

#[derive(Resource)]
struct SelectedParameter {
    value: i32,
    max: i32,
}

impl SelectedParameter {
    fn next(&mut self) {
        self.value = (self.value + 1).rem_euclid(self.max);
    }
    fn prev(&mut self) {
        self.value = (self.value - 1).rem_euclid(self.max);
    }
}

fn update_color_grading_settings(
    keys: Res<Input<KeyCode>>,
    time: Res<Time>,
    mut per_method_settings: ResMut<PerMethodSettings>,
    tonemapping: Query<&Tonemapping>,
    current_scene: Res<CurrentScene>,
    mut selected_parameter: ResMut<SelectedParameter>,
) {
    let method = tonemapping.single();
    let color_grading = per_method_settings.settings.get_mut(method).unwrap();
    let mut dt = time.delta_seconds() * 0.25;
    if keys.pressed(KeyCode::Left) {
        dt = -dt;
    }

    if keys.just_pressed(KeyCode::Down) {
        selected_parameter.next();
    }
    if keys.just_pressed(KeyCode::Up) {
        selected_parameter.prev();
    }
    if keys.pressed(KeyCode::Left) || keys.pressed(KeyCode::Right) {
        match selected_parameter.value {
            0 => {
                color_grading.exposure += dt;
            }
            1 => {
                color_grading.gamma += dt;
            }
            2 => {
                color_grading.pre_saturation += dt;
            }
            3 => {
                color_grading.post_saturation += dt;
            }
            _ => {}
        }
    }

    if keys.just_pressed(KeyCode::Space) {
        for (_, grading) in per_method_settings.settings.iter_mut() {
            *grading = ColorGrading::default();
        }
    }

    if keys.just_pressed(KeyCode::Return) && current_scene.0 == 1 {
        for (mapper, grading) in per_method_settings.settings.iter_mut() {
            *grading = PerMethodSettings::basic_scene_recommendation(*mapper);
        }
    }
}

fn update_ui(
    mut text: Query<&mut Text, Without<SceneNumber>>,
    settings: Query<(&Tonemapping, &ColorGrading)>,
    current_scene: Res<CurrentScene>,
    selected_parameter: Res<SelectedParameter>,
    mut hide_ui: Local<bool>,
    keys: Res<Input<KeyCode>>,
) {
    let (method, color_grading) = settings.single();
    let method = *method;

    let mut text = text.single_mut();
    let text = &mut text.sections[0].value;

    if keys.just_pressed(KeyCode::H) {
        *hide_ui = !*hide_ui;
    }
    text.clear();
    if *hide_ui {
        return;
    }

    let scn = current_scene.0;
    text.push_str("(H) Hide UI\n\n");
    text.push_str("Test Scene: \n");
    text.push_str(&format!(
        "(Q) {} Basic Scene\n",
        if scn == 1 { ">" } else { "" }
    ));
    text.push_str(&format!(
        "(W) {} Color Sweep\n",
        if scn == 2 { ">" } else { "" }
    ));
    text.push_str(&format!(
        "(E) {} Image Viewer\n",
        if scn == 3 { ">" } else { "" }
    ));

    text.push_str("\n\nTonemapping Method:\n");
    text.push_str(&format!(
        "(1) {} Disabled\n",
        if method == Tonemapping::None { ">" } else { "" }
    ));
    text.push_str(&format!(
        "(2) {} Reinhard\n",
        if method == Tonemapping::Reinhard {
            "> "
        } else {
            ""
        }
    ));
    text.push_str(&format!(
        "(3) {} Reinhard Luminance\n",
        if method == Tonemapping::ReinhardLuminance {
            ">"
        } else {
            ""
        }
    ));
    text.push_str(&format!(
        "(4) {} ACES Fitted\n",
        if method == Tonemapping::AcesFitted {
            ">"
        } else {
            ""
        }
    ));
    text.push_str(&format!(
        "(5) {} AgX\n",
        if method == Tonemapping::AgX { ">" } else { "" }
    ));
    text.push_str(&format!(
        "(6) {} SomewhatBoringDisplayTransform\n",
        if method == Tonemapping::SomewhatBoringDisplayTransform {
            ">"
        } else {
            ""
        }
    ));
    text.push_str(&format!(
        "(7) {} TonyMcMapface\n",
        if method == Tonemapping::TonyMcMapface {
            ">"
        } else {
            ""
        }
    ));
    text.push_str(&format!(
        "(8) {} Blender Filmic\n",
        if method == Tonemapping::BlenderFilmic {
            ">"
        } else {
            ""
        }
    ));

    text.push_str("\n\nColor Grading:\n");
    text.push_str("(arrow keys)\n");
    if selected_parameter.value == 0 {
        text.push_str("> ");
    }
    text.push_str(&format!("Exposure: {}\n", color_grading.exposure));
    if selected_parameter.value == 1 {
        text.push_str("> ");
    }
    text.push_str(&format!("Gamma: {}\n", color_grading.gamma));
    if selected_parameter.value == 2 {
        text.push_str("> ");
    }
    text.push_str(&format!(
        "PreSaturation: {}\n",
        color_grading.pre_saturation
    ));
    if selected_parameter.value == 3 {
        text.push_str("> ");
    }
    text.push_str(&format!(
        "PostSaturation: {}\n",
        color_grading.post_saturation
    ));
    text.push_str("(Space) Reset all to default\n");

    if current_scene.0 == 1 {
        text.push_str("(Enter) Reset all to scene recommendation\n");
    }
}

// ----------------------------------------------------------------------------

#[derive(Resource)]
struct PerMethodSettings {
    settings: HashMap<Tonemapping, ColorGrading>,
}

impl PerMethodSettings {
    fn basic_scene_recommendation(method: Tonemapping) -> ColorGrading {
        match method {
            Tonemapping::Reinhard | Tonemapping::ReinhardLuminance => ColorGrading {
                exposure: 0.5,
                ..default()
            },
            Tonemapping::AcesFitted => ColorGrading {
                exposure: 0.35,
                ..default()
            },
            Tonemapping::AgX => ColorGrading {
                exposure: -0.2,
                gamma: 1.0,
                pre_saturation: 1.1,
                post_saturation: 1.1,
            },
            _ => ColorGrading::default(),
        }
    }
}

impl Default for PerMethodSettings {
    fn default() -> Self {
        let mut settings = HashMap::new();

        for method in [
            Tonemapping::None,
            Tonemapping::Reinhard,
            Tonemapping::ReinhardLuminance,
            Tonemapping::AcesFitted,
            Tonemapping::AgX,
            Tonemapping::SomewhatBoringDisplayTransform,
            Tonemapping::TonyMcMapface,
            Tonemapping::BlenderFilmic,
        ] {
            settings.insert(
                method,
                PerMethodSettings::basic_scene_recommendation(method),
            );
        }

        Self { settings }
    }
}

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

#[derive(AsBindGroup, Debug, Clone, TypeUuid, TypePath)]
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
