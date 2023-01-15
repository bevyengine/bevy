use smallvec::SmallVec;
use std::any::Any;

use crate::utility::GenericTypeInfoCell;
use crate::{
    Array, ArrayIter, FromReflect, FromType, GetTypeRegistration, List, ListInfo, PartialReflect,
    Reflect, ReflectFromPtr, ReflectMut, ReflectOwned, ReflectRef, TypeInfo, TypeRegistration,
    Typed,
};

impl<T: smallvec::Array + Send + Sync + 'static> Array for SmallVec<T>
where
    T::Item: FromReflect,
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

    fn len(&self) -> usize {
        <SmallVec<T>>::len(self)
    }

    fn iter(&self) -> ArrayIter {
        ArrayIter::new(self)
    }

    fn drain(self: Box<Self>) -> Vec<Box<dyn PartialReflect>> {
        self.into_iter()
            .map(|value| Box::new(value) as Box<dyn PartialReflect>)
            .collect()
    }
}

impl<T: smallvec::Array + Send + Sync + 'static> List for SmallVec<T>
where
    T::Item: FromReflect,
{
    fn insert(&mut self, index: usize, value: Box<dyn PartialReflect>) {
        let value = value.try_take().unwrap_or_else(|value| {
            <T as smallvec::Array>::Item::from_reflect(&*value).unwrap_or_else(|| {
                panic!(
                    "Attempted to insert invalid value of type {}.",
                    value.type_name()
                )
            })
        });
        SmallVec::insert(self, index, value);
    }

    fn remove(&mut self, index: usize) -> Box<dyn PartialReflect> {
        Box::new(self.remove(index))
    }

    fn push(&mut self, value: Box<dyn PartialReflect>) {
        let value = value.try_take().unwrap_or_else(|value| {
            <T as smallvec::Array>::Item::from_reflect(&*value).unwrap_or_else(|| {
                panic!(
                    "Attempted to push invalid value of type {}.",
                    value.type_name()
                )
            })
        });
        SmallVec::push(self, value);
    }

    fn pop(&mut self) -> Option<Box<dyn PartialReflect>> {
        self.pop()
            .map(|value| Box::new(value) as Box<dyn PartialReflect>)
    }
}

impl<T: smallvec::Array + Send + Sync + 'static> PartialReflect for SmallVec<T>
where
    T::Item: FromReflect,
{
    fn type_name(&self) -> &str {
        std::any::type_name::<Self>()
    }

    fn get_type_info(&self) -> &'static TypeInfo {
        <Self as Typed>::type_info()
    }

    fn as_full(&self) -> Option<&dyn Reflect> {
        Some(self)
    }

    fn as_full_mut(&mut self) -> Option<&mut dyn Reflect> {
        Some(self)
    }

    fn into_full(self: Box<Self>) -> Result<Box<dyn Reflect>, Box<dyn PartialReflect>> {
        Ok(self)
    }

    fn as_partial(&self) -> &dyn PartialReflect {
        self
    }

    fn as_partial_mut(&mut self) -> &mut dyn PartialReflect {
        self
    }

    fn into_partial(self: Box<Self>) -> Box<dyn PartialReflect> {
        self
    }

    fn apply(&mut self, value: &dyn PartialReflect) {
        crate::list_apply(self, value);
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

    fn clone_value(&self) -> Box<dyn PartialReflect> {
        Box::new(List::clone_dynamic(self))
    }

    fn reflect_partial_eq(&self, value: &dyn PartialReflect) -> Option<bool> {
        crate::list_partial_eq(self, value)
    }
}

impl<T: smallvec::Array + Send + Sync + 'static> Reflect for SmallVec<T>
where
    T::Item: FromReflect,
{
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn into_any(self: Box<Self>) -> Box<dyn Any> {
        self
    }

    fn as_reflect(&self) -> &dyn Reflect {
        self
    }

    fn as_reflect_mut(&mut self) -> &mut dyn Reflect {
        self
    }

    fn into_reflect(self: Box<Self>) -> Box<dyn Reflect> {
        self
    }

    fn set(&mut self, value: Box<dyn PartialReflect>) -> Result<(), Box<dyn PartialReflect>> {
        *self = value.try_take()?;
        Ok(())
    }
}

impl<T: smallvec::Array + Send + Sync + 'static> Typed for SmallVec<T>
where
    T::Item: FromReflect,
{
    fn type_info() -> &'static TypeInfo {
        static CELL: GenericTypeInfoCell = GenericTypeInfoCell::new();
        CELL.get_or_insert::<Self, _>(|| TypeInfo::List(ListInfo::new::<Self, T::Item>()))
    }
}

impl<T: smallvec::Array + Send + Sync + 'static> FromReflect for SmallVec<T>
where
    T::Item: FromReflect,
{
    fn from_reflect(reflect: &dyn PartialReflect) -> Option<Self> {
        if let ReflectRef::List(ref_list) = reflect.reflect_ref() {
            let mut new_list = Self::with_capacity(ref_list.len());
            for field in ref_list.iter() {
                new_list.push(<T as smallvec::Array>::Item::from_reflect(field)?);
            }
            Some(new_list)
        } else {
            None
        }
    }
}

impl<T: smallvec::Array + Send + Sync + 'static> GetTypeRegistration for SmallVec<T>
where
    T::Item: FromReflect,
{
    fn get_type_registration() -> TypeRegistration {
        let mut registration = TypeRegistration::of::<SmallVec<T>>();
        registration.insert::<ReflectFromPtr>(FromType::<SmallVec<T>>::from_type());
        registration
    }
}
