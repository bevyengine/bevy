use crate::resource::Resources;
use bevy_hecs::{Access, Query, World};
use bevy_utils::HashSet;
use fixedbitset::FixedBitSet;
use std::{any::TypeId, borrow::Cow};

/// Determines the strategy used to run the `run_thread_local` function in a [System]
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum ThreadLocalExecution {
    Immediate,
    NextFlush,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct SystemId(pub usize);

impl SystemId {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        SystemId(rand::random::<usize>())
    }
}

/// An ECS system that can be added to a [Schedule](crate::Schedule)
pub trait System: Send + Sync {
    fn name(&self) -> Cow<'static, str>;
    fn id(&self) -> SystemId;
    fn update_archetype_access(&mut self, world: &World);
    fn archetype_access(&self) -> &ArchetypeAccess;
    fn resource_access(&self) -> &TypeAccess;
    fn thread_local_execution(&self) -> ThreadLocalExecution;
    fn run(&mut self, world: &World, resources: &Resources);
    fn run_thread_local(&mut self, world: &mut World, resources: &mut Resources);
    fn initialize(&mut self, _world: &mut World, _resources: &mut Resources) {}
}

/// Provides information about the archetypes a [System] reads and writes
#[derive(Debug, Default)]
pub struct ArchetypeAccess {
    pub immutable: FixedBitSet,
    pub mutable: FixedBitSet,
}

// credit to Ratysz from the Yaks codebase
impl ArchetypeAccess {
    pub fn is_compatible(&self, other: &ArchetypeAccess) -> bool {
        self.mutable.is_disjoint(&other.mutable)
            && self.mutable.is_disjoint(&other.immutable)
            && self.immutable.is_disjoint(&other.mutable)
    }

    pub fn union(&mut self, other: &ArchetypeAccess) {
        self.mutable.union_with(&other.mutable);
        self.immutable.union_with(&other.immutable);
    }

    pub fn set_access_for_query<Q>(&mut self, world: &World)
    where
        Q: Query,
    {
        let iterator = world.archetypes();
        let bits = iterator.len();
        self.immutable.grow(bits);
        self.mutable.grow(bits);
        iterator
            .enumerate()
            .filter_map(|(index, archetype)| archetype.access::<Q>().map(|access| (index, access)))
            .for_each(|(archetype, access)| match access {
                Access::Read => self.immutable.set(archetype, true),
                Access::Write => self.mutable.set(archetype, true),
                Access::Iterate => (),
            });
    }

    pub fn clear(&mut self) {
        self.immutable.clear();
        self.mutable.clear();
    }
}

/// Provides information about the types a [System] reads and writes
#[derive(Debug, Default, Eq, PartialEq, Clone)]
pub struct TypeAccess {
    pub immutable: HashSet<TypeId>,
    pub mutable: HashSet<TypeId>,
}

impl TypeAccess {
    pub fn is_compatible(&self, other: &TypeAccess) -> bool {
        self.mutable.is_disjoint(&other.mutable)
            && self.mutable.is_disjoint(&other.immutable)
            && self.immutable.is_disjoint(&other.mutable)
    }

    pub fn union(&mut self, other: &TypeAccess) {
        self.mutable.extend(&other.mutable);
        self.immutable.extend(&other.immutable);
    }

    pub fn clear(&mut self) {
        self.immutable.clear();
        self.mutable.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::{ArchetypeAccess, TypeAccess};
    use crate::resource::{FetchResource, Res, ResMut, ResourceQuery};
    use bevy_hecs::World;
    use std::any::TypeId;

    struct A;
    struct B;
    struct C;

    #[test]
    fn query_archetype_access() {
        let mut world = World::default();
        let e1 = world.spawn((A,));
        let e2 = world.spawn((A, B));
        let e3 = world.spawn((A, B, C));

        let mut access = ArchetypeAccess::default();
        access.set_access_for_query::<(&A,)>(&world);

        let e1_archetype = world.get_entity_location(e1).unwrap().archetype as usize;
        let e2_archetype = world.get_entity_location(e2).unwrap().archetype as usize;
        let e3_archetype = world.get_entity_location(e3).unwrap().archetype as usize;

        assert!(access.immutable.contains(e1_archetype));
        assert!(access.immutable.contains(e2_archetype));
        assert!(access.immutable.contains(e3_archetype));

        let mut access = ArchetypeAccess::default();
        access.set_access_for_query::<(&A, &B)>(&world);

        assert!(access.immutable.contains(e1_archetype) == false);
        assert!(access.immutable.contains(e2_archetype));
        assert!(access.immutable.contains(e3_archetype));
    }

    #[test]
    fn resource_query_access() {
        let access =
            <<(Res<A>, ResMut<B>, Res<C>) as ResourceQuery>::Fetch as FetchResource>::access();
        let mut expected_access = TypeAccess::default();
        expected_access.immutable.insert(TypeId::of::<A>());
        expected_access.immutable.insert(TypeId::of::<C>());
        expected_access.mutable.insert(TypeId::of::<B>());
        assert_eq!(access, expected_access);
    }
}
