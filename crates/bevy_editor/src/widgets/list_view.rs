use bevy::prelude::*;
use bevy::ui::{Style, UiRect, Val, FlexDirection, AlignItems, JustifyContent};

/// A generic list widget for displaying collections of items
#[derive(Component)]
pub struct ListView<T> {
    pub items: Vec<T>,
    pub selected_index: Option<usize>,
    pub multi_select: bool,
    pub selected_indices: Vec<usize>,
    pub item_height: f32,
    pub show_selection_highlight: bool,
}

impl<T> ListView<T> {
    pub fn new(items: Vec<T>) -> Self {
        Self {
            items,
            selected_index: None,
            multi_select: false,
            selected_indices: Vec::new(),
            item_height: 30.0,
            show_selection_highlight: true,
        }
    }
    
    pub fn with_multi_select(mut self) -> Self {
        self.multi_select = true;
        self
    }
    
    pub fn with_item_height(mut self, height: f32) -> Self {
        self.item_height = height;
        self
    }
    
    pub fn with_selection_highlight(mut self, show: bool) -> Self {
        self.show_selection_highlight = show;
        self
    }
    
    pub fn select_item(&mut self, index: usize) {
        if index < self.items.len() {
            if self.multi_select {
                if !self.selected_indices.contains(&index) {
                    self.selected_indices.push(index);
                }
            } else {
                self.selected_index = Some(index);
                self.selected_indices.clear();
                self.selected_indices.push(index);
            }
        }
    }
    
    pub fn deselect_item(&mut self, index: usize) {
        if self.multi_select {
            self.selected_indices.retain(|&i| i != index);
        }
        if self.selected_index == Some(index) {
            self.selected_index = None;
        }
    }
    
    pub fn clear_selection(&mut self) {
        self.selected_index = None;
        self.selected_indices.clear();
    }
    
    pub fn is_selected(&self, index: usize) -> bool {
        self.selected_indices.contains(&index) || self.selected_index == Some(index)
    }
}

/// Marker component for list items
#[derive(Component)]
pub struct ListItem {
    pub index: usize,
    pub list_entity: Entity,
}

/// Bundle for creating a list view
#[derive(Bundle)]
pub struct ListViewBundle {
    pub node: Node,
    pub style: Style,
    pub background_color: BackgroundColor,
    pub transform: Transform,
    pub global_transform: GlobalTransform,
    pub visibility: Visibility,
    pub inherited_visibility: InheritedVisibility,
    pub view_visibility: ViewVisibility,
    pub z_index: ZIndex,
}

impl Default for ListViewBundle {
    fn default() -> Self {
        Self {
            node: Node::default(),
            style: Style {
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

/// Plugin for list view functionality
pub struct ListViewPlugin;

impl Plugin for ListViewPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (
            handle_list_item_interaction,
            update_list_item_styles,
        ));
    }
}

/// System to handle list item selection
fn handle_list_item_interaction(
    interaction_query: Query<(&Interaction, &ListItem), Changed<Interaction>>,
    mut list_query: Query<&mut ListView<String>>, // Generic over String for now
    keyboard: Res<ButtonInput<KeyCode>>,
) {
    for (interaction, list_item) in &interaction_query {
        if *interaction == Interaction::Pressed {
            if let Ok(mut list_view) = list_query.get_mut(list_item.list_entity) {
                let ctrl_held = keyboard.pressed(KeyCode::ControlLeft) || keyboard.pressed(KeyCode::ControlRight);
                let shift_held = keyboard.pressed(KeyCode::ShiftLeft) || keyboard.pressed(KeyCode::ShiftRight);
                
                if list_view.multi_select && ctrl_held {
                    // Toggle selection
                    if list_view.is_selected(list_item.index) {
                        list_view.deselect_item(list_item.index);
                    } else {
                        list_view.select_item(list_item.index);
                    }
                } else if list_view.multi_select && shift_held {
                    // Range selection
                    if let Some(last_selected) = list_view.selected_index {
                        let start = last_selected.min(list_item.index);
                        let end = last_selected.max(list_item.index);
                        list_view.clear_selection();
                        for i in start..=end {
                            list_view.select_item(i);
                        }
                    } else {
                        list_view.select_item(list_item.index);
                    }
                } else {
                    // Single selection
                    list_view.clear_selection();
                    list_view.select_item(list_item.index);
                }
            }
        }
    }
}

/// System to update list item visual styles based on selection state
fn update_list_item_styles(
    list_query: Query<&ListView<String>, Changed<ListView<String>>>,
    mut item_query: Query<(&ListItem, &mut BackgroundColor)>,
) {
    for list_view in &list_query {
        if !list_view.show_selection_highlight {
            continue;
        }
        
        for (list_item, mut bg_color) in &mut item_query {
            let is_selected = list_view.is_selected(list_item.index);
            let new_color = if is_selected {
                Color::srgb(0.3, 0.4, 0.6) // Selected color
            } else {
                Color::NONE // Default/unselected
            };
            
            *bg_color = BackgroundColor(new_color);
        }
    }
}

/// Helper trait for types that can be displayed in a list
pub trait ListDisplayable {
    fn display_text(&self) -> String;
    fn display_icon(&self) -> Option<Handle<Image>> {
        None
    }
}

impl ListDisplayable for String {
    fn display_text(&self) -> String {
        self.clone()
    }
}

impl ListDisplayable for &str {
    fn display_text(&self) -> String {
        self.to_string()
    }
}

/// Helper function to spawn a list view with items
pub fn spawn_list_view<T: ListDisplayable + Clone + Component>(
    commands: &mut Commands,
    items: Vec<T>,
    list_config: ListView<T>,
) -> Entity {
    let list_entity = commands
        .spawn((
            list_config,
            ListViewBundle::default(),
        ))
        .id();

    // Spawn list items
    for (index, item) in items.iter().enumerate() {
        let item_entity = commands
            .spawn((
                ListItem {
                    index,
                    list_entity,
                },
                item.clone(),
                Button,
                Node {
                    height: Val::Px(30.0), // Default item height
                    padding: UiRect::all(Val::Px(4.0)),
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::FlexStart,
                    ..default()
                },
                BackgroundColor(Color::NONE),
            ))
            .with_children(|parent| {
                // Icon (if available)
                if let Some(icon) = item.display_icon() {
                    parent.spawn((
                        ImageNode::new(icon),
                        Node {
                            width: Val::Px(16.0),
                            height: Val::Px(16.0),
                            margin: UiRect::right(Val::Px(8.0)),
                            ..default()
                        },
                    ));
                }
                
                // Text
                parent.spawn((
                    Text::new(item.display_text()),
                    TextColor(Color::WHITE),
                    TextFont {
                        font_size: 12.0,
                        ..default()
                    },
                ));
            })
            .id();
            
        commands.entity(list_entity).add_child(item_entity);
    }

    list_entity
}

/// Specialized entity list item for the editor
#[derive(Component, Clone)]
pub struct EntityListItem {
    pub entity_id: u32,
    pub name: String,
    pub components: Vec<String>,
    pub children_count: usize,
}

impl ListDisplayable for EntityListItem {
    fn display_text(&self) -> String {
        format!("Entity {} ({})", self.entity_id, self.name)
    }
}

/// Helper function specifically for entity lists
pub fn spawn_entity_list(
    commands: &mut Commands,
    entities: Vec<EntityListItem>,
) -> Entity {
    spawn_list_view(
        commands,
        entities.clone(),
        ListView::new(entities)
            .with_item_height(30.0)
            .with_selection_highlight(true),
    )
}
