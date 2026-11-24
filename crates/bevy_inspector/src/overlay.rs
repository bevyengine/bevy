//! A hidden inspector that opens over any app when a key is pressed.
//!
//! [`InspectorOverlayPlugin`] is part of `DefaultPlugins` when the `inspector` cargo feature is
//! enabled, so the app needs no code of its own. Its panels are not spawned until they are first
//! shown, and they render through the app's default UI camera, so a camera with a viewport limits
//! them to that viewport.
//!
//! The panels are tabs of a dock that fills the window, with the entity tree and the resource list
//! on the left and the details on the right. The group between them has no tabs and shows the
//! app, and the pointer reaches the app or picks for the inspector there, depending on the
//! [`InspectorMode`]. Tabs can be dragged between groups or to the edge of one, and a draggable
//! toolbar switches the mode.

use bevy_app::{App, Plugin, PostUpdate, PreUpdate, Update};
use bevy_camera::visibility::Visibility;
use bevy_ecs::{
    change_detection::DetectChangesMut,
    component::Component,
    entity::Entity,
    event::EntityEvent,
    hierarchy::Children,
    name::Name,
    observer::On,
    query::{Added, Changed, Has, With},
    resource::Resource,
    schedule::IntoScheduleConfigs,
    system::{Commands, Query, Res, ResMut},
    template::template,
};
use bevy_feathers::{
    constants::icons,
    controls::{ButtonVariant, FeathersDock, FeathersToolButton},
    dark_theme::create_dark_theme,
    display::{caption, icon},
    theme::{InheritableThemeTextColor, ThemeBackgroundColor, ThemeBorderColor, UiTheme},
    tokens, FeathersPlugins,
};
use bevy_input::{keyboard::KeyCode, ButtonInput, InputSystems};
use bevy_log::warn;
use bevy_picking::{cursor::EntityCursor, Pickable, PickingSystems};
use bevy_scene::prelude::*;
use bevy_ui::{
    percent, px, AlignItems, BackgroundColor, BorderRadius, DefaultUiCamera, Display,
    FlexDirection, GlobalZIndex, Node, PositionType, UiRect,
};
use bevy_ui_widgets::{
    dock_self_update, Activate, Dialog, DialogDragHandle, DockEdge, DockError, DockNodeBinding,
    DockPanelContent, DockPanelTitle, DockSystems, DockTree, DragOverlayRoot, SplitPane, TabList,
};
use bevy_window::SystemCursorIcon;

use crate::{
    details_panel::{details_body, DetailsCollapsed},
    entity_tree::{entity_tree_body, InspectorUi},
    resource_list::resource_list_body,
    InspectorMode, InspectorPlugin,
};

/// Adds the inspector to an app as an overlay, hidden until [`toggle_key`](Self::toggle_key) is
/// pressed.
///
/// The inspector only walks the world while a view of it is showing.
pub struct InspectorOverlayPlugin {
    /// The key that shows and hides the overlay.
    pub toggle_key: KeyCode,
}

impl Default for InspectorOverlayPlugin {
    fn default() -> Self {
        Self {
            toggle_key: KeyCode::F12,
        }
    }
}

impl Plugin for InspectorOverlayPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((FeathersPlugins, InspectorPlugin))
            .insert_resource(InspectorOverlayKey(self.toggle_key))
            .add_systems(
                PreUpdate,
                toggle_inspector_overlay
                    .after(InputSystems)
                    .before(PickingSystems::Last),
            )
            .add_systems(Update, sync_mode_toggles)
            .add_systems(
                PostUpdate,
                (build_overlay_panels, see_through_empty_groups).in_set(DockSystems::Panels),
            );
        app.world_mut()
            .resource_mut::<DetailsCollapsed>()
            .by_default = true;
    }
}

#[derive(Resource)]
struct InspectorOverlayKey(KeyCode);

const OVERLAY_Z: i32 = 40;

/// Marker for the root entity of the inspector overlay.
#[derive(Component, Clone, Copy, Debug, Default)]
pub struct InspectorOverlay;

#[derive(Component, Clone, Copy, Debug, Default)]
pub(crate) struct OverlayDock;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum InspectorPanel {
    Entities,
    Resources,
    Details,
}

impl InspectorPanel {
    pub(crate) const fn key(self) -> &'static str {
        match self {
            InspectorPanel::Entities => "inspector.entities",
            InspectorPanel::Resources => "inspector.resources",
            InspectorPanel::Details => "inspector.details",
        }
    }

    fn from_key(key: &str) -> Option<Self> {
        [
            InspectorPanel::Entities,
            InspectorPanel::Resources,
            InspectorPanel::Details,
        ]
        .into_iter()
        .find(|panel| panel.key() == key)
    }

    const fn title(self) -> &'static str {
        match self {
            InspectorPanel::Entities => "Entities",
            InspectorPanel::Resources => "Resources",
            InspectorPanel::Details => "Details",
        }
    }
}

