//! This example illustrates various ways of loading a `TextureAtlasLayout`

use bevy::prelude::*;
#[derive(Component)]
struct AnimationIndices {
    first: usize,
    last: usize,
}

#[derive(Component, Deref, DerefMut)]
struct AnimationTimer(Timer);

const Y_EXTENT: f32 = 150.0;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(ImagePlugin::default_nearest()))
        .add_systems(Startup, setup)
        .add_systems(Update, animate_sprite)
        .run();
}

fn setup(mut commands: Commands, assets: Res<AssetServer>) {
    commands.spawn(Camera2dBundle::default());

    // You can load a TextureAtlasLayout using the AssetServer.
    // The TextureAtlasLayout can then be used for multiple textures.
    // There are multiple formats available to load a TextureAtlasLayout,
    // for example here we load a *.atlas.grid.ron file, which specifies a grid wherein all textures are located.
    // This is similar to `TextureAtlasLayout::from_grid`
    let character_layout = assets.load("texture_atlas/character.atlas.grid.ron");

    // You can also specify all texture placements manually using a *.atlas.ron
    let slime_layout = assets.load("texture_atlas/slime.atlas.ron");

    // Lastly, you may specify a series of texture assets from which the TextureAtlasLayout should be built in a *.atlas.composed.ron file.
    // This will also create a texture combining all the textures you specified in the source file into a new one.
    let chest_layout = assets.load("texture_atlas/chest.atlas.composed.ron");
    // You can access that texture by loading the labeled asset 'composed_texture' from the layout asset.
    let chest_texture = assets.load("texture_atlas/chest.atlas.composed.ron#composed_texture");

    let character_textures = [
        assets.load("textures/rpg/chars/gabe/gabe-idle-run.png"),
        assets.load("textures/rpg/chars/mani/mani-idle-run.png"),
    ];
    let slime_textures = [
        assets.load("textures/rpg/mobs/slime-blue.png"),
        assets.load("textures/rpg/mobs/slime-green.png"),
        assets.load("textures/rpg/mobs/slime-orange.png"),
    ];

    // Spawn characters
    let num_characters = character_textures.len();
    for (i, texture) in character_textures.into_iter().enumerate() {
        commands.spawn((
            SpriteSheetBundle {
                texture,
                // You need to create the actual TextureAtlas using the previously loaded TextureAtlasLayout
                atlas: TextureAtlas {
                    // Note, that we are reusing the same TextureAtlasLayout for all characters here
                    layout: character_layout.clone(),
                    index: 1,
                },
                transform: Transform::from_xyz(
                    -200.,
                    -Y_EXTENT / 2. + i as f32 / (num_characters - 1) as f32 * Y_EXTENT,
                    0.,
                )
                .with_scale(Vec3::splat(3.0)),
                ..default()
            },
            AnimationIndices { first: 1, last: 6 },
            AnimationTimer(Timer::from_seconds(0.1, TimerMode::Repeating)),
        ));
    }
    // Spawn chest
    commands.spawn((
        SpriteSheetBundle {
            texture: chest_texture,
            atlas: TextureAtlas {
                layout: chest_layout.clone(),
                ..Default::default()
            },
            transform: Transform::from_scale(Vec3::splat(3.0)),
            ..default()
        },
        AnimationIndices { first: 0, last: 1 },
        AnimationTimer(Timer::from_seconds(0.5, TimerMode::Repeating)),
    ));
    // Spawn slimes
    let num_slimes = slime_textures.len();
    for (i, texture) in slime_textures.into_iter().enumerate() {
        commands.spawn((
            SpriteSheetBundle {
                texture,
                atlas: TextureAtlas {
                    layout: slime_layout.clone(),
                    ..Default::default()
                },
                transform: Transform::from_xyz(
                    200.,
                    -Y_EXTENT / 2. + i as f32 / (num_slimes - 1) as f32 * Y_EXTENT,
                    0.,
                )
                .with_scale(Vec3::splat(3.0)),
                ..default()
            },
            AnimationIndices { first: 0, last: 3 },
            AnimationTimer(Timer::from_seconds(0.2, TimerMode::Repeating)),
        ));
    }
}

fn animate_sprite(
    time: Res<Time>,
    mut query: Query<(&AnimationIndices, &mut AnimationTimer, &mut TextureAtlas)>,
) {
    for (indices, mut timer, mut atlas) in &mut query {
        timer.tick(time.delta());
        if timer.just_finished() {
            atlas.index = if atlas.index == indices.last {
                indices.first
            } else {
                atlas.index + 1
            };
        }
    }
}
