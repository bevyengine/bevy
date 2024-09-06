//! This example illustrates how to how to flip and tile images with 9-slicing in the UI.

use bevy::{
    prelude::*,
    render::texture::{ImageLoaderSettings, ImageSampler},
    winit::WinitSettings,
};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .insert_resource(UiScale(2.))
        // Only run the app when there is user input. This will significantly reduce CPU/GPU use for UI-only apps.
        .insert_resource(WinitSettings::desktop_app())
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
        border: BorderRect::square(16.),
        // With `SliceScaleMode::Tile` the side and center slices are tiled to to fill the side and center sections of the target.
        // And with a `stretch_value` of `1.` the tiles will have the same size as the corresponding slices in the source image.
        center_scale_mode: SliceScaleMode::Tile { stretch_value: 1. },
        sides_scale_mode: SliceScaleMode::Tile { stretch_value: 1. },
        ..default()
    };

    // ui camera
    commands.spawn(Camera2dBundle::default());

    commands
        .spawn(NodeBundle {
            style: Style {
                width: Val::Percent(100.),
                height: Val::Percent(100.),
                justify_content: JustifyContent::Center,
                align_content: AlignContent::Center,
                flex_wrap: FlexWrap::Wrap,
                column_gap: Val::Px(10.),
                row_gap: Val::Px(10.),
                ..default()
            },
            ..default()
        })
        .with_children(|parent| {
            for ([width, height], flip_x, flip_y) in [
                ([160., 160.], false, false),
                ([320., 160.], false, true),
                ([320., 160.], true, false),
                ([160., 160.], true, true),
            ] {
                parent.spawn((
                    NodeBundle {
                        style: Style {
                            width: Val::Px(width),
                            height: Val::Px(height),
                            ..default()
                        },
                        ..Default::default()
                    },
                    UiImage {
                        texture: image.clone(),
                        flip_x,
                        flip_y,
                        ..Default::default()
                    },
                    ImageScaleMode::Sliced(slicer.clone()),
                ));
            }
        });
}
