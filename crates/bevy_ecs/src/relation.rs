use crate::entity::Entity;
use crate::prelude::Component;

/// Represents a relation between two entities. Can be used to add a relation to an entity.
///
/// ## Examples
/// ```rust,ignore
/// commands.entity(foo).insert(rel(ChildOf, parent));
/// ```
pub struct Relation<C: Component> {
    pub relation: C,
    pub target: Entity,
}

/// Shorthand for constructing a relation. See [`Relation`] for more details.
pub fn rel<C: Component>(relation: C, target: Entity) -> Relation<C> {
    Relation { relation, target }
}