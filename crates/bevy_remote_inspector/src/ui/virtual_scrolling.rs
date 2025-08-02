//! Virtual scrolling systems for handling large datasets efficiently

use bevy::prelude::*;
use bevy::input::mouse::MouseWheel;
use crate::ui::entity_list::{EntityListContainer, EntityListVirtualContent, EntityListVirtualState, EntityCache, EntityListItem};
use crate::ui::component_viewer::ComponentViewerPanel;
use crate::http_client::RemoteEntity;


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

/// System to populate entity list with efficient virtual scrolling
pub fn update_entity_list_display(
    mut commands: Commands,
    entity_cache: Res<EntityCache>,
    virtual_state: Res<EntityListVirtualState>,
    virtual_content_query: Query<Entity, With<EntityListVirtualContent>>,
    scroll_query: Query<&ScrollPosition, With<EntityListContainer>>,
    existing_items: Query<Entity, With<EntityListItem>>,
    mut spacer_query: Query<(&mut Node, &VirtualScrollSpacer), Without<EntityListContainer>>,
) {
    // Only update when entity data actually changes
    if !entity_cache.is_changed() {
        return;
    }
    
    let Ok(virtual_content) = virtual_content_query.single() else { return; };
    let Ok(scroll_position) = scroll_query.single() else { return; };
    
    // Calculate viewport dimensions - use a more reliable height calculation
    let container_height = 500.0; // Fixed height for now to avoid layout issues
    
    let visible_count = ((container_height / virtual_state.item_height).ceil() as usize + 4).min(30); // Smaller buffer
    
    // Get sorted entity list first to calculate bounds
    let mut entities: Vec<&RemoteEntity> = entity_cache.entities.values().collect();
    entities.sort_by_key(|e| e.id);
    
    // Calculate max scroll position based on total entities
    let total_content_height = entities.len() as f32 * virtual_state.item_height;
    let max_scroll = (total_content_height - container_height).max(0.0);
    
    // Clamp scroll position to valid range
    let clamped_scroll_y = scroll_position.y.clamp(0.0, max_scroll);
    let scroll_offset = (clamped_scroll_y / virtual_state.item_height) as usize;
    
    // Calculate visible range
    let start_index = scroll_offset;
    let end_index = (start_index + visible_count).min(entities.len());
    
    // Clear only entity items, keep spacers
    for item_entity in existing_items.iter() {
        commands.entity(item_entity).despawn();
    }
    
    // Calculate spacer heights
    let top_spacer_height = start_index as f32 * virtual_state.item_height;
    let bottom_spacer_height = entities.len().saturating_sub(end_index) as f32 * virtual_state.item_height;
    
    // Update existing spacers or create them if they don't exist
    let mut top_spacer_found = false;
    let mut bottom_spacer_found = false;
    
    for (mut node, spacer) in spacer_query.iter_mut() {
        match spacer.spacer_type {
            SpacerType::Top => {
                node.height = Val::Px(top_spacer_height);
                top_spacer_found = true;
            }
            SpacerType::Bottom => {
                node.height = Val::Px(bottom_spacer_height);
                bottom_spacer_found = true;
            }
        }
    }
    
    // Create missing spacers and visible entities
    commands.entity(virtual_content).with_children(|parent| {
        // Create top spacer if it doesn't exist
        if !top_spacer_found && start_index > 0 {
            parent.spawn((
                VirtualScrollSpacer { spacer_type: SpacerType::Top },
                Node {
                    width: Val::Percent(100.0),
                    height: Val::Px(top_spacer_height),
                    ..default()
                },
            ));
        }
        
        // Spawn visible entities
        for i in start_index..end_index {
            if let Some(entity) = entities.get(i) {
                super::entity_list::spawn_entity_list_item(&mut parent.commands(), virtual_content, entity);
            }
        }
        
        // Create bottom spacer if it doesn't exist
        if !bottom_spacer_found && bottom_spacer_height > 0.0 {
            parent.spawn((
                VirtualScrollSpacer { spacer_type: SpacerType::Bottom },
                Node {
                    width: Val::Percent(100.0),
                    height: Val::Px(bottom_spacer_height),
                    ..default()
                },
            ));
        }
    });
    
    println!("🔍 Virtual scrolling: showing {}-{} of {} entities (scroll: {:.1} -> {:.1})", 
        start_index, end_index, entities.len(), scroll_position.y, clamped_scroll_y);
}

/// System to handle mouse wheel scrolling with proper panel detection and virtual scrolling bounds
pub fn handle_mouse_wheel_scrolling(
    mut mouse_wheel_events: EventReader<MouseWheel>,
    mut entity_scroll_query: Query<&mut ScrollPosition, (With<EntityListContainer>, Without<ComponentViewerPanel>)>,
    mut component_scroll_query: Query<&mut ScrollPosition, (With<ComponentViewerPanel>, Without<EntityListContainer>)>,
    windows: Query<&Window>,
    entity_cache: Res<EntityCache>,
    virtual_state: Res<EntityListVirtualState>,
    container_query: Query<&Node, With<EntityListContainer>>,
) {
    for event in mouse_wheel_events.read() {
        if let Ok(window) = windows.single() {
            if let Some(cursor_pos) = window.cursor_position() {
                let window_width = window.width();
                
                if cursor_pos.x < window_width * 0.3 {
                    // Scroll entity list with bounds checking
                    if let Ok(mut scroll_position) = entity_scroll_query.single_mut() {
                        let scroll_delta = event.y * 50.0;
                        scroll_position.y -= scroll_delta;
                        
                        // Calculate max scroll based on total entities
                        if let Ok(container_node) = container_query.single() {
                            let container_height = match container_node.height {
                                Val::Px(h) => h,
                                _ => 600.0,
                            };
                            let total_height = entity_cache.entities.len() as f32 * virtual_state.item_height;
                            let max_scroll = (total_height - container_height).max(0.0);
                            
                            scroll_position.y = scroll_position.y.clamp(0.0, max_scroll);
                        } else {
                            scroll_position.y = scroll_position.y.max(0.0);
                        }
                    }
                } else {
                    // Scroll component viewer
                    if let Ok(mut scroll_position) = component_scroll_query.single_mut() {
                        scroll_position.y -= event.y * 50.0;
                        scroll_position.y = scroll_position.y.max(0.0);
                    }
                }
            } else {
                // Fallback: scroll entity list if no cursor position
                if let Ok(mut scroll_position) = entity_scroll_query.single_mut() {
                    scroll_position.y -= event.y * 50.0;
                    scroll_position.y = scroll_position.y.max(0.0);
                }
            }
        }
    }
}

/// Initialize virtual scrolling resources and reset scroll position
pub fn setup_virtual_scrolling(
    mut commands: Commands,
    mut scroll_query: Query<&mut ScrollPosition, With<EntityListContainer>>,
) {
    commands.insert_resource(EntityListVirtualState::new());
    
    // Reset scroll position to 0 on startup
    if let Ok(mut scroll_position) = scroll_query.single_mut() {
        scroll_position.y = 0.0;
        println!("🔄 Reset scroll position to 0");
    }
}