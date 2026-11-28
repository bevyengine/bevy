use bevy_app::{Plugin, PostUpdate};
use bevy_ecs::{
    change_detection::DetectChangesMut,
    component::Component,
    entity::Entity,
    hierarchy::{ChildOf, Children},
    query::{Added, With, Without},
    reflect::ReflectComponent,
    schedule::IntoScheduleConfigs,
    system::{Commands, Query},
    world::EntityWorldMut,
};
use bevy_picking::Pickable;
use bevy_reflect::{prelude::ReflectDefault, Reflect};
use bevy_scene::{bsn, EntityWorldMutSceneExt};
use bevy_ui::{percent, Node, PositionType, UiSystems};
use bevy_ui_widgets::{
    DockDropArea, DockDropTarget, DockDropZone, DockEdge, DockNodeBinding, DockSystems,
    DockTabBinding, DockTabTitle, SelectedTab, SplitPane, SplitPaneHandle, TabList,
};

use crate::{
    controls::{FeathersSplitPaneHandle, FeathersTab, FeathersTabList, TabCaption},
    theme::{ThemeBackgroundColor, ThemedText},
    tokens,
};

/// Gives the UI of a [`DockTree`](bevy_ui_widgets::DockTree) on the same entity the look of
/// feathers widgets.
///
/// Tab groups look like [`FeathersTabList`]s of [`FeathersTab`]s, splitters like
/// [`FeathersSplitPaneHandle`]s, and the drop zone under a dragged tab is highlighted. The dock
/// still builds and updates its UI, and feathers styles each part as it is spawned, so add this
/// marker when the dock is spawned.
///
/// The tab drag proxy is only shown when an ancestor of the dock has
/// [`DragOverlayRoot`](bevy_ui_widgets::DragOverlayRoot).
#[derive(Component, Debug, Default, Clone, Copy, Reflect)]
#[reflect(Component, Debug, Default, Clone)]
pub struct FeathersDock;

/// The highlight shown over a [`DockDropArea`] where a dragged tab would be dropped.
#[derive(Component, Debug, Default, Clone, Copy, Reflect)]
#[reflect(Component, Debug, Default, Clone)]
pub struct FeathersDockDropPreview;

fn style_dock_groups(
    groups: Query<(Entity, &DockNodeBinding), (Added<DockNodeBinding>, Without<SplitPane>)>,
    docks: Query<(), With<FeathersDock>>,
    mut commands: Commands,
) {
    for (group, binding) in &groups {
        if docks.contains(binding.dock) {
            commands
                .entity(group)
                .insert(ThemeBackgroundColor(tokens::PANE_BODY_BG));
        }
    }
}

fn style_dock_tab_lists(
    lists: Query<(Entity, &ChildOf), (Added<TabList>, Without<FeathersTabList>)>,
    groups: Query<&DockNodeBinding>,
    docks: Query<(), With<FeathersDock>>,
    mut commands: Commands,
) {
    for (list, parent) in &lists {
        let in_dock = groups
            .get(parent.parent())
            .is_ok_and(|binding| docks.contains(binding.dock));
        if in_dock {
            commands.entity(list).queue(apply_tab_list_scene);
        }
    }
}

/// Applies the [`FeathersTabList`] scene to a dock tab list, keeping the list's settings,
/// selection and tabs.
fn apply_tab_list_scene(mut list: EntityWorldMut) -> bevy_ecs::error::Result {
    let Some(tab_list) = list.get::<TabList>().cloned() else {
        return Ok(());
    };
    let selected = list.get::<SelectedTab>().map(|selected| selected.0);
    let tabs: Vec<Entity> = list
        .get::<Children>()
        .map(|children| children.iter().copied().collect())
        .unwrap_or_default();
    let (orientation, activation, drag) =
        (tab_list.orientation, tab_list.activation, tab_list.drag);
    list.apply_scene(bsn! {
        @FeathersTabList {
            @orientation: orientation,
            @activation: activation,
            @drag: drag,
        }
    })?;
    list.insert(tab_list);
    if let Some(selected) = selected {
        list.insert(SelectedTab(selected));
    }
    list.add_children(&tabs);
    if let Some(mut node) = list.get_mut::<Node>() {
        node.flex_shrink = 0.0;
    }
    Ok(())
}

