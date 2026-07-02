use bevy_app::{Plugin, Update};
use bevy_camera::visibility::Visibility;
use bevy_ecs::{
    component::Component,
    entity::Entity,
    hierarchy::{ChildOf, Children},
    observer::On,
    query::{Changed, With},
    reflect::ReflectComponent,
    system::{Commands, Query, ResMut},
};
use bevy_input_focus::{FocusCause, InputFocus, InputFocusVisible};
use bevy_reflect::{prelude::ReflectDefault, Reflect};
use bevy_scene::prelude::*;
use bevy_ui::{px, widget::Text, Node, Selected};
use bevy_ui_widgets::{ListBox, ValueChange};

use super::{
    FeathersListRow, FeathersListView, FeathersMenu, FeathersMenuButton, FeathersMenuPopup,
};
use crate::{display::caption, rounded_corners::RoundedCorners};

const SELECT_ROW_PX: f32 = 28.0;

/// Select control which spawns a menu popup with a list of string options
/// # Emitted events
/// * [`ValueChange<usize>`](bevy_ui_widgets::ValueChange) when the select option is changed.
#[derive(SceneComponent, Default, Clone)]
#[scene(FeathersSelectProps)]
#[derive(Reflect)]
#[reflect(Component, Default, Clone)]
pub struct FeathersSelect;

/// State of the select control
/// TODO: options currently can't change size after construction I think
#[derive(Component, Default, Clone, Reflect)]
#[reflect(Component, Default, Clone)]
pub struct SelectState {
    /// String options
    pub options: Vec<String>,
    /// Which one is currently selected (ie an index into `options`)
    pub selected: usize,
}

/// Index of each option, inserted into each row
#[derive(Component, Default, Clone, Copy, Reflect)]
#[reflect(Component, Default)]
pub struct SelectOption(pub usize);

/// Marker for the caption which changes with selected item
#[derive(Component, Default, Clone, Reflect)]
#[reflect(Component, Default)]
struct SelectCaption;

/// Props for the control
pub struct FeathersSelectProps {
    /// String options
    pub options: Vec<String>,
    /// Which one starts selected
    pub selected: usize,
    /// Corner roundedness
    pub corners: RoundedCorners,
    /// Maximum visible options before it scrolls
    pub max_visible: usize,
}

impl Default for FeathersSelectProps {
    fn default() -> Self {
        Self {
            options: Vec::new(),
            selected: 0,
            corners: Default::default(),
            max_visible: 8,
        }
    }
}

// Implements as a [`FeathersMenu`] under the hood with a row per option
impl FeathersSelect {
    fn scene(props: FeathersSelectProps) -> impl Scene {
        let len = props.options.len();
        let selected = if len == 0 {
            0
        } else {
            props.selected.min(len - 1)
        };
        let current = props.options.get(selected).cloned().unwrap_or_default();

        let rows: Box<dyn SceneList> = Box::new(
            props
                .options
                .iter()
                .enumerate()
                .map(|(i, label)| {
                    let label = label.clone();
                    bsn! {
                        @FeathersListRow
                        SelectOption(i)
                        Children [ caption(label) ]
                    }
                })
                .collect::<Vec<_>>(),
        );

        let max_visible = props.max_visible.max(1);
        let max_height = px(max_visible as f32 * SELECT_ROW_PX);

        let state = SelectState {
            options: props.options,
            selected,
        };

        bsn! {
            @FeathersMenu
            FeathersSelect
            template_value(state)
            Children [
                (
                    @FeathersMenuButton {
                        @caption: bsn! { caption(current) SelectCaption },
                        @corners: {props.corners},
                    }
                    Node {
                        flex_grow: 1.0,
                    }
                ),
                (
                    @FeathersMenuPopup
                    Children [
                        (
                            @FeathersListView {
                                @rows: {rows}
                            }
                            Node {
                                max_height: {max_height},
                            }
                        )
                    ]
                )
            ]
        }
    }
}

fn sync_select(
    q_changed: Query<(Entity, &SelectState), Changed<SelectState>>,
    q_children: Query<&Children>,
    mut q_caption: Query<&mut Text, With<SelectCaption>>,
    q_option: Query<&SelectOption>,
    mut commands: Commands,
) {
    for (select_ent, state) in q_changed.iter() {
        let current = state
            .options
            .get(state.selected)
            .cloned()
            .unwrap_or_default();
        for child in q_children.iter_descendants(select_ent) {
            if let Ok(mut text) = q_caption.get_mut(child) {
                *text = Text::new(current.clone());
            }
            if let Ok(option) = q_option.get(child) {
                if option.0 == state.selected {
                    commands.entity(child).insert(Selected);
                } else {
                    commands.entity(child).remove::<Selected>();
                }
            }
        }
    }
}

fn on_select_row(
    ev: On<ValueChange<Entity>>,
    q_option: Query<&SelectOption>,
    q_parents: Query<&ChildOf>,
    mut q_state: Query<&mut SelectState>,
    q_children: Query<&Children>,
    q_popup: Query<(), With<FeathersMenuPopup>>,
    mut commands: Commands,
) {
    let row = ev.event().value;
    let Ok(option) = q_option.get(row) else {
        return;
    };

    let mut select_ent = None;
    for ancestor in q_parents.iter_ancestors(row) {
        if q_state.contains(ancestor) {
            select_ent = Some(ancestor);
            break;
        }
    }
    let Some(select_ent) = select_ent else {
        return;
    };

    if let Ok(mut state) = q_state.get_mut(select_ent) {
        if state.selected == option.0 {
            return;
        }
        state.selected = option.0;
    }

    commands.trigger(ValueChange {
        source: select_ent,
        value: option.0,
        is_final: true,
    });

    for child in q_children.iter_descendants(select_ent) {
        if q_popup.contains(child) {
            commands.entity(child).insert(Visibility::Hidden);
        }
    }
}

fn focus_select_popup(
    q_popups: Query<(Entity, &Visibility), (With<FeathersMenuPopup>, Changed<Visibility>)>,
    q_select: Query<(), With<FeathersSelect>>,
    q_listbox: Query<(), With<ListBox>>,
    q_button: Query<(), With<FeathersMenuButton>>,
    q_parents: Query<&ChildOf>,
    q_children: Query<&Children>,
    mut focus: ResMut<InputFocus>,
    mut focus_visible: ResMut<InputFocusVisible>,
) {
    for (popup, visibility) in q_popups.iter() {
        let mut select_ent = None;
        for ancestor in q_parents.iter_ancestors(popup) {
            if q_select.contains(ancestor) {
                select_ent = Some(ancestor);
                break;
            }
        }
        let Some(select_ent) = select_ent else {
            continue;
        };

        if *visibility == Visibility::Visible {
            for descendant in q_children.iter_descendants(popup) {
                if q_listbox.contains(descendant) {
                    focus.set(descendant, FocusCause::Navigated);
                    focus_visible.0 = true;
                    break;
                }
            }
        } else {
            let focus_in_select = focus.get().is_some_and(|focused| {
                focused == select_ent || q_parents.iter_ancestors(focused).any(|a| a == select_ent)
            });
            if focus_in_select {
                for descendant in q_children.iter_descendants(select_ent) {
                    if q_button.contains(descendant) {
                        focus.set(descendant, FocusCause::Navigated);
                        break;
                    }
                }
            }
        }
    }
}

/// Plugin which runs the [`FeathersSelect`] control
pub struct SelectPlugin;

impl Plugin for SelectPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        app.add_systems(Update, (sync_select, focus_select_popup));
        app.add_observer(on_select_row);
    }
}
