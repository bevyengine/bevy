use std::marker::PhantomData;

use bevy_utils::HashSet;

use crate::{
    component::{display_component_id_types, ComponentId, Components},
    prelude::Bundle,
    world::World,
};

/// A rule about which [`Component`](crate::component::Component)s can coexist on entities.
///
/// These rules must be true at all times for all entities in the [`World`].
/// The generic [`Bundle`] type `B1` is always used in the `premise`,
/// while `B2` is used in the `consequence`.
/// If only a single generic is provided, these types are the same.
///
/// When added to the [`World`], archetype invariants behave like [`assert!`].
/// Archetype invariants are checked each time [`Archetypes`](crate::archetype::Archetypes) is modified;
/// this can occur on component addition, component removal, and entity spawning.
///
/// Archetypes are only modified when a novel archetype (set of components) is seen for the first time;
/// swapping between existing archetypes will not trigger these checks.
///
/// Note that archetype invariants are not symmetric by default.
/// For example, `ArchetypeInvariant::<B1, B2>::requires_one()` means that `B1` requires `B2`,
/// but not that `B2` requires `B1`.
/// In this case, an entity with just `B2` is completely valid, but an entity with just `B1` is not.
/// If symmetry is desired, repeat the invariant with the order of the types switched.
///
/// When working with dynamic component types (for non-Rust components),
/// use [`UntypedArchetypeInvariant`] and [`UntypedArchetypeStatement`] instead.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ArchetypeInvariant<B1: Bundle, B2: Bundle = B1> {
    /// Defines which entities this invariant applies to.
    /// This is the "if" of the if/then clause.
    pub premise: ArchetypeStatement<B1>,
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
            premise: self.premise.into_untyped(world),
            consequence: self.consequence.into_untyped(world),
        }
    }
}

/// A statement about the presence or absence of some subset of components in the given [`Bundle`].
///
/// This type is used as part of an [`ArchetypeInvariant`].
///
/// When used as a premise, the archetype invariant matches all entities which satisfy the statement.
/// When used as a consequence, then the statment must be true for all entities that were matched by the premise.
///
/// For the statements about a single component `C`, wrap it in a single-component bundle `(C,)`.
/// For single component bundles, `AllOf` and `AnyOf` are equivalent.
/// Prefer `ArchetypeStatement::<(C,)>::all_of` over `ArchetypeStatement::<(C,)>::any_of` for consistency and clarity.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ArchetypeStatement<B: Bundle> {
    /// Evaluates to true if and only if the entity has all of the components present in the bundle `B`.
    AllOf(PhantomData<B>),
    /// The entity has at least one component in the bundle `B`.
    ///
    /// When using a single-component bundle, `AllOf` is preferred by convention.
    AnyOf(PhantomData<B>),
    /// The entity has zero or one of the components in the bundle `B`, but no more.
    ///
    /// When using a single-component bundle this is always true.
    /// Prefer the much clearer `True` variant.
    AtMostOneOf(PhantomData<B>),
    /// The entity has none of the components in the bundle `B`.
    NoneOf(PhantomData<B>),
    /// The entity contains only components from the bundle `B`, and no others.
    Only(PhantomData<B>),
    /// This statement is always true.
    ///
    /// Useful for constructing universal invariants.
    True,
    /// This statement is always false.
    ///
    /// Useful for constructing universal invariants.
    False,
}

impl<B: Bundle> ArchetypeStatement<B> {
    /// Erases the type information of this archetype statement.
    ///
    /// Requires mutable world access, since the components might not have been added to the world yet.
    pub fn into_untyped(self, world: &mut World) -> UntypedArchetypeStatement {
        let mut component_ids = Vec::new();
        B::component_ids(&mut world.components, &mut world.storages, &mut |id| {
            component_ids.push(id);
        });
        let component_ids: HashSet<ComponentId> = component_ids.into_iter().collect();

        match self {
            ArchetypeStatement::AllOf(_) => UntypedArchetypeStatement::AllOf(component_ids),
            ArchetypeStatement::AnyOf(_) => UntypedArchetypeStatement::AnyOf(component_ids),
            ArchetypeStatement::AtMostOneOf(_) => {
                UntypedArchetypeStatement::AtMostOneOf(component_ids)
            }
            ArchetypeStatement::NoneOf(_) => UntypedArchetypeStatement::NoneOf(component_ids),
            ArchetypeStatement::Only(_) => UntypedArchetypeStatement::Only(component_ids),
            ArchetypeStatement::True => UntypedArchetypeStatement::True,
            ArchetypeStatement::False => UntypedArchetypeStatement::False,
        }
    }