fn style_dock_tabs(
    tabs: Query<(Entity, &DockTabBinding), (Added<DockTabBinding>, Without<FeathersTab>)>,
    docks: Query<(), With<FeathersDock>>,
    mut commands: Commands,
) {
    for (tab, binding) in &tabs {
        if docks.contains(binding.dock) {
            commands.entity(tab).queue(apply_tab_scene);
        }
    }
}

/// Applies the [`FeathersTab`] scene to a dock tab and moves its title into the caption.
fn apply_tab_scene(mut tab: EntityWorldMut) -> bevy_ecs::error::Result {
    let titles: Vec<Entity> = tab
        .get::<Children>()
        .map(|children| {
            children
                .iter()
                .copied()
                .filter(|child| tab.world().get::<DockTabTitle>(*child).is_some())
                .collect()
        })
        .unwrap_or_default();
    tab.apply_scene(bsn! { @FeathersTab })?;
    let caption = tab.get::<Children>().and_then(|children| {
        children
            .iter()
            .copied()
            .find(|child| tab.world().get::<TabCaption>(*child).is_some())
    });
    let Some(caption) = caption else {
        return Ok(());
    };
    tab.world_scope(|world| {
        for title in titles {
            world
                .entity_mut(title)
                .insert((ChildOf(caption), ThemedText));
        }
    });
    Ok(())
}

fn style_dock_handles(
    handles: Query<(Entity, &ChildOf), (Added<SplitPaneHandle>, Without<FeathersSplitPaneHandle>)>,
    splits: Query<&DockNodeBinding, With<SplitPane>>,
    docks: Query<(), With<FeathersDock>>,
    mut commands: Commands,
) {
    for (handle, parent) in &handles {
        let in_dock = splits
            .get(parent.parent())
            .is_ok_and(|binding| docks.contains(binding.dock));
        if in_dock {
            commands.entity(handle).queue(|mut handle: EntityWorldMut| {
                handle.apply_scene(bsn! { @FeathersSplitPaneHandle })
            });
        }
    }
}

fn update_drop_previews(
    areas: Query<(
        Entity,
        &DockDropArea,
        Option<&DockDropTarget>,
        Option<&Children>,
    )>,
    docks: Query<(), With<FeathersDock>>,
    mut previews: Query<&mut Node, With<FeathersDockDropPreview>>,
    mut commands: Commands,
) {
    for (area, drop_area, target, children) in &areas {
        if !docks.contains(drop_area.dock) {
            continue;
        }
        let preview = children.and_then(|children| {
            children
                .iter()
                .copied()
                .find(|child| previews.contains(*child))
        });
        match (target, preview) {
            (Some(target), Some(preview)) => {
                if let Ok(mut node) = previews.get_mut(preview) {
                    node.set_if_neq(drop_preview_node(target.0));
                }
            }
            (Some(target), None) => {
                commands.spawn((
                    FeathersDockDropPreview,
                    drop_preview_node(target.0),
                    ThemeBackgroundColor(tokens::DOCK_DROP_PREVIEW),
                    Pickable::IGNORE,
                    ChildOf(area),
                ));
            }
            (None, Some(preview)) => commands.entity(preview).despawn(),
            (None, None) => {}
        }
    }
}

