//! A trait for components that let you traverse the ECS

use crate::{
    component::{Component, StorageType},
    entity::Entity,
};

/// A component that holds a pointer to another entity.
///
/// The implementor is responsible for ensuring that `Traversal::next` cannot produce infinite loops.
pub trait Traversal: Component {
    /// Returns the next entity to visit.
    fn next(&self) -> Option<Entity>;
}

/// A traversial component that dosn't traverse anything. Used to provide a default traversal
/// implementation for events.
///
/// It is not possible to actually construct an instance of this component.
pub struct TraverseNone {
    _private: (),
}

impl Traversal for TraverseNone {
    #[inline(always)]
    fn next(&self) -> Option<Entity> {
        None
    }
}

impl Component for TraverseNone {
    const STORAGE_TYPE: StorageType = StorageType::Table;
}
