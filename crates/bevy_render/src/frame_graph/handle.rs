use std::{hash::Hash, marker::PhantomData};

pub struct TypeHandle<T> {
    pub index: usize,
    _marker: PhantomData<T>,
}

impl<T> Hash for TypeHandle<T> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.index.hash(state);
    }
}

impl<T> Eq for TypeHandle<T> {}

impl<T> PartialEq for TypeHandle<T> {
    fn eq(&self, other: &Self) -> bool {
        self.index == other.index && self._marker == other._marker
    }
}

impl<T> Copy for TypeHandle<T> {}

impl<T> Clone for TypeHandle<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> TypeHandle<T> {
    pub fn new(index: usize) -> Self {
        TypeHandle {
            index,
            _marker: PhantomData,
        }
    }
}
