use bevy_ecs::{
    entity::Entity,
    query::{QueryData, QueryFilter, With},
    system::{Query, SystemParam},
};
use bevy_math::IVec2;

use crate::{EntityTile, TileCoord, TileQueryData, TileStorage, TileStorages, Tilemap};

/// Query for looking up tilemaps.
/// Contains a nested query for [`Tilemap`] entities and Chunk entitites.
#[derive(SystemParam)]
pub struct TilemapQuery<'w, 's, D, F = ()>
where
    D: TileQueryData + 'static,
    F: QueryFilter + 'static,
{
    chunks: Query<'w, 's, <D as TileQueryData>::Storage, (F, With<TileStorages>)>,
    maps: Query<'w, 's, &'static Tilemap>,
}

/// Query for looking up tiles in a tilemap.
/// Contains a nested query for a [`Tilemap`] entity and Chunk entitites.
pub struct TileQuery<'world, 'state, D, F = ()>
where
    D: TileQueryData + 'static,
    F: QueryFilter + 'static,
{
    chunks: Query<'world, 'state, <D as TileQueryData>::Storage, (F, With<TileStorages>)>,
    pub map: &'world Tilemap,
}

impl<'world, 'state, D, F> TilemapQuery<'world, 'state, D, F>
where
    D: TileQueryData + 'static,
    F: QueryFilter + 'static,
{
    /// Gets the query for a given map.
    pub fn get_map<'a>(&'a self, map_id: Entity) -> Option<TileQuery<'a, 'state, D::ReadOnly, F>> {
        let map = self.maps.get(map_id).ok()?;

        Some(TileQuery {
            chunks: self.chunks.as_readonly(),
            map,
        })
    }

    /// Gets the query for a given map.
    pub fn get_map_mut<'a>(&'a mut self, map_id: Entity) -> Option<TileQuery<'a, 'state, D, F>> {
        let map = self.maps.get(map_id).ok()?;

        Some(TileQuery {
            chunks: self.chunks.reborrow(),
            map,
        })
    }
}

impl<'world, 'state, D, F> TileQuery<'world, 'state, D, F>
where
    D: TileQueryData + 'static,
    F: QueryFilter + 'static,
{
    /// Get the readonly variant of this query.
    pub fn as_readonly(&self) -> TileQuery<'_, 'state, D::ReadOnly, F> {
        TileQuery {
            chunks: self.chunks.as_readonly(),
            map: self.map,
        }
    }

    /// Get the readonly variant of this query.
    pub fn reborrow(&mut self) -> TileQuery<'_, 'state, D, F> {
        TileQuery {
            chunks: self.chunks.reborrow(),
            map: self.map,
        }
    }

    /// Get's the readonly query item for the given tile.
    #[inline]
    pub fn get_at(
        &self,
        coord: IVec2,
    ) -> Option<<<D as TileQueryData>::ReadOnly as TileQueryData>::Data<'_>> {
        let chunk_coord = self.map.tile_chunk_position(coord);
        let chunk_entity = self.map.chunks.get(&chunk_coord).cloned()?;

        let Ok(storages) = self.chunks.get(chunk_entity) else {
            return None;
        };

        let index = self.map.index(coord);

        <<D as TileQueryData>::ReadOnly as TileQueryData>::get_at(storages, index)
    }

    /// Get's the mutable query item for the given tile.
    #[inline]
    pub fn get_at_mut(&mut self, coord: IVec2) -> Option<<D as TileQueryData>::Data<'_>> {
        let chunk_coord = self.map.tile_chunk_position(coord);
        let chunk_entity = self.map.chunks.get(&chunk_coord).cloned()?;

        let Ok(storages) = self.chunks.get_mut(chunk_entity) else {
            return None;
        };

        let index = self.map.index(coord);

        <D as TileQueryData>::get_at(storages, index)
    }
}

