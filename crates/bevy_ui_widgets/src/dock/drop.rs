use bevy_app::{App, Plugin, PostUpdate};
use bevy_ecs::{
    component::Component,
    entity::Entity,
    hierarchy::ChildOf,
    observer::On,
    reflect::ReflectComponent,
    schedule::IntoScheduleConfigs,
    system::{Commands, Query, Res, SystemParam},
};
use bevy_math::Vec2;
use bevy_picking::{hover::HoverMap, pointer::PointerId, Pickable};
use bevy_reflect::Reflect;
use bevy_ui::{percent, Node, PositionType};

use crate::{
    DockChange, DockEdge, DockEdit, DockNodeId, DockSystems, DockTabBinding, DockTabId, DockTree,
    DockUi, NodeUi, TabDragLifecycle, TabDragMode, TabDragPhase, TabList,
};

const EDGE_BAND: f32 = 0.25;

pub(crate) struct DockDropPlugin;

impl Plugin for DockDropPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<HoverMap>()
            .add_observer(drop_on_tab_drag)
            .add_systems(
                PostUpdate,
                update_drop_targets.in_set(DockSystems::Structure),
            );
    }
}

/// Where a tab dropped on a leaf's content goes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Reflect)]
#[reflect(Clone, Debug, PartialEq)]
pub enum DockDropZone {
    /// Into a new leaf at this edge of the leaf.
    Edge(DockEdge),
    /// Into the leaf, after its tabs.
    Center,
}

impl DockDropZone {
    /// Returns the zone at a position in a leaf's content, from `(0, 0)` at the top left to
    /// `(1, 1)` at the bottom right.
    ///
    /// A band a quarter of the leaf's size along each edge drops at that edge, the nearest one
    /// in corners, and the rest is the center.
    pub fn at(position: Vec2) -> Self {
        let edges = [
            (position.x, DockEdge::Left),
            (1.0 - position.x, DockEdge::Right),
            (position.y, DockEdge::Top),
            (1.0 - position.y, DockEdge::Bottom),
        ];
        edges
            .into_iter()
            .filter(|(distance, _)| *distance < EDGE_BAND)
            .min_by(|a, b| a.0.total_cmp(&b.0))
            .map_or(DockDropZone::Center, |(_, edge)| DockDropZone::Edge(edge))
    }
}

/// An overlay on a leaf's content area that accepts a dragged tab.
///
/// One is spawned over each leaf of the dock when a drag of one of its tabs starts, and they
/// are despawned when the drag completes or is cancelled, even if the tab is gone by then. A tab
/// dropped on one, and not on a tab list, is moved by sending a [`DockChange`] for its
/// [`DockDropZone`], unless the drop would leave the tree as it is.
#[derive(Component, Clone, Copy, Debug, PartialEq, Reflect)]
#[reflect(Component, Clone, Debug, PartialEq)]
pub struct DockDropArea {
    /// The entity holding the [`DockTree`](crate::DockTree).
    pub dock: Entity,
    /// The leaf this area covers.
    pub leaf: DockNodeId,
    /// The pointer dragging the tab.
    pub pointer_id: PointerId,
}

/// The zone of a [`DockDropArea`] under its pointer, present only while it is hovered.
///
/// Styling can use it to preview where the tab will land.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq, Reflect)]
#[reflect(Component, Clone, Debug, PartialEq)]
pub struct DockDropTarget(pub DockDropZone);

#[derive(SystemParam)]
struct DropZones<'w, 's> {
    hover_map: Res<'w, HoverMap>,
    areas: Query<'w, 's, &'static DockDropArea>,
    tab_lists: Query<'w, 's, &'static TabList>,
    parents: Query<'w, 's, &'static ChildOf>,
}

impl DropZones<'_, '_> {
    /// The area and zone under a pointer, or `None` when it is over a tab list that would
    /// take the drop instead.
    fn hovered(&self, pointer_id: PointerId) -> Option<(Entity, DockDropZone)> {
        let hits = self.hover_map.get(&pointer_id)?;
        if hits.keys().any(|&entity| self.in_tab_list(entity)) {
            return None;
        }
        let (area, hit) = hits
            .iter()
            .filter(|(entity, _)| {
                self.areas
                    .get(**entity)
                    .is_ok_and(|area| area.pointer_id == pointer_id)
            })
            .min_by(|a, b| a.1.depth.total_cmp(&b.1.depth))?;
        let position = hit.position?.truncate() + Vec2::splat(0.5);
        Some((*area, DockDropZone::at(position)))
    }

