//! UI components for Zyns display.

use bevy_app::prelude::*;
use bevy_camera::prelude::Visibility;
use bevy_color::{Alpha, Color};
use bevy_ecs::prelude::*;
use bevy_text::{TextColor, TextFont};
use bevy_time::Time;
use bevy_ui::prelude::*;

use super::{AchievementUnlockedEvent, ZynsEarnedEvent, ZynsWallet};
use crate::config::EarthworksConfig;

/// Plugin for Zyns UI components.
pub struct ZynsUiPlugin;

impl Plugin for ZynsUiPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ZynsUiState>()
            .add_systems(Startup, spawn_zyns_hud)
            .add_systems(
                Update,
                (
                    update_zyns_hud_visibility,
                    update_zyns_display,
                    animate_zyns_earned,
                    show_achievement_notification,
                    fade_notifications,
                )
                    .chain(),
            );
    }
}

/// State for Zyns UI.
#[derive(Resource, Default)]
pub struct ZynsUiState {
    /// Whether the HUD is visible.
    pub visible: bool,
    /// Current display value (for animation).
    pub display_value: f64,
    /// Animation timer for counting up.
    pub animation_timer: f32,
}

// Component markers
/// Marker component for the Zyns HUD container.
#[derive(Component)]
pub struct ZynsHud;

/// Marker component for the Zyns balance text.
#[derive(Component)]
pub struct ZynsBalanceText;

/// Component for floating earned popup text.
#[derive(Component)]
pub struct ZynsEarnedPopup {
    /// Current lifetime in seconds.
    pub lifetime: f32,
    /// Maximum lifetime before despawn.
    pub max_lifetime: f32,
}

/// Component for achievement notification panels.
#[derive(Component)]
pub struct AchievementNotification {
    /// Current lifetime in seconds.
    pub lifetime: f32,
    /// Maximum lifetime before despawn.
    pub max_lifetime: f32,
}

// Colors
const HUD_BG: Color = Color::srgba(0.05, 0.05, 0.1, 0.85);
const ZYNS_COLOR: Color = Color::srgba(1.0, 0.85, 0.2, 1.0); // Gold
const ZYNS_EARNED_COLOR: Color = Color::srgba(0.4, 1.0, 0.4, 1.0); // Green
const ACHIEVEMENT_BG: Color = Color::srgba(0.2, 0.1, 0.4, 0.95); // Purple
const ACHIEVEMENT_TEXT: Color = Color::srgba(1.0, 0.9, 0.5, 1.0); // Light gold

/// Updates HUD visibility based on EarthworksConfig.
fn update_zyns_hud_visibility(
    config: Res<EarthworksConfig>,
    mut hud_query: Query<&mut Visibility, With<ZynsHud>>,
) {
    for mut visibility in hud_query.iter_mut() {
        *visibility = if config.show_zyns_hud {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
    }
}

/// Spawns the Zyns HUD overlay.
fn spawn_zyns_hud(mut commands: Commands) {
    // Main HUD container - top right
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(20.0),
                right: Val::Px(20.0),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::FlexEnd,
                row_gap: Val::Px(8.0),
                ..Default::default()
            },
            ZynsHud,
        ))
        .with_children(|parent| {
            // Zyns balance container
            parent
                .spawn((
                    Node {
                        padding: UiRect::new(
                            Val::Px(16.0),
                            Val::Px(16.0),
                            Val::Px(8.0),
                            Val::Px(8.0),
                        ),
                        flex_direction: FlexDirection::Row,
                        align_items: AlignItems::Center,
                        column_gap: Val::Px(8.0),
                        ..Default::default()
                    },
                    BackgroundColor(HUD_BG),
                ))
                .with_children(|parent| {
                    // Zyns icon/label
                    parent.spawn((
                        Text::new("Z"),
                        TextFont {
                            font_size: 28.0,
                            ..Default::default()
                        },
                        TextColor(ZYNS_COLOR),
                    ));

                    // Balance value
                    parent.spawn((
                        Text::new("0"),
                        TextFont {
                            font_size: 24.0,
                            ..Default::default()
                        },
                        TextColor(ZYNS_COLOR),
                        ZynsBalanceText,
                    ));
                });
        });
}

/// Updates the Zyns display with animated counting.
fn update_zyns_display(
    time: Res<Time>,
    wallet: Res<ZynsWallet>,
    mut ui_state: ResMut<ZynsUiState>,
    mut text_query: Query<&mut Text, With<ZynsBalanceText>>,
) {
    let target = wallet.balance as f64;

    // Animate towards target
    if (ui_state.display_value - target).abs() > 0.5 {
        let speed = ((target - ui_state.display_value).abs() * 5.0).max(50.0);
        let direction = if target > ui_state.display_value {
            1.0
        } else {
            -1.0
        };
        ui_state.display_value += direction * speed * time.delta_secs() as f64;

        // Clamp to prevent overshooting
        if direction > 0.0 {
            ui_state.display_value = ui_state.display_value.min(target);
        } else {
            ui_state.display_value = ui_state.display_value.max(target);
        }
    } else {
        ui_state.display_value = target;
    }

    // Update text
    if let Ok(mut text) = text_query.single_mut() {
        text.0 = format_zyns(ui_state.display_value as u64);
    }
}

