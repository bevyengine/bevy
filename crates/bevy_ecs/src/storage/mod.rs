//! Storage layouts for ECS data.

mod blob_vec;
mod sparse_set;
mod table;

pub use blob_vec::*;
pub use sparse_set::*;
pub use table::*;

/// The raw data stores of a [World](crate::world::World)
#[derive(Default)]
pub struct Storages {
    pub sparse_sets: SparseSets,
    pub tables: Tables,
}

impl Storages {
    pub fn shrink_to_fit(&mut self) {
        self.tables.shrink_to_fit();
        self.sparse_sets.shrink_to_fit();
    }
}
