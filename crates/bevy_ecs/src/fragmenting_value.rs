//! This module defines the concept of "fragmenting value" - a type that can be used to fragment
//! archetypes based on it's value in addition to it's type. The main trait is [`FragmentingValue`],
//! which is used to give each value that implements it a value-based identity, which is used by
//! other ecs functions to fragment archetypes.

use alloc::boxed::Box;
use alloc::sync::Arc;
use alloc::vec::Vec;
use bevy_ecs::component::Component;
use bevy_platform::{collections::HashSet, hash::FixedHasher};
use bevy_ptr::{OwningPtr, Ptr};
use core::{
    any::{Any, TypeId},
    borrow::Borrow,
    hash::{BuildHasher, Hash, Hasher},
    mem::MaybeUninit,
    ops::Deref,
    ptr::NonNull,
};
use indexmap::Equivalent;

use crate::{
    bundle::Bundle,
    component::{
        CheckChangeTicks, ComponentId, ComponentInfo, ComponentKey, Components,
        ComponentsRegistrator, Immutable, KeyOf,
    },
    query::DebugCheckedUnwrap,
};

#[derive(Default)]
pub struct FragmentingValueComponentsStorage {
    existing_values: HashSet<FragmentingValueOwned>,
}

impl FragmentingValueComponentsStorage {
    pub fn check_change_ticks(&mut self, _check: CheckChangeTicks) {}
}

#[derive(Clone)]
pub struct FragmentingValueOwned {
    inner: Arc<DynamicFragmentingValueInner>,
}

impl Hash for FragmentingValueOwned {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.inner.hash(state);
    }
}

impl PartialEq for FragmentingValueOwned {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.inner, &other.inner)
    }
}

impl Eq for FragmentingValueOwned {}

impl FragmentingValueOwned {
    pub fn component_id(&self) -> ComponentId {
        self.inner.component_id
    }
}

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
pub enum Keyless {}

/// A collection of fragmenting component values and ids.
/// This collection is sorted internally to allow for order-independent comparison.
///
/// Owned version can be used as a key in maps. [`FragmentingValuesBorrowed`] is a version that doesn't require cloning the values.
#[derive(Hash, PartialEq, Eq, Default, Clone)]
pub struct FragmentingValuesOwned {
    values: Box<[FragmentingValueOwned]>,
}

impl Deref for FragmentingValuesOwned {
    type Target = [FragmentingValueOwned];

    fn deref(&self) -> &Self::Target {
        &*self.values
    }
}

impl FromIterator<FragmentingValueOwned> for FragmentingValuesOwned {
    fn from_iter<T: IntoIterator<Item = FragmentingValueOwned>>(iter: T) -> Self {
        Self {
            values: iter.into_iter().collect(),
        }
    }
}

impl FragmentingValuesOwned {
    /// Returns `true` if there are no fragmenting values.
    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }
}

pub struct FragmentingValueV2Borrowed<'a> {
    component_id: ComponentId,
    data_hash: u64,
    data: Ptr<'a>,
}

impl<'a> FragmentingValueV2Borrowed<'a> {
    pub unsafe fn new(component_id: ComponentId, data_hash: u64, data: Ptr<'a>) -> Self {
        Self {
            component_id,
            data_hash,
            data,
        }
    }

    pub fn component_id(&self) -> ComponentId {
        self.component_id
    }

    pub const unsafe fn as_key(&self) -> &FragmentingValueBorrowedKey {
        &*(self as *const FragmentingValueV2Borrowed as *const FragmentingValueBorrowedKey)
    }

    pub unsafe fn to_owned(
        &self,
        components: &Components,
        storage: &mut FragmentingValueComponentsStorage,
    ) -> FragmentingValueOwned {
        storage
            .existing_values
            .get_or_insert_with(unsafe { self.as_key() }, |v| {
                let info = unsafe { components.get_info_unchecked(v.component_id) };
                let vtable = unsafe { info.value_component_vtable().debug_checked_unwrap() };
                let layout = info.layout();
                let data = if layout.size() == 0 {
                    NonNull::dangling()
                } else {
                    unsafe { NonNull::new(alloc::alloc::alloc(info.layout())).unwrap() }
                };
                (vtable.clone)(v.data, data);
                FragmentingValueOwned {
                    inner: Arc::new(DynamicFragmentingValueInner {
                        component_id: v.component_id,
                        data_hash: v.data_hash,
                        data,
                        data_drop: info.drop(),
                        data_eq: vtable.eq,
                    }),
                }
            })
            .clone()
    }
}

impl<'a> Hash for FragmentingValueV2Borrowed<'a> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.component_id.hash(state);
        state.write_u64(self.data_hash);
    }
}

#[derive(Hash)]
pub struct FragmentingValuesBorrowed<'a> {
    values: Vec<FragmentingValueV2Borrowed<'a>>,
}

