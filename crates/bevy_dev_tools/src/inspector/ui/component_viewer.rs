//! Component viewer UI with live data updates

use super::collapsible_section::{
    CollapsibleArrowText, CollapsibleContent, CollapsibleHeader, CollapsibleSection,
};
use super::entity_list::{EntityCache, SelectedEntity};
use crate::inspector::http_client::{ComponentUpdate, HttpRemoteClient};
use crate::widgets::selectable_text::{SelectableText, TextSelectionState};
use bevy_color::Color;
use bevy_ecs::prelude::*;
use bevy_ecs::system::ParamSet;
use bevy_input::prelude::*;
use bevy_log::debug;
use bevy_text::{TextColor, TextFont};
use bevy_time::Time;
use bevy_ui::prelude::*;
use serde_json::Value;
use std::collections::{HashMap, HashSet};

/// Component for the component viewer panel
#[derive(Component)]
pub struct ComponentViewerPanel;

/// Component to track component data that needs live updates
#[derive(Component)]
pub struct ComponentData {
    /// The entity this component data belongs to
    pub entity_id: u32,
    /// The name/type of the component being tracked
    pub component_name: String,
}

/// Resource to cache component data for live updates
#[derive(Resource, Default)]
pub struct ComponentCache {
    /// Currently selected entity ID
    pub current_entity: Option<u32>,
    /// Map of component names to their serialized values
    pub components: HashMap<String, Value>,
    /// Timestamp of the last cache update
    pub last_update: f64,
    /// Track which entity we've built UI for
    pub ui_built_for_entity: Option<u32>,
}

/// Enhanced resource for live component caching with change tracking
#[derive(Resource)]
pub struct LiveComponentCache {
    /// Map of entity IDs to their component states
    pub entity_components: HashMap<u32, HashMap<String, ComponentState>>,
    /// Timestamp of the last update cycle
    pub last_update_time: f64,
    /// Target update rate in seconds (e.g., 30 FPS = 1/30)
    pub update_frequency: f64,
}

impl Default for LiveComponentCache {
    fn default() -> Self {
        Self {
            entity_components: HashMap::new(),
            last_update_time: 0.0,
            update_frequency: 30.0, // 30 FPS by default
        }
    }
}

/// State tracking for individual components with change indicators
#[derive(Debug, Clone)]
pub struct ComponentState {
    /// The current serialized value of the component
    pub current_value: Value,
    /// Timestamp when this component was last changed
    pub last_changed_time: f64,
    /// Visual indicator for the change state
    pub change_indicator: ChangeIndicator,
    /// Previous value for showing diffs
    pub previous_value: Option<Value>,
}

/// Visual change indicators for components
#[derive(Debug, Clone, PartialEq)]
pub enum ChangeIndicator {
    /// Component has not changed
    Unchanged,
    /// Component was recently changed, with duration in seconds to show indicator
    Changed {
        /// How long to show the changed indicator in seconds
        duration: f64,
    },
    /// Component was removed from the entity
    Removed,
    /// Component was newly added to the entity
    Added,
}

// SelectableText and TextSelectionState are now imported from widgets::selectable_text

/// System to update component viewer when entity selection changes
pub fn update_component_viewer(
    mut commands: Commands,
    _http_client: Res<HttpRemoteClient>,
    entity_cache: Res<EntityCache>,
    mut component_cache: ResMut<ComponentCache>,
    selected_entity: Res<SelectedEntity>,
    time: Res<Time>,
    viewer_query: Query<Entity, With<ComponentViewerPanel>>,
) {
    let Ok(viewer_entity) = viewer_query.single() else {
        return;
    };

    // Check if we need to update (entity changed or periodic refresh)
    let entity_changed = component_cache.current_entity != selected_entity.entity_id;
    let ui_needs_rebuild = component_cache.ui_built_for_entity != selected_entity.entity_id;
    let should_refresh = time.elapsed_secs_f64() - component_cache.last_update > 1.0; // Refresh every second for real-time updates

    // Debug when meaningful updates happen
    if entity_changed || should_refresh || ui_needs_rebuild {
        debug!("Component viewer update: entity_changed={}, should_refresh={}, ui_needs_rebuild={}", entity_changed, should_refresh, ui_needs_rebuild);
    }

    if !entity_changed && !should_refresh && !ui_needs_rebuild {
        return;
    }

    // Only update timestamp if we're actually going to rebuild
    component_cache.last_update = time.elapsed_secs_f64();

    if entity_changed {
        component_cache.current_entity = selected_entity.entity_id;
        component_cache.components.clear();
    }

    // Mark that we're about to build UI for this entity
    if let Some(entity_id) = selected_entity.entity_id {
        component_cache.ui_built_for_entity = Some(entity_id);
    } else {
        component_cache.ui_built_for_entity = None;
    }

    if let Some(entity_id) = selected_entity.entity_id {
        // Get component data from entity cache
        let components = if let Some(entity) = entity_cache.entities.get(&entity_id) {
            &entity.components
        } else {
            &HashMap::new()
        };

        spawn_component_sections(&mut commands, viewer_entity, entity_id, components);
    } else {
        // Show empty state
        spawn_empty_state(&mut commands, viewer_entity);
    }
}

