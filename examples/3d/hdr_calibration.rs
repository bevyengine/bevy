//! This example allows for calibration of HDR display output.
//!
//! It shows a luminance gradient on the top half and a color gradient on the bottom half.
//! The red line in the middle of the top half indicates the "paper white" level (1.0).
//!
//! The bottom half shows a hue gradient at "paper white" intensity.
//! The very bottom strip of the color gradient is at "max luminance" intensity,
//! which can be used to check for highlight clipping or hue shifts in bright colors.
//!
//! Calibration:
//! - Up/Down: Adjust Paper White
//! - Left/Right: Adjust Max Luminance

use bevy::{
    core_pipeline::{
        core_3d::graph::Node3d,
        fullscreen_material::{FullscreenMaterial, FullscreenMaterialPlugin},
        tonemapping::Tonemapping,
    },
    prelude::*,
    render::{
        extract_component::ExtractComponent,
        render_graph::{InternedRenderLabel, RenderLabel},
        render_resource::ShaderType,
        view::{ColorGrading, Hdr},
    },
    shader::ShaderRef,
    window::PrimaryWindow,
};

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins.set(WindowPlugin {
                primary_window: Some(Window {
                    hdr_output: true,
                    ..default()
                }),
                ..default()
            }),
            FullscreenMaterialPlugin::<HdrCalibrationEffect>::default(),
        ))
        .init_resource::<CalibrationStatus>()
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                update_calibration,
                update_text,
                toggle_mode,
                update_scene.run_if(in_3d_mode),
            ),
        )
        .run();
}

#[derive(Resource, Default)]
struct CalibrationStatus {
    mode: SceneMode,
}

#[derive(Default, PartialEq)]
enum SceneMode {
    #[default]
    Calibration2d,
    Calibration3d,
}

fn in_3d_mode(status: Res<CalibrationStatus>) -> bool {
    status.mode == SceneMode::Calibration3d
}

#[derive(Component)]
struct CalibrationText;

#[derive(Component)]
struct Calibration2dMarker;

#[derive(Component)]
struct Calibration3dMarker;

#[derive(Component)]
struct CalibrationSphere {
    hue: f32,
    saturation: f32,
    brightness: f32,
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Camera
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 0.0, 10.0).looking_at(Vec3::ZERO, Vec3::Y),
        Hdr,
        Tonemapping::None,
        HdrCalibrationEffect::default(),
        Calibration2dMarker,
        Camera {
            clear_color: ClearColorConfig::Custom(Color::srgb(0.5, 0.5, 0.5)),
            ..default()
        },
    ));

    // 3D Scene Elements (initially hidden)

    // Grid of spheres
    // Horizontal axis: brightness from 0 to paper white to max luminance
    // Vertical axis: hue at max saturation
    let n_brightness = 21;
    let n_hues = 11;
    let spacing = 0.6;
    for i in 0..n_brightness {
        for j in 0..=n_hues {
            // Horizontal axis: brightness from 0 to paper white to max luminance
            // Vertical axis: hue at max saturation
            let (hue, saturation) = if j < n_hues {
                (j as f32 / (n_hues - 1) as f32 * 360.0, 1.0)
            } else {
                (0.0, 0.0) // Grayscale row on the bottom
            };
            let brightness = i as f32 / (n_brightness - 1) as f32; // 0.0 to 1.0 (will map to 0.0 to max_ratio)

            commands.spawn((
                Mesh3d(meshes.add(Sphere::new(0.2).mesh().ico(7).unwrap())),
                MeshMaterial3d(materials.add(StandardMaterial {
                    base_color: Color::BLACK.into(),
                    emissive: LinearRgba::WHITE.into(),
                    ..default()
                })),
                Transform::from_xyz(
                    i as f32 * spacing - (n_brightness as f32 - 1.0) * (spacing / 2.0),
                    j as f32 * spacing - (n_hues as f32) * (spacing / 2.0),
                    0.0,
                ),
                CalibrationSphere {
                    hue,
                    saturation,
                    brightness,
                },
                Calibration3dMarker,
                Visibility::Hidden,
            ));
        }
    }

    // Directional Light for reflections
    commands.spawn((
        DirectionalLight {
            illuminance: 1000.0,
            shadow_maps_enabled: true,
            ..default()
        },
        Transform::from_xyz(1.0, 1.0, 1.0).looking_at(Vec3::ZERO, Vec3::Y),
        Calibration3dMarker,
        Visibility::Hidden,
    ));

    // UI
    commands.spawn((
        Text::new(""),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(12.0),
            left: Val::Px(12.0),
            ..default()
        },
        CalibrationText,
    ));
}

fn toggle_mode(
    keys: Res<ButtonInput<KeyCode>>,
    mut status: ResMut<CalibrationStatus>,
    mut query_2d: Query<&mut HdrCalibrationEffect, With<Calibration2dMarker>>,
    mut query_3d: Query<&mut Visibility, With<Calibration3dMarker>>,
) {
    if keys.just_pressed(KeyCode::KeyM) {
        status.mode = match status.mode {
            SceneMode::Calibration2d => SceneMode::Calibration3d,
            SceneMode::Calibration3d => SceneMode::Calibration2d,
        };

        for _effect in &mut query_2d {
            // FullscreenMaterial doesn't have a built-in "disabled" state easily toggled here
            // but we can manage it by changing how we render it or by using Visibility if it was a normal component.
            // However, FullscreenMaterialNode runs if the component is present.
            // For this example, let's just use a dummy value in the shader to bypass it.
        }

        let is_3d = status.mode == SceneMode::Calibration3d;
        for mut visibility in &mut query_3d {
            *visibility = if is_3d {
                Visibility::Visible
            } else {
                Visibility::Hidden
            };
        }
    }
}

