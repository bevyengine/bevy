//! Shows how to use sprite scaling to fill and fit textures into the sprite.

use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, (setup_sprites, setup_texture_atlas, setup_camera))
        .add_systems(Update, animate_sprite)
        .run();
}

fn setup_camera(mut commands: Commands) {
    commands.spawn(Camera2d);
}

fn setup_sprites(mut commands: Commands, asset_server: Res<AssetServer>) {
    let square = asset_server.load("textures/slice_square_2.png");
    let banner = asset_server.load("branding/banner.png");

    let rects = [
        Rect {
            size: Vec2::new(100., 225.),
            text: "Stretched".to_string(),
            transform: Transform::from_translation(Vec3::new(-570., 230., 0.)),
            texture: square.clone(),
            image_mode: SpriteImageMode::Auto,
        },
        Rect {
            size: Vec2::new(100., 225.),
            text: "Fill Center".to_string(),
            transform: Transform::from_translation(Vec3::new(-450., 230., 0.)),
            texture: square.clone(),
            image_mode: SpriteImageMode::Scale(ScalingMode::FillCenter),
        },
        Rect {
            size: Vec2::new(100., 225.),
            text: "Fill Start".to_string(),
            transform: Transform::from_translation(Vec3::new(-330., 230., 0.)),
            texture: square.clone(),
            image_mode: SpriteImageMode::Scale(ScalingMode::FillStart),
        },
        Rect {
            size: Vec2::new(100., 225.),
            text: "Fill End".to_string(),
            transform: Transform::from_translation(Vec3::new(-210., 230., 0.)),
            texture: square.clone(),
            image_mode: SpriteImageMode::Scale(ScalingMode::FillEnd),
        },
        Rect {
            size: Vec2::new(300., 100.),
            text: "Fill Start Horizontal".to_string(),
            transform: Transform::from_translation(Vec3::new(10., 290., 0.)),
            texture: square.clone(),
            image_mode: SpriteImageMode::Scale(ScalingMode::FillStart),
        },
        Rect {
            size: Vec2::new(300., 100.),
            text: "Fill End Horizontal".to_string(),
            transform: Transform::from_translation(Vec3::new(10., 155., 0.)),
            texture: square.clone(),
            image_mode: SpriteImageMode::Scale(ScalingMode::FillEnd),
        },
        Rect {
            size: Vec2::new(200., 200.),
            text: "Fill Center".to_string(),
            transform: Transform::from_translation(Vec3::new(280., 230., 0.)),
            texture: banner.clone(),
            image_mode: SpriteImageMode::Scale(ScalingMode::FillCenter),
        },
        Rect {
            size: Vec2::new(200., 100.),
            text: "Fill Center".to_string(),
            transform: Transform::from_translation(Vec3::new(500., 230., 0.)),
            texture: square.clone(),
            image_mode: SpriteImageMode::Scale(ScalingMode::FillCenter),
        },
        Rect {
            size: Vec2::new(100., 100.),
            text: "Stretched".to_string(),
            transform: Transform::from_translation(Vec3::new(-570., -40., 0.)),
            texture: banner.clone(),
            image_mode: SpriteImageMode::Auto,
        },
        Rect {
            size: Vec2::new(200., 200.),
            text: "Fit Center".to_string(),
            transform: Transform::from_translation(Vec3::new(-400., -40., 0.)),
            texture: banner.clone(),
            image_mode: SpriteImageMode::Scale(ScalingMode::FitCenter),
        },
        Rect {
            size: Vec2::new(200., 200.),
            text: "Fit Start".to_string(),
            transform: Transform::from_translation(Vec3::new(-180., -40., 0.)),
            texture: banner.clone(),
            image_mode: SpriteImageMode::Scale(ScalingMode::FitStart),
        },
        Rect {
            size: Vec2::new(200., 200.),
            text: "Fit End".to_string(),
            transform: Transform::from_translation(Vec3::new(40., -40., 0.)),
            texture: banner.clone(),
            image_mode: SpriteImageMode::Scale(ScalingMode::FitEnd),
        },
        Rect {
            size: Vec2::new(100., 200.),
            text: "Fit Center".to_string(),
            transform: Transform::from_translation(Vec3::new(210., -40., 0.)),
            texture: banner.clone(),
            image_mode: SpriteImageMode::Scale(ScalingMode::FitCenter),
        },
    ];

    for rect in rects {
        let mut cmd = commands.spawn((
            Sprite {
                image: rect.texture,
                custom_size: Some(rect.size),
                image_mode: rect.image_mode,
                ..default()
            },
            rect.transform,
        ));

        cmd.with_children(|builder| {
            builder.spawn((
                Text2d::new(rect.text),
                TextLayout::new_with_justify(Justify::Center),
                TextFont::from_font_size(15.),
                Transform::from_xyz(0., -0.5 * rect.size.y - 10., 0.),
                bevy::sprite::Anchor::TOP_CENTER,
            ));
        });
    }
}

