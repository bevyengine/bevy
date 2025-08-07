//! High-performance virtual scrolling system for large entity lists.
//!
//! This module provides efficient virtual scrolling that can handle thousands of entities
//! by only rendering the visible items plus a buffer. Key features:
//!
//! - **Performance**: Maintains ~50-100 UI items regardless of total entity count
//! - **Smooth Scrolling**: Frame-rate limiting and velocity throttling prevent black screens
//! - **Adaptive Buffering**: Buffer size increases during fast scrolling for smoother experience
//! - **Custom Scroll Position**: Bypasses Bevy's built-in scroll limitations
//! - **Visual Feedback**: Integrated scrollbar shows position within large lists
//!
//! ## Usage
//!
//! The virtual scrolling is automatically set up when the inspector initializes. The main
//! systems handle:
//!
//! - `update_infinite_scrolling_display`: Updates which entities are visible
//! - `handle_infinite_scroll_input`: Processes mouse wheel input with performance limits
//! - `update_scrollbar_indicator`: Updates the visual scrollbar position

use bevy_ecs::prelude::*;
use bevy_ui::prelude::*;
use bevy_time::Time;
use bevy_window::Window;
use bevy_render::view::Visibility;
use bevy_input::mouse::MouseWheel;
use super::entity_list::{EntityListContainer, EntityListVirtualContent, EntityListVirtualState, EntityCache, ScrollbarThumb, ScrollbarIndicator, SelectionDebounce};
use crate::inspector::http_client::RemoteEntity;
use std::collections::HashMap;



/// Resource to manage infinite scrolling with virtual windowing
#[derive(Resource)]
pub struct VirtualScrollState {
    pub target_scroll: f32,
    pub current_scroll: f32,
    pub scroll_velocity: f32,
    pub container_height: f32,
    pub item_height: f32,
    pub visible_range: (usize, usize),
    pub loaded_entities: HashMap<u32, RemoteEntity>,
    pub sorted_entity_ids: Vec<u32>,
    pub total_entity_count: usize,
    pub total_content_height: f32,
    pub loading_threshold: f32,
    pub is_loading_more: bool,
    pub page_size: usize,
    pub current_page: usize,
    pub buffer_size: usize,
    pub last_update_time: f64,
    pub min_update_interval: f64,
    pub max_scroll_velocity: f32,
    pub pending_scroll_position: Option<f32>,
    pub last_cleanup_time: f64,
    pub cleanup_interval: f64,
    pub loading_timer: f32,
}

impl Default for VirtualScrollState {
    fn default() -> Self {
        Self {
            target_scroll: 0.0,
            current_scroll: 0.0,
            scroll_velocity: 0.0,
            container_height: 600.0,
            item_height: 34.0,
            visible_range: (0, 0),
            loaded_entities: HashMap::new(),
            sorted_entity_ids: Vec::new(),
            total_entity_count: 0,
            total_content_height: 0.0,
            loading_threshold: 200.0,
            is_loading_more: false,
            page_size: 50,
            current_page: 0,
            buffer_size: 5, // Reduced buffer for better virtual scrolling with small lists
            last_update_time: 0.0,
            min_update_interval: 0.020, // Slower updates to prevent black screens
            max_scroll_velocity: 1500.0, // Reduced max velocity
            pending_scroll_position: None,
            last_cleanup_time: 0.0,
            cleanup_interval: 0.1, // Only cleanup every 100ms during fast scrolling
            loading_timer: 0.0,
        }
    }
}

/// Cached entity item component to avoid recreating UI elements
#[derive(Component)]
pub struct CachedEntityItem {
    pub entity_id: u32,
    pub is_visible: bool,
    pub cached_position: f32,
}

