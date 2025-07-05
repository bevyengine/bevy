use crate::{
    utility::GenericTypeInfoCell, ApplyError, FromReflect, FromType, Generics, GetTypeRegistration,
    List, ListInfo, ListIter, MaybeTyped, PartialReflect, Reflect, ReflectFromPtr, ReflectKind,
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
    T::Item: FromReflect + MaybeTyped + TypePath,
{
    fn get(&self, index: usize) -> Option<&dyn PartialReflect> {
        if index < SmallVec::len(self) {
            Some(&self[index] as &dyn PartialReflect)
        } else {
            None
        }
    }

    fn get_mut(&mut self, index: usize) -> Option<&mut dyn PartialReflect> {
        if index < SmallVec::len(self) {
            Some(&mut self[index] as &mut dyn PartialReflect)
        } else {
            None
        }
    }

    fn insert(&mut self, index: usize, value: Box<dyn PartialReflect>) {
        let value = value.try_take::<T::Item>().unwrap_or_else(|value| {
            <T as SmallArray>::Item::from_reflect(&*value).unwrap_or_else(|| {
                panic!(
                    "Attempted to insert invalid value of type {}.",
                    value.reflect_type_path()
                )
            })
        });
        SmallVec::insert(self, index, value);
    }

    fn remove(&mut self, index: usize) -> Box<dyn PartialReflect> {
        Box::new(self.remove(index))
    }

    fn push(&mut self, value: Box<dyn PartialReflect>) {
        let value = value.try_take::<T::Item>().unwrap_or_else(|value| {
            <T as SmallArray>::Item::from_reflect(&*value).unwrap_or_else(|| {
                panic!(
                    "Attempted to push invalid value of type {}.",
                    value.reflect_type_path()
                )
            })
        });
        SmallVec::push(self, value);
    }

    fn pop(&mut self) -> Option<Box<dyn PartialReflect>> {
        self.pop()
            .map(|value| Box::new(value) as Box<dyn PartialReflect>)
    }

    fn len(&self) -> usize {
        <SmallVec<T>>::len(self)
    }

    fn iter(&self) -> ListIter {
        ListIter::new(self)
    }

    fn drain(&mut self) -> Vec<Box<dyn PartialReflect>> {
        self.drain(..)
            .map(|value| Box::new(value) as Box<dyn PartialReflect>)
            .collect()
    }
}

impl<T: SmallArray + TypePath + Send + Sync> PartialReflect for SmallVec<T>
where
    T::Item: FromReflect + MaybeTyped + TypePath,
{
    fn get_represented_type_info(&self) -> Option<&'static TypeInfo> {
        Some(<Self as Typed>::type_info())
    }

    #[inline]
    fn into_partial_reflect(self: Box<Self>) -> Box<dyn PartialReflect> {
        self
    }

    fn as_partial_reflect(&self) -> &dyn PartialReflect {
        self
    }

    fn as_partial_reflect_mut(&mut self) -> &mut dyn PartialReflect {
        self
    }

    fn try_into_reflect(self: Box<Self>) -> Result<Box<dyn Reflect>, Box<dyn PartialReflect>> {
        Ok(self)
    }

    fn try_as_reflect(&self) -> Option<&dyn Reflect> {
        Some(self)
    }

    fn try_as_reflect_mut(&mut self) -> Option<&mut dyn Reflect> {
        Some(self)
    }

    fn apply(&mut self, value: &dyn PartialReflect) {
        crate::list_apply(self, value);
    }

    fn try_apply(&mut self, value: &dyn PartialReflect) -> Result<(), ApplyError> {
        crate::list_try_apply(self, value)
    }

    fn reflect_kind(&self) -> ReflectKind {
        ReflectKind::List
    }

    fn reflect_ref(&self) -> ReflectRef {
        ReflectRef::List(self)
    }

    fn reflect_mut(&mut self) -> ReflectMut {
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
                .map(PartialReflect::reflect_clone_and_take)
                .collect::<Result<Self, ReflectCloneError>>()?,
        ))
    }

    fn reflect_partial_eq(&self, value: &dyn PartialReflect) -> Option<bool> {
        crate::list_partial_eq(self, value)
    }
}

impl<T: SmallArray + TypePath + Send + Sync> Reflect for SmallVec<T>
where
    T::Item: FromReflect + MaybeTyped + TypePath,
{
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
    T::Item: FromReflect + MaybeTyped + TypePath,
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

impl<T: SmallArray + TypePath + Send + Sync> FromReflect for SmallVec<T>
where
    T::Item: FromReflect + MaybeTyped + TypePath,
{
    fn from_reflect(reflect: &dyn PartialReflect) -> Option<Self> {
        let ref_list = reflect.reflect_ref().as_list().ok()?;

        let mut new_list = Self::with_capacity(ref_list.len());

        for field in ref_list.iter() {
            new_list.push(<T as SmallArray>::Item::from_reflect(field)?);
        }

        Some(new_list)
    }
}

impl<T: SmallArray + TypePath + Send + Sync> GetTypeRegistration for SmallVec<T>
where
    T::Item: FromReflect + MaybeTyped + TypePath,
{
    fn get_type_registration() -> TypeRegistration {
        let mut registration = TypeRegistration::of::<SmallVec<T>>();
        registration.insert::<ReflectFromPtr>(FromType::<SmallVec<T>>::from_type());
        registration
    }
}

#[cfg(feature = "functions")]
crate::func::macros::impl_function_traits!(SmallVec<T>; <T: SmallArray + TypePath + Send + Sync> where T::Item: FromReflect + MaybeTyped + TypePath);
