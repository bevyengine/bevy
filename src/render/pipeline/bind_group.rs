use super::Binding;
use std::{
    collections::{hash_map::DefaultHasher, BTreeSet},
    hash::{Hash, Hasher},
};

#[derive(Clone, Debug)]
pub struct BindGroup {
    pub index: u32,
    pub bindings: BTreeSet<Binding>,
    hash: Option<u64>,
}

impl BindGroup {
    pub fn new(index: u32, bindings: Vec<Binding>) -> Self {
        BindGroup {
            index,
            bindings: bindings.iter().cloned().collect(),
            hash: None,
        }
    }

    pub fn get_hash(&self) -> Option<u64> {
        self.hash
    }

    pub fn get_or_update_hash(&mut self) -> u64 {
        if self.hash.is_none() {
            self.update_hash();
        }

        self.hash.unwrap()
    }

    pub fn update_hash(&mut self) {
        let mut hasher = DefaultHasher::new();
        self.hash(&mut hasher);
        self.hash = Some(hasher.finish());
    }
}

impl Hash for BindGroup {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.index.hash(state);
        self.bindings.hash(state);
    }
}

impl PartialEq for BindGroup {
    fn eq(&self, other: &BindGroup) -> bool {
        self.index == other.index && self.bindings == other.bindings
    }
}

impl Eq for BindGroup {}
