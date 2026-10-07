use bevy_app::{Plugin, PostUpdate, Propagate};
use bevy_color::{Alpha, Srgba};
use bevy_ecs::{
    component::Component,
    entity::Entity,
    hierarchy::{ChildOf, Children},
    lifecycle::Add,
    observer::On,
    query::{Changed, Has, Or, With, Without},
    reflect::ReflectComponent,
    schedule::IntoScheduleConfigs,
    system::{Commands, Query, Res},
    template::{EntityTemplate, OptionTemplate},
    world::World,
};
use bevy_input_focus::tab_navigation::TabIndex;
use bevy_math::Vec2;
use bevy_picking::{
    cursor::EntityCursor,
    events::PointerState,
    hover::Hovered,
    pointer::{PointerButton, PointerId},
    Pickable,
};
use bevy_reflect::{prelude::ReflectDefault, Reflect};
use bevy_scene::prelude::*;
use bevy_text::{FontWeight, LineBreak, TextLayout};
use bevy_ui::{
    px, AlignItems, BorderRadius, BoxShadow, ComputedNode, Display, FlexDirection, GlobalZIndex,
    InlineDirection, InteractionDisabled, Node, OuterColor, Outline, Overflow, PositionType,
    Selected, UiRect, UiSystems, Val,
};
use bevy_ui_widgets::{
    ControlOrientation, DragOverlayRoot, DragProxy, SelectedTab, Tab, TabActivation, TabDragMode,
    TabDragging, TabInsertionPreview, TabList,
};
use bevy_window::SystemCursorIcon;

use crate::{
    constants::{fonts, size},
    focus::FocusIndicator,
    font_styles::InheritableFont,
    theme::{
        InheritableThemeTextColor, SurfaceLevel, ThemeBackgroundColor, ThemeBorderColor,
        ThemeContext, ThemedText, UiTheme,
    },
    tokens,
};

const TAB_PADDING: f32 = 10.0;
const TAB_GAP: f32 = 6.0;
const TAB_MIN_WIDTH: f32 = TAB_PADDING * 2.0 + 24.0;
const TAB_RADIUS: f32 = 4.0;
const STRIPE_SIZE: f32 = 2.0;
const FILLET_SIZE: f32 = 8.0;
const STRIP_INSET: f32 = 3.0;
const STRIP_BORDER: f32 = 1.0;
const INDICATOR_SIZE: f32 = 2.0;
const DRAG_PROXY_OFFSET: Vec2 = Vec2::new(10.0, 10.0);
const DRAG_PROXY_Z: i32 = 200;

/// A themed strip of [`FeathersTab`]s.
///
/// A more complete explanation of how to control this widget can be found in the documentation
/// for [`TabList`] and [`bevy_ui_widgets`]. Selection changes and drag moves are proposed with
/// events and applied by the app.
///
/// The selected tab shares its background with the pane body below the strip and flares into it
/// at its bottom corners. Children marked [`FeathersTabListLeading`] or
/// [`FeathersTabListTrailing`] are kept at the start or end of the strip, so apps can put their own
/// controls there. Use [`FeathersTabList::child_index`] to apply a [`bevy_ui_widgets::TabMoved`]
/// when such children are present.
///
/// While a tab is dragged, a [`FeathersTabDragProxy`] follows the pointer and a
/// [`FeathersTabInsertionIndicator`] marks the drop position. The proxy is only shown when an
/// ancestor of the list has [`DragOverlayRoot`], usually the app's root UI node.
#[derive(SceneComponent, Default, Clone, Reflect)]
#[scene(FeathersTabListProps)]
#[reflect(Component, Clone, Default)]
pub struct FeathersTabList;

/// Props used to construct a [`FeathersTabList`] scene.
pub struct FeathersTabListProps {
    /// The layout and arrow-key axis.
    pub orientation: ControlOrientation,
    /// Whether moving focus also requests selection.
    pub activation: TabActivation,
    /// The accepted drag gestures.
    pub drag: TabDragMode,
    /// The initially selected tab.
    pub selected: OptionTemplate<EntityTemplate>,
    /// Where the selected tab may flare into the body at the ends of the list.
    pub fillets: TabFillets,
}

impl Default for FeathersTabListProps {
    fn default() -> Self {
        Self {
            orientation: ControlOrientation::Horizontal,
            activation: TabActivation::Manual,
            drag: TabDragMode::Disabled,
            selected: OptionTemplate::None,
            fillets: TabFillets::default(),
        }
    }
}

impl FeathersTabList {
    /// Scene function for a tab list.
    pub fn scene(props: FeathersTabListProps) -> impl Scene {
        let (flex_direction, padding, border, overflow) = match props.orientation {
            ControlOrientation::Horizontal => (
                FlexDirection::Row,
                UiRect::new(px(FILLET_SIZE), px(FILLET_SIZE), px(STRIP_INSET), px(0)),
                UiRect::new(px(STRIP_BORDER), px(STRIP_BORDER), px(STRIP_BORDER), px(0)),
                Overflow::clip_x(),
            ),
            ControlOrientation::Vertical => (
                FlexDirection::Column,
                UiRect::new(px(STRIP_INSET), px(0), px(FILLET_SIZE), px(FILLET_SIZE)),
                UiRect::new(px(STRIP_BORDER), px(0), px(STRIP_BORDER), px(STRIP_BORDER)),
                Overflow::clip_y(),
            ),
        };
        let min_height = match props.orientation {
            ControlOrientation::Horizontal => strip_min_height(),
            ControlOrientation::Vertical => size::ROW_HEIGHT,
        };
        bsn! {
            Node {
                display: Display::Flex,
                flex_direction: {flex_direction},
                align_items: AlignItems::Stretch,
                min_height: {min_height},
                padding: {padding},
                border: {border},
                overflow: {overflow},
            }
            TabList {
                orientation: {props.orientation},
                activation: {props.activation},
                drag: {props.drag},
            }
            SelectedTab({props.selected})
            TabFillets {
                start: {props.fillets.start},
                end: {props.fillets.end},
            }
            ThemeBackgroundColor(tokens::TAB_STRIP_BG)
            ThemeBorderColor(tokens::TAB_STRIP_BORDER)
            Propagate::<ThemeContext>(ThemeContext(SurfaceLevel::Base))
        }
    }

    /// Returns where to insert `tab` among a list's `children` so that it lands at `index`, as
    /// given by [`bevy_ui_widgets::TabMoved`] or [`bevy_ui_widgets::TabDrop`].
    ///
    /// Those indices only count tabs, so this skips slot content, buttons and `tab` itself.
    pub fn child_index(
        children: &[Entity],
        is_tab: impl Fn(Entity) -> bool,
        tab: Entity,
        index: usize,
    ) -> usize {
        let mut seen = 0;
        let mut end = 0;
        for (position, child) in children
            .iter()
            .copied()
            .filter(|child| *child != tab)
            .enumerate()
        {
            if !is_tab(child) {
                continue;
            }
            if seen == index {
                return position;
            }
            seen += 1;
            end = position + 1;
        }
        end
    }
}

/// Controls whether the selected tab flares into the body at the ends of a [`FeathersTabList`].
///
/// A flare between two tabs is always shown. `start` and `end` only apply when the selected tab
/// is the first or last child laid out in the list.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Reflect)]
#[reflect(Component, Default, Clone, PartialEq)]
pub struct TabFillets {
    /// Show the flare at the start of the list.
    pub start: bool,
    /// Show the flare at the end of the list.
    pub end: bool,
}

impl Default for TabFillets {
    fn default() -> Self {
        Self {
            start: true,
            end: true,
        }
    }
}

