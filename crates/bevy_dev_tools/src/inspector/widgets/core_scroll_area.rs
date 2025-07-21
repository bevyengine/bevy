use bevy::{
    prelude::*,
    ecs::event::BufferedEvent,
    ui::{UiSystems, ScrollPosition, RelativeCursorPosition},
    picking::hover::Hovered,
};
use bevy_core_widgets::{CoreScrollbar, CoreScrollbarThumb, ControlOrientation, CoreScrollbarDragState};

/// Core scroll area component that integrates with Bevy's standard ScrollPosition
/// and bevy_core_widgets scrollbars. This provides a bridge between the editor's
/// custom scroll functionality and Bevy's native UI scrolling.
#[derive(Component)]
pub struct CoreScrollArea {
    /// Scroll sensitivity multiplier for mouse wheel events
    pub scroll_sensitivity: f32,
    /// Unique identifier for this scroll area
    pub scroll_id: u32,
    /// Whether to show scrollbars
    pub show_scrollbars: bool,
}

impl Default for CoreScrollArea {
    fn default() -> Self {
        Self {
            scroll_sensitivity: 20.0,
            scroll_id: rand::random(),
            show_scrollbars: true,
        }
    }
}

/// Bundle for creating a scrollable area with optional scrollbars
#[derive(Bundle)]
pub struct ScrollAreaBundle {
    pub scroll_area: CoreScrollArea,
    pub scroll_position: ScrollPosition,
    pub relative_cursor_position: RelativeCursorPosition,
    pub node: Node,
    pub background_color: BackgroundColor,
    pub border_color: BorderColor,
    pub border_radius: BorderRadius,
    pub transform: Transform,
    pub global_transform: GlobalTransform,
    pub visibility: Visibility,
    pub inherited_visibility: InheritedVisibility,
    pub view_visibility: ViewVisibility,
    pub z_index: ZIndex,
}

impl Default for ScrollAreaBundle {
    fn default() -> Self {
        Self {
            scroll_area: CoreScrollArea::default(),
            scroll_position: ScrollPosition::default(),
            relative_cursor_position: RelativeCursorPosition::default(),
            node: Node {
                overflow: Overflow::scroll(),
                ..default()
            },
            background_color: BackgroundColor::default(),
            border_color: BorderColor::default(),
            border_radius: BorderRadius::default(),
            transform: Transform::default(),
            global_transform: GlobalTransform::default(),
            visibility: Visibility::default(),
            inherited_visibility: InheritedVisibility::default(),
            view_visibility: ViewVisibility::default(),
            z_index: ZIndex::default(),
        }
    }
}

/// Marker component for scrollable content within a CoreScrollArea
#[derive(Component)]
pub struct ScrollContent {
    /// ID of the scroll area this content belongs to
    pub scroll_area_id: u32,
}

/// Event for programmatic scrolling requests
#[derive(Event, BufferedEvent)]
pub struct ScrollToEntityEvent {
    pub scroll_area_entity: Entity,
    pub target_entity: Entity,
}

/// Plugin for core scroll area functionality
pub struct CoreScrollAreaPlugin;

impl Plugin for CoreScrollAreaPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_plugins(bevy_core_widgets::CoreScrollbarPlugin)
            .add_event::<ScrollToEntityEvent>()
            .add_systems(
                Update,
                (
                    handle_scroll_to_entity,
                    spawn_scrollbars_for_scroll_areas,
                    update_scrollbar_thumb_colors,
                ).after(UiSystems::Layout),
            );
    }
}

/// System to handle programmatic scroll-to-entity requests
fn handle_scroll_to_entity(
    mut scroll_events: EventReader<ScrollToEntityEvent>,
    mut scroll_areas: Query<(&CoreScrollArea, &mut ScrollPosition)>,
    nodes: Query<(&ComputedNode, &GlobalTransform)>,
    scroll_content: Query<&Children, With<ScrollContent>>,
) {
    for event in scroll_events.read() {
        if let Ok((_scroll_area, mut scroll_position)) = scroll_areas.get_mut(event.scroll_area_entity) {
            if let Ok(scroll_children) = scroll_content.single() {
                if let Some(target_position) = find_entity_position_in_scroll(
                    event.target_entity,
                    &scroll_children,
                    &nodes,
                ) {
                    // Update scroll position to show the target entity
                    scroll_position.y = target_position.y.max(0.0);
                    info!("Scrolled to entity {:?} at position {:?}", event.target_entity, target_position);
                }
            }
        }
    }
}