fn update_scene(
    grading: Query<&ColorGrading, With<Camera>>,
    mut spheres: Query<(&CalibrationSphere, &MeshMaterial3d<StandardMaterial>)>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let Ok(grading) = grading.single() else {
        return;
    };
    let max_ratio = grading.global.max_luminance / grading.global.paper_white;

    for (sphere, material_handle) in &mut spheres {
        if let Some(material) = materials.get_mut(&material_handle.0) {
            let intensity = sphere.brightness * max_ratio;
            let color = if sphere.saturation > 0.0 {
                LinearRgba::from(Hsla {
                    hue: sphere.hue,
                    saturation: sphere.saturation,
                    lightness: 0.5,
                    alpha: 1.0,
                })
            } else {
                LinearRgba::WHITE
            };
            material.emissive = (color * intensity).into();
        }
    }
}

fn update_calibration(
    keys: Res<ButtonInput<KeyCode>>,
    mut camera_query: Query<
        (
            &mut ColorGrading,
            &mut Tonemapping,
            &mut HdrCalibrationEffect,
        ),
        With<Camera>,
    >,
    mut window_query: Query<&mut Window, With<PrimaryWindow>>,
    status: Res<CalibrationStatus>,
) {
    for (mut grading, mut tonemapping, mut effect) in &mut camera_query {
        if keys.pressed(KeyCode::ArrowUp) {
            grading.global.paper_white += 1.0;
        }
        if keys.pressed(KeyCode::ArrowDown) {
            grading.global.paper_white -= 1.0;
        }
        if keys.pressed(KeyCode::ArrowRight) {
            grading.global.max_luminance += 10.0;
        }
        if keys.pressed(KeyCode::ArrowLeft) {
            grading.global.max_luminance -= 10.0;
        }

        if keys.just_pressed(KeyCode::KeyH) {
            if let Ok(mut window) = window_query.single_mut() {
                window.hdr_output = !window.hdr_output;
                *tonemapping = Tonemapping::None;
            }
        }

        if keys.just_pressed(KeyCode::KeyT) {
            if *tonemapping == Tonemapping::None {
                if let Ok(window) = window_query.single() {
                    if window.hdr_output {
                        *tonemapping = Tonemapping::Pq;
                    } else {
                        *tonemapping = Tonemapping::TonyMcMapface;
                    }
                }
            } else {
                *tonemapping = Tonemapping::None;
            }
        }

        grading.global.paper_white = grading.global.paper_white.max(1.0);
        grading.global.max_luminance = grading.global.max_luminance.max(grading.global.paper_white);

        // Manage effect bypass
        if status.mode == SceneMode::Calibration3d {
            effect.enabled = 0.0;
        } else {
            effect.enabled = 1.0;
        }
    }
}

fn update_text(
    mut text_query: Query<&mut Text, With<CalibrationText>>,
    camera_query: Query<(&ColorGrading, &Tonemapping), With<Camera>>,
    window_query: Query<&Window, With<PrimaryWindow>>,
    status: Res<CalibrationStatus>,
) {
    let Ok(window) = window_query.single() else {
        return;
    };
    for (grading, tonemapping) in &camera_query {
        let is_tonemapping_enabled = tonemapping.is_enabled();
        for mut text in &mut text_query {
            text.0 = format!(
                "Mode: {} (M to toggle)\nOutput: {} (H to toggle)\nTonemapping: {} (T to toggle)\nPaper White: {:.0} nits (Up/Down)\nMax Luminance: {:.0} nits (Left/Right)\n\n\
                {}\n\n\
                Note: HDR output requires an HDR display and OS support.",
                match status.mode {
                    SceneMode::Calibration2d => "2D Calibration Pattern",
                    SceneMode::Calibration3d => "3D Scene",
                },
                if window.hdr_output { "HDR Output" } else { "SDR" },
                if is_tonemapping_enabled { format!("{:?}", *tonemapping) } else { "None".to_string() },
                grading.global.paper_white,
                grading.global.max_luminance,
                match status.mode {
                    SceneMode::Calibration2d => "Top half: Grayscale gradient (0.0 to Max / Paper White)\nRed line: Paper White level (1.0)\nBottom half: Color hue gradient (upper) and at Max Luminance (very bottom strip)",
                    SceneMode::Calibration3d => "Spheres vary hue vertically (0-360) and lightness horizontally (0 to Max Luminance). Bottom row is grayscale.",
                }
            );
        }
    }
}

#[derive(Component, ExtractComponent, Clone, Copy, ShaderType)]
struct HdrCalibrationEffect {
    enabled: f32,
}

impl Default for HdrCalibrationEffect {
    fn default() -> Self {
        Self { enabled: 1.0 }
    }
}

impl FullscreenMaterial for HdrCalibrationEffect {
    fn fragment_shader() -> ShaderRef {
        "shaders/hdr_calibration.wgsl".into()
    }

    fn node_edges() -> Vec<InternedRenderLabel> {
        vec![
            Node3d::EndMainPass.intern(),
            Self::node_label().intern(),
            Node3d::Tonemapping.intern(),
        ]
    }
}