    fn in_tab_list(&self, entity: Entity) -> bool {
        core::iter::once(entity)
            .chain(self.parents.iter_ancestors(entity))
            .any(|entity| {
                self.tab_lists
                    .get(entity)
                    .is_ok_and(|list| list.drag == TabDragMode::External)
            })
    }
}

fn drop_on_tab_drag(
    drag: On<TabDragLifecycle>,
    tabs: Query<&DockTabBinding>,
    docks: Query<(&DockUi, &DockTree)>,
    areas: Query<(Entity, &DockDropArea)>,
    zones: DropZones,
    mut commands: Commands,
) {
    if drag.phase == TabDragPhase::Started {
        let Ok(binding) = tabs.get(drag.tab) else {
            return;
        };
        let Ok((ui, _)) = docks.get(binding.dock) else {
            return;
        };
        for (&leaf, node) in &ui.nodes {
            if let NodeUi::Leaf { area, .. } = node {
                commands.spawn((
                    DockDropArea {
                        dock: binding.dock,
                        leaf,
                        pointer_id: drag.pointer_id,
                    },
                    Node {
                        position_type: PositionType::Absolute,
                        width: percent(100),
                        height: percent(100),
                        ..Default::default()
                    },
                    Pickable {
                        should_block_lower: false,
                        is_hoverable: true,
                    },
                    ChildOf(*area),
                ));
            }
        }
        return;
    }
    for (entity, area) in &areas {
        if area.pointer_id == drag.pointer_id {
            commands.entity(entity).try_despawn();
        }
    }
    if drag.phase != (TabDragPhase::Completed { drop: None }) {
        return;
    }
    let Ok(binding) = tabs.get(drag.tab) else {
        return;
    };
    let Some((area, zone)) = zones
        .hovered(drag.pointer_id)
        .and_then(|(entity, zone)| Some((areas.get(entity).ok()?.1, zone)))
        .filter(|(area, _)| area.dock == binding.dock)
    else {
        return;
    };
    let Ok((_, tree)) = docks.get(binding.dock) else {
        return;
    };
    if let Some(edit) = drop_edit(tree, binding.tab, area.leaf, zone) {
        commands.trigger(DockChange {
            dock: binding.dock,
            edit,
            is_final: true,
        });
    }
}

fn drop_edit(
    tree: &DockTree,
    tab: DockTabId,
    leaf: DockNodeId,
    zone: DockDropZone,
) -> Option<DockEdit> {
    let source = tree.find_leaf_for_tab(tab)?;
    match zone {
        DockDropZone::Center if source == leaf => None,
        DockDropZone::Center => Some(DockEdit::MoveTab {
            tab,
            to_leaf: leaf,
            index: None,
        }),
        DockDropZone::Edge(_)
            if source == leaf && tree.leaf(leaf).is_some_and(|l| l.tabs().len() == 1) =>
        {
            None
        }
        DockDropZone::Edge(edge) => Some(DockEdit::MoveToEdge {
            tab,
            target: leaf,
            edge,
        }),
    }
}

