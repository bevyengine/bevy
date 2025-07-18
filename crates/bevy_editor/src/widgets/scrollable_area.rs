use bevy::prelude::*;
use bevy::input::mouse::MouseWheel;
use bevy::window::Window;
use bevy::ui::{UiRect, Val, FlexDirection, Overflow, ComputedNode};

/// A generic scrollable area widget that can contain any content
/// This widget handles mouse wheel scrolling and maintains scroll position
#[derive(Component, Default)]
pub struct ScrollableArea {
    /// Current scroll offset
    pub scroll_offset: f32,
    /// Maximum scroll distance (calculated dynamically)
    pub max_scroll: f32,
    /// Scroll sensitivity multiplier
    pub scroll_sensitivity: f32,
    /// Content height calculation method
    pub content_height_calc: ContentHeightCalculation,
}

impl ScrollableArea {
    pub fn new() -> Self {
        Self {
            scroll_offset: 0.0,
            max_scroll: 0.0,
            scroll_sensitivity: 15.0,
            content_height_calc: ContentHeightCalculation::ChildrenHeight(40.0),
        }
    }
    
    pub fn with_sensitivity(mut self, sensitivity: f32) -> Self {
        self.scroll_sensitivity = sensitivity;
        self
    }
    
    pub fn with_content_calculation(mut self, calc: ContentHeightCalculation) -> Self {
        self.content_height_calc = calc;
        self
    }
}

/// Different methods for calculating content height
pub enum ContentHeightCalculation {
    /// Calculate based on number of children times item height
    ChildrenHeight(f32),
    /// Calculate based on explicit content height
    ExplicitHeight(f32),
    /// Calculate based on sum of children's actual heights
    ActualChildrenHeights,
}

impl Default for ContentHeightCalculation {
    fn default() -> Self {
        Self::ChildrenHeight(40.0)
    }
}

/// Marker component for entities that should receive scroll events
#[derive(Component)]
pub struct ScrollTarget {
    /// Bounds of the scrollable area in screen space
    pub bounds: Rect,
}

/// Bundle for creating a scrollable area
#[derive(Bundle)]
pub struct ScrollableAreaBundle {
    pub scrollable: ScrollableArea,
    pub target: ScrollTarget,
    pub node: Node,
    pub background_color: BackgroundColor,
    pub transform: Transform,
    pub global_transform: GlobalTransform,
    pub visibility: Visibility,
    pub inherited_visibility: InheritedVisibility,
    pub view_visibility: ViewVisibility,
    pub z_index: ZIndex,
}

impl Default for ScrollableAreaBundle {
    fn default() -> Self {
        Self {
            scrollable: ScrollableArea::new(),
            target: ScrollTarget {
                bounds: Rect::new(0.0, 0.0, 0.0, 0.0),
            },
            node: Node {
                overflow: Overflow::clip_y(),
                flex_direction: FlexDirection::Column,
                ..default()
            },
            background_color: BackgroundColor(Color::NONE),
            transform: Transform::IDENTITY,
            global_transform: GlobalTransform::IDENTITY,
            visibility: Visibility::Inherited,
            inherited_visibility: InheritedVisibility::VISIBLE,
            view_visibility: ViewVisibility::HIDDEN,
            z_index: ZIndex::default(),
        }
    }
}

/// Plugin for scrollable area functionality
pub struct ScrollableAreaPlugin;

impl Plugin for ScrollableAreaPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(PostUpdate, (
            update_scroll_bounds,
            handle_scroll_input,
            apply_scroll_offset,
        ).chain());
    }
}

/// System to update the bounds of scrollable areas based on their computed layout
fn update_scroll_bounds(
    mut scroll_query: Query<(&mut ScrollTarget, &ComputedNode, &GlobalTransform), With<ScrollableArea>>,
    windows: Query<&Window>,
) {
    let Ok(window) = windows.single() else { return };
    let window_height = window.height();

    for (mut target, computed_node, transform) in &mut scroll_query {
        let translation = transform.translation();
        let size = computed_node.size;
        
        // Convert UI coordinates to screen coordinates
        target.bounds = Rect::new(
            translation.x,
            window_height - translation.y - size.y,
            translation.x + size.x,
            window_height - translation.y,
        );
    }
}

/// System to handle mouse wheel scroll input for scrollable areas
fn handle_scroll_input(
    mut scroll_events: EventReader<MouseWheel>,
    mut scroll_query: Query<(&mut ScrollableArea, &ScrollTarget)>,
    windows: Query<&Window>,
) {
    if scroll_events.is_empty() {
        return;
    }

    let Ok(window) = windows.single() else {
        scroll_events.clear();
        return;
    };

    let Some(cursor_position) = window.cursor_position() else {
        scroll_events.clear();
        return;
    };

    // Find which scrollable area the cursor is over
    if let Some((mut scrollable, _)) = scroll_query
        .iter_mut()
        .find(|(_, target)| target.bounds.contains(cursor_position))
    {
        for scroll_event in scroll_events.read() {
            let scroll_delta = scroll_event.y * scrollable.scroll_sensitivity;
            scrollable.scroll_offset = (scrollable.scroll_offset - scroll_delta)
                .clamp(-scrollable.max_scroll, 0.0);
        }
    }
    
    scroll_events.clear();
}

/// System to apply scroll offset to the content within scrollable areas
fn apply_scroll_offset(
    mut scroll_query: Query<(&mut ScrollableArea, &Children), Changed<ScrollableArea>>,
    mut content_query: Query<&mut Node>,
    children_query: Query<&Children>,
) {
    for (mut scrollable, children) in &mut scroll_query {
        // Calculate content height based on the chosen method
        let content_height = match scrollable.content_height_calc {
            ContentHeightCalculation::ChildrenHeight(item_height) => {
                children.len() as f32 * item_height
            },
            ContentHeightCalculation::ExplicitHeight(height) => height,
            ContentHeightCalculation::ActualChildrenHeights => {
                // This would require walking the entire hierarchy and summing actual heights
                // For now, fall back to estimated height
                children.len() as f32 * 25.0
            },
        };

        // Update max scroll (assuming container height is calculated elsewhere)
        // This is a simplified version - in a real implementation you'd want to
        // get the actual container height from the computed layout
        let container_height = 400.0; // This should come from the actual container
        scrollable.max_scroll = (content_height - container_height).max(0.0);

        // Apply scroll offset to the first child (content container)
        if let Some(&first_child) = children.first() {
            if let Ok(mut node) = content_query.get_mut(first_child) {
                node.margin.top = Val::Px(scrollable.scroll_offset);
            }
        }
    }
}

/// Helper function to spawn a scrollable area with content
pub fn spawn_scrollable_area(
    commands: &mut Commands,
    content_bundle: impl Bundle,
    scrollable_config: ScrollableArea,
) -> Entity {
    commands
        .spawn(ScrollableAreaBundle {
            scrollable: scrollable_config,
            ..default()
        })
        .with_children(|parent| {
            parent.spawn(content_bundle);
        })
        .id()
}
