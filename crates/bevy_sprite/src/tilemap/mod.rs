mod tilemap_chunk;
mod tilemap_chunk_material;

use bevy_app::{App, Plugin};
use bevy_color::Color;
use bevy_ecs::component::Component;
use bevy_math::{IVec2, UVec2};
use bevy_platform::collections::{HashMap, HashSet};
use derive_more::derive::AsRef;
use lettuces::storage::grid::Grid;
pub use tilemap_chunk::*;
pub use tilemap_chunk_material::*;
use tracing::warn;

/// Plugin that adds support for tilemap chunk materials.
#[derive(Default)]
pub struct TilemapPlugin;

impl Plugin for TilemapPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(TilemapChunkPlugin)
            .add_plugins(TilemapChunkMaterialPlugin);
    }
}

#[derive(Clone, Copy)]
pub struct TileFlip {
    pub x: bool,
    pub y: bool,
    pub d: bool,
}

#[derive(Clone, Copy, AsRef)]
pub struct TileData {
    tileset_index: u16,
    color: Color,
    visible: bool,
    flip: TileFlip,
}

#[derive(Default, Component)]
pub struct TileStorage {
    data: TileStorageData,
    chunk_size: IVec2,
    dirty_chunk_positions: HashSet<IVec2>,
}

pub enum TileStorageAccessError {
    OutOfBounds { x: i32, y: i32 },
}

impl TileStorage {
    pub fn new(chunk_size: UVec2) -> Self {
        Self::sparse(chunk_size)
    }

    pub fn sparse(chunk_size: UVec2) -> Self {
        Self {
            data: TileStorageData::Sparse(HashMap::new()),
            chunk_size: chunk_size.as_ivec2(),
            dirty_chunk_positions: HashSet::new(),
        }
    }

    pub fn dense(size: UVec2, chunk_size: UVec2) -> Self {
        Self {
            data: TileStorageData::Dense(Grid::new(size.x as usize, size.y as usize)),
            chunk_size: chunk_size.as_ivec2(),
            dirty_chunk_positions: HashSet::new(),
        }
    }

    pub fn get(&self, tile_position: IVec2) -> Result<Option<&TileData>, TileStorageAccessError> {
        match &self.data {
            TileStorageData::Sparse(tiles) => Ok(tiles.get(&tile_position)),
            TileStorageData::Dense(tiles) => {
                let Some(tile) = tiles.get(tile_position.x as usize, tile_position.y as usize)
                else {
                    return Err(TileStorageAccessError::OutOfBounds {
                        x: tile_position.x,
                        y: tile_position.y,
                    });
                };
                Ok(tile.as_ref())
            }
        }
    }

    pub fn set(&mut self, tile_position: IVec2, tile_data: TileData) {
        match &mut self.data {
            TileStorageData::Sparse(tiles) => {
                tiles.insert(tile_position, tile_data);
            }
            TileStorageData::Dense(tiles) => {
                let Some(tile) = tiles.get_mut(tile_position.x as usize, tile_position.y as usize)
                else {
                    return;
                };
                *tile = Some(tile_data);
            }
        }

        self.dirty_chunk_positions
            .insert(tile_position.div_euclid(self.chunk_size));
    }

    pub fn fill(&mut self, tile_data: TileData) {
        match &mut self.data {
            TileStorageData::Sparse(_) => {
                warn!("TileStorage::fill is not supported for sparse tile storage");
            }
            TileStorageData::Dense(tiles) => {
                tiles.fill(Some(tile_data));
                self.set_all_dirty();
            }
        }
    }

    pub fn fill_with<F>(&mut self, f: F)
    where
        F: FnMut() -> Option<TileData>,
    {
        match &mut self.data {
            TileStorageData::Sparse(_) => {
                warn!("TileStorage::fill_with is not supported for sparse tile storage");
            }
            TileStorageData::Dense(tiles) => {
                tiles.fill_with(f);
                self.set_all_dirty();
            }
        }
    }

    pub fn set_all_dirty(&mut self) {
        match &mut self.data {
            TileStorageData::Sparse(_) => {
                warn!("TileStorage::set_all_dirty is not supported for sparse tile storage");
            }
            TileStorageData::Dense(tiles) => {
                let (width, height) = tiles.size();
                let size_in_chunks =
                    IVec2::new(width as i32, height as i32).div_euclid(self.chunk_size);
                for y in 0..size_in_chunks.y {
                    for x in 0..size_in_chunks.x {
                        self.dirty_chunk_positions.insert(IVec2::new(x, y));
                    }
                }
            }
        }
    }

    pub fn clear_dirty_chunk_positions(&mut self) {
        self.dirty_chunk_positions.clear();
    }

    pub fn iter_dirty_chunk_positions(&self) -> impl Iterator<Item = &IVec2> {
        self.dirty_chunk_positions.iter()
    }
}

pub enum TileStorageData {
    Sparse(HashMap<IVec2, TileData>),
    Dense(Grid<Option<TileData>>),
}

impl Default for TileStorageData {
    fn default() -> Self {
        Self::Sparse(HashMap::new())
    }
}