impl<'a> Deref for FragmentingValuesBorrowed<'a> {
    type Target = [FragmentingValueV2Borrowed<'a>];

    fn deref(&self) -> &Self::Target {
        &*self.values
    }
}

impl<'a> FragmentingValuesBorrowed<'a> {
    pub unsafe fn from_bundle<B: Bundle>(components: &Components, bundle: &'a B) -> Self {
        let mut values = Vec::with_capacity(B::count_fragmenting_values());
        bundle.get_fragmenting_values(components, &mut |value| {
            values.push(value.debug_checked_unwrap())
        });
        values.sort_unstable_by_key(|v| v.component_id);
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
                    .map(|vtable| FragmentingValueV2Borrowed {
                        component_id: id,
                        data,
                        data_hash: (vtable.hash)(data),
                    })
            })
            .collect();
        values.sort_unstable_by_key(|v| v.component_id);
        FragmentingValuesBorrowed { values }
    }

    pub fn empty() -> Self {
        FragmentingValuesBorrowed { values: Vec::new() }
    }

    /// Returns `true` if there are no fragmenting values.
    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }

    pub unsafe fn to_owned(
        &self,
        components: &Components,
        storage: &mut FragmentingValueComponentsStorage,
    ) -> FragmentingValuesOwned {
        let values = self
            .values
            .iter()
            .map(|v| unsafe { v.to_owned(components, storage) })
            .collect();
        FragmentingValuesOwned { values }
    }

    pub const unsafe fn as_key(&self) -> &FragmentingValuesBorrowedKey {
        &*(self as *const FragmentingValuesBorrowed as *const FragmentingValuesBorrowedKey)
    }
}

#[repr(transparent)]
#[derive(Hash)]
pub struct FragmentingValueBorrowedKey<'a>(FragmentingValueV2Borrowed<'a>);

impl<'a> Deref for FragmentingValueBorrowedKey<'a> {
    type Target = FragmentingValueV2Borrowed<'a>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'a> Equivalent<FragmentingValueOwned> for FragmentingValueBorrowedKey<'a> {
    fn equivalent(&self, key: &FragmentingValueOwned) -> bool {
        self == key
    }
}

impl<'a> PartialEq<FragmentingValueOwned> for FragmentingValueBorrowedKey<'a> {
    fn eq(&self, other: &FragmentingValueOwned) -> bool {
        self.component_id() == other.component_id()
            && unsafe { (other.inner.data_eq)(self.data, other.inner.get_data()) }
    }
}

#[repr(transparent)]
#[derive(Hash)]
pub struct FragmentingValuesBorrowedKey<'a>(FragmentingValuesBorrowed<'a>);

impl<'a> Deref for FragmentingValuesBorrowedKey<'a> {
    type Target = FragmentingValuesBorrowed<'a>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'a> Equivalent<FragmentingValuesOwned> for FragmentingValuesBorrowedKey<'a> {
    fn equivalent(&self, key: &FragmentingValuesOwned) -> bool {
        {
            self.values.len() == key.values.len()
                && self
                    .values
                    .iter()
                    .zip(key.values.iter())
                    .all(|(v1, v2)| unsafe { v1.as_key() == v2 })
        }
    }
}

#[derive(Hash)]
pub struct FragmentingValuesBorrowedTupleKey<'a, T>(
    pub T,
    pub &'a FragmentingValuesBorrowedKey<'a>,
);

impl<'a, T: Equivalent<T>> Equivalent<(T, FragmentingValuesOwned)>
    for FragmentingValuesBorrowedTupleKey<'a, T>
{
    fn equivalent(&self, key: &(T, FragmentingValuesOwned)) -> bool {
        self.0.equivalent(&key.0) && self.1.equivalent(&key.1)
    }
}

struct DynamicFragmentingValueInner {
    component_id: ComponentId,
    data_hash: u64,
    data_eq: for<'a> unsafe fn(Ptr<'a>, Ptr<'a>) -> bool,
    data_drop: Option<for<'a> unsafe fn(OwningPtr<'a>)>,
    data: NonNull<u8>,
}

unsafe impl Sync for DynamicFragmentingValueInner {}

unsafe impl Send for DynamicFragmentingValueInner {}

impl Hash for DynamicFragmentingValueInner {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.component_id.hash(state);
        state.write_u64(self.data_hash);
    }
}

impl DynamicFragmentingValueInner {
    #[inline]
    fn get_data(&self) -> Ptr<'_> {
        unsafe { Ptr::new(self.data) }
    }
}

impl Drop for DynamicFragmentingValueInner {
    fn drop(&mut self) {
        if let Some(drop) = self.data_drop {
            unsafe { drop(OwningPtr::new(self.data)) }
        }
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
    pub fn from_component<T: Component>() -> Option<Self> {
        if TypeId::of::<KeyOf<T>>() != TypeId::of::<T>() {
            return None;
        }
        Some(FragmentingValueVtable {
            eq: |this, other| unsafe { this.deref::<KeyOf<T>>() == other.deref::<KeyOf<T>>() },
            hash: |this| unsafe { this.deref::<KeyOf<T>>().hash_data() },
            clone: |this, target| unsafe {
                target
                    .cast::<KeyOf<T>>()
                    .write(this.deref::<KeyOf<T>>().clone());
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
            Component, ComponentCloneBehavior, ComponentDescriptor, ComponentInfo, StorageType,
        },
        entity::Entity,
        fragmenting_value::FragmentingValueVtable,
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
                .filter(|(k, ..)| *k == bundle_id)
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
