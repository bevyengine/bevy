use bevy_app::{App, Plugin};
use bevy_ecs::{component::Component, entity::Entity, name::Name, reflect::ReflectComponent};
use bevy_math::{IVec2, UVec2};
use bevy_platform::collections::HashMap;
use bevy_reflect::Reflect;
use bevy_transform::components::Transform;

mod commands;
mod storage;

pub use commands::*;
pub use storage::*;

/// Plugin that handles the initialization and updating of tilemap chunks.
/// Adds systems for processing newly added tilemap chunks.
pub struct TilemapPlugin;

impl Plugin for TilemapPlugin {
    fn build(&self, app: &mut App) {
        //app.add_plugins(TilemapChunkPlugin).add_plugins(TilePlugin);
    }
}

#[derive(Component, Clone, Debug, Reflect)]
#[reflect(Component, Clone, Debug)]
#[require(Name::new("TilemapLayer"), Transform)]
pub struct Tilemap {
    pub chunks: HashMap<IVec2, Entity>,
    pub chunk_size: UVec2,
    pub tile_display_size: UVec2,
}

impl Tilemap {
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
    // TODO: NAME THIS BETTER
    pub fn tile_chunk_local_position(&self, tile_position: IVec2) -> UVec2 {
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
}

pub trait TileData: Send + Sync + 'static {}
