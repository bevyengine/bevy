use bevy_ecs::{Fetch, QueryAccess, ReadOnlyFetch, WorldQuery};

/// Especial component type that every entity has, used
/// to build queries where the components doesn't matter.
#[derive(Debug, Copy, Clone)]
pub struct Empty;

impl WorldQuery for Empty {
    type Fetch = EmptyFetch;
}

pub struct EmptyFetch;

impl<'a> Fetch<'a> for EmptyFetch {
    type Item = Empty;
    const DANGLING: Self = EmptyFetch;

    fn access() -> QueryAccess {
        QueryAccess::None
    }

    unsafe fn get(_archetype: &'a bevy_ecs::Archetype, _offset: usize) -> Option<Self> {
        Some(EmptyFetch)
    }

    unsafe fn fetch(&self, _n: usize) -> Self::Item {
        Empty
    }
}

unsafe impl ReadOnlyFetch for EmptyFetch {}
