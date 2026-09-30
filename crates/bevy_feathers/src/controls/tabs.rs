use bevy_app::{Plugin, PostUpdate};
use bevy_ecs::{
    component::Component,
    entity::Entity,
    hierarchy::{ChildOf, Children},
    lifecycle::Add,
    observer::On,
    query::{Has, Or, With},
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
use bevy_text::FontWeight;
use bevy_ui::{
    px, AlignItems, BorderRadius, ComputedNode, Display, FlexDirection, GlobalZIndex,
    InteractionDisabled, Node, PositionType, Selected, UiRect, UiSystems, Val,
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
    theme::{InheritableThemeTextColor, ThemeBackgroundColor, ThemeBorderColor},
    tokens,
};

const TAB_PADDING: f32 = 10.0;
const TAB_GAP: f32 = 6.0;
const TAB_RADIUS: f32 = 4.0;
const INDICATOR_SIZE: f32 = 2.0;
const DRAG_PROXY_OFFSET: Vec2 = Vec2::new(10.0, 10.0);
const DRAG_PROXY_Z: i32 = 200;

/// A themed strip of [`FeathersTab`]s.
///
/// A more complete explanation of how to control this widget can be found in the documentation
/// for [`TabList`] and [`bevy_ui_widgets`]. Selection changes and drag moves are proposed with
/// events and applied by the app.
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
}

impl Default for FeathersTabListProps {
    fn default() -> Self {
        Self {
            orientation: ControlOrientation::Horizontal,
            activation: TabActivation::Manual,
            drag: TabDragMode::Disabled,
            selected: OptionTemplate::None,
        }
    }
}

impl FeathersTabList {
    /// Scene function for a tab list.
    pub fn scene(props: FeathersTabListProps) -> impl Scene {
        let flex_direction = match props.orientation {
            ControlOrientation::Horizontal => FlexDirection::Row,
            ControlOrientation::Vertical => FlexDirection::Column,
        };
        bsn! {
            Node {
                display: Display::Flex,
                flex_direction: {flex_direction},
                align_items: AlignItems::Stretch,
                min_height: size::ROW_HEIGHT,
                border: px(1),
            }
            TabList {
                orientation: {props.orientation},
                activation: {props.activation},
                drag: {props.drag},
            }
            SelectedTab({props.selected})
            ThemeBackgroundColor(tokens::TAB_STRIP_BG)
            ThemeBorderColor(tokens::TAB_STRIP_BORDER)
        }
    }
}

/// A themed tab header inside a [`FeathersTabList`].
///
/// The caption may contain text, icons and app-owned controls such as a close button.
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
        bsn! {
            Node {
                display: Display::Flex,
                min_height: size::ROW_HEIGHT,
                align_items: AlignItems::Center,
                padding: UiRect::horizontal(px(TAB_PADDING)),
                column_gap: px(TAB_GAP),
                border_radius: BorderRadius::all(px(TAB_RADIUS)),
            }
            Tab
            Hovered
            EntityCursor::System(SystemCursorIcon::Pointer)
            FocusIndicator
            ThemeBackgroundColor(tokens::TAB_BG)
            InheritableThemeTextColor(tokens::TAB_TEXT)
            InheritableFont {
                font: fonts::REGULAR,
                font_size: size::MEDIUM_FONT,
                weight: FontWeight::NORMAL,
            }
            Children [
                {props.caption}
            ]
        }
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

/// A semi-transparent copy of a dragged [`FeathersTab`] that follows the pointer.
///
/// Spawned by [`FeathersTabsPlugin`] when a tab starts dragging and moved beneath the nearest
/// [`DragOverlayRoot`]. It holds copies of the tab's children and is despawned with its
/// [`DragProxy`] when the drag ends.
#[derive(Component, Debug, Default, Clone, Copy, Reflect)]
#[reflect(Component, Clone, Default)]
pub struct FeathersTabDragProxy;

