use std::{
    fmt::Debug,
    hash::{Hash, Hasher},
};

use std::{any::TypeId, marker::PhantomData};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub struct HandleId(Uuid);
pub const DEFAULT_HANDLE_ID: HandleId = HandleId(Uuid::from_bytes([
    238, 232, 56, 216, 245, 246, 77, 29, 165, 188, 211, 202, 249, 248, 15, 4,
]));

impl HandleId {
    pub fn new() -> HandleId {
        HandleId(Uuid::new_v4())
    }
}

pub struct Handle<T> {
    pub id: HandleId,
    marker: PhantomData<T>,
}

impl<T> Handle<T> {
    pub fn new(id: HandleId) -> Self {
        Handle {
            id,
            marker: PhantomData,
        }
    }

    pub fn from_untyped(untyped_handle: HandleUntyped) -> Option<Handle<T>>
    where
        T: 'static,
    {
        if TypeId::of::<T>() == untyped_handle.type_id {
            Some(Handle::new(untyped_handle.id))
        } else {
            None
        }
    }
}

impl<T> Hash for Handle<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

impl<T> PartialEq for Handle<T> {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl<T> Eq for Handle<T> {}

impl<T> Debug for Handle<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
        let name = std::any::type_name::<T>().split("::").last().unwrap();
        write!(f, "Handle<{}>({:?})", name, self.id.0)
    }
}

impl<T> Default for Handle<T> {
    fn default() -> Self {
        Handle {
            id: DEFAULT_HANDLE_ID,
            marker: PhantomData,
        }
    }
}

impl<T> Clone for Handle<T> {
    fn clone(&self) -> Self {
        Handle {
            id: self.id.clone(),
            marker: PhantomData,
        }
    }
}
impl<T> Copy for Handle<T> {}

#[derive(Hash, Copy, Clone, Eq, PartialEq, Debug)]
pub struct HandleUntyped {
    pub id: HandleId,
    pub type_id: TypeId,
}

impl HandleUntyped {
    pub fn is_handle<T: 'static>(untyped: &HandleUntyped) -> bool {
        TypeId::of::<T>() == untyped.type_id
    }
}

impl<T> From<Handle<T>> for HandleUntyped
where
    T: 'static,
{
    fn from(handle: Handle<T>) -> Self {
        HandleUntyped {
            id: handle.id,
            type_id: TypeId::of::<T>(),
        }
    }
}

impl<T> From<HandleUntyped> for Handle<T>
where
    T: 'static,
{
    fn from(handle: HandleUntyped) -> Self {
        Handle::from_untyped(handle)
            .expect("attempted to convert untyped handle to incorrect typed handle")
    }
}
