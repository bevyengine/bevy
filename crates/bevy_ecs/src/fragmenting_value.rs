//! This module defines the concept of "fragmenting value" - a type that can be used to fragment
//! archetypes based on it's value in addition to it's type. The main trait is [`FragmentingValue`],
//! which is used to give each value that implements it a value-based identity, which is used by
//! other ecs functions to fragment archetypes.

use alloc::boxed::Box;
use alloc::vec::Vec;
use bevy_ecs::component::Component;
use bevy_platform::hash::FixedHasher;
use bevy_ptr::Ptr;
use core::{
    any::{Any, TypeId},
    borrow::Borrow,
    hash::{BuildHasher, Hash, Hasher},
    ptr::NonNull,
};
use indexmap::Equivalent;

use crate::{
    bundle::Bundle,
    component::{ComponentId, ComponentKey, Components, ComponentsRegistrator, Immutable},
};

/// Trait used to define values that can fragment archetypes.
///
/// This trait is automatically implemented for all immutable components that also implement [`Eq`], [`Hash`] and [`Clone`].
///
/// For dynamic components see [`DynamicFragmentingValue`] and [`FragmentingValueVtable`].
pub trait FragmentingValue: Any {
    /// Return `true` if `self == other`. Dynamic version of [`PartialEq::eq`].
    ///
    /// **NOTE**: This method must be implemented similarly [`PartialEq::eq`], however
    /// when comparing `dyn FragmentingValue`s prefer to use `==` to support values created using [`DynamicFragmentingValue`].
    fn value_eq(&self, other: &dyn FragmentingValue) -> bool;
    /// Returns the hash value of `self`. Dynamic version of [`Hash::hash`].
    fn value_hash(&self) -> u64;
    /// Returns a boxed clone of `self`. Dynamic version of [`Clone::clone`].
    fn clone_boxed(&self) -> Box<dyn FragmentingValue>;
}

impl<T> FragmentingValue for T
where
    T: Component<Mutability = Immutable> + Eq + Hash + Clone,
{
    fn value_eq(&self, other: &dyn FragmentingValue) -> bool {
        #[expect(clippy::let_unit_value, reason = "This is used for static asserts")]
        {
            _ = T::Key::INVARIANT_ASSERT;
        }
        if let Some(other) = (other as &dyn Any).downcast_ref::<T>() {
            return self == other;
        }
        false
    }

    fn value_hash(&self) -> u64 {
        FixedHasher.hash_one(self)
    }

    fn clone_boxed(&self) -> Box<dyn FragmentingValue> {
        Box::new(self.clone())
    }
}

impl FragmentingValue for () {
    fn value_eq(&self, other: &dyn FragmentingValue) -> bool {
        (other as &dyn Any).is::<()>()
    }

    fn value_hash(&self) -> u64 {
        FixedHasher.hash_one(())
    }

    fn clone_boxed(&self) -> Box<dyn FragmentingValue> {
        Box::new(())
    }
}

impl PartialEq for dyn FragmentingValue {
    fn eq(&self, other: &Self) -> bool {
        Self::value_eq_dynamic(self, other)
    }
}

impl Eq for dyn FragmentingValue {}

impl Hash for dyn FragmentingValue {
    fn hash<H: Hasher>(&self, state: &mut H) {
        let key_hash = FragmentingValue::value_hash(self);
        state.write_u64(key_hash);
    }
}

impl dyn FragmentingValue {
    /// Return `true` if `self == other`. This version supports values created using [`DynamicFragmentingValue`].
    #[inline(always)]
    fn value_eq_dynamic(&self, other: &dyn FragmentingValue) -> bool {
        self.value_eq(other)
            || (other as &dyn Any)
                .downcast_ref::<DynamicFragmentingValueInner>()
                .is_some_and(|other| other.value_eq(self))
    }
}

/// A collection of fragmenting component values and ids.
/// This collection is sorted internally to allow for order-independent comparison.
///
/// Owned version can be used as a key in maps. [`FragmentingValuesBorrowed`] is a version that doesn't require cloning the values.
#[derive(Hash, PartialEq, Eq, Default)]
pub struct FragmentingValuesOwned {
    values: Box<[(ComponentId, Box<dyn FragmentingValue>)]>,
}

