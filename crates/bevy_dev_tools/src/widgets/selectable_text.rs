//! Selectable text widget with copy/paste support
//!
//! This widget provides text selection and clipboard copy functionality for Bevy UI.
//! It's designed to be reusable and suitable for upstreaming to bevy_ui.

use bevy_ecs::prelude::*;
use bevy_ui::prelude::*;
use bevy_color::Color;
use bevy_input::prelude::*;
use bevy_ui::widget::Text;
use std::collections::HashSet;

/// Component to make text selectable and copyable
#[derive(Component)]
pub struct SelectableText {
    /// The actual text content
    pub text_content: String,
    /// Whether this text is currently selected
    pub is_selected: bool,
    /// Start position of text selection (in characters)
    pub selection_start: usize,
    /// End position of text selection (in characters) 
    pub selection_end: usize,
    /// Current cursor position
    pub cursor_position: usize,
    /// Whether user is currently dragging to select
    pub is_dragging: bool,
}

impl Default for SelectableText {
    fn default() -> Self {
        Self {
            text_content: String::new(),
            is_selected: false,
            selection_start: 0,
            selection_end: 0,
            cursor_position: 0,
            is_dragging: false,
        }
    }
}

impl SelectableText {
    /// Create a new SelectableText component with the given content
    pub fn new(content: impl Into<String>) -> Self {
        let content = content.into();
        Self {
            text_content: content,
            ..Default::default()
        }
    }

    /// Get the currently selected text portion
    pub fn selected_text(&self) -> String {
        if self.selection_start == self.selection_end {
            // No selection range, return full text
            self.text_content.clone()
        } else {
            // Return selected portion
            let start = self.selection_start.min(self.selection_end);
            let end = self.selection_start.max(self.selection_end);
            self.text_content.chars()
                .skip(start)
                .take(end - start)
                .collect()
        }
    }

    /// Select all text
    pub fn select_all(&mut self) {
        self.is_selected = true;
        self.selection_start = 0;
        self.selection_end = self.text_content.len();
        self.cursor_position = self.text_content.len();
        self.is_dragging = false;
    }

    /// Clear selection
    pub fn clear_selection(&mut self) {
        self.is_selected = false;
        self.selection_start = 0;
        self.selection_end = 0;
        self.is_dragging = false;
    }
}

/// Resource to track global text selection state
#[derive(Resource, Default)]
pub struct TextSelectionState {
    /// Currently selected entity (if any)
    pub selected_entity: Option<Entity>,
    /// Whether clipboard functionality is supported
    pub clipboard_support: bool,
}

/// System to handle text selection interactions and keyboard shortcuts
pub fn handle_text_selection(
    mut queries: ParamSet<(
        Query<
            (Entity, &Interaction, &mut BackgroundColor, &mut SelectableText),
            (Changed<Interaction>, With<Button>)
        >,
        Query<(Entity, &mut BackgroundColor, &mut SelectableText), With<Button>>,
        Query<(Entity, &Interaction), (With<Button>, With<SelectableText>)>,
    )>,
    mut selection_state: ResMut<TextSelectionState>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mouse_input: Res<ButtonInput<MouseButton>>,
) {
    // Handle text selection on interaction changes
    let mut clicked_entity: Option<Entity> = None;
    {
        let mut interaction_query = queries.p0();
        for (entity, interaction, _bg_color, mut selectable_text) in interaction_query.iter_mut() {
            match *interaction {
                Interaction::Pressed => {
                    clicked_entity = Some(entity);
                    selectable_text.select_all();
                    selection_state.selected_entity = Some(entity);
                    println!("Selected text: {}", selectable_text.text_content);
                }
                _ => {}
            }
        }
    }

    // Clear other selections if we clicked something
    if let Some(clicked_entity) = clicked_entity {
        let mut all_selectable_query = queries.p1();
        for (other_entity, mut other_bg_color, mut other_selectable_text) in all_selectable_query.iter_mut() {
            if other_entity != clicked_entity {
                other_selectable_text.clear_selection();
                *other_bg_color = BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.0));
            }
        }
    }

    // Update visual feedback for all selectable text elements
    let mut hover_entities = HashSet::new();
    {
        let interaction_query = queries.p2();
        for (entity, interaction) in interaction_query.iter() {
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
    if (keyboard_input.pressed(KeyCode::ControlLeft) || keyboard_input.pressed(KeyCode::ControlRight)) 
        && keyboard_input.just_pressed(KeyCode::KeyC) {
        if let Some(selected_entity) = selection_state.selected_entity {
            let all_selectable_query = queries.p1();
            if let Ok((_, _, selectable_text)) = all_selectable_query.get(selected_entity) {
                if selectable_text.is_selected {
                    let selected_text = selectable_text.selected_text();
                    copy_to_clipboard(&selected_text);
                    println!("📋 Copied to clipboard: {}", selected_text);
                }
            }
        }
    }

    // Handle Escape key to deselect all
    if keyboard_input.just_pressed(KeyCode::Escape) {
        selection_state.selected_entity = None;
        
        let mut all_selectable_query = queries.p1();
        for (_, mut bg_color, mut selectable_text) in all_selectable_query.iter_mut() {
            selectable_text.clear_selection();
            *bg_color = BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.0));
        }
        
        println!("🚫 Cleared all text selections");
    }

    // Handle clicking outside to deselect
    if mouse_input.just_pressed(MouseButton::Left) {
        let interaction_query = queries.p0();
        let any_interaction = interaction_query.iter().any(|(_, interaction, _, _)| {
            matches!(*interaction, Interaction::Pressed | Interaction::Hovered)
        });
        
        if !any_interaction {
            selection_state.selected_entity = None;
            
            let mut all_selectable_query = queries.p1();
            for (_, mut bg_color, mut selectable_text) in all_selectable_query.iter_mut() {
                selectable_text.clear_selection();
                *bg_color = BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.0));
            }
        }
    }
}