#[derive(Component, Clone)]
struct SeeThroughGroup {
    background: Option<ThemeBackgroundColor>,
    strip_display: Display,
}

#[derive(Component, Clone, Copy, Debug, Default)]
struct ModeToggle(InspectorMode);

fn toggle_inspector_overlay(
    key: Res<InspectorOverlayKey>,
    keys: Option<Res<ButtonInput<KeyCode>>>,
    mut overlays: Query<&mut Visibility, With<InspectorOverlay>>,
    mut theme: ResMut<UiTheme>,
    ui_camera: DefaultUiCamera,
    mut commands: Commands,
) {
    if !keys.is_some_and(|keys| keys.just_pressed(key.0)) {
        return;
    }
    if let Ok(mut visibility) = overlays.single_mut() {
        *visibility = match *visibility {
            Visibility::Hidden => Visibility::Inherited,
            _ => Visibility::Hidden,
        };
        return;
    }
    if theme.0.token_assignments.is_empty() {
        theme.0 = create_dark_theme();
    }
    if ui_camera.get().is_none() {
        warn!("The inspector overlay renders through the default UI camera, but there is none.");
    }
    commands.spawn_scene(overlay_scene());
}

fn on_mode_toggle(
    activate: On<Activate>,
    toggles: Query<&ModeToggle>,
    mut mode: ResMut<InspectorMode>,
) {
    if let Ok(toggle) = toggles.get(activate.event_target()) {
        mode.set_if_neq(toggle.0);
    }
}

fn sync_mode_toggles(
    mode: Res<InspectorMode>,
    mut toggles: Query<(&ModeToggle, &mut ButtonVariant)>,
) {
    for (toggle, mut variant) in &mut toggles {
        variant.set_if_neq(ButtonVariant::selected(toggle.0 == *mode));
    }
}

fn build_overlay_panels(
    contents: Query<(Entity, &DockPanelContent), Added<DockPanelContent>>,
    docks: Query<(), With<OverlayDock>>,
    mut commands: Commands,
) {
    for (content, panel) in &contents {
        if !docks.contains(panel.dock) {
            continue;
        }
        let Some(kind) = InspectorPanel::from_key(panel.panel.as_str()) else {
            continue;
        };
        let mut entity = commands.entity(content);
        entity.insert(DockPanelTitle::new(kind.title()));
        match kind {
            InspectorPanel::Entities => entity.apply_scene(bsn! {
                Children [
                    @entity_tree_body()
                    Node { flex_grow: 1.0, min_width: px(0) }
                ]
            }),
            InspectorPanel::Resources => entity.apply_scene(bsn! {
                Children [
                    @resource_list_body()
                    Node { flex_grow: 1.0, min_width: px(0) }
                ]
            }),
            InspectorPanel::Details => entity.apply_scene(bsn! {
                Children [
                    @details_body()
                    Node { flex_grow: 1.0, min_width: px(0) }
                ]
            }),
        };
    }
}

fn see_through_empty_groups(
    docks: Query<(Entity, &DockTree), (With<OverlayDock>, Changed<DockTree>)>,
    groups: Query<(
        Entity,
        &DockNodeBinding,
        Has<SplitPane>,
        Option<&Children>,
        Option<&ThemeBackgroundColor>,
        Option<&SeeThroughGroup>,
    )>,
    tab_lists: Query<(), With<TabList>>,
    pickables: Query<&Pickable>,
    mut nodes: Query<&mut Node>,
    mut commands: Commands,
) {
    for (dock, tree) in &docks {
        for (group, binding, is_split, children, background, saved) in &groups {
            if binding.dock != dock {
                continue;
            }
            if is_split {
                if pickables.get(group) != Ok(&Pickable::IGNORE) {
                    commands.entity(group).insert(Pickable::IGNORE);
                }
                continue;
            }
            let empty = tree
                .leaf(binding.node)
                .is_some_and(|leaf| leaf.tabs().is_empty());
            if empty == saved.is_some() {
                continue;
            }
            let mut strip_display = Display::None;
            for &child in children.iter().flat_map(|children| children.iter()) {
                if !tab_lists.contains(child) {
                    pass_pointer(&mut commands, child, empty);
                } else if let Ok(mut node) = nodes.get_mut(child) {
                    let display = saved.map_or(Display::None, |saved| saved.strip_display);
                    strip_display = core::mem::replace(&mut node.display, display);
                }
            }
            pass_pointer(&mut commands, group, empty);
            let mut group = commands.entity(group);
            match saved {
                None => {
                    group
                        .remove::<(ThemeBackgroundColor, BackgroundColor)>()
                        .insert(SeeThroughGroup {
                            background: background.cloned(),
                            strip_display,
                        });
                }
                Some(saved) => {
                    group.remove::<SeeThroughGroup>();
                    if let Some(background) = saved.background.clone() {
                        group.insert(background);
                    }
                }
            }
        }
    }
}

