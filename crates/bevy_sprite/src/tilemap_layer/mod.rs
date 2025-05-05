mod tilemap_chunk;
mod tilemap_chunk_material;

use bevy_app::{App, Plugin};
use bevy_asset::Handle;
use bevy_color::Color;
use bevy_ecs::{component::Component, entity::Entity, name::Name};
use bevy_image::Image;
use bevy_math::{IRect, IVec2, URect, UVec2};
use bevy_platform::collections::{HashMap, HashSet};
use bevy_render::view::Visibility;
use bevy_transform::components::Transform;
use bevy_utils::default;
use derive_more::derive::AsRef;
pub use tilemap_chunk::*;
pub use tilemap_chunk_material::*;
use tracing::warn;

use crate::AlphaMode2d;

/// Plugin that adds support for tilemap chunk materials.
#[derive(Default)]
pub struct TilemapPlugin;

impl Plugin for TilemapPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(TilemapChunkPlugin)
            .add_plugins(TilemapChunkMaterialPlugin);
    }
}

/// A component representing a tileset image containing all tile textures.
#[derive(Component, Clone, Debug, Default)]
pub struct Tileset {
    pub image: Handle<Image>,
    pub tile_size: UVec2,
}

#[derive(Component, Clone)]
#[require(TileStorage, Tileset, Name::new("Tilemap"), Transform, Visibility)]
pub struct TilemapLayer {
    chunks: HashMap<IVec2, Entity>,

    /// The alpha mode to use for the tilemap
    pub alpha_mode: AlphaMode2d,
}

impl Default for TilemapLayer {
    fn default() -> Self {
        Self {
            chunks: HashMap::new(),
            alpha_mode: AlphaMode2d::Blend,
        }
    }
}

#[derive(Clone, Copy, Debug, AsRef)]
pub struct TileData {
    pub tileset_index: u16,
    pub color: Color,
    pub visible: bool,
}

impl Default for TileData {
    fn default() -> Self {
        Self {
            tileset_index: u16::MAX,
            color: Color::WHITE,
            visible: true,
        }
    }
}

impl TileData {
    pub fn from_index(tileset_index: u16) -> Self {
        Self {
            tileset_index,
            ..default()
        }
    }
}

pub enum TileStorageError {
    OutOfBounds { x: i32, y: i32 },
}

#[derive(Component)]
pub struct TileStorage {
    data: TileStorageData,
    chunk_size: IVec2,
    dirty_chunk_positions: HashSet<IVec2>,
}

impl Default for TileStorage {
    fn default() -> Self {
        Self {
            data: TileStorageData::Sparse(HashMap::new()),
            chunk_size: IVec2::splat(32),
            dirty_chunk_positions: HashSet::new(),
        }
    }
}

impl TileStorage {
    pub fn new() -> Self {
        Self::sparse()
    }

    pub fn sparse() -> Self {
        Self {
            data: TileStorageData::Sparse(HashMap::new()),
            ..default()
        }
    }

    pub fn dense(size: UVec2) -> Self {
        Self {
            data: TileStorageData::Dense {
                tiles: vec![None; size.x as usize * size.y as usize],
                size,
            },
            ..default()
        }
    }

    pub fn set_chunk_size(&mut self, chunk_size: UVec2) {
        self.chunk_size = chunk_size.as_ivec2();
    }

    pub fn chunk_size(&self) -> UVec2 {
        self.chunk_size.as_uvec2()
    }

    pub fn get(&self, tile_position: IVec2) -> Result<Option<&TileData>, TileStorageError> {
        match &self.data {
            TileStorageData::Sparse(tiles) => Ok(tiles.get(&tile_position)),
            TileStorageData::Dense { tiles, size } => {
                let Some(tile) = tiles
                    .get(tile_position.x as usize + tile_position.y as usize * size.x as usize)
                else {
                    return Err(TileStorageError::OutOfBounds {
                        x: tile_position.x,
                        y: tile_position.y,
                    });
                };
                Ok(tile.as_ref())
            }
        }
    }

