//! This module defines the concept of "fragmenting value" - a type that can be used to fragment
//! archetypes based on it's value in addition to it's type. The main trait is [`FragmentingValue`],
//! which is used to give each value that implements it a value-based identity, which is used by
//! other ecs functions to fragment archetypes.

use alloc::boxed::Box;
use alloc::vec::Vec;
use bevy_ecs::component::Component;
use bevy_platform::hash::FixedHasher;
use bevy_ptr::{OwningPtr, Ptr};
use core::{
    any::{Any, TypeId},
    borrow::Borrow,
    hash::{BuildHasher, Hash, Hasher},
    mem::MaybeUninit,
    ptr::NonNull,
};
use indexmap::Equivalent;

use crate::{
    bundle::Bundle,
    component::{ComponentId, ComponentInfo, ComponentKey, ComponentsRegistrator, Immutable},
    query::DebugCheckedUnwrap,
};

/// Trait used to define values that can fragment archetypes.
///
/// This trait is automatically implemented for all immutable components that also implement [`Eq`], [`Hash`] and [`Clone`].
///
/// For dynamic components see [`DynamicFragmentingValue`] and [`FragmentingValueVtable`].
pub trait FragmentingValue: Any {
    /// Return `true` if `self == other`. Dynamic version of [`PartialEq::eq`].
    fn dyn_eq(&self, other: &dyn FragmentingValue) -> bool;
    /// Returns the hash value of `self`. Dynamic version of [`Hash::hash`].
    fn dyn_hash(&self) -> u64;
    /// Returns a boxed clone of `self`. Dynamic version of [`Clone::clone`].
    fn dyn_clone(&self) -> Box<dyn FragmentingValue>;
}

pub trait FragmentingValueComponent: Component<Mutability = Immutable> + Eq + Hash + Clone {}

impl<C> FragmentingValueComponent for C where
    C: Component<Mutability = Immutable> + Eq + Hash + Clone
{
}

impl<T> FragmentingValue for T
where
    T: FragmentingValueComponent,
{
    #[inline]
    fn dyn_eq(&self, other: &dyn FragmentingValue) -> bool {
        #[expect(clippy::let_unit_value, reason = "This is used for static asserts")]
        {
            _ = T::Key::INVARIANT_ASSERT;
        }
        let other_as_any = other as &dyn Any;
        if let Some(other) = other_as_any.downcast_ref::<T>() {
            self == other
        } else if let Some(other) = other_as_any.downcast_ref::<DynamicFragmentingValueInner>()
            && let Some(other_type_id) = other.get_component_info().type_id()
            && other_type_id == TypeId::of::<Self>()
        {
            unsafe { other.get_data().deref::<Self>() == self }
        } else {
            false
        }
    }

    #[inline]
    fn dyn_hash(&self) -> u64 {
        FixedHasher.hash_one(self)
    }

    #[inline]
    fn dyn_clone(&self) -> Box<dyn FragmentingValue> {
        Box::new(self.clone())
    }
}

#[derive(Component, PartialEq, Eq, Hash, Clone)]
#[component(immutable)]
pub enum Keyless {}

impl PartialEq for dyn FragmentingValue {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.dyn_eq(other)
    }
}

impl Eq for dyn FragmentingValue {}

impl Hash for dyn FragmentingValue {
    #[inline]
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write_u64(self.dyn_hash());
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
            values.push((id, value.borrow().dyn_clone()));
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
                .map(|(id, v)| (*id, v.dyn_clone()))
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
                .all(|((id1, v1), (id2, v2))| id1 == id2 && v1.dyn_eq(&**v2))
    }
}

struct DynamicFragmentingValueInner {
    component_info: NonNull<ComponentInfo>,
    component_data: NonNull<u8>,
}

impl DynamicFragmentingValueInner {
    #[inline]
    fn get_data(&self) -> Ptr<'_> {
        unsafe { Ptr::new(self.component_data) }
    }

    #[inline]
    fn get_component_info(&self) -> &ComponentInfo {
        unsafe { self.component_info.as_ref() }
    }

    #[inline]
    fn get_vtable(&self) -> &FragmentingValueVtable {
        unsafe {
            self.get_component_info()
                .value_component_vtable()
                .debug_checked_unwrap()
        }
    }
}

impl FragmentingValue for DynamicFragmentingValueInner {
    #[inline]
    fn dyn_eq(&self, other: &dyn FragmentingValue) -> bool {
        let other_as_any = other as &dyn Any;
        let self_info = self.get_component_info();
        if let Some(other) = other_as_any.downcast_ref::<Self>() {
            let other_info = other.get_component_info();
            self_info.id() == other_info.id()
                && (self.get_vtable().eq)(self_info, self.get_data(), other.get_data())
        } else if let Some(type_id) = self_info.type_id()
            && type_id == other_as_any.type_id()
        {
            let other_ptr = unsafe {
                Ptr::new(NonNull::new_unchecked(
                    core::ptr::from_ref(other).cast::<u8>().cast_mut(),
                ))
            };
            (self.get_vtable().eq)(self_info, self.get_data(), other_ptr)
        } else {
            false
        }
    }

    #[inline]
    fn dyn_hash(&self) -> u64 {
        (self.get_vtable().hash)(self.get_component_info(), self.get_data())
    }

