//! Shows meshes that always face the camera using [`BillboardMaterial`].
//!
//! All the billboards share one quad mesh and one set of settings, so they are
//! drawn together even though their textures differ.
//!
//! A spot light shines on one side of the wall, which shows the difference
//! between lit and unlit billboards, and a floating slab casts a shadow across
//! some of them to show that they receive shadows.
//!
//! Use the buttons in the corner of the screen to change those settings.
//! Turn off the auto orbit to control the camera yourself:
//! - Orbit: Right mouse button + drag
//! - Pan: Left mouse button + drag
//! - Zoom: Mouse wheel

use crate::pan_orbit_camera_custom_input_plugin::CustomInputPlugin;
use bevy::{
    camera_controller::pan_orbit_camera::prelude::*,
    feathers::{theme::UiTheme, FeathersPlugins},
    image::{ImageLoaderSettings, ImageSampler},
    light::NotShadowReceiver,
    pbr::billboard::{BillboardExtension, BillboardMaterial},
    prelude::*,
    ui_widgets::{radio_self_update, ValueChange},
};

#[path = "../camera/pan_orbit_camera_custom_input_plugin.rs"]
mod pan_orbit_camera_custom_input_plugin;

#[path = "../helpers/radio.rs"]
mod radio;

#[path = "../helpers/theme.rs"]
mod theme;

/// Each example setting that can be changed in the UI.
#[derive(Clone, Copy)]
enum ExampleSetting {
    AutoOrbit(bool),
    LockY(bool),
    DepthTest(bool),
    AlphaBlend(bool),
    Unlit(bool),
    ReceiveShadows(bool),
}

impl Default for ExampleSetting {
    fn default() -> Self {
        Self::AutoOrbit(true)
    }
}

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins.set(WindowPlugin {
                primary_window: Some(Window {
                    title: "Bevy Billboard Example".into(),
                    ..default()
                }),
                ..default()
            }),
            FeathersPlugins,
            DefaultPanOrbitCameraPlugins,
            CustomInputPlugin,
        ))
        .insert_resource(UiTheme(theme::basic_example_theme(Color::WHITE)))
        .add_systems(Startup, setup)
        .add_systems(Update, orbit_camera)
        .add_observer(handle_setting_change)
        .add_observer(radio_self_update)
        .run();
}

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut standard_materials: ResMut<Assets<StandardMaterial>>,
    mut billboard_materials: ResMut<Assets<BillboardMaterial>>,
) {
    commands.spawn((
        Camera3d::default(),
        PanOrbitCamera {
            enabled_motion: EnabledMotion {
                pan: false,
                orbit: false,
                zoom: false,
            },
            ..default()
        },
        Transform::from_xyz(0.0, 5.0, 12.0).looking_at(Vec3::Y, Vec3::Y),
    ));
    commands.spawn((
        DirectionalLight {
            illuminance: 1_000.0,
            shadow_maps_enabled: true,
            ..default()
        },
        Transform::from_xyz(4.0, 8.0, 2.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
    // Spot light, aimed at the billboards on the +Z side of the wall
    commands.spawn((
        SpotLight {
            color: Color::srgb(1.0, 0.7, 0.4),
            intensity: 3_000_000.0,
            shadow_maps_enabled: true,
            inner_angle: 0.4,
            outer_angle: 0.5,
            ..default()
        },
        Transform::from_xyz(0.0, 7.0, 8.0).looking_at(Vec3::new(0.0, 0.0, 2.0), Vec3::Y),
    ));
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(20.0, 20.0))),
        MeshMaterial3d(standard_materials.add(Color::srgb(0.3, 0.5, 0.3))),
    ));
    // Wall. Stands in the middle of the billboards so that the depth
    // test has a visible effect
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(12.0, 3.0, 0.2))),
        MeshMaterial3d(standard_materials.add(Color::srgb(0.6, 0.3, 0.3))),
        Transform::from_xyz(0.0, 1.5, 0.0),
    ));

    // Blocks the spot light, so that the edge of its shadow runs through the middle billboard
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(3.0, 0.2, 2.0))),
        MeshMaterial3d(standard_materials.add(Color::srgb(0.3, 0.3, 0.6))),
        Transform::from_xyz(1.5, 4.0, 5.0),
    ));

    let quad = meshes.add(Rectangle::new(1.0, 1.0));

    let textures = [
        asset_server.load("branding/icon.png"),
        asset_server.load("branding/bevy_bird_dark.png"),
        asset_server
            .load_builder()
            .with_settings(|settings: &mut ImageLoaderSettings| {
                settings.sampler = ImageSampler::nearest();
            })
            .load("textures/rpg/chars/hat-guy/hat-guy.png"),
    ];
    let materials = textures.map(|texture| {
        billboard_materials.add(BillboardMaterial {
            base: StandardMaterial {
                base_color_texture: Some(texture),
                alpha_mode: AlphaMode::Mask(0.5),
                ..default()
            },
            extension: BillboardExtension::default(),
        })
    });

    const BILLBOARDS_PER_ROW: usize = 9;
    const BILLBOARD_SPACING: f32 = 1.5;

    let row_center = (BILLBOARDS_PER_ROW - 1) as f32 * BILLBOARD_SPACING / 2.0;
    for z in [-2.0, 2.0] {
        for x in 0..BILLBOARDS_PER_ROW {
            commands.spawn((
                Mesh3d(quad.clone()),
                MeshMaterial3d(materials[x % materials.len()].clone()),
                Transform::from_xyz(x as f32 * BILLBOARD_SPACING - row_center, 1.0, z),
            ));
        }
    }

    commands.spawn((
        Text::new(
            "Turn off Auto Orbit to control the camera:
Orbit: Right mouse button + drag
Pan: Left mouse button + drag
Zoom: Mouse wheel",
        ),
        Node {
            position_type: PositionType::Absolute,
            top: px(12),
            left: px(12),
            ..default()
        },
    ));

    commands.spawn_scene(bsn! {
        @radio::main_ui_node_scene()
        Children [
            @radio::feathers_option_buttons(
                "Auto Orbit",
                &[
                    (ExampleSetting::AutoOrbit(true), "On"),
                    (ExampleSetting::AutoOrbit(false), "Off"),
                ],
                0,
            )
            --
            @radio::feathers_option_buttons(
                "Lock Y",
                &[
                    (ExampleSetting::LockY(true), "On"),
                    (ExampleSetting::LockY(false), "Off"),
                ],
                1,
            )
            --
            @radio::feathers_option_buttons(
                "Depth Test",
                &[
                    (ExampleSetting::DepthTest(true), "On"),
                    (ExampleSetting::DepthTest(false), "Off"),
                ],
                0,
            )
            --
            @radio::feathers_option_buttons(
                "Alpha",
                &[
                    (ExampleSetting::AlphaBlend(false), "Mask"),
                    (ExampleSetting::AlphaBlend(true), "Blend"),
                ],
                0,
            )
            --
            @radio::feathers_option_buttons(
                "Unlit",
                &[
                    (ExampleSetting::Unlit(true), "On"),
                    (ExampleSetting::Unlit(false), "Off"),
                ],
                1,
            )
            --
            @radio::feathers_option_buttons(
                "Receive Shadows",
                &[
                    (ExampleSetting::ReceiveShadows(true), "On"),
                    (ExampleSetting::ReceiveShadows(false), "Off"),
                ],
                0,
            )
        ]
    });
}