    pub fn set(&mut self, tile_position: IVec2, tile_data: Option<TileData>) {
        match &mut self.data {
            TileStorageData::Sparse(tiles) => {
                if let Some(tile_data) = tile_data {
                    tiles.insert(tile_position, tile_data);
                } else {
                    tiles.remove(&tile_position);
                }
            }
            TileStorageData::Dense { tiles, size } => {
                let Some(tile) = tiles
                    .get_mut(tile_position.x as usize + tile_position.y as usize * size.x as usize)
                else {
                    return;
                };
                *tile = tile_data;
            }
        }

        self.dirty_chunk_positions
            .insert(tile_position.div_euclid(self.chunk_size));
    }

    pub fn fill_rect_with<F>(&mut self, rect: IRect, mut f: F)
    where
        F: FnMut(IVec2) -> Option<TileData>,
    {
        for y in rect.min.y..rect.max.y {
            for x in rect.min.x..rect.max.x {
                self.set(IVec2::new(x, y), f(IVec2::new(x, y)));
            }
        }
    }

    pub fn fill_rect(&mut self, rect: IRect, tile_data: Option<TileData>) {
        self.fill_rect_with(rect, |_| tile_data);
    }

    pub fn fill(&mut self, tile_data: Option<TileData>) {
        match &mut self.data {
            TileStorageData::Sparse(_) => {
                warn!("TileStorage::fill is not supported for sparse tile storage");
            }
            TileStorageData::Dense { tiles, .. } => {
                tiles.fill(tile_data);
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
            TileStorageData::Dense { tiles, .. } => {
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
            TileStorageData::Dense { size, .. } => {
                let size_in_chunks = size.as_ivec2().div_euclid(self.chunk_size);
                for y in 0..size_in_chunks.y {
                    for x in 0..size_in_chunks.x {
                        self.dirty_chunk_positions.insert(IVec2::new(x, y));
                    }
                }
            }
        }
    }

    pub fn clear_dirty_chunk_positions(&mut self, chunk_positions: HashSet<IVec2>) {
        self.dirty_chunk_positions
            .retain(|chunk_position| !chunk_positions.contains(chunk_position));
    }

    pub fn iter_dirty_chunk_positions(&self) -> impl Iterator<Item = &IVec2> {
        self.dirty_chunk_positions.iter()
    }

    pub fn iter_sub_rect(
        &self,
        rect: IRect,
    ) -> Result<impl Iterator<Item = Option<&TileData>>, TileStorageError> {
        let IRect { min, max } = rect;
        match &self.data {
            TileStorageData::Sparse(tiles) => {
                let mut result = Vec::with_capacity(rect.size().element_product() as usize);
                for y in min.y..max.y {
                    for x in min.x..max.x {
                        result.push(tiles.get(&IVec2::new(x, y)));
                    }
                }
                Ok(result.into_iter())
            }
            TileStorageData::Dense { tiles, size } => {
                if min.x < 0 || min.y < 0 || max.x >= size.x as i32 || max.y >= size.y as i32 {
                    return Err(TileStorageError::OutOfBounds { x: min.x, y: min.y });
                }
                let rect: URect = rect.as_urect();
                let width = rect.size().x as usize;
                Ok((rect.min.y..rect.max.y)
                    .flat_map(move |row| {
                        let start = row as usize * size.x as usize + rect.min.x as usize;
                        let end = start + width;
                        tiles[start..end].iter().map(|opt| opt.as_ref())
                    })
                    .collect::<Vec<_>>()
                    .into_iter())
            }
        }
    }

    pub fn iter_chunk_tiles(
        &self,
        chunk_position: IVec2,
    ) -> Result<impl Iterator<Item = Option<&TileData>>, TileStorageError> {
        let chunk_size = self.chunk_size;
        let chunk_rect = IRect::from_corners(
            chunk_position * chunk_size,
            (chunk_position + IVec2::splat(1)) * chunk_size,
        );

        self.iter_sub_rect(chunk_rect)
    }
}

pub enum TileStorageData {
    Sparse(HashMap<IVec2, TileData>),
    Dense {
        tiles: Vec<Option<TileData>>,
        size: UVec2,
    },
}

impl Default for TileStorageData {
    fn default() -> Self {
        Self::Sparse(HashMap::new())
    }
}
