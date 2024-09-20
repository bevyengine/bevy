//! This example shows how to create a node with a shadow

use argh::FromArgs;
use bevy::color::palettes::css::LIGHT_CORAL;
use bevy::prelude::*;
use bevy::ui::box_shadow::BoxShadowSamples;
use bevy::winit::WinitSettings;

#[derive(FromArgs, Resource)]
/// `box_shadow` example
struct Args {
    /// number of samples
    #[argh(option, default = "4")]
    samples: u32,
}

fn main() {
    // `from_env` panics on the web
    #[cfg(not(target_arch = "wasm32"))]
    let args: Args = argh::from_env();
    #[cfg(target_arch = "wasm32")]
    let args = Args::from_args(&[], &[]).unwrap();

    App::new()
        .add_plugins(DefaultPlugins)
        .insert_resource(BoxShadowSamples(args.samples))
        // Only run the app when there is user input. This will significantly reduce CPU/GPU use.
        .insert_resource(WinitSettings::desktop_app())
        .add_systems(Startup, setup)
        .run();
}

fn setup(mut commands: Commands) {
    // ui camera
    commands.spawn(Camera2dBundle::default());

    commands
        .spawn(NodeBundle {
            style: Style {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                row_gap: Val::Px(75.),
                column_gap: Val::Px(75.),
                flex_wrap: FlexWrap::Wrap,
                ..default()
            },
            background_color: BackgroundColor(LIGHT_CORAL.into()),
            ..Default::default()
        })
        .with_children(|commands| {
            let example_nodes = [
                (
                    Vec2::splat(100.),
                    Color::WHITE,
                    Color::BLACK,
                    Vec2::splat(50.),
                    Vec2::ZERO,
                    0.,
                    BorderRadius::ZERO,
                ),
                (
                    Vec2::new(100., 50.),
                    Color::WHITE,
                    Color::BLACK,
                    Vec2::splat(50.),
                    Vec2::ZERO,
                    0.,
                    BorderRadius::ZERO,
                ),
                (
                    Vec2::new(100., 100.),
                    Color::WHITE,
                    Color::BLACK,
                    Vec2::splat(25.),
                    Vec2::ZERO,
                    0.,
                    BorderRadius::MAX,
                ),
                (
                    Vec2::splat(100.),
                    Color::WHITE,
                    Color::BLACK,
                    Vec2::splat(50.),
                    Vec2::ZERO,
                    10.,
                    BorderRadius::ZERO,
                ),
                (
                    Vec2::new(100., 50.),
                    Color::WHITE,
                    Color::BLACK,
                    Vec2::splat(50.),
                    Vec2::ZERO,
                    10.,
                    BorderRadius::ZERO,
                ),
                (
                    Vec2::new(100., 100.),
                    Color::WHITE,
                    Color::BLACK,
                    Vec2::splat(25.),
                    Vec2::ZERO,
                    10.,
                    BorderRadius::MAX,
                ),
                (
                    Vec2::splat(100.),
                    Color::WHITE,
                    Color::BLACK,
                    Vec2::splat(25.),
                    Vec2::ZERO,
                    3.,
                    BorderRadius::ZERO,
                ),
                (
                    Vec2::new(100., 50.),
                    Color::WHITE,
                    Color::BLACK,
                    Vec2::splat(25.),
                    Vec2::ZERO,
                    3.,
                    BorderRadius::ZERO,
                ),
                (
                    Vec2::new(100., 100.),
                    Color::WHITE,
                    Color::BLACK,
                    Vec2::splat(25.),
                    Vec2::ZERO,
                    3.,
                    BorderRadius::MAX,
                ),
                (
                    Vec2::splat(100.),
                    Color::WHITE,
                    Color::BLACK,
                    Vec2::splat(25.),
                    Vec2::ZERO,
                    3.,
                    BorderRadius::all(Val::Px(20.)),
                ),
                (
                    Vec2::new(100., 50.),
                    Color::WHITE,
                    Color::BLACK,
                    Vec2::splat(25.),
                    Vec2::ZERO,
                    3.,
                    BorderRadius::all(Val::Px(20.)),
                ),
                (
                    Vec2::new(50., 100.),
                    Color::WHITE,
                    Color::BLACK,
                    Vec2::splat(25.),
                    Vec2::ZERO,
                    3.,
                    BorderRadius::MAX,
                ),
                (
                    Vec2::splat(100.),
                    Color::WHITE,
                    Color::BLACK,
                    Vec2::splat(25.),
                    Vec2::ZERO,
                    25.,
                    BorderRadius::all(Val::Px(20.)),
                ),
                (
                    Vec2::new(100., 50.),
                    Color::WHITE,
                    Color::BLACK,
                    Vec2::splat(25.),
                    Vec2::ZERO,
                    25.,
                    BorderRadius::all(Val::Px(20.)),
                ),
                (
                    Vec2::new(50., 100.),
                    Color::WHITE,
                    Color::BLACK,
                    Vec2::splat(25.),
                    Vec2::ZERO,
                    25.,
                    BorderRadius::MAX,
                ),
            ];

            for (size, color, shadow_color, offset, spread, blur, border_radius) in example_nodes {
                commands.spawn(box_shadow_node_bundle(
                    size,
                    color,
                    shadow_color,
                    offset,
                    spread,
                    blur,
                    border_radius,
                ));
            }
        });
}

fn box_shadow_node_bundle(
    size: Vec2,
    color: Color,
    shadow_color: Color,
    offset: Vec2,
    spread: Vec2,
    blur: f32,
    border_radius: BorderRadius,
) -> impl Bundle {
    (
        NodeBundle {
            style: Style {
                width: Val::Px(size.x),
                height: Val::Px(size.y),
                ..default()
            },
            border_radius,
            background_color: BackgroundColor(color),
            ..Default::default()
        },
        BoxShadow {
            color: shadow_color,
            x_offset: Val::Percent(offset.x),
            y_offset: Val::Percent(offset.y),
            x_spread: Val::Percent(spread.x),
            y_spread: Val::Percent(spread.y),
            blur_radius: Val::Px(blur),
        },
    )
}