impl FragmentingValuesOwned {
    /// Returns `true` if there are no fragmenting values.
    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }

    /// Returns fragmenting component values with their corresponding [`ComponentId`]s.
    pub fn iter_ids_and_values(
        &self,
    ) -> impl Iterator<Item = (ComponentId, &dyn FragmentingValue)> {
        self.values.iter().map(|(id, v)| (*id, v.as_ref()))
    }
}

impl<T: Borrow<dyn FragmentingValue>> FromIterator<(ComponentId, T)> for FragmentingValuesOwned {
    fn from_iter<I: IntoIterator<Item = (ComponentId, T)>>(iter: I) -> Self {
        let mut values = Vec::new();
        for (id, value) in iter {
            values.push((id, value.borrow().clone_boxed()));
        }
        values.sort_unstable_by_key(|(id, _)| *id);
        FragmentingValuesOwned {
            values: values.into_boxed_slice(),
        }
    }
}

/// A collection of fragmenting component values and ids.
/// This collection is sorted internally to allow for order-independent comparison.
///
/// Borrowed version is used to query maps with [`FragmentingValuesOwned`] keys.
#[derive(Hash, PartialEq, Eq, Default)]
pub struct FragmentingValuesBorrowed<'a> {
    values: Vec<(ComponentId, &'a dyn FragmentingValue)>,
}

impl<'a> FragmentingValuesBorrowed<'a> {
    /// Borrows fragmenting values from a [`Bundle`].
    /// This is used to compare fragmenting values without cloning bundle's data.
    pub fn from_bundle<B: Bundle>(components: &mut ComponentsRegistrator, bundle: &'a B) -> Self {
        let mut values = Vec::new();
        bundle.get_fragmenting_values(components, &mut |id, value| values.push((id, value)));
        values.sort_unstable_by_key(|(id, _)| *id);
        FragmentingValuesBorrowed { values }
    }

    /// Returns `true` if there are no fragmenting values.
    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }

    /// Creates [`FragmentingValuesOwned`] by cloning all fragmenting values.
    pub fn to_owned(&self) -> FragmentingValuesOwned {
        FragmentingValuesOwned {
            values: self
                .values
                .iter()
                .map(|(id, v)| (*id, v.clone_boxed()))
                .collect(),
        }
    }

    /// Returns fragmenting component values with their corresponding [`ComponentId`]s.
    pub fn iter_ids_and_values(
        &self,
    ) -> impl Iterator<Item = (ComponentId, &'a dyn FragmentingValue)> {
        self.values.iter().map(|(id, v)| (*id, *v))
    }
}

impl<'a> FromIterator<(ComponentId, &'a dyn FragmentingValue)> for FragmentingValuesBorrowed<'a> {
    fn from_iter<I: IntoIterator<Item = (ComponentId, &'a dyn FragmentingValue)>>(iter: I) -> Self {
        let mut values = Vec::new();
        for (id, value) in iter {
            values.push((id, value));
        }
        values.sort_unstable_by_key(|(id, _)| *id);
        FragmentingValuesBorrowed { values }
    }
}

impl<'a> Equivalent<FragmentingValuesOwned> for FragmentingValuesBorrowed<'a> {
    fn equivalent(&self, key: &FragmentingValuesOwned) -> bool {
        self.values.len() == key.values.len()
            && self
                .values
                .iter()
                .zip(key.values.iter())
                // We know that v2 is never an instance of DynamicFragmentingValue since it is from FragmentingValuesOwned.
                // Because FragmentingValuesOwned is created by calling clone_boxed, it always creates Box<T> of a proper type that DynamicFragmentingValues abstracts.
                // This means that we don't have to use value_eq_dynamic implementation and can compare with value_eq instead.
                .all(|((id1, v1), (id2, v2))| id1 == id2 && v1.value_eq(&**v2))
    }
}

#[derive(Default)]
enum DynamicFragmentingValueInner {
    #[default]
    Uninit,
    Borrowed {
        value: NonNull<u8>,
        vtable: FragmentingValueVtable,
    },
}

/// Holder for a reference to a dynamic fragmenting component and a vtable.
#[derive(Default)]
pub struct DynamicFragmentingValue(DynamicFragmentingValueInner);