/// Content placed before the tabs in a [`FeathersTabList`].
///
/// Add this as a child of the list and put the app's own controls inside it.
#[derive(SceneComponent, Default, Clone, Reflect)]
#[reflect(Component, Clone, Default)]
pub struct FeathersTabListLeading;

impl FeathersTabListLeading {
    fn scene() -> impl Scene {
        bsn! {
            Node {
                display: Display::Flex,
                align_items: AlignItems::Center,
                column_gap: px(2),
                flex_shrink: 0.0,
            }
        }
    }
}

/// Content placed at the far end of a [`FeathersTabList`].
///
/// Add this as a child of the list and put the app's own controls inside it.
#[derive(SceneComponent, Default, Clone, Reflect)]
#[reflect(Component, Clone, Default)]
pub struct FeathersTabListTrailing;

impl FeathersTabListTrailing {
    fn scene() -> impl Scene {
        bsn! {
            Node {
                display: Display::Flex,
                align_items: AlignItems::Center,
                column_gap: px(2),
                flex_shrink: 0.0,
            }
        }
    }
}

/// A themed tab header inside a [`FeathersTabList`].
///
/// The caption may contain text, icons and app-owned controls such as a close button. When the
/// list is too narrow, tabs shrink down to a minimum width and their captions are clipped.
#[derive(SceneComponent, Default, Clone, Reflect)]
#[scene(FeathersTabProps)]
#[reflect(Component, Clone, Default)]
pub struct FeathersTab;

/// Props used to construct a [`FeathersTab`] scene.
pub struct FeathersTabProps {
    /// The contents of the tab header.
    pub caption: Box<dyn SceneList>,
}

impl Default for FeathersTabProps {
    fn default() -> Self {
        Self {
            caption: Box::new(bsn_list! {}),
        }
    }
}

impl FeathersTab {
    /// Scene function for a tab.
    pub fn scene(props: FeathersTabProps) -> impl Scene {
        let (border, border_radius) = tab_shape(ControlOrientation::Horizontal);
        bsn! {
            Node {
                display: Display::Flex,
                min_width: px(TAB_MIN_WIDTH),
                min_height: size::ROW_HEIGHT,
                flex_shrink: 1.0,
                align_items: AlignItems::Center,
                padding: UiRect::horizontal(px(TAB_PADDING)),
                border: {border},
                border_radius: {border_radius},
            }
            Tab
            Hovered
            EntityCursor::System(SystemCursorIcon::Pointer)
            FocusIndicator
            ThemeBackgroundColor(tokens::TAB_BG)
            ThemeBorderColor(tokens::TAB_STRIPE)
            InheritableThemeTextColor(tokens::TAB_TEXT)
            InheritableFont {
                font: fonts::REGULAR,
                font_size: size::MEDIUM_FONT,
                weight: FontWeight::NORMAL,
            }
            Children [
                @tab_fillet(TabFilletEdge::Start)
                --
                Node {
                    display: Display::Flex,
                    align_items: AlignItems::Center,
                    column_gap: px(TAB_GAP),
                    min_width: px(0),
                    flex_shrink: 1.0,
                    overflow: Overflow::clip_x(),
                }
                TabCaption
                ThemedText
                Children [ {props.caption} ]
                --
                @tab_fillet(TabFilletEdge::End)
            ]
        }
    }
}

/// The node that holds and clips a tab's caption.
#[derive(Component, Debug, Default, Clone, Copy, Reflect)]
#[reflect(Component, Clone, Default)]
struct TabCaption;

/// One of the two flares at the base of a selected [`FeathersTab`].
///
/// Only the area outside its rounded corner is painted, so the neighboring tab shows through.
#[derive(Component, Debug, Default, Clone, Copy, PartialEq, Eq, Reflect)]
#[reflect(Component, Clone, Default, PartialEq)]
pub enum TabFilletEdge {
    /// The flare at the logical start of the tab.
    #[default]
    Start,
    /// The flare at the logical end of the tab.
    End,
}

fn tab_fillet(edge: TabFilletEdge) -> impl Scene {
    bsn! {
        Node {
            position_type: PositionType::Absolute,
            display: Display::None,
        }
        edge
        OuterColor
        Pickable::IGNORE
    }
}

/// A line marking where a dragged tab would be inserted into a [`FeathersTabList`].
///
/// Spawned by [`FeathersTabsPlugin`] from each [`TabInsertionPreview`] entry, as a child of the
/// tab it precedes, or of the last tab or the list itself.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Reflect)]
#[reflect(Component, Clone, PartialEq)]
pub struct FeathersTabInsertionIndicator {
    /// The list showing the preview.
    pub list: Entity,
    /// The pointer that controls the preview.
    pub pointer_id: PointerId,
}

/// A copy of a dragged [`FeathersTab`] that follows the pointer.
///
/// Spawned by [`FeathersTabsPlugin`] when a tab starts dragging and moved beneath the nearest
/// [`DragOverlayRoot`]. It holds copies of the tab's caption and is despawned with its
/// [`DragProxy`] when the drag ends.
#[derive(Component, Debug, Default, Clone, Copy, Reflect)]
#[reflect(Component, Clone, Default)]
pub struct FeathersTabDragProxy;

/// Returns the height of a horizontal strip holding one row of tabs, so an empty strip matches.
fn strip_min_height() -> Val {
    match size::ROW_HEIGHT {
        Val::Px(row) => px(STRIP_INSET + STRIP_BORDER + row),
        other => other,
    }
}

/// Returns a tab's border, which holds the selection stripe, and its corner radii.
fn tab_shape(orientation: ControlOrientation) -> (UiRect, BorderRadius) {
    match orientation {
        ControlOrientation::Horizontal => (
            UiRect::top(px(STRIPE_SIZE)),
            BorderRadius::top(px(TAB_RADIUS)),
        ),
        ControlOrientation::Vertical => (
            UiRect::left(px(STRIPE_SIZE)),
            BorderRadius::left(px(TAB_RADIUS)),
        ),
    }
}

fn update_tab_styles(
    mut tabs: Query<
        (
            Entity,
            Option<&ChildOf>,
            &Hovered,
            Has<Selected>,
            Has<InteractionDisabled>,
            Has<TabDragging>,
            &ThemeBackgroundColor,
            &ThemeBorderColor,
            &InheritableThemeTextColor,
            Option<&EntityCursor>,
            &mut Node,
        ),
        With<FeathersTab>,
    >,
    lists: Query<
        (
            Entity,
            &TabList,
            Has<InteractionDisabled>,
            Has<TabInsertionPreview>,
            &ThemeBorderColor,
        ),
        With<FeathersTabList>,
    >,
    mut commands: Commands,
) {
    for (
        tab,
        parent,
        hovered,
        selected,
        tab_disabled,
        dragging,
        bg,
        stripe,
        text,
        cursor,
        mut node,
    ) in &mut tabs
    {
        let list = parent.and_then(|parent| lists.get(parent.parent()).ok());
        let disabled = tab_disabled || list.is_some_and(|(_, _, disabled, _, _)| disabled);
        let orientation = list.map_or(ControlOrientation::Horizontal, |(_, tablist, ..)| {
            tablist.orientation
        });

        let bg_token = match (disabled, dragging, selected, hovered.0) {
            (false, true, _, _) => tokens::TAB_BG_DRAGGING,
            (false, false, true, _) => tokens::TAB_BG_SELECTED,
            (false, false, false, true) => tokens::TAB_BG_HOVER,
            _ => tokens::TAB_BG,
        };
        let stripe_token = match !disabled && !dragging && selected {
            true => tokens::TAB_STRIPE_SELECTED,
            false => tokens::TAB_STRIPE,
        };
        let text_token = match (disabled, dragging, selected) {
            (true, _, _) => tokens::TAB_TEXT_DISABLED,
            (false, true, _) => tokens::TAB_TEXT_DRAGGING,
            (false, false, true) => tokens::TAB_TEXT_SELECTED,
            (false, false, false) => tokens::TAB_TEXT,
        };
        let cursor_shape = EntityCursor::System(match disabled {
            true => SystemCursorIcon::NotAllowed,
            false => SystemCursorIcon::Pointer,
        });
        let (border, border_radius) = tab_shape(orientation);

        if bg.0 != bg_token {
            commands.entity(tab).insert(ThemeBackgroundColor(bg_token));
        }
        if stripe.0 != stripe_token {
            commands.entity(tab).insert(ThemeBorderColor(stripe_token));
        }
        if text.0 != text_token {
            commands
                .entity(tab)
                .insert(InheritableThemeTextColor(text_token));
        }
        if cursor != Some(&cursor_shape) {
            commands.entity(tab).insert(cursor_shape);
        }
        if node.border != border || node.border_radius != border_radius {
            node.border = border;
            node.border_radius = border_radius;
        }
    }

    for (list, _, _, preview, border) in &lists {
        let border_token = match preview {
            true => tokens::TAB_STRIP_BORDER_PREVIEW,
            false => tokens::TAB_STRIP_BORDER,
        };
        if border.0 != border_token {
            commands.entity(list).insert(ThemeBorderColor(border_token));
        }
    }
}

