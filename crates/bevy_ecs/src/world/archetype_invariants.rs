use std::marker::PhantomData;

use bevy_utils::{tracing::warn, HashSet};

use crate::{component::ComponentId, prelude::Bundle, world::World};

/// A rule about which [`Component`](crate::component::Component)s can coexist on entities.
///
/// These rules must be true at all times for all entities in the [`World`].
/// The generic [`Bundle`] type `B1` is always used in the `predicate`,
/// while `B2` is used in the `consequence`.
/// If only a single generic is provided, these types are the same.
///
/// When added to the [`World`], archetype invariants behave like [`assert!`].
/// Archetype invariants are checked each time [`Archetypes`](crate::archetype::Archetypes) is modified;
/// this can occur on component addition, component removal, and entity spawning.
///
/// Archetypes are only modified when a novel archetype (set of components) is seen for the first time;
/// swapping between existing archetypes will not trigger these checks.
#[derive(Clone, Debug, PartialEq)]
pub struct ArchetypeInvariant<B1: Bundle, B2: Bundle = B1> {
    /// Defines which entities this invariant applies to. 
    /// This is the "if" of the if/then clause.
    pub predicate: ArchetypeStatement<B1>,
    /// Defines what must be true for the entities that this invariant applies to.
    /// This is the "then" of the if/then clause.
    pub consequence: ArchetypeStatement<B2>,
}

impl<B1: Bundle, B2: Bundle> ArchetypeInvariant<B1, B2> {
    /// Erases the type information of this archetype invariant.
    ///
    /// Requires mutable world access, since the components might not have been added to the world yet.
    #[inline]
    pub fn into_untyped(self, world: &mut World) -> UntypedArchetypeInvariant {
        UntypedArchetypeInvariant {
            predicate: self.predicate.into_untyped(world),
            consequence: self.consequence.into_untyped(world),
        }
    }
}

impl<B: Bundle> ArchetypeInvariant<B, B> {
    /// Creates an archetype invariant where all components of `B` require each other.
    ///
    /// In other words, if any component of this bundle is present, then all of them must be.
    #[inline]
    pub fn atomic_bundle() -> Self {
        Self {
            predicate: ArchetypeStatement::<B>::at_least_one_of(),
            consequence: ArchetypeStatement::<B>::all_of(),
        }
    }
}

/// A statement about the presence or absence of some subset of components in the given [`Bundle`]
///
/// This type is used as part of an [`ArchetypeInvariant`].
///
/// When used as a predicate, the archetype invariant matches all entities which satisfy the statement.
/// When used as a consquence, then the statment must be true for all entities that were matched by the predicate.
///
/// For the statements about a single component `C`, wrap it in a single-component bundle `(C,)`.
/// For single component bundles, `AllOf` and `AtLeastOneOf` are equivalent.
/// Prefer `ArchetypeStatement::<(C,)>::all_of` over `ArchetypeStatement::<(C,)>::at_least_one_of` for consistency and clarity.
///
/// Note that this is converted to an [`UntypedArchetypeStatement`] when added to a [`World`].
/// This is to ensure compatibility between different invariants.
#[derive(Clone, Debug, PartialEq)]
pub enum ArchetypeStatement<B: Bundle> {
    /// Evaluates to true if and only if the entity has all of the components present in the bundle `B`.
    AllOf(PhantomData<B>),
    /// The entity has at least one component in the bundle `B`.
    /// When using a single-component bundle, `AllOf` is preferred.
    AtLeastOneOf(PhantomData<B>),
    /// The entity has zero or one of the components in the bundle `B`, but no more.
    /// When using a single-component bundle, this will always be true.
    AtMostOneOf(PhantomData<B>),
    /// The entity has none of the components in the bundle `B`.
    NoneOf(PhantomData<B>),
}

