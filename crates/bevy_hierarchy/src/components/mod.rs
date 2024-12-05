mod children;
mod parent;

use bevy_ecs::world::{EntityWorldMut, Mut};

pub use children::Children;
pub use parent::Parent;

/// Private trait to consolidate unsafe calls to [`get_mut_assume_mutable`](EntityWorldMut::get_mut_assume_mutable).
pub(crate) trait GetHierarchyComponentsMut {
    fn get_parent_mut(&mut self) -> Option<Mut<Parent>>;
    fn get_children_mut(&mut self) -> Option<Mut<Children>>;
}

#[expect(unsafe_code)]
impl GetHierarchyComponentsMut for EntityWorldMut<'_> {
    fn get_parent_mut(&mut self) -> Option<Mut<Parent>> {
        unsafe { self.get_mut_assume_mutable() }
    }

    fn get_children_mut(&mut self) -> Option<Mut<Children>> {
        unsafe { self.get_mut_assume_mutable() }
    }
}
