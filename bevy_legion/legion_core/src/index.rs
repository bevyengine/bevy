use crate::entity::Entity;
use crate::storage::ArchetypeData;
use crate::storage::Chunkset;
use crate::storage::ComponentStorage;
use std::fmt;
use std::ops::Deref;
use std::ops::Index;
use std::ops::IndexMut;

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub struct SetIndex(pub usize);

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub struct ChunkIndex(pub usize);

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub struct ArchetypeIndex(pub usize);

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub struct ComponentIndex(pub usize);

macro_rules! impl_index {
    ($index_ty:ty: $output_ty:ty) => {
        impl Index<$index_ty> for [$output_ty] {
            type Output = $output_ty;
            #[inline(always)]
            fn index(&self, index: $index_ty) -> &Self::Output { &self[index.0] }
        }
        impl IndexMut<$index_ty> for [$output_ty] {
            #[inline(always)]
            fn index_mut(&mut self, index: $index_ty) -> &mut Self::Output { &mut self[index.0] }
        }
        impl Index<$index_ty> for Vec<$output_ty> {
            type Output = $output_ty;
            #[inline(always)]
            fn index(&self, index: $index_ty) -> &Self::Output { &self[index.0] }
        }
        impl IndexMut<$index_ty> for Vec<$output_ty> {
            #[inline(always)]
            fn index_mut(&mut self, index: $index_ty) -> &mut Self::Output { &mut self[index.0] }
        }
        impl Deref for $index_ty {
            type Target = usize;
            #[inline(always)]
            fn deref(&self) -> &usize { &self.0 }
        }
        impl fmt::Display for $index_ty {
            #[inline]
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                fmt::Display::fmt(&**self, f)
            }
        }
    };
}

impl_index!(SetIndex: Chunkset);
impl_index!(ChunkIndex: ComponentStorage);
impl_index!(ArchetypeIndex: ArchetypeData);
impl_index!(ComponentIndex: Entity);
