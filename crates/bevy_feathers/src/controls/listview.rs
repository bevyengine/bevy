use accesskit::Role;
use bevy_a11y::AccessibilityNode;
use bevy_app::{Plugin, PostUpdate, PreUpdate};
use bevy_ecs::{
    change_detection::DetectChanges,
    component::Component,
    entity::Entity,
    hierarchy::{ChildOf, Children},
    lifecycle::RemovedComponents,
    query::{Added, Changed, Has, Or, With},
    reflect::ReflectComponent,
    schedule::IntoScheduleConfigs as _,
    system::{Commands, Query, Res},
};
use bevy_input_focus::{tab_navigation::TabIndex, InputFocus, InputFocusVisible};
use bevy_picking::{hover::Hovered, PickingSystems};
use bevy_reflect::{prelude::ReflectDefault, Reflect};
use bevy_scene::{bsn, bsn_list, Scene, SceneComponent, SceneList};
use bevy_text::{FontSize, FontWeight};
use bevy_ui::{
    px, AlignItems, BorderRadius, Display, FlexDirection, InteractionDisabled, JustifyContent,
    Node, Overflow, PositionType, Selected, UiRect,
};
use bevy_ui_widgets::{ActiveDescendant, ControlOrientation, ListBox, ListItem, ScrollArea};

use crate::{
    constants::{fonts, size},
    controls::FeathersScrollbar,
    cursor::EntityCursor,
    font_styles::InheritableFont,
    theme::{InheritableThemeTextColor, ThemeBackgroundColor, ThemeBorderColor},
    tokens,
};

/// A container that displays a scrolling list of items
#[derive(SceneComponent, Default, Clone, Reflect)]
#[scene(FeathersListViewProps)]
#[reflect(Component, Clone, Default)]
pub struct FeathersListView;

/// Props used to construct a [`FeathersListView`] scene.
pub struct FeathersListViewProps {
    /// The list of items to be displayed in the list view.
    pub rows: Box<dyn SceneList>,
}

impl Default for FeathersListViewProps {
    fn default() -> Self {
        Self {
            rows: Box::new(bsn_list!()),
        }
    }
}

impl FeathersListView {
    /// Scene function for list view.
    pub fn scene(props: FeathersListViewProps) -> impl Scene {
        bsn! {
            // Outer frame that holds the scrollbar
            Node {
                display: Display::Flex,
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Stretch,
                justify_content: JustifyContent::Start,
                padding: UiRect {
                    right: px(10) // Room for scrollbar
                }
            }
            ListBox
            AccessibilityNode(accesskit::Node::new(Role::ListBox))
            TabIndex(0)
            Children [
                // Inner part that scrolls
                (
                    #inner
                    Node {
                        display: Display::Flex,
                        flex_direction: FlexDirection::Column,
                        align_items: AlignItems::Stretch,
                        justify_content: JustifyContent::Start,
                        overflow: Overflow::scroll_y(),
                    }
                    ScrollArea
                    Children [
                        {props.rows}
                    ]
                ),

                @FeathersScrollbar {
                    @target: #inner,
                    @orientation: {ControlOrientation::Vertical}
                }
                Node {
                    position_type: PositionType::Absolute,
                    right: px(0),
                    top: px(0),
                    bottom: px(0),
                    width: px(6),
                }
            ]
        }
    }
}

/// A selectable row in a list of items
#[derive(SceneComponent, Default, Clone, Reflect)]
#[reflect(Component, Clone, Default)]
pub struct FeathersListRow;

impl FeathersListRow {
    /// Scene function for list row.
    pub fn scene() -> impl Scene {
        bsn! {
            Node {
                min_height: size::ROW_HEIGHT,
                min_width: size::ROW_HEIGHT,
                display: Display::Flex,
                flex_direction: FlexDirection::Row,
                justify_content: JustifyContent::Start,
                align_items: AlignItems::Center,
                padding: UiRect::axes(px(8), px(2)),
            }
            AccessibilityNode(accesskit::Node::new(Role::ListItem))
            InheritableThemeTextColor(tokens::LISTROW_TEXT)
            ThemeBackgroundColor(tokens::LISTROW_BG)
            InheritableFont {
                font: fonts::REGULAR,
                font_size: FontSize::Px(14.0),
                weight: FontWeight::NORMAL,
            }
            Hovered
            ListItem
        }
    }
}

/// Marker for the listrow check mark
#[derive(Component, Default, Clone, Reflect)]
#[reflect(Component, Clone, Default)]
struct ActiveRowOutline;

fn update_listrow_styles(
    q_listrows: Query<
        (
            Entity,
            Has<InteractionDisabled>,
            Has<Selected>,
            &Hovered,
            &ThemeBackgroundColor,
            &InheritableThemeTextColor,
        ),
        (
            With<FeathersListRow>,
            Or<(
                Changed<Hovered>,
                Added<Selected>,
                Added<InteractionDisabled>,
            )>,
        ),
    >,
    mut commands: Commands,
) {
    for (listrow_ent, disabled, selected, hovered, bg_color, font_color) in q_listrows.iter() {
        set_listrow_styles(
            listrow_ent,
            disabled,
            selected,
            hovered.0,
            bg_color,
            font_color,
            &mut commands,
        );
    }
}