/// Component marker for clearing content
#[derive(Component)]
pub struct ComponentViewerContent {
    /// Entity ID this content is associated with
    pub entity_id: u32,
}

/// Spawn component sections for an entity
fn spawn_component_sections(
    commands: &mut Commands,
    parent: Entity,
    entity_id: u32,
    components: &HashMap<String, Value>,
) {
    // Use with_children to properly manage the parent-child relationship
    commands.entity(parent).with_children(|parent| {
        // Header
        parent.spawn((
            ComponentViewerContent { entity_id },
            Text::new(format!("Entity {entity_id} Components")),
            TextFont {
                font_size: 18.0,
                ..Default::default()
            },
            TextColor(Color::srgb(0.9, 0.9, 1.0)),
            Node {
                margin: UiRect::bottom(Val::Px(16.0)),
                ..Default::default()
            },
        ));

        // Scrollable content area
        let scroll_container = parent
            .spawn((
                ComponentViewerContent { entity_id },
                Node {
                    width: Val::Percent(100.0),
                    flex_grow: 1.0,
                    flex_direction: FlexDirection::Column,
                    overflow: Overflow::clip_y(),
                    ..Default::default()
                },
            ))
            .id();

        // Create collapsible sections for each component
        if components.is_empty() {
            // Show empty state when no components are available
            parent.spawn((
                ComponentViewerContent { entity_id },
                Text::new("No components found for this entity."),
                TextFont {
                    font_size: 14.0,
                    ..Default::default()
                },
                TextColor(Color::srgb(0.6, 0.6, 0.6)),
                Node {
                    margin: UiRect::all(Val::Px(16.0)),
                    align_self: AlignSelf::Center,
                    ..Default::default()
                },
            ));
        } else {
            for (component_name, component_value) in components {
                let formatted_data = format_component_value(component_value);
                create_component_section(
                    &mut parent.commands(),
                    scroll_container,
                    entity_id,
                    component_name,
                    &formatted_data,
                );
            }
        }
    });
}

/// System to clean up old component viewer content  
pub fn cleanup_old_component_content(
    mut commands: Commands,
    content_query: Query<(Entity, &ComponentViewerContent)>,
    component_cache: Res<ComponentCache>,
    selected_entity: Res<SelectedEntity>,
    time: Res<Time>,
) {
    // Only clean up if we're going to rebuild the component viewer
    // Use the same logic as update_component_viewer to determine if rebuild is needed
    let entity_changed = component_cache.current_entity != selected_entity.entity_id;
    let ui_needs_rebuild = component_cache.ui_built_for_entity != selected_entity.entity_id;
    let should_refresh = time.elapsed_secs_f64() - component_cache.last_update > 1.0;

    if entity_changed || should_refresh || ui_needs_rebuild {
        // Clean up all existing component viewer content before rebuilding
        // This prevents duplicates when refreshing the same entity
        for (entity, _content) in content_query.iter() {
            commands.entity(entity).despawn();
        }
    }
}