impl FragmentingValue for DynamicFragmentingValueInner {
    fn value_eq(&self, other: &dyn FragmentingValue) -> bool {
        match self {
            DynamicFragmentingValueInner::Uninit => panic!("Uninitialized"),
            DynamicFragmentingValueInner::Borrowed { value, vtable } => (vtable.eq)(*value, other),
        }
    }

    fn value_hash(&self) -> u64 {
        match self {
            DynamicFragmentingValueInner::Uninit => panic!("Uninitialized"),
            DynamicFragmentingValueInner::Borrowed { value, vtable } => (vtable.hash)(*value),
        }
    }

    fn clone_boxed(&self) -> Box<dyn FragmentingValue> {
        match self {
            DynamicFragmentingValueInner::Uninit => panic!("Uninitialized"),
            DynamicFragmentingValueInner::Borrowed { value, vtable } => {
                (vtable.clone_boxed)(*value)
            }
        }
    }
}

impl DynamicFragmentingValue {
    /// Create a new `&dyn` [`FragmentingValue`] from passed component data and id.
    /// This is used mostly to construct [`FragmentingValuesBorrowed`] to compare dynamic components without copying data.
    ///
    /// Will return `None` if component isn't fragmenting.
    ///
    /// # Safety
    /// - `component_id` must match data which `component` points to.
    pub unsafe fn from_component<'a>(
        &'a mut self,
        components: &Components,
        component_id: ComponentId,
        component: Ptr<'a>,
    ) -> Option<&'a dyn FragmentingValue> {
        let info = components.get_info(component_id)?;
        let vtable = info.value_component_vtable()?;
        let inner = DynamicFragmentingValueInner::Borrowed {
            value: NonNull::new_unchecked(component.as_ptr()),
            vtable: *vtable,
        };
        self.0 = inner;
        Some(&self.0)
    }

    /// Create a new `&dyn` [`FragmentingValue`] from passed component data and [`FragmentingValueVtable`].
    ///
    /// # Safety
    /// - `vtable` must be usable for the data which `component` points to.
    pub unsafe fn from_vtable<'a>(
        &'a mut self,
        vtable: FragmentingValueVtable,
        component: Ptr<'a>,
    ) -> &'a dyn FragmentingValue {
        let inner = DynamicFragmentingValueInner::Borrowed {
            value: NonNull::new_unchecked(component.as_ptr()),
            vtable,
        };
        self.0 = inner;
        &self.0
    }
}

/// Dynamic vtable for [`FragmentingValue`].
/// This is used by [`crate::component::ComponentDescriptor`] to work with dynamic fragmenting components.
#[derive(Clone, Copy, Debug)]
pub struct FragmentingValueVtable {
    eq: fn(NonNull<u8>, &dyn FragmentingValue) -> bool,
    hash: fn(NonNull<u8>) -> u64,
    clone_boxed: fn(NonNull<u8>) -> Box<dyn FragmentingValue>,
}
impl FragmentingValueVtable {
    /// Create a new vtable from raw functions.
    ///
    /// Also see [`from_fragmenting_value`](FragmentingValueVtable::from_fragmenting_value) and
    /// [`from_component`](FragmentingValueVtable::from_component) for more convenient constructors.
    pub fn new(
        eq: fn(NonNull<u8>, &dyn FragmentingValue) -> bool,
        hash: fn(NonNull<u8>) -> u64,
        clone_boxed: fn(NonNull<u8>) -> Box<dyn FragmentingValue>,
    ) -> Self {
        Self {
            eq,
            hash,
            clone_boxed,
        }
    }

    /// Creates [`FragmentingValueVtable`] from existing [`FragmentingValue`].
    pub fn from_fragmenting_value<T: FragmentingValue>() -> Self {
        FragmentingValueVtable {
            eq: |ptr, other| {
                // SAFETY: caller is responsible for using this vtable only with correct values
                *(unsafe { ptr.cast::<T>().as_ref() } as &dyn FragmentingValue) == *other
            },
            hash: |ptr| {
                // SAFETY: caller is responsible for using this vtable only with correct values
                unsafe { ptr.cast::<T>().as_ref() }.value_hash()
            },
            clone_boxed: |ptr| {
                // SAFETY: caller is responsible for using this vtable only with correct values
                unsafe { ptr.cast::<T>().as_ref() }.clone_boxed()
            },
        }
    }

