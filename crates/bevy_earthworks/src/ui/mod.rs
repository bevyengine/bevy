//! UI components for earthworks visualization.
//!
//! Provides timeline and playback controls for plan execution.

use bevy_app::prelude::*;
use bevy_camera::prelude::Visibility;
use bevy_color::Color;
use bevy_ecs::prelude::*;
use bevy_input::mouse::MouseButton;
use bevy_input::prelude::*;
use bevy_text::{TextColor, TextFont};
use bevy_ui::prelude::*;
use bevy_ui::{ComputedNode, UiGlobalTransform};

use crate::plan::PlanPlayback;

/// Plugin for UI systems.
pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<TimelineState>()
            .init_resource::<ScoreOverlayState>()
            .add_systems(Startup, spawn_timeline_ui)
            .add_systems(
                Update,
                (
                    update_timeline_visibility,
                    update_timeline_display,
                    handle_timeline_interaction,
                    handle_scrubber_interaction,
                )
                    .chain(),
            );
    }
}

/// Marker component for the timeline UI entity.
#[derive(Component)]
pub struct TimelineUi;

/// Marker component for the score overlay UI entity.
#[derive(Component)]
pub struct ScoreOverlayUi;

/// UI state for timeline controls.
#[derive(Resource, Default)]
pub struct TimelineState {
    /// Whether the timeline panel is visible.
    pub visible: bool,
    /// Whether currently dragging the scrubber.
    pub dragging: bool,
}

/// UI state for score overlay.
#[derive(Resource, Default)]
pub struct ScoreOverlayState {
    /// Whether the overlay is visible.
    pub visible: bool,
    /// Whether to show detailed stats.
    pub show_details: bool,
}

// Component markers for UI elements
#[derive(Component)]
struct PlayPauseButton;

#[derive(Component)]
struct TimelineBar;

#[derive(Component)]
struct TimelineProgress;

#[derive(Component)]
struct TimeDisplay;

#[derive(Component)]
struct SpeedButton {
    speed: f32,
}

#[derive(Component)]
struct Scrubber;

#[derive(Component)]
struct ScrubberHandle;

// Color constants
const TIMELINE_BG: Color = Color::srgba(0.1, 0.1, 0.1, 0.9);
const TIMELINE_BAR_BG: Color = Color::srgba(0.2, 0.2, 0.2, 1.0);
const TIMELINE_PROGRESS: Color = Color::srgba(0.3, 0.6, 0.9, 1.0);
const BUTTON_BG: Color = Color::srgba(0.25, 0.25, 0.25, 1.0);
const BUTTON_HOVER: Color = Color::srgba(0.35, 0.35, 0.35, 1.0);
const BUTTON_PRESSED: Color = Color::srgba(0.15, 0.15, 0.15, 1.0);
const BUTTON_ACTIVE: Color = Color::srgba(0.2, 0.5, 0.8, 1.0);
const TEXT_COLOR: Color = Color::srgba(0.9, 0.9, 0.9, 1.0);
const SCRUBBER_COLOR: Color = Color::srgba(0.9, 0.9, 0.9, 1.0);

