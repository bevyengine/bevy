//! Demonstrates why pointer capture matters during drag operations.
//!
//! A slider sits above a row of buttons. Without [`PointerCaptureMap`], dragging the slider thumb
//! over the buttons causes them to show a hover highlight — misleading the user into thinking the
//! button is about to be activated. With capture enabled the slider owns the pointer for the
//! duration of the drag, so no other widget receives hover state.
//!
//! Toggle capture on/off with the button at the bottom to see the difference.
//!
//! Additional UI elements showcased:
//! - A **fill bar** that grows with the slider value.
//! - **Min / max labels** flanking the track.
//! - A **Reset button** that snaps the value back to the default.

use bevy::prelude::*;

const TRACK_WIDTH: f32 = 500.0;
const TRACK_HEIGHT: f32 = 14.0;
const THUMB_SIZE: f32 = 28.0;
const THUMB_HALF: f32 = THUMB_SIZE / 2.0;
const FILL_HEIGHT: f32 = 8.0;

const SLIDER_DEFAULT_VALUE: f32 = 0.5;

const COLOR_TRACK: Color = Color::srgb(0.2, 0.2, 0.2);
const COLOR_FILL: Color = Color::srgb(0.35, 0.65, 0.35);
const COLOR_FILL_DRAGGING: Color = Color::srgb(0.45, 0.85, 0.45);
const COLOR_THUMB_IDLE: Color = Color::srgb(0.85, 0.55, 0.1);
const COLOR_THUMB_HOVER: Color = Color::srgb(1.0, 0.75, 0.2);
const COLOR_DECOY_IDLE: Color = Color::srgb(0.2, 0.45, 0.8);
const COLOR_DECOY_HOVER: Color = Color::srgb(0.9, 0.2, 0.2);
const COLOR_TOGGLE_ON: Color = Color::srgb(0.15, 0.65, 0.3);
const COLOR_TOGGLE_OFF: Color = Color::srgb(0.55, 0.15, 0.15);
const COLOR_RESET_IDLE: Color = Color::srgb(0.3, 0.3, 0.55);
const COLOR_RESET_HOVER: Color = Color::srgb(0.5, 0.5, 0.75);

/// The normalized (0 – 1) position of the slider thumb.
#[derive(Resource)]
struct SliderValue(f32);

impl Default for SliderValue {
    fn default() -> Self {
        Self(SLIDER_DEFAULT_VALUE)
    }
}

/// Whether [`PointerCaptureMap`] is used while dragging the thumb.
#[derive(Resource)]
struct CaptureEnabled(bool);

impl Default for CaptureEnabled {
    fn default() -> Self {
        Self(true)
    }
}

#[derive(Component)]
struct SliderThumb;

#[derive(Component)]
struct SliderFill;

#[derive(Component)]
struct SliderLabel;

#[derive(Component)]
struct ToggleLabel;

#[derive(Component)]
struct ToggleButton;

#[derive(Component)]
struct ResetButton;

#[derive(Component)]
struct DecoyButton;

/// X offset of the left edge of the thumb node.
fn thumb_left(v: f32) -> f32 {
    v.clamp(0.0, 1.0) * (TRACK_WIDTH - THUMB_SIZE)
}