/// Query for looking up tilemaps.
/// Contains a nested query for [`Tilemap`] entities and Chunk entitites.
#[derive(SystemParam)]
pub struct TilemapEntityQuery<'w, 's, D, F = ()>
where
    D: QueryData + 'static,
    F: QueryFilter + 'static,
{
    tiles: Query<'w, 's, D, (F, With<TileCoord>)>,
    chunks: Query<'w, 's, &'static TileStorage<EntityTile>, With<TileStorages>>,
    maps: Query<'w, 's, &'static Tilemap>,
}

/// Query for looking up tiles in a tilemap.
/// Contains a nested query for a [`Tilemap`] entity and Chunk entitites.
pub struct TileEntityQuery<'w, 's, D, F = ()>
where
    D: QueryData + 'static,
    F: QueryFilter + 'static,
{
    tiles: Query<'w, 's, D, (F, With<TileCoord>)>,
    chunks: Query<'w, 's, &'static TileStorage<EntityTile>, With<TileStorages>>,
    pub map: &'w Tilemap,
}

impl<'world, 'state, D, F> TilemapEntityQuery<'world, 'state, D, F>
where
    D: QueryData + 'static,
    F: QueryFilter + 'static,
{
    /// Gets the query for a given map.
    pub fn get_map<'a>(
        &'a self,
        map_id: Entity,
    ) -> Option<TileEntityQuery<'a, 'state, D::ReadOnly, F>> {
        let map = self.maps.get(map_id).ok()?;

        Some(TileEntityQuery {
            tiles: self.tiles.as_readonly(),
            chunks: self.chunks.as_readonly(),
            map,
        })
    }

    /// Gets the query for a given map.
    pub fn get_map_mut<'a>(
        &'a mut self,
        map_id: Entity,
    ) -> Option<TileEntityQuery<'a, 'state, D, F>> {
        let map = self.maps.get(map_id).ok()?;

        Some(TileEntityQuery {
            tiles: self.tiles.reborrow(),
            chunks: self.chunks.reborrow(),
            map,
        })
    }
}

impl<'world, 'state, D, F> TileEntityQuery<'world, 'state, D, F>
where
    D: QueryData + 'static,
    F: QueryFilter + 'static,
{
    /// Get the readonly variant of this query.
    pub fn as_readonly(&self) -> TileEntityQuery<'_, 'state, D::ReadOnly, F> {
        TileEntityQuery {
            tiles: self.tiles.as_readonly(),
            chunks: self.chunks.as_readonly(),
            map: self.map,
        }
    }

    /// Get the readonly variant of this query.
    pub fn reborrow(&mut self) -> TileEntityQuery<'_, 'state, D, F> {
        TileEntityQuery {
            tiles: self.tiles.reborrow(),
            chunks: self.chunks.reborrow(),
            map: self.map,
        }
    }

    /// Get's the readonly query item for the given tile.
    #[inline]
    pub fn get_at(
        &self,
        coord: IVec2,
    ) -> Option<<<D as QueryData>::ReadOnly as QueryData>::Item<'_, 'state>> {
        let chunk_coord = self.map.tile_chunk_position(coord);
        let chunk_entity = self.map.chunks.get(&chunk_coord).cloned()?;

        let storages = self.chunks.get(chunk_entity).ok()?;

        let index = self.map.index(coord);

        let entity =
            <<&EntityTile as TileQueryData>::ReadOnly as TileQueryData>::get_at(storages, index)?;

        self.tiles.get(**entity).ok()
    }

    /// Get's the mutable query item for the given tile.
    #[inline]
    pub fn get_at_mut(&mut self, coord: IVec2) -> Option<<D as QueryData>::Item<'_, 'state>> {
        let chunk_coord = self.map.tile_chunk_position(coord);
        let chunk_entity = self.map.chunks.get(&chunk_coord).cloned()?;

        let Ok(storages) = self.chunks.get_mut(chunk_entity) else {
            return None;
        };

        let index = self.map.index(coord);

        let entity =
            <<&EntityTile as TileQueryData>::ReadOnly as TileQueryData>::get_at(storages, index)?;

        self.tiles.get_mut(**entity).ok()
    }
}
