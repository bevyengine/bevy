//! This module defines the concept of "fragmenting value" - a type that can be used to fragment
//! archetypes based on it's value in addition to it's type. The main trait is [`FragmentingValue`],
//! which is used to give each value that implements it a value-based identity, which is used by
//! other ecs functions to fragment archetypes.

use alloc::boxed::Box;
use alloc::sync::Arc;
use alloc::vec::Vec;
use bevy_ecs::component::Component;
use bevy_platform::hash::FixedHasher;
use bevy_ptr::{OwningPtr, Ptr};
use core::{
    any::TypeId,
    hash::{BuildHasher, Hash, Hasher},
    ops::Deref,
    ptr::NonNull,
};
use indexmap::Equivalent;

use crate::{
    bundle::Bundle,
    component::{ComponentId, Components, Immutable},
    query::DebugCheckedUnwrap,
    storage::FragmentingValueComponentsStorage,
};

pub trait FragmentingValueComponent: Component<Mutability = Immutable> + Eq + Hash + Clone {
    #[inline]
    fn hash_data(&self) -> u64 {
        let mut hasher = FixedHasher.build_hasher();
        self.hash(&mut hasher);
        hasher.finish()
    }
}

impl<C> FragmentingValueComponent for C where
    C: Component<Mutability = Immutable> + Eq + Hash + Clone
{
}

#[derive(Component, PartialEq, Eq, Hash, Clone)]
#[component(immutable)]
pub enum NoKey {}

#[derive(Clone)]
pub struct FragmentingValue {
    inner: Arc<FragmentingValueInner>,
}

impl FragmentingValue {
    #[inline]
    pub fn component_id(&self) -> ComponentId {
        self.inner.component_id
    }

    #[inline]
    pub fn component_data(&self) -> Ptr<'_> {
        unsafe { Ptr::new(self.inner.data) }
    }
}

impl Hash for FragmentingValue {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.component_id().hash(state);
        state.write_u64(self.inner.data_hash);
    }
}

impl PartialEq for FragmentingValue {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.inner, &other.inner)
    }
}

struct FragmentingValueInner {
    component_id: ComponentId,
    data_hash: u64,
    data_eq: for<'a> unsafe fn(Ptr<'a>, Ptr<'a>) -> bool,
    data_drop: Option<for<'a> unsafe fn(OwningPtr<'a>)>,
    data: NonNull<u8>,
}

unsafe impl Sync for FragmentingValueInner {}

unsafe impl Send for FragmentingValueInner {}

impl Drop for FragmentingValueInner {
    fn drop(&mut self) {
        if let Some(drop) = self.data_drop {
            unsafe { drop(OwningPtr::new(self.data)) }
        }
    }
}

impl Eq for FragmentingValue {}

/// A collection of fragmenting component values and ids.
/// This collection is sorted internally to allow for order-independent comparison.
///
/// Owned version can be used as a key in maps. [`FragmentingValuesBorrowed`] is a version that doesn't require cloning the values.
#[derive(Hash, PartialEq, Eq, Default, Clone)]
pub struct FragmentingValues {
    values: Box<[FragmentingValue]>,
}

impl Deref for FragmentingValues {
    type Target = [FragmentingValue];

    fn deref(&self) -> &Self::Target {
        &self.values
    }
}

impl FromIterator<FragmentingValue> for FragmentingValues {
    fn from_iter<T: IntoIterator<Item = FragmentingValue>>(iter: T) -> Self {
        let mut values: Box<_> = iter.into_iter().collect();
        values.sort_unstable_by_key(FragmentingValue::component_id);
        Self { values }
    }
}

impl FragmentingValues {
    pub fn from_sorted<T: IntoIterator<Item = FragmentingValue>>(iter: T) -> Self {
        Self {
            values: iter.into_iter().collect(),
        }
    }
}

pub struct FragmentingValueBorrowed<'a> {
    component_id: ComponentId,
    data_hash: u64,
    data: Ptr<'a>,
}

impl<'a> FragmentingValueBorrowed<'a> {
    pub unsafe fn new(
        components: &Components,
        component_id: ComponentId,
        component_data: Ptr<'a>,
    ) -> Option<Self> {
        components
            .get_info(component_id)
            .and_then(|info| info.value_component_vtable())
            .map(|vtable| {
                let data_hash = unsafe { (vtable.hash)(component_data) };
                Self {
                    component_id,
                    data_hash,
                    data: component_data,
                }
            })
    }

    pub fn from_component<C: FragmentingValueComponent>(
        components: &Components,
        component: &'a C,
    ) -> Option<Self> {
        components
            .get_id(TypeId::of::<C>())
            .map(|component_id| Self {
                component_id,
                data_hash: component.hash_data(),
                data: Ptr::from(component),
            })
    }

    #[inline]
    pub fn component_id(&self) -> ComponentId {
        self.component_id
    }