/// Returns the node covering the part of a drop area that `zone` stands for.
fn drop_preview_node(zone: DockDropZone) -> Node {
    let band = DockDropZone::EDGE_BAND * 100.0;
    let (left, top, width, height) = match zone {
        DockDropZone::Edge(DockEdge::Left) => (0.0, 0.0, band, 100.0),
        DockDropZone::Edge(DockEdge::Right) => (100.0 - band, 0.0, band, 100.0),
        DockDropZone::Edge(DockEdge::Top) => (0.0, 0.0, 100.0, band),
        DockDropZone::Edge(DockEdge::Bottom) => (0.0, 100.0 - band, 100.0, band),
        DockDropZone::Center => (0.0, 0.0, 100.0, 100.0),
    };
    Node {
        position_type: PositionType::Absolute,
        left: percent(left),
        top: percent(top),
        width: percent(width),
        height: percent(height),
        ..Default::default()
    }
}

/// Plugin which registers the systems for styling docks marked with [`FeathersDock`].
pub struct FeathersDockPlugin;

impl Plugin for FeathersDockPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        app.add_systems(
            PostUpdate,
            (
                style_dock_groups,
                style_dock_tab_lists,
                style_dock_tabs,
                style_dock_handles,
                update_drop_previews,
            )
                .after(DockSystems::Structure)
                .before(DockSystems::Panels)
                .before(UiSystems::Prepare),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy_app::{App, TaskPoolPlugin};
    use bevy_asset::{AssetApp, AssetPlugin};
    use bevy_picking::pointer::PointerId;
    use bevy_scene::ScenePlugin;
    use bevy_ui::{FlexDirection, Selected};
    use bevy_ui_widgets::{ControlOrientation, DockPlugin, DockTree, TabDragMode, TabPlugin};

    use crate::controls::{FeathersTabsPlugin, SplitPanePlugin};

    fn app() -> App {
        let mut app = App::new();
        app.add_plugins((
            TaskPoolPlugin::default(),
            AssetPlugin::default(),
            ScenePlugin,
            TabPlugin,
            DockPlugin,
            FeathersTabsPlugin,
            SplitPanePlugin,
            FeathersDockPlugin,
        ));
        app.init_asset::<bevy_text::Font>();
        app.set_error_handler(bevy_ecs::error::panic);
        app
    }

    fn tree() -> DockTree {
        let mut tree = DockTree::new();
        let root = tree.root();
        tree.add_tab(root, "viewport").unwrap();
        tree.add_tab(root, "notes").unwrap();
        tree.split(root, DockEdge::Bottom, "console").unwrap();
        tree
    }

    fn spawn_dock(app: &mut App, feathers: bool) -> Entity {
        let mut dock = app.world_mut().spawn((tree(), Node::default()));
        if feathers {
            dock.insert(FeathersDock);
        }
        let dock = dock.id();
        app.update();
        dock
    }

    fn tabs(app: &mut App) -> Vec<Entity> {
        app.world_mut()
            .query_filtered::<Entity, With<DockTabBinding>>()
            .iter(app.world())
            .collect()
    }

    #[test]
    fn dock_tabs_get_the_feathers_tab_scene_with_their_title_in_the_caption() {
        let mut app = app();
        spawn_dock(&mut app, true);
        for tab in tabs(&mut app) {
            let world = app.world();
            assert!(world.get::<FeathersTab>(tab).is_some());
            let caption = world
                .get::<Children>(tab)
                .unwrap()
                .iter()
                .copied()
                .find(|child| world.get::<TabCaption>(*child).is_some())
                .expect("tab has a caption");
            let title = world
                .get::<Children>(caption)
                .unwrap()
                .iter()
                .copied()
                .find(|child| world.get::<DockTabTitle>(*child).is_some())
                .expect("title is in the caption");
            assert!(world.get::<ThemedText>(title).is_some());
        }
    }

    #[test]
    fn dock_tab_lists_keep_their_settings_selection_and_tabs() {
        let mut app = app();
        spawn_dock(&mut app, true);
        let mut lists = app
            .world_mut()
            .query_filtered::<(&TabList, &SelectedTab, &Children), With<FeathersTabList>>();
        let lists: Vec<_> = lists
            .iter(app.world())
            .map(|(list, selected, children)| (list.drag, selected.0, children.to_vec()))
            .collect();
        assert_eq!(lists.len(), 2);
        let mut tab_count = 0;
        for (drag, selected, children) in lists {
            assert_eq!(drag, TabDragMode::External);
            assert!(selected.is_some_and(|tab| children.contains(&tab)));
            assert!(app.world().get::<Selected>(selected.unwrap()).is_some());
            tab_count += children.len();
        }
        assert_eq!(tab_count, 3);
    }

    #[test]
    fn dock_handles_get_the_feathers_handle_scene() {
        let mut app = app();
        spawn_dock(&mut app, true);
        app.update();
        let mut handles = app
            .world_mut()
            .query_filtered::<(&Node, &ChildOf), With<FeathersSplitPaneHandle>>();
        let handles: Vec<_> = handles
            .iter(app.world())
            .map(|(node, parent)| (node.flex_direction, parent.parent()))
            .collect();
        assert_eq!(handles.len(), 1);
        let (direction, split) = handles[0];
        assert_eq!(
            app.world().get::<SplitPane>(split).unwrap().orientation,
            ControlOrientation::Vertical
        );
        assert_eq!(direction, FlexDirection::Column);
    }

    #[test]
    fn a_group_split_off_later_is_styled_and_keeps_its_selection() {
        let mut app = app();
        let dock = spawn_dock(&mut app, true);
        let mut tree = app.world_mut().get_mut::<DockTree>(dock).unwrap();
        let root = tree.root();
        tree.split(root, DockEdge::Right, "preview").unwrap();
        app.update();
        let mut lists = app
            .world_mut()
            .query_filtered::<(&SelectedTab, &Children), With<FeathersTabList>>();
        let lists: Vec<_> = lists
            .iter(app.world())
            .map(|(selected, children)| (selected.0, children.to_vec()))
            .collect();
        assert_eq!(lists.len(), 3);
        for (selected, children) in lists {
            assert!(selected.is_some_and(|tab| children.contains(&tab)));
            for tab in children {
                assert!(app.world().get::<FeathersTab>(tab).is_some());
            }
        }
    }

    #[test]
    fn the_hovered_drop_zone_is_previewed_until_the_target_goes_away() {
        let mut app = App::new();
        app.add_plugins(FeathersDockPlugin);
        let dock = app.world_mut().spawn(FeathersDock).id();
        let leaf = DockTree::new().root();
        let area = app
            .world_mut()
            .spawn((
                DockDropArea {
                    dock,
                    leaf,
                    tab_list: Entity::PLACEHOLDER,
                    pointer_id: PointerId::Mouse,
                },
                DockDropTarget(DockDropZone::Edge(DockEdge::Right)),
                Node::default(),
            ))
            .id();
        app.update();
        let preview = app.world().get::<Children>(area).unwrap()[0];
        let node = app.world().get::<Node>(preview).unwrap();
        assert!(app
            .world()
            .get::<FeathersDockDropPreview>(preview)
            .is_some());
        assert_eq!(node.left, percent(75.0));
        assert_eq!(node.width, percent(25.0));

        app.world_mut()
            .entity_mut(area)
            .insert(DockDropTarget(DockDropZone::Center));
        app.update();
        assert_eq!(
            app.world().get::<Node>(preview).unwrap().width,
            percent(100.0)
        );

        app.world_mut().entity_mut(area).remove::<DockDropTarget>();
        app.update();
        assert!(app.world().get_entity(preview).is_err());
    }

    #[test]
    fn docks_without_the_marker_are_left_alone() {
        let mut app = app();
        spawn_dock(&mut app, false);
        let world = app.world_mut();
        assert_eq!(
            world
                .query_filtered::<(), With<FeathersTab>>()
                .iter(world)
                .count(),
            0
        );
        assert_eq!(
            world
                .query_filtered::<(), With<FeathersTabList>>()
                .iter(world)
                .count(),
            0
        );
        assert_eq!(
            world
                .query_filtered::<(), With<FeathersSplitPaneHandle>>()
                .iter(world)
                .count(),
            0
        );
    }
}
