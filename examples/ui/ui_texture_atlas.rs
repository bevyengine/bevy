//! This example illustrates how to use `TextureAtlases` within ui

use bevy::{color::palettes::css::*, prelude::*, winit::WinitSettings};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(
            // This sets image filtering to nearest
            // This is done to prevent textures with low resolution (e.g. pixel art) from being blurred
            // by linear filtering.
            ImagePlugin::default_nearest(),
        ))
        // Only run the app when there is user input. This will significantly reduce CPU/GPU use.
        .insert_resource(WinitSettings::desktop_app())
        .add_systems(Startup, setup)
        .add_systems(Update, increment_atlas_index)
        .run();
}

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut texture_atlases: ResMut<Assets<TextureAtlasLayout>>,
) {
    // Camera
    commands.spawn(Camera2d);

    let text_font = TextFont::default();

    let texture_handle = asset_server.load("textures/rpg/chars/gabe/gabe-idle-run.png");
    let texture_atlas = TextureAtlasLayout::from_grid(UVec2::splat(24), 7, 1, None, None);
    let texture_atlas_handle = texture_atlases.add(texture_atlas);

    // root node
    commands
        .spawn(Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            flex_direction: FlexDirection::Column,
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            row_gap: Val::Px(text_font.font_size * 2.),
            ..default()
        })
        .with_children(|parent| {
            parent.spawn((
                ImageNode::from_atlas_image(
                    texture_handle,
                    TextureAtlas::from(texture_atlas_handle),
                ),
                Node {
                    width: Val::Px(256.),
                    height: Val::Px(256.),
                    ..default()
                },
                BackgroundColor(ANTIQUE_WHITE.into()),
                Outline::new(Val::Px(8.0), Val::ZERO, CRIMSON.into()),
            ));
            parent
                .spawn((Text::new("press "), text_font.clone()))
                .with_child((
                    TextSpan::new("space"),
                    TextColor(YELLOW.into()),
                    text_font.clone(),
                ))
                .with_child((TextSpan::new(" to advance frames"), text_font));
        });
}

fn increment_atlas_index(
    mut image_nodes: Query<&mut ImageNode>,
    keyboard: Res<ButtonInput<KeyCode>>,
) {
    if keyboard.just_pressed(KeyCode::Space) {
        for mut image_node in &mut image_nodes {
            if let Some(atlas) = &mut image_node.texture_atlas {
                atlas.index = (atlas.index + 1) % 6;
            }
        }
    }
}
