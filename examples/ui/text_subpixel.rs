//! Side-by-side comparison of the three [`FontSmoothing`] variants at four
//! font sizes, rendering realistic content (prose, a code snippet, and a
//! run of digits) — the workloads where subpixel antialiasing matters most.
//!
//! Use this example to eyeball the quality difference between
//! [`FontSmoothing::AntiAliased`] (Bevy's default grayscale AA) and
//! [`FontSmoothing::SubpixelAntiAliased`] (the RGB subpixel path). The
//! [`FontSmoothing::None`] column is included as a reference point for "no
//! smoothing".
//!
//! On an adapter that supports `wgpu::Features::DUAL_SOURCE_BLENDING` (Metal,
//! Vulkan on most modern GPUs, DX12), the subpixel column should look visibly
//! sharper than the anti-aliased column at 10pt and 14pt — particularly on
//! the vertical stems of `l`, `i`, `k`, `b`, on the round strokes of digits,
//! and on the small punctuation in the code snippet. On adapters without DSB
//! the subpixel column transparently falls back to grayscale AA.
//!
//! # Interactive controls
//!
//! The example reacts to keyboard input while running so reviewers can
//! A/B the tuning knobs without recompiling:
//!
//! | Key | Effect |
//! |---|---|
//! | `1` | Set [`SubpixelTextSettings::enhanced_contrast`] to `0.25` (muted) |
//! | `2` | Set `enhanced_contrast` to `0.50` (default) |
//! | `3` | Set `enhanced_contrast` to `0.75` (aggressive) |
//! | `R` | Set [`SubpixelLcdLayout`] to `HorizontalRgb` (default) |
//! | `B` | Set `SubpixelLcdLayout` to `HorizontalBgr` |
//! | `S` | Save a screenshot of the primary window to `/tmp/text_subpixel_<unix-ms>.png` |
//!
//! A HUD in the top-right corner shows the current values. Changes take
//! effect on the next rendered frame — `SubpixelTextSettings` and
//! `SubpixelLcdLayout` are plain resources consumed by the subpixel fragment
//! shader each frame.
//!
//! # Environment-variable overrides
//!
//! For automated screenshots and CI, three env vars override the initial
//! values before any keypress:
//!
//! - `BEVY_TEXT_SUBPIXEL_ENHANCED_CONTRAST=<f32>` — initial contrast.
//! - `BEVY_TEXT_SUBPIXEL_LCD_LAYOUT=<name>` — one of
//!   `horizontal-rgb` / `horizontal-bgr`.
//! - `BEVY_TEXT_SUBPIXEL_SCREENSHOT=<path>` — grab one screenshot after
//!   warmup and exit (used for PR-body asset generation).

use bevy::input::ButtonInput;
use bevy::prelude::*;
use bevy::render::view::screenshot::{save_to_disk, Screenshot};
use bevy::text::{FontSize, FontSmoothing, SubpixelLcdLayout, SubpixelTextSettings};
use std::time::{SystemTime, UNIX_EPOCH};

/// Prose sample — one line of English at four sizes exercises most of the
/// Latin lowercase and caps.
const PROSE: &str = "The quick brown fox jumps over the lazy dog.";

/// Code sample — punctuation and monospace proportions stress subpixel
/// positioning.
const CODE: &str = "fn render(ctx: &mut Context) -> Result<()> { ctx.flush() }";

/// Digits — round strokes and small verticals are where grayscale AA looks
/// worst and subpixel AA helps most.
const DIGITS: &str = "0123456789  3.14159  1,234,567  -42";

/// Four realistic body sizes: small UI label, reading text, heading, large
/// display. Subpixel AA has the most visible impact at 10pt and 14pt; at
/// 20pt and 32pt the three variants should converge.
const SIZES: [f32; 4] = [10.0, 14.0, 20.0, 32.0];

const SMOOTHINGS: [(FontSmoothing, &str); 3] = [
    (FontSmoothing::None, "None"),
    (FontSmoothing::AntiAliased, "AntiAliased"),
    (FontSmoothing::SubpixelAntiAliased, "SubpixelAntiAliased"),
];

/// Marker on the HUD text entity so the update system can find it cheaply.
#[derive(Component)]
struct HudText;

