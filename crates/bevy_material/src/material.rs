use crate::render::MeshPipelineKey;
use crate::*;
use alloc::sync::Arc;
use bevy_platform::hash::FixedHasher;
use core::any::{Any, TypeId};
use core::hash::Hash;
use core::hash::{BuildHasher, Hasher};

pub const MATERIAL_BIND_GROUP_INDEX: usize = 3;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ErasedMaterialPipelineKey {
    pub mesh_key: MeshPipelineKey,
    pub material_key: ErasedMaterialKey,
    pub type_id: TypeId,
}

#[derive(Debug)]
pub struct ErasedMaterialKey {
    type_id: TypeId,
    hash: u64,
    value: Box<dyn Any + Send + Sync>,
    vtable: Arc<ErasedMaterialKeyVTable>,
}

#[derive(Debug)]
pub struct ErasedMaterialKeyVTable {
    clone_fn: fn(&dyn Any) -> Box<dyn Any + Send + Sync>,
    partial_eq_fn: fn(&dyn Any, &dyn Any) -> bool,
}

impl ErasedMaterialKey {
    pub fn new<T>(material_key: T) -> Self
    where
        T: Clone + Hash + PartialEq + Send + Sync + 'static,
    {
        let type_id = TypeId::of::<T>();
        let hash = FixedHasher::hash_one(&FixedHasher, &material_key);

        fn clone<T: Clone + Send + Sync + 'static>(any: &dyn Any) -> Box<dyn Any + Send + Sync> {
            Box::new(any.downcast_ref::<T>().unwrap().clone())
        }
        fn partial_eq<T: PartialEq + 'static>(a: &dyn Any, b: &dyn Any) -> bool {
            a.downcast_ref::<T>().unwrap() == b.downcast_ref::<T>().unwrap()
        }

        Self {
            type_id,
            hash,
            value: Box::new(material_key),
            vtable: Arc::new(ErasedMaterialKeyVTable {
                clone_fn: clone::<T>,
                partial_eq_fn: partial_eq::<T>,
            }),
        }
    }

    pub fn to_key<T: Clone + 'static>(&self) -> T {
        debug_assert_eq!(self.type_id, TypeId::of::<T>());
        self.value.downcast_ref::<T>().unwrap().clone()
    }
}

impl PartialEq for ErasedMaterialKey {
    fn eq(&self, other: &Self) -> bool {
        self.type_id == other.type_id
            && (self.vtable.partial_eq_fn)(self.value.as_ref(), other.value.as_ref())
    }
}

impl Eq for ErasedMaterialKey {}

impl Clone for ErasedMaterialKey {
    fn clone(&self) -> Self {
        Self {
            type_id: self.type_id,
            hash: self.hash,
            value: (self.vtable.clone_fn)(self.value.as_ref()),
            vtable: self.vtable.clone(),
        }
    }
}

impl Hash for ErasedMaterialKey {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.type_id.hash(state);
        self.hash.hash(state);
    }
}

impl Default for ErasedMaterialKey {
    fn default() -> Self {
        Self::new(())
    }
}

// pub struct MaterialProperties {

#[derive(Clone, Copy, Default)]
pub enum RenderPhaseType {
    #[default]
    Opaque,
    AlphaMask,
    Transmissive,
    Transparent,
}
