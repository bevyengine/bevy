use crate::MeshMaterial2d;
use bevy_app::{App, Plugin, PreUpdate};
use bevy_asset::{Assets, Handle, RenderAssetUsages};
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::{
    component::{Component, HookContext},
    entity::Entity,
    hierarchy::ChildOf,
    name::Name,
    query::Changed,
    relationship::{Relationship, RelationshipHookMode, RelationshipTarget},
    resource::Resource,
    schedule::IntoScheduleConfigs,
    system::{Commands, Query, ResMut},
    world::DeferredWorld,
};
use bevy_image::{Image, ImageSampler};
use bevy_math::{FloatOrd, UVec2, Vec2, Vec3};
use bevy_platform::collections::{hash_map::Entry, HashMap, HashSet};
use bevy_render::{
    mesh::{Indices, Mesh, Mesh2d, PrimitiveTopology},
    render_resource::{
        Extent3d, TextureDescriptor, TextureDimension, TextureFormat, TextureUsages,
    },
    view::ViewVisibility,
};
use bevy_transform::components::Transform;
use lettuces::IVec2;
use tracing::warn;

use super::{TileStorage, Tilemap, TilemapChunkMaterial, Tileset, ATTRIBUTE_TILE_INDEX};

/// Plugin that handles the initialization and updating of tilemap chunks.
/// Adds systems for processing newly added tilemap chunks and updating their indices.
pub struct TilemapChunkPlugin;

impl Plugin for TilemapChunkPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<TilemapChunkMeshCache>().add_systems(
            PreUpdate,
            (spawn_missing_tilemap_chunks, update_visible_tilemap_chunks).chain(),
        );
    }
}

type TilemapChunkMeshCacheKey = (UVec2, FloatOrd, FloatOrd);

/// A resource storing the meshes for each tilemap chunk size.
#[derive(Resource, Default, Deref, DerefMut)]
pub struct TilemapChunkMeshCache(HashMap<TilemapChunkMeshCacheKey, Handle<Mesh>>);

/// A component representing a chunk of a tilemap.
/// Each chunk is a rectangular section of tiles that is rendered as a single mesh.
#[derive(Component, Clone, Debug)]
#[component(
    immutable,
    on_insert = <Self as Relationship>::on_insert,
    on_replace = <Self as Relationship>::on_replace,
)]
#[require(Mesh2d, MeshMaterial2d<TilemapChunkMaterial>)]
pub struct TilemapChunk {
    tilemap: Entity,
    location: IVec2,
}

impl Relationship for TilemapChunk {
    type RelationshipTarget = Tilemap;

    fn from(tilemap: Entity) -> Self {
        Self {
            tilemap,
            location: IVec2::ZERO,
        }
    }

    fn get(&self) -> Entity {
        self.tilemap
    }

    fn on_insert(
        mut world: DeferredWorld,
        HookContext {
            entity,
            caller,
            relationship_hook_mode,
            ..
        }: HookContext,
    ) {
        match relationship_hook_mode {
            RelationshipHookMode::Run => {}
            RelationshipHookMode::Skip => return,
            RelationshipHookMode::RunIfNotLinked => {
                if <Self::RelationshipTarget as RelationshipTarget>::LINKED_SPAWN {
                    return;
                }
            }
        }

        let &TilemapChunk {
            tilemap: tilemap_entity,
            location,
        } = world.get(entity).unwrap();

        if tilemap_entity == entity {
            warn!(
                "{}The {}{{ tilemap: {tilemap_entity}, location: {location} }} relationship on entity {entity} points to itself. The invalid {} relationship has been removed.",
                caller
                    .map(|location| format!("{location}: "))
                    .unwrap_or_default(),
                core::any::type_name::<Self>(),
                core::any::type_name::<Self>()
            );
            world.commands().entity(entity).remove::<Self>();
            return;
        }

        let Ok(mut tilemap_entity_mut) = world.get_entity_mut(tilemap_entity) else {
            warn!(
                "{}The {}{{ tilemap: {tilemap_entity}, location: {location} }} relationship on entity {entity} relates to an entity that does not exist. The invalid {} relationship has been removed.",
                caller
                    .map(|location| format!("{location}: "))
                    .unwrap_or_default(),
                core::any::type_name::<Self>(),
                core::any::type_name::<Self>()
            );
            world.commands().entity(entity).remove::<Self>();
            return;
        };

        if let Some(mut tilemap) = tilemap_entity_mut.get_mut::<Tilemap>() {
            match tilemap.chunks.entry(location) {
                Entry::Occupied(e) => {
                    warn!(
                        "{}The {}{{ tilemap: {tilemap_entity}, location: {location} }} relationship on entity {entity} relates to a location that is already occupied by {}. The invalid {} relationship has been removed.",
                        caller
                            .map(|location| format!("{location}: "))
                            .unwrap_or_default(),
                        core::any::type_name::<Self>(),
                        e.get(),
                        core::any::type_name::<Self>(),
                    );
                    world.commands().entity(entity).remove::<Self>();
                }
                Entry::Vacant(e) => {
                    e.insert(entity);
                }
            }
        } else {
            let mut tilemap = Tilemap::default();
            tilemap.chunks.insert(location, entity);
            world.commands().entity(tilemap_entity).insert(tilemap);
        }
    }

