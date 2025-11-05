use crate::tilemap::{TileData, TileStorage, Tilemap};
use bevy_ecs::{entity::Entity, hierarchy::ChildOf, system::Commands, world::World};
use bevy_math::{IVec2, UVec2, Vec2, Vec3};
use bevy_transform::components::Transform;

pub trait CommandsTilemapExt {
    fn set_tile<T: TileData>(
        &mut self,
        tilemap: Entity,
        tile_position: IVec2,
        maybe_tile: Option<T>,
    );

    fn remove_tile(&mut self, tilemap: Entity, tile_position: IVec2);
}

impl CommandsTilemapExt for Commands<'_, '_> {
    fn set_tile<T: TileData>(
        &mut self,
        tilemap_id: Entity,
        tile_position: IVec2,
        maybe_tile: Option<T>,
    ) {
        self.queue(move |w: &mut World| {
            let Ok(mut tilemap_entity) = w.get_entity_mut(tilemap_id) else {
                tracing::warn!("Could not find Tilemap Entity {:?}", tilemap_id);
                return;
            };

            let Some(tilemap) = tilemap_entity.get::<Tilemap>() else {
                tracing::warn!("Could not find Tilemap on Entity {:?}", tilemap_id);
                return;
            };

            let chunk_position = tilemap.tile_chunk_position(tile_position);
            let tile_position = tilemap.tile_chunk_local_position(tile_position);

            if let Some(tile_storage_id) = tilemap.chunks.get(&chunk_position).cloned() {
                tilemap_entity.world_scope(move |w| {
                    let Ok(mut tilestorage_entity) = w.get_entity_mut(tile_storage_id) else {
                        tracing::warn!("Could not find TileStorage Entity {:?}", tile_storage_id);
                        return;
                    };

                    let Some(mut tile_storage) = tilestorage_entity.get_mut::<TileStorage<T>>()
                    else {
                        tracing::warn!(
                            "Could not find TileStorage on Entity {:?}",
                            tile_storage_id
                        );
                        return;
                    };

                    tile_storage.set(tile_position, maybe_tile);
                });
            } else {
                let chunk_size = tilemap.chunk_size;
                let tile_size = tilemap.tile_display_size;
                let tile_storage_id = tilemap_entity.world_scope(move |w| {
                    let mut tile_storage = TileStorage::<T>::new(chunk_size);
                    tile_storage.set(tile_position, maybe_tile);
                    let translation = Vec2::new(chunk_size.x as f32, chunk_size.y as f32) * Vec2::new(tile_size.x as f32, tile_size.y as f32) * Vec2::new(chunk_position.x as f32, chunk_position.y as f32);
                    let translation = Vec3::new(translation.x, translation.y, 0.0);
                    let transform = Transform::from_translation(translation);
                    w.spawn((ChildOf(tilemap_id), tile_storage, transform)).id()
                });
                let Some(mut tilemap) = tilemap_entity.get_mut::<Tilemap>() else {
                    tracing::warn!("Could not find Tilemap on Entity {:?}", tilemap_id);
                    return;
                };
                tilemap.chunks.insert(chunk_position, tile_storage_id);
            };
        });
    }

    fn remove_tile(&mut self, tilemap: Entity, tile_position: IVec2) {
        todo!()
    }
}
