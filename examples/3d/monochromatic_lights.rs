//! Showcases support for monochromatic lights in a 3D scene.
//!
//! ## Controls
//!
//! | Key Binding        | Action                                               |
//! |:-------------------|:-----------------------------------------------------|
//! | 1–6                | Switch light preset                                  |

use bevy::{camera::SpectralModel, prelude::*};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_systems(Update, update)
        .insert_resource(GlobalAmbientLight {
            brightness: 0.0,
            ..default()
        })
        .insert_resource(ClearColor(Color::srgb(0.0, 0.0, 0.0)))
        .run();
}

const AMBER: Color = Color::linear_rgb(1.0, 0.216, 0.0);
const PURPLE: Color = Color::linear_rgb(0.164, 0.0, 1.0);

#[derive(Debug, Default)]
enum LightPreset {
    #[default]
    White,
    Amber,
    SodiumVapor,
    Purple,
    Violet,
    MonochromaticWhite, // Non-physical!
}

/// set up a simple 3D scene
fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // camera
    commands.spawn((
        Camera3d::default(),
        SpectralModel::MonochromaticLights, // Important: Enables monochromatic lights support
        Transform::from_xyz(-3.5, 7.0, 12.0).looking_at(Vec3::new(0.0, 2.5, 0.), Vec3::Y),
    ));

    // circular base
    commands.spawn((
        Mesh3d(meshes.add(Circle::new(4.0))),
        MeshMaterial3d(materials.add(Color::WHITE)),
        Transform::from_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2)),
    ));

    // red sphere
    commands.spawn((
        Mesh3d(meshes.add(Sphere::new(1.0))),
        MeshMaterial3d(materials.add(Color::srgb(1.0, 0.0, 0.0))),
        Transform::from_xyz(-2.0, 1.0, 0.0),
    ));

    // green sphere
    commands.spawn((
        Mesh3d(meshes.add(Sphere::new(1.0))),
        MeshMaterial3d(materials.add(Color::srgb(0.0, 1.0, 0.0))),
        Transform::from_xyz(0.0, 1.0, 0.0),
    ));

    // blue sphere
    commands.spawn((
        Mesh3d(meshes.add(Sphere::new(1.0))),
        MeshMaterial3d(materials.add(Color::srgb(0.0, 0.0, 1.0))),
        Transform::from_xyz(2.0, 1.0, 0.0),
    ));

    // HSV cubes
    for j in 0..=5 {
        for i in 0..=17 {
            commands.spawn((
                Mesh3d(meshes.add(Cuboid::new(0.5, 0.5, 0.5))),
                MeshMaterial3d(materials.add(StandardMaterial {
                    base_color: if j == 5 {
                        // Grayscale band at the top
                        Color::hsv(0.0, 0.0, i as f32 / 17.0)
                    } else {
                        Color::hsv(i as f32 * 15.0, j as f32 / 4.0, 1.0)
                    },
                    perceptual_roughness: 1.0,
                    reflectance: 0.0,
                    ..default()
                })),
                Transform::from_xyz(-4.0 + i as f32 * 0.5, 2.5 + j as f32 * 0.5, 0.0),
            ));
        }
    }

    // point light
    commands.spawn((
        PointLight {
            shadow_maps_enabled: true,

            ..default()
        },
        Transform::from_xyz(4.0, 8.0, 4.0),
    ));

    // UI
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            padding: UiRect::all(px(5)),
            ..default()
        },
        children![(Text::default(), children![TextSpan::new("")])],
    ));
}

fn update(
    mut text_query: Query<&mut TextSpan>,
    mut light_query: Query<&mut PointLight>,
    mut light_type: Local<LightPreset>,
    keyboard: Res<ButtonInput<KeyCode>>,
) -> Result {
    let mut light = light_query.single_mut()?;
    let mut text = text_query.single_mut()?;

    if keyboard.just_pressed(KeyCode::Digit1) {
        *light_type = LightPreset::White;
    }

    if keyboard.just_pressed(KeyCode::Digit2) {
        *light_type = LightPreset::Amber;
    }

    if keyboard.just_pressed(KeyCode::Digit3) {
        *light_type = LightPreset::SodiumVapor;
    }

    if keyboard.just_pressed(KeyCode::Digit4) {
        *light_type = LightPreset::Purple;
    }

    if keyboard.just_pressed(KeyCode::Digit5) {
        *light_type = LightPreset::Violet;
    }

    if keyboard.just_pressed(KeyCode::Digit6) {
        *light_type = LightPreset::MonochromaticWhite;
    }

    match *light_type {
        LightPreset::Amber => {
            light.color = AMBER;
            light.monochromatic = false;
        }
        LightPreset::SodiumVapor => {
            light.color = AMBER;
            light.monochromatic = true;
        }
        LightPreset::Purple => {
            light.color = PURPLE;
            light.monochromatic = false;
        }
        LightPreset::Violet => {
            // See: https://en.wikipedia.org/wiki/Violet_(color)#Relationship_to_purple
            light.color = PURPLE;
            light.monochromatic = true;
        }
        LightPreset::White => {
            light.color = Color::WHITE;
            light.monochromatic = false;
        }
        LightPreset::MonochromaticWhite => {
            light.color = Color::WHITE;
            light.monochromatic = true;
        }
    }

    let linear_rgb = light.color.to_linear();

    text.0 = format!(
        "Preset:\n{} 1. White Light (e.g. Sun or Cool LED Light)\n{} 2. Amber Polychromatic Light (e.g. Incandescent or Warm LED Light)\n{} 3. Amber Monochromatic Light (e.g. Sodium Vapor Light)\n{} 4. Purple Light\n{} 5. Violet Light\n{} 6. White Monochromatic Light (Non-Physical!)\n\nMonochromatic: {}\nR: {}\nG: {}\nB: {}",
        if let LightPreset::White = *light_type { "*" } else { " " },
        if let LightPreset::Amber = *light_type { "*" } else { " " },
        if let LightPreset::SodiumVapor = *light_type { "*" } else { " " },
        if let LightPreset::Purple = *light_type { "*" } else { " " },
        if let LightPreset::Violet = *light_type { "*" } else { " " },
        if let LightPreset::MonochromaticWhite = *light_type { "*" } else { " " },
        light.monochromatic, linear_rgb.red, linear_rgb.green, linear_rgb.blue
    );

    Ok(())
}
