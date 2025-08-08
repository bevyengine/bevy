//! High-performance virtual scrolling widget for Bevy UI
//!
//! This widget provides efficient virtual scrolling that can handle thousands of items
//! by only rendering the visible items plus a buffer. It's generic and can work with any content.
//!
//! Key features:
//! - **Performance**: Only renders visible items + buffer
//! - **Smooth Scrolling**: Velocity-based scrolling with frame rate limiting  
//! - **Adaptive Buffering**: Buffer size increases during fast scrolling
//! - **Custom Scroll Position**: Bypasses Bevy's built-in scroll limitations
//! - **Visual Feedback**: Optional scrollbar indicator
//! - **Generic**: Can scroll any type of content

use bevy_color::Color;
use bevy_ecs::prelude::*;
use bevy_input::mouse::MouseWheel;
use bevy_time::Time;
use bevy_ui::prelude::*;
use std::marker::PhantomData;

/// Resource to manage virtual scrolling state
#[derive(Resource)]
pub struct VirtualScrollState<T: Component + Clone> {
    /// Target scroll position (what user wants)
    pub target_scroll: f32,
    /// Current scroll position (smoothly animated towards target)
    pub current_scroll: f32,
    /// Scroll velocity for smooth animation
    pub scroll_velocity: f32,
    /// Height of the scrollable container
    pub container_height: f32,
    /// Height of each item
    pub item_height: f32,
    /// Currently visible range (start_index, end_index)
    pub visible_range: (usize, usize),
    /// All available items
    pub items: Vec<T>,
    /// Total number of items
    pub total_item_count: usize,
    /// Total content height (item_height * item_count)
    pub total_content_height: f32,
    /// Buffer size for rendering extra items
    pub buffer_size: usize,
    /// Last time we updated the display
    pub last_update_time: f64,
    /// Minimum interval between updates (for performance)
    pub min_update_interval: f64,
    /// Maximum scroll velocity
    pub max_scroll_velocity: f32,
    /// Pending scroll position to apply
    pub pending_scroll_position: Option<f32>,
    /// Cleanup interval for removing off-screen items
    pub cleanup_interval: f64,
    /// Last cleanup time
    pub last_cleanup_time: f64,
}

impl<T: Component + Clone> Default for VirtualScrollState<T> {
    fn default() -> Self {
        Self {
            target_scroll: 0.0,
            current_scroll: 0.0,
            scroll_velocity: 0.0,
            container_height: 600.0,
            item_height: 34.0,
            visible_range: (0, 0),
            items: Vec::new(),
            total_item_count: 0,
            total_content_height: 0.0,
            buffer_size: 5,
            last_update_time: 0.0,
            min_update_interval: 0.016, // ~60 FPS
            max_scroll_velocity: 2000.0,
            pending_scroll_position: None,
            cleanup_interval: 0.1,
            last_cleanup_time: 0.0,
        }
    }
}

/// Marker component for virtual scroll containers
#[derive(Component)]
pub struct VirtualScrollContainer<T: Component> {
    phantom: PhantomData<T>,
}

impl<T: Component> Default for VirtualScrollContainer<T> {
    fn default() -> Self {
        Self {
            phantom: PhantomData,
        }
    }
}

/// Marker component for virtual scroll content area
#[derive(Component)]
pub struct VirtualScrollContent<T: Component> {
    phantom: PhantomData<T>,
}

impl<T: Component> Default for VirtualScrollContent<T> {
    fn default() -> Self {
        Self {
            phantom: PhantomData,
        }
    }
}

/// Component for individual virtual scroll items
#[derive(Component)]
pub struct VirtualScrollItem<T: Component + Clone> {
    /// Index in the full item list
    pub index: usize,
    /// The actual item data
    pub data: T,
    /// Whether this item is currently visible
    pub is_visible: bool,
    /// Cached position for performance
    pub cached_position: f32,
}

/// Resource for custom scroll position (bypasses Bevy's ScrollPosition limitations)
#[derive(Resource, Default)]
pub struct CustomScrollPosition {
    /// Y position
    pub y: f32,
    /// Maximum Y position
    pub max_y: f32,
    /// Last scroll time for debouncing
    pub last_scroll_time: f64,
    /// Debounce interval
    pub scroll_debounce_interval: f64,
}

/// Optional scrollbar indicator component
#[derive(Component)]
pub struct ScrollbarIndicator;

/// Scrollbar thumb component
#[derive(Component)]
pub struct ScrollbarThumb;

/// Trait that items must implement to be used with virtual scrolling
pub trait VirtualScrollable: Component + Clone {
    /// Spawn the UI representation of this item
    fn spawn_ui(&self, commands: &mut Commands, parent: Entity, index: usize, item_height: f32);

    /// Get a unique identifier for this item (for efficient updates)
    fn get_id(&self) -> u64;
}

