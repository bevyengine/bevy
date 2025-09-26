//! This module defines the concept of "fragmenting value" - a type that can be used to fragment
//! archetypes based on it's value in addition to it's type. The main trait is [`FragmentingValue`],
//! which is used to give each value that implements it a value-based identity, which is used by
//! other ecs functions to fragment archetypes.

use alloc::boxed::Box;
use alloc::vec::Vec;
use bevy_ecs::component::Component;
use bevy_platform::{hash::FixedHasher, sync::Arc};
use bevy_ptr::{OwningPtr, Ptr};
use core::{
    any::{Any, TypeId},
    hash::{BuildHasher, Hash, Hasher},
    ops::Deref,
    ptr::NonNull,
};
use indexmap::Equivalent;

use crate::{
    bundle::Bundle,
    component::{ComponentId, Components, Immutable},
    query::DebugCheckedUnwrap,
    storage::FragmentingValuesStorage,
};

/// Trait defining a [`Component`] that fragments archetypes by it's value.
pub trait FragmentingValueComponent:
    Component<Mutability = Immutable> + Eq + Hash + Clone + private::Seal
{
    /// Returns hash of this [`Component`] as a `u64`.
    #[inline]
    fn hash_data(&self) -> u64 {
        FixedHasher.hash_one(self)
    }
}

impl<C> FragmentingValueComponent for C where
    C: Component<Mutability = Immutable> + Eq + Hash + Clone
{
}

impl<C> private::Seal for C where C: Component<Mutability = Immutable> + Eq + Hash + Clone {}

mod private {
    pub trait Seal {}
}

/// A [`FragmentingValueComponent`] that is used to mark components as **non**-fragmenting by value.
/// See [`Component::Key`] for more detail.
#[derive(Component, PartialEq, Eq, Hash, Clone)]
#[component(immutable)]
pub enum NoKey {}

/// A type-erased version of [`FragmentingValueComponent`].
///
/// Each combination of component type + value is unique and is stored exactly once in [`FragmentingValuesStorage`].
#[derive(Clone)]
pub struct FragmentingValue {
    inner: Arc<FragmentingValueInner>,
}

impl FragmentingValue {
    #[inline]
    /// Returns [`ComponentId`] of this fragmenting value component.
    pub fn component_id(&self) -> ComponentId {
        self.inner.component_id
    }

    #[inline]
    /// Returns pointer to data of this fragmenting value component.
    pub fn component_data(&self) -> Ptr<'_> {
        // Safety: data points to properly-aligned, valid value of the type this component id is registered for.
        unsafe { Ptr::new(self.inner.data) }
    }
}

impl Hash for FragmentingValue {
    fn hash<H: Hasher>(&self, state: &mut H) {
        // This must be implemented exactly the same way as Hash for FragmentingValueBorrowed!
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

// Safety: data points to a type that is Sync, all other field are Sync
unsafe impl Sync for FragmentingValueInner {}

// Safety: data points to a type that is Send, all other field are Send
unsafe impl Send for FragmentingValueInner {}

impl Drop for FragmentingValueInner {
    fn drop(&mut self) {
        if let Some(drop) = self.data_drop {
            // Safety:
            // - `data` points to properly-aligned, valid value of the type this component id is registered for.
            // - `drop` is valid to call for data with the type of of this component.
            unsafe { drop(OwningPtr::new(self.data)) }
        }
    }
}

impl Eq for FragmentingValue {}

/// A collection of [`FragmentingValue`].
///
/// This collection is sorted internally to allow for order-independent comparison and can be used as a key in maps.
/// Can be created from [`FragmentingValuesBorrowed`].
#[derive(Hash, PartialEq, Eq, Default, Clone)]
pub(crate) struct FragmentingValues {
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
    pub(crate) fn from_sorted<T: IntoIterator<Item = FragmentingValue>>(iter: T) -> Self {
        Self {
            values: iter.into_iter().collect(),
        }
    }
}

/// Type representing a [`FragmentingValueComponent`] borrowed from a bundle.
pub struct FragmentingValueBorrowed<'a> {
    component_id: ComponentId,
    data_hash: u64,
    data: Ptr<'a>,
}

impl<'a> FragmentingValueBorrowed<'a> {
    /// Create a new [`FragmentingValueBorrowed`] from raw component data.
    ///
    /// This will return `None` if:
    /// - Component isn't registered in the `components`.
    /// - Component isn't a [`FragmentingValueComponent`].
    ///
    /// # Safety
    /// Data behind `component_data` pointer must match the component registered with this `component_id` in `components`.
    #[inline]
    pub unsafe fn new(
        components: &Components,
        component_id: ComponentId,
        component_data: Ptr<'a>,
    ) -> Option<Self> {
        components
            .get_info(component_id)
            .and_then(|info| info.value_component_vtable())
            .map(|vtable| {
                // Safety: component_data is a valid data of type represented by this ComponentId.
                let data_hash = unsafe { (vtable.hash)(component_data) };
                Self {
                    component_id,
                    data_hash,
                    data: component_data,
                }
            })
    }

