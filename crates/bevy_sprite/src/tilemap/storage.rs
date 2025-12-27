use std::ops::Deref;

use bevy_ecs::{
    change_detection::{DetectChangesMut, MutUntyped},
    component::{Component, ComponentId},
    lifecycle::HookContext,
    name::Name,
    reflect::ReflectComponent,
    world::{DeferredWorld, World},
};
use bevy_math::{URect, UVec2};
use bevy_platform::collections::HashMap;
use bevy_reflect::Reflect;
use bevy_transform::components::Transform;
use tracing::error;

#[derive(Component, Clone, Debug, Default)]
#[require(Name::new("TileStorage"), Transform)]
pub struct TileStorages {
    // Stores removal operations
    pub(crate) removals: HashMap<ComponentId, fn(MutUntyped<'_>, UVec2)>,
}

#[derive(Component, Clone, Debug, Default, Reflect)]
#[reflect(Component)]
#[require(Name::new("TileStorage"), TileStorages, Transform)]
#[component(on_add = on_add_tile_storage::<T>)]
pub struct TileStorage<T: Send + Sync + 'static> {
    pub tiles: Vec<Option<T>>,
    size: UVec2,
}

impl<T: Send + Sync + 'static> TileStorage<T> {
    pub fn new(size: UVec2) -> Self {
        let mut tiles = Vec::new();
        tiles.resize_with(size.element_product() as usize, Default::default);
        Self { tiles, size }
    }

    pub fn index(&self, tile_coord: UVec2) -> usize {
        (tile_coord.y * self.size.x + tile_coord.x) as usize
    }

    pub fn get_at(&self, tile_coord: UVec2) -> Option<&T> {
        let index = self.index(tile_coord);
        self.tiles.get(index).map(Option::as_ref).flatten()
    }

    pub fn get_at_mut(&mut self, tile_coord: UVec2) -> Option<&mut T> {
        let index = self.index(tile_coord);
        self.tiles.get_mut(index).map(Option::as_mut).flatten()
    }

    pub fn set(&mut self, tile_position: UVec2, maybe_tile: Option<T>) -> Option<T> {
        let index = self.index(tile_position);
        let tile = self.tiles.get_mut(index)?;
        core::mem::replace(tile, maybe_tile)
    }

    pub fn remove(&mut self, tile_position: UVec2) -> Option<T> {
        self.set(tile_position, None)
    }

    pub fn iter(&self) -> impl Iterator<Item = Option<&T>> {
        self.tiles.iter().map(|item| item.as_ref())
    }

    pub fn iter_sub_rect(&self, rect: URect) -> impl Iterator<Item = Option<&T>> {
        let URect { min, max } = rect;

        (min.y..max.y).flat_map(move |y| {
            (min.x..max.x).map(move |x| {
                if x >= self.size.x || y >= self.size.y {
                    return None;
                }

                self.get_at(UVec2 { x, y })
            })
        })
    }

    pub fn size(&self) -> UVec2 {
        self.size
    }
}

fn on_add_tile_storage<T: Send + Sync + 'static>(
    mut world: DeferredWorld<'_>,
    HookContext {
        component_id,
        entity,
        ..
    }: HookContext,
) {
    world.commands().queue(move |world: &mut World| {
        let Ok(mut tile_storage_entity) = world.get_entity_mut(entity) else {
            error!("Could not fine Tile Storage {}", entity);
            return;
        };

        if let Some(mut storages) = tile_storage_entity.get_mut::<TileStorages>() {
            storages.removals.insert(component_id, remove_tile::<T>);
        } else {
            let mut tile_storages = TileStorages {
                removals: HashMap::with_capacity(1),
            };
            tile_storages
                .removals
                .insert(component_id, remove_tile::<T>);
            tile_storage_entity.insert(tile_storages);
        }
    });
}

fn remove_tile<T: Send + Sync + 'static>(mut raw: MutUntyped<'_>, tile_coord: UVec2) {
    let storage = raw.bypass_change_detection().reborrow();
    // SAFETY: We only call this from entities that have a TileStorage<T>
    // TODO: Maybe change this function to accept the enttity and do the component id look up here?
    #[expect(unsafe_code, reason = "testing")]
    let storage = unsafe { storage.deref_mut::<TileStorage<T>>() };
    if storage.remove(tile_coord).is_some() {
        raw.set_changed();
    }
}
