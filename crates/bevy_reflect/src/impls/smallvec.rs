use bevy_reflect_derive::impl_type_path;
use smallvec::{Array as SmallArray, SmallVec};

use std::any::Any;

use crate::diff::{
    diff_list, DiffApplyError, DiffApplyResult, DiffResult, ElementDiff, ListDiff, ValueDiff,
};
use crate::utility::GenericTypeInfoCell;
use crate::{
    self as bevy_reflect, ApplyError, FromReflect, FromType, GetTypeRegistration, List, ListInfo,
    ListIter, Reflect, ReflectFromPtr, ReflectKind, ReflectMut, ReflectOwned, ReflectRef, TypeInfo,
    TypePath, TypeRegistration, Typed,
};

impl<T: SmallArray + TypePath + Send + Sync> List for SmallVec<T>
where
    T::Item: FromReflect + TypePath,
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
            <T as SmallArray>::Item::from_reflect(&*value).unwrap_or_else(|| {
                panic!(
                    "Attempted to insert invalid value of type {}.",
                    value.reflect_type_path()
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
            <T as SmallArray>::Item::from_reflect(&*value).unwrap_or_else(|| {
                panic!(
                    "Attempted to push invalid value of type {}.",
                    value.reflect_type_path()
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

    fn apply_list_diff(&mut self, diff: ListDiff) -> DiffApplyResult {
        let info = <Self as Typed>::type_info();

        if info.type_id() != diff.type_info().type_id() {
            return Err(DiffApplyError::TypeMismatch);
        }

        let new_len = (self.len() + diff.total_insertions()) - diff.total_deletions();
        let mut new = Self::with_capacity(new_len);

        let mut changes = diff.take_changes();
        changes.reverse();

        fn has_change(changes: &[ElementDiff], index: usize) -> bool {
            changes
                .last()
                .map(|change| change.index() == index)
                .unwrap_or_default()
        }

        let insert = |value: ValueDiff, list: &mut Self| -> Result<(), DiffApplyError> {
            list.push(
                <T::Item as FromReflect>::from_reflect(value.as_reflect())
                    .ok_or(DiffApplyError::TypeMismatch)?,
            );
            Ok(())
        };

        'outer: for (curr_index, element) in self.drain(..).enumerate() {
            while has_change(&changes, curr_index) {
                match changes.pop().unwrap() {
                    ElementDiff::Deleted(_) => {
                        continue 'outer;
                    }
                    ElementDiff::Inserted(_, value) => {
                        insert(value, &mut new)?;
                    }
                }
            }

            new.push(element);
        }

        // Insert any remaining elements
        while let Some(ElementDiff::Inserted(_, value)) = changes.pop() {
            insert(value, &mut new)?;
        }

        *self = new;
        Ok(())
    }
}

impl<T: SmallArray + TypePath + Send + Sync> Reflect for SmallVec<T>
where
    T::Item: FromReflect + TypePath,
{
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

    fn try_apply(&mut self, value: &dyn Reflect) -> Result<(), ApplyError> {
        crate::list_try_apply(self, value)
    }

    fn set(&mut self, value: Box<dyn Reflect>) -> Result<(), Box<dyn Reflect>> {
        *self = value.take()?;
        Ok(())
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

    fn clone_value(&self) -> Box<dyn Reflect> {
        Box::new(self.clone_dynamic())
    }

    fn diff<'new>(&self, other: &'new dyn Reflect) -> DiffResult<'_, 'new> {
        diff_list(self, other)
    }

    fn reflect_partial_eq(&self, value: &dyn Reflect) -> Option<bool> {
        crate::list_partial_eq(self, value)
    }
}

impl<T: SmallArray + TypePath + Send + Sync + 'static> Typed for SmallVec<T>
where
    T::Item: FromReflect + TypePath,
{
    fn type_info() -> &'static TypeInfo {
        static CELL: GenericTypeInfoCell = GenericTypeInfoCell::new();
        CELL.get_or_insert::<Self, _>(|| TypeInfo::List(ListInfo::new::<Self, T::Item>()))
    }
}

impl_type_path!(::smallvec::SmallVec<T: SmallArray>);

impl<T: SmallArray + TypePath + Send + Sync> FromReflect for SmallVec<T>
where
    T::Item: FromReflect + TypePath,
{
    fn from_reflect(reflect: &dyn Reflect) -> Option<Self> {
        if let ReflectRef::List(ref_list) = reflect.reflect_ref() {
            let mut new_list = Self::with_capacity(ref_list.len());
            for field in ref_list.iter() {
                new_list.push(<T as SmallArray>::Item::from_reflect(field)?);
            }
            Some(new_list)
        } else {
            None
        }
    }
}

impl<T: SmallArray + TypePath + Send + Sync> GetTypeRegistration for SmallVec<T>
where
    T::Item: FromReflect + TypePath,
{
    fn get_type_registration() -> TypeRegistration {
        let mut registration = TypeRegistration::of::<SmallVec<T>>();
        registration.insert::<ReflectFromPtr>(FromType::<SmallVec<T>>::from_type());
        registration
    }
}

#[cfg(feature = "functions")]
crate::func::macros::impl_function_traits!(SmallVec<T>; <T: SmallArray + TypePath + Send + Sync> where T::Item: FromReflect + TypePath);
