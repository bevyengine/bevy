use crate::Children;
use bevy_ecs::prelude::{Component, Entity, World};
use bevy_log::warn;
use std::any::TypeId;

/// A component that references an ancestor marked
/// with the [`MarkDescendants`] component.
///
/// The `Entity` field is the ancestor to which [`MarkDescendants`] was added.
#[derive(Component, Clone, Copy, PartialEq, Eq)]
pub struct Ancestor(pub Entity);

/// Mark descendants of this entity with an [`Ancestor`] component.
///
/// "Descendants" here means: the whole tree of entities
/// direct or indirect children of this [`Entity`].
///
/// Use this simply by adding it as a component to the entity you want
/// the descendants of to be marked with a [`Ancestor`] component.
///
/// This is useful if you want a "reference" to a parent entity
/// when reading other components.
///
/// The [`Ancestor`] components are added once
/// and the [`MarkDescendants`] component is erased from this `Entity`.
///
/// Use [`MarkDescendants::limit_to`] to only marks descendants with a particular
/// component.
///
/// Use [`MarkDescendants::limit_to_id`] to only marks descendants with a particular
/// [`TypeId`].
///
/// Use [`MarkDescendants::all`] to mark all descendants.
///
/// Warning:
/// - This only marks the specified descendents once,
///   and doesn't update dynamically based on updates to the hierarchy.
/// - The [`Ancestor`] component will be overwritten if you `MarkDescendants`
///   another ancestor of an entity.
/// - If this `Entity` is despawned, the content of [`Ancestor`] points to
///   an invalid `Entity`.
///
/// # Example
///
/// ```rust,no_run
/// # use bevy_ecs::prelude::*;
/// # use bevy_app::prelude::*;
/// # use bevy_hierarchy::prelude::*;
/// # #[derive(Component)]
/// # struct Name;
/// # impl Name { fn new(s: &str) -> Self { Name } }
/// # impl std::fmt::Display for Name { fn fmt(&self, _: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { Ok(()) } }
/// use bevy_hierarchy::{MarkDescendants, mark_descendants, Ancestor};
///
/// #[derive(Component)]
/// struct ShouldMark;
///
/// fn main() {
///     let mut app = App::new();
///     app.add_startup_system_to_stage(StartupStage::PreStartup, setup)
///         .add_startup_system_to_stage(StartupStage::Startup, mark_descendants.exclusive_system())
///         .add_startup_system_to_stage(StartupStage::PostStartup, print_marked);
///     app.run();
/// }
/// fn setup(mut commands: Commands) {
///     commands
///         .spawn_bundle((MarkDescendants::limit_to::<ShouldMark>(), Name::new("The Ancestor")))
///         .with_children(|parent| {
///             parent.spawn_bundle((Name::new("Child One"),));
///             parent.spawn_bundle((Name::new("Child Two"), ShouldMark));
///             parent
///                 .spawn_bundle((Name::new("Child Three"), ))
///                 .with_children(|parent| {
///                     parent.spawn_bundle((Name::new("Child Four"), ShouldMark));
///                 });
///         });
/// }
/// fn print_marked(ancestors: Query<(Entity, &Ancestor)>, names: Query<&Name>) {
///     for (entity, Ancestor(parent)) in &ancestors {
///         if let (Ok(name), Ok(parent_name)) = (names.get(entity), names.get(*parent)) {
///             println!("{name} has parent: {parent_name}");
///             // Child Two has parent: The Ancestor
///             // Child Four has parent: The Ancestor
///         }
///     }
/// }
/// ```
#[derive(Component, Clone, Copy)]
pub struct MarkDescendants(Option<TypeId>);
impl MarkDescendants {
    /// Mark all descendants.
    pub fn all() -> Self {
        MarkDescendants(None)
    }
    /// Only mark descendents with the specified [`Component`].
    pub fn limit_to<T: Component>() -> Self {
        MarkDescendants(Some(TypeId::of::<T>()))
    }
    /// Type-erased equivalent of [`MarkDescendants::limit_to`].
    ///
    /// `id` must be the `TypeId` of a `T: Component` already spawned in the world,
    /// otherwise `MarkDescendants` will simply be removed without doing anything.
    pub fn limit_to_id(id: TypeId) -> Self {
        MarkDescendants(Some(id))
    }
}

// NOTE: we take a `&mut World` because we type-erase the component type we want
// to limit the addition of `Ancestor` to.
/// Add [`Ancestor`] component to descendents of [`MarkDescendants`] according to its limit.
pub fn mark_descendants(world: &mut World) {
    // allow: we make a temporary Vec because we cannot both hold a mutable reference
    // to `world` (world.query) in the iterator, and the body of the for loop
    // (world.component and world.get_by_id) This is a clippy false positive
    #[allow(clippy::needless_collect)]
    let marking_ancestors: Vec<_> = world
        .query::<(Entity, &MarkDescendants)>()
        .iter(world)
        .map(|(entity, component)| (entity, *component))
        .collect();
    for (entity, MarkDescendants(limit_to)) in marking_ancestors.into_iter() {
        world.entity_mut(entity).remove::<MarkDescendants>();
        let limit_to = match limit_to {
            None => None,
            Some(type_id) => match world.components().get_id(type_id) {
                Some(component_id) => Some(component_id),
                None => {
                    warn!("The TypeId specified in `MarkDescendants::limit_to_id` is not a known component, skipping");
                    continue;
                }
            },
        };
        let mut to_explore = Vec::new();
        let mut to_mark = Vec::new();
        let mut current = entity;
        loop {
            if let Ok(children) = world.query::<&Children>().get(world, current) {
                to_explore.extend(children);
            }
            current = match to_explore.pop() {
                Some(new_current) => new_current,
                None => break,
            };
            let within_limit = limit_to.map_or(true, |id| world.entity(current).contains_id(id));
            if within_limit {
                to_mark.push(current);
            }
        }
        let _ = world.insert_or_spawn_batch(to_mark.into_iter().map(|e| (e, (Ancestor(entity),))));
    }
}