impl<B: Bundle> ArchetypeStatement<B> {
    /// Erases the type information of this archetype statement.
    ///
    /// Requires mutable world access, since the components might not have been added to the world yet.
    pub fn into_untyped(self, world: &mut World) -> UntypedArchetypeStatement {
        let component_ids = B::component_ids(&mut world.components, &mut world.storages);
        let component_ids: HashSet<ComponentId> = component_ids.into_iter().collect();

        match self {
            ArchetypeStatement::AllOf(_) => UntypedArchetypeStatement::AllOf(component_ids),
            ArchetypeStatement::AtLeastOneOf(_) => {
                if component_ids.len() == 1 {
                    warn!("An `ArchetypeStatement::AtLeastOneOf` was constructed for a bundle with only one component. Prefer the equivalent `ArchetypeStatment:AllOf` for consistency and clarity.");
                }
                UntypedArchetypeStatement::AtLeastOneOf(component_ids)
            }
            ArchetypeStatement::AtMostOneOf(_) => {
                UntypedArchetypeStatement::AtMostOneOf(component_ids)
            }
            ArchetypeStatement::NoneOf(_) => UntypedArchetypeStatement::NoneOf(component_ids),
        }
    }

    /// Constructs a new [`ArchetypeStatement::AllOf`] variant for all components stored in the bundle `B`.
    #[inline]
    pub const fn all_of() -> Self {
        ArchetypeStatement::AllOf(PhantomData)
    }

    /// Constructs a new [`ArchetypeStatement::AtLeastOneOf`] variant for all components stored in the bundle `B`.
    #[inline]
    pub const fn at_least_one_of() -> Self {
        ArchetypeStatement::AtLeastOneOf(PhantomData)
    }

    /// Constructs a new [`ArchetypeStatement::AtMostOneOf`] variant for all components stored in the bundle `B`.
    #[inline]
    pub const fn at_most_one_of() -> Self {
        ArchetypeStatement::AtMostOneOf(PhantomData)
    }

    /// Constructs a new [`ArchetypeStatement::NoneOf`] variant for all components stored in the bundle `B`.
    #[inline]
    pub const fn none_of() -> Self {
        ArchetypeStatement::NoneOf(PhantomData)
    }
}

/// A type-erased version of [`ArchetypeInvariant`].
///
/// Intended to be used with dynamic components that cannot be represented with Rust types.
/// Prefer [`ArchetypeInvariant`] when possible.
#[derive(Clone, Debug, PartialEq)]
pub struct UntypedArchetypeInvariant {
    /// For all entities where the predicate is true
    pub predicate: UntypedArchetypeStatement,
    /// The consequence must also be true
    pub consequence: UntypedArchetypeStatement,
}

impl UntypedArchetypeInvariant {
    /// Asserts that the provided iterator of [`ComponentId`]s obeys this archetype invariant.
    ///
    /// `component_ids` is generally provided via the `components` field on [`Archetype`](crate::archetype::Archetype).
    /// When testing against multiple archetypes, [`ArchetypeInvariants::test_archetype`] is preferred,
    /// as it can more efficiently cache checks between archetypes.
    ///
    /// # Panics
    /// Panics if the archetype invariant is violated.
    pub fn test_archetype(&self, component_ids_of_archetype: impl Iterator<Item = ComponentId>) {
        let component_ids_of_archetype: HashSet<ComponentId> = component_ids_of_archetype.collect();

        if self.predicate.test(&component_ids_of_archetype)
            && !self.consequence.test(&component_ids_of_archetype)
        {
            panic!(
                "Archetype invariant violated! The invariant {:?} failed for archetype {:?}",
                self, component_ids_of_archetype
            );
        }
    }
}

/// A type-erased version of [`ArchetypeStatement`].
///
/// Intended to be used with dynamic components that cannot be represented with Rust types.
/// Prefer [`ArchetypeStatement`] when possible.
#[derive(Clone, Debug, PartialEq)]
pub enum UntypedArchetypeStatement {
    /// Evaluates to true if and only if the entity has all of the components present in the set.
    AllOf(HashSet<ComponentId>),
    /// The entity has at least one component in the set, and may have all of them.
    /// When using a single-component set, `AllOf` is preferred.
    AtLeastOneOf(HashSet<ComponentId>),
    /// The entity has zero or one of the components in the set, but no more.
    /// When using a single-component set, this is a tautology.
    AtMostOneOf(HashSet<ComponentId>),
    /// The entity has none of the components in the set.
    NoneOf(HashSet<ComponentId>),
}

impl UntypedArchetypeStatement {
    /// Get the set of [`ComponentId`]s affected by this statement
    pub fn component_ids(&self) -> &HashSet<ComponentId> {
        match self {
            UntypedArchetypeStatement::AllOf(set)
            | UntypedArchetypeStatement::AtLeastOneOf(set)
            | UntypedArchetypeStatement::AtMostOneOf(set)
            | UntypedArchetypeStatement::NoneOf(set) => set,
        }
    }

