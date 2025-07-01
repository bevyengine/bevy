use bevy_ecs::{component::Component, entity::Entity, reflect::ReflectComponent};
use bevy_math::{URect, UVec2};
use bevy_reflect::Reflect;

#[derive(Component, Clone, Debug, Default, Reflect)]
#[reflect(Component)]
pub struct TileStorage {
    tiles: Vec<Option<Entity>>,
    size: UVec2,
}

impl TileStorage {
    pub fn new(size: UVec2) -> Self {
        Self {
            tiles: vec![None; size.element_product() as usize],
            size,
        }
    }

    fn index(&self, tile_position: UVec2) -> usize {
        (tile_position.y * self.size.x + tile_position.x) as usize
    }

    pub fn get(&self, tile_position: UVec2) -> Option<Entity> {
        let index = self.index(tile_position);
        self.tiles.get(index).cloned().flatten()
    }

    pub fn set(&mut self, tile_position: UVec2, maybe_tile_entity: Option<Entity>) {
        let index = self.index(tile_position);
        let Some(tile) = self.tiles.get_mut(index) else {
            return;
        };
        *tile = maybe_tile_entity;
    }

    pub fn remove(&mut self, tile_position: UVec2) {
        self.set(tile_position, None);
    }

    pub fn iter(&self) -> impl Iterator<Item = Option<Entity>> {
        self.tiles.iter().cloned()
    }

    pub fn iter_sub_rect(&self, rect: URect) -> impl Iterator<Item = Option<Entity>> {
        let URect { min, max } = rect;

        (min.y..max.y).flat_map(move |y| {
            (min.x..max.x).map(move |x| {
                if x >= self.size.x || y >= self.size.y {
                    return None;
                }

                let index = (y * self.size.x + x) as usize;
                self.tiles.get(index).cloned().flatten()
            })
        })
    }

    pub fn iter_chunk_tiles(
        &self,
        chunk_position: UVec2,
        chunk_size: UVec2,
    ) -> impl Iterator<Item = Option<Entity>> {
        let chunk_rect = URect::from_corners(
            chunk_position * chunk_size,
            (chunk_position + UVec2::splat(1)) * chunk_size,
        );

        self.iter_sub_rect(chunk_rect)
    }

    pub fn size(&self) -> UVec2 {
        self.size
    }
}