fn update_tab_styles(
    tabs: Query<
        (
            Entity,
            Option<&ChildOf>,
            &Hovered,
            Has<Selected>,
            Has<InteractionDisabled>,
            Has<TabDragging>,
            &ThemeBackgroundColor,
            &InheritableThemeTextColor,
            Option<&EntityCursor>,
        ),
        With<FeathersTab>,
    >,
    lists: Query<
        (
            Entity,
            Has<InteractionDisabled>,
            Has<TabInsertionPreview>,
            &ThemeBorderColor,
        ),
        With<FeathersTabList>,
    >,
    mut commands: Commands,
) {
    for (tab, parent, hovered, selected, tab_disabled, dragging, bg, text, cursor) in &tabs {
        let list_disabled = parent
            .and_then(|parent| lists.get(parent.parent()).ok())
            .is_some_and(|(_, disabled, _, _)| disabled);
        let disabled = tab_disabled || list_disabled;

        let bg_token = match (disabled, dragging, selected, hovered.0) {
            (false, true, _, _) => tokens::TAB_BG_DRAGGING,
            (false, false, true, _) => tokens::TAB_BG_SELECTED,
            (false, false, false, true) => tokens::TAB_BG_HOVER,
            _ => tokens::TAB_BG,
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

        if bg.0 != bg_token {
            commands.entity(tab).insert(ThemeBackgroundColor(bg_token));
        }
        if text.0 != text_token {
            commands
                .entity(tab)
                .insert(InheritableThemeTextColor(text_token));
        }
        if cursor != Some(&cursor_shape) {
            commands.entity(tab).insert(cursor_shape);
        }
    }

    for (list, _, preview, border) in &lists {
        let border_token = match preview {
            true => tokens::TAB_STRIP_BORDER_PREVIEW,
            false => tokens::TAB_STRIP_BORDER,
        };
        if border.0 != border_token {
            commands.entity(list).insert(ThemeBorderColor(border_token));
        }
    }
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
        for entry in &preview.entries {
            let remaining = children
                .into_iter()
                .flatten()
                .copied()
                .filter(|child| *child != entry.tab && tabs.contains(*child))
                .collect::<Vec<_>>();
            let (host, node) =
                indicator_placement(list, tablist.orientation, &remaining, entry.index);
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

/// Returns the entity that hosts an insertion line at `index` among `tabs`, and the line's node.
fn indicator_placement(
    list: Entity,
    orientation: ControlOrientation,
    tabs: &[Entity],
    index: usize,
) -> (Entity, Node) {
    let (host, at_end) = match tabs.get(index) {
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
        )>,
    >,
    pointer_state: Option<Res<PointerState>>,
    mut commands: Commands,
) {
    let tab = add.entity;
    let Ok((dragging, computed, font, children)) = tabs.get(tab) else {
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
            width: length(size.x),
            height: length(size.y),
            min_height: size::ROW_HEIGHT,
            align_items: AlignItems::Center,
            padding: UiRect::horizontal(px(TAB_PADDING)),
            column_gap: px(TAB_GAP),
            border: UiRect::all(px(1)),
            border_radius: BorderRadius::all(px(TAB_RADIUS)),
            ..Default::default()
        },
        GlobalZIndex(DRAG_PROXY_Z),
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
            (update_tab_styles, update_insertion_indicators).in_set(UiSystems::Content),
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
                        InheritableThemeTextColor(tokens::TAB_TEXT),
                        ChildOf(list),
                    ))
                    .id()
            })
            .collect();
        (list, tabs)
    }

    fn set_preview(app: &mut App, list: Entity, tab: Entity, index: usize) {
        app.world_mut()
            .entity_mut(list)
            .insert(TabInsertionPreview {
                entries: vec![TabInsertionPoint {
                    pointer_id: PointerId::Mouse,
                    tab,
                    index,
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

    #[test]
    fn scene_applies_props_and_named_selection() {
        let mut app = App::new();
        app.add_plugins((
            TaskPoolPlugin::default(),
            AssetPlugin::default(),
            ScenePlugin,
        ));
        app.init_asset::<bevy_text::Font>();
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
        let caption = tab.get::<Children>().unwrap()[0];
        assert_eq!(world.entity(caption).get::<Text>().unwrap().0, "Scene");
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

        app.world_mut().entity_mut(tab).insert(TabDragging {
            pointer_id: PointerId::Mouse,
        });
        app.update();
        assert_eq!(
            style(&app),
            (tokens::TAB_BG_DRAGGING, tokens::TAB_TEXT_DRAGGING)
        );

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

        set_preview(&mut app, list, tabs[0], 1);
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

        set_preview(&mut app, list, tabs[0], 2);
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
    fn insertion_indicator_shows_on_other_list() {
        let mut app = tabs_app();
        let (_, source_tabs) = spawn_list(&mut app, 1);
        let (list, tabs) = spawn_list(&mut app, 2);
        let (empty, _) = spawn_list(&mut app, 0);

        set_preview(&mut app, list, source_tabs[0], 0);
        let placed = indicators(&mut app);
        assert_eq!(placed.len(), 1);
        assert_eq!(placed[0].0, tabs[0]);

        app.world_mut()
            .entity_mut(list)
            .remove::<TabInsertionPreview>();
        set_preview(&mut app, empty, source_tabs[0], 0);
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
        let label = app
            .world_mut()
            .spawn((Text::new("Scene"), ChildOf(tab)))
            .id();
        app.world_mut().spawn((Text::new("x"), ChildOf(tab)));
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
        assert!(!children.contains(&label));
        assert_eq!(
            app.world().entity(label).get::<ChildOf>(),
            Some(&ChildOf(tab))
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
}