fn pass_pointer(commands: &mut Commands, entity: Entity, pass: bool) {
    if pass {
        commands.entity(entity).insert(Pickable::IGNORE);
    } else {
        commands.entity(entity).remove::<Pickable>();
    }
}

fn overlay_dock_tree() -> Result<DockTree, DockError> {
    let mut tree = DockTree::new();
    let center = tree.root();
    tree.set_persistent(center, true)?;
    let (left, entities) = tree.split(center, DockEdge::Left, InspectorPanel::Entities.key())?;
    tree.add_tab(left, InspectorPanel::Resources.key())?;
    tree.set_active(left, entities)?;
    tree.split(center, DockEdge::Right, InspectorPanel::Details.key())?;
    tree.set_sizes(tree.root(), &[1.0, 3.0, 1.3])?;
    Ok(tree)
}

fn overlay_scene() -> impl Scene {
    bsn! {
        InspectorOverlay
        InspectorUi
        Name::new("Inspector")
        GlobalZIndex(OVERLAY_Z)
        Node {
            position_type: PositionType::Absolute,
            width: percent(100),
            height: percent(100),
        }
        DragOverlayRoot
        Pickable::IGNORE
        Children [
            OverlayDock
            FeathersDock
            template(|_| Ok(overlay_dock_tree()?))
            Node {
                flex_grow: 1.0,
                min_width: px(0),
                min_height: px(0),
            }
            InheritableThemeTextColor(tokens::TEXT_MAIN)
            Pickable::IGNORE
            on(dock_self_update)
            --
            @toolbar()
        ]
    }
}