    /// Constructs a new [`ArchetypeStatement::AllOf`] variant for all components stored in the bundle `B`.
    #[inline]
    pub const fn all_of() -> Self {
        ArchetypeStatement::AllOf(PhantomData)
    }

    /// Constructs a new [`ArchetypeStatement::AnyOf`] variant for all components stored in the bundle `B`.
    #[inline]
    pub const fn any_of() -> Self {
        ArchetypeStatement::AnyOf(PhantomData)
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

    /// Constructs a new [`ArchetypeStatement::Only`] variant for all components stored in the bundle `B`.
    #[inline]
    pub const fn only() -> Self {
        ArchetypeStatement::Only(PhantomData)
    }
}

// We must pass in a generic type to all archetype statements;
// we use the empty bundle `()` by convention.
// These helper methods are useful because they improve type inference and consistency in user code.
impl ArchetypeStatement<()> {
    /// Constructs a new [`ArchetypeStatement::True`] variant.
    #[inline]
    pub const fn always_true() -> Self {
        ArchetypeStatement::<()>::True
    }

    /// Constructs a new [`ArchetypeStatement::False`] variant.
    #[inline]
    pub const fn always_false() -> Self {
        ArchetypeStatement::<()>::False
    }
}

/// Defines helper methods to eaily contruct common archetype invariants.
///
/// For more details on each method, see the implementation docs.
/// This trait is sealed: it should never be implemented by dependencies.
pub trait ArchetypeInvariantHelpers<B: Bundle>: private::Sealed {
    fn forbids<B2: Bundle>() -> ArchetypeInvariant<B, B2>;

    fn requires<B2: Bundle>() -> ArchetypeInvariant<B, B2>;

    fn requires_one<B2: Bundle>() -> ArchetypeInvariant<B, B2>;

    fn atomic() -> ArchetypeInvariant<B>;

    fn disjoint() -> ArchetypeInvariant<B>;

    fn exclusive() -> ArchetypeInvariant<B>;
}

impl<B: Bundle> ArchetypeInvariantHelpers<B> for B {
    /// Creates an archetype invariant where any component of `B` forbids every comonent from `B2`, and vice versa.
    ///
    /// In other words, if any component from `B` is present, then none of the components from `B2` can be present.
    /// Although this appears asymmetric, it actually implies its own converse.
    /// This is particularly useful for avoiding query conflicts.
    #[inline]
    fn forbids<B2: Bundle>() -> ArchetypeInvariant<B, B2> {
        ArchetypeInvariant {
            premise: ArchetypeStatement::<B>::any_of(),
            consequence: ArchetypeStatement::<B2>::none_of(),
        }
    }

    /// Creates an archetype invariant where components of `B` require all the components of `B2`.
    ///
    /// In other words, if any component from `B` is present, then all of the components from `B2` must be.
    #[inline]
    fn requires<B2: Bundle>() -> ArchetypeInvariant<B, B2> {
        ArchetypeInvariant {
            premise: ArchetypeStatement::<B>::any_of(),
            consequence: ArchetypeStatement::<B2>::all_of(),
        }
    }

    /// Creates an archetype invariant where components of `B` require at least one component of `B2`.
    ///
    /// In other words, if any component from `B` is present, then at least one component from `B2` must be.
    #[inline]
    fn requires_one<B2: Bundle>() -> ArchetypeInvariant<B, B2> {
        ArchetypeInvariant {
            premise: ArchetypeStatement::<B>::any_of(),
            consequence: ArchetypeStatement::<B2>::any_of(),
        }
    }

    /// Creates an archetype invariant where all components of `B` require each other.
    ///
    /// In other words, if any component of this bundle is present, then all of them must be.
    #[inline]
    fn atomic() -> ArchetypeInvariant<B> {
        ArchetypeInvariant {
            premise: ArchetypeStatement::<B>::any_of(),
            consequence: ArchetypeStatement::<B>::all_of(),
        }
    }

    /// Creates an archetype where components of `B` cannot appear with each other.
    ///
    /// In other words, if any component of this bundle is present, then no others can be.
    /// This is particularly useful for creating enum-like groups of components, such as `Dead` and `Alive`.
    #[inline]
    fn disjoint() -> ArchetypeInvariant<B> {
        ArchetypeInvariant {
            premise: ArchetypeStatement::<B>::any_of(),
            consequence: ArchetypeStatement::<B>::at_most_one_of(),
        }
    }

