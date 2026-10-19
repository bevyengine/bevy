use bevy_app::{Plugin, PostUpdate};
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
    template::{EntityTemplate, OptionTemplate},
};
use bevy_input_focus::{InputFocus, InputFocusVisible};
use bevy_math::Rot2;
use bevy_picking::{cursor::EntityCursor, hover::Hovered};
use bevy_reflect::{prelude::ReflectDefault, Reflect};
use bevy_scene::{bsn, bsn_list, Scene, SceneComponent, SceneList};
use bevy_text::{FontSize, FontWeight};
use bevy_ui::{
    px, AlignItems, Display, Expandable, Expanded, FlexDirection, InteractionDisabled,
    JustifyContent, Node, Outline, Overflow, PositionType, Selected, UiRect, UiSystems,
    UiTransform,
};
use bevy_ui_widgets::{
    ControlOrientation, MenuFocusSystem, ScrollArea, SelectedTreeItem, TreeItem, TreeItemChildren,
    TreeItemToggle, TreeView,
};

use crate::{
    constants::{fonts, icons, size},
    controls::{FeathersScrollbar, ScrollbarGutter},
    display::icon,
    font_styles::InheritableFont,
    theme::{InheritableThemeTextColor, SurfaceLevel, ThemeBackgroundColor, UiTheme},
    tokens,
};

const INDENT_STEP: f32 = 14.0;
const ROW_PADDING: f32 = 8.0;

/// A container that displays a hierarchy of expandable rows.
///
/// A more complete explanation of how to control this widget can be found in the documentation
/// for [`TreeView`] and [`bevy_ui_widgets`].
#[derive(SceneComponent, Default, Clone, Reflect)]
#[scene(FeathersTreeViewProps)]
#[reflect(Component, Clone, Default)]
pub struct FeathersTreeView;

/// Props used to construct a [`FeathersTreeView`] scene.
pub struct FeathersTreeViewProps {
    /// The initially selected row of the tree.
    pub selected: OptionTemplate<EntityTemplate>,
    /// The top-level rows of the tree.
    pub rows: Box<dyn SceneList>,
}

impl Default for FeathersTreeViewProps {
    fn default() -> Self {
        Self {
            selected: OptionTemplate::None,
            rows: Box::new(bsn_list! {}),
        }
    }
}

impl FeathersTreeView {
    /// Scene function for tree view.
    pub fn scene(props: FeathersTreeViewProps) -> impl Scene {
        bsn! {
            Node {
                display: Display::Flex,
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Stretch,
                justify_content: JustifyContent::Start,
                padding: UiRect {
                    top: px(2),
                    bottom: px(2),
                    left: px(2),
                    right: px(14)
                },
            }
            ScrollbarGutter(px(14))
            TreeView
            SelectedTreeItem({props.selected})
            Children [
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
                --
                @FeathersScrollbar {
                    @target: #inner,
                    @orientation: {ControlOrientation::Vertical}
                }
                Node {
                    position_type: PositionType::Absolute,
                    right: px(4),
                    top: px(0),
                    bottom: px(0),
                    width: px(6),
                }
            ]
        }
    }
}

/// A row of a [`FeathersTreeView`], made of a header line and a container of child rows.
#[derive(SceneComponent, Default, Clone, Reflect)]
#[scene(FeathersTreeItemProps)]
#[reflect(Component, Clone, Default)]
pub struct FeathersTreeItem;

/// Props used to construct a [`FeathersTreeItem`] scene.
pub struct FeathersTreeItemProps {
    /// Content shown on the header line, after the disclosure toggle.
    pub label: Box<dyn SceneList>,
    /// Child rows, shown while the row is expanded.
    pub children: Box<dyn SceneList>,
    /// Whether the row can be expanded.
    /// Set this for rows whose child rows are populated later, such as on first expand.
    pub expandable: bool,
}

