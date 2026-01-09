use crate::{
    tilemap::{TileStorage, Tilemap},
    TileStorages,
};
use bevy_ecs::{
    entity::Entity,
    hierarchy::ChildOf,
    system::{Command, Commands},
    world::World,
};
use bevy_math::{IVec2, Vec2, Vec3};
use bevy_transform::components::Transform;

pub trait CommandsTilemapExt {
    fn set_tile<T: Send + Sync + 'static>(
        &mut self,
        tilemap_id: Entity,
        tile_position: IVec2,
        maybe_tile: Option<T>,
    );

    fn remove_tile(&mut self, tilemap_id: Entity, tile_position: IVec2);
}

impl CommandsTilemapExt for Commands<'_, '_> {
    fn set_tile<T: Send + Sync + 'static>(
        &mut self,
        tilemap_id: Entity,
        tile_position: IVec2,
        maybe_tile: Option<T>,
    ) {
        self.queue(move |world: &mut World| {
            SetTile {
                tilemap_id,
                tile_position,
                maybe_tile,
            }
            .apply(world);
        });
    }

    fn remove_tile(&mut self, tilemap_id: Entity, tile_position: IVec2) {
        self.queue(move |world: &mut World| {
            RemoveTile {
                tilemap_id,
                tile_position,
            }
            .apply(world);
        });
    }
}

pub struct SetTile<T: Send + Sync + 'static> {
    pub tilemap_id: Entity,
    pub tile_position: IVec2,
    pub maybe_tile: Option<T>,
}

pub struct SetTileResult<T: Send + Sync + 'static> {
    pub replaced_tile: Option<T>,
    pub chunk_id: Option<Entity>,
}

impl<T: Send + Sync + 'static> Default for SetTileResult<T> {
    fn default() -> Self {
        Self {
            replaced_tile: Default::default(),
            chunk_id: Default::default(),
        }
    }
}

impl<T: Send + Sync + 'static> Command<SetTileResult<T>> for SetTile<T> {
    fn apply(self, world: &mut World) -> SetTileResult<T> {
        let Ok(mut tilemap_entity) = world.get_entity_mut(self.tilemap_id) else {
            tracing::warn!("Could not find Tilemap Entity {:?}", self.tilemap_id);
            return Default::default();
        };

        let Some(tilemap) = tilemap_entity.get::<Tilemap>() else {
            tracing::warn!("Could not find Tilemap on Entity {:?}", self.tilemap_id);
            return Default::default();
        };

        let chunk_position = tilemap.tile_chunk_position(self.tile_position);
        let tile_relative_position = tilemap.tile_relative_position(self.tile_position);

        let chunk_size = tilemap.chunk_size;
        let tile_size = tilemap.tile_display_size;

        if let Some(tile_storage_id) = tilemap.chunks.get(&chunk_position).cloned() {
            let replaced_tile = tilemap_entity.world_scope(move |w| {
                let Ok(mut tilestorage_entity) = w.get_entity_mut(tile_storage_id) else {
                    tracing::warn!("Could not find TileStorage Entity {:?}", tile_storage_id);
                    return None;
                };

                let Some(mut tile_storage) = tilestorage_entity.get_mut::<TileStorage<T>>() else {
                    let mut tile_storage = TileStorage::<T>::new(chunk_size);
                    tile_storage.set(tile_relative_position, self.maybe_tile);
                    tilestorage_entity.insert(tile_storage);
                    return None;
                };

                tile_storage.set(tile_relative_position, self.maybe_tile)
            });
            SetTileResult {
                chunk_id: Some(tile_storage_id),
                replaced_tile,
            }
        } else {
            let tile_storage_id = tilemap_entity.world_scope(move |w| {
                let mut tile_storage = TileStorage::<T>::new(chunk_size);
                tile_storage.set(tile_relative_position, self.maybe_tile);
                let translation = Vec2::new(chunk_size.x as f32, chunk_size.y as f32)
                    * Vec2::new(tile_size.x as f32, tile_size.y as f32)
                    * Vec2::new(chunk_position.x as f32, chunk_position.y as f32);
                let translation = Vec3::new(translation.x, translation.y, 0.0);
                let transform = Transform::from_translation(translation);
                w.spawn((ChildOf(self.tilemap_id), tile_storage, transform))
                    .id()
            });
            let Some(mut tilemap) = tilemap_entity.get_mut::<Tilemap>() else {
                tracing::warn!("Could not find Tilemap on Entity {:?}", self.tilemap_id);
                return Default::default();
            };
            tilemap.chunks.insert(chunk_position, tile_storage_id);
            SetTileResult {
                chunk_id: Some(tile_storage_id),
                replaced_tile: None,
            }
        }
    }
}

pub struct RemoveTile {
    pub tilemap_id: Entity,
    pub tile_position: IVec2,
}

impl Command for RemoveTile {
    fn apply(self, world: &mut World) {
        let Ok(mut tilemap_entity) = world.get_entity_mut(self.tilemap_id) else {
            tracing::warn!("Could not find Tilemap Entity {:?}", self.tilemap_id);
            return;
        };

        let Some(tilemap) = tilemap_entity.get::<Tilemap>() else {
            tracing::warn!("Could not find Tilemap on Entity {:?}", self.tilemap_id);
            return;
        };

        let chunk_position = tilemap.tile_chunk_position(self.tile_position);
        let tile_relative_position = tilemap.tile_relative_position(self.tile_position);

        if let Some(tile_storage_id) = tilemap.chunks.get(&chunk_position).cloned() {
            tilemap_entity.world_scope(move |w| {
                let Ok(mut tile_storage_entity) = w.get_entity_mut(tile_storage_id) else {
                    tracing::warn!("Could not find TileStorage Entity {:?}", tile_storage_id);
                    return;
                };

                let Some(tile_storages) = tile_storage_entity.get::<TileStorages>().cloned() else {
                    tracing::warn!(
                        "Could not find TileStorages on Entity {:?}",
                        tile_storage_id
                    );
                    return;
                };

                for (tile_storage, tile_removal) in tile_storages.removals {
                    let Ok(storage) = tile_storage_entity.get_mut_by_id(tile_storage) else {
                        continue;
                    };
                    tile_removal(storage, tile_relative_position);
                }
            });
        }
    }
}
