//! UI creation and management for the entity inspector.

use bevy_color::Color;
use bevy_ecs::{entity::Entity, system::Commands};
use bevy_ui::{
    BorderColor, FlexDirection, Node, PositionType, UiRect, Val, BackgroundColor, UiTargetCamera,
};
use bevy_text::{TextColor, TextFont};
use bevy_ui::widget::Text;

use super::systems::{EntityListContainer, ComponentViewerContainer, InspectorEntity};

/// Creates the main inspector UI layout.
pub fn create_inspector_ui(commands: &mut Commands, camera_entity: Entity) -> Entity {
    let ui_root = commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Row,
                ..Default::default()
            },
            BackgroundColor(Color::srgb(0.1, 0.1, 0.1)),
            UiTargetCamera(camera_entity),
            InspectorEntity,
        ))
        .with_children(|parent| {
            // Left pane - Entity list
            parent
                .spawn((
                    Node {
                        width: Val::Percent(40.0),
                        height: Val::Percent(100.0),
                        border: UiRect::all(Val::Px(1.0)),
                        padding: UiRect::all(Val::Px(8.0)),
                        flex_direction: FlexDirection::Column,
                        ..Default::default()
                    },
                    BackgroundColor(Color::srgb(0.15, 0.15, 0.15)),
                    BorderColor::all(Color::srgb(0.3, 0.3, 0.3)),
                    InspectorEntity,
                ))
                .with_children(|parent| {
                    // Entity list header
                    parent.spawn((
                        Text::new("Entities"),
                        TextFont {
                            font_size: 16.0,
                            ..Default::default()
                        },
                        TextColor(Color::WHITE),
                        InspectorEntity,
                    ));
                    
                    // Scrollable entity list container
                    parent.spawn((
                        Node {
                            width: Val::Percent(100.0),
                            height: Val::Percent(90.0),
                            flex_direction: FlexDirection::Column,
                            overflow: bevy_ui::Overflow::clip_y(),
                            ..Default::default()
                        },
                        BackgroundColor(Color::srgb(0.1, 0.1, 0.1)),
                        EntityListContainer,
                        InspectorEntity,
                    ));
                });

            // Right pane - Component viewer
            parent
                .spawn((
                    Node {
                        width: Val::Percent(60.0),
                        height: Val::Percent(100.0),
                        border: UiRect::all(Val::Px(1.0)),
                        padding: UiRect::all(Val::Px(8.0)),
                        flex_direction: FlexDirection::Column,
                        ..Default::default()
                    },
                    BackgroundColor(Color::srgb(0.12, 0.12, 0.12)),
                    BorderColor::all(Color::srgb(0.3, 0.3, 0.3)),
                    InspectorEntity,
                ))
                .with_children(|parent| {
                    // Component viewer header
                    parent.spawn((
                        Text::new("Components"),
                        TextFont {
                            font_size: 16.0,
                            ..Default::default()
                        },
                        TextColor(Color::WHITE),
                        InspectorEntity,
                    ));
                    
                    // Scrollable component viewer container
                    parent.spawn((
                        Node {
                            width: Val::Percent(100.0),
                            height: Val::Percent(90.0),
                            flex_direction: FlexDirection::Column,
                            overflow: bevy_ui::Overflow::clip(),
                            ..Default::default()
                        },
                        BackgroundColor(Color::srgb(0.08, 0.08, 0.08)),
                        ComponentViewerContainer,
                        InspectorEntity,
                    ));
                });
        })
        .id();

    ui_root
}

/// Creates the inspector UI as an overlay on the main game window.
pub fn create_inspector_overlay(commands: &mut Commands) -> Entity {
    let ui_root = commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(10.0),
                right: Val::Px(10.0),
                width: Val::Px(400.0),
                height: Val::Percent(80.0),
                flex_direction: FlexDirection::Row,
                ..Default::default()
            },
            BackgroundColor(Color::srgba(0.1, 0.1, 0.1, 0.9)), // Semi-transparent
            BorderColor::all(Color::srgb(0.4, 0.4, 0.4)),
            InspectorEntity,
        ))
        .with_children(|parent| {
            // Left pane - Entity list (smaller in overlay mode)
            parent
                .spawn((
                    Node {
                        width: Val::Percent(40.0),
                        height: Val::Percent(100.0),
                        border: UiRect::all(Val::Px(1.0)),
                        padding: UiRect::all(Val::Px(4.0)),
                        flex_direction: FlexDirection::Column,
                        ..Default::default()
                    },
                    BackgroundColor(Color::srgba(0.15, 0.15, 0.15, 0.95)),
                    BorderColor::all(Color::srgb(0.3, 0.3, 0.3)),
                    InspectorEntity,
                ))
                .with_children(|parent| {
                    // Entity list header
                    parent.spawn((
                        Text::new("Entities"),
                        TextFont {
                            font_size: 14.0,
                            ..Default::default()
                        },
                        TextColor(Color::WHITE),
                        InspectorEntity,
                    ));
                    
                    // Scrollable entity list container
                    parent.spawn((
                        Node {
                            width: Val::Percent(100.0),
                            height: Val::Percent(90.0),
                            flex_direction: FlexDirection::Column,
                            overflow: bevy_ui::Overflow::clip_y(),
                            ..Default::default()
                        },
                        BackgroundColor(Color::srgba(0.1, 0.1, 0.1, 0.9)),
                        EntityListContainer,
                        InspectorEntity,
                    ));
                });

            // Right pane - Component viewer
            parent
                .spawn((
                    Node {
                        width: Val::Percent(60.0),
                        height: Val::Percent(100.0),
                        border: UiRect::all(Val::Px(1.0)),
                        padding: UiRect::all(Val::Px(4.0)),
                        flex_direction: FlexDirection::Column,
                        ..Default::default()
                    },
                    BackgroundColor(Color::srgba(0.12, 0.12, 0.12, 0.95)),
                    BorderColor::all(Color::srgb(0.3, 0.3, 0.3)),
                    InspectorEntity,
                ))
                .with_children(|parent| {
                    // Component viewer header
                    parent.spawn((
                        Text::new("Components"),
                        TextFont {
                            font_size: 14.0,
                            ..Default::default()
                        },
                        TextColor(Color::WHITE),
                        InspectorEntity,
                    ));
                    
                    // Scrollable component viewer container
                    parent.spawn((
                        Node {
                            width: Val::Percent(100.0),
                            height: Val::Percent(90.0),
                            flex_direction: FlexDirection::Column,
                            overflow: bevy_ui::Overflow::clip(),
                            ..Default::default()
                        },
                        BackgroundColor(Color::srgba(0.08, 0.08, 0.08, 0.9)),
                        ComponentViewerContainer,
                        InspectorEntity,
                    ));
                });
        })
        .id();

    ui_root
}