impl Default for FeathersTreeItemProps {
    fn default() -> Self {
        Self {
            label: Box::new(bsn_list! {}),
            children: Box::new(bsn_list! {}),
            expandable: false,
        }
    }
}

impl FeathersTreeItem {
    /// Scene function for tree row.
    pub fn scene(props: FeathersTreeItemProps) -> impl Scene {
        bsn! {
            Node {
                display: Display::Flex,
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Stretch,
            }
            TreeItem {
                expandable: {props.expandable},
            }
            Children [
                FeathersTreeItemHeader
                Node {
                    min_height: size::ROW_HEIGHT,
                    display: Display::Flex,
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    column_gap: px(4),
                    padding: UiRect::axes(px(ROW_PADDING), px(2)),
                }
                Hovered
                ThemeBackgroundColor(tokens::LISTROW_BG)
                InheritableThemeTextColor(tokens::LISTROW_TEXT)
                InheritableFont {
                    font: fonts::REGULAR,
                    font_size: FontSize::Px(14.0),
                    weight: FontWeight::NORMAL,
                }
                Children [
                    FeathersTreeItemToggle
                    TreeItemToggle
                    Node {
                        width: px(12),
                        height: px(12),
                        display: Display::Flex,
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::Center,
                    }
                    InheritableThemeTextColor(tokens::BUTTON_TEXT)
                    Children [
                        @icon(icons::CHEVRON_RIGHT)
                        FeathersTreeItemChevron
                    ]
                    --
                    {props.label}
                ]
                --
                TreeItemChildren
                FeathersTreeItemChildren
                Node {
                    display: Display::None,
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Stretch,
                }
                Children [
                    {props.children}
                ]
            ]
        }
    }
}

/// The header line of a [`FeathersTreeItem`], carrying the row background and focus ring.
#[derive(Component, Default, Clone, Reflect)]
#[reflect(Component, Clone, Default)]
pub struct FeathersTreeItemHeader;

/// The disclosure control of a [`FeathersTreeItem`].
#[derive(Component, Default, Clone, Reflect)]
#[reflect(Component, Clone, Default)]
pub struct FeathersTreeItemToggle;

/// The chevron displayed inside a [`FeathersTreeItemToggle`].
#[derive(Component, Default, Clone, Reflect)]
#[reflect(Component, Clone, Default)]
pub struct FeathersTreeItemChevron;

/// The child row container of a [`FeathersTreeItem`].
#[derive(Component, Default, Clone, Reflect)]
#[reflect(Component, Clone, Default)]
pub struct FeathersTreeItemChildren;

