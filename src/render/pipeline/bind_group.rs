use super::BindingDescriptor;
use std::{
    collections::{hash_map::DefaultHasher, BTreeSet},
    hash::{Hash, Hasher},
};

#[derive(Clone, Debug)]
pub struct BindGroupDescriptor {
    pub index: u32,
    pub bindings: BTreeSet<BindingDescriptor>,
    hash: Option<BindGroupDescriptorId>,
}

#[derive(Hash, Copy, Clone, Eq, PartialEq, Debug)]
pub struct BindGroupDescriptorId(u64);

impl BindGroupDescriptor {
    pub fn new(index: u32, bindings: Vec<BindingDescriptor>) -> Self {
        BindGroupDescriptor {
            index,
            bindings: bindings.iter().cloned().collect(),
            hash: None,
        }
    }

    pub fn get_id(&self) -> Option<BindGroupDescriptorId> {
        self.hash
    }

    pub fn get_or_update_id(&mut self) -> BindGroupDescriptorId {
        if self.hash.is_none() {
            self.update_id();
        }

        self.hash.unwrap()
    }

    pub fn update_id(&mut self) {
        let mut hasher = DefaultHasher::new();
        self.hash(&mut hasher);
        self.hash = Some(BindGroupDescriptorId(hasher.finish()));
    }
}

impl Hash for BindGroupDescriptor {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.index.hash(state);
        self.bindings.hash(state);
    }
}

impl PartialEq for BindGroupDescriptor {
    fn eq(&self, other: &BindGroupDescriptor) -> bool {
        self.index == other.index && self.bindings == other.bindings
    }
}

impl Eq for BindGroupDescriptor {}
