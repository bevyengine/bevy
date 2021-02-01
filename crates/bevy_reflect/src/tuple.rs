use std::any::Any;

use crate::{serde::Serializable, Reflect, ReflectMut, ReflectRef};

pub trait Tuple: Reflect {
    fn field(&self, index: usize) -> Option<&dyn Reflect>;
    fn field_mut(&mut self, index: usize) -> Option<&mut dyn Reflect>;
    fn field_len(&self) -> usize;
    fn iter_fields(&self) -> TupleFieldIter;
    fn clone_dynamic(&self) -> DynamicTuple;
}

pub struct TupleFieldIter<'a> {
    pub(crate) tuple: &'a dyn Tuple,
    pub(crate) index: usize,
}

impl<'a> TupleFieldIter<'a> {
    pub fn new(value: &'a dyn Tuple) -> Self {
        TupleFieldIter {
            tuple: value,
            index: 0,
        }
    }
}

impl<'a> Iterator for TupleFieldIter<'a> {
    type Item = &'a dyn Reflect;

    fn next(&mut self) -> Option<Self::Item> {
        let value = self.tuple.field(self.index);
        self.index += 1;
        value
    }
}

pub trait GetTupleField {
    fn get_field<T: Reflect>(&self, index: usize) -> Option<&T>;
    fn get_field_mut<T: Reflect>(&mut self, index: usize) -> Option<&mut T>;
}

impl<S: Tuple> GetTupleField for S {
    fn get_field<T: Reflect>(&self, index: usize) -> Option<&T> {
        self.field(index)
            .and_then(|value| value.downcast_ref::<T>())
    }

    fn get_field_mut<T: Reflect>(&mut self, index: usize) -> Option<&mut T> {
        self.field_mut(index)
            .and_then(|value| value.downcast_mut::<T>())
    }
}

impl GetTupleField for dyn Tuple {
    fn get_field<T: Reflect>(&self, index: usize) -> Option<&T> {
        self.field(index)
            .and_then(|value| value.downcast_ref::<T>())
    }

    fn get_field_mut<T: Reflect>(&mut self, index: usize) -> Option<&mut T> {
        self.field_mut(index)
            .and_then(|value| value.downcast_mut::<T>())
    }
}

#[derive(Default)]
pub struct DynamicTuple {
    pub(crate) fields: Vec<Box<dyn Reflect>>,
}

impl DynamicTuple {
    pub fn insert_boxed(&mut self, value: Box<dyn Reflect>) {
        self.fields.push(value);
    }

    pub fn insert<T: Reflect>(&mut self, value: T) {
        self.insert_boxed(Box::new(value));
    }
}

impl Tuple for DynamicTuple {
    #[inline]
    fn field(&self, index: usize) -> Option<&dyn Reflect> {
        self.fields.get(index).map(|field| &**field)
    }

    #[inline]
    fn field_mut(&mut self, index: usize) -> Option<&mut dyn Reflect> {
        self.fields.get_mut(index).map(|field| &mut **field)
    }

    #[inline]
    fn field_len(&self) -> usize {
        self.fields.len()
    }

    #[inline]
    fn iter_fields(&self) -> TupleFieldIter {
        TupleFieldIter {
            tuple: self,
            index: 0,
        }
    }

    #[inline]
    fn clone_dynamic(&self) -> DynamicTuple {
        DynamicTuple {
            fields: self
                .fields
                .iter()
                .map(|value| value.clone_value())
                .collect(),
        }
    }
}

impl Reflect for DynamicTuple {
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

    #[inline]
    fn clone_value(&self) -> Box<dyn Reflect> {
        Box::new(self.clone_dynamic())
    }

    #[inline]
    fn reflect_ref(&self) -> ReflectRef {
        ReflectRef::Tuple(self)
    }

    #[inline]
    fn reflect_mut(&mut self) -> ReflectMut {
        ReflectMut::Tuple(self)
    }

    fn apply(&mut self, value: &dyn Reflect) {
        tuple_apply(self, value);
    }

    fn set(&mut self, value: Box<dyn Reflect>) -> Result<(), Box<dyn Reflect>> {
        *self = value.take()?;
        Ok(())
    }

    fn reflect_hash(&self) -> Option<u64> {
        None
    }

    fn reflect_partial_eq(&self, value: &dyn Reflect) -> Option<bool> {
        tuple_partial_eq(self, value)
    }

    fn serializable(&self) -> Option<Serializable> {
        None
    }
}