/// Create a collapsible section for a component
/// Get enhanced display info for a component
fn get_component_display_info(component_name: &str) -> (String, String, String) {
    let short_name = component_name.split("::").last().unwrap_or(component_name);

    // Categorize component types - show actual crate names instead of generic names
    let (category, display_name) = if component_name.starts_with("bevy_") {
        // Built-in Bevy components - show actual crate name
        let parts: Vec<&str> = component_name.split("::").collect();
        let crate_name = if !parts.is_empty() {
            parts[0] // Use the actual crate name like "bevy_transform", "bevy_render"
        } else {
            "bevy"
        };
        (crate_name, short_name.to_string())
    } else {
        // Custom components - show as custom
        ("Custom", short_name.to_string())
    };

    (
        category.to_string(),
        display_name,
        component_name.to_string(),
    )
}

fn create_component_section(
    commands: &mut Commands,
    parent: Entity,
    entity_id: u32,
    component_name: &str,
    component_data: &str,
) {
    let (category, display_name, full_path) = get_component_display_info(component_name);

    // Create the section manually to have more control
    let section_entity = commands
        .spawn((
            CollapsibleSection {
                title: display_name.clone(),
                is_expanded: true,
                header_entity: None,
                content_entity: None,
            },
            Node {
                width: Val::Percent(100.0),
                margin: UiRect::bottom(Val::Px(4.0)),
                flex_direction: FlexDirection::Column,
                ..Default::default()
            },
            BackgroundColor(Color::srgb(0.15, 0.15, 0.2)),
            BorderColor::all(Color::srgb(0.3, 0.3, 0.4)),
        ))
        .id();

    commands.entity(parent).add_child(section_entity);

    // Create header
    let header_entity = commands
        .spawn((
            Button,
            Node {
                width: Val::Percent(100.0),
                height: if full_path != display_name {
                    Val::Px(48.0)
                } else {
                    Val::Px(32.0)
                },
                padding: UiRect::all(Val::Px(8.0)),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::FlexStart,
                justify_content: JustifyContent::Center,
                ..Default::default()
            },
            BackgroundColor(Color::srgb(0.2, 0.2, 0.25)),
            CollapsibleHeader { section_entity },
        ))
        .with_children(|parent| {
            // Component name and category
            parent.spawn((
                Text::new(format!("- {display_name} [{category}]")),
                TextFont {
                    font_size: 14.0,
                    ..Default::default()
                },
                TextColor(Color::srgb(0.9, 0.9, 0.6)),
                CollapsibleArrowText {
                    section_entity,
                    text_template: format!("{display_name} [{category}]"),
                },
            ));

            // Full path in smaller text
            if full_path != display_name {
                parent.spawn((
                    Text::new(full_path.clone()),
                    TextFont {
                        font_size: 9.0,
                        ..Default::default()
                    },
                    TextColor(Color::srgb(0.6, 0.6, 0.6)),
                    Node {
                        margin: UiRect::top(Val::Px(1.0)),
                        ..Default::default()
                    },
                ));
            }
        })
        .id();

    // Create content
    let content_entity = commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                padding: UiRect::all(Val::Px(8.0)),
                flex_direction: FlexDirection::Column,
                ..Default::default()
            },
            BackgroundColor(Color::srgb(0.1, 0.1, 0.15)),
            CollapsibleContent { section_entity },
        ))
        .with_children(|parent| {
            parent.spawn((
                Button, // Make it clickable for selection
                Text::new(component_data),
                TextFont {
                    font_size: 11.0,
                    ..Default::default()
                },
                TextColor(Color::srgb(0.8, 0.8, 0.8)),
                Node {
                    width: Val::Percent(100.0),
                    padding: UiRect::all(Val::Px(4.0)),
                    ..Default::default()
                },
                BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.0)),
                ComponentData {
                    entity_id,
                    component_name: component_name.to_string(),
                },
                SelectableText {
                    text_content: component_data.to_string(),
                    is_selected: false,
                    selection_start: 0,
                    selection_end: 0,
                    cursor_position: 0,
                    is_dragging: false,
                },
            ));
        })
        .id();

    // Link everything together
    commands.entity(section_entity).add_child(header_entity);
    commands.entity(section_entity).add_child(content_entity);

    // Update the section with entity references
    commands.entity(section_entity).insert(CollapsibleSection {
        title: display_name.to_string(),
        is_expanded: true,
        header_entity: Some(header_entity),
        content_entity: Some(content_entity),
    });
}

