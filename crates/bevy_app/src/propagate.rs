use alloc::vec::Vec;
use core::marker::PhantomData;

use crate::{App, Plugin, Update};
use bevy_ecs::{
    component::Component,
    entity::Entity,
    hierarchy::{ChildOf, Children},
    query::{Changed, Or, QueryFilter, With, Without},
    removal_detection::RemovedComponents,
    schedule::{IntoSystemConfigs, SystemSet},
    system::{Commands, Local, Query},
};

/// Plugin to automatically propagate a component value to all descendants of entities with
/// a `Propagate<C>` component.
///
/// The plugin Will maintain the target component over hierarchy changes, adding or removing
/// `C` when a child is added or removed from a tree with a `Propagate::<C>` parent, or if the
/// `Propagate::<C>` component is added, changed or removed.
///
/// Optionally you can include a query filter `F` to restrict the entities that are updated.
/// Note that the filter is not rechecked dynamically, changes to the filter state will
/// not be picked up until the `Propagate::<C>` component is touched, or the hierarchy
/// is changed.
/// All members of the tree must match the filter for propagation to occur.
/// Individual entities can be skipped or terminate the propagation with the `PropagateOver<C>`
/// and `PropagateStop<C>` components.
pub struct HierarchyPropagatePlugin<C: Component + Clone + PartialEq, F: QueryFilter = ()>(
    PhantomData<fn() -> (C, F)>,
);

/// Causes the inner component to be added to this entity and all children.
/// A descendant with a `Propagate::<C>` component of it's own will override propagation
/// from that point in the tree
#[derive(Component, Clone, PartialEq)]
pub struct Propagate<C: Component + Clone + PartialEq>(pub C);

/// Stops the output component being added to this entity.
/// Children will still inherit the component from this entity or its parents
#[derive(Component)]
pub struct PropagateOver<C: Component + Clone + PartialEq>(PhantomData<fn() -> C>);

/// Stops the propagation at this entity. Children will not inherit the component.
#[derive(Component)]
pub struct PropagateStop<C: Component + Clone + PartialEq>(PhantomData<fn() -> C>);

/// The set in which propagation systems are added. You can schedule your logic relative to this set.
#[derive(SystemSet, Clone, PartialEq, PartialOrd, Ord)]
pub struct PropagateSet<C: Component + Clone + PartialEq> {
    _p: PhantomData<fn() -> C>,
}

/// Internal struct for managing propagation
#[derive(Component, Clone, PartialEq)]
pub struct Inherited<C: Component + Clone + PartialEq>(pub C);

impl<C: Component + Clone + PartialEq, F: QueryFilter> Default for HierarchyPropagatePlugin<C, F> {
    fn default() -> Self {
        Self(Default::default())
    }
}

impl<C: Component + Clone + PartialEq> Default for PropagateOver<C> {
    fn default() -> Self {
        Self(Default::default())
    }
}

impl<C: Component + Clone + PartialEq> Default for PropagateStop<C> {
    fn default() -> Self {
        Self(Default::default())
    }
}

impl<C: Component + Clone + PartialEq> core::fmt::Debug for PropagateSet<C> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("PropagateSet")
            .field("_p", &self._p)
            .finish()
    }
}

impl<C: Component + Clone + PartialEq> Eq for PropagateSet<C> {}
impl<C: Component + Clone + PartialEq> core::hash::Hash for PropagateSet<C> {
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        self._p.hash(state);
    }
}

impl<C: Component + Clone + PartialEq> Default for PropagateSet<C> {
    fn default() -> Self {
        Self {
            _p: Default::default(),
        }
    }
}

impl<C: Component + Clone + PartialEq, F: QueryFilter + 'static> Plugin
    for HierarchyPropagatePlugin<C, F>
{
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                update_source::<C, F>,
                update_stopped::<C, F>,
                update_reparented::<C, F>,
                propagate_inherited::<C, F>,
                propagate_output::<C, F>,
            )
                .chain()
                .in_set(PropagateSet::<C>::default()),
        );
    }
}

