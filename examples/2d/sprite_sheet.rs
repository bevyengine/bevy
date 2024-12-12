//! Renders an animated sprite by loading all animation frames from a single image (a sprite sheet)
//! into a texture atlas, and changing the displayed image periodically.

use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(ImagePlugin::default_nearest())) // prevents blurry sprites
        .add_systems(Startup, setup)
        .add_systems(Update, animate_sprite)
        .run();
}

#[derive(Component, Deref, DerefMut)]
struct AnimationTimer(Timer);

fn animate_sprite(
    time: Res<Time>,
    mut query: Query<(&mut AnimationTimer, &mut Sprite)>,
) {
    for (mut timer, mut sprite) in &mut query {
        timer.tick(time.delta());

        if timer.just_finished() {
            if let Some(atlas) = &mut sprite.texture_atlas {
                atlas.advance_index();
            }
        }
    }
}

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut texture_atlas_layouts: ResMut<Assets<TextureAtlasLayout>>,
) {
    let texture = asset_server.load("textures/rpg/chars/gabe/gabe-idle-run.png");
    let layout = TextureAtlasLayout::from_grid(UVec2::splat(24), 7, 1, None, None);
    let texture_atlas_layout = texture_atlas_layouts.add(layout);

    commands.spawn(Camera2d);
    commands.spawn((
        Sprite::from_atlas_image(
            texture,
            TextureAtlas {
                layout: texture_atlas_layout,
                index: 1,
                index_range: (1, 6)
            },
        ),
        Transform::from_scale(Vec3::splat(6.0)),
        AnimationTimer(Timer::from_seconds(0.1, TimerMode::Repeating)),
    ));
}