/// Keeps leading content first, and the add button and trailing content after the tabs.
fn arrange_tab_lists(
    lists: Query<
        (Entity, &TabList, &Children),
        (
            With<FeathersTabList>,
            Or<(Changed<Children>, Changed<TabList>)>,
        ),
    >,
    mut leading: Query<
        &mut Node,
        (
            With<FeathersTabListLeading>,
            Without<FeathersTabListTrailing>,
        ),
    >,
    mut trailing: Query<
        &mut Node,
        (
            With<FeathersTabListTrailing>,
            Without<FeathersTabListLeading>,
        ),
    >,
    mut commands: Commands,
) {
    for (list, tablist, children) in &lists {
        let (leading_margin, trailing_margin) = match tablist.orientation {
            ControlOrientation::Horizontal => (UiRect::right(px(4)), UiRect::left(Val::Auto)),
            ControlOrientation::Vertical => (UiRect::bottom(px(4)), UiRect::top(Val::Auto)),
        };
        for child in children.iter().copied() {
            if let Ok(mut node) = leading.get_mut(child)
                && node.margin != leading_margin
            {
                node.margin = leading_margin;
            }
            if let Ok(mut node) = trailing.get_mut(child)
                && node.margin != trailing_margin
            {
                node.margin = trailing_margin;
            }
        }
        let rank = |child: &Entity| {
            if leading.contains(*child) {
                0
            } else if trailing.contains(*child) {
                2
            } else {
                1
            }
        };
        let mut arranged = children.to_vec();
        arranged.sort_by_key(rank);
        if arranged[..] != children[..] {
            commands.entity(list).insert_children(0, &arranged);
        }
    }
}

/// Keeps text placed directly in a tab caption on one line, so it is clipped instead of wrapped.
fn keep_captions_on_one_line(
    captions: Query<&Children, (With<TabCaption>, Changed<Children>)>,
    mut layouts: Query<&mut TextLayout>,
) {
    for children in &captions {
        for child in children.iter().copied() {
            if let Ok(mut layout) = layouts.get_mut(child)
                && layout.linebreak != LineBreak::NoWrap
            {
                layout.linebreak = LineBreak::NoWrap;
            }
        }
    }
}