    /// Creates an archetype invariant where components of `B` can only appear with each other.
    ///
    /// In other words, if any component of this bundle is present, then _only_ components from this bundle can be present.
    #[inline]
    fn exclusive() -> ArchetypeInvariant<B> {
        ArchetypeInvariant {
            premise: ArchetypeStatement::<B>::any_of(),
            consequence: ArchetypeStatement::<B>::only(),
        }
    }
}

/// A special module used to prevent [`ArchetypeInvariantHelpers`] from being implemented by any other module.
///
/// For more information, see: <https://rust-lang.github.io/api-guidelines/future-proofing.html>.
mod private {
    use crate::prelude::Bundle;

    pub trait Sealed {}

    impl<B: Bundle> Sealed for B {}
}

/// A type-erased version of [`ArchetypeInvariant`].
///
/// Intended to be used with dynamic components that cannot be represented with Rust types.
/// Prefer [`ArchetypeInvariant`] when possible.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UntypedArchetypeInvariant {
    /// Defines which entities this invariant applies to.
    /// This is the "if" of the if/then clause.
    pub premise: UntypedArchetypeStatement,
    /// Defines what must be true for the entities that this invariant applies to.
    /// This is the "then" of the if/then clause.
    pub consequence: UntypedArchetypeStatement,
}

impl UntypedArchetypeInvariant {
    /// Asserts that the provided iterator of [`ComponentId`]s obeys this archetype invariant.
    ///
    /// When testing against multiple archetype invariants, [`ArchetypeInvariants::test_archetype`] is preferred,
    /// as it can more efficiently cache checks between archetypes.
    ///
    /// # Panics
    /// Panics if the archetype invariant is violated.
    pub(crate) fn test_archetype(
        &self,
        component_ids_of_archetype: impl Iterator<Item = ComponentId>,
        components: &Components,
    ) {
        let component_ids_of_archetype: HashSet<ComponentId> = component_ids_of_archetype.collect();

        if self.premise.test(&component_ids_of_archetype)
            && !self.consequence.test(&component_ids_of_archetype)
        {
            let archetype_component_names =
                display_component_id_types(component_ids_of_archetype.iter(), components);

            panic!(
                "Archetype invariant {} failed for archetype [{}]",
                self.display(components),
                archetype_component_names
            );
        }
    }

    /// Returns formatted string describing this archetype invariant
    pub fn display(&self, components: &Components) -> String {
        format!(
            "{{Premise: {}, Consequence: {}}}",
            self.premise.display(components),
            self.consequence.display(components)
        )
    }
}

/// A type-erased version of [`ArchetypeStatement`].
///
/// Intended to be used with dynamic components that cannot be represented with Rust types.
/// Prefer [`ArchetypeStatement`] when possible.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum UntypedArchetypeStatement {
    /// Evaluates to true if and only if the entity has all of the components present in the set.
    AllOf(HashSet<ComponentId>),
    /// The entity has at least one component in the set, and may have all of them.
    ///
    /// When using a single-component set, `AllOf` is preferred.
    AnyOf(HashSet<ComponentId>),
    /// The entity has zero or one of the components in the set, but no more.
    ///
    /// When using a single-component bundle this is always true.
    /// Prefer the much clearer `True` variant.
    AtMostOneOf(HashSet<ComponentId>),
    /// The entity has none of the components in the set.
    NoneOf(HashSet<ComponentId>),
    /// The entity contains only components from the bundle `B`, and no others.
    Only(HashSet<ComponentId>),
    /// This statement is always true.
    ///
    /// Useful for constructing universal invariants.
    True,
    /// This statement is always false.
    ///
    /// Useful for constructing universal invariants.
    False,
}

impl UntypedArchetypeStatement {
    /// Returns the set of [`ComponentId`]s affected by this statement.
    ///
    /// Returns `Some` for all variants other than the static `True` and `False`.
    pub fn component_ids(&self) -> Option<&HashSet<ComponentId>> {
        match self {
            UntypedArchetypeStatement::AllOf(set)
            | UntypedArchetypeStatement::AnyOf(set)
            | UntypedArchetypeStatement::AtMostOneOf(set)
            | UntypedArchetypeStatement::NoneOf(set)
            | UntypedArchetypeStatement::Only(set) => Some(set),
            UntypedArchetypeStatement::True | UntypedArchetypeStatement::False => None,
        }
    }

