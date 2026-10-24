use crate::{
    dynamic::DynamicValue,
    list::{List, ListInfo, ListIter},
    utility::GenericTypeInfoCell,
    FromDynamic, Generics, GetTypeRegistration, MaybeTyped, Reflect, ReflectFromPtr, ReflectKind,
    ReflectMut, ReflectOwned, ReflectRef, TypeInfo, TypeParamInfo, TypePath, TypeRegistration,
    Typed,
};
use alloc::{boxed::Box, vec::Vec};
use bevy_reflect::ReflectCloneError;
use bevy_reflect_derive::impl_type_path;
use core::any::Any;
use smallvec::{Array as SmallArray, SmallVec};

impl<T: SmallArray + TypePath + Send + Sync> List for SmallVec<T>
where
    T::Item: FromDynamic + MaybeTyped + TypePath,
{
    fn get(&self, index: usize) -> Option<&dyn Reflect> {
        if index < SmallVec::len(self) {
            Some(&self[index] as &dyn Reflect)
        } else {
            None
        }
    }

    fn get_mut(&mut self, index: usize) -> Option<&mut dyn Reflect> {
        if index < SmallVec::len(self) {
            Some(&mut self[index] as &mut dyn Reflect)
        } else {
            None
        }
    }

    fn value_from_dynamic(&self, dynamic: DynamicValue) -> Option<Box<dyn Reflect>> {
        T::Item::from_dynamic(dynamic).map(|value| Box::new(value) as _)
    }

    fn insert(&mut self, index: usize, value: Box<dyn Reflect>) {
        let value = value.take::<T::Item>().unwrap_or_else(|value| {
            panic!(
                "Attempted to insert invalid value of type {}.",
                value.reflect_type_path()
            )
        });
        SmallVec::insert(self, index, value);
    }

    fn remove(&mut self, index: usize) -> Box<dyn Reflect> {
        Box::new(self.remove(index))
    }

    fn swap(&mut self, a: usize, b: usize) {
        self.as_mut_slice().swap(a, b);
    }

    fn push(&mut self, value: Box<dyn Reflect>) {
        let value = value.take::<T::Item>().unwrap_or_else(|value| {
            panic!(
                "Attempted to push invalid value of type {}.",
                value.reflect_type_path()
            )
        });
        SmallVec::push(self, value);
    }

    fn pop(&mut self) -> Option<Box<dyn Reflect>> {
        self.pop().map(|value| Box::new(value) as Box<dyn Reflect>)
    }

    fn len(&self) -> usize {
        <SmallVec<T>>::len(self)
    }

    fn iter(&self) -> ListIter<'_> {
        ListIter::new(self)
    }

    fn drain(&mut self) -> Vec<Box<dyn Reflect>> {
        self.drain(..)
            .map(|value| Box::new(value) as Box<dyn Reflect>)
            .collect()
    }
}

impl<T: SmallArray + TypePath + Send + Sync> Reflect for SmallVec<T>
where
    T::Item: FromDynamic + MaybeTyped + TypePath,
{
    fn reflect_kind(&self) -> ReflectKind {
        ReflectKind::List
    }

    fn reflect_ref(&self) -> ReflectRef<'_> {
        ReflectRef::List(self)
    }

    fn reflect_mut(&mut self) -> ReflectMut<'_> {
        ReflectMut::List(self)
    }

    fn reflect_owned(self: Box<Self>) -> ReflectOwned {
        ReflectOwned::List(self)
    }

    fn reflect_clone(&self) -> Result<Box<dyn Reflect>, ReflectCloneError> {
        Ok(Box::new(
            // `(**self)` avoids getting `SmallVec<T> as List::iter`, which
            // would give us the wrong item type.
            (**self)
                .iter()
                .map(Reflect::reflect_clone_and_take)
                .collect::<Result<Self, ReflectCloneError>>()?,
        ))
    }

    fn reflect_partial_eq(&self, value: &dyn Reflect) -> Option<bool> {
        crate::list::list_partial_eq(self, value)
    }

    fn reflect_partial_cmp(&self, value: &dyn Reflect) -> Option<core::cmp::Ordering> {
        crate::list::list_partial_cmp(self, value)
    }

    fn into_any(self: Box<Self>) -> Box<dyn Any> {
        self
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn into_reflect(self: Box<Self>) -> Box<dyn Reflect> {
        self
    }

    fn as_reflect(&self) -> &dyn Reflect {
        self
    }

    fn as_reflect_mut(&mut self) -> &mut dyn Reflect {
        self
    }

    fn set(&mut self, value: Box<dyn Reflect>) -> Result<(), Box<dyn Reflect>> {
        *self = value.take()?;
        Ok(())
    }
}

impl<T: SmallArray + TypePath + Send + Sync + 'static> Typed for SmallVec<T>
where
    T::Item: FromDynamic + MaybeTyped + TypePath,
{
    fn type_info() -> &'static TypeInfo {
        static CELL: GenericTypeInfoCell = GenericTypeInfoCell::new();
        CELL.get_or_insert::<Self, _>(|| {
            TypeInfo::List(
                ListInfo::new::<Self, T::Item>()
                    .with_generics(Generics::from_iter([TypeParamInfo::new::<T>("T")])),
            )
        })
    }
}

impl_type_path!(::smallvec::SmallVec<T: SmallArray>);

impl<T: SmallArray + TypePath + Send + Sync> FromDynamic for SmallVec<T>
where
    T::Item: FromDynamic + MaybeTyped + TypePath,
{
    fn from_dynamic(dynamic: DynamicValue) -> Option<Self> {
        let dynamic = match dynamic {
            DynamicValue::Concrete(concrete) => return concrete.downcast().ok().map(|v| *v),
            DynamicValue::List(dynamic) => dynamic,
            _ => return None,
        };

        let mut new_list = Self::with_capacity(dynamic.len());

        for value in dynamic.into_iter() {
            new_list.push(<T as SmallArray>::Item::from_dynamic(value)?);
        }

        Some(new_list)
    }
}

impl<T: SmallArray + TypePath + Send + Sync> GetTypeRegistration for SmallVec<T>
where
    T::Item: FromDynamic + MaybeTyped + TypePath,
{
    fn get_type_registration() -> TypeRegistration {
        let mut registration = TypeRegistration::of::<SmallVec<T>>();
        registration.register_type_data::<ReflectFromPtr, Self>();
        registration
    }
}

#[cfg(feature = "functions")]
crate::func::macros::impl_function_traits!(SmallVec<T>; <T: SmallArray + TypePath + Send + Sync> where T::Item: FromDynamic + MaybeTyped + TypePath);