/// System to automatically spawn scrollbars for scroll areas that need them
/// This only creates scrollbars for standalone CoreScrollArea components,
/// not for ones that are part of a ScrollView (which manages its own scrollbars)
fn spawn_scrollbars_for_scroll_areas(
    mut commands: Commands,
    scroll_areas: Query<(Entity, &CoreScrollArea), (Added<CoreScrollArea>, Without<CoreScrollbar>)>,
    existing_scrollbars: Query<&CoreScrollbar>,
) {
    for (entity, scroll_area) in scroll_areas.iter() {
        // Check if there's already a scrollbar targeting this entity
        let has_existing_scrollbar = existing_scrollbars.iter().any(|scrollbar| scrollbar.target == entity);
        
        if has_existing_scrollbar {
            continue; // Skip - already has a scrollbar
        }
        
        if scroll_area.show_scrollbars {
            // Add a vertical scrollbar
            let scrollbar = commands.spawn((
                CoreScrollbar::new(entity, ControlOrientation::Vertical, 20.0),
                Node {
                    position_type: PositionType::Absolute,
                    right: Val::Px(2.0), // Small margin from the edge
                    top: Val::Px(0.0),
                    bottom: Val::Px(0.0),
                    width: Val::Px(16.0),
                    ..default()
                },
                BackgroundColor(Color::srgba(0.2, 0.2, 0.2, 0.8)),
            )).with_children(|parent| {
                parent.spawn((
                    CoreScrollbarThumb,
                    Node {
                        position_type: PositionType::Absolute,
                        width: Val::Percent(100.0),
                        ..default()
                    },
                    BackgroundColor(Color::srgba(0.5, 0.5, 0.5, 0.9)),
                    BorderRadius::all(Val::Px(4.0)),
                ));
            }).id();
            
            commands.entity(entity).add_child(scrollbar);
        }
    }
}

/// System to update scrollbar thumb colors based on hover state
fn update_scrollbar_thumb_colors(
    mut q_thumb: Query<
        (&mut BackgroundColor, &Hovered, &CoreScrollbarDragState),
        (
            With<CoreScrollbarThumb>,
            Or<(Changed<Hovered>, Changed<CoreScrollbarDragState>)>,
        ),
    >,
) {
    for (mut thumb_bg, Hovered(is_hovering), drag) in q_thumb.iter_mut() {
        let color: Color = if *is_hovering || drag.dragging {
            // Lighter color when hovering or dragging
            Color::srgba(0.7, 0.7, 0.7, 0.9)
        } else {
            // Default color
            Color::srgba(0.5, 0.5, 0.5, 0.9)
        }.into();

        if thumb_bg.0 != color {
            thumb_bg.0 = color;
        }
    }
}

/// Recursive helper to find an entity's position within scrollable content
fn find_entity_position_in_scroll(
    target: Entity,
    _children: &Children,
    nodes: &Query<(&ComputedNode, &GlobalTransform)>,
) -> Option<Vec2> {
    // Simplified implementation - just return a default position for now
    // This can be improved later when needed
    if let Ok((_node, transform)) = nodes.get(target) {
        Some(transform.translation().truncate())
    } else {
        None
    }
}

impl CoreScrollArea {
    /// Create a new scroll area with a specific ID
    pub fn with_id(scroll_id: u32) -> Self {
        Self {
            scroll_id,
            ..default()
        }
    }
    
    /// Create a new scroll area without scrollbars
    pub fn without_scrollbars() -> Self {
        Self {
            show_scrollbars: false,
            ..default()
        }
    }
    
    /// Set scroll sensitivity
    pub fn with_sensitivity(mut self, sensitivity: f32) -> Self {
        self.scroll_sensitivity = sensitivity;
        self
    }
}