/// System to handle scroll input with performance throttling
pub fn handle_virtual_scroll_input<T: Component + Clone + VirtualScrollable>(
    mut scroll_state: ResMut<VirtualScrollState<T>>,
    mut custom_scroll: ResMut<CustomScrollPosition>,
    mut mouse_wheel_events: EventReader<MouseWheel>,
    time: Res<Time>,
) {
    let current_time = time.elapsed_secs_f64();

    // Debounce scroll events for performance
    if current_time - custom_scroll.last_scroll_time < custom_scroll.scroll_debounce_interval {
        return;
    }

    for event in mouse_wheel_events.read() {
        let scroll_delta = event.y * 100.0; // Scale mouse wheel input

        // Update target scroll position
        let new_target = (scroll_state.target_scroll - scroll_delta).clamp(
            0.0,
            (scroll_state.total_content_height - scroll_state.container_height).max(0.0),
        );

        scroll_state.target_scroll = new_target;
        custom_scroll.last_scroll_time = current_time;

        // Add velocity for smooth scrolling
        scroll_state.scroll_velocity = -scroll_delta * 0.5;
        scroll_state.scroll_velocity = scroll_state.scroll_velocity.clamp(
            -scroll_state.max_scroll_velocity,
            scroll_state.max_scroll_velocity,
        );

        println!(
            "Scroll input: delta={}, target={}, velocity={}",
            scroll_delta, scroll_state.target_scroll, scroll_state.scroll_velocity
        );
    }
}

/// System to update virtual scroll display
pub fn update_virtual_scroll_display<T: Component + Clone + VirtualScrollable>(
    mut commands: Commands,
    mut scroll_state: ResMut<VirtualScrollState<T>>,
    time: Res<Time>,
    _container_query: Query<Entity, With<VirtualScrollContainer<T>>>,
    content_query: Query<Entity, With<VirtualScrollContent<T>>>,
    mut item_query: Query<(Entity, &mut VirtualScrollItem<T>)>,
) {
    let current_time = time.elapsed_secs_f64();

    // Rate limiting - only update at specified interval
    if current_time - scroll_state.last_update_time < scroll_state.min_update_interval {
        return;
    }

    // Update scroll position with smooth animation
    let scroll_diff = scroll_state.target_scroll - scroll_state.current_scroll;
    if scroll_diff.abs() > 1.0 {
        scroll_state.current_scroll += scroll_diff * 0.1; // Smooth animation
        scroll_state.scroll_velocity *= 0.9; // Decay velocity
    } else {
        scroll_state.current_scroll = scroll_state.target_scroll;
        scroll_state.scroll_velocity = 0.0;
    }

    // Calculate which items should be visible
    let scroll_offset = (scroll_state.current_scroll / scroll_state.item_height).floor() as usize;
    let items_per_screen =
        (scroll_state.container_height / scroll_state.item_height).ceil() as usize + 1;
    let adaptive_buffer = (scroll_state.buffer_size as f32
        * (1.0 + scroll_state.scroll_velocity.abs() / 500.0))
        .round() as usize;

    let start_index = scroll_offset.saturating_sub(adaptive_buffer);
    let end_index =
        (scroll_offset + items_per_screen + adaptive_buffer).min(scroll_state.total_item_count);

    let new_range = (start_index, end_index);

    // Only update if range changed significantly
    if new_range != scroll_state.visible_range {
        scroll_state.visible_range = new_range;

        println!(
            "Virtual scroll update: range=({}, {}), scroll={:.1}, items={}",
            start_index, end_index, scroll_state.current_scroll, scroll_state.total_item_count
        );

        if let Ok(content_entity) = content_query.single() {
            update_visible_items(
                &mut commands,
                content_entity,
                &mut scroll_state,
                &mut item_query,
            );
        }
    }

    scroll_state.last_update_time = current_time;
}

/// Update which items are visible in the virtual scroll
fn update_visible_items<T: Component + Clone + VirtualScrollable>(
    commands: &mut Commands,
    content_entity: Entity,
    scroll_state: &mut VirtualScrollState<T>,
    item_query: &mut Query<(Entity, &mut VirtualScrollItem<T>)>,
) {
    let (start_index, end_index) = scroll_state.visible_range;

    // Hide items outside visible range
    for (item_entity, mut item) in item_query.iter_mut() {
        let should_be_visible = item.index >= start_index && item.index < end_index;
        if item.is_visible && !should_be_visible {
            commands.entity(item_entity).despawn();
        }
        item.is_visible = should_be_visible;
    }

    // Spawn new items in visible range
    for index in start_index..end_index {
        if index < scroll_state.items.len() {
            // Check if item already exists
            let exists = item_query
                .iter()
                .any(|(_, item)| item.index == index && item.is_visible);

            if !exists {
                let item_data = scroll_state.items[index].clone();
                let y_position =
                    -(index as f32 * scroll_state.item_height - scroll_state.current_scroll);

                // Spawn the item
                let item_entity = commands
                    .spawn((
                        VirtualScrollItem {
                            index,
                            data: item_data.clone(),
                            is_visible: true,
                            cached_position: y_position,
                        },
                        Node {
                            position_type: PositionType::Absolute,
                            top: Val::Px(y_position),
                            left: Val::Px(0.0),
                            width: Val::Percent(100.0),
                            height: Val::Px(scroll_state.item_height),
                            ..Default::default()
                        },
                    ))
                    .id();

                // Let the item spawn its UI
                item_data.spawn_ui(commands, item_entity, index, scroll_state.item_height);

                // Add to content container
                commands.entity(content_entity).add_child(item_entity);
            }
        }
    }
}