    /// Create a new [`FragmentingValueBorrowed`] from a [`Component`].
    ///
    /// This will return `None` if:
    /// - `C` isn't registered in `components`.
    /// - `C` isn't a [`FragmentingValueComponent`].
    #[inline]
    pub fn from_component<C: Component>(components: &Components, component: &'a C) -> Option<Self> {
        (component as &dyn Any)
            .downcast_ref::<C::Key>()
            .and_then(|component| {
                components
                    .get_id(TypeId::of::<C>())
                    .map(|component_id| Self {
                        component_id,
                        data_hash: component.hash_data(),
                        data: Ptr::from(component),
                    })
            })
    }

    /// Return [`ComponentId`] of this borrowed [`FragmentingValueComponent`].
    #[inline]
    pub fn component_id(&self) -> ComponentId {
        self.component_id
    }

    /// Return pointer to data of this borrowed [`FragmentingValueComponent`].
    #[inline]
    pub fn component_data(&self) -> Ptr<'a> {
        self.data
    }

    /// Create [`FragmentingValue`] by cloning data pointed to by this [`FragmentingValueBorrowed`] or getting existing one from [`FragmentingValuesStorage`].
    ///
    /// # Safety
    /// - `components` must be the same one as the one used to create `self`.
    /// - `storage` and `components` must be from the same world.
    pub(crate) unsafe fn to_owned(
        &self,
        components: &Components,
        storage: &mut FragmentingValuesStorage,
    ) -> FragmentingValue {
        // Safety: `self` was created using same `Components` as the ones we will be comparing with
        // since `components` and `storage` are from the same world.
        let key = unsafe { self.as_equivalent() };
        storage
            .existing_values
            .get_or_insert_with(&key, |_| {
                // Safety: id is a valid and registered component.
                let info = unsafe { components.get_info_unchecked(self.component_id()) };
                // Safety: component is fragmenting since `FragmentingValueBorrowed` can only be created for valid fragmenting components.
                let vtable = unsafe { info.value_component_vtable().debug_checked_unwrap() };
                let layout = info.layout();
                let data = if layout.size() == 0 {
                    NonNull::dangling()
                } else {
                    // Safety: layout.size() != 0
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

    /// Get a value that can be used to compare with and query maps containing [`FragmentingValue`]s as the key.
    ///
    /// # Safety
    /// Caller must ensure that [`FragmentingValue`] this will compare with was created using the same [`Components`] as `self`.
    #[inline]
    pub(crate) unsafe fn as_equivalent(&self) -> impl AsEquivalent<FragmentingValue> {
        #[derive(Hash)]
        pub struct FragmentingValueBorrowedKey<'a>(&'a FragmentingValueBorrowed<'a>);

        impl<'a> Equivalent<FragmentingValue> for FragmentingValueBorrowedKey<'a> {
            #[inline]
            fn equivalent(&self, key: &FragmentingValue) -> bool {
                self.0.component_id() == key.component_id()
                    // Safety: `self` and `key` point to the same component type since `self.component_id` and `key.component_id` are equal
                    // and were created using the same `Components` instance.
                    && unsafe { (key.inner.data_eq)(self.0.data, key.component_data()) }
            }
        }

        FragmentingValueBorrowedKey(self)
    }
}

impl<'a> Hash for FragmentingValueBorrowed<'a> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        // This must be implemented exactly the same way as Hash for FragmentingValue!
        self.component_id.hash(state);
        state.write_u64(self.data_hash);
    }
}

/// A collection of [`FragmentingValueBorrowed`].
///
/// This collection is sorted internally to allow for order-independent comparison.
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
    /// Create a new [`FragmentingValuesBorrowed`] from a [`Bundle`].
    ///
    /// NOTE: If any of the components in the bundle weren't registered, this might return incorrect result!
    pub fn from_bundle<B: Bundle>(components: &Components, bundle: &'a B) -> Self {
        let mut values = Vec::with_capacity(B::count_fragmenting_values());
        bundle.get_fragmenting_values(components, &mut |value| {
            values.push(value);
        });
        values.sort_unstable_by_key(FragmentingValueBorrowed::component_id);
        FragmentingValuesBorrowed { values }
    }

    /// Create a new [`FragmentingValuesBorrowed`] from a raw component data + their [`ComponentId`].
    ///
    /// NOTE: If any of the components in the bundle weren't registered, this might return incorrect result!
    ///
    /// # Safety
    /// Pointer to data for the corresponding [`ComponentId`] must point to a valid component data represented by the id.
    pub unsafe fn from_components(
        components: &Components,
        iter: impl IntoIterator<Item = (ComponentId, Ptr<'a>)>,
    ) -> Self {
        let mut values: Vec<_> = iter
            .into_iter()
            .filter_map(|(id, data)| FragmentingValueBorrowed::new(components, id, data))
            .collect();
        values.sort_unstable_by_key(FragmentingValueBorrowed::component_id);
        FragmentingValuesBorrowed { values }
    }

    /// Create [`FragmentingValues`] by cloning data pointed to by this [`FragmentingValuesBorrowed`] or getting existing ones from [`FragmentingValuesStorage`].
    ///
    /// # Safety
    /// `components` must be the same one as the one used to create `self`.
    pub(crate) unsafe fn to_owned(
        &self,
        components: &Components,
        storage: &mut FragmentingValuesStorage,
    ) -> FragmentingValues {
        let values = self
            .values
            .iter()
            // Safety: v was created using `components`
            .map(|v| unsafe { v.to_owned(components, storage) })
            .collect();
        FragmentingValues { values }
    }

    /// Get a value that can be used to compare with and query maps containing [`FragmentingValues`]s as the key.
    ///
    /// # Safety
    /// Caller must ensure that [`FragmentingValues`] this will compare with was created using the same [`Components`] as `self`.
    #[inline]
    pub(crate) unsafe fn as_equivalent(&self) -> impl AsEquivalent<FragmentingValues> {
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
                            // Safety: v1 was created using the same Component as v2
                            .all(|(v1, v2)| unsafe { v1.as_equivalent().equivalent(v2) })
                }
            }
        }