    /// Creates [`FragmentingValueVtable`] from a [`Component`].
    ///
    /// This will return `None` if the component isn't fragmenting.
    pub fn from_component<T: Component>() -> Option<Self> {
        if TypeId::of::<<T::Key as ComponentKey>::KeyType>() == TypeId::of::<T>() {
            Some(Self::from_fragmenting_value::<
                <T::Key as ComponentKey>::KeyType,
            >())
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use core::{
        alloc::Layout,
        any::Any,
        hash::{BuildHasher, Hasher},
        ptr::NonNull,
    };

    use crate::{
        archetype::ArchetypeId,
        component::{Component, ComponentCloneBehavior, ComponentDescriptor, StorageType},
        entity::Entity,
        fragmenting_value::{DynamicFragmentingValue, FragmentingValue, FragmentingValueVtable},
        world::World,
    };
    use alloc::boxed::Box;
    use alloc::vec::Vec;
    use bevy_platform::hash::FixedHasher;
    use bevy_ptr::{OwningPtr, Ptr};
    use core::hash::Hash;

    #[derive(Component, Clone, Eq, PartialEq, Hash)]
    #[component(
        key=Self,
        immutable,
    )]
    struct Fragmenting(u32);

    #[derive(Component)]
    struct NonFragmenting;

    #[test]
    fn fragment_on_spawn() {
        let mut world = World::default();
        let e1 = world.spawn(Fragmenting(1)).id();
        let e2 = world.spawn(Fragmenting(2)).id();
        let e3 = world.spawn(Fragmenting(1)).id();

        let [id1, id2, id3] = world.entity([e1, e2, e3]).map(|e| e.archetype().id());
        assert_eq!(id1, id3);
        assert_ne!(id2, id1);
    }

    #[test]
    fn fragment_on_spawn_batch() {
        let mut world = World::default();
        let entities: [Entity; 5] = world
            .spawn_batch([
                Fragmenting(1),
                Fragmenting(2),
                Fragmenting(1),
                Fragmenting(3),
                Fragmenting(1),
            ])
            .collect::<Vec<_>>()
            .try_into()
            .unwrap();

        let [id1, id2, id3, id4, id5] = world.entity(entities).map(|e| e.archetype().id());
        assert_eq!(id1, id3);
        assert_eq!(id1, id5);
        assert_ne!(id1, id2);
        assert_ne!(id1, id4);

        assert_ne!(id2, id4);
    }

    #[test]
    fn fragment_on_insert() {
        let mut world = World::default();
        let e1 = world.spawn_empty().id();
        let e2 = world.spawn_empty().id();

        world.entity_mut(e1).insert(Fragmenting(1));
        world.entity_mut(e2).insert(Fragmenting(2));

        let [id1, id2] = world.entity([e1, e2]).map(|e| e.archetype().id());
        assert_ne!(id1, id2);

        world.entity_mut(e2).insert(Fragmenting(1));

        let [id1, id2] = world.entity([e1, e2]).map(|e| e.archetype().id());
        assert_eq!(id1, id2);

        world.entity_mut(e1).insert(Fragmenting(3));
        world.entity_mut(e2).insert(Fragmenting(3));

        let [id1, id2] = world.entity([e1, e2]).map(|e| e.archetype().id());
        assert_eq!(id1, id2);
    }

    #[test]
    fn fragment_on_insert_batch() {
        let mut world = World::default();
        let entities: [Entity; 5] = world
            .spawn_batch([
                Fragmenting(1),
                Fragmenting(2),
                Fragmenting(1),
                Fragmenting(3),
                Fragmenting(1),
            ])
            .collect::<Vec<_>>()
            .try_into()
            .unwrap();
        world.insert_batch(entities.into_iter().zip([
            Fragmenting(2),
            Fragmenting(2),
            Fragmenting(3),
            Fragmenting(4),
            Fragmenting(4),
        ]));

        let [id1, id2, id3, id4, id5] = world.entity(entities).map(|e| e.archetype().id());
        assert_eq!(id1, id2);
        assert_ne!(id1, id3);
        assert_ne!(id1, id4);

        assert_ne!(id3, id4);

        assert_eq!(id4, id5);
    }