    /// Test if this statement is true for the provided set of [`ComponentId`]s.
    pub fn test(&self, component_ids: &HashSet<ComponentId>) -> bool {
        match self {
            UntypedArchetypeStatement::AllOf(required_ids) => {
                for required_id in required_ids {
                    if !component_ids.contains(required_id) {
                        return false;
                    }
                }
                true
            }
            UntypedArchetypeStatement::AtLeastOneOf(desired_ids) => {
                for desired_id in desired_ids {
                    if component_ids.contains(desired_id) {
                        return true;
                    }
                }
                false
            }
            UntypedArchetypeStatement::AtMostOneOf(exclusive_ids) => {
                let mut found_previous = false;
                for exclusive_id in exclusive_ids {
                    if component_ids.contains(exclusive_id) {
                        if found_previous {
                            return false;
                        }
                        found_previous = true;
                    }
                }
                true
            }
            UntypedArchetypeStatement::NoneOf(forbidden_ids) => {
                for forbidden_id in forbidden_ids {
                    if component_ids.contains(forbidden_id) {
                        return false;
                    }
                }
                true
            }
        }
    }
}

/// A list of [`ArchetypeInvariant`]s to be stored on a [`World`].
#[derive(Default)]
pub struct ArchetypeInvariants {
    /// The list of invariants that must be upheld.
    raw_list: Vec<UntypedArchetypeInvariant>,
}

impl ArchetypeInvariants {
    /// Adds a new [`ArchetypeInvariant`] to this set of archetype invariants.
    ///
    /// Whenever a new archetype invariant is added, all existing archetypes are re-checked.
    /// This may include empty archetypes: archetypes that contain no entities.
    #[inline]
    pub fn add(&mut self, archetype_invariant: UntypedArchetypeInvariant) {
        self.raw_list.push(archetype_invariant);
    }

    /// Asserts that the provided iterator of [`ComponentId`]s obeys all archetype invariants.
    ///
    /// `component_ids` is generally provided via the `components` field on [`Archetype`](crate::archetype::Archetype).
    ///
    /// # Panics
    ///
    /// Panics if any archetype invariant is violated.
    pub fn test_archetype(&self, component_ids_of_archetype: impl Iterator<Item = ComponentId>) {
        let component_ids_of_archetype: HashSet<ComponentId> = component_ids_of_archetype.collect();

        for invariant in &self.raw_list {
            if invariant.predicate.test(&component_ids_of_archetype)
                && !invariant.consequence.test(&component_ids_of_archetype)
            {
                let mut failed_invariants = vec![];

                for invariant in &self.raw_list {
                    if invariant.predicate.test(&component_ids_of_archetype)
                        && !invariant.consequence.test(&component_ids_of_archetype)
                    {
                        failed_invariants.push(invariant.clone());
                    }
                }

                panic!(
                    "Archetype invariant violated! The following invariants were violated for archetype {:?}:\n{:?}",
                    component_ids_of_archetype,
                    failed_invariants,
                )
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        self as bevy_ecs, component::Component, world::archetype_invariants::ArchetypeInvariant,
        world::World,
    };

    #[derive(Component)]
    struct A;

    #[derive(Component)]
    struct B;

    #[derive(Component)]
    struct C;

    #[test]
    fn full_bundle_happy() {
        let mut world = World::new();

        world.add_archetype_invariant(ArchetypeInvariant::<(A, B, C)>::atomic_bundle());
        world.spawn().insert_bundle((A, B, C));
    }

    #[test]
    fn full_bundle_on_insert_happy() {
        let mut world = World::new();

        world.spawn().insert_bundle((A, B, C));
        world.add_archetype_invariant(ArchetypeInvariant::<(A, B, C)>::atomic_bundle());
    }

    #[test]
    #[should_panic]
    fn full_bundle_sad() {
        let mut world = World::new();

        world.add_archetype_invariant(ArchetypeInvariant::<(A, B, C)>::atomic_bundle());
        world.spawn().insert_bundle((A, B));
    }

    #[test]
    #[should_panic]
    fn full_bundle_on_insert_sad() {
        let mut world = World::new();

        world.spawn().insert_bundle((A, B));
        world.add_archetype_invariant(ArchetypeInvariant::<(A, B, C)>::atomic_bundle());
    }
}