fn main() {
    let mut app = App::new();
    app.add_plugins(DefaultPlugins)
        .insert_resource(ClearColor(Color::srgb(0.08, 0.08, 0.08)))
        .add_systems(Startup, setup)
        .add_systems(Update, (handle_input, update_hud));

    // Optional override of `SubpixelTextSettings::enhanced_contrast` for
    // demonstrating the tunable. Parse `BEVY_TEXT_SUBPIXEL_ENHANCED_CONTRAST`
    // (e.g. `=0.2` for a visibly muted look vs. the default `0.5`).
    if let Ok(raw) = std::env::var("BEVY_TEXT_SUBPIXEL_ENHANCED_CONTRAST")
        && let Ok(value) = raw.trim().parse::<f32>()
    {
        app.insert_resource(SubpixelTextSettings {
            enhanced_contrast: value,
            ..Default::default()
        });
    }

    // Optional override of `SubpixelLcdLayout` for demonstrating the layout
    // knob. Parse `BEVY_TEXT_SUBPIXEL_LCD_LAYOUT` as one of
    // `horizontal-rgb` / `horizontal-bgr` (case-insensitive, `-` or `_`
    // separator tolerated). Missing or unrecognised values fall back to the
    // default `HorizontalRgb`.
    if let Ok(raw) = std::env::var("BEVY_TEXT_SUBPIXEL_LCD_LAYOUT") {
        let normalised = raw.trim().to_ascii_lowercase().replace('_', "-");
        let layout = match normalised.as_str() {
            "horizontal-rgb" | "hrgb" | "rgb" => Some(SubpixelLcdLayout::HorizontalRgb),
            "horizontal-bgr" | "hbgr" | "bgr" => Some(SubpixelLcdLayout::HorizontalBgr),
            _ => None,
        };
        if let Some(layout) = layout {
            app.insert_resource(layout);
        }
    }

    // Optional automated screenshot capture for CI / PR body asset generation.
    // Set `BEVY_TEXT_SUBPIXEL_SCREENSHOT=<path>` to have the example grab the
    // primary window a few frames after startup and write a PNG to that path,
    // then exit.
    if let Ok(path) = std::env::var("BEVY_TEXT_SUBPIXEL_SCREENSHOT") {
        app.insert_resource(ScreenshotPath(path));
        app.insert_resource(ScreenshotFrame(0));
        app.add_systems(Update, take_screenshot_after_warmup);
    }

    app.run();
}

#[derive(Resource)]
struct ScreenshotPath(String);

#[derive(Resource)]
struct ScreenshotFrame(u32);

fn take_screenshot_after_warmup(
    mut commands: Commands,
    path: Res<ScreenshotPath>,
    mut frame: ResMut<ScreenshotFrame>,
    mut exit: MessageWriter<AppExit>,
) {
    frame.0 += 1;
    // Capture on frame 30 (~0.5s at 60fps) so the atlas has warmed up.
    if frame.0 == 30 {
        commands
            .spawn(Screenshot::primary_window())
            .observe(save_to_disk(path.0.clone()));
    }
    // Exit a handful of frames later so `save_to_disk` has landed on disk.
    if frame.0 >= 90 {
        exit.write(AppExit::Success);
    }
}

/// Reacts to keypresses to mutate the subpixel tuning resources. Using
/// `just_pressed` gives us edge-triggered semantics — holding the key down
/// doesn't spam updates.
fn handle_input(
    input: Res<ButtonInput<KeyCode>>,
    mut commands: Commands,
    mut settings: ResMut<SubpixelTextSettings>,
    mut layout: ResMut<SubpixelLcdLayout>,
) {
    // Enhanced-contrast presets. Preserve the existing `gamma_ratios` so
    // app authors who tuned them don't get silently clobbered by a keypress.
    if input.just_pressed(KeyCode::Digit1) {
        settings.enhanced_contrast = 0.25;
    }
    if input.just_pressed(KeyCode::Digit2) {
        settings.enhanced_contrast = 0.50;
    }
    if input.just_pressed(KeyCode::Digit3) {
        settings.enhanced_contrast = 0.75;
    }

    // LCD layout cycle.
    if input.just_pressed(KeyCode::KeyR) {
        *layout = SubpixelLcdLayout::HorizontalRgb;
    }
    if input.just_pressed(KeyCode::KeyB) {
        *layout = SubpixelLcdLayout::HorizontalBgr;
    }

    // Screenshot. Save under `/tmp` with a unix-millisecond suffix so
    // repeat presses don't clobber each other.
    if input.just_pressed(KeyCode::KeyS) {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis())
            .unwrap_or(0);
        let path = format!("/tmp/text_subpixel_{stamp}.png");
        commands
            .spawn(Screenshot::primary_window())
            .observe(save_to_disk(path));
    }
}