    #[test]
    fn fragment_on_remove() {
        let mut world = World::default();
        let entities: [Entity; 4] = world
            .spawn_batch([
                (Fragmenting(1), NonFragmenting),
                (Fragmenting(1), NonFragmenting),
                (Fragmenting(1), NonFragmenting),
                (Fragmenting(2), NonFragmenting),
            ])
            .collect::<Vec<_>>()
            .try_into()
            .unwrap();

        world.entity_mut(entities[1]).remove::<Fragmenting>();
        world.entity_mut(entities[2]).remove::<NonFragmenting>();
        world.entity_mut(entities[3]).remove::<Fragmenting>();

        let [id1, id2, id3, id4] = world.entity(entities).map(|e| e.archetype().id());
        assert_ne!(id1, id2);
        assert_ne!(id1, id3);
        assert_ne!(id1, id4);

        assert_ne!(id2, id3);
        assert_eq!(id2, id4);

        world.entity_mut(entities[3]).insert(Fragmenting(1));
        let [id1, id4] = world
            .entity([entities[0], entities[3]])
            .map(|e| e.archetype().id());
        assert_eq!(id1, id4);
    }

    #[test]
    fn fragment_dynamic() {
        let mut world = World::default();

        const COMPONENT_SIZE: usize = 10;
        #[derive(Clone)]
        struct DynamicComponentWrapper<T> {
            id: u64,
            data: T,
        }

        type DynamicComponent = DynamicComponentWrapper<[u8; COMPONENT_SIZE]>;

        impl<T: Eq + PartialEq + Hash + Clone + 'static> FragmentingValue for DynamicComponentWrapper<T> {
            fn value_eq(&self, other: &dyn FragmentingValue) -> bool {
                (other as &dyn Any)
                    .downcast_ref::<DynamicComponentWrapper<T>>()
                    .is_some_and(|other| other.id == self.id && other.data == self.data)
            }

            fn value_hash(&self) -> u64 {
                let mut hasher = FixedHasher.build_hasher();
                self.id.hash(&mut hasher);
                self.data.hash(&mut hasher);
                hasher.finish()
            }

            fn clone_boxed(&self) -> Box<dyn FragmentingValue> {
                Box::new(self.clone())
            }
        }
        let layout = Layout::new::<DynamicComponent>();
        let vtable = FragmentingValueVtable::from_fragmenting_value::<DynamicComponent>();

        // SAFETY:
        // - No drop command is required
        // - The component is Send and Sync
        // - vtable matches the component layout, mutable is false, storage type is SparseSet
        let descriptor = unsafe {
            ComponentDescriptor::new_with_layout(
                "DynamicComp1",
                StorageType::SparseSet,
                layout,
                None,
                false,
                ComponentCloneBehavior::Ignore,
                Some(vtable),
            )
        };
        let component_id1 = world.register_component_with_descriptor(descriptor);
        // SAFETY:
        // - No drop command is required
        // - The component is Send and Sync
        // - vtable matches the component layout, mutable is false, storage type is SparseSet
        let descriptor = unsafe {
            ComponentDescriptor::new_with_layout(
                "DynamicComp2",
                StorageType::SparseSet,
                layout,
                None,
                false,
                ComponentCloneBehavior::Ignore,
                Some(vtable),
            )
        };
        let component_id2 = world.register_component_with_descriptor(descriptor);

        let component1_1 = DynamicComponent {
            id: 1,
            data: [5; 10],
        };
        let component1_2 = DynamicComponent {
            id: 1,
            data: [8; 10],
        };
        let component2_1 = DynamicComponent {
            id: 2,
            data: [5; 10],
        };

        let entities: [Entity; 3] = (0..3)
            .map(|_| world.spawn_empty().id())
            .collect::<Vec<_>>()
            .try_into()
            .unwrap();
        // SAFETY: all ids and pointers match
        unsafe {
            world.entity_mut(entities[0]).insert_by_id(
                component_id1,
                OwningPtr::new(NonNull::from(&component1_1).cast()),
            );
            world.entity_mut(entities[1]).insert_by_ids(
                &[component_id1, component_id2],
                [
                    OwningPtr::new(NonNull::from(&component1_1).cast()),
                    OwningPtr::new(NonNull::from(&component2_1).cast()),
                ]
                .into_iter(),
            );
            world.entity_mut(entities[2]).insert_by_id(
                component_id1,
                OwningPtr::new(NonNull::from(&component1_2).cast()),
            );
        }

