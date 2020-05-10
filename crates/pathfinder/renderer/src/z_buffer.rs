// pathfinder/renderer/src/z_buffer.rs
//
// Copyright © 2019 The Pathfinder Project Developers.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Software occlusion culling.

use crate::builder::Occluder;
use crate::gpu_data::{Tile, TileBatch};
use crate::paint::{PaintId, PaintMetadata};
use crate::tile_map::DenseTileMap;
use crate::tiles;
use pathfinder_content::effects::BlendMode;
use pathfinder_geometry::alignment::{AlignedU16, AlignedI16};
use pathfinder_geometry::rect::RectF;
use pathfinder_geometry::vector::Vector2I;
use vec_map::VecMap;

pub(crate) struct ZBuffer {
    buffer: DenseTileMap<u32>,
    depth_metadata: VecMap<DepthMetadata>,
}

pub(crate) struct SolidTiles {
    pub(crate) batches: Vec<TileBatch>,
}

#[derive(Clone, Copy)]
pub(crate) struct DepthMetadata {
    pub(crate) paint_id: PaintId,
}
impl ZBuffer {
    pub(crate) fn new(view_box: RectF) -> ZBuffer {
        let tile_rect = tiles::round_rect_out_to_tile_bounds(view_box);
        ZBuffer {
            buffer: DenseTileMap::from_builder(|_| 0, tile_rect),
            depth_metadata: VecMap::new(),
        }
    }

    pub(crate) fn test(&self, coords: Vector2I, depth: u32) -> bool {
        let tile_index = self.buffer.coords_to_index_unchecked(coords);
        self.buffer.data[tile_index as usize] < depth
    }

    pub(crate) fn update(&mut self,
                         solid_tiles: &[Occluder],
                         depth: u32,
                         metadata: DepthMetadata) {
        self.depth_metadata.insert(depth as usize, metadata);
        for solid_tile in solid_tiles {
            let tile_index = self.buffer.coords_to_index_unchecked(solid_tile.coords);
            let z_dest = &mut self.buffer.data[tile_index as usize];
            *z_dest = u32::max(*z_dest, depth);
        }
    }

    pub(crate) fn build_solid_tiles(&self, paint_metadata: &[PaintMetadata]) -> SolidTiles {
        let mut solid_tiles = SolidTiles { batches: vec![] };

        for tile_index in 0..self.buffer.data.len() {
            let depth = self.buffer.data[tile_index];
            if depth == 0 {
                continue;
            }

            let tile_coords = self.buffer.index_to_coords(tile_index);

            let depth_metadata = self.depth_metadata[depth as usize];
            let paint_id = depth_metadata.paint_id;
            let paint_metadata = &paint_metadata[paint_id.0 as usize];

            let tile_position = tile_coords + self.buffer.rect.origin();

            // Create a batch if necessary.
            let paint_tile_batch_texture = paint_metadata.tile_batch_texture();
            let paint_filter = paint_metadata.filter();
            match solid_tiles.batches.last() {
                Some(TileBatch { color_texture: tile_batch_texture, filter: tile_filter, .. }) if
                        *tile_batch_texture == paint_tile_batch_texture &&
                        *tile_filter == paint_filter => {}
                _ => {
                    // Batch break.
                    //
                    // TODO(pcwalton): We could be more aggressive with batching here, since we
                    // know there are no overlaps.
                    solid_tiles.batches.push(TileBatch {
                        color_texture: paint_tile_batch_texture,
                        tiles: vec![],
                        filter: paint_filter,
                        blend_mode: BlendMode::default(),
                        tile_page: !0,
                    });
                }
            }

            let batch = solid_tiles.batches.last_mut().unwrap();
            batch.tiles.push(Tile::new_solid_from_paint_id(tile_position, paint_id));
        }

        solid_tiles
    }
}

impl Tile {
    pub(crate) fn new_solid_from_paint_id(tile_origin: Vector2I, paint_id: PaintId) -> Tile {
        Tile {
            tile_x: tile_origin.x() as AlignedI16,
            tile_y: tile_origin.y() as AlignedI16,
            mask_0_backdrop: 0,
            mask_0_u: 0,
            mask_0_v: 0,
            ctrl: 0,
            pad: 0,
            color: paint_id.0 as AlignedU16,
        }
    }
}
