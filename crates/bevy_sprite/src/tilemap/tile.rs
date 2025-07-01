use bevy_app::{App, Plugin, PreUpdate};
use bevy_asset::Assets;
use bevy_color::Color;
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::{
    component::Component, entity::Entity, hierarchy::ChildOf, lifecycle::{HookContext, Insert, Replace}, name::Name, observer::On, query::{Changed, Or}, reflect::ReflectComponent, schedule::IntoScheduleConfigs, system::{Commands, Query, ResMut}, world::DeferredWorld
};
use bevy_math::{primitives::Rectangle, UVec2};
use bevy_reflect::Reflect;
use bevy_render::mesh::{Mesh, Mesh2d};
use bevy_transform::components::Transform;
use tracing::warn;

use crate::{
    Anchor, TileStorage, TilemapChunk, TilemapChunkDirty, TilemapChunkMeshCache, TilemapLayer,
    Tiles, Tileset,
};

pub struct TilePlugin;

impl Plugin for TilePlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<TileOf>()
            .add_observer(on_insert_tile_of)
            .add_observer(on_replace_tile_of)
            .add_systems(PreUpdate, (update_tiles, update_previous_positions).chain());
    }
}

#[derive(Component, Clone, Debug, Deref, DerefMut, Reflect)]
#[reflect(Component, Clone, Debug)]
#[relationship(relationship_target = Tiles)]
pub struct TileOf(pub Entity);

/// Index that corresponds to the position in the tileset array texture.
#[derive(Component, Clone, Debug, Default, Reflect)]
#[reflect(Component, Clone, Debug)]
pub struct TileIndex(pub u16);

/// Grid position of the tile in the tilemap, in tile coordinates.
#[derive(Component, Clone, Copy, Debug, Default, Deref, DerefMut, Reflect, PartialEq, Eq, Hash)]
#[reflect(Component, Clone, Debug, PartialEq, Hash)]
#[component(    
    on_add = on_add_tile_position
)]
pub struct TilePosition(pub UVec2);

/// The previous grid position of the tile in the tilemap, in tile coordinates.
#[derive(Component, Clone, Copy, Debug, Default, Deref, DerefMut, Reflect, PartialEq, Eq, Hash)]
#[reflect(Component, Clone, Debug, PartialEq, Hash)]
pub struct PreviousTilePosition(pub UVec2);

/// The tint color of a tile.
#[derive(Component, Reflect, Default, Clone, Copy, Debug)]
#[reflect(Component, Clone, Debug)]
pub struct TileColor(pub Color);

impl From<Color> for TileColor {
    fn from(color: Color) -> Self {
        TileColor(color)
    }
}

/// Hides or shows a tile based on the boolean. Default: True
#[derive(Component, Reflect, Clone, Copy, Debug, Hash, PartialEq, Eq)]
#[reflect(Component, Clone, Debug, Hash, PartialEq)]
pub struct TileVisible(pub bool);

impl Default for TileVisible {
    fn default() -> Self {
        Self(true)
    }
}

fn on_insert_tile_of(
    trigger: On<Insert, TileOf>,
    tile_query: Query<(&TileOf, &TilePosition)>,
    mut tilemap_layer_query: Query<(&mut TilemapLayer, &mut TileStorage, &Tileset, &Anchor)>,
    mut tilemap_chunk_mesh_cache: ResMut<TilemapChunkMeshCache>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut commands: Commands,
) {
    let tile_entity = trigger.target();
    let Ok((TileOf(tilemap_layer_entity), tile_position)) = tile_query.get(tile_entity) else {
        warn!("TileOf: Tile not found");
        return;
    };

    let Ok((mut tilemap_layer, mut tile_storage, tileset, anchor)) =
        tilemap_layer_query.get_mut(*tilemap_layer_entity)
    else {
        warn!("TileOf: Tilemap not found");
        return;
    };

    tile_storage.set(**tile_position, Some(tile_entity));

    let chunk_position = tile_position.0 / tilemap_layer.chunk_size;
    let tile_size = tilemap_layer.tile_display_size.unwrap_or(tileset.tile_size);
    let layer_size = (tile_storage.size() * tile_size).as_vec2();
    let mesh_size = tilemap_layer.chunk_size * tile_size;

    let chunk_entity = tilemap_layer
        .chunks
        .entry(chunk_position)
        .or_insert_with(|| {
            let anchor_offset = -((layer_size / 2.0) + (anchor.as_vec() * layer_size));
            let chunk_world_position = (anchor_offset
                + ((chunk_position * mesh_size) + (mesh_size / 2)).as_vec2())
            .extend(0.0);

            let mesh = tilemap_chunk_mesh_cache
                .entry(mesh_size)
                .or_insert_with(|| meshes.add(Rectangle::from_size(mesh_size.as_vec2())));

            let chunk_entity = commands
                .spawn((
                    Name::new(format!(
                        "TilemapChunk {},{}",
                        chunk_position.x, chunk_position.y
                    )),
                    TilemapChunk {
                        tilemap_layer: *tilemap_layer_entity,
                        location: chunk_position,
                    },
                    Transform::from_translation(chunk_world_position),
                    Mesh2d(mesh.clone()),
                    ChildOf(*tilemap_layer_entity),
                ))
                .id();

            chunk_entity
        });

    commands.entity(tile_entity).insert((
        Name::new(format!("Tile {},{}", tile_position.x, tile_position.y)),
        ChildOf(*chunk_entity),
    ));
}