    fn on_replace(
        mut world: DeferredWorld,
        HookContext {
            entity,
            relationship_hook_mode,
            ..
        }: HookContext,
    ) {
        match relationship_hook_mode {
            RelationshipHookMode::Run => {}
            RelationshipHookMode::Skip => return,
            RelationshipHookMode::RunIfNotLinked => {
                if <Self::RelationshipTarget as RelationshipTarget>::LINKED_SPAWN {
                    return;
                }
            }
        }

        let &TilemapChunk { tilemap, location } = world.get(entity).unwrap();

        if let Some(mut tilemap) = world.get_mut::<Tilemap>(tilemap) {
            if let Entry::Occupied(e) = tilemap.chunks.entry(location) {
                if *e.get() == entity {
                    e.remove();
                }
            }
        }
    }
}

fn spawn_missing_tilemap_chunks(
    tilemap_query: Query<(Entity, &Tilemap, &TileStorage, &Tileset), Changed<TileStorage>>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut tilemap_chunk_mesh_cache: ResMut<TilemapChunkMeshCache>,
) {
    for (tilemap_entity, tilemap, tile_storage, tileset) in tilemap_query {
        let chunk_size = tile_storage.chunk_size();
        let display_size = (chunk_size * tileset.tile_size).as_vec2();

        for chunk_position in tile_storage
            .iter_dirty_chunk_positions()
            .filter(|pos| !tilemap.chunks.contains_key(*pos))
        {
            let chunk_world_position = chunk_position.as_vec2() * display_size;

            let mesh_key: TilemapChunkMeshCacheKey = (
                chunk_size,
                FloatOrd(display_size.x),
                FloatOrd(display_size.y),
            );

            let mesh = tilemap_chunk_mesh_cache
                .entry(mesh_key)
                .or_insert_with(|| meshes.add(make_chunk_mesh(&chunk_size, &display_size)));

            commands.spawn((
                Name::new(format!("TilemapChunk: {chunk_position}")),
                TilemapChunk {
                    tilemap: tilemap_entity,
                    location: *chunk_position,
                },
                Transform::from_translation(chunk_world_position.extend(0.0)),
                Mesh2d(mesh.clone()),
                ChildOf(tilemap_entity),
            ));
        }
    }
}