fn toolbar() -> impl Scene {
    bsn! {
        Dialog
        Node {
            position_type: PositionType::Absolute,
            left: percent(20),
            top: px(8),
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            column_gap: px(6),
            padding: UiRect::all(px(3)),
            border: UiRect::all(px(1)),
            border_radius: BorderRadius::all(px(4)),
        }
        ThemeBackgroundColor(tokens::DIALOG_BG)
        ThemeBorderColor(tokens::DIALOG_BORDER)
        InheritableThemeTextColor(tokens::DIALOG_TEXT)
        Children [
            @FeathersToolButton {
                @caption: bsn! { @caption("Edit") },
            }
            ModeToggle(InspectorMode::Edit)
            on(on_mode_toggle)
            --
            @FeathersToolButton {
                @caption: bsn! { @caption("Preview") },
            }
            ModeToggle(InspectorMode::Preview)
            on(on_mode_toggle)
            --
            DialogDragHandle
            EntityCursor::System(SystemCursorIcon::Grab)
            Node {
                align_items: AlignItems::Center,
            }
            Children [
                @icon(icons::GRIP_VERTICAL)
            ]
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        details_panel_visible,
        entity_tree::{EntityTreeSync, InspectorEntityRow, InspectorHeaderRow, InspectorTreeView},
        entity_tree_visible,
        resource_list::{InspectorResourceList, InspectorResourceRow, ResourceListSync},
        resource_list_visible, InspectableEntity, InspectorSelection, InspectorSource,
    };
    use bevy_app::TaskPoolPlugin;
    use bevy_asset::{AssetApp, AssetPlugin};
    use bevy_ecs::{
        entity::Entity, hierarchy::ChildOf, name::Name, prelude::*, system::RunSystemOnce,
    };
    use bevy_feathers::{
        controls::{FeathersDockPlugin, FeathersTabsPlugin, SplitPanePlugin},
        theme::UiTheme,
        FeathersSystems,
    };
    use bevy_ui::UiSystems;
    use bevy_ui_widgets::{
        DockChange, DockEdit, DockNodeId, DockPlugin, SelectedTreeItem, TabPlugin, ValueChange,
    };

    fn test_app() -> App {
        let mut app = App::new();
        app.add_plugins((
            TaskPoolPlugin::default(),
            bevy_time::TimePlugin,
            AssetPlugin::default(),
            bevy_scene::ScenePlugin,
            TabPlugin,
            DockPlugin,
            FeathersTabsPlugin,
            SplitPanePlugin,
            FeathersDockPlugin,
            InspectorPlugin,
        ));
        app.init_asset::<bevy_image::Image>();
        app.init_asset::<bevy_text::Font>();
        app.init_resource::<UiTheme>();
        app.init_resource::<ButtonInput<KeyCode>>();
        app.insert_resource(InspectorOverlayKey(KeyCode::F12));
        app.add_systems(
            Update,
            (toggle_inspector_overlay, sync_mode_toggles).chain(),
        );
        app.add_systems(
            PostUpdate,
            (build_overlay_panels, see_through_empty_groups).in_set(DockSystems::Panels),
        );
        app.configure_sets(PostUpdate, (UiSystems::Prepare, UiSystems::Content).chain());
        app.configure_sets(PostUpdate, FeathersSystems.in_set(UiSystems::Content));
        app
    }

    fn press(app: &mut App) {
        let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
        keys.press(KeyCode::F12);
        app.update();
        let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
        keys.release(KeyCode::F12);
        keys.clear();
    }

    fn overlays(app: &mut App) -> Vec<Entity> {
        let world = app.world_mut();
        world
            .query_filtered::<Entity, With<InspectorOverlay>>()
            .iter(world)
            .collect()
    }

    fn row_count(app: &mut App, source: Entity) -> usize {
        let world = app.world_mut();
        world
            .query::<&InspectorEntityRow>()
            .iter(world)
            .filter(|row| row.inspected == source)
            .count()
    }

    fn shows(app: &mut App, panel: InspectorPanel) -> bool {
        let world = app.world_mut();
        match panel {
            InspectorPanel::Entities => world.run_system_once(entity_tree_visible),
            InspectorPanel::Resources => world.run_system_once(resource_list_visible),
            InspectorPanel::Details => world.run_system_once(details_panel_visible),
        }
        .unwrap()
    }

    fn dock_tree(app: &mut App) -> DockTree {
        let dock = single::<With<OverlayDock>>(app);
        app.world().get::<DockTree>(dock).unwrap().clone()
    }

    fn edit_dock(app: &mut App, edit: DockEdit) {
        let dock = single::<With<OverlayDock>>(app);
        app.world_mut().trigger(DockChange {
            dock,
            edit,
            is_final: true,
        });
        app.update();
    }

    fn activate(app: &mut App, panel: InspectorPanel) {
        let (leaf, tab) = dock_tree(app).find_panel(panel.key()).unwrap();
        edit_dock(app, DockEdit::Activate { leaf, tab });
    }

    fn group(app: &mut App, node: DockNodeId) -> Entity {
        let world = app.world_mut();
        world
            .query::<(Entity, &DockNodeBinding)>()
            .iter(world)
            .find(|(_, binding)| binding.node == node)
            .unwrap()
            .0
    }

    fn see_through(app: &mut App, group: Entity) -> bool {
        let world = app.world();
        let ignores = |entity: Entity| world.get::<Pickable>(entity) == Some(&Pickable::IGNORE);
        let children = world.get::<Children>(group).unwrap();
        ignores(group)
            && world.get::<BackgroundColor>(group).is_none()
            && children.iter().all(|child| {
                if world.get::<TabList>(child).is_some() {
                    world.get::<Node>(child).unwrap().display == Display::None
                } else {
                    ignores(child)
                }
            })
    }

    #[test]
    fn nothing_is_spawned_until_the_key_is_pressed() {
        let mut app = test_app();
        app.update();
        app.update();
        assert!(overlays(&mut app).is_empty());
        assert!(!shows(&mut app, InspectorPanel::Entities));
        assert!(app
            .world()
            .resource::<UiTheme>()
            .0
            .token_assignments
            .is_empty());
    }

    #[test]
    fn the_key_shows_and_hides_the_overlay() {
        let mut app = test_app();
        press(&mut app);
        let overlay = overlays(&mut app)[0];
        assert!(!app
            .world()
            .resource::<UiTheme>()
            .0
            .token_assignments
            .is_empty());
        assert!(shows(&mut app, InspectorPanel::Entities));

        press(&mut app);
        assert_eq!(overlays(&mut app), [overlay]);
        assert_eq!(
            app.world().get::<Visibility>(overlay),
            Some(&Visibility::Hidden)
        );
        assert!(!shows(&mut app, InspectorPanel::Entities));

        press(&mut app);
        assert_eq!(overlays(&mut app), [overlay]);
        assert_eq!(
            app.world().get::<Visibility>(overlay),
            Some(&Visibility::Inherited)
        );
    }

    #[test]
    fn the_dock_opens_with_the_world_views_left_an_empty_center_and_the_details_right() {
        let mut app = test_app();
        press(&mut app);
        let tree = dock_tree(&mut app);
        let root = tree.node(tree.root()).unwrap().as_split().unwrap();
        let groups: Vec<(Vec<&str>, Option<&str>)> = root
            .children()
            .iter()
            .map(|child| {
                let leaf = tree.leaf(child.node).unwrap();
                (
                    leaf.tabs().iter().map(|tab| tab.panel.as_str()).collect(),
                    leaf.active_tab().map(|tab| tab.panel.as_str()),
                )
            })
            .collect();
        let entities = InspectorPanel::Entities.key();
        let details = InspectorPanel::Details.key();
        assert_eq!(
            groups,
            [
                (
                    vec![entities, InspectorPanel::Resources.key()],
                    Some(entities)
                ),
                (vec![], None),
                (vec![details], Some(details)),
            ]
        );
        assert!(tree.leaf(root.children()[1].node).unwrap().is_persistent());
        let sizes: Vec<f32> = root.children().iter().map(|child| child.size).collect();
        assert!(sizes[1] > sizes[0] && sizes[1] > sizes[2]);

        let world = app.world_mut();
        let titles: Vec<(String, String)> = world
            .query::<(&DockPanelContent, &DockPanelTitle)>()
            .iter(world)
            .map(|(content, title)| (content.panel.as_str().into(), title.0.clone()))
            .collect();
        assert_eq!(titles.len(), 3);
        for (key, title) in titles {
            assert_eq!(InspectorPanel::from_key(&key).unwrap().title(), title);
        }
        assert!(shows(&mut app, InspectorPanel::Entities));
        assert!(!shows(&mut app, InspectorPanel::Resources));
        assert!(shows(&mut app, InspectorPanel::Details));
    }

    #[test]
    fn rows_are_current_on_the_frame_the_overlay_opens() {
        let mut app = test_app();
        let subject = app.world_mut().spawn(Name::new("Subject")).id();
        app.update();
        assert_eq!(row_count(&mut app, subject), 0);
        press(&mut app);
        assert_eq!(row_count(&mut app, subject), 1);
    }

    #[test]
    fn hidden_overlay_does_not_sync_and_catches_up_when_shown() {
        let mut app = test_app();
        press(&mut app);
        press(&mut app);
        let subject = app.world_mut().spawn(Name::new("Late")).id();
        app.world_mut().resource_mut::<EntityTreeSync>().set_dirty();
        app.update();
        assert_eq!(row_count(&mut app, subject), 0);
        press(&mut app);
        assert_eq!(row_count(&mut app, subject), 1);
    }

    fn click(app: &mut App, button: Entity) {
        app.world_mut().trigger(Activate { entity: button });
        app.update();
    }

    #[test]
    fn dock_tabs_switch_between_the_entity_tree_and_the_resource_list() {
        let mut app = test_app();
        press(&mut app);
        assert!(shows(&mut app, InspectorPanel::Entities));
        assert!(!shows(&mut app, InspectorPanel::Resources));

        activate(&mut app, InspectorPanel::Resources);
        assert!(!shows(&mut app, InspectorPanel::Entities));
        assert!(shows(&mut app, InspectorPanel::Resources));
        let world = app.world_mut();
        assert!(world
            .query::<&InspectorResourceRow>()
            .iter(world)
            .next()
            .is_some());

        activate(&mut app, InspectorPanel::Entities);
        assert!(shows(&mut app, InspectorPanel::Entities));
        assert!(!shows(&mut app, InspectorPanel::Resources));
    }

    #[test]
    fn an_inactive_entities_tab_does_not_sync_and_catches_up_when_shown() {
        let mut app = test_app();
        press(&mut app);
        activate(&mut app, InspectorPanel::Resources);
        let subject = app.world_mut().spawn(Name::new("Late")).id();
        app.world_mut().resource_mut::<EntityTreeSync>().set_dirty();
        app.update();
        assert_eq!(row_count(&mut app, subject), 0);

        activate(&mut app, InspectorPanel::Entities);
        assert_eq!(row_count(&mut app, subject), 1);
    }

    #[test]
    fn a_tab_moved_to_another_group_keeps_its_content_working() {
        let mut app = test_app();
        press(&mut app);
        let list = single::<With<InspectorResourceList>>(&mut app);
        let tree = dock_tree(&mut app);
        let (right, _) = tree.find_panel(InspectorPanel::Details.key()).unwrap();
        let (_, resources) = tree.find_panel(InspectorPanel::Resources.key()).unwrap();
        edit_dock(
            &mut app,
            DockEdit::MoveTab {
                tab: resources,
                to_leaf: right,
                index: None,
            },
        );
        assert_eq!(single::<With<InspectorResourceList>>(&mut app), list);
        assert!(shows(&mut app, InspectorPanel::Entities));
        assert!(shows(&mut app, InspectorPanel::Resources));
        assert!(!shows(&mut app, InspectorPanel::Details));

        app.insert_resource(Marker);
        sync_resources(&mut app);
        let row = resource_row(&mut app).unwrap();
        select_row(&mut app, list, row);
        assert_eq!(
            app.world().get::<SelectedTreeItem>(list).unwrap().0,
            Some(row)
        );
    }

    fn move_tab(app: &mut App, panel: InspectorPanel, to_leaf: DockNodeId) {
        let (_, tab) = dock_tree(app).find_panel(panel.key()).unwrap();
        edit_dock(
            app,
            DockEdit::MoveTab {
                tab,
                to_leaf,
                index: None,
            },
        );
    }

    fn move_to_edge(app: &mut App, panel: InspectorPanel, to_node: DockNodeId, edge: DockEdge) {
        let (_, tab) = dock_tree(app).find_panel(panel.key()).unwrap();
        edit_dock(app, DockEdit::MoveToEdge { tab, to_node, edge });
    }

    #[test]
    fn the_center_group_stays_and_shows_the_app_while_it_has_no_tabs() {
        let mut app = test_app();
        press(&mut app);
        let tree = dock_tree(&mut app);
        let center = tree
            .leaves()
            .find(|(_, leaf)| leaf.tabs().is_empty())
            .unwrap()
            .0;
        let (left, _) = tree.find_panel(InspectorPanel::Entities.key()).unwrap();
        let center_group = group(&mut app, center);
        let left_group = group(&mut app, left);
        let split = group(&mut app, tree.root());
        let pane_background = app
            .world()
            .get::<ThemeBackgroundColor>(left_group)
            .unwrap()
            .0
            .clone();
        assert_eq!(app.world().get::<Pickable>(split), Some(&Pickable::IGNORE));
        assert!(see_through(&mut app, center_group));
        assert!(!see_through(&mut app, left_group));

        move_tab(&mut app, InspectorPanel::Details, center);
        assert!(!see_through(&mut app, center_group));
        assert_eq!(
            app.world()
                .get::<ThemeBackgroundColor>(center_group)
                .map(|background| &background.0),
            Some(&pane_background)
        );
        assert!(shows(&mut app, InspectorPanel::Details));

        move_to_edge(&mut app, InspectorPanel::Details, center, DockEdge::Bottom);
        let tree = dock_tree(&mut app);
        assert!(tree.leaf(center).is_some_and(|leaf| leaf.tabs().is_empty()));
        let column = tree.parent_of(center).unwrap();
        assert_ne!(column, tree.root());
        let column = group(&mut app, column);
        assert_eq!(app.world().get::<Pickable>(column), Some(&Pickable::IGNORE));
        assert_eq!(group(&mut app, center), center_group);
        assert!(see_through(&mut app, center_group));
        assert!(shows(&mut app, InspectorPanel::Details));

        for panel in [
            InspectorPanel::Entities,
            InspectorPanel::Resources,
            InspectorPanel::Details,
        ] {
            move_tab(&mut app, panel, center);
        }
        assert_eq!(dock_tree(&mut app).root(), center);
        assert_eq!(group(&mut app, center), center_group);
        let dock = single::<With<OverlayDock>>(&mut app);
        assert_eq!(
            app.world()
                .get::<ChildOf>(center_group)
                .map(ChildOf::parent),
            Some(dock)
        );
        assert!(!see_through(&mut app, center_group));
        assert!(shows(&mut app, InspectorPanel::Details));

        move_to_edge(&mut app, InspectorPanel::Details, center, DockEdge::Right);
        move_to_edge(&mut app, InspectorPanel::Entities, center, DockEdge::Left);
        let (left, _) = dock_tree(&mut app)
            .find_panel(InspectorPanel::Entities.key())
            .unwrap();
        move_tab(&mut app, InspectorPanel::Resources, left);
        let tree = dock_tree(&mut app);
        assert_ne!(tree.root(), center);
        assert!(tree.leaf(center).is_some_and(|leaf| leaf.tabs().is_empty()));
        assert_eq!(group(&mut app, center), center_group);
        assert!(see_through(&mut app, center_group));
        assert!(!shows(&mut app, InspectorPanel::Entities));
        assert!(shows(&mut app, InspectorPanel::Resources));
        assert!(shows(&mut app, InspectorPanel::Details));
    }

    #[test]
    fn inspector_keys_in_another_dock_are_left_alone() {
        let mut app = test_app();
        let mut tree = DockTree::new();
        let root = tree.root();
        tree.add_tab(root, InspectorPanel::Entities.key()).unwrap();
        app.world_mut().spawn((tree, Node::default()));
        press(&mut app);
        let world = app.world_mut();
        let contents: Vec<Entity> = world
            .query::<(Entity, &DockPanelContent)>()
            .iter(world)
            .filter(|(_, content)| world.get::<OverlayDock>(content.dock).is_none())
            .map(|(entity, _)| entity)
            .collect();
        assert_eq!(contents.len(), 1);
        assert!(world.get::<DockPanelTitle>(contents[0]).is_none());
        assert!(world.get::<Children>(contents[0]).is_none());
        single::<With<InspectorTreeView>>(&mut app);
    }

    #[test]
    fn selecting_a_resource_clears_the_entity_tree_highlight() {
        let mut app = test_app();
        let subject = app.world_mut().spawn(Name::new("Subject")).id();
        app.insert_resource(Marker);
        press(&mut app);
        app.world_mut().resource_mut::<InspectorSelection>().0 =
            Some(InspectableEntity::new(subject, InspectorSource::Local));
        app.update();
        let tree = single::<With<InspectorTreeView>>(&mut app);
        assert!(app
            .world()
            .get::<SelectedTreeItem>(tree)
            .unwrap()
            .0
            .is_some());

        activate(&mut app, InspectorPanel::Resources);
        let list = single::<With<InspectorResourceList>>(&mut app);
        let row = resource_row(&mut app).unwrap();
        select_row(&mut app, list, row);
        assert_eq!(app.world().get::<SelectedTreeItem>(tree).unwrap().0, None);
        assert_eq!(
            app.world().get::<SelectedTreeItem>(list).unwrap().0,
            Some(row)
        );
    }

    #[derive(Resource)]
    struct Marker;

    fn single<F: bevy_ecs::query::QueryFilter>(app: &mut App) -> Entity {
        let world = app.world_mut();
        world.query_filtered::<Entity, F>().single(world).unwrap()
    }

    fn resource_row(app: &mut App) -> Option<Entity> {
        let world = app.world_mut();
        let id = world.component_id::<Marker>()?;
        let inspected = world.resource_entities().get(id)?;
        world
            .query::<(Entity, &InspectorResourceRow)>()
            .iter(world)
            .find_map(|(row, shown)| (shown.inspected == inspected).then_some(row))
    }

    fn sync_resources(app: &mut App) {
        app.world_mut()
            .resource_mut::<ResourceListSync>()
            .set_dirty();
        app.update();
    }

    fn select_row(app: &mut App, list: Entity, row: Entity) {
        app.world_mut().trigger(ValueChange {
            source: list,
            value: Some(row),
            is_final: true,
        });
        app.update();
    }

    #[test]
    fn a_selected_header_stays_highlighted_until_a_resource_is_selected() {
        let mut app = test_app();
        app.insert_resource(Marker);
        press(&mut app);
        let tree = single::<With<InspectorTreeView>>(&mut app);
        let header = single::<With<InspectorHeaderRow>>(&mut app);
        select_row(&mut app, tree, header);
        assert_eq!(app.world().resource::<InspectorSelection>().0, None);
        assert_eq!(
            app.world().get::<SelectedTreeItem>(tree).unwrap().0,
            Some(header)
        );
        app.world_mut().resource_mut::<EntityTreeSync>().set_dirty();
        app.update();
        assert_eq!(
            app.world().get::<SelectedTreeItem>(tree).unwrap().0,
            Some(header)
        );

        activate(&mut app, InspectorPanel::Resources);
        let list = single::<With<InspectorResourceList>>(&mut app);
        let row = resource_row(&mut app).unwrap();
        select_row(&mut app, list, row);
        assert_eq!(app.world().get::<SelectedTreeItem>(tree).unwrap().0, None);
    }

    #[test]
    fn a_reinserted_resource_keeps_its_highlight() {
        let mut app = test_app();
        app.insert_resource(Marker);
        press(&mut app);
        activate(&mut app, InspectorPanel::Resources);
        let list = single::<With<InspectorResourceList>>(&mut app);
        let row = resource_row(&mut app).unwrap();
        select_row(&mut app, list, row);

        app.world_mut().remove_resource::<Marker>();
        sync_resources(&mut app);
        assert!(resource_row(&mut app).is_none());

        app.insert_resource(Marker);
        sync_resources(&mut app);
        let respawned = resource_row(&mut app).unwrap();
        assert_ne!(respawned, row);
        assert_eq!(
            app.world().get::<SelectedTreeItem>(list).unwrap().0,
            Some(respawned)
        );
    }

    #[test]
    fn a_hidden_resource_tab_does_not_sync_and_catches_up_when_shown() {
        let mut app = test_app();
        press(&mut app);
        activate(&mut app, InspectorPanel::Resources);
        activate(&mut app, InspectorPanel::Entities);
        app.insert_resource(Marker);
        sync_resources(&mut app);
        assert!(resource_row(&mut app).is_none());

        activate(&mut app, InspectorPanel::Resources);
        assert!(resource_row(&mut app).is_some());
    }

    #[test]
    fn keeps_the_apps_theme() {
        let mut app = test_app();
        let mut theme = create_dark_theme();
        theme.semantic_base.clear();
        app.insert_resource(UiTheme(theme));
        press(&mut app);
        assert!(app.world().resource::<UiTheme>().0.semantic_base.is_empty());
    }

    #[test]
    fn plugin_can_be_added_more_than_once() {
        let mut app = test_app();
        app.add_plugins(InspectorPlugin);
        app.update();
    }

    #[test]
    fn a_hidden_ancestor_hides_a_panel() {
        let mut world = World::new();
        let root = world.spawn(Visibility::Hidden).id();
        let panel = world.spawn((InspectorTreeView, ChildOf(root))).id();
        assert!(!world.run_system_once(entity_tree_visible).unwrap());

        world.entity_mut(root).insert(Visibility::Inherited);
        assert!(world.run_system_once(entity_tree_visible).unwrap());

        world.entity_mut(panel).insert(Visibility::Hidden);
        assert!(!world.run_system_once(entity_tree_visible).unwrap());
    }

    fn mode_button(app: &mut App, mode: InspectorMode) -> Entity {
        let world = app.world_mut();
        world
            .query::<(Entity, &ModeToggle)>()
            .iter(world)
            .find(|(_, toggle)| toggle.0 == mode)
            .unwrap()
            .0
    }

    #[test]
    fn the_toolbar_switches_and_shows_the_mode() {
        let mut app = test_app();
        press(&mut app);
        let edit = mode_button(&mut app, InspectorMode::Edit);
        let preview = mode_button(&mut app, InspectorMode::Preview);
        assert_eq!(
            app.world().get::<ButtonVariant>(edit),
            Some(&ButtonVariant::Primary)
        );
        assert_eq!(
            app.world().get::<ButtonVariant>(preview),
            Some(&ButtonVariant::Normal)
        );

        click(&mut app, preview);
        assert_eq!(
            *app.world().resource::<InspectorMode>(),
            InspectorMode::Preview
        );
        assert_eq!(
            app.world().get::<ButtonVariant>(preview),
            Some(&ButtonVariant::Primary)
        );

        *app.world_mut().resource_mut::<InspectorMode>() = InspectorMode::Edit;
        app.update();
        assert_eq!(
            app.world().get::<ButtonVariant>(edit),
            Some(&ButtonVariant::Primary)
        );
        assert_eq!(
            app.world().get::<ButtonVariant>(preview),
            Some(&ButtonVariant::Normal)
        );
    }

    #[test]
    fn picking_from_the_resources_tab_selects_and_shows_the_entity_tree() {
        use bevy_picking::{
            backend::{HitData, PointerHits},
            pointer::{Location, PointerAction, PointerButton, PointerId, PointerInput},
        };

        let mut app = test_app();
        press(&mut app);
        activate(&mut app, InspectorPanel::Resources);
        assert!(!shows(&mut app, InspectorPanel::Entities));
        let root = app.world_mut().spawn(Name::new("Root")).id();
        let subject = app
            .world_mut()
            .spawn((Name::new("Subject"), ChildOf(root)))
            .id();
        let camera = app.world_mut().spawn_empty().id();
        for action in [
            PointerAction::Press(PointerButton::Primary),
            PointerAction::Release(PointerButton::Primary),
        ] {
            app.world_mut().write_message(PointerHits::new(
                PointerId::Mouse,
                alloc::vec![(subject, HitData::new(camera, 1.0, None, None))],
                0.0,
            ));
            app.world_mut().write_message(PointerInput::new(
                PointerId::Mouse,
                Location {
                    target: bevy_camera::NormalizedRenderTarget::None {
                        width: 1,
                        height: 1,
                    },
                    position: bevy_math::Vec2::ZERO,
                },
                action,
            ));
            app.update();
        }
        assert_eq!(
            app.world().resource::<InspectorSelection>().0,
            Some(InspectableEntity::new(subject, InspectorSource::Local))
        );
        assert!(shows(&mut app, InspectorPanel::Entities));
        assert!(!shows(&mut app, InspectorPanel::Resources));
        app.update();
        let tree = single::<With<InspectorTreeView>>(&mut app);
        let row = app
            .world()
            .resource::<crate::entity_tree::TreeRowIndex>()
            .row(subject, InspectorSource::Local);
        assert!(row.is_some());
        assert_eq!(app.world().get::<SelectedTreeItem>(tree).unwrap().0, row);
    }
}