/// System to update scroll momentum and smooth scrolling
pub fn update_scroll_momentum<T: Component + Clone>(
    mut scroll_state: ResMut<VirtualScrollState<T>>,
    time: Res<Time>,
) {
    let dt = time.delta_secs();

    // Apply momentum to target scroll position
    if scroll_state.scroll_velocity.abs() > 0.1 {
        scroll_state.target_scroll += scroll_state.scroll_velocity * dt;
        scroll_state.target_scroll = scroll_state.target_scroll.clamp(
            0.0,
            (scroll_state.total_content_height - scroll_state.container_height).max(0.0),
        );

        // Apply friction
        scroll_state.scroll_velocity *= 0.95;
    } else {
        scroll_state.scroll_velocity = 0.0;
    }
}

/// System to update optional scrollbar indicator
pub fn update_scrollbar_indicator<T: Component + Clone>(
    scroll_state: Res<VirtualScrollState<T>>,
    mut scrollbar_query: Query<&mut Node, (With<ScrollbarThumb>, Without<ScrollbarIndicator>)>,
    scrollbar_container_query: Query<&Node, (With<ScrollbarIndicator>, Without<ScrollbarThumb>)>,
) {
    if let (Ok(mut thumb_style), Ok(container_style)) = (
        scrollbar_query.single_mut(),
        scrollbar_container_query.single(),
    ) {
        let container_height = match container_style.height {
            Val::Px(h) => h,
            _ => 400.0, // Fallback
        };

        if scroll_state.total_content_height > scroll_state.container_height {
            let scroll_percentage = scroll_state.current_scroll
                / (scroll_state.total_content_height - scroll_state.container_height).max(1.0);

            let thumb_height = (scroll_state.container_height / scroll_state.total_content_height
                * container_height)
                .max(20.0);
            let thumb_position = scroll_percentage * (container_height - thumb_height);

            thumb_style.top = Val::Px(thumb_position);
            thumb_style.height = Val::Px(thumb_height);
        }
    }
}

/// Setup system for virtual scrolling
pub fn setup_virtual_scrolling<T: Component + Clone>(mut commands: Commands) {
    commands.init_resource::<VirtualScrollState<T>>();
    commands.init_resource::<CustomScrollPosition>();
}

/// Helper function to spawn a virtual scroll container with optional scrollbar
pub fn spawn_virtual_scroll_container<T: Component>(
    commands: &mut Commands,
    parent: Entity,
    width: Val,
    height: Val,
    with_scrollbar: bool,
) -> (Entity, Entity) {
    let container = commands
        .spawn((
            VirtualScrollContainer::<T>::default(),
            Node {
                width,
                height,
                flex_direction: FlexDirection::Row,
                overflow: Overflow::clip(),
                ..Default::default()
            },
        ))
        .id();

    let content = commands
        .spawn((
            VirtualScrollContent::<T>::default(),
            Node {
                width: if with_scrollbar {
                    Val::Percent(95.0)
                } else {
                    Val::Percent(100.0)
                },
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                position_type: PositionType::Relative,
                ..Default::default()
            },
        ))
        .id();

    commands.entity(parent).add_child(container);
    commands.entity(container).add_child(content);

    // Add scrollbar if requested
    if with_scrollbar {
        let scrollbar = commands
            .spawn((
                ScrollbarIndicator,
                Node {
                    width: Val::Percent(5.0),
                    height: Val::Percent(100.0),
                    ..Default::default()
                },
                BackgroundColor(Color::srgb(0.2, 0.2, 0.2)),
            ))
            .id();

        let thumb = commands
            .spawn((
                ScrollbarThumb,
                Node {
                    width: Val::Percent(100.0),
                    height: Val::Px(50.0),
                    ..Default::default()
                },
                BackgroundColor(Color::srgb(0.5, 0.5, 0.5)),
            ))
            .id();

        commands.entity(container).add_child(scrollbar);
        commands.entity(scrollbar).add_child(thumb);
    }

    (container, content)
}
