//! Renders a scrolling background behind an animated sprite at a high zoom
//! level, to test consistency and smoothness of performance.
//!
//! To measure performance realistically, be sure to run this in release mode.
//! `cargo run --example time_smoothness --release`
//!
//! By default, this example scrolls the background at 120 pixels per second,
//! and always moves in whole-pixel increments (since limiting movement to whole
//! pixels at high zoom seems to make it easier to perceive any problems with
//! frame consistency). There are several keyboard controls for changing the
//! example's behavior at runtime:
//!
//! - P: Cycle the `PresentMode` between `Fifo`, `Mailbox`, and `Immediate`.
//! - W: Cycle the `WindowMode` between `Windowed`, `BorderlessFullscreen`, and
//!   `Fullscreen`.
//! - T: Cycle the delta time between normal (measured from the `Time` resource)
//!   and fixed-interval. This can be useful when trying to distinguish slow
//!   frametimes from inaccurate time measurement. Fixed-interval delta time is
//!   hardcoded to 1/60th of a second per frame, which will look highly wacky
//!   unless you're using Fifo mode on a 60hz display.
//! - M: Cycle the scrolling motion style between whole-pixel and sub-pixel
//!   Transform increments.
//!
//! A number of factors contribute to scrolling without perceptible
//! hiccups/stutters/jank, including the accuracy of the delta time measurement,
//! the ability to consistently present frames to the GPU at the expected pace,
//! etc. This example doesn't isolate all of those factors, but it can help
//! identify when a problem exists and provide a starting point for further
//! investigation.
use bevy::{
    diagnostic::{Diagnostic, DiagnosticId, Diagnostics},
    prelude::*,
    render::texture::ImageSettings,
    window::{PresentMode, WindowMode},
};

// BG_WIDTH is smaller than the image's actual pixel width, because we want the
// empty space on the background tiles to overlap a bit. That way there's always
// a "landmark" on screen at the default window size.
const BG_WIDTH: f32 = 755.0;
const BG_HEIGHT: f32 = 363.0;
const BG_TILES: usize = 3;
const BG_SPEED: f32 = 120.0;

const CUSTOM_FRAME_TIME: DiagnosticId =
    DiagnosticId::from_u128(76860576947891895965111337840552081898);
const MAX_FRAME_HISTORY: usize = 800;
const FRAME_ANALYSIS_INTERVAL_SECONDS: f32 = 10.0;

fn main() {
    App::new()
        .insert_resource(WindowDescriptor {
            present_mode: PresentMode::Fifo,
            mode: WindowMode::Windowed,
            ..Default::default()
        })
        // Prevents blurry sprites
        .insert_resource(ImageSettings::default_nearest())
        .add_plugins(DefaultPlugins)
        // Adds frame time diagnostics
        .add_startup_system(setup_diagnostics)
        .add_system(update_diagnostics)
        .add_system(log_diagnostics)
        .insert_resource(FrameAnalysisTimer(Timer::from_seconds(
            FRAME_ANALYSIS_INTERVAL_SECONDS,
            true,
        )))
        // Main app setup
        .add_startup_system(setup)
        .insert_resource(MoveRemainder(Vec2::ZERO))
        .insert_resource(TimeStyle::Normal)
        .insert_resource(MoveStyle::WholePixel)
        .add_system(change_settings)
        .add_system(animate_runner)
        .add_system(scroll_background)
        .run();
}

// Create a custom frame time diagnostic. We need this because
// FrameTimeDiagnosticsPlugin only keeps 20 frames, which is too narrow a view
// to be useful when hunting irregular blips.
fn setup_diagnostics(mut diagnostics: ResMut<Diagnostics>) {
    diagnostics
        .add(Diagnostic::new(CUSTOM_FRAME_TIME, "frame_time", MAX_FRAME_HISTORY).with_suffix("s"));
}

// Update our custom frame time diagnostic with the delta time in milliseconds.
fn update_diagnostics(mut diagnostics: ResMut<Diagnostics>, time: Res<Time>) {
    diagnostics.add_measurement(CUSTOM_FRAME_TIME, || time.delta_seconds_f64() * 1000.0);
}

