//! Virtual scrolling with infinite loading

use bevy::prelude::*;
use bevy::input::mouse::MouseWheel;
use crate::ui::entity_list::{EntityListContainer, EntityListVirtualContent, EntityListVirtualState, EntityCache};
use crate::ui::component_viewer::ComponentViewerPanel;
use crate::http_client::{RemoteEntity, HttpRemoteClient};
use std::collections::HashMap;


/// Marker for virtual scrolling spacers to avoid despawning them
#[derive(Component)]
pub struct VirtualScrollSpacer {
    pub spacer_type: SpacerType,
}

#[derive(Debug, PartialEq)]
pub enum SpacerType {
    Top,
    Bottom,
}

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
            buffer_size: 10, // Extra items above/below viewport
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
    custom_scroll: Res<CustomScrollPosition>,
    virtual_content_query: Query<Entity, With<EntityListVirtualContent>>,
    container_query: Query<&Node, (With<EntityListContainer>, Without<CachedEntityItem>, Without<VirtualScrollSpacer>)>,
    mut cached_items: Query<(&mut Node, &mut CachedEntityItem, &mut Visibility), (With<CachedEntityItem>, Without<VirtualScrollSpacer>, Without<EntityListContainer>)>,
    mut spacer_query: Query<(&mut Node, &VirtualScrollSpacer), (With<VirtualScrollSpacer>, Without<CachedEntityItem>, Without<EntityListContainer>)>,
    time: Res<Time>,
) {
    let Ok(virtual_content) = virtual_content_query.single() else { return; };
    
    // Use our custom scroll position instead of Bevy's limited scroll
    let current_scroll_y = custom_scroll.y;
    
    // Update container height from actual UI
    if let Ok(container_node) = container_query.single() {
        virtual_scroll_state.container_height = match container_node.height {
            Val::Px(h) => h,
            Val::Vh(vh) => 600.0 * vh / 100.0,
            _ => 600.0,
        };
    }
    
    // Check if scroll position changed significantly (more than half an item height)  
    let scroll_changed = (virtual_scroll_state.current_scroll - current_scroll_y).abs() > virtual_scroll_state.item_height * 0.5;
    
    // Use custom scroll position directly (no interpolation needed since we control it)
    virtual_scroll_state.current_scroll = current_scroll_y;
    virtual_scroll_state.target_scroll = current_scroll_y;
    
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
    
    // Only update display if something meaningful changed
    if !entities_changed && !scroll_changed {
        return;
    }
    
    // Calculate which items should be visible in the current viewport
    let items_per_screen = (virtual_scroll_state.container_height / virtual_scroll_state.item_height).ceil() as usize;
    let scroll_offset = (virtual_scroll_state.current_scroll / virtual_scroll_state.item_height) as usize;
    
    // Use buffer to ensure smooth scrolling, but handle boundaries better
    let start_index = scroll_offset.saturating_sub(virtual_scroll_state.buffer_size);
    let mut end_index = (scroll_offset + items_per_screen + virtual_scroll_state.buffer_size)
        .min(virtual_scroll_state.total_entity_count);
    
    // Ensure we always show at least some items when at the bottom
    if end_index == virtual_scroll_state.total_entity_count && start_index >= end_index.saturating_sub(items_per_screen) {
        // We're at the bottom, make sure we show the last screen worth of items
        let adjusted_start = virtual_scroll_state.total_entity_count.saturating_sub(items_per_screen + virtual_scroll_state.buffer_size);
        if adjusted_start < start_index {
            end_index = virtual_scroll_state.total_entity_count;
        }
    }
    
    // Ensure we have a valid range
    if start_index >= end_index && virtual_scroll_state.total_entity_count > 0 {
        // Fallback: show the last screen of items
        let fallback_start = virtual_scroll_state.total_entity_count.saturating_sub(items_per_screen);
        virtual_scroll_state.visible_range = (fallback_start, virtual_scroll_state.total_entity_count);
        println!("Using fallback range: {}-{}", fallback_start, virtual_scroll_state.total_entity_count);
    } else {
        virtual_scroll_state.visible_range = (start_index, end_index);
    }
    
    // Use the finalized visible range for all calculations
    let (final_start_index, final_end_index) = virtual_scroll_state.visible_range;
    
    // Calculate spacer heights to represent non-visible content
    let top_spacer_height = final_start_index as f32 * virtual_scroll_state.item_height;
    let bottom_spacer_height = (virtual_scroll_state.total_entity_count.saturating_sub(final_end_index)) as f32 * virtual_scroll_state.item_height;
    
    // The virtual content height should be set by the sum of spacers and visible items
    // Let's ensure spacers account for the full content height
    
    // Update spacers to maintain proper scroll height
    for (mut node, spacer) in spacer_query.iter_mut() {
        match spacer.spacer_type {
            SpacerType::Top => {
                node.height = Val::Px(top_spacer_height);
            }
            SpacerType::Bottom => {
                node.height = Val::Px(bottom_spacer_height);
            }
        }
    }
    
    // Track which entities are currently visible
    let mut visible_entity_ids = std::collections::HashSet::new();
    
    // First pass: hide all items, then selectively show the ones in range
    for (mut _node, mut cached_item, mut visibility) in cached_items.iter_mut() {
        cached_item.is_visible = false;
        *visibility = Visibility::Hidden;
    }
    
    // Second pass: show and position items in the visible range
    for (mut node, mut cached_item, mut visibility) in cached_items.iter_mut() {
        // Find this entity's index in the sorted list
        let entity_index = virtual_scroll_state.sorted_entity_ids.iter().position(|&id| id == cached_item.entity_id);
        
        if let Some(index) = entity_index {
            if index >= final_start_index && index < final_end_index {
                // Entity should be visible
                cached_item.is_visible = true;
                *visibility = Visibility::Inherited;
                
                // Position relative to current viewport using finalized range
                let relative_index = index - final_start_index;
                let y_pos = relative_index as f32 * virtual_scroll_state.item_height;
                node.top = Val::Px(y_pos);
                cached_item.cached_position = y_pos;
                
                visible_entity_ids.insert(cached_item.entity_id);
            } else {
                // Entity outside visible range
                cached_item.is_visible = false;
                *visibility = Visibility::Hidden;
            }
        } else {
            // Entity no longer exists
            cached_item.is_visible = false;
            *visibility = Visibility::Hidden;
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
                    let relative_index = index - final_start_index;
                    let y_pos = relative_index as f32 * virtual_scroll_state.item_height;
                    
                    let item_entity = super::entity_list::spawn_entity_list_item(&mut parent.commands(), virtual_content, entity);
                    
                    parent.commands().entity(item_entity).insert((
                        CachedEntityItem {
                            entity_id,
                            is_visible: true,
                            cached_position: y_pos,
                        },
                        Node {
                            position_type: PositionType::Relative,
                            top: Val::Px(y_pos),
                            ..default()
                        },
                    ));
                }
            }
        });
    }
    
    // Enhanced debug output when things change
    if entities_changed || scroll_changed || (time.elapsed_secs() as i32) % 2 == 0 && time.delta_secs() < 0.1 {
        let (final_start, final_end) = virtual_scroll_state.visible_range;
        println!("Showing items {}-{} of {} total (scroll: {:.1}px, viewport: {:.1}px)", 
            final_start, final_end, virtual_scroll_state.total_entity_count,
            virtual_scroll_state.current_scroll, virtual_scroll_state.container_height);
        
        // Extra debug for boundary conditions and range changes
        if virtual_scroll_state.current_scroll > virtual_scroll_state.total_content_height - 1000.0 {
            println!("Near bottom: scroll_offset={}, items_per_screen={}, total_entities={}", 
                scroll_offset, items_per_screen, virtual_scroll_state.total_entity_count);
        }
        
        // Debug when there's a significant change in visible range
        if final_start != start_index || final_end != end_index {
            println!("Range adjusted: {}-{} -> {}-{} (scroll: {:.1})", 
                start_index, end_index, final_start, final_end, virtual_scroll_state.current_scroll);
        }
        
        // Debug scroll changes
        if scroll_changed {
            println!("Scroll changed significantly, updating display");
        }
    }
}

