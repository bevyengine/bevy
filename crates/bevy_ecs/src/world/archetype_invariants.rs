use bevy_utils::HashSet;

use crate::{prelude::{Bundle, Component}, world::World, component::{ComponentId}};

/// A rule about the [`Component`]s that can coexist on entities
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
#[derive(Clone, Debug, PartialEq)]
pub struct ArchetypeInvariant {
    /// For all entities where the predicate is true
    pub predicate: ArchetypeStatement,
    /// The consequence must also be true
    pub consequence: ArchetypeStatement,
}

/// A statement about the presence or absence of some subset of components in the given [`Bundle`]
///
/// This type is used as part of an [`ArchetypeInvariant`]. 
/// For the single-component equivalent, see [`ComponentStatement`]. 
///
/// When used as a predicate, the archetype invariant matches all entities which satisfy the statement. 
/// When used as a consquence, then the statment must be true for all entities that were matched by the predicate.
#[derive(Clone, Debug, PartialEq)]
pub enum ArchetypeStatement {
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

impl ArchetypeStatement {
    /// Constructs a new [`ArchetypeStatement::Has`] variant for a component of type `C`
	pub fn has<C: Component>(world: &mut World) -> Self {
		let component_id = world.init_component::<C>();
        ArchetypeStatement::Has(component_id)
    }

    /// Constructs a new [`ArchetypeStatement::DoesNotHave`] variant for a component of type `C`
	pub fn does_not_have<C: Component>(world: &mut World) -> Self {
		let component_id = world.init_component::<C>();
        ArchetypeStatement::DoesNotHave(component_id)
    }

	/// Constructs a new [`ArchetypeStatement::AllOf`] variant for all components stored in the bundle `B`
    pub fn all_of<B: Bundle>(world: &mut World) -> Self {
        let component_ids = B::component_ids(&mut world.components, &mut world.storages);
        ArchetypeStatement::AllOf(component_ids.into_iter().collect())
    }
    
	/// Constructs a new [`ArchetypeStatement::AtLeastOneOf`] variant for all components stored in the bundle `B`
    pub fn at_least_one_of<B: Bundle>(world: &mut World) -> Self {
        let component_ids = B::component_ids(&mut world.components, &mut world.storages);
        ArchetypeStatement::AtLeastOneOf(component_ids.into_iter().collect())
    }
	
    /// Constructs a new [`ArchetypeStatement::NoneOf`] variant for all components stored in the bundle `B`
    pub fn none_of<B: Bundle>(world: &mut World) -> Self {
        let component_ids = B::component_ids(&mut world.components, &mut world.storages);
        ArchetypeStatement::NoneOf(component_ids.into_iter().collect())
    }
}

#[derive(Default)]
pub struct ArchetypeInvariants {
    raw_list: Vec<ArchetypeInvariant>,
    last_checked_archetype_index: u32,
}

impl ArchetypeInvariants {

    /// Adds a new [`ArchetypeInvariant`] to this set of archetype invariants.
    pub fn add(&mut self, archetype_invariant: ArchetypeInvariant) {
        self.raw_list.push(archetype_invariant);
    }
}