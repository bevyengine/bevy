//! Shows how to use sprite scaling modes to fill and fit textures into the sprite.

use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .run();
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn(Camera2d);
    let font = asset_server.load("fonts/FiraSans-Bold.ttf");
    let style = TextFont {
        font: font.clone(),
        ..default()
    };

    let square = asset_server.load("textures/slice_square_2.png");
    let banner = asset_server.load("branding/banner.png");

    let rects = vec![
        Rect {
            size: Vec2::new(100., 300.),
            text: "Stretched".to_string(),
            transform: Transform::from_translation(Vec3::new(-550.0, 200.0, 0.0)),
            texture: square.clone(),
            image_mode: SpriteImageMode::Auto,
        },
        Rect {
            size: Vec2::new(100., 300.),
            text: "Fill Center".to_string(),
            transform: Transform::from_translation(Vec3::new(-400.0, 200.0, 0.0)),
            texture: square.clone(),
            image_mode: SpriteImageMode::ScaleMode(TextureScale::FillCenter),
        },
        Rect {
            size: Vec2::new(100., 300.),
            text: "Fill Start".to_string(),
            transform: Transform::from_translation(Vec3::new(-250.0, 200.0, 0.0)),
            texture: square.clone(),
            image_mode: SpriteImageMode::ScaleMode(TextureScale::FillStart),
        },
        Rect {
            size: Vec2::new(100., 300.),
            text: "Fill End".to_string(),
            transform: Transform::from_translation(Vec3::new(-100.0, 200.0, 0.0)),
            texture: square.clone(),
            image_mode: SpriteImageMode::ScaleMode(TextureScale::FillEnd),
        },
        Rect {
            size: Vec2::new(300., 100.),
            text: "Fill Start Horizontal".to_string(),
            transform: Transform::from_translation(Vec3::new(150.0, 300.0, 0.0)),
            texture: square.clone(),
            image_mode: SpriteImageMode::ScaleMode(TextureScale::FillStart),
        },
        Rect {
            size: Vec2::new(300., 100.),
            text: "Fill End Horizontal".to_string(),
            transform: Transform::from_translation(Vec3::new(150.0, 100.0, 0.0)),
            texture: square.clone(),
            image_mode: SpriteImageMode::ScaleMode(TextureScale::FillEnd),
        },
        Rect {
            size: Vec2::new(200., 200.),
            text: "Fill Center".to_string(),
            transform: Transform::from_translation(Vec3::new(450.0, 200.0, 0.0)),
            texture: banner.clone(),
            image_mode: SpriteImageMode::ScaleMode(TextureScale::FillCenter),
        },
        Rect {
            size: Vec2::new(100., 100.),
            text: "Stretched".to_string(),
            transform: Transform::from_translation(Vec3::new(-550.0, -200.0, 0.0)),
            texture: banner.clone(),
            image_mode: SpriteImageMode::Auto,
        },
        Rect {
            size: Vec2::new(200., 200.),
            text: "Fit Center".to_string(),
            transform: Transform::from_translation(Vec3::new(-350.0, -200.0, 0.0)),
            texture: banner.clone(),
            image_mode: SpriteImageMode::ScaleMode(TextureScale::FitCenter),
        },
        Rect {
            size: Vec2::new(200., 200.),
            text: "Fit Start".to_string(),
            transform: Transform::from_translation(Vec3::new(-100.0, -200.0, 0.0)),
            texture: banner.clone(),
            image_mode: SpriteImageMode::ScaleMode(TextureScale::FitStart),
        },
        Rect {
            size: Vec2::new(200., 200.),
            text: "Fit End".to_string(),
            transform: Transform::from_translation(Vec3::new(150.0, -200.0, 0.0)),
            texture: banner.clone(),
            image_mode: SpriteImageMode::ScaleMode(TextureScale::FitEnd),
        },
        Rect {
            size: Vec2::new(100., 200.),
            text: "Fit Center".to_string(),
            transform: Transform::from_translation(Vec3::new(350.0, -200.0, 0.0)),
            texture: banner.clone(),
            image_mode: SpriteImageMode::ScaleMode(TextureScale::FitCenter),
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
                style.clone(),
                TextLayout::new_with_justify(JustifyText::Center),
                Transform::from_xyz(0., -0.5 * rect.size.y - 10., 0.0),
                bevy::sprite::Anchor::TopCenter,
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
