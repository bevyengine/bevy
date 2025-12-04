//! RTS-style selection UI for machine control.
//!
//! Provides:
//! - Selection panel showing selected machine info
//! - Command card with available actions
//! - Unit portrait and status display

use bevy_app::prelude::*;
use bevy_camera::prelude::Visibility;
use bevy_color::Color;
use bevy_ecs::prelude::*;
use bevy_text::{TextColor, TextFont};
use bevy_ui::prelude::*;

use crate::machines::{Machine, MachineActivity, MachineType, PlayerControlled};

/// Plugin for RTS-style selection UI.
pub struct SelectionUiPlugin;

impl Plugin for SelectionUiPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SelectionState>()
            .add_systems(Startup, spawn_selection_ui)
            .add_systems(
                Update,
                (
                    update_selection_state,
                    update_selection_panel,
                    update_command_card,
                    handle_command_clicks,
                )
                    .chain(),
            );
    }
}

/// Current selection state.
#[derive(Resource, Default)]
pub struct SelectionState {
    /// Currently selected machine entity.
    pub selected: Option<Entity>,
    /// Type of selected machine.
    pub machine_type: Option<MachineType>,
    /// Current activity of selected machine.
    pub activity: Option<MachineActivity>,
}

/// Marker for the main selection panel.
#[derive(Component)]
pub struct SelectionPanel;

/// Marker for the unit portrait area.
#[derive(Component)]
pub struct UnitPortrait;

/// Marker for unit name text.
#[derive(Component)]
pub struct UnitNameText;

/// Marker for unit status text.
#[derive(Component)]
pub struct UnitStatusText;

/// Marker for the command card grid.
#[derive(Component)]
pub struct CommandCard;

/// Component for command buttons.
#[derive(Component)]
pub struct CommandButton {
    /// The command this button triggers.
    pub command: MachineCommand,
    /// Hotkey for this command.
    pub hotkey: Option<char>,
}

/// Available commands for machines.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MachineCommand {
    /// Stop current action.
    Stop,
    /// Move to location.
    Move,
    /// Dig/excavate at location.
    Dig,
    /// Dump load at location.
    Dump,
    /// Attack-move (dig while moving).
    AttackMove,
    /// Hold position.
    HoldPosition,
    /// Patrol between points.
    Patrol,
}

impl MachineCommand {
    /// Returns the display name for this command.
    pub fn name(&self) -> &'static str {
        match self {
            Self::Stop => "Stop",
            Self::Move => "Move",
            Self::Dig => "Dig",
            Self::Dump => "Dump",
            Self::AttackMove => "A-Move",
            Self::HoldPosition => "Hold",
            Self::Patrol => "Patrol",
        }
    }

    /// Returns the icon character for this command.
    pub fn icon(&self) -> &'static str {
        match self {
            Self::Stop => "X",
            Self::Move => "M",
            Self::Dig => "D",
            Self::Dump => "U",
            Self::AttackMove => "A",
            Self::HoldPosition => "H",
            Self::Patrol => "P",
        }
    }

    /// Returns the hotkey for this command.
    pub fn hotkey(&self) -> char {
        match self {
            Self::Stop => 'S',
            Self::Move => 'M',
            Self::Dig => 'D',
            Self::Dump => 'U',
            Self::AttackMove => 'A',
            Self::HoldPosition => 'H',
            Self::Patrol => 'P',
        }
    }

    /// Returns commands available for a machine type.
    pub fn for_machine_type(machine_type: MachineType) -> Vec<Self> {
        match machine_type {
            MachineType::Excavator => vec![
                Self::Stop,
                Self::Move,
                Self::Dig,
                Self::AttackMove,
                Self::HoldPosition,
            ],
            MachineType::Dozer => vec![
                Self::Stop,
                Self::Move,
                Self::Dig,
                Self::AttackMove,
                Self::HoldPosition,
                Self::Patrol,
            ],
            MachineType::Loader => vec![
                Self::Stop,
                Self::Move,
                Self::Dig,
                Self::Dump,
                Self::HoldPosition,
            ],
            MachineType::DumpTruck => vec![
                Self::Stop,
                Self::Move,
                Self::Dump,
                Self::HoldPosition,
                Self::Patrol,
            ],
        }
    }
}

