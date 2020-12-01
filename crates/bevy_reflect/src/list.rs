use std::any::Any;

use crate::{serde::Serializable, Reflect, ReflectMut, ReflectRef};

/// An ordered, mutable list of [ReflectValue] items. This corresponds to types like [std::vec::Vec].
pub trait List: Reflect {
    fn get(&self, index: usize) -> Option<&dyn Reflect>;
    fn get_mut(&mut self, index: usize) -> Option<&mut dyn Reflect>;
    fn push(&mut self, value: Box<dyn Reflect>);
    fn len(&self) -> usize;
    fn is_empty(&self) -> bool {
        self.len() == 0
    }
    fn iter(&self) -> ListIter;
    fn clone_dynamic(&self) -> DynamicList {
        DynamicList {
            values: self.iter().map(|value| value.clone_value()).collect(),
        }
    }
}

#[derive(Default)]
pub struct DynamicList {
    pub(crate) values: Vec<Box<dyn Reflect>>,
}

impl DynamicList {
    pub fn push<T: Reflect>(&mut self, value: T) {
        self.values.push(Box::new(value));
    }

    pub fn push_box(&mut self, value: Box<dyn Reflect>) {
        self.values.push(value);
    }
}

impl List for DynamicList {
    fn get(&self, index: usize) -> Option<&dyn Reflect> {
        self.values.get(index).map(|value| &**value)
    }

    fn get_mut(&mut self, index: usize) -> Option<&mut dyn Reflect> {
        self.values.get_mut(index).map(|value| &mut **value)
    }

    fn len(&self) -> usize {
        self.values.len()
    }

    fn clone_dynamic(&self) -> DynamicList {
        DynamicList {
            values: self
                .values
                .iter()
                .map(|value| value.clone_value())
                .collect(),
        }
    }

    fn iter(&self) -> ListIter {
        ListIter {
            list: self,
            index: 0,
        }
    }

    fn push(&mut self, value: Box<dyn Reflect>) {
        DynamicList::push_box(self, value);
    }
}

impl Reflect for DynamicList {
    #[inline]
    fn type_name(&self) -> &str {
        std::any::type_name::<Self>()
    }

    #[inline]
    fn any(&self) -> &dyn Any {
        self
    }

    #[inline]
    fn any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn apply(&mut self, value: &dyn Reflect) {
        list_apply(self, value);
    }

    #[inline]
    fn set(&mut self, value: Box<dyn Reflect>) -> Result<(), Box<dyn Reflect>> {
        *self = value.take()?;
        Ok(())
    }

    #[inline]
    fn reflect_ref(&self) -> ReflectRef {
        ReflectRef::List(self)
    }

    #[inline]
    fn reflect_mut(&mut self) -> ReflectMut {
        ReflectMut::List(self)
    }

    #[inline]
    fn clone_value(&self) -> Box<dyn Reflect> {
        Box::new(self.clone_dynamic())
    }

    #[inline]
    fn reflect_hash(&self) -> Option<u64> {
        None
    }

    fn reflect_partial_eq(&self, value: &dyn Reflect) -> Option<bool> {
        list_partial_eq(self, value)
    }

    fn serializable(&self) -> Option<Serializable> {
        None
    }
}

pub struct ListIter<'a> {
    pub(crate) list: &'a dyn List,
    pub(crate) index: usize,
}

impl<'a> Iterator for ListIter<'a> {
    type Item = &'a dyn Reflect;

    fn next(&mut self) -> Option<Self::Item> {
        let value = self.list.get(self.index);
        self.index += 1;
        value
    }
}

#[inline]
pub fn list_apply<L: List>(a: &mut L, b: &dyn Reflect) {
    if let ReflectRef::List(list_value) = b.reflect_ref() {
        for (i, value) in list_value.iter().enumerate() {
            if i < a.len() {
                if let Some(v) = a.get_mut(i) {
                    v.apply(value);
                }
            } else {
                List::push(a, value.clone_value());
            }
        }
    } else {
        panic!("Attempted to apply a non-list type to a list type.");
    }
}

#[inline]
pub fn list_partial_eq<L: List>(a: &L, b: &dyn Reflect) -> Option<bool> {
    let list = if let ReflectRef::List(list) = b.reflect_ref() {
        list
    } else {
        return Some(false);
    };

    if a.len() != list.len() {
        return Some(false);
    }

    for (a_value, b_value) in a.iter().zip(list.iter()) {
        if let Some(false) | None = a_value.reflect_partial_eq(b_value) {
            return Some(false);
        }
    }

    Some(true)
}