/// Infinite scrolling system using custom scroll position to bypass Bevy limitations
pub fn update_infinite_scrolling_display(
    mut commands: Commands,
    entity_cache: Res<EntityCache>,
    mut virtual_scroll_state: ResMut<VirtualScrollState>,
    mut selection_debounce: ResMut<SelectionDebounce>,
    custom_scroll: Res<CustomScrollPosition>,
    mut virtual_content_query: Query<(Entity, &mut Node), (With<EntityListVirtualContent>, Without<EntityListContainer>, Without<CachedEntityItem>)>,
    container_query: Query<&Node, (With<EntityListContainer>, Without<CachedEntityItem>, Without<EntityListVirtualContent>)>,
    mut cached_items: Query<(Entity, &mut Node, &mut CachedEntityItem, &mut Visibility), (With<CachedEntityItem>, Without<EntityListContainer>, Without<EntityListVirtualContent>)>,
    time: Res<Time>,
) {
    let Ok((virtual_content, mut virtual_content_node)) = virtual_content_query.single_mut() else { return; };
    
    // Use our custom scroll position instead of Bevy's limited scroll
    let current_scroll_y = custom_scroll.y;
    
    // Update container height from actual UI - simplified approach like bevy_remote_inspector
    if let Ok(container_node) = container_query.single() {
        let new_container_height = match container_node.height {
            Val::Px(h) => h,
            Val::Vh(vh) => 600.0 * vh / 100.0,
            Val::Percent(p) => virtual_scroll_state.container_height * p / 100.0,
            _ => virtual_scroll_state.container_height,
        };
        
        // Only update if we get a reasonable value and it changed significantly
        if new_container_height > 100.0 && new_container_height < 4000.0 {
            let height_changed = (virtual_scroll_state.container_height - new_container_height).abs() > 10.0;
            if height_changed {
                println!("Container height changed from {:.1}px to {:.1}px", 
                    virtual_scroll_state.container_height, new_container_height);
            }
            virtual_scroll_state.container_height = new_container_height;
        }
    }
    
    // Check if enough time has passed since last update to prevent overwhelming UI
    let current_time = time.elapsed_secs_f64();
    let time_since_last_update = current_time - virtual_scroll_state.last_update_time;
    let can_update_by_time = time_since_last_update >= virtual_scroll_state.min_update_interval;
    
    // Check if scroll position changed significantly (more than half an item height)  
    let scroll_changed = (virtual_scroll_state.current_scroll - current_scroll_y).abs() > virtual_scroll_state.item_height * 0.5;
    
    // Store pending scroll position if we can't update yet
    if scroll_changed && !can_update_by_time {
        virtual_scroll_state.pending_scroll_position = Some(current_scroll_y);
        return;
    }
    
    // Use pending scroll position if available and enough time has passed
    let target_scroll_y = if let Some(pending_pos) = virtual_scroll_state.pending_scroll_position.take() {
        pending_pos
    } else {
        current_scroll_y
    };
    
    // Recalculate scroll_changed with target position
    let scroll_changed = (virtual_scroll_state.current_scroll - target_scroll_y).abs() > virtual_scroll_state.item_height * 0.5;
    
    // Use target scroll position (may be different from current if we had pending updates)
    virtual_scroll_state.current_scroll = target_scroll_y;
    virtual_scroll_state.target_scroll = target_scroll_y;
    virtual_scroll_state.last_update_time = current_time;
    
    // Calculate scroll velocity for cleanup timing
    let scroll_delta = (target_scroll_y - virtual_scroll_state.current_scroll).abs();
    virtual_scroll_state.scroll_velocity = scroll_delta;
    
    // Update entity data when cache changes
    let mut entities_changed = false;
    if entity_cache.is_changed() {
        virtual_scroll_state.loaded_entities.clear();
        virtual_scroll_state.sorted_entity_ids.clear();
        
        // Load all entities and sort them consistently
        for (id, entity) in &entity_cache.entities {
            virtual_scroll_state.loaded_entities.insert(*id, entity.clone());
            virtual_scroll_state.sorted_entity_ids.push(*id);
        }
        
        // Sort by entity ID for consistent ordering
        virtual_scroll_state.sorted_entity_ids.sort();
        virtual_scroll_state.total_entity_count = virtual_scroll_state.sorted_entity_ids.len();
        
        // Calculate total content height for infinite scrolling
        virtual_scroll_state.total_content_height = virtual_scroll_state.total_entity_count as f32 * virtual_scroll_state.item_height;
        
        println!("Loaded {} entities, item_height: {:.1}px, total height: {:.1}px", 
            virtual_scroll_state.total_entity_count, virtual_scroll_state.item_height, virtual_scroll_state.total_content_height);
        println!("Expected max scroll: {:.1}px for container height: {:.1}px", 
            virtual_scroll_state.total_content_height - virtual_scroll_state.container_height, virtual_scroll_state.container_height);
        
        entities_changed = true;
    }
    
    if virtual_scroll_state.total_entity_count == 0 {
        return;
    }
    
    // Only update display if something meaningful changed and we're allowed to update
    if !entities_changed && !scroll_changed {
        return;
    }
    
    // Additional check: if we're updating too frequently, skip this frame
    if !entities_changed && !can_update_by_time {
        return;
    }
    
    // Update selection debounce only when we're doing virtual scrolling updates that affect entity positions
    if scroll_changed {
        selection_debounce.last_virtual_scroll_time = current_time;
        println!("Virtual scroll position change - blocking interactions for {:.1}ms", selection_debounce.scroll_interaction_lockout * 1000.0);
    }
    
    // Calculate which items should be visible in the current viewport
    let items_per_screen = (virtual_scroll_state.container_height / virtual_scroll_state.item_height).ceil() as usize;
    let scroll_offset = (virtual_scroll_state.current_scroll / virtual_scroll_state.item_height) as usize;
    
    // Use adaptive buffer based on scroll velocity - but ensure we don't show everything
    let base_buffer = if virtual_scroll_state.total_entity_count <= items_per_screen * 2 {
        // For small lists, use minimal buffer to ensure virtual scrolling actually works
        2.max(virtual_scroll_state.total_entity_count / 10)
    } else {
        virtual_scroll_state.buffer_size
    };
    
    let adaptive_buffer = if virtual_scroll_state.scroll_velocity > 1000.0 {
        base_buffer * 2 // Larger buffer during fast scrolling (but not 3x)
    } else if virtual_scroll_state.scroll_velocity > 500.0 {
        (base_buffer * 3) / 2 // 1.5x buffer during medium scrolling
    } else {
        base_buffer // Normal buffer for slow scrolling
    };
    
    let start_index = scroll_offset.saturating_sub(adaptive_buffer);
    let mut end_index = (scroll_offset + items_per_screen + adaptive_buffer)
        .min(virtual_scroll_state.total_entity_count);
    
    // IMPORTANT: Always enforce proper virtual scrolling - never show more than necessary
    // Calculate the maximum items we should ever show for virtual scrolling to work
    let max_virtual_items = (items_per_screen + (adaptive_buffer * 2)).min(50); // Cap at 50 items max
    
    // Always clamp to reasonable virtual scrolling window
    let range_size = end_index - start_index;
    if range_size > max_virtual_items || virtual_scroll_state.total_entity_count > items_per_screen * 2 {
        // Force virtual scrolling by limiting visible range
        let window_size = max_virtual_items.min(virtual_scroll_state.total_entity_count);
        let half_window = window_size / 2;
        
        // Center the window around scroll position
        let window_start = if scroll_offset >= half_window {
            scroll_offset - half_window
        } else {
            0
        };
        let window_end = (window_start + window_size).min(virtual_scroll_state.total_entity_count);
        let final_start = if window_end == virtual_scroll_state.total_entity_count && window_end >= window_size {
            window_end - window_size
        } else {
            window_start
        };
        
        virtual_scroll_state.visible_range = (final_start, window_end);
        println!("Virtual scrolling: showing {}-{} (window size: {}) at scroll_offset: {}", 
            final_start, window_end, window_size, scroll_offset);
    } else {
        // Small list - can show all but still use proper positioning
        virtual_scroll_state.visible_range = (start_index, end_index);
    }
    
    // Use the finalized visible range for all calculations
    let (final_start_index, final_end_index) = virtual_scroll_state.visible_range;
    
    
    // Set virtual content container (viewport-relative approach)
    // Inner container: Fill parent height completely
    virtual_content_node.width = Val::Percent(100.0);
    virtual_content_node.height = Val::Percent(100.0); // Fill parent container
    virtual_content_node.position_type = PositionType::Relative;
    virtual_content_node.overflow = Overflow::hidden();
    virtual_content_node.display = Display::Block;
    virtual_content_node.padding = UiRect::ZERO;
    
    // Track which entities are currently visible
    let mut visible_entity_ids = std::collections::HashSet::new();
    
    // First pass: despawn items that are outside the visible range immediately
    let mut entities_to_despawn_immediately = Vec::new();
    for (entity, _node, mut cached_item, mut visibility) in cached_items.iter_mut() {
        let entity_index = virtual_scroll_state.sorted_entity_ids.iter().position(|&id| id == cached_item.entity_id);
        
        if let Some(index) = entity_index {
            if index < final_start_index || index >= final_end_index {
                // Entity is outside visible range, mark for immediate cleanup
                entities_to_despawn_immediately.push(entity);
            } else {
                // Entity is in range, ensure it's visible
                cached_item.is_visible = true;
                *visibility = Visibility::Inherited;
            }
        } else {
            // Entity no longer exists
            entities_to_despawn_immediately.push(entity);
        }
    }
    
    // Immediately despawn items outside visible range
    for entity in entities_to_despawn_immediately {
        commands.entity(entity).despawn();
    }
    
    // Update positioning for remaining visible items (SCROLL-RELATIVE POSITIONING)
    for (_entity, mut node, mut cached_item, _visibility) in cached_items.iter_mut() {
        if cached_item.is_visible {
            let entity_index = virtual_scroll_state.sorted_entity_ids.iter().position(|&id| id == cached_item.entity_id);
            if let Some(index) = entity_index {
                // Calculate absolute position in full content
                let absolute_y_pos = index as f32 * virtual_scroll_state.item_height;
                
                // Adjust position relative to scroll position for viewport positioning
                let viewport_relative_y = absolute_y_pos - virtual_scroll_state.current_scroll;
                
                node.position_type = PositionType::Absolute; // Absolute positioning within relative parent
                node.top = Val::Px(viewport_relative_y); // Position relative to viewport
                node.left = Val::Px(0.0); // Left edge
                node.right = Val::Px(0.0); // Right edge (full width)
                node.height = Val::Px(virtual_scroll_state.item_height); // Fixed height
                node.margin = UiRect::ZERO;
                node.padding = UiRect::all(Val::Px(8.0));
                node.align_items = AlignItems::Center;
                node.justify_content = JustifyContent::FlexStart;
                node.display = Display::Flex;
                node.overflow = Overflow::visible();
                cached_item.cached_position = viewport_relative_y;
                
                visible_entity_ids.insert(cached_item.entity_id);
            }
        }
    }
    
    // Create new UI items for entities that need them
    let entities_needing_items: Vec<u32> = virtual_scroll_state.sorted_entity_ids.iter()
        .enumerate()
        .filter(|(index, entity_id)| {
            *index >= final_start_index && *index < final_end_index && !visible_entity_ids.contains(entity_id)
        })
        .map(|(_, entity_id)| *entity_id)
        .collect();
    
    if !entities_needing_items.is_empty() {
        commands.entity(virtual_content).with_children(|parent| {
            for entity_id in entities_needing_items {
                if let Some(entity) = virtual_scroll_state.loaded_entities.get(&entity_id) {
                    let index = virtual_scroll_state.sorted_entity_ids.iter().position(|&id| id == entity_id).unwrap();
                    
                    // Calculate scroll-relative position for new items too
                    let absolute_y_pos = index as f32 * virtual_scroll_state.item_height;
                    let viewport_relative_y = absolute_y_pos - virtual_scroll_state.current_scroll;
                    
                    let item_entity = super::entity_list::spawn_entity_list_item(&mut parent.commands(), virtual_content, entity);
                    
                    parent.commands().entity(item_entity).insert((
                        CachedEntityItem {
                            entity_id,
                            is_visible: true,
                            cached_position: viewport_relative_y,
                        },
                        Node {
                            position_type: PositionType::Absolute, // Absolute positioning within relative parent
                            top: Val::Px(viewport_relative_y), // Position relative to viewport
                            left: Val::Px(0.0), // Left edge
                            right: Val::Px(0.0), // Right edge (full width)
                            height: Val::Px(virtual_scroll_state.item_height), // Fixed height
                            margin: UiRect::ZERO,
                            padding: UiRect::all(Val::Px(8.0)),
                            align_items: AlignItems::Center,
                            justify_content: JustifyContent::FlexStart,
                            display: Display::Flex,
                            overflow: Overflow::visible(),
                            ..Default::default()
                        },
                    ));
                }
            }
        });
    }
    
    // Note: Cleanup is now handled in the first pass above - we despawn items outside visible range immediately
    
    // Debug output when things change
    if entities_changed || scroll_changed {
        let (final_start, final_end) = virtual_scroll_state.visible_range;
        let total_items = cached_items.iter().count();
        let visible_items = cached_items.iter().filter(|(_, _, cached_item, _)| cached_item.is_visible).count();
        println!("Virtual scroll (scroll-relative positioning): showing {}-{} of {} total (scroll: {:.1}px) [UI items: {}, Visible: {}]", 
            final_start, final_end, virtual_scroll_state.total_entity_count, virtual_scroll_state.current_scroll, total_items, visible_items);
        
        // Show viewport positioning for debugging
        let first_item_absolute_y = final_start_index as f32 * virtual_scroll_state.item_height;
        let first_item_viewport_y = first_item_absolute_y - virtual_scroll_state.current_scroll;
        println!("Scroll-relative: first_item_absolute_y={:.1}px, viewport_relative_y={:.1}px, scroll={:.1}px", 
            first_item_absolute_y, first_item_viewport_y, virtual_scroll_state.current_scroll);
    }
}