    /// Returns formatted string describing this archetype statement.
    ///
    /// For Rust types, the names should match the type name.
    /// If any [`ComponentId`]s in the statement have not been registered in the world,
    /// then the raw component id will be returned instead.
    pub fn display(&self, components: &Components) -> String {
        let component_names = display_component_id_types(
            self.component_ids().unwrap_or(&HashSet::new()).iter(),
            components,
        );

        match self {
            UntypedArchetypeStatement::AllOf(_) => format!("AllOf({component_names})"),
            UntypedArchetypeStatement::AnyOf(_) => format!("AnyOf({component_names})"),
            UntypedArchetypeStatement::AtMostOneOf(_) => format!("AtMostOneOf({component_names})"),
            UntypedArchetypeStatement::NoneOf(_) => format!("NoneOf({component_names})"),
            UntypedArchetypeStatement::Only(_) => format!("Only({component_names})"),
            UntypedArchetypeStatement::True => "True".to_owned(),
            UntypedArchetypeStatement::False => "False".to_owned(),
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
            UntypedArchetypeStatement::AnyOf(desired_ids) => {
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
            UntypedArchetypeStatement::Only(only_ids) => {
                for component_id in component_ids {
                    if !only_ids.contains(component_id) {
                        return false;
                    }
                }
                true
            }
            UntypedArchetypeStatement::True => true,
            UntypedArchetypeStatement::False => false,
        }
    }
}

/// A list of [`ArchetypeInvariant`]s, stored on a [`World`].
///
/// These store [`UntypedArchetypeInvariant`]s to ensure fast computation
/// and compatiblity with dynamic (non-Rust) component types.
#[derive(Default)]
pub struct ArchetypeInvariants {
    /// The list of invariants that must be upheld.
    raw_list: Vec<UntypedArchetypeInvariant>,
}

impl ArchetypeInvariants {
    /// Adds a new [`ArchetypeInvariant`] to this set of archetype invariants.
    #[inline]
    pub fn add(&mut self, archetype_invariant: UntypedArchetypeInvariant) {
        self.raw_list.push(archetype_invariant);
    }

    /// Returns the raw list of [`UntypedArchetypeInvariant`]s
    pub fn raw_list(&self) -> &Vec<UntypedArchetypeInvariant> {
        &self.raw_list
    }