/// Startup system to create the timeline UI.
fn spawn_timeline_ui(mut commands: Commands) {
    // Root timeline container - positioned at bottom of screen
    commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Px(80.0),
                position_type: PositionType::Absolute,
                bottom: Val::Px(0.0),
                left: Val::Px(0.0),
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                padding: UiRect::all(Val::Px(10.0)),
                column_gap: Val::Px(10.0),
                ..Default::default()
            },
            BackgroundColor(TIMELINE_BG),
            TimelineUi,
        ))
        .with_children(|parent| {
            // Play/Pause button
            parent
                .spawn((
                    Button,
                    Node {
                        width: Val::Px(60.0),
                        height: Val::Px(60.0),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..Default::default()
                    },
                    BackgroundColor(BUTTON_BG),
                    PlayPauseButton,
                ))
                .with_children(|parent| {
                    parent.spawn((
                        Text::new("▶"),
                        TextFont {
                            font_size: 24.0,
                            ..Default::default()
                        },
                        TextColor(TEXT_COLOR),
                    ));
                });

            // Timeline bar container
            parent
                .spawn((
                    Node {
                        width: Val::Px(400.0),
                        height: Val::Px(60.0),
                        flex_direction: FlexDirection::Column,
                        justify_content: JustifyContent::Center,
                        row_gap: Val::Px(5.0),
                        ..Default::default()
                    },
                    TimelineBar,
                ))
                .with_children(|parent| {
                    // Time display
                    parent.spawn((
                        Text::new("0.0s / 0.0s"),
                        TextFont {
                            font_size: 14.0,
                            ..Default::default()
                        },
                        TextColor(TEXT_COLOR),
                        TimeDisplay,
                    ));

                    // Progress bar with scrubber
                    parent
                        .spawn((
                            Node {
                                width: Val::Percent(100.0),
                                height: Val::Px(20.0),
                                position_type: PositionType::Relative,
                                ..Default::default()
                            },
                            BackgroundColor(TIMELINE_BAR_BG),
                            Scrubber,
                        ))
                        .with_children(|parent| {
                            // Progress fill
                            parent.spawn((
                                Node {
                                    width: Val::Percent(0.0),
                                    height: Val::Percent(100.0),
                                    ..Default::default()
                                },
                                BackgroundColor(TIMELINE_PROGRESS),
                                TimelineProgress,
                            ));

                            // Scrubber handle
                            parent.spawn((
                                Node {
                                    width: Val::Px(12.0),
                                    height: Val::Px(24.0),
                                    position_type: PositionType::Absolute,
                                    left: Val::Percent(0.0),
                                    top: Val::Px(-2.0),
                                    ..Default::default()
                                },
                                BackgroundColor(SCRUBBER_COLOR),
                                ScrubberHandle,
                            ));
                        });
                });

            // Playback speed controls
            parent
                .spawn(Node {
                    flex_direction: FlexDirection::Row,
                    column_gap: Val::Px(5.0),
                    align_items: AlignItems::Center,
                    ..Default::default()
                })
                .with_children(|parent| {
                    // Speed label
                    parent.spawn((
                        Text::new("Speed:"),
                        TextFont {
                            font_size: 14.0,
                            ..Default::default()
                        },
                        TextColor(TEXT_COLOR),
                    ));

                    // Speed buttons
                    for speed in [0.5, 1.0, 2.0, 4.0] {
                        parent
                            .spawn((
                                Button,
                                Node {
                                    width: Val::Px(45.0),
                                    height: Val::Px(30.0),
                                    justify_content: JustifyContent::Center,
                                    align_items: AlignItems::Center,
                                    ..Default::default()
                                },
                                BackgroundColor(if speed == 1.0 {
                                    BUTTON_ACTIVE
                                } else {
                                    BUTTON_BG
                                }),
                                SpeedButton { speed },
                            ))
                            .with_children(|parent| {
                                parent.spawn((
                                    Text::new(format!("{}x", speed)),
                                    TextFont {
                                        font_size: 12.0,
                                        ..Default::default()
                                    },
                                    TextColor(TEXT_COLOR),
                                ));
                            });
                    }
                });
        });
}