// Periodically analyze recent frame times and print a summary.
fn log_diagnostics(
    mut timer: ResMut<FrameAnalysisTimer>,
    diagnostics: Res<Diagnostics>,
    time: Res<Time>,
) {
    timer.0.tick(time.delta());
    if timer.0.finished() {
        let frame_times = diagnostics.get(CUSTOM_FRAME_TIME).unwrap();
        if let Some(average) = frame_times.average() {
            if let Some(std_dev) = std_deviation(frame_times) {
                let mut sorted_times: Vec<f64> = frame_times.values().copied().collect();
                sorted_times.sort_unstable_by(|a, b| a.partial_cmp(b).unwrap());

                let count = sorted_times.len();

                // Indexes corresponding to percentile ranks:
                let p95 = (0.95 * count as f32).round() as usize - 1;
                let p99 = (0.99 * count as f32).round() as usize - 1;
                let p99_5 = (0.995 * count as f32).round() as usize - 1;

                let min = sorted_times.first().unwrap();
                let max = sorted_times.last().unwrap();

                info!("-------------------------");
                info!("Average frame time: {:.6} ms", average);
                info!("Standard deviation: {:.6} ms", std_dev);
                info!("Shortest frame:     {:.6} ms", min);
                info!("95th percentile:    {:.6} ms", sorted_times[p95]);
                info!("99th percentile:    {:.6} ms", sorted_times[p99]);
                info!("99.5th percentile:  {:.6} ms", sorted_times[p99_5]);
                info!("Longest frame:      {:.6} ms", max);
                info!("-------------------------");
            }
        }
    }
}

fn std_deviation(diagnostic: &Diagnostic) -> Option<f64> {
    if let Some(average) = diagnostic.average() {
        let variance = diagnostic
            .values()
            .map(|val| {
                let diff = average - *val;
                diff * diff
            })
            .sum::<f64>()
            / diagnostic.history_len() as f64;
        Some(variance.sqrt())
    } else {
        None
    }
}

// Set up entities and assets.
fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut texture_atlases: ResMut<Assets<TextureAtlas>>,
) {
    // Locate the center(ish) of the background conveyer belt, so we can
    // position the player and camera there.
    let bg_center = Vec2::new(BG_WIDTH * BG_TILES as f32 / 2.0, BG_HEIGHT / 2.0).round();

    // Set up camera.
    let mut camera_bundle = Camera2dBundle::default();
    camera_bundle.projection.scale = 1.0 / 4.0;
    camera_bundle.transform.translation += bg_center.extend(0.0);
    commands.spawn_bundle(camera_bundle);

    // Set up animated player sprite.
    let runner_texture: Handle<Image> =
        asset_server.load("textures/rpg/chars/mani/mani-idle-run.png");
    let mut runner_atlas = TextureAtlas::from_grid(runner_texture, Vec2::new(24.0, 24.0), 7, 1);
    // Drop the first (idle) frame so we just have the run frames.
    runner_atlas.textures = runner_atlas.textures[1..].into();
    let runner_handle = texture_atlases.add(runner_atlas);
    // Offset by half our size to find where we should place our bottom left corner.
    let runner_location = bg_center - Vec2::new(12.0, 12.0);
    commands
        .spawn_bundle(SpriteSheetBundle {
            texture_atlas: runner_handle,
            sprite: TextureAtlasSprite {
                anchor: bevy::sprite::Anchor::BottomLeft,
                ..Default::default()
            },
            transform: Transform::from_translation(runner_location.extend(3.0)),
            ..Default::default()
        })
        .insert(Player)
        .insert(AnimationTimer {
            timer: Timer::from_seconds(0.1, true),
        });

    // Set up scrolling background, using a conveyor belt of three long sprites.
    let background_texture: Handle<Image> = asset_server.load("branding/banner.png");
    for i in 0..BG_TILES {
        commands
            .spawn_bundle(SpriteBundle {
                sprite: Sprite {
                    anchor: bevy::sprite::Anchor::BottomLeft,
                    ..Default::default()
                },
                transform: Transform::from_translation(Vec3::new(
                    i as f32 * BG_WIDTH + 1.0,
                    0.0,
                    0.0,
                )),
                texture: background_texture.clone(),
                ..Default::default()
            })
            .insert(Background);
    }
}