/// Width of the fill bar (reaches to the center of the thumb).
fn fill_width(v: f32) -> f32 {
    v.clamp(0.0, 1.0) * (TRACK_WIDTH - THUMB_SIZE) + THUMB_HALF
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .init_resource::<SliderValue>()
        .init_resource::<CaptureEnabled>()
        .add_systems(Startup, setup)
        .add_systems(Update, (sync_thumb_and_label, sync_fill))
        .run();
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);

    // Root column – centered, full-screen, non-pickable container.
    commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                row_gap: Val::Px(28.0),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .with_children(|root| {
            root.spawn((
                Text::new(
                    "Drag the slider over the blue buttons.\n\
                     With capture ON they stay blue. With capture OFF they incorrectly turn red.",
                ),
                TextFont::from_font_size(15.0),
                TextColor(Color::srgb(0.75, 0.75, 0.75)),
            ));

            root.spawn((
                Node {
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    column_gap: Val::Px(10.0),
                    ..default()
                },
                Pickable::IGNORE,
            ))
            .with_children(|row| {
                row.spawn((
                    Text::new("0.0"),
                    TextFont::from_font_size(13.0),
                    TextColor(Color::srgb(0.55, 0.55, 0.55)),
                ));

                row.spawn((
                    Node {
                        width: Val::Px(TRACK_WIDTH),
                        height: Val::Px(TRACK_HEIGHT),
                        position_type: PositionType::Relative,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    BackgroundColor(COLOR_TRACK),
                    Pickable::IGNORE,
                ))
                .with_children(|track| {
                    track.spawn((
                        SliderFill,
                        Node {
                            width: Val::Px(fill_width(SLIDER_DEFAULT_VALUE)),
                            height: Val::Px(FILL_HEIGHT),
                            position_type: PositionType::Absolute,
                            left: Val::Px(0.0),
                            ..default()
                        },
                        BackgroundColor(COLOR_FILL),
                        Pickable::IGNORE,
                    ));

                    track
                        .spawn((
                            SliderThumb,
                            Node {
                                width: Val::Px(THUMB_SIZE),
                                height: Val::Px(THUMB_SIZE),
                                position_type: PositionType::Absolute,
                                left: Val::Px(thumb_left(SLIDER_DEFAULT_VALUE)),
                                border_radius: BorderRadius::all(Val::Px(4.0)),
                                ..default()
                            },
                            BackgroundColor(COLOR_THUMB_IDLE),
                        ))
                        .observe(on_drag_start)
                        .observe(on_drag)
                        .observe(on_drag_end)
                        .observe(
                            |_: On<Pointer<Over>>,
                             mut q: Single<&mut BackgroundColor, With<SliderThumb>>| {
                                q.0 = COLOR_THUMB_HOVER;
                            },
                        )
                        .observe(
                            |_: On<Pointer<Out>>,
                             mut q: Single<&mut BackgroundColor, With<SliderThumb>>| {
                                q.0 = COLOR_THUMB_IDLE;
                            },
                        );
                });

                row.spawn((
                    Text::new("1.0"),
                    TextFont::from_font_size(13.0),
                    TextColor(Color::srgb(0.55, 0.55, 0.55)),
                ));
            });

            root.spawn((
                SliderLabel,
                Text::new(format!("Value: {:.2}", SLIDER_DEFAULT_VALUE)),
                TextFont::from_font_size(18.0),
            ));

            root.spawn((
                Node {
                    flex_direction: FlexDirection::Row,
                    column_gap: Val::Px(16.0),
                    ..default()
                },
                Pickable::IGNORE,
            ))
            .with_children(|row| {
                for label in ["Button A", "Button B", "Button C", "Button D", "Button E"] {
                    spawn_decoy_button(row, label);
                }
            });

            root.spawn((
                Node {
                    flex_direction: FlexDirection::Row,
                    column_gap: Val::Px(20.0),
                    align_items: AlignItems::Center,
                    ..default()
                },
                Pickable::IGNORE,
            ))
            .with_children(|row| {
                row.spawn((
                    ToggleButton,
                    Node {
                        padding: UiRect::axes(Val::Px(20.0), Val::Px(10.0)),
                        border_radius: BorderRadius::all(Val::Px(6.0)),
                        ..default()
                    },
                    BackgroundColor(COLOR_TOGGLE_ON),
                ))
                .with_children(|btn| {
                    btn.spawn((
                        ToggleLabel,
                        Text::new("Capture: ON  (click to toggle)"),
                        TextFont::from_font_size(16.0),
                        TextColor(Color::WHITE),
                        Pickable::IGNORE,
                    ));
                })
                .observe(on_toggle_click);

                row.spawn((
                    ResetButton,
                    Node {
                        padding: UiRect::axes(Val::Px(20.0), Val::Px(10.0)),
                        border_radius: BorderRadius::all(Val::Px(6.0)),
                        ..default()
                    },
                    BackgroundColor(COLOR_RESET_IDLE),
                ))
                .with_children(|btn| {
                    btn.spawn((
                        Text::new("Reset"),
                        TextFont::from_font_size(16.0),
                        TextColor(Color::WHITE),
                        Pickable::IGNORE,
                    ));
                })
                .observe(
                    |_: On<Pointer<Over>>,
                     mut q: Single<&mut BackgroundColor, With<ResetButton>>| {
                        q.0 = COLOR_RESET_HOVER;
                    },
                )
                .observe(
                    |_: On<Pointer<Out>>,
                     mut q: Single<&mut BackgroundColor, With<ResetButton>>| {
                        q.0 = COLOR_RESET_IDLE;
                    },
                )
                .observe(on_reset_click);
            });
        });
}