/// System to update timeline visibility based on TimelineState.
fn update_timeline_visibility(
    timeline_state: Res<TimelineState>,
    mut timeline_query: Query<&mut Visibility, With<TimelineUi>>,
) {
    for mut visibility in timeline_query.iter_mut() {
        *visibility = if timeline_state.visible {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
    }
}

/// System to update timeline display based on PlanPlayback state.
fn update_timeline_display(
    playback: Res<PlanPlayback>,
    mut play_pause_query: Query<&Children, With<PlayPauseButton>>,
    mut text_query: Query<&mut Text, Without<TimeDisplay>>,
    mut time_display_query: Query<&mut Text, With<TimeDisplay>>,
    mut progress_query: Query<&mut Node, (With<TimelineProgress>, Without<ScrubberHandle>)>,
    mut scrubber_handle_query: Query<&mut Node, (With<ScrubberHandle>, Without<TimelineProgress>)>,
) {
    // Update play/pause button text
    if let Ok(children) = play_pause_query.single() {
        for child in children.iter() {
            if let Ok(mut text) = text_query.get_mut(child) {
                text.0 = if playback.is_playing() {
                    "⏸".to_string()
                } else {
                    "▶".to_string()
                };
            }
        }
    }

    // Update time display
    if let Ok(mut text) = time_display_query.single_mut() {
        text.0 = format!(
            "{:.1}s / {:.1}s",
            playback.current_time(),
            playback.duration()
        );
    }

    // Update progress bar
    if let Ok(mut node) = progress_query.single_mut() {
        let progress = playback.progress() * 100.0;
        node.width = Val::Percent(progress);
    }

    // Update scrubber handle position
    if let Ok(mut node) = scrubber_handle_query.single_mut() {
        let progress = playback.progress() * 100.0;
        node.left = Val::Percent(progress);
    }
}

/// System to handle button clicks and interactions.
fn handle_timeline_interaction(
    mut playback: ResMut<PlanPlayback>,
    mut play_pause_query: Query<&Interaction, (Changed<Interaction>, With<PlayPauseButton>)>,
    mut speed_buttons: Query<
        (&Interaction, &SpeedButton, &mut BackgroundColor),
        Without<PlayPauseButton>,
    >,
) {
    // Handle play/pause button
    for interaction in play_pause_query.iter_mut() {
        if *interaction == Interaction::Pressed {
            playback.toggle();
        }
    }

    // First pass: check if any speed button was pressed and capture the new speed
    let mut new_speed: Option<f32> = None;
    for (interaction, speed_button, _) in speed_buttons.iter() {
        if *interaction == Interaction::Pressed {
            new_speed = Some(speed_button.speed);
            playback.set_speed(speed_button.speed);
            break;
        }
    }

    // Second pass: update colors based on interaction state and active speed
    let current_speed = playback.speed();
    for (interaction, speed_button, mut bg_color) in speed_buttons.iter_mut() {
        let is_active = (speed_button.speed - current_speed).abs() < 0.01;

        if new_speed.is_some() {
            // A button was just pressed - update all to show active/inactive state
            bg_color.0 = if is_active { BUTTON_ACTIVE } else { BUTTON_BG };
        } else {
            // Normal hover/none handling
            match *interaction {
                Interaction::Pressed => {
                    // Already handled above
                }
                Interaction::Hovered => {
                    if !is_active {
                        bg_color.0 = BUTTON_HOVER;
                    }
                }
                Interaction::None => {
                    bg_color.0 = if is_active { BUTTON_ACTIVE } else { BUTTON_BG };
                }
            }
        }
    }
}

/// System to handle scrubber dragging and seeking.
fn handle_scrubber_interaction(
    mut playback: ResMut<PlanPlayback>,
    mut timeline_state: ResMut<TimelineState>,
    mouse_button: Res<ButtonInput<MouseButton>>,
    scrubber_query: Query<(&ComputedNode, &UiGlobalTransform), With<Scrubber>>,
    windows: Query<&bevy_window::Window, With<bevy_window::PrimaryWindow>>,
) {
    let Ok((scrubber_node, scrubber_transform)) = scrubber_query.single() else {
        return;
    };

    let Ok(window) = windows.single() else {
        return;
    };

    let Some(cursor_position) = window.cursor_position() else {
        return;
    };

    // Start dragging on press
    if mouse_button.just_pressed(MouseButton::Left) {
        // Check if cursor is over scrubber
        if scrubber_node.contains_point(*scrubber_transform, cursor_position) {
            timeline_state.dragging = true;
        }
    }

    // Stop dragging on release
    if mouse_button.just_released(MouseButton::Left) {
        timeline_state.dragging = false;
    }

    // Handle dragging
    if timeline_state.dragging && mouse_button.pressed(MouseButton::Left) {
        // Get scrubber size and position
        let scrubber_size = scrubber_node.size();

        // Transform cursor position to local coordinates
        if let Some(inverse_transform) = scrubber_transform.try_inverse() {
            let local_cursor = inverse_transform.transform_point2(cursor_position);

            // Calculate relative position within scrubber (local coordinates are centered)
            let relative_x = local_cursor.x + scrubber_size.x / 2.0;
            let scrubber_width = scrubber_size.x;

            if scrubber_width > 0.0 {
                let progress = (relative_x / scrubber_width).clamp(0.0, 1.0);
                let new_time = progress * playback.duration();
                playback.seek(new_time);
            }
        }
    }
}