/// add/remove `Inherited::<C>` and `C` for entities with a direct `Propagate::<C>`
pub fn update_source<C: Component + Clone + PartialEq, F: QueryFilter>(
    mut commands: Commands,
    changed: Query<
        (Entity, &Propagate<C>),
        (
            Or<(Changed<Propagate<C>>, Without<Inherited<C>>)>,
            Without<PropagateStop<C>>,
        ),
    >,
    mut removed: RemovedComponents<Propagate<C>>,
) {
    for (entity, source) in &changed {
        commands
            .entity(entity)
            .try_insert(Inherited(source.0.clone()));
    }

    for removed in removed.read() {
        if let Some(mut commands) = commands.get_entity(removed) {
            commands.remove::<(Inherited<C>, C)>();
        }
    }
}

/// remove `Inherited::<C>` and `C` for entities with a `PropagateStop::<C>`
pub fn update_stopped<C: Component + Clone + PartialEq, F: QueryFilter>(
    mut commands: Commands,
    q: Query<Entity, (With<Inherited<C>>, With<PropagateStop<C>>, F)>,
) {
    for entity in q.iter() {
        let mut cmds = commands.entity(entity);
        cmds.remove::<(Inherited<C>, C)>();
    }
}

/// add/remove `Inherited::<C>` and `C` for entities which have changed parent
pub fn update_reparented<C: Component + Clone + PartialEq, F: QueryFilter>(
    mut commands: Commands,
    moved: Query<
        (Entity, &ChildOf, Option<&Inherited<C>>),
        (
            Changed<ChildOf>,
            Without<Propagate<C>>,
            Without<PropagateStop<C>>,
            F,
        ),
    >,
    parents: Query<&Inherited<C>>,

    orphaned: Query<
        Entity,
        (
            With<Inherited<C>>,
            Without<Propagate<C>>,
            Without<ChildOf>,
            F,
        ),
    >,
) {
    for (entity, parent, maybe_inherited) in &moved {
        if let Ok(inherited) = parents.get(parent.get()) {
            commands.entity(entity).try_insert(inherited.clone());
        } else if maybe_inherited.is_some() {
            commands.entity(entity).remove::<(Inherited<C>, C)>();
        }
    }

    for orphan in &orphaned {
        commands.entity(orphan).remove::<(Inherited<C>, C)>();
    }
}

/// add/remove `Inherited::<C>` for children of entities with modified `Inherited::<C>`
pub fn propagate_inherited<C: Component + Clone + PartialEq, F: QueryFilter>(
    mut commands: Commands,
    changed: Query<
        (&Inherited<C>, &Children),
        (Changed<Inherited<C>>, Without<PropagateStop<C>>, F),
    >,
    recurse: Query<
        (Option<&Children>, Option<&Inherited<C>>),
        (Without<Propagate<C>>, Without<PropagateStop<C>>, F),
    >,
    mut removed: RemovedComponents<Inherited<C>>,
    mut to_process: Local<Vec<(Entity, Option<Inherited<C>>)>>,
) {
    // gather changed
    for (inherited, children) in &changed {
        to_process.extend(
            children
                .iter()
                .map(|child| (*child, Some(inherited.clone()))),
        );
    }

    // and removed
    for entity in removed.read() {
        if let Ok((Some(children), _)) = recurse.get(entity) {
            to_process.extend(children.iter().map(|child| (*child, None)));
        }
    }

    // propagate
    while let Some((entity, maybe_inherited)) = (*to_process).pop() {
        let Ok((maybe_children, maybe_current)) = recurse.get(entity) else {
            continue;
        };

        if maybe_current == maybe_inherited.as_ref() {
            continue;
        }

        if let Some(children) = maybe_children {
            to_process.extend(
                children
                    .iter()
                    .map(|child| (*child, maybe_inherited.clone())),
            );
        }

        if let Some(inherited) = maybe_inherited {
            commands.entity(entity).try_insert(inherited.clone());
        } else {
            commands.entity(entity).remove::<(Inherited<C>, C)>();
        }
    }
}

