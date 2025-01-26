//! This example shows how to create a node with a shadow

use argh::FromArgs;
use bevy::color::palettes::css::BLUE;
use bevy::color::palettes::css::DEEP_SKY_BLUE;
use bevy::color::palettes::css::GREEN;
use bevy::color::palettes::css::LIGHT_SKY_BLUE;
use bevy::color::palettes::css::RED;
use bevy::color::palettes::css::YELLOW;
use bevy::prelude::*;
use bevy::winit::WinitSettings;

#[derive(FromArgs, Resource)]
/// `box_shadow` example
struct Args {
    /// number of samples
    #[argh(option, default = "4")]
    samples: u32,
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        // Only run the app when there is user input. This will significantly reduce CPU/GPU use.
        .insert_resource(WinitSettings::desktop_app())
        .add_systems(Startup, setup)
        .run();
}

fn setup(mut commands: Commands) {
    // `from_env` panics on the web
    #[cfg(not(target_arch = "wasm32"))]
    let args: Args = argh::from_env();
    #[cfg(target_arch = "wasm32")]
    let args = Args::from_args(&[], &[]).unwrap();

    // ui camera
    commands.spawn((Camera2d, BoxShadowSamples(args.samples)));

    commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                padding: UiRect::all(Val::Px(30.)),
                column_gap: Val::Px(30.),
                flex_wrap: FlexWrap::Wrap,
                ..default()
            },
            BackgroundColor(DEEP_SKY_BLUE.into()),
        ))
        .with_children(|commands| {
            let example_nodes = [
                (
                    Vec2::splat(50.),
                    Vec2::ZERO,
                    10.,
                    0.,
                    BorderRadius::bottom_right(Val::Px(10.)),
                ),
                (Vec2::new(50., 25.), Vec2::ZERO, 10., 0., BorderRadius::ZERO),
                (Vec2::splat(50.), Vec2::ZERO, 10., 0., BorderRadius::MAX),
                (Vec2::new(100., 25.), Vec2::ZERO, 10., 0., BorderRadius::MAX),
                (
                    Vec2::splat(50.),
                    Vec2::ZERO,
                    10.,
                    0.,
                    BorderRadius::bottom_right(Val::Px(10.)),
                ),
                (Vec2::new(50., 25.), Vec2::ZERO, 0., 10., BorderRadius::ZERO),
                (
                    Vec2::splat(50.),
                    Vec2::ZERO,
                    0.,
                    10.,
                    BorderRadius::bottom_right(Val::Px(10.)),
                ),
                (Vec2::new(100., 25.), Vec2::ZERO, 0., 10., BorderRadius::MAX),
                (
                    Vec2::splat(50.),
                    Vec2::splat(25.),
                    0.,
                    0.,
                    BorderRadius::ZERO,
                ),
                (
                    Vec2::new(50., 25.),
                    Vec2::splat(25.),
                    0.,
                    0.,
                    BorderRadius::ZERO,
                ),
                (
                    Vec2::splat(50.),
                    Vec2::splat(10.),
                    0.,
                    0.,
                    BorderRadius::bottom_right(Val::Px(10.)),
                ),
                (
                    Vec2::splat(50.),
                    Vec2::splat(25.),
                    0.,
                    10.,
                    BorderRadius::ZERO,
                ),
                (
                    Vec2::new(50., 25.),
                    Vec2::splat(25.),
                    0.,
                    10.,
                    BorderRadius::ZERO,
                ),
                (
                    Vec2::splat(50.),
                    Vec2::splat(10.),
                    0.,
                    10.,
                    BorderRadius::bottom_right(Val::Px(10.)),
                ),
                (
                    Vec2::splat(50.),
                    Vec2::splat(10.),
                    0.,
                    3.,
                    BorderRadius::ZERO,
                ),
                (
                    Vec2::new(50., 25.),
                    Vec2::splat(10.),
                    0.,
                    3.,
                    BorderRadius::ZERO,
                ),
                (
                    Vec2::splat(50.),
                    Vec2::splat(10.),
                    0.,
                    3.,
                    BorderRadius::bottom_right(Val::Px(10.)),
                ),
                (
                    Vec2::splat(50.),
                    Vec2::splat(10.),
                    0.,
                    3.,
                    BorderRadius::all(Val::Px(20.)),
                ),
                (
                    Vec2::new(50., 25.),
                    Vec2::splat(10.),
                    0.,
                    3.,
                    BorderRadius::all(Val::Px(20.)),
                ),
                (
                    Vec2::new(25., 50.),
                    Vec2::splat(10.),
                    0.,
                    3.,
                    BorderRadius::MAX,
                ),
                (
                    Vec2::splat(50.),
                    Vec2::splat(10.),
                    0.,
                    10.,
                    BorderRadius::all(Val::Px(20.)),
                ),
                (
                    Vec2::new(50., 25.),
                    Vec2::splat(10.),
                    0.,
                    10.,
                    BorderRadius::all(Val::Px(20.)),
                ),
                (
                    Vec2::new(25., 50.),
                    Vec2::splat(10.),
                    0.,
                    10.,
                    BorderRadius::MAX,
                ),
            ];

            for (size, offset, spread, blur, border_radius) in example_nodes {
                commands.spawn(box_shadow_node_bundle(
                    size,
                    offset,
                    spread,
                    blur,
                    border_radius,
                ));
            }

            // Demonstrate multiple shadows on one node
            commands.spawn((
                Node {
                    width: Val::Px(40.),
                    height: Val::Px(40.),
                    border: UiRect::all(Val::Px(4.)),
                    ..default()
                },
                BorderColor(LIGHT_SKY_BLUE.into()),
                BorderRadius::all(Val::Px(20.)),
                BackgroundColor(DEEP_SKY_BLUE.into()),
                BoxShadow(vec![
                    ShadowStyle {
                        color: RED.with_alpha(0.7).into(),
                        x_offset: Val::Px(-20.),
                        y_offset: Val::Px(-5.),
                        spread_radius: Val::Percent(10.),
                        blur_radius: Val::Px(3.),
                    },
                    ShadowStyle {
                        color: BLUE.with_alpha(0.7).into(),
                        x_offset: Val::Px(-5.),
                        y_offset: Val::Px(-20.),
                        spread_radius: Val::Percent(10.),
                        blur_radius: Val::Px(3.),
                    },
                    ShadowStyle {
                        color: YELLOW.with_alpha(0.7).into(),
                        x_offset: Val::Px(20.),
                        y_offset: Val::Px(5.),
                        spread_radius: Val::Percent(10.),
                        blur_radius: Val::Px(3.),
                    },
                    ShadowStyle {
                        color: GREEN.with_alpha(0.7).into(),
                        x_offset: Val::Px(5.),
                        y_offset: Val::Px(20.),
                        spread_radius: Val::Percent(10.),
                        blur_radius: Val::Px(3.),
                    },
                ]),
            ));
        });
}

fn box_shadow_node_bundle(
    size: Vec2,
    offset: Vec2,
    spread: f32,
    blur: f32,
    border_radius: BorderRadius,
) -> impl Bundle {
    (
        Node {
            width: Val::Px(size.x),
            height: Val::Px(size.y),
            border: UiRect::all(Val::Px(4.)),
            ..default()
        },
        BorderColor(LIGHT_SKY_BLUE.into()),
        border_radius,
        BackgroundColor(DEEP_SKY_BLUE.into()),
        BoxShadow::new(
            Color::BLACK.with_alpha(0.8),
            Val::Percent(offset.x),
            Val::Percent(offset.y),
            Val::Percent(spread),
            Val::Px(blur),
        ),
    )
}
