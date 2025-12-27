use bevy_app::{App, Plugin};
use bevy_ecs::{
    component::Component, entity::Entity, name::Name, query::QueryData, reflect::ReflectComponent,
    world::Mut,
};
use bevy_math::{IVec2, UVec2, Vec2};
use bevy_platform::collections::HashMap;
use bevy_reflect::Reflect;
use bevy_transform::components::Transform;

mod commands;
mod entity_tiles;
mod query;
mod storage;

pub use commands::*;
pub use entity_tiles::*;
pub use query::*;
pub use storage::*;

/// Plugin that handles the initialization and updating of tilemap chunks.
/// Adds systems for processing newly added tilemap chunks.
pub struct TilemapPlugin;

impl Plugin for TilemapPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(EntityTilePlugin);
    }
}

#[derive(Component, Clone, Debug, Reflect)]
#[reflect(Component, Clone, Debug)]
#[require(Name::new("Tilemap"), Transform)]
pub struct Tilemap {
    pub chunks: HashMap<IVec2, Entity>,
    pub chunk_size: UVec2,
    pub tile_display_size: UVec2,
}

impl Tilemap {
    pub fn new(chunk_size: UVec2, tile_display_size: UVec2) -> Self {
        Self {
            chunks: HashMap::new(),
            chunk_size,
            tile_display_size,
        }
    }

    /// Get the coordinates of the chunk a given tile is in.
    // TODO: NAME THIS BETTER
    pub fn tile_chunk_position(&self, tile_position: IVec2) -> IVec2 {
        tile_position.div_euclid(
            self.chunk_size
                .try_into()
                .expect("Could not convert chunk size into IVec2"),
        )
    }

    /// Get the coordinates with in a chunk from a tiles global coordinates.
    pub fn tile_relative_position(&self, tile_position: IVec2) -> UVec2 {
        let chunk_size = self
            .chunk_size
            .try_into()
            .expect("Could not convert chunk size into IVec2");
        let mut res = tile_position.rem_euclid(chunk_size);
        if res.x < 0 {
            res.x = chunk_size.x - res.x.abs() - 1;
        }
        if res.y < 0 {
            res.y = chunk_size.y - res.y.abs() - 1;
        }
        res.try_into()
            .expect("Could not convert chunk local position into UVec2")
    }

    pub fn index(&self, tile_coord: IVec2) -> usize {
        let tile_coord = self.tile_relative_position(tile_coord);
        (tile_coord.y * self.chunk_size.x + tile_coord.x) as usize
    }

    //TODO: CORRECT FOR POSITIVE/NEGATIVE REGIONS
    pub fn calculate_tile_transform(&self, tile_position: IVec2) -> Transform {
        Transform::from_xyz(
            // tile position
            tile_position.x as f32
            // times display size for a tile
            * self.tile_display_size.x as f32
            // plus 1/2 the tile_display_size to correct the center
            + self.tile_display_size.x as f32 / 2.
            // minus 1/2 the tilechunk size, in terms of the tile_display_size,
            // to place the 0 at left of tilemapchunk
            - self.tile_display_size.x as f32 * self.chunk_size.x as f32 / 2.,
            // tile position
            tile_position.y as f32
            // times display size for a tile
            * self.tile_display_size.y as f32
            // minus 1/2 the tile_display_size to correct the center
            + self.tile_display_size.y as f32 / 2.
            // plus 1/2 the tilechunk size, in terms of the tile_display_size,
            // to place the 0 at top of tilemapchunk
            - self.tile_display_size.y as f32 * self.chunk_size.y as f32 / 2.,
            0.,
        )
    }

    //TODO: CORRECT FOR POSITIVE/NEGATIVE REGIONS
    pub fn get_tile_coord(&self, tile_position: Vec2) -> IVec2 {
        IVec2::new(
            // tile position
            ((tile_position.x
            // plus 1/2 the tile_display_size to correct the center
            //- self.tile_display_size.x as f32 / 2.
            // minus 1/2 the tilechunk size, in terms of the tile_display_size,
            // to place the 0 at left of tilemapchunk
            + self.tile_display_size.x as f32 * self.chunk_size.x as f32 / 2.)
            // times display size for a tile
            / self.tile_display_size.x as f32) as i32,
            // tile position
            ((tile_position.y
            // minus 1/2 the tile_display_size to correct the center
            //- self.tile_display_size.y as f32 / 2.
            // plus 1/2 the tilechunk size, in terms of the tile_display_size,
            // to place the 0 at top of tilemapchunk
            + self.tile_display_size.y as f32 * self.chunk_size.y as f32 / 2.)
            // times display size for a tile
            / self.tile_display_size.y as f32) as i32,
        )
    }
}

pub trait TileQueryData {
    type Data<'w>;
    type Storage: QueryData;
    type ReadOnly: TileQueryData<
        Storage = <<Self as TileQueryData>::Storage as QueryData>::ReadOnly,
    >;

    fn get_at<'world, 'state>(
        storage: <Self::Storage as QueryData>::Item<'world, 'state>,
        index: usize,
    ) -> Option<Self::Data<'world>>;
}

impl<T: Send + Sync + 'static> TileQueryData for &T {
    type Data<'w> = &'w T;
    type Storage = &'static TileStorage<T>;
    type ReadOnly = Self;

    fn get_at<'world, 'state>(
        storage: <Self::Storage as QueryData>::Item<'world, 'state>,
        index: usize,
    ) -> Option<Self::Data<'world>> {
        storage.tiles.get(index).and_then(Option::as_ref)
    }
}

impl<T: Send + Sync + 'static> TileQueryData for &mut T {
    type Data<'w> = &'w mut T;
    type Storage = &'static mut TileStorage<T>;
    type ReadOnly = &'static T;

    fn get_at<'world: 'world, 'state>(
        mut storage: Mut<'world, TileStorage<T>>,
        index: usize,
    ) -> Option<Self::Data<'world>> {
        storage
            .into_inner()
            .tiles
            .get_mut(index)
            .and_then(Option::as_mut)
    }
}