// UI Colors - StarCraft inspired dark theme
const PANEL_BG: Color = Color::srgba(0.08, 0.08, 0.12, 0.95);
const PANEL_BORDER: Color = Color::srgba(0.3, 0.35, 0.45, 1.0);
const PORTRAIT_BG: Color = Color::srgba(0.05, 0.05, 0.08, 1.0);
const COMMAND_BTN_BG: Color = Color::srgba(0.15, 0.15, 0.2, 1.0);
const COMMAND_BTN_HOVER: Color = Color::srgba(0.25, 0.25, 0.35, 1.0);
const COMMAND_BTN_PRESSED: Color = Color::srgba(0.1, 0.1, 0.15, 1.0);
const TEXT_PRIMARY: Color = Color::srgba(0.95, 0.95, 0.9, 1.0);
const TEXT_SECONDARY: Color = Color::srgba(0.7, 0.7, 0.65, 1.0);
const ACCENT_COLOR: Color = Color::srgba(0.4, 0.7, 1.0, 1.0);
const STATUS_IDLE: Color = Color::srgba(0.5, 0.8, 0.5, 1.0);
const STATUS_WORKING: Color = Color::srgba(1.0, 0.8, 0.3, 1.0);
const STATUS_MOVING: Color = Color::srgba(0.4, 0.6, 1.0, 1.0);

/// Default commands for empty command slots.
const DEFAULT_COMMANDS: [Option<MachineCommand>; 9] = [
    Some(MachineCommand::Stop),
    Some(MachineCommand::Move),
    Some(MachineCommand::Dig),
    Some(MachineCommand::Dump),
    Some(MachineCommand::AttackMove),
    Some(MachineCommand::HoldPosition),
    Some(MachineCommand::Patrol),
    None,
    None,
];

