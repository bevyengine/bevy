//! Demonstrates how to write a custom fullscreen shader
//!
//! This is currently limited to 3d only but work is in progress to make it work in 2d

use bevy::{
    core_pipeline::{
        core_3d::graph::{Core3d, Node3d},
        fullscreen_material::{FullscreenMaterial, FullscreenMaterialPlugin},
    },
    prelude::*,
    render::{
        extract_component::ExtractComponent,
        render_graph::{InternedRenderLabel, RenderLabel, RenderSubGraph},
        render_resource::ShaderType,
    },
    shader::ShaderRef,
};

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins,
            FullscreenMaterialPlugin::<FullscreenEffect>::default(),
        ))
        .add_systems(Startup, setup)
        .run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // camera
    commands.spawn((
        Camera3d::default(),
        Transform::from_translation(Vec3::new(0.0, 0.0, 5.0)).looking_at(Vec3::default(), Vec3::Y),
        FullscreenEffect { intensity: 0.005 },
    ));

    // cube
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::default())),
        MeshMaterial3d(materials.add(Color::srgb(0.8, 0.7, 0.6))),
        Transform::default(),
    ));
    // light
    commands.spawn(DirectionalLight {
        illuminance: 1_000.,
        ..default()
    });
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, RenderLabel)]
struct FullscreenEffectLabel;

#[derive(Component, ExtractComponent, Clone, Copy, ShaderType, Default)]
struct FullscreenEffect {
    // For this example, this is used as the intensity of the effect, but you can pass in any valid
    // ShaderType
    //
    // In the future, you will be able to use a full bind group
    intensity: f32,
}

impl FullscreenMaterial for FullscreenEffect {
    // The shader that will be used
    fn fragment_shader() -> ShaderRef {
        "shaders/fullscreen_effect.wgsl".into()
    }

    // The material needs to know the label that will be used for the render graph
    fn node_label() -> impl RenderLabel {
        FullscreenEffectLabel
    }

    // The sub graph the effect will run in. In 2d, this will generally be [`Core2d`] and in 3d it will
    // be [`Core3d`]
    fn sub_graph() -> impl RenderSubGraph {
        Core3d
    }

    // This let's you specify a list of edges used to order when your effect pass will run
    //
    // This example is a post processing effect so it will run after tonemapping but before the end
    // post processing pass.
    //
    // In 2d you would need to use [`Node2d`] instead of [`Node3d`]
    fn node_edges() -> Vec<InternedRenderLabel> {
        vec![
            Node3d::Tonemapping.intern(),
            Self::node_label().intern(),
            Node3d::EndMainPassPostProcessing.intern(),
        ]
    }
}
