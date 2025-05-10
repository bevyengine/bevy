mod tilemap_chunk;
mod tilemap_chunk_material;

use bevy_app::{App, Plugin};
use bevy_asset::Handle;
use bevy_color::Color;
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::{
    component::Component,
    entity::Entity,
    name::Name,
    relationship::{RelationshipSourceCollection, RelationshipTarget},
};
use bevy_image::Image;
use bevy_math::{IRect, IVec2, UVec2};
use bevy_platform::collections::{hash_map::Values, HashMap, HashSet};
use bevy_render::view::Visibility;
use bevy_transform::components::Transform;
use bevy_utils::default;
use derive_more::derive::AsRef;
use lettuces::storage::grid::Grid;
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
pub struct Tilemap {
    chunks: EntitiesMap,

    /// The alpha mode to use for the tilemap
    pub alpha_mode: AlphaMode2d,
}

impl Default for Tilemap {
    fn default() -> Self {
        Self {
            chunks: EntitiesMap::default(),
            alpha_mode: AlphaMode2d::default(),
        }
    }
}

impl RelationshipTarget for Tilemap {
    type Collection = EntitiesMap;
    type Relationship = TilemapChunk;

    const LINKED_SPAWN: bool = false;

    fn collection(&self) -> &Self::Collection {
        &self.chunks
    }

    fn collection_mut_risky(&mut self) -> &mut Self::Collection {
        &mut self.chunks
    }

    fn from_collection_risky(collection: Self::Collection) -> Self {
        Self {
            chunks: collection,
            alpha_mode: AlphaMode2d::default(),
        }
    }
}

#[derive(Clone, Copy, Default, Debug)]
pub struct TileFlip {
    pub x: bool,
    pub y: bool,
    pub d: bool,
}

#[derive(Clone, Copy, Debug, AsRef)]
pub struct TileData {
    pub tileset_index: u16,
    pub color: Color,
    pub visible: bool,
    pub flip: TileFlip,
}

impl Default for TileData {
    fn default() -> Self {
        Self {
            tileset_index: u16::MAX,
            color: Color::WHITE,
            visible: true,
            flip: TileFlip::default(),
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

pub enum TileStorageAccessError {
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
            data: TileStorageData::Dense(Grid::new(size.x as usize, size.y as usize)),
            ..default()
        }
    }

    pub fn set_chunk_size(&mut self, chunk_size: UVec2) {
        self.chunk_size = chunk_size.as_ivec2();
    }

    pub fn chunk_size(&self) -> UVec2 {
        self.chunk_size.as_uvec2()
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

    pub fn set(&mut self, tile_position: IVec2, tile_data: Option<TileData>) {
        match &mut self.data {
            TileStorageData::Sparse(tiles) => {
                if let Some(tile_data) = tile_data {
                    tiles.insert(tile_position, tile_data);
                } else {
                    tiles.remove(&tile_position);
                }
            }
            TileStorageData::Dense(tiles) => {
                let Some(tile) = tiles.get_mut(tile_position.x as usize, tile_position.y as usize)
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
            TileStorageData::Dense(tiles) => {
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

    pub fn clear_dirty_chunk_positions(&mut self, chunk_positions: HashSet<IVec2>) {
        self.dirty_chunk_positions
            .retain(|chunk_position| !chunk_positions.contains(chunk_position));
    }

    pub fn iter_dirty_chunk_positions(&self) -> impl Iterator<Item = &IVec2> {
        self.dirty_chunk_positions.iter()
    }

    pub fn rect_tiles(
        &self,
        rect: IRect,
    ) -> Result<Vec<Option<&TileData>>, TileStorageAccessError> {
        let IRect { min, max } = rect;
        match &self.data {
            TileStorageData::Sparse(tiles) => {
                let mut result = Vec::with_capacity(rect.size().element_product() as usize);
                for y in min.y..max.y {
                    for x in min.x..max.x {
                        result.push(tiles.get(&IVec2::new(x, y)));
                    }
                }
                Ok(result)
            }
            TileStorageData::Dense(tiles) => {
                if min.x < 0
                    || min.y < 0
                    || max.x >= tiles.cols() as i32
                    || max.y >= tiles.rows() as i32
                {
                    return Err(TileStorageAccessError::OutOfBounds { x: min.x, y: min.y });
                }
                let mut result = Vec::with_capacity(rect.size().element_product() as usize);
                for y in min.y..max.y {
                    for x in min.x..max.x {
                        result.push(tiles[(x as usize, y as usize)].as_ref());
                    }
                }
                Ok(result)
            }
        }
    }

    pub fn chunk_tiles(
        &self,
        chunk_position: IVec2,
    ) -> Result<Vec<Option<&TileData>>, TileStorageAccessError> {
        let chunk_size = self.chunk_size;
        let chunk_rect = IRect::from_corners(
            chunk_position * chunk_size,
            (chunk_position + IVec2::splat(1)) * chunk_size,
        );

        self.rect_tiles(chunk_rect)
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

#[derive(Default, Clone, Deref, DerefMut)]
pub struct EntitiesMap(HashMap<IVec2, Entity>);

impl RelationshipSourceCollection for EntitiesMap {
    type SourceIter<'a> = std::iter::Copied<Values<'a, IVec2, Entity>>;

    fn new() -> Self {
        Self::default()
    }

    fn with_capacity(capacity: usize) -> Self {
        Self(HashMap::with_capacity(capacity))
    }

    fn reserve(&mut self, additional: usize) {
        self.0.reserve(additional);
    }

    fn add(&mut self, _: Entity) -> bool {
        false
    }

    fn remove(&mut self, _: Entity) -> bool {
        false
    }

    fn iter(&self) -> Self::SourceIter<'_> {
        self.0.values().copied()
    }

    fn len(&self) -> usize {
        self.0.len()
    }

    fn shrink_to_fit(&mut self) {
        self.0.shrink_to_fit();
    }

    fn clear(&mut self) {
        self.0.clear();
    }
}
