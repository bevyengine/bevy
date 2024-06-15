//! Displays a single [`Sprite`] tiled in a grid, with a scaling animation

use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_systems(Update, animate)
        .run();
}

#[derive(Resource)]
struct AnimationState {
    min: f32,
    max: f32,
    current: f32,
    speed: f32,
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn(Camera2dBundle::default());
    commands.insert_resource(AnimationState {
        min: 128.0,
        max: 512.0,
        current: 128.0,
        speed: 50.0,
    });
    commands.spawn((
        SpriteBundle {
            texture: asset_server.load("branding/icon.png"),
            ..default()
        },
        ImageScaleMode::Tiled {
            tile_x: true,
            tile_y: true,
            stretch_value: 0.5, // The image will tile every 128px
        },
    ));
}

fn animate(mut sprites: Query<&mut Sprite>, mut state: ResMut<AnimationState>, time: Res<Time>) {
    if state.current >= state.max || state.current <= state.min {
        state.speed = -state.speed;
    };
    state.current += state.speed * time.delta_seconds();
    for mut sprite in &mut sprites {
        sprite.custom_size = Some(Vec2::splat(state.current));
    }
}
