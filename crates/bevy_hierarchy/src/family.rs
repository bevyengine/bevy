use crate::{ManyToOne, OneToMany, OneToManyEvent};

/// A familial relationship
#[cfg_attr(feature = "reflect", derive(bevy_reflect::Reflect))]
pub struct Family;

/// Holds a reference to the parent entity of this entity.
/// This component should only be present on entities that actually have a parent entity.
///
/// Parent entity must have this entity stored in its [`Children`] component.
/// This invariant will be upheld using component hooks, but will only be valid after a sync point,
/// when deferred commands are applied.
/// To avoid this delay, consider using higher level utilities like [`BuildChildren::with_children`].
///
/// See [`HierarchyQueryExt`] for hierarchy related methods on [`Query`].
///
/// [`HierarchyQueryExt`]: crate::query_extension::HierarchyQueryExt
/// [`Query`]: bevy_ecs::system::Query
/// [`BuildChildren::with_children`]: crate::child_builder::BuildChildren::with_children
pub type Parent = ManyToOne<Family>;

/// Contains references to the child entities of this entity.
///
/// Each child must contain a [`Parent`] component that points back to this entity.
/// This component rarely needs to be created manually, the recommended way to
/// work with this component is to insert [`Parent`] on all child entities, as
/// component hooks will ensure this component is available.
/// You may also consider using higher level utilities like [`BuildChildren::with_children`].
///
/// See [`HierarchyQueryExt`] for hierarchy related methods on [`Query`].
///
/// [`HierarchyQueryExt`]: crate::query_extension::HierarchyQueryExt
/// [`Query`]: bevy_ecs::system::Query
/// [`BuildChildren::with_children`]: crate::child_builder::BuildChildren::with_children
pub type Children = OneToMany<Family>;

/// An [`Event`] that is fired whenever there is a change in the world's hierarchy.
///
/// [`Event`]: bevy_ecs::event::Event
pub type HierarchyEvent = OneToManyEvent<Family>;