fn update_hud(
    settings: Res<SubpixelTextSettings>,
    layout: Res<SubpixelLcdLayout>,
    mut hud: Query<&mut Text, With<HudText>>,
) {
    // Only rewrite the HUD when something changed; `Res::is_changed` covers
    // the env-var, startup, and keypress-induced edits.
    if !settings.is_changed() && !layout.is_changed() {
        return;
    }
    let Ok(mut text) = hud.single_mut() else {
        return;
    };
    let layout_name = match *layout {
        SubpixelLcdLayout::HorizontalRgb => "HorizontalRgb",
        SubpixelLcdLayout::HorizontalBgr => "HorizontalBgr",
    };
    **text = format!(
        "contrast: {:.2}  layout: {}\n[1/2/3] contrast   [R/B] layout   [S] screenshot",
        settings.enhanced_contrast, layout_name,
    );
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn(Camera2d);

    let body_font = asset_server.load("fonts/FiraSans-Bold.ttf");
    let mono_font = asset_server.load("fonts/FiraMono-Medium.ttf");

    commands
        .spawn(Node {
            width: percent(100),
            height: percent(100),
            flex_direction: FlexDirection::Column,
            padding: UiRect::all(px(20)),
            row_gap: px(12),
            ..default()
        })
        .with_children(|root| {
            // Caption.
            root.spawn((
                Text::new(
                    "FontSmoothing comparison — three variants (columns) at four sizes \
                     (rows). The SubpixelAntiAliased column should look visibly sharper \
                     at 10pt and 14pt on a DSB-capable adapter; otherwise it falls back \
                     to grayscale AA.",
                ),
                TextFont {
                    font: body_font.clone().into(),
                    font_size: FontSize::Px(12.0),
                    font_smoothing: FontSmoothing::AntiAliased,
                    ..default()
                },
                TextColor(Color::srgb(0.70, 0.70, 0.70)),
            ));

            // Column header row.
            root.spawn(Node {
                flex_direction: FlexDirection::Row,
                column_gap: px(16),
                ..default()
            })
            .with_children(|header| {
                for (_, label) in SMOOTHINGS {
                    header
                        .spawn(Node {
                            flex_direction: FlexDirection::Column,
                            flex_grow: 1.0,
                            flex_basis: percent(0),
                            ..default()
                        })
                        .with_children(|col| {
                            col.spawn((
                                Text::new(format!("FontSmoothing::{label}")),
                                TextFont {
                                    font: body_font.clone().into(),
                                    font_size: FontSize::Px(14.0),
                                    font_smoothing: FontSmoothing::AntiAliased,
                                    ..default()
                                },
                                TextColor(Color::srgb(0.55, 0.80, 1.0)),
                            ));
                        });
                }
            });

            // Body: one row per font size, each row holds three cells.
            for size in SIZES {
                root.spawn(Node {
                    flex_direction: FlexDirection::Row,
                    column_gap: px(16),
                    ..default()
                })
                .with_children(|row| {
                    for (smoothing, _) in SMOOTHINGS {
                        row.spawn((
                            Node {
                                flex_direction: FlexDirection::Column,
                                flex_grow: 1.0,
                                flex_basis: percent(0),
                                row_gap: px(2),
                                padding: UiRect::all(px(8)),
                                border: UiRect::all(px(1)),
                                ..default()
                            },
                            BorderColor::all(Color::srgb(0.22, 0.22, 0.22)),
                            BackgroundColor(Color::BLACK),
                        ))
                        .with_children(|cell| {
                            // Size badge.
                            cell.spawn((
                                Text::new(format!("{size:>4.1}pt")),
                                TextFont {
                                    font: body_font.clone().into(),
                                    font_size: FontSize::Px(11.0),
                                    font_smoothing: FontSmoothing::AntiAliased,
                                    ..default()
                                },
                                TextColor(Color::srgb(0.45, 0.45, 0.45)),
                            ));

                            // Prose (sans).
                            cell.spawn((
                                Text::new(PROSE),
                                TextFont {
                                    font: body_font.clone().into(),
                                    font_size: FontSize::Px(size),
                                    font_smoothing: smoothing,
                                    ..default()
                                },
                                TextColor(Color::WHITE),
                            ));

                            // Code (mono).
                            cell.spawn((
                                Text::new(CODE),
                                TextFont {
                                    font: mono_font.clone().into(),
                                    font_size: FontSize::Px(size),
                                    font_smoothing: smoothing,
                                    ..default()
                                },
                                TextColor(Color::srgb(0.90, 0.90, 0.78)),
                            ));

                            // Digits (mono).
                            cell.spawn((
                                Text::new(DIGITS),
                                TextFont {
                                    font: mono_font.clone().into(),
                                    font_size: FontSize::Px(size),
                                    font_smoothing: smoothing,
                                    ..default()
                                },
                                TextColor(Color::srgb(0.78, 0.88, 1.0)),
                            ));
                        });
                    }
                });
            }
        });

    // HUD in the top-right corner. Uses absolute positioning so it floats over
    // the grid without reflowing the existing layout.
    commands.spawn((
        HudText,
        Text::new(
            "contrast: 0.50  layout: HorizontalRgb\n[1/2/3] contrast   [R/B] layout   [S] screenshot",
        ),
        TextFont {
            font: mono_font.clone().into(),
            font_size: FontSize::Px(11.0),
            font_smoothing: FontSmoothing::AntiAliased,
            ..default()
        },
        TextColor(Color::srgb(0.85, 0.85, 0.60)),
        Node {
            position_type: PositionType::Absolute,
            top: px(8),
            right: px(12),
            ..default()
        },
    ));
}
