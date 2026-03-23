use bevy_platform::{hash::FixedHasher, sync::Arc};
use core::{
    any::{Any, TypeId},
    hash::{BuildHasher, Hash, Hasher},
};

/// A type-erased mesh pipeline key, which stores the bits of the key as a `u64`.
#[derive(Clone, Copy)]
pub struct ErasedMeshPipelineKey {
    bits: u64,
    type_id: TypeId,
}

impl ErasedMeshPipelineKey {
    #[inline]
    pub fn new<T: 'static>(key: T) -> Self
    where
        u64: From<T>,
    {
        Self {
            bits: key.into(),
            type_id: TypeId::of::<T>(),
        }
    }

    #[inline]
    pub fn downcast<T: 'static + From<u64>>(&self) -> T {
        assert_eq!(
            self.type_id,
            TypeId::of::<T>(),
            "ErasedMeshPipelineKey::downcast called with wrong type"
        );
        self.bits.into()
    }
}

impl PartialEq for ErasedMeshPipelineKey {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.type_id == other.type_id && self.bits == other.bits
    }
}

impl Eq for ErasedMeshPipelineKey {}

impl Hash for ErasedMeshPipelineKey {
    #[inline]
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.type_id.hash(state);
        self.bits.hash(state);
    }
}

impl Default for ErasedMeshPipelineKey {
    fn default() -> Self {
        Self {
            bits: 0,
            type_id: TypeId::of::<()>(),
        }
    }
}

impl core::fmt::Debug for ErasedMeshPipelineKey {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("ErasedMeshPipelineKey")
            .field("type_id", &self.type_id)
            .field("bits", &format_args!("{:#018x}", self.bits))
            .finish()
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ErasedMaterialPipelineKey {
    pub mesh_key: ErasedMeshPipelineKey,
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