#[inline]
pub fn tuple_apply<T: Tuple>(a: &mut T, b: &dyn Reflect) {
    if let ReflectRef::Tuple(tuple) = b.reflect_ref() {
        for (i, value) in tuple.iter_fields().enumerate() {
            if let Some(v) = a.field_mut(i) {
                v.apply(value)
            }
        }
    } else {
        panic!("Attempted to apply non-Tuple type to Tuple type.");
    }
}

#[inline]
pub fn tuple_partial_eq<T: Tuple>(a: &T, b: &dyn Reflect) -> Option<bool> {
    let b = if let ReflectRef::Tuple(tuple) = b.reflect_ref() {
        tuple
    } else {
        return Some(false);
    };

    if a.field_len() != b.field_len() {
        return Some(false);
    }

    for (a_field, b_field) in a.iter_fields().zip(b.iter_fields()) {
        match a_field.reflect_partial_eq(b_field) {
            Some(false) | None => return Some(false),
            Some(true) => {}
        }
    }

    Some(true)
}

macro_rules! impl_reflect_tuple {
    {$($index:tt : $name:tt),*} => {
        impl<$($name: Reflect),*> Tuple for ($($name,)*) {
            #[inline]
            fn field(&self, index: usize) -> Option<&dyn Reflect> {
                match index {
                    $($index => Some(&self.$index as &dyn Reflect),)*
                    _ => None,
                }
            }

            #[inline]
            fn field_mut(&mut self, index: usize) -> Option<&mut dyn Reflect> {
                match index {
                    $($index => Some(&mut self.$index as &mut dyn Reflect),)*
                    _ => None,
                }
            }

            #[inline]
            fn field_len(&self) -> usize {
                let indices: &[usize] = &[$($index as usize),*];
                indices.len()
            }

            #[inline]
            fn iter_fields(&self) -> TupleFieldIter {
                TupleFieldIter {
                    tuple: self,
                    index: 0,
                }
            }

            #[inline]
            fn clone_dynamic(&self) -> DynamicTuple {
                DynamicTuple {
                    fields: self
                        .iter_fields()
                        .map(|value| value.clone_value())
                        .collect(),
                }
            }
        }

        impl<$($name: Reflect),*> Reflect for ($($name,)*) {
            fn type_name(&self) -> &str {
                std::any::type_name::<Self>()
            }

            fn any(&self) -> &dyn Any {
                self
            }

            fn any_mut(&mut self) -> &mut dyn Any {
                self
            }

            fn apply(&mut self, value: &dyn Reflect) {
                crate::tuple_apply(self, value);
            }

            fn set(&mut self, value: Box<dyn Reflect>) -> Result<(), Box<dyn Reflect>> {
                *self = value.take()?;
                Ok(())
            }

            fn reflect_ref(&self) -> ReflectRef {
                ReflectRef::Tuple(self)
            }

            fn reflect_mut(&mut self) -> ReflectMut {
                ReflectMut::Tuple(self)
            }

            fn clone_value(&self) -> Box<dyn Reflect> {
                Box::new(self.clone_dynamic())
            }

            fn reflect_hash(&self) -> Option<u64> {
                None
            }

            fn reflect_partial_eq(&self, value: &dyn Reflect) -> Option<bool> {
                crate::tuple_partial_eq(self, value)
            }

            fn serializable(&self) -> Option<Serializable> {
                None
            }
        }
    }
}

impl_reflect_tuple! {}
impl_reflect_tuple! {0: A}
impl_reflect_tuple! {0: A, 1: B}
impl_reflect_tuple! {0: A, 1: B, 2: C}
impl_reflect_tuple! {0: A, 1: B, 2: C, 3: D}
impl_reflect_tuple! {0: A, 1: B, 2: C, 3: D, 4: E}
impl_reflect_tuple! {0: A, 1: B, 2: C, 3: D, 4: E, 5: F}
impl_reflect_tuple! {0: A, 1: B, 2: C, 3: D, 4: E, 5: F, 6: G}
impl_reflect_tuple! {0: A, 1: B, 2: C, 3: D, 4: E, 5: F, 6: G, 7: H}
impl_reflect_tuple! {0: A, 1: B, 2: C, 3: D, 4: E, 5: F, 6: G, 7: H, 8: I}
impl_reflect_tuple! {0: A, 1: B, 2: C, 3: D, 4: E, 5: F, 6: G, 7: H, 8: I, 9: J}
impl_reflect_tuple! {0: A, 1: B, 2: C, 3: D, 4: E, 5: F, 6: G, 7: H, 8: I, 9: J, 10: K}
impl_reflect_tuple! {0: A, 1: B, 2: C, 3: D, 4: E, 5: F, 6: G, 7: H, 8: I, 9: J, 10: K, 11: L}
