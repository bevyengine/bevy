//! Expansion button widget for expandable UI elements

use bevy_ecs::prelude::*;
use bevy_ui::prelude::*;
use crate::inspector::remote::types::ComponentDataFetched;
use crate::inspector::panels::{ComponentDisplayState, EditorState};
use crate::inspector::themes::DarkTheme;

/// Component for expansion button widgets
#[derive(Component)]
pub struct ExpansionButton {
    pub path: String,
    pub is_expanded: bool,
}

/// Handle clicks on expansion buttons
pub fn handle_expansion_clicks(
    mut interaction_query: Query<
        (&Interaction, &mut ExpansionButton, &mut BackgroundColor), 
        (Changed<Interaction>, With<Button>)
    >,
    mut display_state: ResMut<ComponentDisplayState>,
    editor_state: Res<EditorState>,
    mut commands: Commands,
) {
    for (interaction, mut expansion_button, mut bg_color) in &mut interaction_query {
        match *interaction {
            Interaction::Pressed => {
                // Toggle the expansion state
                if display_state.expanded_paths.contains(&expansion_button.path) {
                    display_state.expanded_paths.remove(&expansion_button.path);
                    expansion_button.is_expanded = false;
                } else {
                    display_state.expanded_paths.insert(expansion_button.path.clone());
                    expansion_button.is_expanded = true;
                }
                
                // For local world data, we can trigger a refresh without remote calls
                if let Some(selected_entity_id) = editor_state.selected_entity_id {
                    if let Some(selected_entity) = editor_state.entities.iter().find(|e| e.id == selected_entity_id) {
                        if !selected_entity.full_component_names.is_empty() {
                            // For local world, we'll use a simplified component display
                            let component_data = format!(
                                "Component names for Entity {}:\n\n{}",
                                selected_entity_id,
                                selected_entity.components.join("\n")
                            );
                            commands.trigger(ComponentDataFetched {
                                entity_id: selected_entity_id,
                                component_data,
                            });
                        }
                    }
                }
                
                *bg_color = BackgroundColor(DarkTheme::EXPANSION_BUTTON_PRESSED);
            }
            Interaction::Hovered => {
                *bg_color = BackgroundColor(DarkTheme::EXPANSION_BUTTON_HOVER);
            }
            Interaction::None => {
                *bg_color = BackgroundColor(DarkTheme::EXPANSION_BUTTON_DEFAULT);
            }
        }
    }
}
