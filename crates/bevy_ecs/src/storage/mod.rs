//! Storage layouts for ECS data.

mod blob_vec;
mod sparse_set;
mod table;

pub use blob_vec::*;
pub use sparse_set::*;
pub use table::*;

#[derive(Default)]
pub struct Storages {
    pub sparse_sets: SparseSets,
    pub tables: Tables,
}
