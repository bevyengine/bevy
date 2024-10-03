use crate::{OneToManyEvent, OneToManyMany, OneToManyOne};

/// A familial relationship
#[cfg_attr(feature = "reflect", derive(bevy_reflect::Reflect))]
pub struct Family;

/// Holds a reference to the parent entity of this entity.
/// This component should only be present on entities that actually have a parent entity.
///
/// Parent entity must have this entity stored in its [`Children`] component.
/// It is hard to set up parent/child relationships manually,
/// consider using higher level utilities like [`BuildChildren::with_children`].
///
/// See [`HierarchyQueryExt`] for hierarchy related methods on [`Query`].
///
/// [`HierarchyQueryExt`]: crate::query_extension::HierarchyQueryExt
/// [`Query`]: bevy_ecs::system::Query
/// [`Children`]: super::children::Children
/// [`BuildChildren::with_children`]: crate::child_builder::BuildChildren::with_children
pub type Parent = OneToManyOne<Family>;

/// Contains references to the child entities of this entity.
///
/// Each child must contain a [`Parent`] component that points back to this entity.
/// This component rarely needs to be created manually,
/// consider using higher level utilities like [`BuildChildren::with_children`]
/// which are safer and easier to use.
///
/// See [`HierarchyQueryExt`] for hierarchy related methods on [`Query`].
///
/// [`HierarchyQueryExt`]: crate::query_extension::HierarchyQueryExt
/// [`Query`]: bevy_ecs::system::Query
/// [`Parent`]: crate::components::parent::Parent
/// [`BuildChildren::with_children`]: crate::child_builder::BuildChildren::with_children
pub type Children = OneToManyMany<Family>;

/// An [`Event`] that is fired whenever there is a change in the world's hierarchy.
///
/// [`Event`]: bevy_ecs::event::Event
pub type HierarchyEvent = OneToManyEvent<Family>;