/// Custom scroll position resource that bypasses Bevy's built-in scroll limitations
#[derive(Resource, Default)]
pub struct CustomScrollPosition {
    pub y: f32,
    pub max_y: f32,
}

/// Enhanced mouse wheel scrolling that bypasses Bevy's scroll system limitations
pub fn handle_infinite_scroll_input(
    mut mouse_wheel_events: EventReader<MouseWheel>,
    mut custom_scroll: ResMut<CustomScrollPosition>,
    mut entity_scroll_query: Query<&mut ScrollPosition, (With<EntityListContainer>, Without<ComponentViewerPanel>)>,
    mut component_scroll_query: Query<&mut ScrollPosition, (With<ComponentViewerPanel>, Without<EntityListContainer>)>,
    windows: Query<&Window>,
    virtual_scroll_state: Res<VirtualScrollState>,
) {
    // Update max scroll based on current content
    custom_scroll.max_y = (virtual_scroll_state.total_content_height - virtual_scroll_state.container_height).max(0.0);
    
    for event in mouse_wheel_events.read() {
        if let Ok(window) = windows.single() {
            if let Some(cursor_pos) = window.cursor_position() {
                let window_width = window.width();
                
                if cursor_pos.x < window_width * 0.3 {
                    // Use our custom scroll system for infinite scrolling
                    let scroll_delta = event.y * 30.0;
                    custom_scroll.y -= scroll_delta;
                    custom_scroll.y = custom_scroll.y.clamp(0.0, custom_scroll.max_y);
                    
                    // Debug scroll
                    if custom_scroll.y % 100.0 < 30.0 || custom_scroll.y > custom_scroll.max_y - 100.0 {
                        println!("Custom scroll: pos={:.1}, max={:.1}, total_height={:.1}", 
                            custom_scroll.y, custom_scroll.max_y, virtual_scroll_state.total_content_height);
                    }
                    
                    // Sync with Bevy's scroll position for UI consistency (but don't let it limit us)
                    if let Ok(mut scroll_position) = entity_scroll_query.single_mut() {
                        scroll_position.y = custom_scroll.y;
                    }
                } else {
                    // Scroll component viewer with standard system
                    if let Ok(mut scroll_position) = component_scroll_query.single_mut() {
                        scroll_position.y -= event.y * 30.0;
                        scroll_position.y = scroll_position.y.max(0.0);
                    }
                }
            } else {
                // Fallback: use custom scroll system
                let scroll_delta = event.y * 30.0;
                custom_scroll.y -= scroll_delta;
                custom_scroll.y = custom_scroll.y.clamp(0.0, custom_scroll.max_y);
                
                if let Ok(mut scroll_position) = entity_scroll_query.single_mut() {
                    scroll_position.y = custom_scroll.y;
                }
            }
        }
    }
}

