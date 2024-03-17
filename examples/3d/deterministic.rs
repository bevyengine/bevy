//! Shows how to enable deterministic rendering which helps with flickering due to z-fighting.
//! Rendering is not deterministic by default.
//! Note most users don't need rendering to be deterministic, and should rely on depth bias instead.

use bevy::{
    app::{App, Startup},
    prelude::*,
    render::deterministic::DeterministicRenderingConfig,
};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_systems(Update, (keys, update_help).chain())
        .run();
}

fn setup(
    mut commands: Commands,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut deterministic_rendering_config: ResMut<DeterministicRenderingConfig>,
) {
    // Safe default.
    deterministic_rendering_config.stable_sort_z_fighting = true;

    // Help message will be rendered there.
    commands.spawn(TextBundle::default().with_style(Style {
        position_type: PositionType::Absolute,
        top: Val::Px(12.0),
        left: Val::Px(12.0),
        ..default()
    }));

    commands.spawn(Camera3dBundle {
        transform: Transform::from_xyz(3.0, 3.0, 3.0).looking_at(Vec3::new(0., 0., 0.), Vec3::Y),
        ..default()
    });

    let mesh = meshes.add(Plane3d::default().mesh().size(2.0, 2.0));
    let nb_plane = 10;
    for i in 0..nb_plane {
        let color = Color::hsl(i as f32 * 360.0 / nb_plane as f32, 1.0, 0.5);
        commands.spawn(PbrBundle {
            mesh: mesh.clone(),
            material: materials.add(StandardMaterial {
                base_color: color,
                // Setting depth bias would be a default choice to fix z-fighting.
                // When it is not possible, deterministic rendering can be used.
                // Here we intentionally don't use depth bias to demonstrate the issue.
                depth_bias: 0.0,
                unlit: true,
                ..Default::default()
            }),
            ..default()
        });
    }
}

fn keys(
    mut deterministic_rendering_config: ResMut<DeterministicRenderingConfig>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
) {
    if keyboard_input.just_pressed(KeyCode::KeyD) {
        deterministic_rendering_config.stable_sort_z_fighting ^= true;
    }
}

fn update_help(
    mut text: Query<&mut Text>,
    deterministic_rendering_config: Res<DeterministicRenderingConfig>,
) {
    if deterministic_rendering_config.is_changed() {
        *text.single_mut() = Text::from_section(
            format!(
                "\
            Press D to enable/disable deterministic rendering\n\
            \n\
            Deterministic rendering: {}\n\
            \n\
            When rendering is not deterministic, you may notice flickering due to z-fighting\n\
            \n\
            Warning: may cause seizures for people with photosensitive epilepsy",
                deterministic_rendering_config.stable_sort_z_fighting
            ),
            TextStyle {
                font_size: 20.,
                ..default()
            },
        );
    }
}
