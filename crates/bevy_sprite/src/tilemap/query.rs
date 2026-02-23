use bevy_ecs::{
    entity::Entity,
    query::{QueryData, QueryFilter, With},
    system::{Query, SystemParam},
};
use bevy_math::{IRect, IVec2};

use crate::{EntityTile, TileCoord, TileQueryData, TileStorage, TileStorages, Tilemap};

/// Query for looking up tilemaps.
/// Contains a nested query for [`Tilemap`] entities and Chunk entities.
#[derive(SystemParam)]
pub struct TilemapQuery<'w, 's, D>
where
    D: TileQueryData + 'static,
{
    chunks: Query<'w, 's, <D as TileQueryData>::Storage, With<TileStorages>>,
    maps: Query<'w, 's, &'static Tilemap>,
}

/// Query for looking up tiles in a tilemap.
/// Contains a nested query for a [`Tilemap`] entity and Chunk entities.
pub struct TileQuery<'world, 'state, D>
where
    D: TileQueryData + 'static,
{
    chunks: Query<'world, 'state, <D as TileQueryData>::Storage, With<TileStorages>>,
    pub map: &'world Tilemap,
}

impl<'world, 'state, D> TilemapQuery<'world, 'state, D>
where
    D: TileQueryData + 'static,
{
    /// Gets the query for a given map.
    pub fn get_map<'a>(&'a self, map_id: Entity) -> Option<TileQuery<'a, 'state, D::ReadOnly>> {
        let map = self.maps.get(map_id).ok()?;

        Some(TileQuery {
            chunks: self.chunks.as_readonly(),
            map,
        })
    }

    /// Gets the query for a given map.
    pub fn get_map_mut<'a>(&'a mut self, map_id: Entity) -> Option<TileQuery<'a, 'state, D>> {
        let map = self.maps.get(map_id).ok()?;

        Some(TileQuery {
            chunks: self.chunks.reborrow(),
            map,
        })
    }
}

impl<'world, 'state, D> TileQuery<'world, 'state, D>
where
    D: TileQueryData + 'static,
{
    /// Get the readonly variant of this query.
    pub fn as_readonly(&self) -> TileQuery<'_, 'state, D::ReadOnly> {
        TileQuery {
            chunks: self.chunks.as_readonly(),
            map: self.map,
        }
    }

    /// Get the readonly variant of this query.
    pub fn reborrow(&mut self) -> TileQuery<'_, 'state, D> {
        TileQuery {
            chunks: self.chunks.reborrow(),
            map: self.map,
        }
    }

    /// Gets the query item for the given tile.
    /// # Safety
    /// This function makes it possible to violate Rust's aliasing guarantees: please use responsibly.
    // SAFETY: This function makes it possible to violate Rust's aliasing guarantees: please use responsibly.
    #[inline]
    #[expect(unsafe_code, reason = "necessary for unchecked access")]
    pub unsafe fn get_at_unchecked(&self, coord: IVec2) -> Option<<D as TileQueryData>::Data<'_>> {
        let chunk_coord = self.map.tile_chunk_position(coord);
        let chunk_entity = self.map.chunks.get(&chunk_coord).cloned()?;

        #[expect(unsafe_code, reason = "unchecked accessor")]
        let storages = if let Ok(storages) = unsafe { self.chunks.get_unchecked(chunk_entity) } {
            storages
        } else {
            return None;
        };

        let index = self.map.index(coord);

        <D as TileQueryData>::get_at(storages, index)
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

    #[inline]
    pub fn iter_in(&self, rect: IRect) -> TileQueryIter<'_, 'state, D::ReadOnly> {
        TileQueryIter {
            query: self.as_readonly(),
            rect,
            current_pos: IVec2::new(rect.min.x, rect.min.y),
        }
    }

    #[inline]
    pub fn iter_in_mut(&mut self, rect: IRect) -> TileQueryIter<'_, 'state, D> {
        TileQueryIter {
            query: self.reborrow(),
            rect,
            current_pos: IVec2::new(rect.min.x, rect.min.y),
        }
    }
}

pub struct TileQueryIter<'q, 's, D>
where
    D: TileQueryData + 'static,
{
    query: TileQuery<'q, 's, D>,
    rect: IRect,
    current_pos: IVec2,
}

