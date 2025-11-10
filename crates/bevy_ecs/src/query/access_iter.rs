use core::{option};

use crate::component::ComponentId;



/// The data storage type that is being accessed.
#[derive(Clone, Copy)]
pub enum EcsAccessType {
    /// Accesses [`Component`] data
    Component(EcsAccessLevel),
    /// Accesses [`Resource`] data
    Resource(ResourceAccessLevel),
}

/// The way the data will be accessed and whether we take access on all the components on
/// an entity or just one component.
#[derive(Clone, Copy)]
pub enum EcsAccessLevel {
    /// Reads [`Component`] with [`ComponentId`]
    Read(ComponentId),
    /// Writes [`Component`] with [`ComponentId`]
    Write(ComponentId),
    /// Potentially reads all [`Component`]'s in the [`World`]
    ReadAll,
    /// Potentially writes all [`Component`]'s in the [`World`]
    WriteAll,
    /// [`FilteredEntityRef`] captures it's access at the `SystemParam` level, so will
    /// not conflict with other `QueryData` in the same Query
    FilteredReadAll,
    /// [`FilteredEntityMut`] captures it's access at the `SystemParam` level, so will
    /// not conflict with other `QueryData` in the same Query
    FilteredWriteAll,
    /// Potentially reads all [`Components`]'s except [`ComponentId`]
    ReadAllExcept {
        /// used to group excepts from the same QueryData together
        index: usize,
        /// read all except this id
        component_id: ComponentId,
    },
    /// Potentially writes all [`Components`]'s except [`ComponentId`]
    WriteAllExcept {
        /// used to group excepts from the same QueryData together
        index: usize,
        /// write all except this id
        component_id: ComponentId,
    },
}

/// Access level needed by QueryData fetch to the resource.
#[derive(Copy, Clone)]
pub enum ResourceAccessLevel {
    /// Reads the resource with [`ComponentId`]
    Read(ComponentId),
    /// Writes the resource with [`ComponentId`]
    Write(ComponentId),
}

impl EcsAccessType {
    /// Returns true if the access between `self` and `other` do not conflict.
    pub fn is_compatible(&self, other: Self) -> bool {
        use EcsAccessLevel::*;
        use EcsAccessType::*;

        match (self, other) {
            (Component(ReadAll), Component(Write(_)))
            | (Component(Write(_)), Component(ReadAll))
            | (Component(_), Component(WriteAll))
            | (Component(WriteAll), Component(_)) 
            | (Component(WriteAllExcept { .. }), Component(ReadAllExcept { .. }))
            | (Component(ReadAllExcept { .. }), Component(WriteAllExcept { .. }))
            | (Component(WriteAllExcept { .. }), Component(ReadAll))
            | (Component(ReadAll), Component(WriteAllExcept { .. })) => false,

            (Component(Read(id)), Component(Write(id_other)))
            | (Component(Write(id)), Component(Read(id_other)))
            | (Component(Write(id)), Component(Write(id_other)))
            | (Resource(ResourceAccessLevel::Read(id)), Resource(ResourceAccessLevel::Write(id_other)))
            | (Resource(ResourceAccessLevel::Write(id)), Resource(ResourceAccessLevel::Read(id_other)))
            | (Resource(ResourceAccessLevel::Write(id)), Resource(ResourceAccessLevel::Write(id_other))) => *id != id_other,

            (Component(_), Resource(_))
            | (Resource(_), Component(_))
            | (Component(Read(_)), Component(Read(_)))
            | (Component(ReadAll), Component(Read(_)))
            | (Component(Read(_)), Component(ReadAll))
            | (Component(ReadAll), Component(ReadAll))
            | (Resource(ResourceAccessLevel::Read(_)), Resource(ResourceAccessLevel::Read(_)))
            // TODO: I think (FilterdReadAll, FilteredWriteAll) should probably conflict, but should
            // double check with the normal conflict check
            | (Component(FilteredReadAll), _)
            | (_, Component(FilteredReadAll))
            | (Component(FilteredWriteAll), _)
            | (_, Component(FilteredWriteAll))
            | (Component(ReadAllExcept { .. }), Component(Read(_))) 
            | (Component(Read(_)), Component(ReadAllExcept { .. })) 
            | (Component(ReadAllExcept { .. }), Component(ReadAll)) 
            | (Component(ReadAll), Component(ReadAllExcept { .. })) 
            | (Component(ReadAllExcept { .. }), Component(ReadAllExcept { .. }))=> true,

            (Component(ReadAllExcept { component_id: id, .. }), Component(Write(id_other))) |
            (Component(WriteAllExcept { component_id: id, .. }), Component(Read(id_other))) |
            (Component(WriteAllExcept { component_id: id, .. }), Component(Write(id_other))) | 
            (Component(Write(id)), Component(ReadAllExcept { component_id: id_other, .. })) |
            (Component(Read(id)), Component(WriteAllExcept { component_id: id_other, .. })) |
            (Component(Write(id)), Component(WriteAllExcept { component_id: id_other, .. })) => *id == id_other,
            
            (Component(WriteAllExcept { index, .. }), Component(WriteAllExcept { index: index_other, .. })) => *index == index_other, 
        }
    }
}

pub trait AccessIter {
    fn fetch_next(&mut self) -> Option<Option<EcsAccessType>>;
    
    fn chain<U>(self, other: U) -> Chain<Self, U>
        where Self: Sized, U: AccessIter, {
            Chain::new(self, other)
    }
}

pub const fn empty() -> Empty {
    Empty
}

/// An AccessIter that yields nothing.
///
/// This `struct` is created by the [`empty()`] function. See its documentation for more.
pub struct Empty;

impl AccessIter for Empty {
    fn fetch_next(&mut self) -> Option<Option<EcsAccessType>> {
        None
    }
}

pub fn once(value: Option<EcsAccessType>) -> Once {
    Once { inner: Some(value).into_iter() }
}

pub struct Once {
    inner: option::IntoIter<Option<EcsAccessType>>
}

impl AccessIter for Once {
    fn fetch_next(&mut self) -> Option<Option<EcsAccessType>> {
        self.inner.next()
    }
}

pub struct Chain<A, B> {
    a: Option<A>,
    b: Option<B>,
}
impl<A, B> Chain<A, B> {
    pub fn new(a: A, b: B) -> Chain<A, B> {
        Chain { a: Some(a), b: Some(b) }
    }
}

impl<A, B> AccessIter for Chain<A, B> where A: AccessIter, B: AccessIter {
    fn fetch_next(&mut self) -> Option<Option<EcsAccessType>> {
        and_then_or_clear(&mut self.a, A::fetch_next).or_else(|| self.b.as_mut()?.fetch_next())
    }
}

#[inline]
fn and_then_or_clear<T, U>(opt: &mut Option<T>, f: impl FnOnce(&mut T) -> Option<U>) -> Option<U> {
    let x = f(opt.as_mut()?);
    if x.is_none() {
        *opt = None;
    }
    x
}