/// Show empty state when no entity is selected
fn spawn_empty_state(commands: &mut Commands, parent: Entity) {
    // For now, don't clear

    commands.entity(parent).with_children(|parent| {
        parent.spawn((
            Text::new(
                "No entity selected\n\nSelect an entity from the list to view its components.",
            ),
            TextFont {
                font_size: 14.0,
                ..Default::default()
            },
            TextColor(Color::srgb(0.6, 0.6, 0.6)),
            Node {
                align_self: AlignSelf::Center,
                justify_self: JustifySelf::Center,
                ..Default::default()
            },
        ));
    });
}

/// Format a JSON value for display
fn format_component_value(value: &Value) -> String {
    match value {
        Value::String(s) => s.clone(),
        _ => serde_json::to_string_pretty(value).unwrap_or_else(|_| "Invalid JSON".to_string()),
    }
}

/// Spawn the component viewer UI
pub fn spawn_component_viewer(commands: &mut Commands, parent: Entity) -> Entity {
    let viewer = commands
        .spawn((
            ComponentViewerPanel,
            Node {
                flex_grow: 1.0, // Fill remaining space
                height: Val::Vh(100.0),
                padding: UiRect::all(Val::Px(16.0)),
                flex_direction: FlexDirection::Column,
                overflow: Overflow::scroll_y(),
                ..Default::default()
            },
            ScrollPosition::default(),
            BackgroundColor(Color::srgb(0.3, 0.2, 0.2)), // More visible color
        ))
        .id();

    commands.entity(parent).add_child(viewer);
    viewer
}

/// System to process live component updates from HTTP client
pub fn process_live_component_updates(
    mut live_cache: ResMut<LiveComponentCache>,
    mut http_client: ResMut<HttpRemoteClient>,
    time: Res<Time>,
    _selected_entity: Res<SelectedEntity>,
) {
    let current_time = time.elapsed_secs_f64();

    // Rate limiting - only process updates at target frequency
    if current_time - live_cache.last_update_time < 1.0 / live_cache.update_frequency {
        return;
    }

    // Process all pending updates from the new component update system
    let updates = http_client.check_component_updates();

    if !updates.is_empty() {
        debug!("Processing {} component updates", updates.len());
    }

    for update in updates {
        process_component_update(&mut live_cache, update, current_time);
    }

    live_cache.last_update_time = current_time;
}

/// Process a single component update and update the live cache
fn process_component_update(
    cache: &mut LiveComponentCache,
    update: ComponentUpdate,
    current_time: f64,
) {
    let entity_components = cache.entity_components.entry(update.entity_id).or_default();

    // Process changed components
    for (component_name, new_value) in update.changed_components {
        let component_state = entity_components
            .entry(component_name.clone())
            .or_insert_with(|| ComponentState {
                current_value: Value::Null,
                last_changed_time: current_time,
                change_indicator: ChangeIndicator::Added,
                previous_value: None,
            });

        // Check if value actually changed
        if component_state.current_value != new_value {
            component_state.previous_value = Some(component_state.current_value.clone());
            component_state.current_value = new_value;
            component_state.last_changed_time = current_time;
            component_state.change_indicator = ChangeIndicator::Changed { duration: 2.0 };
        }
    }

    // Process removed components
    for component_name in update.removed_components {
        if let Some(component_state) = entity_components.get_mut(&component_name) {
            component_state.change_indicator = ChangeIndicator::Removed;
            component_state.last_changed_time = current_time;
        }
    }
}

/// System to cleanup expired change indicators
pub fn cleanup_expired_change_indicators(
    mut live_cache: ResMut<LiveComponentCache>,
    time: Res<Time>,
) {
    let current_time = time.elapsed_secs_f64();

    for (_, entity_components) in live_cache.entity_components.iter_mut() {
        for (_, component_state) in entity_components.iter_mut() {
            if let ChangeIndicator::Changed { duration } = &component_state.change_indicator {
                let age = current_time - component_state.last_changed_time;
                if age > *duration {
                    component_state.change_indicator = ChangeIndicator::Unchanged;
                }
            }
        }
    }
}

