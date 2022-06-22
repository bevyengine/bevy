use std::any::TypeId;

use bevy_utils::HashSet;

use crate::{prelude::{Bundle, Component}, world::World, component::ComponentId};

/// A rule about which [`Component`]s can coexist on entities
///
/// These rules must be true at all times for all entities in the [`World`].
///
/// When added to the [`World`], archetype invariants behave like [`assert!`];
/// all archetype invariants must be true for every entity in the [`World`].
/// Archetype invariants are checked each time [`Archetypes`] is modified;
/// this can occur on component addition, component removal, and entity spawning.
/// 
/// Archetypes are only modified when a novel archetype (set of components) is seen for the first time;
/// swapping between existing archetypes will not trigger these checks.
/// 
/// Note that this is converted to an [`UntypedArchetypeInvariant`] when added to a [`World`].
/// This is to ensure compatibility between different invariants.
#[derive(Clone, Debug, PartialEq)]
pub struct ArchetypeInvariant {
    /// For all entities where the predicate is true
    pub predicate: ArchetypeStatement,
    /// The consequence must also be true
    pub consequence: ArchetypeStatement,
}

impl ArchetypeInvariant {
    /// This is a helper function for constructing common invariants.
    /// All components of the provided bundle require each other.
    /// In other words, if any one component of this bundle is present, then all of them must be.
    #[inline]
    pub fn full_bundle<B: Bundle>() -> Self {
        Self { 
            predicate: ArchetypeStatement::at_least_one_of::<B>(),
            consequence: ArchetypeStatement::all_of::<B>()
        }
    }

    /// Erases the type information of this archetype invariant.
    /// 
    /// Requires mutable world access, since the components might not have been added to the world yet.
    #[inline]
    pub fn into_untyped(self, world: &mut World) -> UntypedArchetypeInvariant {
        UntypedArchetypeInvariant { 
            predicate: self.predicate.into_untyped(world),
            consequence: self.consequence.into_untyped(world)
        }
    }
}

/// A statement about the presence or absence of some subset of components in the given [`Bundle`]
///
/// This type is used as part of an [`ArchetypeInvariant`]. 
/// For the single-component equivalent, see [`ComponentStatement`]. 
///
/// When used as a predicate, the archetype invariant matches all entities which satisfy the statement. 
/// When used as a consquence, then the statment must be true for all entities that were matched by the predicate.
/// 
/// Note that this is converted to an [`UntypedArchetypeStatment`] when added to a [`World`].
/// This is to ensure compatibility between different invariants.
#[derive(Clone, Debug, PartialEq)]
pub enum ArchetypeStatement {
	/// Evaluates to true if and only if the entity has the component of type `C`
	Has(TypeId),
	/// Evaluates to true if and only if the entity does not have the component of type `C`
	DoesNotHave(TypeId),
	/// Evaluates to true if and only if the entity has all of the components present in Bundle `B`
    AllOf(HashSet<TypeId>),
    /// The entity has at least one component in the bundle, and may have all of them
    AtLeastOneOf(HashSet<TypeId>),
    /// The entity has none of the components in the bundle
    NoneOf(HashSet<TypeId>),
}

impl ArchetypeStatement {
    /// Erases the type information of this archetype statment.
    /// 
    /// Requires mutable world access, since the components might not have been added to the world yet.
    pub fn into_untyped(self, _world: &mut World) -> UntypedArchetypeStatement {
        match self {
            ArchetypeStatement::Has(_) => todo!(),
            ArchetypeStatement::DoesNotHave(_) => todo!(),
            ArchetypeStatement::AllOf(_) => todo!(),
            ArchetypeStatement::AtLeastOneOf(_) => todo!(),
            ArchetypeStatement::NoneOf(_) => todo!(),
        }
    }
    
    /// Constructs a new [`ArchetypeStatement::Has`] variant for a component of type `C`
	pub fn has<C: Component>() -> Self {
		let type_id = TypeId::of::<C>();
        ArchetypeStatement::Has(type_id)
    }

    /// Constructs a new [`ArchetypeStatement::DoesNotHave`] variant for a component of type `C`
	pub fn does_not_have<C: Component>() -> Self {
        let type_id = TypeId::of::<C>();
        ArchetypeStatement::DoesNotHave(type_id)
    }

	/// Constructs a new [`ArchetypeStatement::AllOf`] variant for all components stored in the bundle `B`
    pub fn all_of<B: Bundle>() -> Self {
        todo!()
    }
    
	/// Constructs a new [`ArchetypeStatement::AtLeastOneOf`] variant for all components stored in the bundle `B`
    pub fn at_least_one_of<B: Bundle>() -> Self {
        todo!()
    }
	
    /// Constructs a new [`ArchetypeStatement::NoneOf`] variant for all components stored in the bundle `B`
    pub fn none_of<B: Bundle>() -> Self {
        todo!()
    }
}

/// A type-erased version of [`ArchetypeInvariant`].
/// Intended to be used with dynamic components that cannot be represented with Rust types.
/// Prefer [`ArchetypeInvariant`] when possible.
#[derive(Clone, Debug, PartialEq)]
pub struct UntypedArchetypeInvariant {
    /// For all entities where the predicate is true
    pub predicate: UntypedArchetypeStatement,
    /// The consequence must also be true
    pub consequence: UntypedArchetypeStatement,
}

/// A type-erased version of [`ArchetypeStatment`]
/// Intended to be used with dynamic components that cannot be represented with Rust types.
/// Prefer [`ArchetypeStatment`] when possible.
#[derive(Clone, Debug, PartialEq)]
pub enum UntypedArchetypeStatement {
	/// Evaluates to true if and only if the entity has the component of type `C`
	Has(ComponentId),
	/// Evaluates to true if and only if the entity does not have the component of type `C`
	DoesNotHave(ComponentId),
	/// Evaluates to true if and only if the entity has all of the components present in Bundle `B`
    AllOf(HashSet<ComponentId>),
    /// The entity has at least one component in the bundle, and may have all of them
    AtLeastOneOf(HashSet<ComponentId>),
    /// The entity has none of the components in the bundle
    NoneOf(HashSet<ComponentId>),
}

#[derive(Default)]
pub struct ArchetypeInvariants {
    raw_list: Vec<UntypedArchetypeInvariant>,
    last_checked_archetype_index: u32,
}

impl ArchetypeInvariants {

    /// Adds a new [`ArchetypeInvariant`] to this set of archetype invariants.
    /// 
    /// Whenever a new archetype invariant is added, all existing archetypes are re-checked.
    /// This may include empty archetypes- archetypes that contain no entities.
    pub fn add(&mut self, archetype_invariant: UntypedArchetypeInvariant) {
        self.last_checked_archetype_index = 0;
        self.raw_list.push(archetype_invariant);
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        self as bevy_ecs,
        component::Component,
        world::World,
        world::archetype_invariants::ArchetypeInvariant
    };

    #[derive(Component)]
    struct A;
    
    #[derive(Component)]
    struct B;
    
    #[derive(Component)]
    struct C;

    #[test]
    fn full_bundle() {
        let mut world = World::new();

        world.add_archetype_invariant(
            ArchetypeInvariant::full_bundle::<(A, B, C)>()
        );

        todo!();
    }
}