/// Spawns the selection UI.
fn spawn_selection_ui(mut commands: Commands) {
    // Main bottom panel container
    commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Px(180.0),
                position_type: PositionType::Absolute,
                bottom: Val::Px(0.0),
                left: Val::Px(0.0),
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Stretch,
                ..Default::default()
            },
            SelectionPanel,
        ))
        .with_children(|parent| {
            // Left section - Unit Portrait & Info
            parent
                .spawn((
                    Node {
                        width: Val::Px(280.0),
                        height: Val::Percent(100.0),
                        flex_direction: FlexDirection::Row,
                        padding: UiRect::all(Val::Px(8.0)),
                        column_gap: Val::Px(12.0),
                        ..Default::default()
                    },
                    BackgroundColor(PANEL_BG),
                ))
                .with_children(|parent| {
                    // Portrait box
                    parent
                        .spawn((
                            Node {
                                width: Val::Px(120.0),
                                height: Val::Px(120.0),
                                justify_content: JustifyContent::Center,
                                align_items: AlignItems::Center,
                                border: UiRect::all(Val::Px(2.0)),
                                ..Default::default()
                            },
                            BackgroundColor(PORTRAIT_BG),
                            BorderColor::all(PANEL_BORDER),
                            UnitPortrait,
                        ))
                        .with_children(|parent| {
                            // Placeholder icon
                            parent.spawn((
                                Text::new("?"),
                                TextFont {
                                    font_size: 48.0,
                                    ..Default::default()
                                },
                                TextColor(TEXT_SECONDARY),
                            ));
                        });

                    // Unit info text
                    parent
                        .spawn(Node {
                            flex_direction: FlexDirection::Column,
                            justify_content: JustifyContent::SpaceEvenly,
                            flex_grow: 1.0,
                            ..Default::default()
                        })
                        .with_children(|parent| {
                            // Unit name
                            parent.spawn((
                                Text::new("No Selection"),
                                TextFont {
                                    font_size: 18.0,
                                    ..Default::default()
                                },
                                TextColor(TEXT_PRIMARY),
                                UnitNameText,
                            ));

                            // Unit status
                            parent.spawn((
                                Text::new("-"),
                                TextFont {
                                    font_size: 14.0,
                                    ..Default::default()
                                },
                                TextColor(TEXT_SECONDARY),
                                UnitStatusText,
                            ));

                            // Health/fuel bar placeholder
                            parent
                                .spawn((
                                    Node {
                                        width: Val::Percent(100.0),
                                        height: Val::Px(8.0),
                                        ..Default::default()
                                    },
                                    BackgroundColor(Color::srgba(0.2, 0.2, 0.2, 1.0)),
                                ))
                                .with_children(|parent| {
                                    parent.spawn((
                                        Node {
                                            width: Val::Percent(100.0),
                                            height: Val::Percent(100.0),
                                            ..Default::default()
                                        },
                                        BackgroundColor(STATUS_IDLE),
                                    ));
                                });
                        });
                });

            // Center section - Stats / Details (spacer for now)
            parent.spawn(Node {
                flex_grow: 1.0,
                ..Default::default()
            });

            // Right section - Command Card
            parent
                .spawn((
                    Node {
                        width: Val::Px(300.0),
                        height: Val::Percent(100.0),
                        padding: UiRect::all(Val::Px(8.0)),
                        flex_direction: FlexDirection::Column,
                        ..Default::default()
                    },
                    BackgroundColor(PANEL_BG),
                    CommandCard,
                ))
                .with_children(|parent| {
                    // Command card title
                    parent.spawn((
                        Text::new("Commands"),
                        TextFont {
                            font_size: 14.0,
                            ..Default::default()
                        },
                        TextColor(TEXT_SECONDARY),
                    ));

                    // Command button grid (3x3)
                    parent
                        .spawn(Node {
                            width: Val::Percent(100.0),
                            flex_grow: 1.0,
                            flex_direction: FlexDirection::Column,
                            justify_content: JustifyContent::Center,
                            row_gap: Val::Px(4.0),
                            ..Default::default()
                        })
                        .with_children(|parent| {
                            // Create 3 rows of command buttons
                            for row in 0..3 {
                                parent
                                    .spawn(Node {
                                        width: Val::Percent(100.0),
                                        flex_direction: FlexDirection::Row,
                                        justify_content: JustifyContent::Center,
                                        column_gap: Val::Px(4.0),
                                        ..Default::default()
                                    })
                                    .with_children(|parent| {
                                        // 3 buttons per row
                                        for col in 0..3 {
                                            let index = row * 3 + col;
                                            let command = DEFAULT_COMMANDS
                                                .get(index)
                                                .copied()
                                                .flatten();

                                            let mut button = parent.spawn((
                                                Button,
                                                Node {
                                                    width: Val::Px(80.0),
                                                    height: Val::Px(45.0),
                                                    justify_content: JustifyContent::Center,
                                                    align_items: AlignItems::Center,
                                                    flex_direction: FlexDirection::Column,
                                                    ..Default::default()
                                                },
                                                BackgroundColor(COMMAND_BTN_BG),
                                            ));

                                            if let Some(cmd) = command {
                                                button.insert(CommandButton {
                                                    command: cmd,
                                                    hotkey: Some(cmd.hotkey()),
                                                });

                                                button.with_children(|parent| {
                                                    // Command icon/letter
                                                    parent.spawn((
                                                        Text::new(cmd.icon()),
                                                        TextFont {
                                                            font_size: 20.0,
                                                            ..Default::default()
                                                        },
                                                        TextColor(ACCENT_COLOR),
                                                    ));

                                                    // Hotkey hint
                                                    parent.spawn((
                                                        Text::new(format!("[{}]", cmd.hotkey())),
                                                        TextFont {
                                                            font_size: 10.0,
                                                            ..Default::default()
                                                        },
                                                        TextColor(TEXT_SECONDARY),
                                                    ));
                                                });
                                            }
                                        }
                                    });
                            }
                        });
                });
        });
}

