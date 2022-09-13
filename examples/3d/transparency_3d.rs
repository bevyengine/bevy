//! Demonstrates how to use transparency in 3D.
//! Shows the effects of different blend modes.
//! The `fade_transparency` system smoothly changes the transparency over time.

use bevy::prelude::*;

fn main() {
    App::new()
        .insert_resource(Msaa { samples: 4 })
        .add_plugins(DefaultPlugins)
        .add_startup_system(setup)
        .add_system(fade_transparency)
        .run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // opaque plane, uses `alpha_mode: Opaque` by default
    commands.spawn_bundle(PbrBundle {
        mesh: meshes.add(Mesh::from(shape::Plane { size: 6.0 })),
        material: materials.add(Color::rgb(0.3, 0.5, 0.3).into()),
        ..default()
    });
    // transparent sphere, uses `alpha_mode: Mask(f32)`
    commands.spawn_bundle(PbrBundle {
        mesh: meshes.add(Mesh::from(shape::Icosphere {
            radius: 0.5,
            subdivisions: 3,
        })),
        material: materials.add(StandardMaterial {
            // Alpha channel of the color controls transparency.
            // We set it to 0.0 here, because it will be changed over time in the
            // `fade_transparency` function.
            // Note that the transparency has no effect on the objects shadow.
            base_color: Color::rgba(0.2, 0.7, 0.1, 0.0),
            // Mask sets a cutoff for transparency. Alpha values below are fully transparent,
            // alpha values above are fully opaque.
            alpha_mode: AlphaMode::Mask(0.5),
            ..default()
        }),
        transform: Transform::from_xyz(1.0, 0.5, -1.5),
        ..default()
    });
    // transparent cube, uses `alpha_mode: Blend`
    commands.spawn_bundle(PbrBundle {
        mesh: meshes.add(Mesh::from(shape::Cube { size: 1.0 })),
        // Notice how there is no need to set the `alpha_mode` explicitly here.
        // When converting a color to a material using `into()`, the alpha mode is
        // automatically set to `Blend` if the alpha channel is anything lower than 1.0.
        material: materials.add(Color::rgba(0.5, 0.5, 1.0, 0.0).into()),
        transform: Transform::from_xyz(0.0, 0.5, 0.0),
        ..default()
    });
    // opaque sphere
    commands.spawn_bundle(PbrBundle {
        mesh: meshes.add(Mesh::from(shape::Icosphere {
            radius: 0.5,
            subdivisions: 3,
        })),
        material: materials.add(Color::rgb(0.7, 0.2, 0.1).into()),
        transform: Transform::from_xyz(0.0, 0.5, -1.5),
        ..default()
    });
    // light
    commands.spawn_bundle(PointLightBundle {
        point_light: PointLight {
            intensity: 1500.0,
            shadows_enabled: true,
            ..default()
        },
        transform: Transform::from_xyz(4.0, 8.0, 4.0),
        ..default()
    });
    // camera
    commands.spawn_bundle(Camera3dBundle {
        transform: Transform::from_xyz(-2.0, 3.0, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
        ..default()
    });
}

/// Fades the alpha channel of all materials between 0 and 1 over time.
/// Each blend mode responds differently to this:
/// - `Opaque`: Ignores alpha channel altogether, these materials stay completely opaque.
/// - `Mask(f32)`: Object appears when the alpha value goes above the mask's threshold, disappears
///                when the alpha value goes back below the threshold.
/// - `Blend`: Object fades in and out smoothly.
pub fn fade_transparency(time: Res<Time>, mut materials: ResMut<Assets<StandardMaterial>>) {
    let alpha = (time.time_since_startup().as_secs_f32().sin() / 2.0) + 0.5;
    for (_, material) in materials.iter_mut() {
        material.base_color.set_a(alpha);
    }
}