fn update_listrow_styles_remove(
    q_listrows: Query<
        (
            Entity,
            Has<InteractionDisabled>,
            Has<Selected>,
            &Hovered,
            &ThemeBackgroundColor,
            &InheritableThemeTextColor,
        ),
        With<FeathersListRow>,
    >,
    mut removed_disabled: RemovedComponents<InteractionDisabled>,
    mut removed_selected: RemovedComponents<Selected>,
    mut commands: Commands,
) {
    removed_disabled
        .read()
        .chain(removed_selected.read())
        .for_each(|ent| {
            if let Ok((listrow_ent, disabled, selected, hovered, bg_color, font_color)) =
                q_listrows.get(ent)
            {
                set_listrow_styles(
                    listrow_ent,
                    disabled,
                    selected,
                    hovered.0,
                    bg_color,
                    font_color,
                    &mut commands,
                );
            }
        });
}

fn set_listrow_styles(
    listrow_ent: Entity,
    disabled: bool,
    selected: bool,
    hovered: bool,
    bg_color: &ThemeBackgroundColor,
    font_color: &InheritableThemeTextColor,
    commands: &mut Commands,
) {
    let outline_bg_token = match (disabled, selected, hovered) {
        (false, true, _) => tokens::LISTROW_BG_SELECTED,
        (false, false, true) => tokens::LISTROW_BG_HOVER,
        _ => tokens::LISTROW_BG,
    };

    let font_color_token = match disabled {
        true => tokens::LISTROW_TEXT_DISABLED,
        false => tokens::LISTROW_TEXT,
    };

    let cursor_shape = match disabled {
        true => bevy_window::SystemCursorIcon::NotAllowed,
        false => bevy_window::SystemCursorIcon::Pointer,
    };

    // Change outline background
    if bg_color.0 != outline_bg_token {
        commands
            .entity(listrow_ent)
            .insert(ThemeBackgroundColor(outline_bg_token));
    }

    // Change font color
    if font_color.0 != font_color_token {
        commands
            .entity(listrow_ent)
            .insert(InheritableThemeTextColor(font_color_token));
    }

    // Change cursor shape
    commands
        .entity(listrow_ent)
        .insert(EntityCursor::System(cursor_shape));
}

fn on_change_focus(
    focus: Res<InputFocus>,
    focus_visible: Res<InputFocusVisible>,
    q_listbox: Query<&ActiveDescendant, With<ListBox>>,
    q_row_outline: Query<(Entity, &ChildOf), With<ActiveRowOutline>>,
    mut commands: Commands,
) {
    if focus.is_changed() || focus_visible.is_changed() {
        if let Some(focus_entity) = focus.get()
            && let Ok(active_descendant) = q_listbox.get(focus_entity)
        {
            // Highlight the active descendant of the current focused listbox, clear all others.
            highlight_active(
                &q_row_outline,
                &mut commands,
                active_descendant.0,
                focus_visible.0,
            );
        } else {
            // Clear all highlights
            highlight_active(&q_row_outline, &mut commands, None, focus_visible.0);
        }
    }
}

fn highlight_active(
    q_row_outline: &Query<'_, '_, (Entity, &ChildOf), With<ActiveRowOutline>>,
    commands: &mut Commands<'_, '_>,
    active_row: Option<Entity>,
    show_highlight: bool,
) {
    // Despawn all active outlines that aren't the current active descendant.
    let mut needs_spawn = show_highlight;
    for (outline_id, ChildOf(outline_parent)) in q_row_outline.iter() {
        let is_active = Some(*outline_parent) == active_row;
        if is_active && show_highlight {
            // If we already have a highlight for the active element, then do nothing.
            needs_spawn = false;
        } else if !is_active || !show_highlight {
            // If this isn't the active highlight, or we are not showing highlights, then
            // despawn any highlight entities.
            commands.entity(outline_id).despawn();
        }
    }

    if let Some(active_item) = active_row
        && needs_spawn
    {
        commands.entity(active_item).with_child((
            Node {
                position_type: PositionType::Absolute,
                left: px(0),
                right: px(0),
                top: px(0),
                bottom: px(0),
                border: UiRect::all(px(2)),
                border_radius: BorderRadius::all(px(3)),
                ..Default::default()
            },
            ThemeBorderColor(tokens::FOCUS_RING),
            ActiveRowOutline,
        ));
    }
}

/// Plugin which registers the systems for updating the listrow styles.
pub struct ListViewPlugin;

impl Plugin for ListViewPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        app.add_systems(
            PreUpdate,
            (update_listrow_styles, update_listrow_styles_remove).in_set(PickingSystems::Last),
        );
        app.add_systems(PostUpdate, on_change_focus);
    }
}