        let [id1, id2, id3] = world.entity(entities).map(|e| e.archetype().id());
        assert_ne!(id1, id2);
        assert_ne!(id1, id3);
    }

    #[test]
    fn fragmenting_component_compare_with_dynamic() {
        let mut world = World::default();

        let component_id = world.register_component::<Fragmenting>();

        let e1 = world.spawn(Fragmenting(1)).id();
        let e2 = world.spawn_empty().id();
        OwningPtr::make(Fragmenting(1), |ptr|
        // SAFETY: 
        // - ComponentId is from the same world.
        // - OwningPtr points to valid value of type represented by ComponentId
        unsafe {
            world.entity_mut(e2).insert_by_id(component_id, ptr);
        });
        let e3 = world.spawn_empty().id();
        OwningPtr::make(Fragmenting(1), |ptr|
        // SAFETY: 
        // - ComponentId is from the same world.
        // - OwningPtr points to valid value of type represented by ComponentId
        unsafe {
            world
                .entity_mut(e3)
                .insert_by_ids(&[component_id], [ptr].into_iter());
        });

        let e4 = world.spawn(Fragmenting(1)).id();
        let e5 = world.spawn_empty().id();
        OwningPtr::make(Fragmenting(1), |ptr|
        // SAFETY: 
        // - ComponentId is from the same world.
        // - OwningPtr points to valid value of type represented by ComponentId
        unsafe {
            world.entity_mut(e5).insert_by_id(component_id, ptr);
        });
        let e6 = world.spawn_empty().id();
        OwningPtr::make(Fragmenting(1), |ptr|
        // SAFETY: 
        // - ComponentId is from the same world.
        // - OwningPtr points to valid value of type represented by ComponentId
        unsafe {
            world
                .entity_mut(e6)
                .insert_by_ids(&[component_id], [ptr].into_iter());
        });

        let [id1, id2, id3, id4, id5, id6] = world
            .entity([e1, e2, e3, e4, e5, e6])
            .map(|e| e.archetype().id());

        assert_eq!(id1, id2);
        assert_eq!(id1, id3);
        assert_eq!(id1, id4);
        assert_eq!(id1, id5);
        assert_eq!(id1, id6);
    }

    #[test]
    fn fragmenting_value_compare_with_dynamic() {
        let value: &dyn FragmentingValue = &Fragmenting(1);
        let mut dynamic_holder = DynamicFragmentingValue::default();
        let vtable = FragmentingValueVtable::from_fragmenting_value::<Fragmenting>();
        // SAFETY:
        // - vtable matches component data
        let dynamic = unsafe { dynamic_holder.from_vtable(vtable, Ptr::from(&Fragmenting(1))) };

        assert!(*value == *dynamic);
        assert!(*dynamic == *value);
    }

    #[test]
    fn fragmenting_value_edges_cache_does_not_reset() {
        let mut world = World::default();

        let bundle_id = world.register_bundle::<Fragmenting>().id();
        let get_keys = |world: &World, archetype_id: ArchetypeId| {
            world.archetypes[archetype_id]
                .edges()
                .get_archetype_after_bundle_insert_internal(bundle_id)
                .unwrap()
                .fragmenting_values_map
                .keys()
                .len()
        };

        let empty_archetype = world.spawn_empty().archetype().id();

        // Added components path
        let entity = world.spawn(Fragmenting(1));
        let fragmenting_archetype = entity.archetype().id();
        assert_eq!(get_keys(&world, empty_archetype), 1);

        world.spawn(Fragmenting(2));
        assert_eq!(get_keys(&world, empty_archetype), 2);

        // No new components path
        let e1 = world.spawn(Fragmenting(1)).id();
        world.entity_mut(e1).insert(Fragmenting(2));
        assert_eq!(get_keys(&world, fragmenting_archetype), 1);

        world
            .entity_mut(e1)
            .insert(Fragmenting(1))
            .insert(Fragmenting(3));
        assert_eq!(get_keys(&world, fragmenting_archetype), 2);
    }
}