/// System to keep SelectableText in sync with Text components
pub fn sync_selectable_text_with_text(
    mut query: Query<(&mut SelectableText, &Text), Changed<Text>>,
) {
    for (mut selectable_text, text) in query.iter_mut() {
        if selectable_text.text_content != text.0 {
            selectable_text.text_content = text.0.clone();
            // Reset selection if text changed
            if selectable_text.is_selected {
                selectable_text.clear_selection();
            }
        }
    }
}

/// Cross-platform clipboard copy function
pub fn copy_to_clipboard(text: &str) {
    #[cfg(not(target_arch = "wasm32"))]
    {
        use std::process::Command;
        use std::io::Write;
        
        #[cfg(target_os = "windows")]
        {
            let mut cmd = Command::new("cmd");
            cmd.args(&["/C", &format!("echo {} | clip", text.replace('\n', "^\n"))]);
            match cmd.output() {
                Ok(_) => println!("✅ Text copied to clipboard (Windows)"),
                Err(e) => println!("❌ Failed to copy text to clipboard (Windows): {}", e),
            }
        }
        
        #[cfg(target_os = "macos")]
        {
            let mut cmd = Command::new("pbcopy");
            match cmd.stdin(std::process::Stdio::piped()).spawn() {
                Ok(mut child) => {
                    if let Some(stdin) = child.stdin.as_mut() {
                        match stdin.write_all(text.as_bytes()) {
                            Ok(_) => {
                                let _ = stdin;
                                match child.wait() {
                                    Ok(_) => println!("✅ Text copied to clipboard (macOS)"),
                                    Err(e) => println!("❌ Failed to wait for pbcopy: {}", e),
                                }
                            }
                            Err(e) => println!("❌ Failed to write to pbcopy: {}", e),
                        }
                    }
                }
                Err(e) => println!("❌ Failed to spawn pbcopy: {}", e),
            }
        }
        
        #[cfg(target_os = "linux")]
        {
            let mut cmd = Command::new("xclip");
            cmd.args(&["-selection", "clipboard"]);
            
            match cmd.stdin(std::process::Stdio::piped()).spawn() {
                Ok(mut child) => {
                    if let Some(stdin) = child.stdin.as_mut() {
                        match stdin.write_all(text.as_bytes()) {
                            Ok(_) => {
                                let _ = stdin;
                                match child.wait() {
                                    Ok(_) => println!("✅ Text copied to clipboard (Linux - xclip)"),
                                    Err(e) => println!("❌ xclip wait failed: {}", e),
                                }
                            }
                            Err(e) => println!("❌ Failed to write to xclip: {}", e),
                        }
                    }
                }
                Err(_) => {
                    // Fallback to xsel
                    let mut cmd = Command::new("xsel");
                    cmd.args(&["--clipboard", "--input"]);
                    match cmd.stdin(std::process::Stdio::piped()).spawn() {
                        Ok(mut child) => {
                            if let Some(stdin) = child.stdin.as_mut() {
                                match stdin.write_all(text.as_bytes()) {
                                    Ok(_) => {
                                        drop(stdin);
                                        match child.wait() {
                                            Ok(_) => println!("✅ Text copied to clipboard (Linux - xsel)"),
                                            Err(e) => println!("❌ xsel wait failed: {}", e),
                                        }
                                    }
                                    Err(e) => println!("❌ Failed to write to xsel: {}", e),
                                }
                            }
                        }
                        Err(e) => println!("❌ No clipboard utility available (xclip/xsel): {}", e),
                    }
                }
            }
        }
    }
    
    #[cfg(target_arch = "wasm32")]
    {
        println!("📋 Clipboard copy not implemented for WASM target: {}", text);
    }
}