    #[inline]
    fn dyn_clone(&self) -> Box<dyn FragmentingValue> {
        let info = self.get_component_info();
        let layout = info.layout();
        let component_data = if layout.size() != 0 {
            unsafe { NonNull::new(alloc::alloc::alloc(layout)).debug_checked_unwrap() }
        } else {
            NonNull::dangling()
        };
        let component_info = unsafe {
            NonNull::new_unchecked(Box::into_raw(Box::new(self.get_component_info().clone())))
        };

        (self.get_vtable().clone)(info, self.get_data(), component_data);

        Box::new(Self {
            component_data,
            component_info,
        })
    }
}

impl Drop for DynamicFragmentingValueInner {
    fn drop(&mut self) {
        if let Some(drop) = self.get_component_info().drop() {
            unsafe { drop(OwningPtr::new(self.component_data)) }
        }
        unsafe {
            Box::from_raw(self.component_info.as_ptr());
        }
    }
}

/// Holder for a reference to a dynamic fragmenting component and a vtable.
pub struct DynamicFragmentingValue(MaybeUninit<DynamicFragmentingValueInner>);

impl DynamicFragmentingValue {
    pub fn new() -> Self {
        Self(MaybeUninit::uninit())
    }

    /// Create a new `&dyn` [`FragmentingValue`] from passed component data and id.
    /// This is used mostly to construct [`FragmentingValuesBorrowed`] to compare dynamic components without copying data.
    ///
    /// Will return `None` if component isn't fragmenting.
    ///
    /// # Safety
    /// - `component_id` must match data which `component` points to.
    pub unsafe fn from_component<'a>(
        &'a mut self,
        component_info: &'a ComponentInfo,
        component_data: Ptr<'a>,
    ) -> Option<&'a dyn FragmentingValue> {
        if component_info.mutable() {
            return None;
        }
        let inner = DynamicFragmentingValueInner {
            component_info: component_info.into(),
            component_data: component_data.into(),
        };
        Some(self.0.write(inner))
    }
}

/// Dynamic vtable for [`FragmentingValue`].
/// This is used by [`crate::component::ComponentDescriptor`] to work with dynamic fragmenting components.
#[derive(Clone, Copy, Debug)]
pub struct FragmentingValueVtable {
    eq: for<'a> fn(&'a ComponentInfo, Ptr<'a>, Ptr<'a>) -> bool,
    hash: for<'a> fn(&'a ComponentInfo, Ptr<'a>) -> u64,
    clone: for<'a> fn(&'a ComponentInfo, Ptr<'a>, NonNull<u8>),
}

impl FragmentingValueVtable {
    /// Create a new vtable from raw functions.
    ///
    /// Also see [`from_fragmenting_value`](FragmentingValueVtable::from_fragmenting_value) and
    /// [`from_component`](FragmentingValueVtable::from_component) for more convenient constructors.
    pub fn new(
        eq: fn(&ComponentInfo, Ptr<'_>, Ptr<'_>) -> bool,
        hash: fn(&ComponentInfo, Ptr<'_>) -> u64,
        clone: fn(&ComponentInfo, Ptr<'_>, NonNull<u8>),
    ) -> Self {
        Self { eq, hash, clone }
    }

    /// Creates [`FragmentingValueVtable`] from a [`Component`].
    ///
    /// This will return `None` if the component isn't fragmenting.
    pub fn from_component<T: Component>() -> Option<Self> {
        type KeyOf<T: Component> = <T::Key as ComponentKey>::KeyType;

        if TypeId::of::<KeyOf<T>>() == TypeId::of::<T>() {
            Some(FragmentingValueVtable {
                eq: |_, this, other| unsafe {
                    this.deref::<KeyOf<T>>() == other.deref::<KeyOf<T>>()
                },
                hash: |_, this| unsafe { FixedHasher.hash_one(this.deref::<KeyOf<T>>()) },
                clone: |_, this, target| unsafe {
                    target
                        .cast::<KeyOf<T>>()
                        .write(this.deref::<KeyOf<T>>().clone());
                },
            })
        } else {
            None
        }
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
        fragmenting_value::{DynamicFragmentingValue, FragmentingValue, FragmentingValueVtable},
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

        fn eq(_: &ComponentInfo, this: Ptr<'_>, other: Ptr<'_>) -> bool {
            unsafe { this.deref::<DynamicComponent>() == other.deref::<DynamicComponent>() }
        }

        fn hash(_: &ComponentInfo, this: Ptr<'_>) -> u64 {
            unsafe { FixedHasher.hash_one(this.deref::<DynamicComponent>()) }
        }

        fn clone(_: &ComponentInfo, this: Ptr<'_>, target: NonNull<u8>) {
            unsafe {
                target
                    .cast::<DynamicComponent>()
                    .write(this.deref::<DynamicComponent>().clone())
            }
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
    fn fragmenting_value_compare_with_dynamic() {
        let mut world = World::default();

        let component_id = world.register_component::<Fragmenting>();
        let info = world.components().get_info(component_id).unwrap();
        let value: &dyn FragmentingValue = &Fragmenting(1);
        let mut dynamic_holder = DynamicFragmentingValue::new();
        let data = Fragmenting(1);
        let dynamic = unsafe { dynamic_holder.from_component(info, Ptr::from(&data)) }.unwrap();

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