/// add `C` to entities with `Inherited::<C>`
pub fn propagate_output<C: Component + Clone + PartialEq, F: QueryFilter>(
    mut commands: Commands,
    changed: Query<
        (Entity, &Inherited<C>, Option<&C>),
        (Changed<Inherited<C>>, Without<PropagateOver<C>>, F),
    >,
) {
    for (entity, inherited, maybe_current) in &changed {
        if maybe_current.is_some_and(|c| &inherited.0 == c) {
            continue;
        }

        commands.entity(entity).try_insert(inherited.0.clone());
    }
}

#[cfg(test)]
mod tests {
    use bevy_ecs::schedule::Schedule;

    use crate::{App, Update};

    use super::*;

    #[derive(Component, Clone, PartialEq, Debug)]
    struct TestValue(u32);

    #[test]
    fn test_simple_propagate() {
        let mut app = App::new();
        app.add_schedule(Schedule::new(Update));
        app.add_plugins(HierarchyPropagatePlugin::<TestValue>::default());

        let propagator = app.world_mut().spawn(Propagate(TestValue(1))).id();
        let intermediate = app
            .world_mut()
            .spawn_empty()
            .insert(ChildOf(propagator))
            .id();
        let propagatee = app
            .world_mut()
            .spawn_empty()
            .insert(ChildOf(intermediate))
            .id();

        app.update();

        assert!(app
            .world_mut()
            .query::<&TestValue>()
            .get(app.world(), propagatee)
            .is_ok());
    }

    #[test]
    fn test_reparented() {
        let mut app = App::new();
        app.add_schedule(Schedule::new(Update));
        app.add_plugins(HierarchyPropagatePlugin::<TestValue>::default());

        let propagator = app.world_mut().spawn(Propagate(TestValue(1))).id();
        let propagatee = app
            .world_mut()
            .spawn_empty()
            .insert(ChildOf(propagator))
            .id();

        app.update();

        assert!(app
            .world_mut()
            .query::<&TestValue>()
            .get(app.world(), propagatee)
            .is_ok());
    }

    #[test]
    fn test_reparented_with_prior() {
        let mut app = App::new();
        app.add_schedule(Schedule::new(Update));
        app.add_plugins(HierarchyPropagatePlugin::<TestValue>::default());

        let propagator_a = app.world_mut().spawn(Propagate(TestValue(1))).id();
        let propagator_b = app.world_mut().spawn(Propagate(TestValue(2))).id();
        let propagatee = app
            .world_mut()
            .spawn_empty()
            .insert(ChildOf(propagator_a))
            .id();

        app.update();
        assert_eq!(
            app.world_mut()
                .query::<&TestValue>()
                .get(app.world(), propagatee),
            Ok(&TestValue(1))
        );
        app.world_mut()
            .commands()
            .entity(propagatee)
            .insert(ChildOf(propagator_b));
        app.update();
        assert_eq!(
            app.world_mut()
                .query::<&TestValue>()
                .get(app.world(), propagatee),
            Ok(&TestValue(2))
        );
    }

    #[test]
    fn test_remove_orphan() {
        let mut app = App::new();
        app.add_schedule(Schedule::new(Update));
        app.add_plugins(HierarchyPropagatePlugin::<TestValue>::default());

        let propagator = app.world_mut().spawn(Propagate(TestValue(1))).id();
        let propagatee = app
            .world_mut()
            .spawn_empty()
            .insert(ChildOf(propagator))
            .id();

        app.update();
        assert!(app
            .world_mut()
            .query::<&TestValue>()
            .get(app.world(), propagatee)
            .is_ok());
        app.world_mut()
            .commands()
            .entity(propagatee)
            .remove::<ChildOf>();
        app.update();
        assert!(app
            .world_mut()
            .query::<&TestValue>()
            .get(app.world(), propagatee)
            .is_err());
    }