/// Spawns floating "+X" text when Zyns are earned.
fn animate_zyns_earned(
    mut commands: Commands,
    mut events: MessageReader<ZynsEarnedEvent>,
    hud_query: Query<Entity, With<ZynsHud>>,
) {
    let Ok(hud_entity) = hud_query.single() else {
        return;
    };

    for event in events.read() {
        if event.amount == 0 {
            continue;
        }

        // Spawn floating text as child of HUD
        commands.entity(hud_entity).with_children(|parent| {
            parent.spawn((
                Node {
                    position_type: PositionType::Relative,
                    ..Default::default()
                },
                Text::new(format!("+{}", event.amount)),
                TextFont {
                    font_size: 18.0,
                    ..Default::default()
                },
                TextColor(ZYNS_EARNED_COLOR),
                ZynsEarnedPopup {
                    lifetime: 0.0,
                    max_lifetime: 1.5,
                },
            ));
        });
    }
}

/// Shows achievement notification.
fn show_achievement_notification(
    mut commands: Commands,
    mut events: MessageReader<AchievementUnlockedEvent>,
    hud_query: Query<Entity, With<ZynsHud>>,
) {
    let Ok(hud_entity) = hud_query.single() else {
        return;
    };

    for event in events.read() {
        commands.entity(hud_entity).with_children(|parent| {
            parent
                .spawn((
                    Node {
                        padding: UiRect::all(Val::Px(12.0)),
                        flex_direction: FlexDirection::Column,
                        align_items: AlignItems::Center,
                        row_gap: Val::Px(4.0),
                        ..Default::default()
                    },
                    BackgroundColor(ACHIEVEMENT_BG),
                    AchievementNotification {
                        lifetime: 0.0,
                        max_lifetime: 3.0,
                    },
                ))
                .with_children(|parent| {
                    // Achievement title
                    parent.spawn((
                        Text::new("ACHIEVEMENT UNLOCKED"),
                        TextFont {
                            font_size: 12.0,
                            ..Default::default()
                        },
                        TextColor(ACHIEVEMENT_TEXT),
                    ));

                    // Achievement name
                    parent.spawn((
                        Text::new(event.achievement.name()),
                        TextFont {
                            font_size: 16.0,
                            ..Default::default()
                        },
                        TextColor(Color::WHITE),
                    ));

                    // Reward
                    parent.spawn((
                        Text::new(format!("+{} Zyns", event.reward)),
                        TextFont {
                            font_size: 14.0,
                            ..Default::default()
                        },
                        TextColor(ZYNS_COLOR),
                    ));
                });
        });
    }
}

/// Fades and removes notification elements.
fn fade_notifications(
    mut commands: Commands,
    time: Res<Time>,
    mut popup_query: Query<(Entity, &mut ZynsEarnedPopup, &mut TextColor)>,
    mut achievement_query: Query<
        (Entity, &mut AchievementNotification, &mut BackgroundColor),
        Without<ZynsEarnedPopup>,
    >,
) {
    let dt = time.delta_secs();

    // Fade earned popups
    for (entity, mut popup, mut color) in popup_query.iter_mut() {
        popup.lifetime += dt;
        let alpha = 1.0 - (popup.lifetime / popup.max_lifetime);
        color.0 = color.0.with_alpha(alpha.max(0.0));

        if popup.lifetime >= popup.max_lifetime {
            commands.entity(entity).despawn();
        }
    }

    // Fade achievement notifications
    for (entity, mut notification, mut bg) in achievement_query.iter_mut() {
        notification.lifetime += dt;

        // Start fading after 2 seconds
        if notification.lifetime > 2.0 {
            let fade_progress = (notification.lifetime - 2.0) / (notification.max_lifetime - 2.0);
            let alpha = 1.0 - fade_progress;
            bg.0 = bg.0.with_alpha(alpha.max(0.0));
        }

        if notification.lifetime >= notification.max_lifetime {
            commands.entity(entity).despawn();
        }
    }
}

/// Formats Zyns value with thousands separators.
fn format_zyns(value: u64) -> String {
    if value < 1000 {
        value.to_string()
    } else if value < 1_000_000 {
        format!("{}.{}K", value / 1000, (value % 1000) / 100)
    } else {
        format!("{}.{}M", value / 1_000_000, (value % 1_000_000) / 100_000)
    }
}
