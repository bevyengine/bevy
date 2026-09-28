//! Demonstrates using `SpriteMaterial`s to create custom materials for sprites

use bevy::{
    input::common_conditions::input_just_pressed, prelude::*, render::render_resource::AsBindGroup,
    shader::ShaderRef,
};

/// This example uses a shader source file from the assets subdirectory
const SHADER_ASSET_PATH: &str = "shaders/sprite_material.wesl";

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins,
            SpriteMaterialPlugin::<DissolveMaterial>::default(),
        ))
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                dissolve_on_input.run_if(input_just_pressed(KeyCode::Space)),
                update_dissolve,
            )
                .chain(),
        )
        .run();
}

#[derive(AsBindGroup, Asset, Reflect, Clone)]
struct DissolveMaterial {
    // Start at 20 to not collide with the sprite's bindings
    #[uniform(20)]
    burn_edge_color: LinearRgba,
    #[uniform(20)]
    amount: f32,
    #[uniform(20)]
    burn_edge: f32,

    // WebGL2 requires structs to be aligned to 16 bytes
    #[cfg(feature = "webgl2")]
    #[uniform(20)]
    _webgl2_padding_12b: u32,
    #[cfg(feature = "webgl2")]
    #[uniform(20)]
    _webgl2_padding_16b: u32,
}

impl MaterialExtension2d for DissolveMaterial {
    fn fragment_shader() -> Option<ShaderRef> {
        Some(SHADER_ASSET_PATH.into())
    }
}

impl DissolveMaterial {
    fn new(amount: f32) -> Self {
        Self {
            amount,
            burn_edge: 0.05,
            burn_edge_color: LinearRgba::rgb(1.0, 0.66, 0.0),

            #[cfg(feature = "webgl2")]
            _webgl2_padding_12b: 0,
            #[cfg(feature = "webgl2")]
            _webgl2_padding_16b: 0,
        }
    }
}

#[derive(Component, FromTemplate, Clone, Copy)]
#[component(immutable)]
enum DissolveState {
    #[default]
    Visible,
    Dissolving {
        progress: f32,
    },
    Appearing {
        progress: f32,
    },
    Hidden,
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);
    commands.spawn(Text::new("Space to dissolve"));

    commands.queue_spawn_scene(bsn! {
        Sprite {
            image: "branding/bevy_bird_dark.png",
        }
        DissolveState::Visible
        SpriteMaterial<DissolveMaterial>(asset_value(DissolveMaterial::new(0.0)))
    });
}

fn dissolve_on_input(dissolving: Query<(Entity, &DissolveState)>, mut commands: Commands) {
    for (entity, state) in dissolving {
        let new = match *state {
            DissolveState::Visible => DissolveState::Dissolving { progress: 0.0 },
            DissolveState::Hidden => DissolveState::Appearing { progress: 0.0 },
            DissolveState::Dissolving { progress } => DissolveState::Appearing {
                progress: 1.0 - progress,
            },
            DissolveState::Appearing { progress } => DissolveState::Dissolving {
                progress: 1.0 - progress,
            },
        };

        commands.entity(entity).insert(new);
    }
}

fn update_dissolve(
    dissolving: Query<(Entity, &SpriteMaterial<DissolveMaterial>, &DissolveState)>,
    time: Res<Time>,
    mut materials: ResMut<Assets<DissolveMaterial>>,
    mut commands: Commands,
) {
    for (entity, material, state) in dissolving {
        match *state {
            DissolveState::Dissolving { mut progress } => {
                progress += time.delta_secs();
                let new = if progress >= 1.0 {
                    progress = 1.0;
                    DissolveState::Hidden
                } else {
                    DissolveState::Dissolving { progress }
                };
                commands.entity(entity).insert(new);

                if let Some(mut material) = materials.get_mut(&material.0) {
                    material.amount = progress;
                }
            }
            DissolveState::Appearing { mut progress } => {
                progress += time.delta_secs();
                let new = if progress >= 1.0 {
                    progress = 1.0;
                    DissolveState::Visible
                } else {
                    DissolveState::Appearing { progress }
                };
                commands.entity(entity).insert(new);

                if let Some(mut material) = materials.get_mut(&material.0) {
                    material.amount = 1.0 - progress;
                }
            }
            _ => { /* ignore */ }
        }
    }
}