/// System to automatically start watching components when entity is selected
pub fn auto_start_component_watching(
    mut http_client: ResMut<HttpRemoteClient>,
    selected_entity: Res<SelectedEntity>,
    entity_cache: Res<EntityCache>,
    tokio_handle: Res<crate::inspector::plugin::TokioRuntimeHandle>,
) {
    if !selected_entity.is_changed() {
        return;
    }

    if let Some(entity_id) = selected_entity.entity_id {
        if let Some(entity) = entity_cache.entities.get(&entity_id) {
            // Start watching all components of the selected entity
            let components: Vec<String> = entity.components.keys().cloned().collect();
            if !components.is_empty() {
                let _ = http_client.start_component_watching(
                    entity_id,
                    components.clone(),
                    &tokio_handle.0,
                );
            }
        }
    }
}

/// System to update component display with live data
pub fn update_live_component_display(
    live_cache: Res<LiveComponentCache>,
    mut component_data_query: Query<(&ComponentData, &mut Text, &mut SelectableText)>,
    time: Res<Time>,
) {
    if live_cache.entity_components.is_empty() {
        return; // No live data to display
    }

    let _current_time = time.elapsed_secs_f64();

    for (component_data, mut text, mut selectable_text) in component_data_query.iter_mut() {
        if let Some(entity_components) = live_cache.entity_components.get(&component_data.entity_id)
        {
            if let Some(component_state) = entity_components.get(&component_data.component_name) {
                // Update the text with the live component value
                let formatted_value = format_component_value(&component_state.current_value);

                // Use clean formatted value without text indicators
                let display_text = formatted_value.clone();

                // Only update if the text actually changed to avoid unnecessary updates
                if text.0 != display_text {
                    text.0 = display_text;
                    selectable_text.text_content = formatted_value;
                }
            }
        }
    }
}

/// System to handle text selection and copying
pub fn handle_text_selection(
    mut queries: ParamSet<(
        Query<
            (
                Entity,
                &Interaction,
                &mut BackgroundColor,
                &mut SelectableText,
                &ComponentData,
            ),
            (Changed<Interaction>, With<Button>),
        >,
        Query<(Entity, &mut BackgroundColor, &mut SelectableText), With<Button>>,
        Query<(Entity, &Interaction, &ComponentData), (With<Button>, With<SelectableText>)>,
    )>,
    mut selection_state: ResMut<TextSelectionState>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mouse_input: Res<ButtonInput<MouseButton>>,
) {
    // Handle text selection on interaction changes - first pass
    let mut clicked_entity: Option<Entity> = None;
    {
        let mut interaction_query = queries.p0();
        for (entity, interaction, _bg_color, mut selectable_text, _component_data) in
            interaction_query.iter_mut()
        {
            if *interaction == Interaction::Pressed {
                clicked_entity = Some(entity);

                // Select all text in clicked element
                selectable_text.is_selected = true;
                selectable_text.selection_start = 0;
                selectable_text.selection_end = selectable_text.text_content.len();
                selectable_text.cursor_position = selectable_text.text_content.len();
                selectable_text.is_dragging = false;

                // Update global selection state
                selection_state.selected_entity = Some(entity);
            }
        }
    }

    // Clear other selections if we clicked something - second pass
    if let Some(clicked_entity) = clicked_entity {
        let mut all_selectable_query = queries.p1();
        for (other_entity, mut other_bg_color, mut other_selectable_text) in
            all_selectable_query.iter_mut()
        {
            if other_entity != clicked_entity {
                other_selectable_text.is_selected = false;
                other_selectable_text.selection_start = 0;
                other_selectable_text.selection_end = 0;
                other_selectable_text.is_dragging = false;
                *other_bg_color = BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.0));
            }
        }
    }

    // Update visual feedback for all selectable text elements - third pass
    // Pre-gather interaction info to avoid conflicting borrows
    let mut hover_entities = HashSet::new();
    {
        let interaction_query = queries.p2();
        for (entity, interaction, _) in interaction_query.iter() {
            if matches!(*interaction, Interaction::Hovered) {
                hover_entities.insert(entity);
            }
        }
    }

    {
        let mut all_selectable_query = queries.p1();
        for (entity, mut bg_color, selectable_text) in all_selectable_query.iter_mut() {
            if selectable_text.is_selected {
                *bg_color = BackgroundColor(Color::srgba(0.2, 0.4, 0.8, 0.3));
            } else if hover_entities.contains(&entity) {
                *bg_color = BackgroundColor(Color::srgba(0.3, 0.3, 0.3, 0.2));
            } else {
                *bg_color = BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.0));
            }
        }
    }

    // Handle copy to clipboard with Ctrl+C
    if (keyboard_input.pressed(KeyCode::ControlLeft)
        || keyboard_input.pressed(KeyCode::ControlRight))
        && keyboard_input.just_pressed(KeyCode::KeyC)
    {
        if let Some(selected_entity) = selection_state.selected_entity {
            // Find the selected text using the all_selectable_query
            let all_selectable_query = queries.p1();
            if let Ok((_, _, selectable_text)) = all_selectable_query.get(selected_entity) {
                if selectable_text.is_selected {
                    let selected_text =
                        if selectable_text.selection_start != selectable_text.selection_end {
                            // Copy only the selected portion
                            let start = selectable_text
                                .selection_start
                                .min(selectable_text.selection_end);
                            let end = selectable_text
                                .selection_start
                                .max(selectable_text.selection_end);
                            selectable_text
                                .text_content
                                .chars()
                                .skip(start)
                                .take(end - start)
                                .collect::<String>()
                        } else {
                            // Copy all text if no selection range
                            selectable_text.text_content.clone()
                        };

                    copy_to_clipboard(&selected_text);
                }
            }
        }
    }

    // Handle Escape key to deselect all
    if keyboard_input.just_pressed(KeyCode::Escape) {
        selection_state.selected_entity = None;

        // Clear all selections
        let mut all_selectable_query = queries.p1();
        for (_, mut bg_color, mut selectable_text) in all_selectable_query.iter_mut() {
            selectable_text.is_selected = false;
            selectable_text.selection_start = 0;
            selectable_text.selection_end = 0;
            selectable_text.is_dragging = false;
            *bg_color = BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.0));
        }
    }

    // Handle clicking outside to deselect
    if mouse_input.just_pressed(MouseButton::Left) {
        // If no text element is currently being interacted with, deselect all
        let interaction_query = queries.p0();
        let any_interaction = interaction_query.iter().any(|(_, interaction, _, _, _)| {
            matches!(*interaction, Interaction::Pressed | Interaction::Hovered)
        });

        if !any_interaction {
            selection_state.selected_entity = None;

            // Clear all selections
            let mut all_selectable_query = queries.p1();
            for (_, mut bg_color, mut selectable_text) in all_selectable_query.iter_mut() {
                selectable_text.is_selected = false;
                selectable_text.selection_start = 0;
                selectable_text.selection_end = 0;
                selectable_text.is_dragging = false;
                *bg_color = BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.0));
            }
        }
    }
}

