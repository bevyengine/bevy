use bevy_ecs::{
    query::{QueryData, QueryFilter, With},
    system::Query,
};

use crate::{TileData, TileStorages, Tilemap};

/// Query for looking up tile information in a tilemap.
/// Contains a nested query for a TileMap entity and Chunk entitites.
pub struct TileMapQuery<'world, 'state, D, F = ()>
where
    D: TileData,
    F: QueryFilter,
{
    chunks: Query<'world, 'state, D, (F, With<TileStorages>)>,
    maps: Query<'world, 'state, &'static Tilemap>,
}