    /// Asserts that the provided iterator of [`ComponentId`]s obeys all archetype invariants.
    ///
    /// # Panics
    ///
    /// Panics if any archetype invariant is violated.
    pub(crate) fn test_archetype(
        &self,
        component_ids_of_archetype: impl Iterator<Item = ComponentId>,
        components: &Components,
    ) {
        let component_ids_of_archetype: HashSet<ComponentId> = component_ids_of_archetype.collect();

        for invariant in &self.raw_list {
            if invariant.premise.test(&component_ids_of_archetype)
                && !invariant.consequence.test(&component_ids_of_archetype)
            {
                let mut failed_invariants = vec![];

                for invariant in &self.raw_list {
                    if invariant.premise.test(&component_ids_of_archetype)
                        && !invariant.consequence.test(&component_ids_of_archetype)
                    {
                        failed_invariants.push(invariant.clone());
                    }
                }

                let archetype_component_names =
                    display_component_id_types(component_ids_of_archetype.iter(), components);

                let failed_invariant_names = failed_invariants
                    .iter()
                    .map(|invariant| invariant.display(components))
                    .reduce(|acc, s| format!("{}\n{}", acc, s))
                    .unwrap_or_default();

                panic!(
                    "At least one archetype invariant was violated for the archetype [{archetype_component_names}]: \
                    \n{failed_invariant_names}"
                )
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate as bevy_ecs;
    use crate::{
        component::Component, world::archetype_invariants::ArchetypeInvariant,
        world::archetype_invariants::ArchetypeInvariantHelpers,
        world::archetype_invariants::ArchetypeStatement, world::World,
    };

    #[derive(Component)]
    struct A;

    #[derive(Component)]
    struct B;

    #[derive(Component)]
    struct C;

    #[test]
    fn on_insert_happy() {
        let mut world = World::new();

        world.spawn((A, B, C));
        world.add_archetype_invariant(<(A, B, C)>::atomic());
    }

    #[test]
    #[should_panic]
    fn on_insert_sad() {
        let mut world = World::new();

        world.spawn((A, B));
        world.add_archetype_invariant(<(A, B, C)>::atomic());
    }

    #[test]
    fn on_insert_untyped_happy() {
        let mut world = World::new();

        world.spawn((A, B, C));
        let archetype_invariant = <(A, B, C)>::atomic().into_untyped(&mut world);
        world.add_untyped_archetype_invariant(archetype_invariant);
    }

    #[test]
    #[should_panic]
    fn on_insert_untyped_sad() {
        let mut world = World::new();

        world.spawn((A, B));
        let archetype_invariant = <(A, B, C)>::atomic().into_untyped(&mut world);
        world.add_untyped_archetype_invariant(archetype_invariant);
    }

    #[test]
    fn tautology() {
        let mut world = World::new();

        // This invariant is a tautology.
        world.add_archetype_invariant(ArchetypeInvariant {
            premise: ArchetypeStatement::always_true(),
            consequence: ArchetypeStatement::always_true(),
        });
        // This invariant is also a tautology.
        world.add_archetype_invariant(ArchetypeInvariant {
            premise: ArchetypeStatement::always_false(),
            consequence: ArchetypeStatement::always_false(),
        });

        // Since invariants are only checked when archetypes are created,
        // we must add something to trigger the check.
        world.spawn(A);
    }

    #[test]
    #[should_panic]
    fn contradiction() {
        let mut world = World::new();

        // This invariant is a contradiction.
        world.add_archetype_invariant(ArchetypeInvariant {
            premise: ArchetypeStatement::always_true(),
            consequence: ArchetypeStatement::always_false(),
        });

        // Since invariants are only checked when archetypes are created,
        // we must add something to trigger the check.
        world.spawn(A);
    }

    #[test]
    fn forbids_happy() {
        let mut world = World::new();

        world.add_archetype_invariant(<(A,)>::forbids::<(B, C)>());
        world.spawn(A);
        world.spawn((B, C));
    }

    #[test]
    #[should_panic]
    fn forbids_sad() {
        let mut world = World::new();

        world.add_archetype_invariant(<(A,)>::forbids::<(B, C)>());
        world.spawn((A, B));
    }

    #[test]
    fn requires_happy() {
        let mut world = World::new();

        world.add_archetype_invariant(<(A,)>::requires::<(B, C)>());
        world.spawn((A, B, C));
        world.spawn((B, C));
    }

    #[test]
    #[should_panic]
    fn requires_sad_partial() {
        let mut world = World::new();

        world.add_archetype_invariant(<(A,)>::requires::<(B, C)>());
        world.spawn((A, B));
    }

    #[test]
    #[should_panic]
    fn requires_sad_none() {
        let mut world = World::new();

        world.add_archetype_invariant(<(A,)>::requires::<(B, C)>());
        world.spawn(A);
    }

    #[test]
    fn requires_one_happy() {
        let mut world = World::new();

        world.add_archetype_invariant(<(A,)>::requires_one::<(B, C)>());
        world.spawn((A, B, C));
        world.spawn((A, B));
        world.spawn((B, C));
    }

    #[test]
    #[should_panic]
    fn requires_one_sad() {
        let mut world = World::new();

        world.add_archetype_invariant(<(A,)>::requires_one::<(B, C)>());
        world.spawn(A);
    }

    #[test]
    fn atomic_happy() {
        let mut world = World::new();

        world.add_archetype_invariant(<(A, B, C)>::atomic());
        world.spawn((A, B, C));
    }

    #[test]
    #[should_panic]
    fn atomic_sad() {
        let mut world = World::new();

        world.add_archetype_invariant(<(A, B, C)>::atomic());
        world.spawn((A, B));
    }

    #[test]
    fn disjoint_happy() {
        let mut world = World::new();

        world.add_archetype_invariant(<(A, B, C)>::disjoint());
        world.spawn(A);
        world.spawn(B);
        world.spawn(C);
    }

    #[test]
    #[should_panic]
    fn disjoint_sad_partial() {
        let mut world = World::new();

        world.add_archetype_invariant(<(A, B, C)>::disjoint());
        world.spawn((A, B));
    }

    #[test]
    #[should_panic]
    fn disjoint_sad_all() {
        let mut world = World::new();

        world.add_archetype_invariant(<(A, B, C)>::disjoint());
        world.spawn((A, B, C));
    }

    #[test]
    fn exclusive_happy() {
        let mut world = World::new();

        world.add_archetype_invariant(<(A, B)>::exclusive());
        world.spawn((A, B));
        world.spawn(A);
        world.spawn(C);
    }

    #[test]
    #[should_panic]
    fn exclusive_sad_partial() {
        let mut world = World::new();

        world.add_archetype_invariant(<(A, B)>::exclusive());
        world.spawn((A, C));
    }

    #[test]
    #[should_panic]
    fn exclusive_sad_all() {
        let mut world = World::new();

        world.add_archetype_invariant(<(A, B)>::exclusive());
        world.spawn((A, B, C));
    }
}
