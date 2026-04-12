//! This example illustrates how to how to flip and tile images with 9-slicing in the UI.

use bevy::{
    image::{ImageLoaderSettings, ImageSampler},
    prelude::*,
    ui::widget::NodeImageMode,
};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .insert_resource(UiScale(2.))
        .add_systems(Startup, setup)
        .run();
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    let image = asset_server.load_with_settings(
        "textures/fantasy_ui_borders/numbered_slices.png",
        |settings: &mut ImageLoaderSettings| {
            // Need to use nearest filtering to avoid bleeding between the slices with tiling
            settings.sampler = ImageSampler::nearest();
        },
    );

    let slicer = TextureSlicer {
        // `numbered_slices.png` is 48 pixels square. `BorderRect::square(16.)` insets the slicing line from each edge by 16 pixels, resulting in nine slices that are each 16 pixels square.
        border: BorderRect::all(16.),
        // With `SliceScaleMode::Tile` the side and center slices are tiled to fill the side and center sections of the target.
        // And with a `stretch_value` of `1.` the tiles will have the same size as the corresponding slices in the source image.
        center_scale_mode: SliceScaleMode::Tile { stretch_value: 1. },
        sides_scale_mode: SliceScaleMode::Tile { stretch_value: 1. },
        ..default()
    };

    // ui camera
    commands.spawn(Camera2d);

    commands
        .spawn(Node {
            width: percent(100),
            height: percent(100),
            justify_content: JustifyContent::Center,
            align_content: AlignContent::Center,
            flex_wrap: FlexWrap::Wrap,
            column_gap: px(10),
            row_gap: px(10),
            ..default()
        })
        .with_children(|parent| {
            for [columns, rows] in [[3, 3], [4, 4], [5, 4], [4, 5], [5, 5]] {
                for (flip_x, flip_y) in [(false, false), (false, true), (true, false), (true, true)]
                {
                    parent.spawn((
                        ImageNode {
                            image: image.clone(),
                            flip_x,
                            flip_y,
                            image_mode: NodeImageMode::Sliced(slicer.clone()),
                            ..default()
                        },
                        Node {
                            width: px(16 * columns),
                            height: px(16 * rows),
                            ..default()
                        },
                    ));
                }
            }
        });
}