fn update_visible_tilemap_chunks(
    tilemap_query: Query<(Entity, &Tilemap, &mut TileStorage, &Tileset)>,
    mut chunk_query: Query<(
        Entity,
        &TilemapChunk,
        &mut MeshMaterial2d<TilemapChunkMaterial>,
        &ViewVisibility,
    )>,
    mut chunk_materials: ResMut<Assets<TilemapChunkMaterial>>,
    mut images: ResMut<Assets<Image>>,
) {
    for (tilemap_entity, tilemap, mut tile_storage, tileset) in tilemap_query {
        let mut chunk_positions_to_clear = HashSet::new();
        for chunk_pos in tile_storage.iter_dirty_chunk_positions() {
            let Some(chunk_entity) = tilemap.chunks.get(chunk_pos) else {
                continue;
            };
            let Ok((chunk_entity, chunk, mut chunk_material, visibility)) =
                chunk_query.get_mut(*chunk_entity)
            else {
                continue;
            };
            if !visibility.get() {
                continue;
            }

            chunk_positions_to_clear.insert(chunk.location);

            let chunk_size = tile_storage.chunk_size();
            let Ok(chunk_tiles) = tile_storage.chunk_tiles(chunk.location) else {
                warn!(
                    "Unable to access TileStorage data for tilemap chunk {} in tilemap {}",
                    chunk_entity, tilemap_entity
                );
                continue;
            };

            let indices = chunk_tiles.iter().copied().flat_map(|tile| {
                u16::to_ne_bytes(tile.map(|tile| tile.tileset_index).unwrap_or(u16::MAX))
            });

            if let Some(material) = chunk_materials.get_mut(chunk_material.id()) {
                let Some(chunk_image) = images.get_mut(&material.indices) else {
                    return;
                };
                let Some(data) = chunk_image.data.as_mut() else {
                    warn!(
                    "TilemapChunkMaterial indices image data not found for tilemap chunk {} in tilemap {}",
                    chunk_entity,
                    tilemap_entity
                );
                    return;
                };
                data.clear();
                data.extend(indices);
            } else {
                let chunk_image = make_chunk_image(&chunk_size, indices.collect::<Vec<_>>());

                let material = chunk_materials.add(TilemapChunkMaterial {
                    alpha_mode: tilemap.alpha_mode,
                    tileset: tileset.image.clone(),
                    indices: images.add(chunk_image),
                });

                *chunk_material = MeshMaterial2d(material);
            }
        }

        tile_storage.clear_dirty_chunk_positions(chunk_positions_to_clear);
    }
}

fn make_chunk_image(size: &UVec2, data: Vec<u8>) -> Image {
    Image {
        data: Some(data),
        texture_descriptor: TextureDescriptor {
            size: Extent3d {
                width: size.x,
                height: size.y,
                depth_or_array_layers: 1,
            },
            dimension: TextureDimension::D2,
            format: TextureFormat::R16Uint,
            label: None,
            mip_level_count: 1,
            sample_count: 1,
            usage: TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST,
            view_formats: &[],
        },
        sampler: ImageSampler::nearest(),
        texture_view_descriptor: None,
        asset_usage: RenderAssetUsages::RENDER_WORLD | RenderAssetUsages::MAIN_WORLD,
    }
}

fn make_chunk_mesh(size: &UVec2, display_size: &Vec2) -> Mesh {
    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::RENDER_WORLD | RenderAssetUsages::MAIN_WORLD,
    );

    let num_quads = size.element_product() as usize;
    let quad_size = display_size / size.as_vec2();

    let mut positions = Vec::with_capacity(4 * num_quads);
    let mut uvs = Vec::with_capacity(4 * num_quads);
    let mut indices = Vec::with_capacity(6 * num_quads);

    for y in 0..size.y {
        for x in 0..size.x {
            let i = positions.len() as u32;

            let p0 = quad_size * UVec2::new(x, y).as_vec2();
            let p1 = p0 + quad_size;

            positions.extend([
                Vec3::new(p0.x, p0.y, 0.0),
                Vec3::new(p1.x, p0.y, 0.0),
                Vec3::new(p0.x, p1.y, 0.0),
                Vec3::new(p1.x, p1.y, 0.0),
            ]);

            uvs.extend([
                Vec2::new(0.0, 1.0),
                Vec2::new(1.0, 1.0),
                Vec2::new(0.0, 0.0),
                Vec2::new(1.0, 0.0),
            ]);

            indices.extend([i, i + 2, i + 1]);
            indices.extend([i + 3, i + 1, i + 2]);
        }
    }

    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.insert_attribute(
        ATTRIBUTE_TILE_INDEX,
        (0..size.element_product())
            .flat_map(|i| [i; 4])
            .collect::<Vec<u32>>(),
    );
    mesh.insert_indices(Indices::U32(indices));

    mesh
}