    #[test]
    fn test_remove_propagated() {
        let mut app = App::new();
        app.add_schedule(Schedule::new(Update));
        app.add_plugins(HierarchyPropagatePlugin::<TestValue>::default());

        let propagator = app.world_mut().spawn(Propagate(TestValue(1))).id();
        let propagatee = app
            .world_mut()
            .spawn_empty()
            .insert(ChildOf(propagator))
            .id();

        app.update();
        assert!(app
            .world_mut()
            .query::<&TestValue>()
            .get(app.world(), propagatee)
            .is_ok());
        app.world_mut()
            .commands()
            .entity(propagator)
            .remove::<Propagate<TestValue>>();
        app.update();
        assert!(app
            .world_mut()
            .query::<&TestValue>()
            .get(app.world(), propagatee)
            .is_err());
    }

    #[test]
    fn test_propagate_over() {
        let mut app = App::new();
        app.add_schedule(Schedule::new(Update));
        app.add_plugins(HierarchyPropagatePlugin::<TestValue>::default());

        let propagator = app.world_mut().spawn(Propagate(TestValue(1))).id();
        let propagate_over = app
            .world_mut()
            .spawn(TestValue(2))
            .insert(ChildOf(propagator))
            .id();
        let propagatee = app
            .world_mut()
            .spawn_empty()
            .insert(ChildOf(propagate_over))
            .id();

        app.update();
        assert_eq!(
            app.world_mut()
                .query::<&TestValue>()
                .get(app.world(), propagatee),
            Ok(&TestValue(1))
        );
    }

    #[test]
    fn test_propagate_stop() {
        let mut app = App::new();
        app.add_schedule(Schedule::new(Update));
        app.add_plugins(HierarchyPropagatePlugin::<TestValue>::default());

        let propagator = app.world_mut().spawn(Propagate(TestValue(1))).id();
        let propagate_stop = app
            .world_mut()
            .spawn(PropagateStop::<TestValue>::default())
            .insert(ChildOf(propagator))
            .id();
        let no_propagatee = app
            .world_mut()
            .spawn_empty()
            .insert(ChildOf(propagate_stop))
            .id();

        app.update();
        assert!(app
            .world_mut()
            .query::<&TestValue>()
            .get(app.world(), no_propagatee)
            .is_err());
    }

    #[test]
    fn test_intermediate_override() {
        let mut app = App::new();
        app.add_schedule(Schedule::new(Update));
        app.add_plugins(HierarchyPropagatePlugin::<TestValue>::default());

        let propagator = app.world_mut().spawn(Propagate(TestValue(1))).id();
        let intermediate = app
            .world_mut()
            .spawn_empty()
            .insert(ChildOf(propagator))
            .id();
        let propagatee = app
            .world_mut()
            .spawn_empty()
            .insert(ChildOf(intermediate))
            .id();

        app.update();
        assert_eq!(
            app.world_mut()
                .query::<&TestValue>()
                .get(app.world(), propagatee),
            Ok(&TestValue(1))
        );

        app.world_mut()
            .entity_mut(intermediate)
            .insert(Propagate(TestValue(2)));
        app.update();
        assert_eq!(
            app.world_mut()
                .query::<&TestValue>()
                .get(app.world(), propagatee),
            Ok(&TestValue(2))
        );
    }

    #[test]
    fn test_filter() {
        #[derive(Component)]
        struct Marker;

        let mut app = App::new();
        app.add_schedule(Schedule::new(Update));
        app.add_plugins(HierarchyPropagatePlugin::<TestValue, With<Marker>>::default());

        let propagator = app.world_mut().spawn(Propagate(TestValue(1))).id();
        let propagatee = app
            .world_mut()
            .spawn_empty()
            .insert(ChildOf(propagator))
            .id();

        app.update();
        assert!(app
            .world_mut()
            .query::<&TestValue>()
            .get(app.world(), propagatee)
            .is_err());

        // NOTE: changes to the filter condition are not rechecked
        app.world_mut().entity_mut(propagator).insert(Marker);
        app.world_mut().entity_mut(propagatee).insert(Marker);
        app.update();
        assert!(app
            .world_mut()
            .query::<&TestValue>()
            .get(app.world(), propagatee)
            .is_err());

        app.world_mut()
            .entity_mut(propagator)
            .insert(Propagate(TestValue(1)));
        app.update();
        assert!(app
            .world_mut()
            .query::<&TestValue>()
            .get(app.world(), propagatee)
            .is_ok());
    }
}