fn on_replace_tile_of(
    trigger: On<Replace, TileOf>,
    tile_query: Query<(&TileOf, &TilePosition)>,
    mut tile_storage_query: Query<&mut TileStorage>,
) {
    let tile_entity = trigger.target();
    let Ok((TileOf(tilemap_entity), tile_position)) = tile_query.get(tile_entity) else {
        return;
    };

    let Ok(mut tile_storage) = tile_storage_query.get_mut(*tilemap_entity) else {
        return;
    };

    tile_storage.remove(**tile_position);
}

fn update_tiles(
    tile_query: Query<
        (Entity, &TileOf, &TilePosition, &PreviousTilePosition),
        Or<(
            Changed<TilePosition>,
            Changed<TileIndex>,
            Changed<TileVisible>,
            Changed<TileColor>,
        )>,
    >,
    mut tilemap_layer_query: Query<(&TilemapLayer, &mut TileStorage)>,
    mut commands: Commands,
) {
    for (tile_entity, tile_of, tile_pos, prev_tile_pos) in tile_query {
        let Ok((tilemap_layer, mut tile_storage)) = tilemap_layer_query.get_mut(**tile_of) else {
            warn!("Couldn't find tilemap layer {}", tile_of.0);
            continue;
        };

        if tile_pos.0 != prev_tile_pos.0 {
            tile_storage.remove(prev_tile_pos.0);
            tile_storage.set(tile_pos.0, Some(tile_entity));
        }

        let old_chunk_position = prev_tile_pos.0 / tilemap_layer.chunk_size;
        let chunk_position = tile_pos.0 / tilemap_layer.chunk_size;

        // This fails if the chunk hasn't been lazily created yet by the on_insert_tile_of trigger.
        let Some(&chunk_entity) = tilemap_layer.chunks.get(&chunk_position) else {
            warn!(
                "Couldn't find chunk {} in tilemap layer {}",
                chunk_position, tile_of.0
            );
            continue;
        };

        commands.entity(chunk_entity).insert(TilemapChunkDirty);

        if old_chunk_position != chunk_position {
            commands.entity(tile_entity).insert(ChildOf(chunk_entity));

            if let Some(&old_chunk_entity) = tilemap_layer.chunks.get(&old_chunk_position) {
                commands.entity(old_chunk_entity).insert(TilemapChunkDirty);
            } else {
                warn!(
                    "Couldn't find old chunk {} in tilemap layer {}",
                    old_chunk_position, tile_of.0
                );
            };
        }
    }
}

fn on_add_tile_position(mut world: DeferredWorld, context: HookContext) {
    let Some(tile_position) = world.get::<TilePosition>(context.entity).cloned() else {
        return;
    };
    world.commands().entity(context.entity).insert(PreviousTilePosition(*tile_position));
}


fn update_previous_positions(
    mut tile_query: Query<(&TilePosition, &mut PreviousTilePosition)>,
) {
    for (tile_pos, mut prev_pos) in &mut tile_query {
        // Update the previous position to the current position
        // This will be the "previous" position for the next frame
        prev_pos.0 = tile_pos.0;
    }
}
