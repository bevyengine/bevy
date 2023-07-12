use crate::entity::Entity;
use crate::prelude::Component;

/// Represents a relation between two entities. Can be used to add a relation to an entity.
///
/// ## Examples
/// ```rust,ignore
/// commands.entity(foo).insert(rel(ChildOf, parent));
/// ```
pub struct Relation<C: Component> {
    /// The kind of relation. For example, `Eats`, `ChildOf`, `Has`, `IsAt`, etc.
    pub relation: C,
    /// The target of the relation. For `Eats`, this would be what is eaten. For `ChildOf`, this
    /// would be the parent. For `IsAt`, this would be the location.
    pub target: Entity,
}

/// Shorthand for constructing a relation. See [`Relation`] for more details.
pub fn rel<C: Component>(relation: C, target: Entity) -> Relation<C> {
    Relation { relation, target }
}
