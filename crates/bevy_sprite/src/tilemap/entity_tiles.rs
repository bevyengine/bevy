use bevy_app::{App, Plugin};
use bevy_derive::Deref;
use bevy_ecs::{component::Component, entity::Entity, hierarchy::ChildOf, lifecycle::HookContext, system::Command, world::{DeferredWorld, World}};
use bevy_math::IVec2;
use tracing::warn;

use crate::{SetTile, SetTileResult, TileData};

/// Plugin that handles the initialization and updating of tilemap chunks.
/// Adds systems for processing newly added tilemap chunks.
pub struct EntityTilePlugin;

impl Plugin for EntityTilePlugin {
    fn build(&self, app: &mut App) {
        app.world_mut().register_component_hooks::<TileCoord>().on_insert(on_insert_entity_tile);
    }
}

/// An Entity in the tilemap
pub struct EntityTile(pub Entity);

impl TileData for EntityTile {
    
}

#[derive(Component, Clone, Debug, Deref)]
pub struct InMap(pub Entity);

#[derive(Component, Clone, Debug, Deref)]
pub struct TileCoord(pub IVec2);

fn on_insert_entity_tile(mut world: DeferredWorld, HookContext { entity, .. }: HookContext){
    let Ok(tile) = world.get_entity(entity) else {
        warn!("Tile {} not found", entity);
        return;
    };
    let Some(in_map) = tile.get::<InMap>().cloned() else {
        warn!("Tile {} is not in a TileMap", entity);
        return;
    };
    let Some(tile_position) = tile.get::<TileCoord>().cloned() else {
        warn!("Tile {} has no tile coord.", entity);
        return;
    };

    world
        .commands()
        .queue(move |world: &mut World| {
            let SetTileResult { chunk_id: Some(chunk_id), replaced_tile} = SetTile {
                tilemap_id: in_map.0,
                tile_position: tile_position.0,
                maybe_tile: Some(EntityTile(entity)),
            }.apply(world) else {
                warn!("Could not create chunk to place Tile {} entity.", entity);
                return;
            };
            
            world.entity_mut(entity).insert(ChildOf(chunk_id));

            if let Some(replaced_tile) = replaced_tile {
                world.despawn(replaced_tile.0);
            }
        });
}