type RowQuery<'w, 's> = Query<
    'w,
    's,
    (&'static Children, Has<InteractionDisabled>, Has<Selected>),
    With<FeathersTreeItem>,
>;

type HeaderQuery<'w, 's> =
    Query<'w, 's, (&'static Hovered, &'static ThemeBackgroundColor), With<FeathersTreeItemHeader>>;

type ToggleQuery<'w, 's> =
    Query<'w, 's, (&'static mut UiTransform, &'static Children), With<FeathersTreeItemToggle>>;

fn update_row_styles(
    rows: RowQuery,
    changed: Query<
        Entity,
        (
            With<FeathersTreeItem>,
            Or<(
                Added<FeathersTreeItem>,
                Added<Selected>,
                Added<InteractionDisabled>,
            )>,
        ),
    >,
    headers: HeaderQuery,
    mut commands: Commands,
) {
    for row in changed.iter() {
        set_row_styles(row, &rows, &headers, &mut commands);
    }
}

fn update_row_styles_remove(
    rows: RowQuery,
    headers: HeaderQuery,
    mut removed_selected: RemovedComponents<Selected>,
    mut removed_disabled: RemovedComponents<InteractionDisabled>,
    mut commands: Commands,
) {
    removed_selected
        .read()
        .chain(removed_disabled.read())
        .for_each(|row| set_row_styles(row, &rows, &headers, &mut commands));
}

fn update_row_hover(
    headers: Query<
        (Entity, &ChildOf, &Hovered, &ThemeBackgroundColor),
        (With<FeathersTreeItemHeader>, Changed<Hovered>),
    >,
    rows: Query<(Has<InteractionDisabled>, Has<Selected>), With<FeathersTreeItem>>,
    mut commands: Commands,
) {
    for (header, child_of, hovered, bg_color) in headers.iter() {
        if let Ok((disabled, selected)) = rows.get(child_of.parent()) {
            set_header_styles(
                header,
                disabled,
                selected,
                hovered.0,
                bg_color,
                &mut commands,
            );
        }
    }
}

fn set_row_styles(row: Entity, rows: &RowQuery, headers: &HeaderQuery, commands: &mut Commands) {
    let Ok((row_children, disabled, selected)) = rows.get(row) else {
        return;
    };
    for child in row_children.iter().copied() {
        if let Ok((hovered, bg_color)) = headers.get(child) {
            set_header_styles(child, disabled, selected, hovered.0, bg_color, commands);
        }
    }
}

fn set_header_styles(
    header: Entity,
    disabled: bool,
    selected: bool,
    hovered: bool,
    bg_color: &ThemeBackgroundColor,
    commands: &mut Commands,
) {
    let bg_token = match (disabled, selected, hovered) {
        (false, true, _) => tokens::LISTROW_BG_SELECTED,
        (false, false, true) => tokens::LISTROW_BG_HOVER,
        _ => tokens::LISTROW_BG,
    };

    let text_token = match disabled {
        true => tokens::LISTROW_TEXT_DISABLED,
        false => tokens::LISTROW_TEXT,
    };

    let cursor_shape = match disabled {
        true => bevy_window::SystemCursorIcon::NotAllowed,
        false => bevy_window::SystemCursorIcon::Pointer,
    };

    if bg_color.0 != bg_token {
        commands
            .entity(header)
            .insert(ThemeBackgroundColor(bg_token));
    }

    commands.entity(header).insert((
        InheritableThemeTextColor(text_token),
        EntityCursor::System(cursor_shape),
    ));
}

fn update_toggle_styles(
    changed: Query<
        (
            &Children,
            Has<Expanded>,
            Has<Expandable>,
            Has<InteractionDisabled>,
        ),
        (
            With<FeathersTreeItem>,
            Or<(
                Added<FeathersTreeItem>,
                Added<Expanded>,
                Added<Expandable>,
                Added<InteractionDisabled>,
            )>,
        ),
    >,
    children: Query<&Children>,
    mut toggles: ToggleQuery,
    mut chevrons: Query<&mut Node, With<FeathersTreeItemChevron>>,
    mut commands: Commands,
) {
    for (row_children, expanded, expandable, disabled) in changed.iter() {
        set_toggle_styles(
            row_children,
            expanded,
            expandable,
            disabled,
            &children,
            &mut toggles,
            &mut chevrons,
            &mut commands,
        );
    }
}

fn update_toggle_styles_remove(
    rows: Query<
        (
            &Children,
            Has<Expanded>,
            Has<Expandable>,
            Has<InteractionDisabled>,
        ),
        With<FeathersTreeItem>,
    >,
    children: Query<&Children>,
    mut toggles: ToggleQuery,
    mut chevrons: Query<&mut Node, With<FeathersTreeItemChevron>>,
    mut removed_expanded: RemovedComponents<Expanded>,
    mut removed_expandable: RemovedComponents<Expandable>,
    mut removed_disabled: RemovedComponents<InteractionDisabled>,
    mut commands: Commands,
) {
    removed_expanded
        .read()
        .chain(removed_expandable.read())
        .chain(removed_disabled.read())
        .for_each(|row| {
            if let Ok((row_children, expanded, expandable, disabled)) = rows.get(row) {
                set_toggle_styles(
                    row_children,
                    expanded,
                    expandable,
                    disabled,
                    &children,
                    &mut toggles,
                    &mut chevrons,
                    &mut commands,
                );
            }
        });
}

#[expect(
    clippy::too_many_arguments,
    reason = "the toggle look is spread over the toggle node and its chevron"
)]
fn set_toggle_styles(
    row_children: &Children,
    expanded: bool,
    expandable: bool,
    disabled: bool,
    children: &Query<&Children>,
    toggles: &mut ToggleQuery,
    chevrons: &mut Query<&mut Node, With<FeathersTreeItemChevron>>,
    commands: &mut Commands,
) {
    let Some(toggle) = row_children
        .iter()
        .copied()
        .filter_map(|child| children.get(child).ok())
        .flat_map(|header_children| header_children.iter().copied())
        .find(|candidate| toggles.contains(*candidate))
    else {
        return;
    };

    let rotation = match expanded {
        true => Rot2::turn_fraction(0.25),
        false => Rot2::turn_fraction(0.0),
    };
    let chevron_display = match expandable {
        true => Display::Flex,
        false => Display::None,
    };

    if let Ok((mut transform, toggle_children)) = toggles.get_mut(toggle) {
        if transform.rotation != rotation {
            transform.rotation = rotation;
        }
        for chevron in toggle_children.iter().copied() {
            if let Ok(mut node) = chevrons.get_mut(chevron)
                && node.display != chevron_display
            {
                node.display = chevron_display;
            }
        }
    }

    let text_token = match disabled {
        true => tokens::BUTTON_TEXT_DISABLED,
        false => tokens::BUTTON_TEXT,
    };
    commands
        .entity(toggle)
        .insert(InheritableThemeTextColor(text_token));
}