fn setup_texture_atlas(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut texture_atlas_layouts: ResMut<Assets<TextureAtlasLayout>>,
) {
    let gabe = asset_server.load("textures/rpg/chars/gabe/gabe-idle-run.png");
    let animation_indices_gabe = AnimationIndices { first: 0, last: 6 };
    let gabe_atlas = TextureAtlas {
        layout: texture_atlas_layouts.add(TextureAtlasLayout::from_grid(
            UVec2::splat(24),
            7,
            1,
            None,
            None,
        )),
        index: animation_indices_gabe.first,
    };

    let sprite_sheets = [
        SpriteSheet {
            size: Vec2::new(120., 50.),
            text: "Stretched".to_string(),
            transform: Transform::from_translation(Vec3::new(-570., -200., 0.)),
            texture: gabe.clone(),
            image_mode: SpriteImageMode::Auto,
            atlas: gabe_atlas.clone(),
            indices: animation_indices_gabe.clone(),
            timer: AnimationTimer(Timer::from_seconds(0.1, TimerMode::Repeating)),
        },
        SpriteSheet {
            size: Vec2::new(120., 50.),
            text: "Fill Center".to_string(),
            transform: Transform::from_translation(Vec3::new(-570., -300., 0.)),
            texture: gabe.clone(),
            image_mode: SpriteImageMode::Scale(ScalingMode::FillCenter),
            atlas: gabe_atlas.clone(),
            indices: animation_indices_gabe.clone(),
            timer: AnimationTimer(Timer::from_seconds(0.1, TimerMode::Repeating)),
        },
        SpriteSheet {
            size: Vec2::new(120., 50.),
            text: "Fill Start".to_string(),
            transform: Transform::from_translation(Vec3::new(-430., -200., 0.)),
            texture: gabe.clone(),
            image_mode: SpriteImageMode::Scale(ScalingMode::FillStart),
            atlas: gabe_atlas.clone(),
            indices: animation_indices_gabe.clone(),
            timer: AnimationTimer(Timer::from_seconds(0.1, TimerMode::Repeating)),
        },
        SpriteSheet {
            size: Vec2::new(120., 50.),
            text: "Fill End".to_string(),
            transform: Transform::from_translation(Vec3::new(-430., -300., 0.)),
            texture: gabe.clone(),
            image_mode: SpriteImageMode::Scale(ScalingMode::FillEnd),
            atlas: gabe_atlas.clone(),
            indices: animation_indices_gabe.clone(),
            timer: AnimationTimer(Timer::from_seconds(0.1, TimerMode::Repeating)),
        },
        SpriteSheet {
            size: Vec2::new(50., 120.),
            text: "Fill Center".to_string(),
            transform: Transform::from_translation(Vec3::new(-300., -250., 0.)),
            texture: gabe.clone(),
            image_mode: SpriteImageMode::Scale(ScalingMode::FillCenter),
            atlas: gabe_atlas.clone(),
            indices: animation_indices_gabe.clone(),
            timer: AnimationTimer(Timer::from_seconds(0.1, TimerMode::Repeating)),
        },
        SpriteSheet {
            size: Vec2::new(50., 120.),
            text: "Fill Start".to_string(),
            transform: Transform::from_translation(Vec3::new(-190., -250., 0.)),
            texture: gabe.clone(),
            image_mode: SpriteImageMode::Scale(ScalingMode::FillStart),
            atlas: gabe_atlas.clone(),
            indices: animation_indices_gabe.clone(),
            timer: AnimationTimer(Timer::from_seconds(0.1, TimerMode::Repeating)),
        },
        SpriteSheet {
            size: Vec2::new(50., 120.),
            text: "Fill End".to_string(),
            transform: Transform::from_translation(Vec3::new(-90., -250., 0.)),
            texture: gabe.clone(),
            image_mode: SpriteImageMode::Scale(ScalingMode::FillEnd),
            atlas: gabe_atlas.clone(),
            indices: animation_indices_gabe.clone(),
            timer: AnimationTimer(Timer::from_seconds(0.1, TimerMode::Repeating)),
        },
        SpriteSheet {
            size: Vec2::new(120., 50.),
            text: "Fit Center".to_string(),
            transform: Transform::from_translation(Vec3::new(20., -200., 0.)),
            texture: gabe.clone(),
            image_mode: SpriteImageMode::Scale(ScalingMode::FitCenter),
            atlas: gabe_atlas.clone(),
            indices: animation_indices_gabe.clone(),
            timer: AnimationTimer(Timer::from_seconds(0.1, TimerMode::Repeating)),
        },
        SpriteSheet {
            size: Vec2::new(120., 50.),
            text: "Fit Start".to_string(),
            transform: Transform::from_translation(Vec3::new(20., -300., 0.)),
            texture: gabe.clone(),
            image_mode: SpriteImageMode::Scale(ScalingMode::FitStart),
            atlas: gabe_atlas.clone(),
            indices: animation_indices_gabe.clone(),
            timer: AnimationTimer(Timer::from_seconds(0.1, TimerMode::Repeating)),
        },
        SpriteSheet {
            size: Vec2::new(120., 50.),
            text: "Fit End".to_string(),
            transform: Transform::from_translation(Vec3::new(160., -200., 0.)),
            texture: gabe.clone(),
            image_mode: SpriteImageMode::Scale(ScalingMode::FitEnd),
            atlas: gabe_atlas.clone(),
            indices: animation_indices_gabe.clone(),
            timer: AnimationTimer(Timer::from_seconds(0.1, TimerMode::Repeating)),
        },
    ];

    for sprite_sheet in sprite_sheets {
        let mut cmd = commands.spawn((
            Sprite {
                image_mode: sprite_sheet.image_mode,
                custom_size: Some(sprite_sheet.size),
                ..Sprite::from_atlas_image(sprite_sheet.texture.clone(), sprite_sheet.atlas.clone())
            },
            sprite_sheet.indices,
            sprite_sheet.timer,
            sprite_sheet.transform,
        ));

        cmd.with_children(|builder| {
            builder.spawn((
                Text2d::new(sprite_sheet.text),
                TextLayout::new_with_justify(Justify::Center),
                TextFont::from_font_size(15.),
                Transform::from_xyz(0., -0.5 * sprite_sheet.size.y - 10., 0.),
                bevy::sprite::Anchor::TOP_CENTER,
            ));
        });
    }
}

struct Rect {
    size: Vec2,
    text: String,
    transform: Transform,
    texture: Handle<Image>,
    image_mode: SpriteImageMode,
}

struct SpriteSheet {
    size: Vec2,
    text: String,
    transform: Transform,
    texture: Handle<Image>,
    image_mode: SpriteImageMode,
    atlas: TextureAtlas,
    indices: AnimationIndices,
    timer: AnimationTimer,
}

#[derive(Component, Clone)]
struct AnimationIndices {
    first: usize,
    last: usize,
}

#[derive(Component, Deref, DerefMut)]
struct AnimationTimer(Timer);

fn animate_sprite(
    time: Res<Time>,
    mut query: Query<(&AnimationIndices, &mut AnimationTimer, &mut Sprite)>,
) {
    for (indices, mut timer, mut sprite) in &mut query {
        timer.tick(time.delta());

        if timer.just_finished()
            && let Some(atlas) = &mut sprite.texture_atlas
        {
            atlas.index = if atlas.index == indices.last {
                indices.first
            } else {
                atlas.index + 1
            };
        }
    }
}