fn update_tab_fillets(
    lists: Query<
        (&Node, &Children, &SelectedTab, Option<&TabFillets>),
        (With<FeathersTabList>, Without<TabFilletEdge>),
    >,
    tabs: Query<(Has<InteractionDisabled>, Has<TabDragging>, &Children), With<FeathersTab>>,
    layout: Query<&Node, Without<TabFilletEdge>>,
    mut fillets: Query<(
        &TabFilletEdge,
        &mut Node,
        &mut OuterColor,
        Option<&ThemeContext>,
    )>,
    theme: Option<Res<UiTheme>>,
) {
    for (list_node, children, selection, options) in &lists {
        let options = options.copied().unwrap_or_default();
        let selected = selection.0.filter(|selected| {
            children.contains(selected)
                && tabs
                    .get(*selected)
                    .is_ok_and(|(disabled, dragging, _)| !disabled && !dragging)
        });
        let laid_out = children
            .iter()
            .copied()
            .filter(|child| {
                layout.get(*child).is_ok_and(|node| {
                    node.position_type != PositionType::Absolute && node.display != Display::None
                })
            })
            .collect::<Vec<_>>();
        let reversed = matches!(
            list_node.flex_direction,
            FlexDirection::RowReverse | FlexDirection::ColumnReverse
        );
        let start_edge = logical_start_edge(list_node);

        for tab in children.iter().copied() {
            let Ok((_, _, tab_children)) = tabs.get(tab) else {
                continue;
            };
            let position = laid_out.iter().position(|child| *child == tab);
            let before = position.is_some_and(|index| index > 0);
            let after = position.is_some_and(|index| index + 1 < laid_out.len());
            let (start_space, end_space) = match reversed {
                true => (after, before),
                false => (before, after),
            };

            for child in tab_children.iter().copied() {
                let Ok((edge, mut node, mut outer, context)) = fillets.get_mut(child) else {
                    continue;
                };
                let visible = selected == Some(tab)
                    && match edge {
                        TabFilletEdge::Start => start_space || options.start,
                        TabFilletEdge::End => end_space || options.end,
                    };
                let physical = match edge {
                    TabFilletEdge::Start => start_edge,
                    TabFilletEdge::End => start_edge.opposite(),
                };
                let desired = fillet_node(visible, physical);
                if *node != desired {
                    *node = desired;
                }
                if let Some(theme) = theme.as_ref() {
                    let context = context.map_or(SurfaceLevel::Base, |context| context.0);
                    let color = theme.context_color(&tokens::TAB_BG_SELECTED, context);
                    if outer.0 != color {
                        outer.0 = color;
                    }
                }
            }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PhysicalEdge {
    Left,
    Right,
    Top,
    Bottom,
}

impl PhysicalEdge {
    fn opposite(self) -> Self {
        match self {
            Self::Left => Self::Right,
            Self::Right => Self::Left,
            Self::Top => Self::Bottom,
            Self::Bottom => Self::Top,
        }
    }
}

fn logical_start_edge(node: &Node) -> PhysicalEdge {
    match node.flex_direction {
        FlexDirection::Row | FlexDirection::RowReverse => match node.direction {
            InlineDirection::Ltr => PhysicalEdge::Left,
            InlineDirection::Rtl => PhysicalEdge::Right,
        },
        FlexDirection::Column | FlexDirection::ColumnReverse => PhysicalEdge::Top,
    }
}

/// Returns the node of a flare beside `edge` of a selected tab, curving into the body.
fn fillet_node(visible: bool, edge: PhysicalEdge) -> Node {
    let size = px(FILLET_SIZE);
    let mut node = Node {
        position_type: PositionType::Absolute,
        display: match visible {
            true => Display::Flex,
            false => Display::None,
        },
        width: size,
        height: size,
        ..Default::default()
    };
    match edge {
        PhysicalEdge::Left => {
            node.left = px(-FILLET_SIZE);
            node.bottom = px(0);
            node.border_radius = BorderRadius::bottom_right(size);
        }
        PhysicalEdge::Right => {
            node.right = px(-FILLET_SIZE);
            node.bottom = px(0);
            node.border_radius = BorderRadius::bottom_left(size);
        }
        PhysicalEdge::Top => {
            node.top = px(-FILLET_SIZE);
            node.right = px(0);
            node.border_radius = BorderRadius::bottom_right(size);
        }
        PhysicalEdge::Bottom => {
            node.bottom = px(-FILLET_SIZE);
            node.right = px(0);
            node.border_radius = BorderRadius::top_right(size);
        }
    }
    node
}
fn update_insertion_indicators(
    lists: Query<
        (
            Entity,
            &TabList,
            Option<&TabInsertionPreview>,
            Option<&Children>,
        ),
        With<FeathersTabList>,
    >,
    tabs: Query<(), With<Tab>>,
    mut indicators: Query<(Entity, &FeathersTabInsertionIndicator, &ChildOf, &mut Node)>,
    mut commands: Commands,
) {
    for (entity, indicator, _, _) in &indicators {
        let active = lists
            .get(indicator.list)
            .ok()
            .and_then(|(_, _, preview, _)| preview)
            .is_some_and(|preview| {
                preview
                    .entries
                    .iter()
                    .any(|entry| entry.pointer_id == indicator.pointer_id)
            });
        if !active {
            commands.entity(entity).try_despawn();
        }
    }

    for (list, tablist, preview, children) in &lists {
        let Some(preview) = preview else {
            continue;
        };
        let list_tabs = children
            .into_iter()
            .flatten()
            .copied()
            .filter(|child| tabs.contains(*child))
            .collect::<Vec<_>>();
        for entry in &preview.entries {
            let (host, node) =
                indicator_placement(list, tablist.orientation, &list_tabs, entry.slot);
            let existing = indicators
                .iter_mut()
                .find(|(_, indicator, _, _)| {
                    indicator.list == list && indicator.pointer_id == entry.pointer_id
                })
                .map(|(entity, _, parent, node)| (entity, parent.parent(), node));
            match existing {
                Some((entity, parent, mut current)) => {
                    if parent != host {
                        commands.entity(entity).insert(ChildOf(host));
                    }
                    if *current != node {
                        *current = node;
                    }
                }
                None => {
                    commands.spawn((
                        FeathersTabInsertionIndicator {
                            list,
                            pointer_id: entry.pointer_id,
                        },
                        node,
                        ThemeBackgroundColor(tokens::TAB_INSERTION_INDICATOR),
                        Pickable::IGNORE,
                        ChildOf(host),
                    ));
                }
            }
        }
    }
}

/// Returns the entity that hosts an insertion line at `slot` among `tabs`, and the line's node.
fn indicator_placement(
    list: Entity,
    orientation: ControlOrientation,
    tabs: &[Entity],
    slot: usize,
) -> (Entity, Node) {
    let (host, at_end) = match tabs.get(slot) {
        Some(tab) => (*tab, false),
        None => match tabs.last() {
            Some(tab) => (*tab, true),
            None => (list, false),
        },
    };
    let edge = match host == list {
        true => px(0),
        false => px(-INDICATOR_SIZE * 0.5),
    };
    let mut node = Node {
        position_type: PositionType::Absolute,
        ..Default::default()
    };
    match orientation {
        ControlOrientation::Horizontal => {
            node.width = px(INDICATOR_SIZE);
            node.top = px(0);
            node.bottom = px(0);
            match at_end {
                true => node.right = edge,
                false => node.left = edge,
            }
        }
        ControlOrientation::Vertical => {
            node.height = px(INDICATOR_SIZE);
            node.left = px(0);
            node.right = px(0);
            match at_end {
                true => node.bottom = edge,
                false => node.top = edge,
            }
        }
    }
    (host, node)
}

fn spawn_tab_drag_proxy(
    add: On<Add<TabDragging>>,
    tabs: Query<
        (
            &TabDragging,
            &Node,
            Option<&ComputedNode>,
            Option<&InheritableFont>,
            Option<&Children>,
        ),
        With<FeathersTab>,
    >,
    parents: Query<&ChildOf>,
    overlay_roots: Query<(), With<DragOverlayRoot>>,
    skipped: Query<
        (),
        Or<(
            With<FeathersTabInsertionIndicator>,
            With<FeathersTabDragProxy>,
            With<TabFilletEdge>,
        )>,
    >,
    caption_nodes: Query<&Children, With<TabCaption>>,
    pointer_state: Option<Res<PointerState>>,
    theme: Option<Res<UiTheme>>,
    mut commands: Commands,
) {
    let tab = add.entity;
    let Ok((dragging, tab_node, computed, font, children)) = tabs.get(tab) else {
        return;
    };
    if !parents
        .iter_ancestors(tab)
        .any(|ancestor| overlay_roots.contains(ancestor))
    {
        return;
    }
    let source =
        pointer_state
            .as_ref()
            .and_then(|state| state.get(dragging.pointer_id, PointerButton::Primary))
            .and_then(|state| {
                state.dragging.keys().copied().find(|entity| {
                    *entity == tab || parents.iter_ancestors(*entity).any(|a| a == tab)
                })
            })
            .unwrap_or(tab);

    let size = computed
        .map(|computed| computed.size() * computed.inverse_scale_factor())
        .unwrap_or_default();
    let length = |value: f32| match value > 0.0 {
        true => px(value),
        false => Val::Auto,
    };
    let proxy = commands.spawn_empty().id();
    let captions = children
        .into_iter()
        .flatten()
        .copied()
        .filter(|child| !skipped.contains(*child))
        .flat_map(|child| match caption_nodes.get(child) {
            Ok(caption) => caption.to_vec(),
            Err(_) => vec![child],
        })
        .collect::<Vec<_>>();
    commands.queue(move |world: &mut World| {
        for caption in captions {
            if world.get_entity(caption).is_err() {
                continue;
            }
            let copy = world
                .entity_mut(caption)
                .clone_and_spawn_with_opt_out(|builder| {
                    builder.linked_cloning(true).deny::<TabIndex>();
                });
            world.entity_mut(copy).insert(ChildOf(proxy));
            let mut stack = vec![copy];
            while let Some(entity) = stack.pop() {
                let mut entity = world.entity_mut(entity);
                entity.insert(Pickable::IGNORE);
                if let Some(mut layout) = entity.get_mut::<TextLayout>() {
                    layout.linebreak = LineBreak::NoWrap;
                }
                if let Some(mut node) = entity.get_mut::<Node>() {
                    node.flex_shrink = 0.0;
                }
                if let Some(children) = entity.get::<Children>() {
                    stack.extend(children.iter());
                }
            }
        }
    });

    let mut proxy = commands.entity(proxy);
    proxy.insert((
        FeathersTabDragProxy,
        DragProxy {
            pointer_id: dragging.pointer_id,
            offset: DRAG_PROXY_OFFSET,
        },
        Node {
            display: Display::Flex,
            position_type: PositionType::Absolute,
            width: Val::Auto,
            height: length(size.y),
            min_height: size::ROW_HEIGHT,
            align_items: AlignItems::Center,
            padding: UiRect::horizontal(px(TAB_PADDING)),
            column_gap: px(TAB_GAP),
            border: tab_node.border,
            border_radius: tab_node.border_radius,
            ..Default::default()
        },
        GlobalZIndex(DRAG_PROXY_Z),
        Outline::new(
            px(1),
            px(0),
            theme.map_or(Srgba::BLACK.into(), |theme| {
                theme.color(&tokens::TAB_DRAG_PROXY_OUTLINE)
            }),
        ),
        BoxShadow::new(
            Srgba::BLACK.with_alpha(0.6).into(),
            px(0),
            px(2),
            px(0),
            px(6),
        ),
        ThemeBackgroundColor(tokens::TAB_DRAG_PROXY_BG),
        ThemeBorderColor(tokens::TAB_DRAG_PROXY_BORDER),
        InheritableThemeTextColor(tokens::TAB_DRAG_PROXY_TEXT),
        ChildOf(source),
    ));
    if let Some(font) = font {
        proxy.insert(font.clone());
    }
}

/// Plugin which registers the systems for styling tabs and showing tab drag feedback.
pub struct FeathersTabsPlugin;

impl Plugin for FeathersTabsPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        app.add_observer(spawn_tab_drag_proxy).add_systems(
            PostUpdate,
            (
                arrange_tab_lists,
                update_tab_styles,
                keep_captions_on_one_line,
                update_tab_fillets,
                update_insertion_indicators,
            )
                .in_set(UiSystems::Content),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy_app::{App, TaskPoolPlugin};
    use bevy_asset::{AssetApp, AssetPlugin};
    use bevy_picking::events::DragEntry;
    use bevy_scene::{ScenePlugin, WorldSceneExt};
    use bevy_ui::widget::Text;
    use bevy_ui_widgets::TabInsertionPoint;

    use crate::theme::ThemeToken;

    fn scene_app() -> App {
        let mut app = App::new();
        app.add_plugins((
            TaskPoolPlugin::default(),
            AssetPlugin::default(),
            ScenePlugin,
            FeathersTabsPlugin,
        ));
        app.init_asset::<bevy_text::Font>();
        app
    }

    fn tabs_app() -> App {
        let mut app = App::new();
        app.add_plugins(FeathersTabsPlugin);
        app
    }

    fn spawn_list(app: &mut App, tab_count: usize) -> (Entity, Vec<Entity>) {
        let list = app
            .world_mut()
            .spawn((
                FeathersTabList,
                TabList::default(),
                Node::default(),
                ThemeBorderColor(tokens::TAB_STRIP_BORDER),
            ))
            .id();
        let tabs = (0..tab_count)
            .map(|_| {
                app.world_mut()
                    .spawn((
                        FeathersTab,
                        Tab,
                        Node::default(),
                        Hovered::default(),
                        ThemeBackgroundColor(tokens::TAB_BG),
                        ThemeBorderColor(tokens::TAB_STRIPE),
                        InheritableThemeTextColor(tokens::TAB_TEXT),
                        ChildOf(list),
                    ))
                    .id()
            })
            .collect();
        (list, tabs)
    }

    fn set_preview(app: &mut App, list: Entity, tab: Entity, index: usize, slot: usize) {
        app.world_mut()
            .entity_mut(list)
            .insert(TabInsertionPreview {
                entries: vec![TabInsertionPoint {
                    pointer_id: PointerId::Mouse,
                    tab,
                    index,
                    slot,
                }],
            });
        app.update();
    }

    fn indicators(app: &mut App) -> Vec<(Entity, Node)> {
        app.world_mut()
            .query_filtered::<(&ChildOf, &Node), With<FeathersTabInsertionIndicator>>()
            .iter(app.world())
            .map(|(parent, node)| (parent.parent(), node.clone()))
            .collect()
    }

    fn stripe(app: &App, tab: Entity) -> ThemeToken {
        app.world()
            .entity(tab)
            .get::<ThemeBorderColor>()
            .unwrap()
            .0
            .clone()
    }

    fn spawn_fillets(app: &mut App, tab: Entity) -> [Entity; 2] {
        [TabFilletEdge::Start, TabFilletEdge::End].map(|edge| {
            app.world_mut()
                .spawn((edge, Node::default(), OuterColor::default(), ChildOf(tab)))
                .id()
        })
    }

    #[test]
    fn scene_applies_props_and_named_selection() {
        let mut app = scene_app();
        let list = app
            .world_mut()
            .spawn_scene(bsn! {
                @FeathersTabList {
                    @orientation: ControlOrientation::Vertical,
                    @activation: TabActivation::Automatic,
                    @drag: TabDragMode::External,
                    @selected: OptionTemplate::Some(#selected),
                }
                Children [
                    #selected
                    @FeathersTab {
                        @caption: bsn! { Text("Scene") }
                    }
                ]
            })
            .unwrap()
            .id();

        let world = app.world();
        let list = world.entity(list);
        assert_eq!(
            list.get::<TabList>(),
            Some(&TabList {
                orientation: ControlOrientation::Vertical,
                activation: TabActivation::Automatic,
                drag: TabDragMode::External,
            })
        );
        assert_eq!(
            list.get::<Node>().unwrap().flex_direction,
            FlexDirection::Column
        );
        let tab = list.get::<Children>().unwrap()[0];
        assert_eq!(list.get::<SelectedTab>(), Some(&SelectedTab(Some(tab))));
        let tab = world.entity(tab);
        assert!(tab.contains::<Tab>());
        assert!(tab.contains::<FocusIndicator>());
        let tab_children = tab.get::<Children>().unwrap();
        assert_eq!(tab_children.len(), 3);
        assert!(world.entity(tab_children[0]).contains::<TabFilletEdge>());
        let caption = world.entity(tab_children[1]);
        assert!(caption.contains::<TabCaption>());
        let text = caption.get::<Children>().unwrap()[0];
        assert_eq!(world.entity(text).get::<Text>().unwrap().0, "Scene");
        assert!(world.entity(tab_children[2]).contains::<TabFilletEdge>());
    }

    #[test]
    fn tab_styles_follow_state() {
        let mut app = tabs_app();
        let (list, tabs) = spawn_list(&mut app, 1);
        let tab = tabs[0];
        let style = |app: &App| {
            let tab = app.world().entity(tab);
            (
                tab.get::<ThemeBackgroundColor>().unwrap().0.clone(),
                tab.get::<InheritableThemeTextColor>().unwrap().0.clone(),
            )
        };

        app.world_mut().entity_mut(tab).insert(Hovered(true));
        app.update();
        assert_eq!(style(&app), (tokens::TAB_BG_HOVER, tokens::TAB_TEXT));

        app.world_mut().entity_mut(tab).insert(Selected);
        app.update();
        assert_eq!(
            style(&app),
            (tokens::TAB_BG_SELECTED, tokens::TAB_TEXT_SELECTED)
        );
        assert_eq!(stripe(&app, tab), tokens::TAB_STRIPE_SELECTED);

        app.world_mut().entity_mut(tab).insert(TabDragging {
            pointer_id: PointerId::Mouse,
        });
        app.update();
        assert_eq!(
            style(&app),
            (tokens::TAB_BG_DRAGGING, tokens::TAB_TEXT_DRAGGING)
        );
        assert_eq!(stripe(&app, tab), tokens::TAB_STRIPE);

        app.world_mut().entity_mut(list).insert(InteractionDisabled);
        app.update();
        assert_eq!(style(&app), (tokens::TAB_BG, tokens::TAB_TEXT_DISABLED));
        assert_eq!(
            app.world().entity(tab).get::<EntityCursor>(),
            Some(&EntityCursor::System(SystemCursorIcon::NotAllowed))
        );
    }

    #[test]
    fn insertion_indicator_follows_preview() {
        let mut app = tabs_app();
        let (list, tabs) = spawn_list(&mut app, 3);

        set_preview(&mut app, list, tabs[0], 1, 2);
        let placed = indicators(&mut app);
        assert_eq!(placed.len(), 1);
        assert_eq!(placed[0].0, tabs[2]);
        assert_eq!(placed[0].1.left, px(-INDICATOR_SIZE * 0.5));
        assert_eq!(
            app.world()
                .entity(list)
                .get::<ThemeBorderColor>()
                .unwrap()
                .0,
            tokens::TAB_STRIP_BORDER_PREVIEW
        );

        set_preview(&mut app, list, tabs[0], 2, 3);
        let placed = indicators(&mut app);
        assert_eq!(placed.len(), 1);
        assert_eq!(placed[0].0, tabs[2]);
        assert_eq!(placed[0].1.right, px(-INDICATOR_SIZE * 0.5));

        app.world_mut()
            .entity_mut(list)
            .remove::<TabInsertionPreview>();
        app.update();
        assert!(indicators(&mut app).is_empty());
        assert_eq!(
            app.world()
                .entity(list)
                .get::<ThemeBorderColor>()
                .unwrap()
                .0,
            tokens::TAB_STRIP_BORDER
        );
    }

    #[test]
    fn insertion_indicator_shows_on_the_side_nearest_the_pointer() {
        let mut app = tabs_app();
        let (list, tabs) = spawn_list(&mut app, 3);

        set_preview(&mut app, list, tabs[1], 1, 1);
        let placed = indicators(&mut app);
        assert_eq!(placed.len(), 1);
        assert_eq!(placed[0].0, tabs[1]);
        assert_eq!(placed[0].1.left, px(-INDICATOR_SIZE * 0.5));

        set_preview(&mut app, list, tabs[1], 1, 2);
        let placed = indicators(&mut app);
        assert_eq!(placed.len(), 1);
        assert_eq!(placed[0].0, tabs[2]);
        assert_eq!(placed[0].1.left, px(-INDICATOR_SIZE * 0.5));

        set_preview(&mut app, list, tabs[2], 2, 2);
        let placed = indicators(&mut app);
        assert_eq!(placed.len(), 1);
        assert_eq!(placed[0].0, tabs[2]);
        assert_eq!(placed[0].1.left, px(-INDICATOR_SIZE * 0.5));

        set_preview(&mut app, list, tabs[2], 2, 3);
        let placed = indicators(&mut app);
        assert_eq!(placed.len(), 1);
        assert_eq!(placed[0].0, tabs[2]);
        assert_eq!(placed[0].1.right, px(-INDICATOR_SIZE * 0.5));
    }

    #[test]
    fn insertion_indicator_shows_on_other_list() {
        let mut app = tabs_app();
        let (_, source_tabs) = spawn_list(&mut app, 1);
        let (list, tabs) = spawn_list(&mut app, 2);
        let (empty, _) = spawn_list(&mut app, 0);

        set_preview(&mut app, list, source_tabs[0], 0, 0);
        let placed = indicators(&mut app);
        assert_eq!(placed.len(), 1);
        assert_eq!(placed[0].0, tabs[0]);

        app.world_mut()
            .entity_mut(list)
            .remove::<TabInsertionPreview>();
        set_preview(&mut app, empty, source_tabs[0], 0, 0);
        let placed = indicators(&mut app);
        assert_eq!(placed.len(), 1);
        assert_eq!(placed[0].0, empty);
    }

    #[test]
    fn drag_proxy_copies_tab_header() {
        let mut app = tabs_app();
        let root = app
            .world_mut()
            .spawn((DragOverlayRoot, Node::default()))
            .id();
        let (list, tabs) = spawn_list(&mut app, 1);
        app.world_mut().entity_mut(list).insert(ChildOf(root));
        let tab = tabs[0];
        let caption = app
            .world_mut()
            .spawn((TabCaption, Node::default(), ChildOf(tab)))
            .id();
        let label = app
            .world_mut()
            .spawn((Text::new("Scene"), ChildOf(caption)))
            .id();
        app.world_mut().spawn((Text::new("x"), ChildOf(caption)));
        let mut pointer_state = PointerState::default();
        pointer_state
            .get_mut(PointerId::Touch(4), PointerButton::Primary)
            .dragging
            .insert(
                label,
                DragEntry {
                    start_pos: Vec2::ZERO,
                    latest_pos: Vec2::ZERO,
                },
            );
        app.insert_resource(pointer_state);

        app.world_mut().entity_mut(tab).insert(TabDragging {
            pointer_id: PointerId::Touch(4),
        });
        app.update();

        let (proxy, drag_proxy, children) = app
            .world_mut()
            .query_filtered::<(Entity, &DragProxy, &Children), With<FeathersTabDragProxy>>()
            .single(app.world())
            .map(|(proxy, drag_proxy, children)| (proxy, *drag_proxy, children.to_vec()))
            .unwrap();
        assert_eq!(drag_proxy.pointer_id, PointerId::Touch(4));
        let ghost = app.world().entity(proxy);
        assert_eq!(
            ghost.get::<Outline>().map(|outline| outline.width),
            Some(px(1))
        );
        assert!(ghost.contains::<BoxShadow>());
        let ghost_node = ghost.get::<Node>().unwrap();
        assert_eq!(ghost_node.width, Val::Auto);
        assert_eq!(ghost_node.overflow, Overflow::visible());
        assert_eq!(
            ghost.get::<ThemeBackgroundColor>().unwrap().0,
            tokens::TAB_DRAG_PROXY_BG
        );
        assert_eq!(
            ghost.get::<ThemeBorderColor>().unwrap().0,
            tokens::TAB_DRAG_PROXY_BORDER
        );
        assert_eq!(
            app.world().entity(proxy).get::<ChildOf>(),
            Some(&ChildOf(label))
        );
        let copied = children
            .iter()
            .map(|child| app.world().entity(*child).get::<Text>().unwrap().0.clone())
            .collect::<Vec<_>>();
        assert_eq!(copied, ["Scene", "x"]);
        assert!(children
            .iter()
            .all(|child| app.world().entity(*child).get::<Pickable>() == Some(&Pickable::IGNORE)));
        assert!(children.iter().all(|child| {
            let entity = app.world().entity(*child);
            entity.get::<TextLayout>().map(|layout| layout.linebreak) == Some(LineBreak::NoWrap)
                && entity.get::<Node>().unwrap().flex_shrink == 0.0
        }));
        assert!(!children.contains(&label));
        assert_eq!(
            app.world().entity(label).get::<ChildOf>(),
            Some(&ChildOf(caption))
        );
    }

    #[test]
    fn drag_proxy_needs_overlay_root() {
        let mut app = tabs_app();
        let (_, tabs) = spawn_list(&mut app, 1);
        app.world_mut().entity_mut(tabs[0]).insert(TabDragging {
            pointer_id: PointerId::Mouse,
        });
        app.update();
        assert!(app
            .world_mut()
            .query_filtered::<(), With<FeathersTabDragProxy>>()
            .iter(app.world())
            .next()
            .is_none());
    }

    #[test]
    fn accent_stripe_shows_only_on_selected_tab() {
        let mut app = tabs_app();
        let (_, tabs) = spawn_list(&mut app, 3);
        app.world_mut().entity_mut(tabs[1]).insert(Selected);
        app.update();
        let stripes = tabs
            .iter()
            .map(|tab| stripe(&app, *tab))
            .collect::<Vec<_>>();
        assert_eq!(
            stripes,
            [
                tokens::TAB_STRIPE,
                tokens::TAB_STRIPE_SELECTED,
                tokens::TAB_STRIPE
            ]
        );
        let node = app.world().entity(tabs[1]).get::<Node>().unwrap();
        assert_eq!(node.border, UiRect::top(px(STRIPE_SIZE)));
        assert_eq!(node.border_radius, BorderRadius::top(px(TAB_RADIUS)));

        app.world_mut().entity_mut(tabs[1]).remove::<Selected>();
        app.world_mut().entity_mut(tabs[2]).insert(Selected);
        app.update();
        assert_eq!(stripe(&app, tabs[1]), tokens::TAB_STRIPE);
        assert_eq!(stripe(&app, tabs[2]), tokens::TAB_STRIPE_SELECTED);
    }

    #[test]
    fn fillets_follow_selected_tab() {
        let mut app = tabs_app();
        let (list, tabs) = spawn_list(&mut app, 3);
        let fillets = tabs
            .iter()
            .map(|tab| spawn_fillets(&mut app, *tab))
            .collect::<Vec<_>>();
        let shown = |app: &App| {
            fillets
                .iter()
                .map(|pair| {
                    pair.map(|fillet| {
                        app.world().entity(fillet).get::<Node>().unwrap().display == Display::Flex
                    })
                })
                .collect::<Vec<_>>()
        };

        app.update();
        assert_eq!(shown(&app), [[false, false]; 3]);

        app.world_mut()
            .entity_mut(list)
            .insert(SelectedTab(Some(tabs[1])));
        app.update();
        assert_eq!(shown(&app), [[false, false], [true, true], [false, false]]);
        let start = app.world().entity(fillets[1][0]).get::<Node>().unwrap();
        assert_eq!(start.left, px(-FILLET_SIZE));
        assert_eq!(start.bottom, px(0));
        let end = app.world().entity(fillets[1][1]).get::<Node>().unwrap();
        assert_eq!(end.right, px(-FILLET_SIZE));

        app.world_mut().entity_mut(list).insert((
            SelectedTab(Some(tabs[0])),
            TabFillets {
                start: false,
                end: true,
            },
        ));
        app.update();
        assert_eq!(shown(&app), [[false, true], [false, false], [false, false]]);

        app.world_mut().entity_mut(tabs[0]).insert(TabDragging {
            pointer_id: PointerId::Mouse,
        });
        app.update();
        assert_eq!(shown(&app), [[false, false]; 3]);
    }

    #[test]
    fn fillets_paint_only_the_selected_flare() {
        let mut app = scene_app();
        app.insert_resource(UiTheme(crate::dark_theme::create_dark_theme()));
        let list = app
            .world_mut()
            .spawn_scene(bsn! {
                @FeathersTabList { @drag: TabDragMode::Reorder }
                Children [
                    @FeathersTab { @caption: bsn! { Text("Home") } }
                    --
                    @FeathersTab { @caption: bsn! { Text("Assets") } }
                    --
                    @FeathersTab { @caption: bsn! { Text("Scene") } }
                ]
            })
            .unwrap()
            .id();
        let tabs = app.world().entity(list).get::<Children>().unwrap().to_vec();
        let flare = app
            .world()
            .resource::<UiTheme>()
            .context_color(&tokens::TAB_BG_SELECTED, SurfaceLevel::Base);
        let check = |app: &mut App, selected: Entity, state: &str| {
            app.update();
            let world = app.world();
            for tab in world
                .entity(list)
                .get::<Children>()
                .unwrap()
                .iter()
                .copied()
            {
                let fillets = world
                    .entity(tab)
                    .get::<Children>()
                    .unwrap()
                    .iter()
                    .copied()
                    .filter(|child| world.entity(*child).contains::<TabFilletEdge>())
                    .collect::<Vec<_>>();
                assert_eq!(fillets.len(), 2, "{state}");
                for fillet in fillets {
                    let fillet = world.entity(fillet);
                    assert!(!fillet.contains::<ThemeBackgroundColor>(), "{state}");
                    assert!(
                        fillet
                            .get::<bevy_ui::BackgroundColor>()
                            .is_none_or(|bg| bg.0.is_fully_transparent()),
                        "{state}"
                    );
                    assert_eq!(fillet.get::<OuterColor>().unwrap().0, flare, "{state}");
                    let shown = fillet.get::<Node>().unwrap().display == Display::Flex;
                    assert_eq!(shown, tab == selected, "{state}");
                }
            }
        };

        app.world_mut()
            .entity_mut(list)
            .insert(SelectedTab(Some(tabs[1])));
        check(&mut app, tabs[1], "selected");

        app.world_mut().entity_mut(tabs[0]).insert(Hovered(true));
        check(&mut app, tabs[1], "neighbor hovered");
        app.world_mut().entity_mut(tabs[1]).insert(Hovered(true));
        check(&mut app, tabs[1], "selected hovered");

        app.insert_resource(bevy_input_focus::InputFocus::from_entity(tabs[0]));
        app.insert_resource(bevy_input_focus::InputFocusVisible(true));
        check(&mut app, tabs[1], "neighbor focused");

        app.world_mut()
            .entity_mut(list)
            .insert(SelectedTab(Some(tabs[2])));
        check(&mut app, tabs[2], "selection changed");

        app.world_mut()
            .entity_mut(list)
            .insert_children(0, &[tabs[2]]);
        check(&mut app, tabs[2], "reordered");

        app.world_mut().entity_mut(tabs[2]).insert(TabDragging {
            pointer_id: PointerId::Mouse,
        });
        check(&mut app, Entity::PLACEHOLDER, "dragging");
        app.world_mut().entity_mut(tabs[2]).remove::<TabDragging>();
        check(&mut app, tabs[2], "drag cancelled");
    }

    #[test]
    fn tabs_drag_between_external_strips_with_slots_and_captions() {
        use bevy_picking::{
            backend::HitData,
            events::{Pointer, PointerDrag, PointerDragEnd, PointerDragStart},
            hover::HoverMap,
            pointer::Location,
        };
        use bevy_ui::UiGlobalTransform;
        use bevy_ui_widgets::{tablist_self_update, TabMoved, TabPlugin};
        use bevy_window::{Window, WindowRef};

        fn apply_tab_move(
            moved: On<TabMoved>,
            children: Query<&Children>,
            tabs: Query<(), With<Tab>>,
            mut commands: Commands,
        ) {
            let siblings = children
                .get(moved.to_strip)
                .map(|children| children.to_vec())
                .unwrap_or_default();
            let index = FeathersTabList::child_index(
                &siblings,
                |entity| tabs.contains(entity),
                moved.tab,
                moved.index,
            );
            let mut strip = commands.entity(moved.to_strip);
            strip.insert_child(index, moved.tab);
            if moved.to_strip != moved.from_strip {
                strip.insert(SelectedTab(Some(moved.tab)));
            }
        }

        let mut app = scene_app();
        app.add_plugins(TabPlugin);
        app.init_resource::<bevy_input_focus::InputFocus>();
        let window = app.world_mut().spawn(Window::default()).id();
        let strip = |name: &'static str| {
            bsn! {
                @FeathersTabList { @drag: TabDragMode::External }
                on(tablist_self_update)
                on(apply_tab_move)
                Children [
                    @FeathersTabListLeading
                    --
                    @FeathersTab { @caption: bsn! { Text(name) } }
                    --
                    @FeathersTabListTrailing
                ]
            }
        };
        let source = app.world_mut().spawn_scene(strip("A")).unwrap().id();
        let destination = app.world_mut().spawn_scene(strip("B")).unwrap().id();
        app.update();

        let world = app.world();
        let tab_in = |list: Entity| world.entity(list).get::<Children>().unwrap()[1];
        let (dragged, target) = (tab_in(source), tab_in(destination));
        let label = |tab: Entity| {
            let caption = world.entity(tab).get::<Children>().unwrap()[1];
            world.entity(caption).get::<Children>().unwrap()[0]
        };
        let (dragged_label, target_label) = (label(dragged), label(target));
        app.world_mut()
            .entity_mut(dragged)
            .insert(UiGlobalTransform::from_xy(50.0, 10.0));
        app.world_mut()
            .entity_mut(target)
            .insert(UiGlobalTransform::from_xy(50.0, 40.0));

        let pointer = |position: Vec2| {
            Pointer::new(
                PointerId::Mouse,
                Location {
                    target: bevy_camera::NormalizedRenderTarget::Window(
                        WindowRef::Entity(window).normalize(Some(window)).unwrap(),
                    ),
                    position,
                },
            )
        };
        let hit = HitData::new(window, 0.0, None, None);
        let hover = |app: &mut App, entity: Entity| {
            let mut map = app.world_mut().resource_mut::<HoverMap>();
            map.clear();
            map.entry(PointerId::Mouse)
                .or_default()
                .insert(entity, hit.clone());
        };
        let start = Vec2::new(50.0, 10.0);
        let end = Vec2::new(80.0, 40.0);

        hover(&mut app, dragged_label);
        app.world_mut().trigger(PointerDragStart {
            entity: dragged_label,
            pointer: pointer(start),
            button: PointerButton::Primary,
            hit: hit.clone(),
        });
        app.update();
        hover(&mut app, target_label);
        app.world_mut().trigger(PointerDrag {
            entity: dragged_label,
            pointer: pointer(end),
            button: PointerButton::Primary,
            distance: end - start,
            delta: end - start,
        });
        app.update();
        let preview = app.world().entity(destination).get::<TabInsertionPreview>();
        assert_eq!(preview.map(|preview| preview.entries[0].index), Some(1));
        assert!(app.world().entity(dragged).contains::<TabDragging>());

        app.world_mut().trigger(PointerDragEnd {
            entity: dragged_label,
            pointer: pointer(end),
            button: PointerButton::Primary,
            distance: end - start,
        });
        app.update();
        app.update();

        let world = app.world();
        let kinds = |list: Entity| {
            world
                .entity(list)
                .get::<Children>()
                .map(|children| children.to_vec())
                .unwrap_or_default()
                .into_iter()
                .map(|child| match child {
                    _ if child == dragged => "dragged",
                    _ if world.entity(child).contains::<FeathersTabListLeading>() => "leading",
                    _ if world.entity(child).contains::<FeathersTabListTrailing>() => "trailing",
                    _ => "tab",
                })
                .collect::<Vec<_>>()
        };
        assert_eq!(kinds(source), ["leading", "trailing"]);
        assert_eq!(
            kinds(destination),
            ["leading", "tab", "dragged", "trailing"]
        );
        assert_eq!(
            world.entity(destination).get::<SelectedTab>(),
            Some(&SelectedTab(Some(dragged)))
        );
        assert!(world.entity(dragged).contains::<Selected>());
        assert!(!world.entity(dragged).contains::<TabDragging>());
        assert!(world
            .entity(destination)
            .get::<TabInsertionPreview>()
            .is_none());
    }

    #[test]
    fn slot_content_is_placed_in_the_strip() {
        let mut app = scene_app();
        let list = app
            .world_mut()
            .spawn_scene(bsn! {
                @FeathersTabList
                Children [
                    @FeathersTab { @caption: bsn! { Text("A") } }
                    --
                    @FeathersTabListTrailing
                    --
                    @FeathersTab { @caption: bsn! { Text("B") } }
                    --
                    @FeathersTabListLeading
                ]
            })
            .unwrap()
            .id();
        app.update();

        let world = app.world();
        let children = world.entity(list).get::<Children>().unwrap().to_vec();
        let kinds = children
            .iter()
            .map(|child| {
                let child = world.entity(*child);
                match () {
                    _ if child.contains::<FeathersTabListLeading>() => "leading",
                    _ if child.contains::<FeathersTab>() => "tab",
                    _ if child.contains::<FeathersTabListTrailing>() => "trailing",
                    _ => "other",
                }
            })
            .collect::<Vec<_>>();
        assert_eq!(kinds, ["leading", "tab", "tab", "trailing"]);
        assert_eq!(
            world.entity(children[3]).get::<Node>().unwrap().margin,
            UiRect::left(Val::Auto)
        );
        let is_tab = |entity: Entity| world.entity(entity).contains::<Tab>();
        assert_eq!(
            FeathersTabList::child_index(&children, is_tab, children[2], 0),
            1
        );
        assert_eq!(
            FeathersTabList::child_index(&children, is_tab, children[1], 1),
            2
        );
        assert_eq!(
            FeathersTabList::child_index(&children, is_tab, Entity::PLACEHOLDER, 2),
            3
        );
    }

    #[test]
    fn empty_horizontal_strip_is_as_tall_as_one_with_tabs() {
        let mut app = scene_app();
        let list = app
            .world_mut()
            .spawn_scene(bsn! { @FeathersTabList })
            .unwrap()
            .id();
        let tab = app
            .world_mut()
            .spawn_scene(bsn! { @FeathersTab })
            .unwrap()
            .id();
        app.update();

        let world = app.world();
        let list = world.entity(list).get::<Node>().unwrap();
        let tab = world.entity(tab).get::<Node>().unwrap();
        let (Val::Px(tab_height), Val::Px(min_height)) = (tab.min_height, list.min_height) else {
            panic!("expected pixel heights");
        };
        let Val::Px(top) = list.padding.top else {
            panic!("expected pixel padding");
        };
        let Val::Px(border) = list.border.top else {
            panic!("expected pixel border");
        };
        assert_eq!(min_height, top + border + tab_height);
    }

    #[test]
    fn tabs_shrink_and_clip_in_narrow_strips() {
        let mut app = scene_app();
        let list = app
            .world_mut()
            .spawn_scene(bsn! {
                @FeathersTabList
                Children [
                    @FeathersTabListLeading
                    --
                    @FeathersTab { @caption: bsn! { Text("A long tab caption") } }
                    --
                    @FeathersTabListTrailing
                ]
            })
            .unwrap()
            .id();
        app.update();

        let world = app.world();
        assert_eq!(
            world.entity(list).get::<Node>().unwrap().overflow,
            Overflow::clip_x()
        );
        let children = world.entity(list).get::<Children>().unwrap();
        let node = |entity: Entity| world.entity(entity).get::<Node>().unwrap();
        assert_eq!(node(children[0]).flex_shrink, 0.0);
        assert_eq!(node(children[2]).flex_shrink, 0.0);

        let tab = node(children[1]);
        assert!(tab.flex_shrink > 0.0);
        assert_eq!(tab.min_width, px(TAB_MIN_WIDTH));
        assert_eq!(tab.overflow, Overflow::visible());

        let caption = world.entity(children[1]).get::<Children>().unwrap()[1];
        let caption_node = node(caption);
        assert_eq!(caption_node.overflow, Overflow::clip_x());
        assert_eq!(caption_node.min_width, px(0));
        assert!(caption_node.flex_shrink > 0.0);
        let text = world.entity(caption).get::<Children>().unwrap()[0];
        assert_eq!(
            world.entity(text).get::<TextLayout>().unwrap().linebreak,
            LineBreak::NoWrap
        );

        let vertical = app
            .world_mut()
            .spawn_scene(bsn! {
                @FeathersTabList { @orientation: ControlOrientation::Vertical }
            })
            .unwrap()
            .id();
        assert_eq!(
            app.world().entity(vertical).get::<Node>().unwrap().overflow,
            Overflow::clip_y()
        );
    }
}