        FragmentingValuesBorrowedKey(self)
    }
}

/// [`Hash`] + [`Equivalent`] supertrait for hashmap queries.
pub trait AsEquivalent<T: ?Sized>: Hash + Equivalent<T> {}

impl<Q, K> AsEquivalent<K> for Q
where
    Q: Hash + Equivalent<K>,
    K: ?Sized,
{
}

/// Workaround to allow querying hashmaps using tuples of [`Equivalent`] types.
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

/// Dynamic vtable for [`FragmentingValueComponent`].
/// This stores the functions required to compare, hash and store fragmenting values on [`crate::component::ComponentDescriptor`].
#[derive(Clone, Copy, Debug)]
pub struct FragmentingValueVtable {
    // Safety: This functions must be safe to call with pointers to the data of the component's type this vtable registered for.
    eq: for<'a> unsafe fn(Ptr<'a>, Ptr<'a>) -> bool,
    // Safety: This functions must be safe to call with pointer to the data of the component's type this vtable registered for.
    hash: for<'a> unsafe fn(Ptr<'a>) -> u64,
    // Safety: This functions must be safe to call with pointers to the data of the component's type this vtable registered for.
    clone: for<'a> unsafe fn(Ptr<'a>, NonNull<u8>),
}

impl FragmentingValueVtable {
    /// Create a new vtable from raw functions.
    ///
    /// All pointers passed to the functions will point to the data of the component for which this vtable will be registered for.
    /// - `eq`: `(self: *const C, other: *const C)`
    /// - `hash`: `(self: *const C)`
    /// - `clone`: `(self: *const C, target: *mut C)`
    ///
    /// # Safety
    /// `clone` must initialize data behind `target` pointer to valid value of the same type as the one this vtable is registered for on call.
    #[inline]
    pub unsafe fn new(
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
            // Safety: `this` and `other` are of type T, and T::Key == T
            eq: |this, other| unsafe { this.deref::<T::Key>() == other.deref::<T::Key>() },
            // Safety: `this` is of type T and T::Key == T
            hash: |this| unsafe { this.deref::<T::Key>().hash_data() },
            // Safety: `this` and `target` are of type T, and T::Key == T
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
                .write(this.deref::<DynamicComponent>().clone());
        }

        let layout = Layout::new::<DynamicComponent>();
        // Safety: `clone` properly initializes value of type `DynamicComponent`
        let vtable = unsafe { FragmentingValueVtable::new(eq, hash, clone) };

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