/// Custom scroll position resource that bypasses Bevy's built-in scroll limitations
#[derive(Resource, Default)]
pub struct CustomScrollPosition {
    pub y: f32,
    pub max_y: f32,
    pub last_scroll_time: f64,
    pub scroll_debounce_interval: f64,
}

/// Enhanced mouse wheel scrolling that bypasses Bevy's scroll system limitations
pub fn handle_infinite_scroll_input(
    time: Res<Time>,
    virtual_scroll_state: Res<VirtualScrollState>,
    windows: Query<&Window>,
    mut custom_scroll: ResMut<CustomScrollPosition>,
    mut mouse_wheel_events: EventReader<MouseWheel>,
    mut component_scroll_query: Query<&mut ScrollPosition, With<super::component_viewer::ComponentViewerPanel>>,
) {
    // Update max scroll based on current content AND current container height
    let new_max_y = (virtual_scroll_state.total_content_height - virtual_scroll_state.container_height).max(0.0);
    
    // If max_y changed significantly, update it and clamp current position
    if (custom_scroll.max_y - new_max_y).abs() > 10.0 {
        println!("Max scroll updated from {:.1}px to {:.1}px (container: {:.1}px, content: {:.1}px)", 
            custom_scroll.max_y, new_max_y, virtual_scroll_state.container_height, virtual_scroll_state.total_content_height);
        custom_scroll.max_y = new_max_y;
        // Clamp current scroll position to new bounds
        custom_scroll.y = custom_scroll.y.clamp(0.0, custom_scroll.max_y);
    } else {
        custom_scroll.max_y = new_max_y;
    }
    
    let current_time = time.elapsed_secs_f64();
    let mut scroll_events_processed = 0;
    
    for event in mouse_wheel_events.read() {
        // Debounce rapid scroll events to prevent UI overwhelm
        if current_time - custom_scroll.last_scroll_time < custom_scroll.scroll_debounce_interval {
            scroll_events_processed += 1;
            if scroll_events_processed > 1 {
                // Skip processing if too many events in rapid succession
                continue;
            }
        }
        
        custom_scroll.last_scroll_time = current_time;
        
        if let Ok(window) = windows.single() {
            if let Some(cursor_pos) = window.cursor_position() {
                let window_width = window.width();
                
                if cursor_pos.x < window_width * 0.3 {
                    // Use our custom scroll system for infinite scrolling with velocity limiting
                    let mut scroll_delta = event.y * 15.0; // Reduced from 30.0 for more precise control
                    
                    // Apply velocity limiting to prevent overwhelming the UI system
                    let abs_delta = scroll_delta.abs();
                    if abs_delta > virtual_scroll_state.max_scroll_velocity {
                        scroll_delta = scroll_delta.signum() * virtual_scroll_state.max_scroll_velocity;
                    }
                    
                    custom_scroll.y -= scroll_delta;
                    custom_scroll.y = custom_scroll.y.clamp(0.0, custom_scroll.max_y);
                    
                    // Debug scroll
                    if custom_scroll.y % 100.0 < 30.0 || custom_scroll.y > custom_scroll.max_y - 100.0 {
                        println!("Custom scroll: pos={:.1}, max={:.1}, total_height={:.1}", 
                            custom_scroll.y, custom_scroll.max_y, virtual_scroll_state.total_content_height);
                    }
                    
                    // Sync with Bevy's scroll position for UI consistency (but don't let it limit us)
                    // Note: entity_scroll_query removed to avoid additional dependencies
                    // if let Ok(mut scroll_position) = entity_scroll_query.single_mut() {
                    //     scroll_position.y = custom_scroll.y;
                    // }
                } else {
                    // Scroll component viewer with standard system (cursor on right side)
                    if let Ok(mut scroll_position) = component_scroll_query.single_mut() {
                        scroll_position.y -= event.y * 30.0;
                        scroll_position.y = scroll_position.y.max(0.0);
                        println!("Component viewer scroll: {:.1}px", scroll_position.y);
                    }
                }
            } else {
                // Fallback: use custom scroll system with velocity limiting
                let mut scroll_delta = event.y * 15.0; // Reduced from 30.0 for more precise control
                
                // Apply velocity limiting to prevent overwhelming the UI system
                let abs_delta = scroll_delta.abs();
                if abs_delta > virtual_scroll_state.max_scroll_velocity {
                    scroll_delta = scroll_delta.signum() * virtual_scroll_state.max_scroll_velocity;
                }
                
                custom_scroll.y -= scroll_delta;
                custom_scroll.y = custom_scroll.y.clamp(0.0, custom_scroll.max_y);
                
                // Note: entity_scroll_query removed to avoid additional dependencies
                // if let Ok(mut scroll_position) = entity_scroll_query.single_mut() {
                //     scroll_position.y = custom_scroll.y;
                // }
            }
        }
    }
}


