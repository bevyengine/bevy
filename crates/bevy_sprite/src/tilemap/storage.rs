use std::ops::Deref;

use bevy_ecs::{component::Component, entity::Entity, name::Name, reflect::ReflectComponent};
use bevy_math::{URect, UVec2};
use bevy_reflect::Reflect;
use bevy_transform::components::Transform;

#[derive(Component, Clone, Debug, Default, Reflect)]
#[reflect(Component)]
#[require(Name::new("TileStorage"), Transform)]
pub struct TileStorage<T> {
    pub tiles: Vec<Option<T>>,
    size: UVec2,
}

impl<T> TileStorage<T> {
    pub fn new(size: UVec2) -> Self {
        let mut tiles = Vec::new();
        tiles.resize_with(size.element_product() as usize, Default::default);
        Self { tiles, size }
    }

    pub fn index(&self, tile_coord: UVec2) -> usize {
        (tile_coord.y * self.size.x + tile_coord.x) as usize
    }

    pub fn get(&self, tile_coord: UVec2) -> Option<&T> {
        let index = self.index(tile_coord);
        self.tiles.get(index).map(Option::as_ref).flatten()
    }

    pub fn get_mut(&mut self, tile_coord: UVec2) -> Option<&mut T> {
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

    // pub fn iter(&self) -> impl Iterator<Item = Option<Entity>> {
    //     self.tiles.iter().cloned()
    // }

    // pub fn iter_sub_rect(&self, rect: URect) -> impl Iterator<Item = Option<Entity>> {
    //     let URect { min, max } = rect;

    //     (min.y..max.y).flat_map(move |y| {
    //         (min.x..max.x).map(move |x| {
    //             if x >= self.size.x || y >= self.size.y {
    //                 return None;
    //             }

    //             let index = (y * self.size.x + x) as usize;
    //             self.tiles.get(index).cloned().flatten()
    //         })
    //     })
    // }

    // pub fn iter_chunk_tiles(
    //     &self,
    //     chunk_position: UVec2,
    //     chunk_size: UVec2,
    // ) -> impl Iterator<Item = Option<Entity>> {
    //     let chunk_rect = URect::from_corners(
    //         chunk_position * chunk_size,
    //         (chunk_position + UVec2::splat(1)) * chunk_size,
    //     );

    //     self.iter_sub_rect(chunk_rect)
    // }

    pub fn size(&self) -> UVec2 {
        self.size
    }
}
