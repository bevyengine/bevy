//! This examples compares Tonemapping

// cargo run --release --example tonemapping --features="ktx2 zstd exr debug_asset_server"

use std::f32::consts::PI;

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
    },
};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(AssetPlugin {
            // Tell the asset server to watch for asset changes on disk:
            watch_for_changes: true,
            ..default()
        }))
        .add_plugin(MaterialPlugin::<TestMaterial>::default())
        .insert_resource(CamTrans(
            Transform::from_xyz(0.7, 0.7, 1.0).looking_at(Vec3::new(0.0, 0.3, 0.0), Vec3::Y),
        ))
        .add_startup_system(setup_camera)
        .add_startup_system(scene1)
        .add_startup_system(scene2)
        .add_startup_system(scene3)
        .add_system(hdr_viewer)
        .add_system(toggle_scene)
        .add_system(toggle_tonemapping)
        .run();
}

#[derive(Component)]
struct Scene(u32);

#[derive(Component)]
struct HDRViewer;

#[derive(Resource)]
struct CamTrans(Transform);

fn setup_camera(mut commands: Commands, asset_server: Res<AssetServer>, cam_trans: Res<CamTrans>) {
    println!("Toggle with:");
    println!("1 - Helmet and stuff");
    println!("2 - Image viewer");
    println!("3 - Color Sweep");

    println!("");

    println!("4 - Bypass");
    println!("5 - Reinhard");
    println!("6 - Reinhard Luminance (old bevy default)");
    println!("7 - Aces");
    println!("8 - AgX");
    println!("9 - SBDT");
    println!("0 - Blender Filmic");

    // camera
    commands
        .spawn(Camera3dBundle {
            camera: Camera {
                hdr: true, // Works with and without hdr
                ..default()
            },
            transform: cam_trans.0,
            tonemapping: Tonemapping::Enabled {
                deband_dither: true,
                method: TonemappingMethod::AgX,
            },
            ..default()
        })
        .insert(EnvironmentMapLight {
            diffuse_map: asset_server.load("environment_maps/pisa_diffuse_rgb9e5_zstd.ktx2"),
            specular_map: asset_server.load("environment_maps/pisa_specular_rgb9e5_zstd.ktx2"),
        });
}

fn scene1(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
    asset_server: Res<AssetServer>,
) {
    // plane
    commands.spawn((
        PbrBundle {
            mesh: meshes.add(Mesh::from(shape::Plane { size: 5.0 })),
            material: materials.add(StandardMaterial {
                base_color: Color::rgb(0.3, 0.5, 0.3).into(),
                perceptual_roughness: 0.5,
                ..default()
            }),
            ..default()
        },
        Scene(1),
    ));

    let cube_material = materials.add(StandardMaterial {
        base_color_texture: Some(images.add(uv_debug_texture())),
        ..default()
    });

    // cubes
    for i in 0..5 {
        commands.spawn((
            PbrBundle {
                mesh: meshes.add(Mesh::from(shape::Cube { size: 0.25 })),
                material: cube_material.clone(),
                transform: Transform::from_xyz(i as f32 * 0.25 - 1.0, 0.125, -i as f32 * 0.5),
                ..default()
            },
            Scene(1),
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
            Scene(1),
        ));
    }

    // Flight Helmet
    commands.spawn((
        SceneBundle {
            scene: asset_server.load("models/FlightHelmet/FlightHelmet.gltf#Scene0"),
            transform: Transform::from_xyz(-0.5, 0.0, 0.25),
            ..default()
        },
        Scene(1),
    ));

    // light
    commands.spawn((
        DirectionalLightBundle {
            directional_light: DirectionalLight {
                shadows_enabled: true,
                //illuminance: 30000.0,
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
        Scene(1),
    ));
}

fn scene2(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    cam_trans: Res<CamTrans>,
) {
    let mut transform = cam_trans.0;
    transform.translation += transform.forward();

    // exr/hdr viewer
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
        Scene(2),
        HDRViewer,
    ));
}