/// Updates selection state based on player-controlled machines.
fn update_selection_state(
    mut selection: ResMut<SelectionState>,
    controlled_query: Query<(Entity, &Machine, &MachineActivity), With<PlayerControlled>>,
) {
    // For now, just select the first player-controlled machine
    if let Some((entity, machine, activity)) = controlled_query.iter().next() {
        selection.selected = Some(entity);
        selection.machine_type = Some(machine.machine_type);
        selection.activity = Some(activity.clone());
    } else {
        selection.selected = None;
        selection.machine_type = None;
        selection.activity = None;
    }
}

/// Updates the selection panel display.
fn update_selection_panel(
    selection: Res<SelectionState>,
    mut name_query: Query<&mut Text, (With<UnitNameText>, Without<UnitStatusText>)>,
    mut status_query: Query<
        (&mut Text, &mut TextColor),
        (With<UnitStatusText>, Without<UnitNameText>),
    >,
    mut portrait_query: Query<&Children, With<UnitPortrait>>,
    mut portrait_text_query: Query<&mut Text, (Without<UnitNameText>, Without<UnitStatusText>)>,
) {
    // Update name
    if let Ok(mut name_text) = name_query.single_mut() {
        name_text.0 = selection
            .machine_type
            .map(|t| t.name().to_string())
            .unwrap_or_else(|| "No Selection".to_string());
    }

    // Update status
    if let Ok((mut status_text, mut status_color)) = status_query.single_mut() {
        if let Some(ref activity) = selection.activity {
            let (text, color) = match activity {
                MachineActivity::Idle => ("Idle", STATUS_IDLE),
                MachineActivity::Traveling { .. } => ("Moving", STATUS_MOVING),
                MachineActivity::Excavating { .. } => ("Excavating", STATUS_WORKING),
                MachineActivity::Dumping { .. } => ("Dumping", STATUS_WORKING),
                MachineActivity::Pushing { .. } => ("Pushing", STATUS_WORKING),
            };
            status_text.0 = text.to_string();
            status_color.0 = color;
        } else {
            status_text.0 = "-".to_string();
            status_color.0 = TEXT_SECONDARY;
        }
    }

    // Update portrait icon
    if let Ok(children) = portrait_query.single_mut() {
        for child in children.iter() {
            if let Ok(mut text) = portrait_text_query.get_mut(child) {
                text.0 = selection
                    .machine_type
                    .map(machine_icon)
                    .unwrap_or("?")
                    .to_string();
            }
        }
    }
}

/// Updates command card visibility based on selection.
fn update_command_card(
    selection: Res<SelectionState>,
    mut button_query: Query<(&CommandButton, &mut BackgroundColor, &mut Visibility)>,
) {
    let available_commands = selection
        .machine_type
        .map(MachineCommand::for_machine_type)
        .unwrap_or_default();

    for (cmd_btn, mut bg, mut visibility) in button_query.iter_mut() {
        if selection.selected.is_some() && available_commands.contains(&cmd_btn.command) {
            *visibility = Visibility::Inherited;
            bg.0 = COMMAND_BTN_BG;
        } else {
            *visibility = Visibility::Hidden;
        }
    }
}

/// Handles command button clicks.
fn handle_command_clicks(
    mut button_query: Query<
        (&Interaction, &CommandButton, &mut BackgroundColor),
        Changed<Interaction>,
    >,
    selection: Res<SelectionState>,
) {
    for (interaction, cmd_btn, mut bg) in button_query.iter_mut() {
        match *interaction {
            Interaction::Pressed => {
                bg.0 = COMMAND_BTN_PRESSED;
                if selection.selected.is_some() {
                    // TODO: Dispatch command to selected machine
                    bevy_log::info!("Command: {:?}", cmd_btn.command);
                }
            }
            Interaction::Hovered => {
                bg.0 = COMMAND_BTN_HOVER;
            }
            Interaction::None => {
                bg.0 = COMMAND_BTN_BG;
            }
        }
    }
}

/// Returns an icon character for a machine type.
fn machine_icon(machine_type: MachineType) -> &'static str {
    match machine_type {
        MachineType::Excavator => "E",
        MachineType::Dozer => "D",
        MachineType::Loader => "L",
        MachineType::DumpTruck => "T",
    }
}