fn update_drop_targets(
    zones: DropZones,
    areas: Query<(Entity, &DockDropArea, Option<&DockDropTarget>)>,
    mut commands: Commands,
) {
    for (entity, area, current) in &areas {
        let target = zones
            .hovered(area.pointer_id)
            .filter(|(hovered, _)| *hovered == entity)
            .map(|(_, zone)| DockDropTarget(zone));
        if target.as_ref() == current {
            continue;
        }
        match target {
            Some(target) => commands.entity(entity).insert(target),
            None => commands.entity(entity).remove::<DockDropTarget>(),
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::TabDrop;
    use crate::{dock_self_update, DockNodeBinding, DockPlugin};
    use alloc::vec::Vec;
    use bevy_ecs::{hierarchy::Children, resource::Resource, system::ResMut, world::World};
    use bevy_picking::backend::HitData;

    const POINTER: PointerId = PointerId::Mouse;

    struct Fixture {
        app: App,
        dock: Entity,
        tree: DockTree,
        main: DockNodeId,
        side: DockNodeId,
        a: DockTabId,
        c: DockTabId,
    }

    fn fixture() -> Fixture {
        let mut tree = DockTree::new();
        let main = tree.root();
        let a = tree.add_tab(main, "a").unwrap();
        tree.add_tab(main, "b").unwrap();
        let (side, c) = tree.split(main, DockEdge::Right, "c").unwrap();
        let mut app = App::new();
        app.add_plugins(DockPlugin);
        let dock = app
            .world_mut()
            .spawn((tree.clone(), Node::default()))
            .observe(dock_self_update)
            .id();
        app.update();
        Fixture {
            app,
            dock,
            tree,
            main,
            side,
            a,
            c,
        }
    }

    fn header(world: &mut World, tab: DockTabId) -> Entity {
        let mut query = world.query::<(Entity, &DockTabBinding)>();
        query.iter(world).find(|(_, b)| b.tab == tab).unwrap().0
    }

    fn leaf_entity(world: &mut World, leaf: DockNodeId) -> Entity {
        let mut query = world.query::<(Entity, &DockNodeBinding)>();
        query.iter(world).find(|(_, b)| b.node == leaf).unwrap().0
    }

    fn areas(world: &mut World) -> Vec<(Entity, DockDropArea)> {
        let mut query = world.query::<(Entity, &DockDropArea)>();
        query.iter(world).map(|(e, a)| (e, *a)).collect()
    }

    fn area_for(world: &mut World, leaf: DockNodeId) -> Entity {
        areas(world)
            .into_iter()
            .find(|(_, area)| area.leaf == leaf)
            .unwrap()
            .0
    }

    fn lifecycle(f: &mut Fixture, phase: TabDragPhase) {
        let tab = f.a;
        lifecycle_tab(f, tab, phase);
    }

    fn lifecycle_tab(f: &mut Fixture, tab: DockTabId, phase: TabDragPhase) {
        let world = f.app.world_mut();
        let tab = header(world, tab);
        let source = world.get::<ChildOf>(tab).unwrap().parent();
        world.trigger(TabDragLifecycle {
            source,
            tab,
            pointer_id: POINTER,
            phase,
        });
        f.app.update();
    }

    fn hover(f: &mut Fixture, hits: &[(Entity, Vec2)]) {
        let mut map = f.app.world_mut().resource_mut::<HoverMap>();
        let entry = map.0.entry(POINTER).or_default();
        entry.clear();
        for (depth, &(entity, position)) in hits.iter().enumerate() {
            let position = (position - Vec2::splat(0.5)).extend(0.0);
            entry.insert(
                entity,
                HitData::new(Entity::PLACEHOLDER, depth as f32, Some(position), None),
            );
        }
    }

    fn hover_side(f: &mut Fixture, position: Vec2) {
        let leaf = f.side;
        hover_leaf(f, leaf, position);
    }

    fn hover_leaf(f: &mut Fixture, leaf: DockNodeId, position: Vec2) {
        let area = area_for(f.app.world_mut(), leaf);
        hover(f, &[(area, position)]);
    }

    #[derive(Resource, Default)]
    struct Changes(usize);

    fn count_changes(f: &mut Fixture) {
        f.app
            .init_resource::<Changes>()
            .add_observer(|_: On<DockChange>, mut changes: ResMut<Changes>| changes.0 += 1);
    }

    fn changes(f: &Fixture) -> usize {
        f.app.world().resource::<Changes>().0
    }

    fn targets(world: &mut World) -> usize {
        world.query::<&DockDropTarget>().iter(world).count()
    }

    fn tree(f: &Fixture) -> &DockTree {
        f.app.world().get::<DockTree>(f.dock).unwrap()
    }

    #[test]
    fn zones_by_position() {
        let cases = [
            (Vec2::new(0.1, 0.5), DockDropZone::Edge(DockEdge::Left)),
            (Vec2::new(0.9, 0.5), DockDropZone::Edge(DockEdge::Right)),
            (Vec2::new(0.5, 0.1), DockDropZone::Edge(DockEdge::Top)),
            (Vec2::new(0.5, 0.9), DockDropZone::Edge(DockEdge::Bottom)),
            (Vec2::new(0.05, 0.2), DockDropZone::Edge(DockEdge::Left)),
            (Vec2::new(0.5, 0.5), DockDropZone::Center),
            (Vec2::new(0.26, 0.74), DockDropZone::Center),
        ];
        for (position, zone) in cases {
            assert_eq!(DockDropZone::at(position), zone, "{position}");
        }
    }

    #[test]
    fn areas_follow_the_drag() {
        let mut f = fixture();
        assert!(areas(f.app.world_mut()).is_empty());
        lifecycle(&mut f, TabDragPhase::Started);
        let world = f.app.world_mut();
        let spawned = areas(world);
        assert_eq!(spawned.len(), 2);
        for (entity, area) in spawned {
            assert_eq!(area.dock, f.dock);
            assert_eq!(area.pointer_id, POINTER);
            let leaf = leaf_entity(world, area.leaf);
            let content = world.get::<Children>(leaf).unwrap()[1];
            assert_eq!(world.get::<ChildOf>(entity).unwrap().parent(), content);
        }

        hover_side(&mut f, Vec2::new(0.9, 0.5));
        f.app.update();
        let world = f.app.world_mut();
        let side = area_for(world, f.side);
        assert_eq!(
            world.get::<DockDropTarget>(side),
            Some(&DockDropTarget(DockDropZone::Edge(DockEdge::Right)))
        );
        let main = area_for(world, f.main);
        assert!(world.get::<DockDropTarget>(main).is_none());

        let side_leaf = f.side;
        f.app
            .world_mut()
            .get_mut::<DockTree>(f.dock)
            .unwrap()
            .add_tab(side_leaf, "d")
            .unwrap();
        f.app.update();
        assert_eq!(areas(f.app.world_mut()).len(), 2);
        let world = f.app.world_mut();
        let leaf = leaf_entity(world, side_leaf);
        let content = world.get::<Children>(leaf).unwrap()[1];
        assert!(world.get::<Children>(content).unwrap().contains(&side));

        lifecycle(
            &mut f,
            TabDragPhase::Completed {
                drop: Some(TabDrop {
                    destination: Entity::PLACEHOLDER,
                    index: 0,
                }),
            },
        );
        assert!(areas(f.app.world_mut()).is_empty());
        assert_eq!(tree(&f).leaf(side_leaf).unwrap().tabs().len(), 2);
    }

    #[test]
    fn edge_drop_splits() {
        for edge in [
            DockEdge::Top,
            DockEdge::Bottom,
            DockEdge::Left,
            DockEdge::Right,
        ] {
            let mut f = fixture();
            lifecycle(&mut f, TabDragPhase::Started);
            let position = match edge {
                DockEdge::Top => Vec2::new(0.5, 0.1),
                DockEdge::Bottom => Vec2::new(0.5, 0.9),
                DockEdge::Left => Vec2::new(0.1, 0.5),
                DockEdge::Right => Vec2::new(0.9, 0.5),
            };
            hover_side(&mut f, position);
            lifecycle(&mut f, TabDragPhase::Completed { drop: None });

            let mut expected = f.tree.clone();
            let leaf = expected.move_tab_to_edge(f.a, f.side, edge).unwrap();
            assert_eq!(tree(&f), &expected, "{edge:?}");
            let world = f.app.world_mut();
            let tab = header(world, f.a);
            let list = world.get::<ChildOf>(tab).unwrap().parent();
            let leaf_entity = leaf_entity(world, leaf);
            assert_eq!(world.get::<ChildOf>(list).unwrap().parent(), leaf_entity);
            assert!(areas(world).is_empty());
        }
    }

    #[test]
    fn center_drop_moves_tab() {
        let mut f = fixture();
        lifecycle(&mut f, TabDragPhase::Started);
        hover_side(&mut f, Vec2::splat(0.5));
        lifecycle(&mut f, TabDragPhase::Completed { drop: None });
        let mut expected = f.tree.clone();
        expected.move_tab(f.a, f.side, None).unwrap();
        assert_eq!(tree(&f), &expected);
    }

    #[test]
    fn drop_outside_does_nothing() {
        let mut f = fixture();
        lifecycle(&mut f, TabDragPhase::Started);
        hover(&mut f, &[]);
        lifecycle(&mut f, TabDragPhase::Completed { drop: None });
        assert_eq!(tree(&f), &f.tree);
        assert!(areas(f.app.world_mut()).is_empty());
    }

    #[test]
    fn cancel_does_nothing() {
        let mut f = fixture();
        lifecycle(&mut f, TabDragPhase::Started);
        hover_side(&mut f, Vec2::new(0.9, 0.5));
        lifecycle(&mut f, TabDragPhase::Cancelled);
        assert_eq!(tree(&f), &f.tree);
        assert!(areas(f.app.world_mut()).is_empty());
    }

    #[test]
    fn removed_tab_cleans_up_areas() {
        let mut f = fixture();
        lifecycle(&mut f, TabDragPhase::Started);
        hover_side(&mut f, Vec2::new(0.9, 0.5));
        f.app.update();
        assert_eq!(targets(f.app.world_mut()), 1);

        let world = f.app.world_mut();
        let tab = header(world, f.a);
        let source = world.get::<ChildOf>(tab).unwrap().parent();
        let a = f.a;
        world
            .get_mut::<DockTree>(f.dock)
            .unwrap()
            .remove_tab(a)
            .unwrap();
        f.app.update();
        assert!(f.app.world().get_entity(tab).is_err());

        f.app.world_mut().trigger(TabDragLifecycle {
            source,
            tab,
            pointer_id: POINTER,
            phase: TabDragPhase::Cancelled,
        });
        f.app.update();
        let world = f.app.world_mut();
        assert!(areas(world).is_empty());
        assert_eq!(targets(world), 0);
    }

    #[test]
    fn center_drop_on_own_group_does_nothing() {
        let mut f = fixture();
        count_changes(&mut f);
        lifecycle(&mut f, TabDragPhase::Started);
        let main = f.main;
        hover_leaf(&mut f, main, Vec2::splat(0.5));
        lifecycle(&mut f, TabDragPhase::Completed { drop: None });
        assert_eq!(changes(&f), 0);
        assert_eq!(tree(&f), &f.tree);
    }

    #[test]
    fn edge_drop_of_only_tab_on_own_group_does_nothing() {
        let mut f = fixture();
        count_changes(&mut f);
        let c = f.c;
        lifecycle_tab(&mut f, c, TabDragPhase::Started);
        hover_side(&mut f, Vec2::new(0.1, 0.5));
        lifecycle_tab(&mut f, c, TabDragPhase::Completed { drop: None });
        assert_eq!(changes(&f), 0);
        assert_eq!(tree(&f), &f.tree);
        assert!(areas(f.app.world_mut()).is_empty());
    }

    #[test]
    fn edge_drop_on_own_group_splits_it() {
        let mut f = fixture();
        lifecycle(&mut f, TabDragPhase::Started);
        let main = f.main;
        hover_leaf(&mut f, main, Vec2::new(0.5, 0.9));
        lifecycle(&mut f, TabDragPhase::Completed { drop: None });
        let mut expected = f.tree.clone();
        expected
            .move_tab_to_edge(f.a, f.main, DockEdge::Bottom)
            .unwrap();
        assert_eq!(tree(&f), &expected);
    }

    #[test]
    fn tab_list_wins_over_area() {
        let mut f = fixture();
        lifecycle(&mut f, TabDragPhase::Started);
        let world = f.app.world_mut();
        let area = area_for(world, f.side);
        let leaf = leaf_entity(world, f.side);
        let list = world.get::<Children>(leaf).unwrap()[0];
        let tab = world.get::<Children>(list).unwrap()[0];
        hover(
            &mut f,
            &[(tab, Vec2::splat(0.5)), (area, Vec2::new(0.9, 0.5))],
        );
        f.app.update();
        assert!(f.app.world().get::<DockDropTarget>(area).is_none());
        lifecycle(&mut f, TabDragPhase::Completed { drop: None });
        assert_eq!(tree(&f), &f.tree);
    }

    #[test]
    fn ignores_other_tabs() {
        let mut f = fixture();
        let world = f.app.world_mut();
        let list = world.spawn(TabList::default()).id();
        let tab = world.spawn(ChildOf(list)).id();
        world.trigger(TabDragLifecycle {
            source: list,
            tab,
            pointer_id: POINTER,
            phase: TabDragPhase::Started,
        });
        f.app.update();
        assert!(areas(f.app.world_mut()).is_empty());
    }
}