fn scene3(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<TestMaterial>>,
    cam_trans: Res<CamTrans>,
) {
    let mut transform = cam_trans.0;
    transform.translation += transform.forward();
    // exr/hdr viewer
    commands.spawn((
        MaterialMeshBundle {
            mesh: meshes.add(Mesh::from(shape::Quad {
                size: vec2(1.0, 1.0) * 0.7,
                flip: false,
            })),
            material: materials.add(TestMaterial {}),
            transform,
            visibility: Visibility::Hidden,
            ..default()
        },
        Scene(3),
        HDRViewer,
    ));
}

fn hdr_viewer(
    query: Query<(&Handle<StandardMaterial>, &Handle<Mesh>), With<HDRViewer>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>,
    images: Res<Assets<Image>>,
    mut drop_events: EventReader<FileDragAndDrop>,
    mut drop_hovered: Local<bool>,
    asset_server: Res<AssetServer>,
    mut image_events: EventReader<AssetEvent<Image>>,
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

    for (mat_h, mesh_h) in &query {
        if let Some(mat) = materials.get_mut(mat_h) {
            if let Some(ref new_image) = new_image {
                // Update texture
                mat.base_color_texture = Some(new_image.clone());
            }

            for event in image_events.iter() {
                let image_changed_h = match event {
                    AssetEvent::Created { handle } => handle,
                    AssetEvent::Modified { handle } => handle,
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

fn toggle_scene(keys: Res<Input<KeyCode>>, mut query: Query<(&mut Visibility, &Scene)>) {
    let mut pressed = None;
    if keys.just_pressed(KeyCode::Key1) {
        pressed = Some(1);
    } else if keys.just_pressed(KeyCode::Key2) {
        pressed = Some(2);
    } else if keys.just_pressed(KeyCode::Key3) {
        pressed = Some(3);
    }
    if let Some(pressed) = pressed {
        for (mut vis, scene) in query.iter_mut() {
            if scene.0 == pressed {
                *vis = Visibility::Visible;
            } else {
                *vis = Visibility::Hidden;
            }
        }
    }
}

fn toggle_tonemapping(keys: Res<Input<KeyCode>>, mut query: Query<&mut Tonemapping>) {
    if let Some(mut tonemapping) = query.iter_mut().next() {
        if keys.just_pressed(KeyCode::Key4) {
            *tonemapping = Tonemapping::Enabled {
                deband_dither: true,
                method: TonemappingMethod::None,
            };
            println!("Bypass");
        } else if keys.just_pressed(KeyCode::Key5) {
            *tonemapping = Tonemapping::Enabled {
                deband_dither: true,
                method: TonemappingMethod::Reinhard,
            };
            println!("Reinhard");
        } else if keys.just_pressed(KeyCode::Key6) {
            *tonemapping = Tonemapping::Enabled {
                deband_dither: true,
                method: TonemappingMethod::ReinhardLuminance,
            };
            println!("ReinhardLuminance (old bevy default)");
        } else if keys.just_pressed(KeyCode::Key7) {
            *tonemapping = Tonemapping::Enabled {
                deband_dither: true,
                method: TonemappingMethod::Aces,
            };
            println!("Aces");
        } else if keys.just_pressed(KeyCode::Key8) {
            *tonemapping = Tonemapping::Enabled {
                deband_dither: true,
                method: TonemappingMethod::AgX,
            };
            println!("AgX");
        } else if keys.just_pressed(KeyCode::Key9) {
            *tonemapping = Tonemapping::Enabled {
                deband_dither: true,
                method: TonemappingMethod::SBDT,
            };
            println!("SBDT");
        } else if keys.just_pressed(KeyCode::Key0) {
            *tonemapping = Tonemapping::Enabled {
                deband_dither: true,
                method: TonemappingMethod::BlenderFilmic,
            };
            println!("Blender Filmic");
        }
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

impl Material for TestMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/tonemapping_test_patterns.wgsl".into()
    }
}

#[derive(AsBindGroup, Debug, Clone, TypeUuid)]
#[uuid = "117f64fe-6844-1822-8926-e3ed372291c8"]
pub struct TestMaterial {}
