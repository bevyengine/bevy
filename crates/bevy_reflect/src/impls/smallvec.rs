use bevy_reflect_derive::impl_type_path;
use smallvec::SmallVec;
use std::any::{Any, TypeId};

use crate::utility::GenericTypeInfoCell;
use crate::{
    self as bevy_reflect, FromReflect, FromType, GetTypeRegistration, List, ListInfo, ListIter,
    Reflect, ReflectFromPtr, ReflectMut, ReflectOwned, ReflectRef, TypeInfo, TypePath,
    TypeRegistration, Typed,
};
use std::hash::{Hash, Hasher};

impl<T: smallvec::Array + TypePath + Send + Sync> List for SmallVec<T>
where
    T::Item: FromReflect,
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

    fn insert(&mut self, index: usize, value: Box<dyn Reflect>) {
        let value = value.take::<T::Item>().unwrap_or_else(|value| {
            <T as smallvec::Array>::Item::from_reflect(&*value).unwrap_or_else(|| {
                panic!(
                    "Attempted to insert invalid value of type {}.",
                    value.type_name()
                )
            })
        });
        SmallVec::insert(self, index, value);
    }

    fn remove(&mut self, index: usize) -> Box<dyn Reflect> {
        Box::new(self.remove(index))
    }

    fn push(&mut self, value: Box<dyn Reflect>) {
        let value = value.take::<T::Item>().unwrap_or_else(|value| {
            <T as smallvec::Array>::Item::from_reflect(&*value).unwrap_or_else(|| {
                panic!(
                    "Attempted to push invalid value of type {}.",
                    value.type_name()
                )
            })
        });
        SmallVec::push(self, value);
    }

    fn pop(&mut self) -> Option<Box<dyn Reflect>> {
        self.pop().map(|value| Box::new(value) as Box<dyn Reflect>)
    }

    fn len(&self) -> usize {
        <SmallVec<T>>::len(self)
    }

    fn iter(&self) -> ListIter {
        ListIter::new(self)
    }

    fn drain(self: Box<Self>) -> Vec<Box<dyn Reflect>> {
        self.into_iter()
            .map(|value| Box::new(value) as Box<dyn Reflect>)
            .collect()
    }
}

impl<T: smallvec::Array + TypePath + Send + Sync> Reflect for SmallVec<T>
where
    T::Item: FromReflect,
{
    fn type_name(&self) -> &str {
        std::any::type_name::<Self>()
    }

    fn get_represented_type_info(&self) -> Option<&'static TypeInfo> {
        Some(<Self as Typed>::type_info())
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

    fn apply(&mut self, value: &dyn Reflect) {
        crate::list_apply(self, value);
    }

    fn set(&mut self, value: Box<dyn Reflect>) -> Result<(), Box<dyn Reflect>> {
        *self = value.take()?;
        Ok(())
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

    fn clone_value(&self) -> Box<dyn Reflect> {
        Box::new(self.clone_dynamic())
    }

    fn reflect_hash(&self) -> Option<u64> {
        let mut hasher = crate::utility::reflect_hasher();
        Hash::hash(&TypeId::of::<Self>(), &mut hasher);
        Hash::hash(&self.len(), &mut hasher);

        for element in self {
            Hash::hash(&element.reflect_hash()?, &mut hasher);
        }

        Some(hasher.finish())
    }

    fn reflect_partial_eq(&self, other: &dyn Reflect) -> Option<bool> {
        if let Some(other) = other.downcast_ref::<Self>() {
            for (a, b) in <[T::Item]>::iter(self).zip(<[T::Item]>::iter(other)) {
                if !a.reflect_partial_eq(b)? {
                    return Some(false);
                }
            }
        } else {
            let ReflectRef::List(other) = Reflect::reflect_ref(other) else {
                return Some(false);
            };

            if other.len() != self.len() {
                return Some(false);
            }

            for (a, b) in self.iter().zip(other.iter()) {
                if !a.reflect_partial_eq(b)? {
                    return Some(false);
                }
            }
        }

        Some(true)
    }
}

impl<T: smallvec::Array + TypePath + Send + Sync + 'static> Typed for SmallVec<T>
where
    T::Item: FromReflect,
{
    fn type_info() -> &'static TypeInfo {
        static CELL: GenericTypeInfoCell = GenericTypeInfoCell::new();
        CELL.get_or_insert::<Self, _>(|| TypeInfo::List(ListInfo::new::<Self, T::Item>()))
    }
}

impl_type_path!(::smallvec::SmallVec<T: smallvec::Array + TypePath + Send + Sync>);

impl<T: smallvec::Array + TypePath + Send + Sync> FromReflect for SmallVec<T>
where
    T::Item: FromReflect,
{
    fn from_reflect(reflect: &dyn Reflect) -> Option<Self> {
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

impl<T: smallvec::Array + TypePath + Send + Sync> GetTypeRegistration for SmallVec<T>
where
    T::Item: FromReflect,
{
    fn get_type_registration() -> TypeRegistration {
        let mut registration = TypeRegistration::of::<SmallVec<T>>();
        registration.insert::<ReflectFromPtr>(FromType::<SmallVec<T>>::from_type());
        registration
    }
}
