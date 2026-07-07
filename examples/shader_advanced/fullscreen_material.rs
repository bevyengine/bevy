//! Demonstrates how to write a custom fullscreen shader
//!
//! This example demonstrates working in 3d. To make the example work in 2d,
//! replace 3d components with their 2d counterparts, and schedule the work
//! to run in the `Core2d` schedule as described in the `FullscreenMaterial`
//! comment in this file.

use bevy::{
    core_pipeline::fullscreen_material::{FullscreenMaterial, FullscreenMaterialPlugin},
    prelude::*,
    render::{extract_component::ExtractComponent, render_resource::ShaderType},
    shader::ShaderRef,
};

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins,
            FullscreenMaterialPlugin::<FullscreenEffect>::default(),
        ))
        .add_systems(Startup, setup)
        .add_systems(Update, (update_intensity, toggle_effect))
        .run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.spawn((
        Camera3d::default(),
        Transform::from_translation(Vec3::new(0.0, 0.0, 5.0)).looking_at(Vec3::default(), Vec3::Y),
        FullscreenEffect::new(FullscreenEffect::MAX_INTENSITY),
    ));

    commands.spawn((
        Mesh3d(meshes.add(Cuboid::default())),
        MeshMaterial3d(materials.add(Color::srgb(0.8, 0.7, 0.6))),
        Transform::default(),
    ));

    commands.spawn(DirectionalLight {
        illuminance: 1_000.,
        ..default()
    });

    commands.spawn((
        Text::new("(T) FullscreenEffect: On"),
        Node {
            position_type: PositionType::Absolute,
            top: px(12),
            left: px(12),
            ..default()
        },
    ));
}

fn update_intensity(mut effects: Query<&mut FullscreenEffect>, time: Res<Time>) {
    for mut effect in &mut effects {
        let phase = time.elapsed_secs() * FullscreenEffect::FREQUENCY;
        // Make it loop periodically
        let mut intensity = ops::sin(phase);

        // We need to remap the intensity to be between 0 and 1 instead of -1 and 1
        intensity = (intensity + 1.0) / 2.0;
        effect.intensity = intensity * FullscreenEffect::MAX_INTENSITY;
    }
}

fn toggle_effect(
    mut text: Single<&mut Text>,
    keys: Res<ButtonInput<KeyCode>>,
    camera: Single<(Entity, Option<&FullscreenEffect>), With<Camera3d>>,
    mut commands: Commands,
) {
    if keys.just_pressed(KeyCode::KeyT) {
        let (entity, effect) = *camera;

        if effect.is_some() {
            commands.entity(entity).remove::<FullscreenEffect>();
            text.clear();
            text.push_str("(T) FullscreenEffect: Off");
        } else {
            commands
                .entity(entity)
                .insert(FullscreenEffect::new(FullscreenEffect::MAX_INTENSITY));
            text.clear();
            text.push_str("(T) FullscreenEffect: On");
        }
    }
}

#[derive(Component, ExtractComponent, Clone, Copy, ShaderType, Default)]
struct FullscreenEffect {
    intensity: f32,
    // WebGL2 structs must be 16 byte aligned.
    // Intensity is an `f32`, which is 4 bytes, so 12 more bytes (3 floats) are needed.
    #[cfg(feature = "webgl2")]
    _webgl2_padding: Vec3,
}

impl FullscreenEffect {
    const FREQUENCY: f32 = 2.0;
    const MAX_INTENSITY: f32 = 0.015;

    fn new(intensity: f32) -> Self {
        Self {
            intensity,
            ..Default::default()
        }
    }
}

impl FullscreenMaterial for FullscreenEffect {
    fn fragment_shader() -> ShaderRef {
        "shaders/fullscreen_effect.wgsl".into()
    }

    // The `FullscreenMaterial` uses 3d schedules by default.
    // To make this work in 2d, you would need to schedule to
    // run in `Core2d` and in a `Core2dSystems` set.
    //
    // fn schedule() -> impl bevy::ecs::schedule::ScheduleLabel + Clone {
    //     bevy::core_pipeline::Core2d
    // }
    // fn schedule_configs(
    //     system: bevy::ecs::schedule::ScheduleConfigs<bevy::ecs::system::BoxedSystem>,
    // ) -> bevy::ecs::schedule::ScheduleConfigs<bevy::ecs::system::BoxedSystem> {
    //     system
    //         .in_set(bevy::core_pipeline::Core2dSystems::PostProcess)
    //         .before(bevy::core_pipeline::tonemapping::tonemapping)
    // }
}