fn spawn_decoy_button(parent: &mut ChildSpawnerCommands, label: &str) {
    parent
        .spawn((
            DecoyButton,
            Node {
                padding: UiRect::axes(Val::Px(24.0), Val::Px(14.0)),
                border_radius: BorderRadius::all(Val::Px(6.0)),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(COLOR_DECOY_IDLE),
        ))
        .with_child((
            Text::new(label),
            TextFont::from_font_size(15.0),
            TextColor(Color::WHITE),
            Pickable::IGNORE,
        ))
        .observe(
            |event: On<Pointer<Over>>,
             mut colors: Query<&mut BackgroundColor, With<DecoyButton>>| {
                if let Ok(mut color) = colors.get_mut(event.entity) {
                    color.0 = COLOR_DECOY_HOVER;
                }
            },
        )
        .observe(
            |event: On<Pointer<Out>>,
             mut colors: Query<&mut BackgroundColor, With<DecoyButton>>| {
                if let Ok(mut color) = colors.get_mut(event.entity) {
                    color.0 = COLOR_DECOY_IDLE;
                }
            },
        );
}

fn on_drag_start(
    event: On<Pointer<DragStart>>,
    capture_enabled: Res<CaptureEnabled>,
    mut capture_map: ResMut<PointerCaptureMap>,
    mut fill_color: Single<&mut BackgroundColor, With<SliderFill>>,
) {
    if capture_enabled.0 {
        capture_map.capture(event.pointer_id, event.entity, event.hit.clone());
    }
    // Signal active drag with a brighter fill color.
    fill_color.0 = COLOR_FILL_DRAGGING;
}

fn on_drag(event: On<Pointer<Drag>>, mut value: ResMut<SliderValue>) {
    value.0 = (value.0 + event.delta.x / (TRACK_WIDTH - THUMB_SIZE)).clamp(0.0, 1.0);
}

fn on_drag_end(
    event: On<Pointer<DragEnd>>,
    mut capture_map: ResMut<PointerCaptureMap>,
    mut fill_color: Single<&mut BackgroundColor, With<SliderFill>>,
) {
    capture_map.release(event.pointer_id);
    fill_color.0 = COLOR_FILL;
}

fn on_toggle_click(
    _: On<Pointer<Click>>,
    mut capture_enabled: ResMut<CaptureEnabled>,
    mut label: Single<&mut Text, With<ToggleLabel>>,
    mut toggle_bg: Single<&mut BackgroundColor, With<ToggleButton>>,
) {
    capture_enabled.0 = !capture_enabled.0;
    if capture_enabled.0 {
        label.0 = "Capture: ON  (click to toggle)".into();
        toggle_bg.0 = COLOR_TOGGLE_ON;
    } else {
        label.0 = "Capture: OFF  (click to toggle)".into();
        toggle_bg.0 = COLOR_TOGGLE_OFF;
    }
}

fn on_reset_click(_: On<Pointer<Click>>, mut value: ResMut<SliderValue>) {
    value.0 = SLIDER_DEFAULT_VALUE;
}

/// Moves the thumb and updates the numeric readout.
fn sync_thumb_and_label(
    value: Res<SliderValue>,
    mut thumb: Single<&mut Node, With<SliderThumb>>,
    mut value_label: Single<&mut Text, With<SliderLabel>>,
) {
    thumb.left = Val::Px(thumb_left(value.0));
    value_label.0 = format!("Value: {:.2}", value.0);
}

/// Resizes the fill bar to match the current slider value.
fn sync_fill(value: Res<SliderValue>, mut fill: Single<&mut Node, With<SliderFill>>) {
    fill.width = Val::Px(fill_width(value.0));
}