/// Initialize virtual scrolling with infinite loading
pub fn setup_virtual_scrolling(
    mut commands: Commands,
    mut scroll_query: Query<&mut ScrollPosition, With<EntityListContainer>>,
    _virtual_content_query: Query<Entity, With<EntityListVirtualContent>>,
) {
    // Initialize new virtual scroll state
    commands.insert_resource(VirtualScrollState::default());
    commands.insert_resource(EntityListVirtualState::new());
    commands.insert_resource(CustomScrollPosition {
        y: 0.0,
        max_y: 0.0,
        last_scroll_time: 0.0,
        scroll_debounce_interval: 0.016, // ~60fps limit for scroll events
    });
    
    // Reset scroll position to 0 on startup
    if let Ok(mut scroll_position) = scroll_query.single_mut() {
        scroll_position.y = 0.0;
        println!("Reset scroll position to 0");
    }
    
    // No spacers needed with absolute positioning approach
    
    println!("Initialized virtual scrolling");
}

/// Update the visual scrollbar to reflect current scroll position
pub fn update_scrollbar_indicator(
    virtual_scroll_state: Res<VirtualScrollState>,
    custom_scroll: Res<CustomScrollPosition>,
    container_query: Query<&Node, (With<EntityListContainer>, Without<ScrollbarThumb>, Without<ScrollbarIndicator>)>,
    indicator_query: Query<&Node, (With<ScrollbarIndicator>, Without<EntityListContainer>, Without<ScrollbarThumb>)>,
    mut thumb_query: Query<&mut Node, (With<ScrollbarThumb>, Without<EntityListContainer>, Without<ScrollbarIndicator>)>,
) {
    // Only update if we have content that needs scrolling
    if virtual_scroll_state.total_content_height <= virtual_scroll_state.container_height {
        // Hide scrollbar when no scrolling is needed
        if let Ok(mut thumb_node) = thumb_query.single_mut() {
            thumb_node.display = Display::None;
        }
        return;
    }
    
    let Ok(container_node) = container_query.single() else { return; };
    let Ok(indicator_node) = indicator_query.single() else { return; };
    let Ok(mut thumb_node) = thumb_query.single_mut() else { return; };
    
    // Show scrollbar
    thumb_node.display = Display::Flex;
    
    // Calculate dimensions - use virtual scroll state container height as primary source
    let container_height = match container_node.height {
        Val::Px(h) => h,
        Val::Percent(p) => virtual_scroll_state.container_height * p / 100.0,
        _ => virtual_scroll_state.container_height, // Use the tracked container height
    };
    
    let indicator_height = match indicator_node.height {
        Val::Px(h) => h,
        Val::Percent(p) => container_height * p / 100.0,
        _ => container_height,
    };
    
    // Calculate thumb size (proportional to visible content ratio)
    let content_ratio = container_height / virtual_scroll_state.total_content_height;
    let thumb_height = (indicator_height * content_ratio).max(20.0).min(indicator_height);
    
    // Calculate thumb position
    let scroll_ratio = if custom_scroll.max_y > 0.0 {
        custom_scroll.y / custom_scroll.max_y
    } else {
        0.0
    };
    let max_thumb_travel = indicator_height - thumb_height;
    let thumb_top = scroll_ratio * max_thumb_travel;
    
    // Update thumb node
    thumb_node.height = Val::Px(thumb_height);
    thumb_node.top = Val::Px(thumb_top);
}

