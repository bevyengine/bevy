use crate::{
    borrow::{Ref, RefIter, RefIterMut, RefMut},
    filter::{ComponentFilter, EntityFilterTuple, Passthrough},
    index::{ChunkIndex, SetIndex},
    query::{DefaultFilter, View, ViewElement},
    storage::{ArchetypeData, Component, ComponentStorage, ComponentTypeId},
};
use std::{
    any::TypeId,
    slice::{Iter, IterMut},
};

impl<'a, T: Component> DefaultFilter for RefMut<'static, T> {
    type Filter = EntityFilterTuple<ComponentFilter<T>, Passthrough, Passthrough>;

    fn filter() -> Self::Filter { super::filter::filter_fns::component() }
}

impl<'a, T: Component> View<'a> for RefMut<'static, T> {
    type Iter = RefIterMut<'a, T, IterMut<'a, T>>;

    #[inline]
    fn fetch(
        _: &'a ArchetypeData,
        chunk: &'a ComponentStorage,
        _: ChunkIndex,
        _: SetIndex,
    ) -> Self::Iter {
        let (slice_borrow, slice) = unsafe {
            chunk
                .components(ComponentTypeId::of::<T>())
                .unwrap_or_else(|| {
                    panic!(
                        "Component of type {:?} not found in chunk when fetching Write view",
                        std::any::type_name::<T>()
                    )
                })
                .data_slice_mut::<T>()
                .deconstruct()
        };
        RefIterMut::new(slice_borrow, slice.iter_mut())
    }

    #[inline]
    fn validate() -> bool { true }

    #[inline]
    fn reads<D: Component>() -> bool { TypeId::of::<T>() == TypeId::of::<D>() }

    #[inline]
    fn writes<D: Component>() -> bool { TypeId::of::<T>() == TypeId::of::<D>() }

    #[inline]
    fn read_types() -> Vec<ComponentTypeId> { vec![ComponentTypeId::of::<T>()] }

    #[inline]
    fn write_types() -> Vec<ComponentTypeId> { vec![ComponentTypeId::of::<T>()] }
}

impl<'a, T: Component> ViewElement for RefMut<'static, T> {
    type Component = T;
}

impl<'a, T: Component> DefaultFilter for Ref<'static, T> {
    type Filter = EntityFilterTuple<ComponentFilter<T>, Passthrough, Passthrough>;

    fn filter() -> Self::Filter { super::filter::filter_fns::component() }
}

impl<'a, T: Component> View<'a> for Ref<'static, T> {
    type Iter = RefIter<'a, T, Iter<'a, T>>;

    #[inline]
    fn fetch(
        _: &'a ArchetypeData,
        chunk: &'a ComponentStorage,
        _: ChunkIndex,
        _: SetIndex,
    ) -> Self::Iter {
        let (slice_borrow, slice) = unsafe {
            chunk
                .components(ComponentTypeId::of::<T>())
                .unwrap_or_else(|| {
                    panic!(
                        "Component of type {:?} not found in chunk when fetching Write view",
                        std::any::type_name::<T>()
                    )
                })
                .data_slice::<T>()
                .deconstruct()
        };
        RefIter::new(slice_borrow, slice.iter())
    }

    #[inline]
    fn validate() -> bool { true }

    #[inline]
    fn reads<D: Component>() -> bool { TypeId::of::<T>() == TypeId::of::<D>() }

    #[inline]
    fn writes<D: Component>() -> bool { false }

    #[inline]
    fn read_types() -> Vec<ComponentTypeId> { vec![ComponentTypeId::of::<T>()] }

    #[inline]
    fn write_types() -> Vec<ComponentTypeId> { Vec::new() }
}

impl<'a, T: Component> ViewElement for Ref<'static, T> {
    type Component = T;
}