impl<'q, 's, D> Iterator for TileQueryIter<'q, 's, D>
where
    D: TileQueryData + 'static,
{
    type Item = Option<<D as TileQueryData>::Data<'q>>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.current_pos.y >= self.rect.max.y {
            return None;
        }

        // SAFETY: The returned types are tied to the lifetime of the iterator, which is tied to the lifetime and mutability of the query.
        #[expect(unsafe_code, reason = "necessary for iterator lifetimes")]
        let tile = unsafe { self.query.get_at_unchecked(self.current_pos) };

        self.current_pos.x += 1;
        if self.current_pos.x >= self.rect.max.x {
            self.current_pos.x = self.rect.min.x;
            self.current_pos.y += 1;
        }
        if tile.is_some() {
            // SAFETY: Since this is always tied to the lifetime of the reference we are reborrowing query from, we're just
            // telling the compiler here that we understand this particular item is pointing to something above this iterator.
            // Even if we drop the iterator, we can't create a new one or mutably borrow the underlying query again, since
            // this returned itemed will keep the original borrow used to make the iterator alive in the mind of the compiler.
            #[expect(unsafe_code, reason = "necessary for iterator lifetimes")]
            return unsafe {
                std::mem::transmute::<
                    Option<Option<<D as TileQueryData>::Data<'_>>>,
                    Option<Option<<D as TileQueryData>::Data<'_>>>,
                >(Some(tile))
            };
        }

        None
    }
}

/// Query for looking up tilemaps.
/// Contains a nested query for [`Tilemap`] entities and Chunk entities.
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
/// Contains a nested query for a [`Tilemap`] entity and Chunk entities.
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

    /// Gets the query item for the given tile.
    /// # Safety
    /// This function makes it possible to violate Rust's aliasing guarantees: please use responsibly.
    // SAFETY: This function makes it possible to violate Rust's aliasing guarantees: please use responsibly.
    #[inline]
    #[expect(unsafe_code, reason = "necessary for unchecked access")]
    pub unsafe fn get_at_unchecked(
        &self,
        coord: IVec2,
    ) -> Option<<D as QueryData>::Item<'_, 'state>> {
        let chunk_coord = self.map.tile_chunk_position(coord);
        let chunk_entity = self.map.chunks.get(&chunk_coord).cloned()?;

        let storages = self.chunks.get(chunk_entity).ok()?;

        let index = self.map.index(coord);

        let entity =
            <<&EntityTile as TileQueryData>::ReadOnly as TileQueryData>::get_at(storages, index)?;

        #[expect(unsafe_code, reason = "unchecked access")]
        unsafe {
            self.tiles.get_unchecked(**entity).ok()
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

    #[inline]
    pub fn iter_in(&self, rect: IRect) -> TileEntityQueryIter<'_, 'state, D::ReadOnly, F> {
        TileEntityQueryIter {
            query: self.as_readonly(),
            rect,
            current_pos: IVec2::new(rect.min.x, rect.min.y),
        }
    }

    #[inline]
    pub fn iter_in_mut(&mut self, rect: IRect) -> TileEntityQueryIter<'_, 'state, D, F> {
        TileEntityQueryIter {
            query: self.reborrow(),
            rect,
            current_pos: IVec2::new(rect.min.x, rect.min.y),
        }
    }
}

pub struct TileEntityQueryIter<'q, 's, D, F>
where
    D: QueryData + 'static,
    F: QueryFilter + 'static,
{
    query: TileEntityQuery<'q, 's, D, F>,
    rect: IRect,
    current_pos: IVec2,
}

impl<'q, 's, D, F> Iterator for TileEntityQueryIter<'q, 's, D, F>
where
    D: QueryData + 'static,
    F: QueryFilter + 'static,
{
    type Item = Option<<D as QueryData>::Item<'q, 's>>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.current_pos.y >= self.rect.max.y {
            return None;
        }

        // SAFETY: The returned types are tied to the lifetime of the iterator, which is tied to the lifetime and mutability of the query.
        #[expect(unsafe_code, reason = "necessary for iterator lifetimes")]
        let tile = unsafe { self.query.get_at_unchecked(self.current_pos) };

        self.current_pos.x += 1;
        if self.current_pos.x >= self.rect.max.x {
            self.current_pos.x = self.rect.min.x;
            self.current_pos.y += 1;
        }
        if tile.is_some() {
            // SAFETY: Since this is always tied to the lifetime of the reference we are reborrowing query from, we're just
            // telling the compiler here that we understand this particular item is pointing to something above this iterator.
            // Even if we drop the iterator, we can't create a new one or mutably borrow the underlying query again, since
            // this returned itemed will keep the original borrow used to make the iterator alive in the mind of the compiler.
            #[expect(unsafe_code, reason = "necessary for iterator lifetimes")]
            return unsafe {
                std::mem::transmute::<
                    Option<Option<<D as QueryData>::Item<'_, '_>>>,
                    Option<Option<<D as QueryData>::Item<'_, '_>>>,
                >(Some(tile))
            };
        }

        None
    }
}