// Change settings on demand, to display different behaviors without recompiling.
fn change_settings(
    input: Res<Input<KeyCode>>,
    mut windows: ResMut<Windows>,
    mut time_style: ResMut<TimeStyle>,
    mut move_style: ResMut<MoveStyle>,
    mut background_query: Query<&mut Transform, With<Background>>,
) {
    let window = windows.primary_mut();
    if input.just_pressed(KeyCode::P) {
        // P: cycle PresentMode.
        let next_present_mode = match window.present_mode() {
            PresentMode::Fifo => PresentMode::Mailbox,
            PresentMode::Mailbox => PresentMode::Immediate,
            PresentMode::Immediate => PresentMode::Fifo,
        };
        info!("Switching present mode to {:?}", next_present_mode);
        window.set_present_mode(next_present_mode);
    } else if input.just_pressed(KeyCode::W) {
        // W: cycle WindowMode.
        let next_window_mode = match window.mode() {
            WindowMode::Windowed => WindowMode::BorderlessFullscreen,
            WindowMode::BorderlessFullscreen => WindowMode::Fullscreen,
            _ => WindowMode::Windowed,
        };
        info!("Switching window mode to {:?}", next_window_mode);
        window.set_mode(next_window_mode);
        if next_window_mode == WindowMode::Windowed {
            window.set_resolution(1280.0, 720.0);
        }
    } else if input.just_pressed(KeyCode::T) {
        // T: cycle delta time style
        let next_time_style = match *time_style {
            TimeStyle::Normal => TimeStyle::Fixed,
            TimeStyle::Fixed => TimeStyle::Normal,
        };
        info!("Switching time style to {:?}", next_time_style);
        *time_style = next_time_style;
    } else if input.just_pressed(KeyCode::M) {
        // M: cycle scroll motion style
        let next_move_style = match *move_style {
            MoveStyle::WholePixel => MoveStyle::SubPixel,
            MoveStyle::SubPixel => {
                // Re-lock the background positions to whole-pixel boundaries
                for mut transform in background_query.iter_mut() {
                    transform.translation = transform.translation.round();
                }
                MoveStyle::WholePixel
            }
        };
        info!("Switching scroll style to {:?}", next_move_style);
        *move_style = next_move_style;
    }
}

// Increase+loop the player sprite's frame index, per its animation timer.
fn animate_runner(
    time: Res<Time>,
    texture_atlases: Res<Assets<TextureAtlas>>,
    mut query: Query<
        (
            &mut AnimationTimer,
            &mut TextureAtlasSprite,
            &Handle<TextureAtlas>,
        ),
        With<Player>,
    >,
) {
    for (mut sprite_timer, mut sprite, texture_atlas_handle) in query.iter_mut() {
        sprite_timer.timer.tick(time.delta());
        if sprite_timer.timer.finished() {
            let texture_atlas = texture_atlases.get(texture_atlas_handle).unwrap();
            sprite.index = (sprite.index + 1) % texture_atlas.textures.len();
        }
    }
}

// Scroll the background in pixel-perfect increments, re-using the sprites as
// they scroll off the left side.
fn scroll_background(
    time: Res<Time>,
    time_style: Res<TimeStyle>,
    move_style: Res<MoveStyle>,
    mut move_remainder: ResMut<MoveRemainder>,
    mut query: Query<&mut Transform, With<Background>>,
) {
    let delta = match *time_style {
        TimeStyle::Normal => time.delta_seconds(),
        TimeStyle::Fixed => 1.0 / 60.0,
    };
    let move_input = -Vec2::X * BG_SPEED * delta;
    // Complain if the raw movement amount is unexpectedly big:
    if move_input.x.abs() > 2.5 {
        info!("Big jump: {} px", move_input.x.abs());
    }
    // Calculate how many pixels to scroll this frame, and save any
    // leftover/leftunder for future frames:
    move_remainder.0 += move_input;
    let move_pixels = match *move_style {
        MoveStyle::WholePixel => move_remainder.0.round(),
        MoveStyle::SubPixel => move_remainder.0,
    };
    move_remainder.0 -= move_pixels;

    // Move the background tiles.
    for mut transform in query.iter_mut() {
        // First, move this tile to the back of the line if it just scrolled past zero.
        if transform.translation.x < 0.0 {
            transform.translation.x =
                BG_WIDTH * (BG_TILES - 1) as f32 + 1.0 - transform.translation.x;
        }
        // Next, move the amount we calculated:
        transform.translation += move_pixels.extend(0.0);
    }
}

// Marker struct for player sprite
#[derive(Component)]
struct Player;

// Animation time for player sprite
#[derive(Component)]
struct AnimationTimer {
    timer: Timer,
}

// Marker struct for background sprites
#[derive(Component)]
struct Background;

// Sub-pixel movement accumulator for background sprites
#[derive(Component)]
struct MoveRemainder(Vec2);

// Timer for printing frame time analysis
struct FrameAnalysisTimer(Timer);

// Enums for changing runtime settings
#[derive(Debug)]
enum TimeStyle {
    Normal,
    Fixed,
}

#[derive(Debug)]
enum MoveStyle {
    WholePixel,
    SubPixel,
}