    #[inline]
    pub fn component_data(&self) -> Ptr<'a> {
        self.data
    }

    pub unsafe fn to_owned(
        &self,
        components: &Components,
        storage: &mut FragmentingValueComponentsStorage,
    ) -> FragmentingValue {
        let key = unsafe { self.as_equivalent() };
        storage
            .existing_values
            .get_or_insert_with(&key, |_| {
                let info = unsafe { components.get_info_unchecked(self.component_id()) };
                let vtable = unsafe { info.value_component_vtable().debug_checked_unwrap() };
                let layout = info.layout();
                let data = if layout.size() == 0 {
                    NonNull::dangling()
                } else {
                    unsafe { NonNull::new(alloc::alloc::alloc(info.layout())).unwrap() }
                };
                (vtable.clone)(self.data, data);
                FragmentingValue {
                    inner: Arc::new(FragmentingValueInner {
                        component_id: self.component_id,
                        data_hash: self.data_hash,
                        data,
                        data_drop: info.drop(),
                        data_eq: vtable.eq,
                    }),
                }
            })
            .clone()
    }

    #[inline]
    pub unsafe fn as_equivalent(&self) -> impl AsEquivalent<FragmentingValue> {
        #[derive(Hash)]
        pub struct FragmentingValueBorrowedKey<'a>(&'a FragmentingValueBorrowed<'a>);

        impl<'a> Equivalent<FragmentingValue> for FragmentingValueBorrowedKey<'a> {
            #[inline]
            fn equivalent(&self, key: &FragmentingValue) -> bool {
                self.0.component_id() == key.component_id()
                    && unsafe { (key.inner.data_eq)(self.0.data, key.component_data()) }
            }
        }

        FragmentingValueBorrowedKey(self)
    }
}

impl<'a> Hash for FragmentingValueBorrowed<'a> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.component_id.hash(state);
        state.write_u64(self.data_hash);
    }
}

#[derive(Hash)]
pub struct FragmentingValuesBorrowed<'a> {
    values: Vec<FragmentingValueBorrowed<'a>>,
}

impl<'a> Deref for FragmentingValuesBorrowed<'a> {
    type Target = [FragmentingValueBorrowed<'a>];

    fn deref(&self) -> &Self::Target {
        self.values.as_slice()
    }
}

impl<'a> FragmentingValuesBorrowed<'a> {
    pub unsafe fn from_bundle<B: Bundle>(components: &Components, bundle: &'a B) -> Self {
        let mut values = Vec::with_capacity(B::count_fragmenting_values());
        bundle.get_fragmenting_values(components, &mut |value| {
            values.push(value.debug_checked_unwrap());
        });
        values.sort_unstable_by_key(FragmentingValueBorrowed::component_id);
        FragmentingValuesBorrowed { values }
    }

    pub unsafe fn from_components(
        components: &Components,
        iter: impl IntoIterator<Item = (ComponentId, Ptr<'a>)>,
    ) -> Self {
        let mut values: Vec<_> = iter
            .into_iter()
            .filter_map(|(id, data)| {
                let info = components.get_info_unchecked(id);
                info.value_component_vtable()
                    .map(|vtable| FragmentingValueBorrowed {
                        component_id: id,
                        data,
                        data_hash: (vtable.hash)(data),
                    })
            })
            .collect();
        values.sort_unstable_by_key(FragmentingValueBorrowed::component_id);
        FragmentingValuesBorrowed { values }
    }

    pub unsafe fn to_owned(
        &self,
        components: &Components,
        storage: &mut FragmentingValueComponentsStorage,
    ) -> FragmentingValues {
        let values = self
            .values
            .iter()
            .map(|v| unsafe { v.to_owned(components, storage) })
            .collect();
        FragmentingValues { values }
    }

    #[inline]
    pub unsafe fn as_equivalent(&self) -> impl AsEquivalent<FragmentingValues> {
        #[derive(Hash)]
        pub struct FragmentingValuesBorrowedKey<'a>(&'a FragmentingValuesBorrowed<'a>);

        impl<'a> Equivalent<FragmentingValues> for FragmentingValuesBorrowedKey<'a> {
            #[inline]
            fn equivalent(&self, key: &FragmentingValues) -> bool {
                {
                    self.0.values.len() == key.values.len()
                        && self
                            .0
                            .values
                            .iter()
                            .zip(key.values.iter())
                            .all(|(v1, v2)| unsafe { v1.as_equivalent().equivalent(v2) })
                }
            }
        }

        FragmentingValuesBorrowedKey(self)
    }
}

pub trait AsEquivalent<T: ?Sized>: Hash + Equivalent<T> {}

impl<Q, K> AsEquivalent<K> for Q
where
    Q: Hash + Equivalent<K>,
    K: ?Sized,
{
}

#[derive(Hash, Eq, PartialEq)]
pub(crate) struct TupleKey<K1, K2>(pub K1, pub K2);

impl<Q1, Q2, K1, K2> Equivalent<TupleKey<K1, K2>> for (Q1, Q2)
where
    Q1: Equivalent<K1>,
    Q2: Equivalent<K2>,
{
    #[inline]
    fn equivalent(&self, key: &TupleKey<K1, K2>) -> bool {
        self.0.equivalent(&key.0) && self.1.equivalent(&key.1)
    }
}

/// Dynamic vtable for [`FragmentingValue`].
/// This is used by [`crate::component::ComponentDescriptor`] to work with dynamic fragmenting components.
#[derive(Clone, Copy, Debug)]
pub struct FragmentingValueVtable {
    eq: for<'a> unsafe fn(Ptr<'a>, Ptr<'a>) -> bool,
    hash: for<'a> unsafe fn(Ptr<'a>) -> u64,
    clone: for<'a> unsafe fn(Ptr<'a>, NonNull<u8>),
}