/// Cross-platform clipboard copy function
fn copy_to_clipboard(text: &str) {
    #[cfg(not(target_arch = "wasm32"))]
    {
        use std::io::Write;
        use std::process::Command;

        #[cfg(target_os = "windows")]
        {
            let mut cmd = Command::new("cmd");
            cmd.args(&["/C", &format!("echo {} | clip", text.replace('\n', "^\n"))]);
            let _ = cmd.output();
        }

        #[cfg(target_os = "macos")]
        {
            let mut cmd = Command::new("pbcopy");
            if let Ok(mut child) = cmd.stdin(std::process::Stdio::piped()).spawn() {
                if let Some(stdin) = child.stdin.as_mut() {
                    let _ = stdin.write_all(text.as_bytes());
                    let _ = child.wait();
                }
            }
        }

        #[cfg(target_os = "linux")]
        {
            // Try xclip first, then xsel as fallback
            let mut cmd = Command::new("xclip");
            cmd.args(&["-selection", "clipboard"]);

            if let Ok(mut child) = cmd.stdin(std::process::Stdio::piped()).spawn() {
                if let Some(stdin) = child.stdin.as_mut() {
                    let _ = stdin.write_all(text.as_bytes());
                    let _ = child.wait();
                }
            } else {
                // Fallback to xsel
                let mut cmd = Command::new("xsel");
                cmd.args(&["--clipboard", "--input"]);
                if let Ok(mut child) = cmd.stdin(std::process::Stdio::piped()).spawn() {
                    if let Some(stdin) = child.stdin.as_mut() {
                        let _ = stdin.write_all(text.as_bytes());
                        let _ = child.wait();
                    }
                }
            }
        }
    }

    #[cfg(target_arch = "wasm32")]
    {
        // Clipboard copy not implemented for WASM target
        let _ = text;
    }
}
