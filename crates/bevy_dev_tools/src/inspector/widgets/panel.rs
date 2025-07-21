use bevy::prelude::*;
use bevy::ui::{UiRect, Val, FlexDirection, AlignItems, JustifyContent};
use crate::widgets::scrollable_area::{ScrollableArea, ScrollableAreaBundle};

/// A generic panel widget that provides common panel functionality
#[derive(Component, Clone)]
pub struct Panel {
    pub title: String,
    pub collapsible: bool,
    pub collapsed: bool,
    pub resizable: bool,
    pub min_width: f32,
    pub min_height: f32,
}

impl Panel {
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            collapsible: false,
            collapsed: false,
            resizable: false,
            min_width: 100.0,
            min_height: 100.0,
        }
    }
    
    pub fn collapsible(mut self) -> Self {
        self.collapsible = true;
        self
    }
    
    pub fn resizable(mut self) -> Self {
        self.resizable = true;
        self
    }
    
    pub fn with_min_size(mut self, width: f32, height: f32) -> Self {
        self.min_width = width;
        self.min_height = height;
        self
    }
}

/// Bundle for creating a panel with a header and content area
#[derive(Bundle)]
pub struct PanelBundle {
    pub panel: Panel,
    pub node: Node,
    pub background_color: BackgroundColor,
    pub border_color: BorderColor,
    pub transform: Transform,
    pub global_transform: GlobalTransform,
    pub visibility: Visibility,
    pub inherited_visibility: InheritedVisibility,
    pub view_visibility: ViewVisibility,
    pub z_index: ZIndex,
}

impl Default for PanelBundle {
    fn default() -> Self {
        Self {
            panel: Panel::new("Panel"),
            node: Node {
                flex_direction: FlexDirection::Column,
                border: UiRect::all(Val::Px(1.0)),
                ..default()
            },
            background_color: BackgroundColor(Color::srgb(0.1, 0.1, 0.1)),
            border_color: BorderColor::all(Color::srgb(0.3, 0.3, 0.3)),
            transform: Transform::IDENTITY,
            global_transform: GlobalTransform::IDENTITY,
            visibility: Visibility::Inherited,
            inherited_visibility: InheritedVisibility::VISIBLE,
            view_visibility: ViewVisibility::HIDDEN,
            z_index: ZIndex::default(),
        }
    }
}

/// Marker component for panel headers
#[derive(Component)]
pub struct PanelHeader;

/// Marker component for panel content areas
#[derive(Component)]
pub struct PanelContent;

/// Bundle for scrollable panel content
#[derive(Bundle)]
pub struct ScrollablePanelBundle {
    pub panel: Panel,
    pub scrollable: ScrollableArea,
    pub node: Node,
    pub style: Style,
    pub background_color: BackgroundColor,
    pub border_color: BorderColor,
    pub transform: Transform,
    pub global_transform: GlobalTransform,
    pub visibility: Visibility,
    pub inherited_visibility: InheritedVisibility,
    pub view_visibility: ViewVisibility,
    pub z_index: ZIndex,
}

impl Default for ScrollablePanelBundle {
    fn default() -> Self {
        Self {
            panel: Panel::new("Scrollable Panel"),
            scrollable: ScrollableArea::new(),
            node: Node::default(),
            style: Style {
                flex_direction: FlexDirection::Column,
                border: UiRect::all(Val::Px(1.0)),
                overflow: Overflow::clip_y(),
                ..default()
            },
            background_color: BackgroundColor(Color::srgb(0.1, 0.1, 0.1)),
            border_color: BorderColor::all(Color::srgb(0.3, 0.3, 0.3)),
            transform: Transform::IDENTITY,
            global_transform: GlobalTransform::IDENTITY,
            visibility: Visibility::Inherited,
            inherited_visibility: InheritedVisibility::VISIBLE,
            view_visibility: ViewVisibility::HIDDEN,
            z_index: ZIndex::default(),
        }
    }
}

/// Plugin for panel functionality
pub struct PanelPlugin;

impl Plugin for PanelPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (
            handle_panel_collapse,
            update_panel_layout,
        ));
    }
}

/// System to handle panel collapse/expand interactions
fn handle_panel_collapse(
    mut interaction_query: Query<&Interaction, (Changed<Interaction>, With<PanelHeader>)>,
    mut panel_query: Query<&mut Panel>,
    parent_query: Query<&Parent>,
) {
    for interaction in &interaction_query {
        if *interaction == Interaction::Pressed {
            // Find the parent panel and toggle collapse state
            // This is a simplified version - in practice you'd want better
            // association between header and panel
        }
    }
}

/// System to update panel layout based on collapse state
fn update_panel_layout(
    mut panel_query: Query<(&Panel, &mut Style), Changed<Panel>>,
) {
    for (panel, mut style) in &mut panel_query {
        if panel.collapsed {
            style.height = Val::Px(30.0); // Header height only
        } else {
            style.height = Val::Auto; // Full height
        }
    }
}

/// Helper functions for creating common panel types

/// Creates a basic panel with title header and content area
pub fn spawn_panel(
    commands: &mut Commands,
    panel_config: Panel,
    content_spawn_fn: impl FnOnce(&mut ChildBuilder),
) -> Entity {
    commands
        .spawn(PanelBundle {
            panel: panel_config.clone(),
            ..default()
        })
        .with_children(|parent| {
            // Header
            parent
                .spawn((
                    PanelHeader,
                    Node::default(),
                    Style {
                        height: Val::Px(30.0),
                        padding: UiRect::all(Val::Px(8.0)),
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.15, 0.15, 0.15)),
                ))
                .with_children(|header| {
                    header.spawn((
                        Text::new(panel_config.title),
                        TextColor(Color::WHITE),
                        TextFont {
                            font_size: 14.0,
                            ..default()
                        },
                    ));
                });

            // Content area
            parent
                .spawn((
                    PanelContent,
                    Node::default(),
                    Style {
                        flex_grow: 1.0,
                        padding: UiRect::all(Val::Px(8.0)),
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.1, 0.1, 0.1)),
                ))
                .with_children(content_spawn_fn);
        })
        .id()
}

/// Creates a scrollable panel with title header and scrollable content
pub fn spawn_scrollable_panel(
    commands: &mut Commands,
    panel_config: Panel,
    scroll_config: ScrollableArea,
    content_spawn_fn: impl FnOnce(&mut ChildBuilder),
) -> Entity {
    commands
        .spawn(ScrollablePanelBundle {
            panel: panel_config.clone(),
            scrollable: scroll_config,
            ..default()
        })
        .with_children(|parent| {
            // Header
            parent
                .spawn((
                    PanelHeader,
                    Node::default(),
                    Style {
                        height: Val::Px(30.0),
                        padding: UiRect::all(Val::Px(8.0)),
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.15, 0.15, 0.15)),
                ))
                .with_children(|header| {
                    header.spawn((
                        Text::new(panel_config.title),
                        TextColor(Color::WHITE),
                        TextFont {
                            font_size: 14.0,
                            ..default()
                        },
                    ));
                });

            // Scrollable content area
            parent
                .spawn(ScrollableAreaBundle {
                    style: Style {
                        flex_grow: 1.0,
                        padding: UiRect::all(Val::Px(8.0)),
                        overflow: Overflow::clip_y(),
                        ..default()
                    },
                    background_color: BackgroundColor(Color::srgb(0.1, 0.1, 0.1)),
                    ..default()
                })
                .with_children(content_spawn_fn);
        })
        .id()
}