impl FragmentingValueVtable {
    /// Create a new vtable from raw functions.
    ///
    /// Also see [`from_fragmenting_value`](FragmentingValueVtable::from_fragmenting_value) and
    /// [`from_component`](FragmentingValueVtable::from_component) for more convenient constructors.
    #[inline]
    pub fn new(
        eq: unsafe fn(Ptr<'_>, Ptr<'_>) -> bool,
        hash: unsafe fn(Ptr<'_>) -> u64,
        clone: unsafe fn(Ptr<'_>, NonNull<u8>),
    ) -> Self {
        Self { eq, hash, clone }
    }

    /// Creates [`FragmentingValueVtable`] from a [`Component`].
    ///
    /// This will return `None` if the component isn't fragmenting.
    #[inline]
    pub fn from_component<T: Component>() -> Option<Self> {
        if TypeId::of::<T::Key>() != TypeId::of::<T>() {
            return None;
        }
        Some(FragmentingValueVtable {
            eq: |this, other| unsafe { this.deref::<T::Key>() == other.deref::<T::Key>() },
            hash: |this| unsafe { this.deref::<T::Key>().hash_data() },
            clone: |this, target| unsafe {
                target
                    .cast::<T::Key>()
                    .write(this.deref::<T::Key>().clone());
            },
        })
    }
}

#[cfg(test)]
mod tests {
    use core::{alloc::Layout, hash::BuildHasher, ptr::NonNull};

    use crate::{
        archetype::ArchetypeId,
        component::{
            Component, ComponentCloneBehavior, ComponentDescriptor, FragmentingValueVtable,
            StorageType,
        },
        entity::Entity,
        world::World,
    };
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

    #[derive(Component, Clone, Eq, PartialEq, Hash)]
    #[component(
            key=Self,
            immutable,
        )]
    struct FragmentingN<const N: usize>(u32);

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
    fn fragment_on_spawn_order_does_not_matter() {
        let mut world = World::default();
        let e1 = world
            .spawn((NonFragmenting, FragmentingN::<1>(1), FragmentingN::<2>(1)))
            .id();
        let e2 = world
            .spawn((NonFragmenting, FragmentingN::<2>(1), FragmentingN::<1>(1)))
            .id();

        let [id1, id2] = world.entity([e1, e2]).map(|e| e.archetype().id());
        assert_eq!(id1, id2);
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
        #[derive(Clone, PartialEq, Eq, Hash)]
        struct DynamicComponent {
            data: [u8; COMPONENT_SIZE],
        }

        unsafe fn eq(this: Ptr<'_>, other: Ptr<'_>) -> bool {
            this.deref::<DynamicComponent>() == other.deref::<DynamicComponent>()
        }

        unsafe fn hash(this: Ptr<'_>) -> u64 {
            FixedHasher.hash_one(this.deref::<DynamicComponent>())
        }

        unsafe fn clone(this: Ptr<'_>, target: NonNull<u8>) {
            target
                .cast::<DynamicComponent>()
                .write(this.deref::<DynamicComponent>().clone())
        }

        let layout = Layout::new::<DynamicComponent>();
        let vtable = FragmentingValueVtable::new(eq, hash, clone);

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

        let component1_1 = DynamicComponent { data: [5; 10] };
        let component1_2 = DynamicComponent { data: [8; 10] };
        let component2_1 = DynamicComponent { data: [5; 10] };

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
    fn fragmenting_value_edges_cache_does_not_reset() {
        let mut world = World::default();

        let bundle_id = world.register_bundle::<Fragmenting>().id();
        let get_keys = |world: &World, archetype_id: ArchetypeId| {
            world.archetypes[archetype_id]
                .edges()
                .insert_bundle_fragmenting_components
                .keys()
                .filter(|k| k.0 == bundle_id)
                .count()
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
