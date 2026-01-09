use bevy_app::{App, Plugin};
use bevy_derive::Deref;
use bevy_ecs::{
    component::Component,
    entity::Entity,
    hierarchy::ChildOf,
    lifecycle::HookContext,
    system::Command,
    world::{DeferredWorld, World},
};
use bevy_math::IVec2;
use tracing::warn;

use crate::{RemoveTile, SetTile, SetTileResult};

/// Plugin that handles the initialization and updating of tilemap chunks.
/// Adds systems for processing newly added tilemap chunks.
pub struct EntityTilePlugin;

impl Plugin for EntityTilePlugin {
    fn build(&self, app: &mut App) {
        app.world_mut()
            .register_component_hooks::<TileCoord>()
            .on_insert(on_insert_entity_tile)
            .on_remove(on_remove_entity_tile);
        app.world_mut()
            .register_component_hooks::<InMap>()
            .on_remove(on_remove_entity_tile);
    }
}

/// An Entity in the tilemap
#[derive(Component, Clone, Debug, Deref)]
pub struct EntityTile(pub Entity);

#[derive(Component, Clone, Debug, Deref)]
#[component(immutable)]
pub struct InMap(pub Entity);

#[derive(Component, Clone, Debug, Deref)]
#[component(immutable)]
pub struct TileCoord(pub IVec2);

impl TileCoord {
    /// Iterate through the non-diagonal adjacent tiles to this coord
    pub fn adjacent(&self) -> impl Iterator<Item = TileCoord> + use<> {
        [
            TileCoord(IVec2::new(self.x + 1, self.y)),
            TileCoord(IVec2::new(self.x, self.y + 1)),
            TileCoord(IVec2::new(self.x, self.y - 1)),
            TileCoord(IVec2::new(self.x - 1, self.y)),
        ]
        .into_iter()
    }
}

#[derive(Component, Clone, Debug)]
pub struct DespawnOnRemove;

fn on_insert_entity_tile(mut world: DeferredWorld, HookContext { entity, .. }: HookContext) {
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

    world.commands().queue(move |world: &mut World| {
        let SetTileResult {
            chunk_id: Some(chunk_id),
            replaced_tile,
        } = SetTile {
            tilemap_id: in_map.0,
            tile_position: tile_position.0,
            maybe_tile: Some(EntityTile(entity)),
        }
        .apply(world)
        else {
            warn!("Could not create chunk to place Tile {} entity.", entity);
            return;
        };

        world.entity_mut(entity).insert(ChildOf(chunk_id));

        if let Some(replaced_tile) = replaced_tile {
            let mut replaced_tile = world.entity_mut(replaced_tile.0);
            if replaced_tile.contains::<DespawnOnRemove>() {
                replaced_tile.despawn();
            } else {
                replaced_tile.remove::<(InMap, TileCoord)>();
            }
        }
    });
}

fn on_remove_entity_tile(mut world: DeferredWorld, HookContext { entity, .. }: HookContext) {
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

    world.commands().queue(move |world: &mut World| {
        RemoveTile {
            tilemap_id: in_map.0,
            tile_position: tile_position.0,
        }
        .apply(world);

        let Ok(mut removed) = world.get_entity_mut(entity) else {
            return;
        };
        if removed.contains::<DespawnOnRemove>() {
            removed.despawn();
        } else {
            removed.remove::<InMap>();
        }
    });
}