fn update_children_visibility(
    mut containers: Query<(&mut Node, &ChildOf), With<FeathersTreeItemChildren>>,
    rows: Query<Has<Expanded>, With<FeathersTreeItem>>,
) {
    for (mut node, child_of) in containers.iter_mut() {
        let Ok(expanded) = rows.get(child_of.parent()) else {
            continue;
        };
        let display = match expanded {
            true => Display::Flex,
            false => Display::None,
        };
        if node.display != display {
            node.display = display;
        }
    }
}

fn update_row_indent(
    rows: Query<(&TreeItem, &Children), Changed<TreeItem>>,
    mut headers: Query<&mut Node, With<FeathersTreeItemHeader>>,
) {
    for (item, row_children) in rows.iter() {
        let left = px(ROW_PADDING + INDENT_STEP * item.level as f32);
        for child in row_children.iter().copied() {
            if let Ok(mut node) = headers.get_mut(child)
                && node.padding.left != left
            {
                node.padding.left = left;
            }
        }
    }
}

fn update_row_focus(
    focus: Res<InputFocus>,
    focus_visible: Res<InputFocusVisible>,
    theme: Res<UiTheme>,
    headers: Query<(Entity, &ChildOf), With<FeathersTreeItemHeader>>,
    mut commands: Commands,
) {
    if !focus.is_changed() && !focus_visible.is_changed() && !theme.is_changed() {
        return;
    }

    for (header, child_of) in headers.iter() {
        if focus_visible.0 && focus.get() == Some(child_of.parent()) {
            commands.entity(header).insert(Outline {
                color: theme.context_color(&tokens::FOCUS_RING, SurfaceLevel::Base),
                width: px(2),
                offset: px(0),
            });
        } else {
            commands.entity(header).remove::<Outline>();
        }
    }
}

/// Plugin which registers the systems for updating the tree view styles.
pub struct TreeViewPlugin;

impl Plugin for TreeViewPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        app.add_systems(
            PostUpdate,
            (
                update_row_styles,
                update_row_styles_remove,
                update_row_hover,
                update_toggle_styles,
                update_toggle_styles_remove,
                update_children_visibility,
                update_row_indent,
                update_row_focus,
            )
                .chain()
                .after(MenuFocusSystem)
                .in_set(UiSystems::Content),
        );
    }
}
