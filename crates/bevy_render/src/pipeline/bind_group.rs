use super::BindingDescriptor;
use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BindGroupDescriptor {
    pub index: u32,
    pub bindings: Vec<BindingDescriptor>,
    pub id: BindGroupDescriptorId,
}

#[derive(Hash, Copy, Clone, Eq, PartialEq, Debug)]
pub struct BindGroupDescriptorId(u64);

impl BindGroupDescriptor {
    pub fn new(index: u32, bindings: Vec<BindingDescriptor>) -> Self {
        let mut descriptor = BindGroupDescriptor {
            index,
            bindings,
            id: BindGroupDescriptorId(0),
        };

        // TODO: remove all instances of get_or_update_id
        descriptor.update_id();
        descriptor
    }

    pub fn update_id(&mut self) {
        let mut hasher = DefaultHasher::new();
        self.hash(&mut hasher);
        self.id = BindGroupDescriptorId(hasher.finish());
    }
}

impl Hash for BindGroupDescriptor {
    fn hash<H: Hasher>(&self, state: &mut H) {
        // TODO: remove index from hash state (or at least id). index is not considered a part of a bind group on the gpu.
        // bind groups are bound to indices in pipelines
        self.index.hash(state);
        self.bindings.hash(state);
    }
}