fn handle_setting_change(
    event: On<ValueChange<Entity>>,
    settings: Query<&radio::RadioButtonOptionValue<ExampleSetting>>,
    mut materials: ResMut<Assets<BillboardMaterial>>,
    billboards: Query<Entity, With<MeshMaterial3d<BillboardMaterial>>>,
    mut camera: Single<&mut PanOrbitCamera>,
    mut commands: Commands,
) {
    let Ok(radio::RadioButtonOptionValue(setting)) = settings.get(event.value) else {
        return;
    };

    if let ExampleSetting::AutoOrbit(enabled) = *setting {
        camera.enabled_motion = if enabled {
            EnabledMotion {
                pan: false,
                orbit: false,
                zoom: false,
            }
        } else {
            EnabledMotion::default()
        };
        return;
    }

    if let ExampleSetting::ReceiveShadows(receive_shadows) = *setting {
        for billboard in &billboards {
            if receive_shadows {
                commands.entity(billboard).remove::<NotShadowReceiver>();
            } else {
                commands.entity(billboard).insert(NotShadowReceiver);
            }
        }
        return;
    }

    for (_, material) in materials.iter_mut() {
        match *setting {
            ExampleSetting::LockY(lock_y) => material.extension.lock_y = lock_y,
            ExampleSetting::DepthTest(depth_test) => material.extension.depth_test = depth_test,
            ExampleSetting::AlphaBlend(true) => material.base.alpha_mode = AlphaMode::Blend,
            ExampleSetting::AlphaBlend(false) => material.base.alpha_mode = AlphaMode::Mask(0.5),
            ExampleSetting::Unlit(unlit) => material.base.unlit = unlit,
            ExampleSetting::AutoOrbit(_) | ExampleSetting::ReceiveShadows(_) => {}
        }
    }
}

fn orbit_camera(time: Res<Time>, camera: Single<(&mut Transform, &PanOrbitCamera)>) {
    let (mut transform, controller) = camera.into_inner();
    if !controller.enabled_motion.orbit {
        let rotation = Quat::from_rotation_y(time.delta_secs() * 0.3);
        transform.rotate_around(Vec3::ZERO, rotation);
    }
}