/// Trigger loading more entities for infinite scrolling
fn trigger_load_more_entities(
    _http_client: &mut HttpRemoteClient,
    virtual_scroll_state: &mut VirtualScrollState,
) {
    // In a real implementation, this would request more entities from the server
    // For now, we'll simulate it by marking as loading and letting the HTTP system handle it
    virtual_scroll_state.current_page += 1;
    
    // The actual loading would happen in the HTTP client update system
    // This just marks that we want more data
    println!("Requesting page {} ({} entities per page)", 
        virtual_scroll_state.current_page, virtual_scroll_state.page_size);
    
    // Reset loading flag after a short delay (simulated async operation)
    // In real implementation, this would be set to false when new data arrives
}

/// Initialize virtual scrolling with infinite loading
pub fn setup_virtual_scrolling(
    mut commands: Commands,
    mut scroll_query: Query<&mut ScrollPosition, With<EntityListContainer>>,
    virtual_content_query: Query<Entity, With<EntityListVirtualContent>>,
) {
    // Initialize new virtual scroll state
    commands.insert_resource(VirtualScrollState::default());
    commands.insert_resource(EntityListVirtualState::new());
    
    // Reset scroll position to 0 on startup
    if let Ok(mut scroll_position) = scroll_query.single_mut() {
        scroll_position.y = 0.0;
        println!("Reset scroll position to 0");
    }
    
    // Create initial spacers
    if let Ok(virtual_content) = virtual_content_query.single() {
        commands.entity(virtual_content).with_children(|parent| {
            // Top spacer - invisible placeholder
            parent.spawn((
                VirtualScrollSpacer { spacer_type: SpacerType::Top },
                Node {
                    width: Val::Percent(100.0),
                    height: Val::Px(0.0),
                    ..default()
                },
                BackgroundColor(Color::NONE), // Make it invisible
            ));
            
            // Bottom spacer - invisible placeholder  
            parent.spawn((
                VirtualScrollSpacer { spacer_type: SpacerType::Bottom },
                Node {
                    width: Val::Percent(100.0),
                    height: Val::Px(0.0),
                    ..default()
                },
                BackgroundColor(Color::NONE), // Make it invisible
            ));
        });
    }
    
    println!("Initialized virtual scrolling");
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
        // In a real implementation, you'd check if new data has arrived
        // For now, just reset after a delay to prevent stuck state
        static mut LOADING_TIMER: f32 = 0.0;
        unsafe {
            LOADING_TIMER += time.delta_secs();
            if LOADING_TIMER > 2.0 {
                virtual_scroll_state.is_loading_more = false;
                LOADING_TIMER = 0.0;
            }
        }
    }
}