/// System to handle momentum scrolling and infinite loading state management
pub fn update_scroll_momentum(
    mut virtual_scroll_state: ResMut<VirtualScrollState>,
    mut scroll_query: Query<&mut ScrollPosition, With<EntityListContainer>>,
    time: Res<Time>,
) {
    // Apply momentum scrolling
    if virtual_scroll_state.scroll_velocity.abs() > 0.1 {
        if let Ok(mut scroll_position) = scroll_query.single_mut() {
            scroll_position.y -= virtual_scroll_state.scroll_velocity * time.delta_secs();
            
            // Calculate bounds and clamp
            let total_height = virtual_scroll_state.total_entity_count as f32 * virtual_scroll_state.item_height;
            let max_scroll = (total_height - virtual_scroll_state.container_height).max(0.0);
            scroll_position.y = scroll_position.y.clamp(0.0, max_scroll);
            
            // Apply friction
            virtual_scroll_state.scroll_velocity *= 0.95;
        }
    }
    
    // Reset loading flag after some time if no new data arrived
    if virtual_scroll_state.is_loading_more {
        virtual_scroll_state.loading_timer += time.delta_secs();
        if virtual_scroll_state.loading_timer > 2.0 {
            virtual_scroll_state.is_loading_more = false;
            virtual_scroll_state.loading_timer = 0.0;
        }
    }
}