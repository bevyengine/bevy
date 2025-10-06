//! Stress test demonstrating gradient performance improvements.
//!
//! This example creates many UI nodes with gradients to measure the performance
//! impact of pre-converting colors to the target color space on the CPU.

use argh::FromArgs;
use bevy::{
    color::palettes::css::*,
    diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin},
    math::ops::sin,
    prelude::*,
    ui::{
        BackgroundGradient, ColorStop, Display, Gradient, InterpolationColorSpace, LinearGradient,
        RepeatedGridTrack,
    },
    window::{PresentMode, WindowResolution},
    winit::{UpdateMode, WinitSettings},
};

const COLS: usize = 30;

#[derive(FromArgs, Resource, Debug)]
/// Gradient stress test
struct Args {
    /// how many gradients per group (default: 900)
    #[argh(option, default = "900")]
    gradient_count: usize,

    /// whether to animate gradients by changing colors
    #[argh(switch)]
    animate: bool,

    /// use sRGB interpolation
    #[argh(switch)]
    srgb: bool,

    /// use HSL interpolation
    #[argh(switch)]
    hsl: bool,
}

fn main() {
    let args: Args = argh::from_env();
    let total_gradients = args.gradient_count;

    println!("Gradient stress test with {total_gradients} gradients");
    println!(
        "Color space: {}",
        if args.srgb {
            "sRGB"
        } else if args.hsl {
            "HSL"
        } else {
            "OkLab (default)"
        }
    );

    App::new()
        .add_plugins((
            LogDiagnosticsPlugin::default(),
            FrameTimeDiagnosticsPlugin::default(),
            DefaultPlugins.set(WindowPlugin {
                primary_window: Some(Window {
                    title: "Gradient Stress Test".to_string(),
                    resolution: WindowResolution::new(1920, 1080).with_scale_factor_override(1.0),
                    present_mode: PresentMode::AutoNoVsync,
                    ..default()
                }),
                ..default()
            }),
        ))
        .insert_resource(WinitSettings {
            focused_mode: UpdateMode::Continuous,
            unfocused_mode: UpdateMode::Continuous,
        })
        .insert_resource(args)
        .insert_resource(WinitSettings::continuous())
        .add_systems(Startup, setup)
        .add_systems(Update, animate_gradients)
        .run();
}

fn setup(mut commands: Commands, args: Res<Args>) {
    commands.spawn(Camera2d);

    let rows_to_spawn = args.gradient_count.div_ceil(COLS);

    // Create a grid of gradients
    commands
        .spawn(Node {
            width: percent(100),
            height: percent(100),
            display: Display::Grid,
            grid_template_columns: RepeatedGridTrack::flex(COLS as u16, 1.0),
            grid_template_rows: RepeatedGridTrack::flex(rows_to_spawn as u16, 1.0),
            ..default()
        })
        .with_children(|parent| {
            for i in 0..args.gradient_count {
                let angle = (i as f32 * 10.0) % 360.0;

                let mut gradient = LinearGradient::new(
                    angle,
                    vec![
                        ColorStop::new(RED, percent(0)),
                        ColorStop::new(BLUE, percent(100)),
                        ColorStop::new(GREEN, percent(20)),
                        ColorStop::new(YELLOW, percent(40)),
                        ColorStop::new(ORANGE, percent(60)),
                        ColorStop::new(LIME, percent(80)),
                        ColorStop::new(DARK_CYAN, percent(90)),
                    ],
                );

                gradient.color_space = if args.srgb {
                    InterpolationColorSpace::Srgba
                } else if args.hsl {
                    InterpolationColorSpace::Hsla
                } else {
                    InterpolationColorSpace::Oklaba
                };

                parent.spawn((
                    Node {
                        width: percent(100),
                        height: percent(100),
                        ..default()
                    },
                    BackgroundGradient(vec![Gradient::Linear(gradient)]),
                    GradientNode { index: i },
                ));
            }
        });
}

#[derive(Component)]
struct GradientNode {
    index: usize,
}

fn animate_gradients(
    mut gradients: Query<(&mut BackgroundGradient, &GradientNode)>,
    args: Res<Args>,
    time: Res<Time>,
) {
    if !args.animate {
        return;
    }

    let t = time.elapsed_secs();

    for (mut bg_gradient, node) in &mut gradients {
        let offset = node.index as f32 * 0.01;
        let hue_shift = sin(t + offset) * 0.5 + 0.5;

        if let Some(Gradient::Linear(gradient)) = bg_gradient.0.get_mut(0) {
            let color1 = Color::hsl(hue_shift * 360.0, 1.0, 0.5);
            let color2 = Color::hsl((hue_shift + 0.3) * 360.0 % 360.0, 1.0, 0.5);

            gradient.stops = vec![
                ColorStop::new(color1, percent(0)),
                ColorStop::new(color2, percent(100)),
                ColorStop::new(
                    Color::hsl((hue_shift + 0.1) * 360.0 % 360.0, 1.0, 0.5),
                    percent(20),
                ),
                ColorStop::new(
                    Color::hsl((hue_shift + 0.15) * 360.0 % 360.0, 1.0, 0.5),
                    percent(40),
                ),
                ColorStop::new(
                    Color::hsl((hue_shift + 0.2) * 360.0 % 360.0, 1.0, 0.5),
                    percent(60),
                ),
                ColorStop::new(
                    Color::hsl((hue_shift + 0.25) * 360.0 % 360.0, 1.0, 0.5),
                    percent(80),
                ),
                ColorStop::new(
                    Color::hsl((hue_shift + 0.28) * 360.0 % 360.0, 1.0, 0.5),
                    percent(90),
                ),
            ];
        }
    }
}
