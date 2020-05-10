// pathfinder/renderer/src/builder.rs
//
// Copyright © 2019 The Pathfinder Project Developers.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Packs data onto the GPU.

use crate::concurrent::executor::Executor;
use crate::gpu::renderer::{BlendModeExt, MASK_TILES_ACROSS, MASK_TILES_DOWN};
use crate::gpu_data::{AlphaTileId, Clip, ClipBatch, ClipBatchKey, ClipBatchKind, Fill};
use crate::gpu_data::{FillBatchEntry, RenderCommand, TILE_CTRL_MASK_0_SHIFT};
use crate::gpu_data::{TILE_CTRL_MASK_EVEN_ODD, TILE_CTRL_MASK_WINDING, Tile, TileBatch};
use crate::gpu_data::{TileBatchTexture, TileObjectPrimitive};
use crate::options::{PreparedBuildOptions, PreparedRenderTransform, RenderCommandListener};
use crate::paint::{PaintInfo, PaintMetadata};
use crate::scene::{DisplayItem, Scene};
use crate::tile_map::DenseTileMap;
use crate::tiles::{self, DrawTilingPathInfo, PackedTile, TILE_HEIGHT, TILE_WIDTH};
use crate::tiles::{Tiler, TilingPathInfo};
use crate::z_buffer::{DepthMetadata, ZBuffer};
use pathfinder_content::effects::{BlendMode, Filter};
use pathfinder_content::fill::FillRule;
use pathfinder_content::render_target::RenderTargetId;
use pathfinder_geometry::alignment::{AlignedI16, AlignedU8, AlignedU16, AlignedI8};
use pathfinder_geometry::line_segment::{LineSegment2F, LineSegmentU4, LineSegmentU8};
use pathfinder_geometry::rect::{RectF, RectI};
use pathfinder_geometry::transform2d::Transform2F;
use pathfinder_geometry::vector::{Vector2F, Vector2I, vec2f, vec2i};
use pathfinder_gpu::TextureSamplingFlags;
use pathfinder_simd::default::{F32x4, I32x4};
use std::sync::atomic::AtomicUsize;
use instant::Instant;
use std::u32;

pub(crate) const ALPHA_TILE_LEVEL_COUNT: usize = 2;
pub(crate) const ALPHA_TILES_PER_LEVEL: usize = 1 << (32 - ALPHA_TILE_LEVEL_COUNT + 1);

pub(crate) struct SceneBuilder<'a, 'b> {
    scene: &'a mut Scene,
    built_options: &'b PreparedBuildOptions,
    next_alpha_tile_indices: [AtomicUsize; ALPHA_TILE_LEVEL_COUNT],
    pub(crate) listener: Box<dyn RenderCommandListener>,
}

#[derive(Debug)]
pub(crate) struct ObjectBuilder {
    pub built_path: BuiltPath,
    pub fills: Vec<FillBatchEntry>,
    pub bounds: RectF,
}

#[derive(Debug)]
struct BuiltDrawPath {
    path: BuiltPath,
    blend_mode: BlendMode,
    filter: Filter,
    color_texture: Option<TileBatchTexture>,
    sampling_flags_1: TextureSamplingFlags,
    mask_0_fill_rule: FillRule,
}

#[derive(Debug)]
pub(crate) struct BuiltPath {
    pub solid_tiles: SolidTiles,
    pub empty_tiles: Vec<BuiltTile>,
    pub single_mask_tiles: Vec<BuiltTile>,
    pub clip_tiles: Vec<BuiltClip>,
    pub tiles: DenseTileMap<TileObjectPrimitive>,
    pub fill_rule: FillRule,
}

#[derive(Clone, Debug)]
pub struct BuiltTile {
    pub page: u16,
    pub tile: Tile,
}

#[derive(Clone, Copy, Debug)]
pub struct BuiltClip {
    pub clip: Clip,
    pub key: ClipBatchKey,
}

#[derive(Clone, Debug)]
pub(crate) enum SolidTiles {
    Occluders(Vec<Occluder>),
    Regular(Vec<BuiltTile>),
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct Occluder {
    pub(crate) coords: Vector2I,
}

impl<'a, 'b> SceneBuilder<'a, 'b> {
    pub(crate) fn new(
        scene: &'a mut Scene,
        built_options: &'b PreparedBuildOptions,
        listener: Box<dyn RenderCommandListener>,
    ) -> SceneBuilder<'a, 'b> {
        SceneBuilder {
            scene,
            built_options,
            next_alpha_tile_indices: [AtomicUsize::new(0), AtomicUsize::new(0)],
            listener,
        }
    }

    pub fn build<E>(&mut self, executor: &E) where E: Executor {
        let start_time = Instant::now();

        // Send the start rendering command.
        let bounding_quad = self.built_options.bounding_quad();

        let clip_path_count = self.scene.clip_paths.len();
        let draw_path_count = self.scene.paths.len();
        let total_path_count = clip_path_count + draw_path_count;

        let needs_readable_framebuffer = self.needs_readable_framebuffer();

        self.listener.send(RenderCommand::Start {
            bounding_quad,
            path_count: total_path_count,
            needs_readable_framebuffer,
        });

        let render_transform = match self.built_options.transform {
            PreparedRenderTransform::Transform2D(transform) => transform.inverse(),
            _ => Transform2F::default()
        };

        // Build paint data.
        let PaintInfo {
            render_commands,
            paint_metadata,
            render_target_metadata: _,
        } = self.scene.build_paint_info(render_transform);
        for render_command in render_commands {
            self.listener.send(render_command);
        }

        let effective_view_box = self.scene.effective_view_box(self.built_options);

        let built_clip_paths = executor.build_vector(clip_path_count, |path_index| {
            self.build_clip_path(PathBuildParams {
                path_index,
                view_box: effective_view_box,
                built_options: &self.built_options,
                scene: &self.scene,
            })
        });

        let built_draw_paths = executor.build_vector(draw_path_count, |path_index| {
            self.build_draw_path(DrawPathBuildParams {
                path_build_params: PathBuildParams {
                    path_index,
                    view_box: effective_view_box,
                    built_options: &self.built_options,
                    scene: &self.scene,
                },
                paint_metadata: &paint_metadata,
                built_clip_paths: &built_clip_paths,
            })
        });

        self.finish_building(&paint_metadata, built_draw_paths);

        let cpu_build_time = Instant::now() - start_time;
        self.listener.send(RenderCommand::Finish { cpu_build_time });
    }

    fn build_clip_path(&self, params: PathBuildParams) -> BuiltPath {
        let PathBuildParams { path_index, view_box, built_options, scene } = params;
        let path_object = &scene.clip_paths[path_index];
        let outline = scene.apply_render_options(path_object.outline(), built_options);

        let mut tiler = Tiler::new(self,
                                   &outline,
                                   path_object.fill_rule(),
                                   view_box,
                                   TilingPathInfo::Clip);

        tiler.generate_tiles();
        self.send_fills(tiler.object_builder.fills);
        tiler.object_builder.built_path
    }

    fn build_draw_path(&self, params: DrawPathBuildParams) -> BuiltDrawPath {
        let DrawPathBuildParams {
            path_build_params: PathBuildParams { path_index, view_box, built_options, scene },
            paint_metadata,
            built_clip_paths,
        } = params;

        let path_object = &scene.paths[path_index];
        let outline = scene.apply_render_options(path_object.outline(), built_options);

        let paint_id = path_object.paint();
        let paint_metadata = &paint_metadata[paint_id.0 as usize];
        let built_clip_path = path_object.clip_path().map(|clip_path_id| {
            &built_clip_paths[clip_path_id.0 as usize]
        });

        let mut tiler = Tiler::new(self,
                                   &outline,
                                   path_object.fill_rule(),
                                   view_box,
                                   TilingPathInfo::Draw(DrawTilingPathInfo {
            paint_id,
            paint_metadata,
            blend_mode: path_object.blend_mode(),
            built_clip_path,
            fill_rule: path_object.fill_rule(),
        }));

        tiler.generate_tiles();
        self.send_fills(tiler.object_builder.fills);
        BuiltDrawPath {
            path: tiler.object_builder.built_path,
            blend_mode: path_object.blend_mode(),
            filter: paint_metadata.filter(),
            color_texture: paint_metadata.tile_batch_texture(),
            sampling_flags_1: TextureSamplingFlags::empty(),
            mask_0_fill_rule: path_object.fill_rule(),
        }
    }

    fn send_fills(&self, fills: Vec<FillBatchEntry>) {
        if !fills.is_empty() {
            self.listener.send(RenderCommand::AddFills(fills));
        }
    }

    fn build_clips(&self, built_draw_paths: &[BuiltDrawPath]) {
        let mut built_clip_tiles = vec![];
        for built_draw_path in built_draw_paths {
            for built_clip_tile in &built_draw_path.path.clip_tiles {
                built_clip_tiles.push(*built_clip_tile);
            }
        }

        built_clip_tiles.sort_by_key(|built_clip_tile| built_clip_tile.key);

        let mut batches: Vec<ClipBatch> = vec![];
        for built_clip_tile in built_clip_tiles {
            if batches.is_empty() || batches.last_mut().unwrap().key != built_clip_tile.key {
                batches.push(ClipBatch { key: built_clip_tile.key, clips: vec![] });
            }
            batches.last_mut().unwrap().clips.push(built_clip_tile.clip);
        }

        if !batches.is_empty() {
            self.listener.send(RenderCommand::ClipTiles(batches));
        }
    }

    fn cull_tiles(&self, paint_metadata: &[PaintMetadata], built_draw_paths: Vec<BuiltDrawPath>)
                  -> CulledTiles {
        let mut culled_tiles = CulledTiles { display_list: vec![] };

        let mut remaining_layer_z_buffers = self.build_solid_tiles(&built_draw_paths);
        remaining_layer_z_buffers.reverse();

        // Process first Z-buffer.
        let first_z_buffer = remaining_layer_z_buffers.pop().unwrap();
        let first_solid_tiles = first_z_buffer.build_solid_tiles(paint_metadata);
        for batch in first_solid_tiles.batches {
            culled_tiles.display_list.push(CulledDisplayItem::DrawTiles(batch));
        }

        let mut layer_z_buffers_stack = vec![first_z_buffer];
        let mut current_depth = 1;

        for display_item in &self.scene.display_list {
            match *display_item {
                DisplayItem::PushRenderTarget(render_target_id) => {
                    culled_tiles.display_list
                                .push(CulledDisplayItem::PushRenderTarget(render_target_id));

                    let z_buffer = remaining_layer_z_buffers.pop().unwrap();
                    let solid_tiles = z_buffer.build_solid_tiles(paint_metadata);
                    for batch in solid_tiles.batches {
                        culled_tiles.display_list.push(CulledDisplayItem::DrawTiles(batch));
                    }
                    layer_z_buffers_stack.push(z_buffer);
                }

                DisplayItem::PopRenderTarget => {
                    culled_tiles.display_list.push(CulledDisplayItem::PopRenderTarget);
                    layer_z_buffers_stack.pop();
                }

                DisplayItem::DrawPaths {
                    start_index: start_draw_path_index,
                    end_index: end_draw_path_index,
                } => {
                    for draw_path_index in start_draw_path_index..end_draw_path_index {
                        let built_draw_path = &built_draw_paths[draw_path_index as usize];
                        let layer_z_buffer = layer_z_buffers_stack.last().unwrap();
                        let color_texture = built_draw_path.color_texture;

                        debug_assert!(built_draw_path.path.empty_tiles.is_empty() ||
                                      built_draw_path.blend_mode.is_destructive());
                        self.add_alpha_tiles(&mut culled_tiles,
                                             layer_z_buffer,
                                             &built_draw_path.path.empty_tiles,
                                             current_depth,
                                             None,
                                             built_draw_path.blend_mode,
                                             built_draw_path.filter);

                        self.add_alpha_tiles(&mut culled_tiles,
                                             layer_z_buffer,
                                             &built_draw_path.path.single_mask_tiles,
                                             current_depth,
                                             color_texture,
                                             built_draw_path.blend_mode,
                                             built_draw_path.filter);

                        match built_draw_path.path.solid_tiles {
                            SolidTiles::Regular(ref tiles) => {
                                self.add_alpha_tiles(&mut culled_tiles,
                                                     layer_z_buffer,
                                                     tiles,
                                                     current_depth,
                                                     color_texture,
                                                     built_draw_path.blend_mode,
                                                     built_draw_path.filter);
                            }
                            SolidTiles::Occluders(_) => {}
                        }

                        current_depth += 1;
                    }
                }
            }
        }

        culled_tiles
    }

    fn build_solid_tiles(&self, built_draw_paths: &[BuiltDrawPath]) -> Vec<ZBuffer> {
        let effective_view_box = self.scene.effective_view_box(self.built_options);
        let mut z_buffers = vec![ZBuffer::new(effective_view_box)];
        let mut z_buffer_index_stack = vec![0];
        let mut current_depth = 1;

        // Create Z-buffers.
        for display_item in &self.scene.display_list {
            match *display_item {
                DisplayItem::PushRenderTarget { .. } => {
                    z_buffer_index_stack.push(z_buffers.len());
                    z_buffers.push(ZBuffer::new(effective_view_box));
                }
                DisplayItem::PopRenderTarget => {
                    z_buffer_index_stack.pop();
                }
                DisplayItem::DrawPaths { start_index, end_index } => {
                    let (start_index, end_index) = (start_index as usize, end_index as usize);
                    let z_buffer = &mut z_buffers[*z_buffer_index_stack.last().unwrap()];
                    for (path_subindex, built_draw_path) in
                            built_draw_paths[start_index..end_index].iter().enumerate() {
                        let path_index = (path_subindex + start_index) as u32;
                        let path = &self.scene.paths[path_index as usize];
                        let metadata = DepthMetadata { paint_id: path.paint() };
                        match built_draw_path.path.solid_tiles {
                            SolidTiles::Regular(_) => {
                                z_buffer.update(&[], current_depth, metadata);
                            }
                            SolidTiles::Occluders(ref occluders) => {
                                z_buffer.update(occluders, current_depth, metadata);
                            }
                        }
                        current_depth += 1;
                    }
                }
            }
        }
        debug_assert_eq!(z_buffer_index_stack.len(), 1);

        z_buffers
    }

    fn add_alpha_tiles(&self,
                       culled_tiles: &mut CulledTiles,
                       layer_z_buffer: &ZBuffer,
                       built_alpha_tiles: &[BuiltTile],
                       current_depth: u32,
                       color_texture: Option<TileBatchTexture>,
                       blend_mode: BlendMode,
                       filter: Filter) {
        let mut batch_indices: Vec<BatchIndex> = vec![];
        for built_alpha_tile in built_alpha_tiles {
            // Early cull if possible.
            let alpha_tile_coords = built_alpha_tile.tile.tile_position();
            if !layer_z_buffer.test(alpha_tile_coords, current_depth) {
                continue;
            }

            // Find an appropriate batch if we can.
            let mut dest_batch_index = batch_indices.iter().filter(|&batch_index| {
                batch_index.tile_page == built_alpha_tile.page
            }).next().cloned();

            // If no batch was found, try to reuse the last batch in the display list.
            //
            // TODO(pcwalton): We could try harder to find a batch by taking tile positions into
            // account...
            if dest_batch_index.is_none() {
                match culled_tiles.display_list.last() {
                    Some(&CulledDisplayItem::DrawTiles(TileBatch {
                        tiles: _,
                        color_texture: ref batch_color_texture,
                        blend_mode: batch_blend_mode,
                        filter: batch_filter,
                        tile_page: batch_tile_page
                    })) if *batch_color_texture == color_texture &&
                            batch_blend_mode == blend_mode &&
                            batch_filter == filter &&
                            !batch_blend_mode.needs_readable_framebuffer() &&
                            batch_tile_page == built_alpha_tile.page => {
                        dest_batch_index = Some(BatchIndex {
                            display_item_index: culled_tiles.display_list.len() - 1,
                            tile_page: batch_tile_page,
                        });
                        batch_indices.push(dest_batch_index.unwrap());
                    }
                    _ => {}
                }
            }

            // If it's still the case that no suitable batch was found, then make a new one.
            if dest_batch_index.is_none() {
                dest_batch_index = Some(BatchIndex {
                    display_item_index: culled_tiles.display_list.len(),
                    tile_page: built_alpha_tile.page,
                });
                batch_indices.push(dest_batch_index.unwrap());
                culled_tiles.display_list.push(CulledDisplayItem::DrawTiles(TileBatch {
                    tiles: vec![],
                    color_texture,
                    blend_mode,
                    filter,
                    tile_page: built_alpha_tile.page,
                }));
            }

            // Add to the appropriate batch.
            match culled_tiles.display_list[dest_batch_index.unwrap().display_item_index] {
                CulledDisplayItem::DrawTiles(ref mut tiles) => {
                    tiles.tiles.push(built_alpha_tile.tile);
                }
                _ => unreachable!(),
            }
        }

        #[derive(Clone, Copy)]
        struct BatchIndex {
            display_item_index: usize,
            tile_page: u16,
        }
    }

    fn pack_tiles(&mut self, culled_tiles: CulledTiles) {
        self.listener.send(RenderCommand::BeginTileDrawing);
        for display_item in culled_tiles.display_list {
            match display_item {
                CulledDisplayItem::DrawTiles(batch) => {
                    self.listener.send(RenderCommand::DrawTiles(batch))
                }
                CulledDisplayItem::PushRenderTarget(render_target_id) => {
                    self.listener.send(RenderCommand::PushRenderTarget(render_target_id))
                }
                CulledDisplayItem::PopRenderTarget => {
                    self.listener.send(RenderCommand::PopRenderTarget)
                }
            }
        }
    }

    fn finish_building(&mut self,
                       paint_metadata: &[PaintMetadata],
                       built_draw_paths: Vec<BuiltDrawPath>) {
        self.listener.send(RenderCommand::FlushFills);
        self.build_clips(&built_draw_paths);
        let culled_tiles = self.cull_tiles(paint_metadata, built_draw_paths);
        self.pack_tiles(culled_tiles);
    }

    fn needs_readable_framebuffer(&self) -> bool {
        let mut framebuffer_nesting = 0;
        for display_item in &self.scene.display_list {
            match *display_item {
                DisplayItem::PushRenderTarget(_) => framebuffer_nesting += 1,
                DisplayItem::PopRenderTarget => framebuffer_nesting -= 1,
                DisplayItem::DrawPaths { start_index, end_index } => {
                    if framebuffer_nesting > 0 {
                        continue;
                    }
                    for path_index in start_index..end_index {
                        let blend_mode = self.scene.paths[path_index as usize].blend_mode();
                        if blend_mode.needs_readable_framebuffer() {
                            return true;
                        }
                    }
                }
            }
        }
        false
    }
}

struct PathBuildParams<'a> {
    path_index: usize,
    view_box: RectF,
    built_options: &'a PreparedBuildOptions,
    scene: &'a Scene,
}

struct DrawPathBuildParams<'a> {
    path_build_params: PathBuildParams<'a>,
    paint_metadata: &'a [PaintMetadata],
    built_clip_paths: &'a [BuiltPath],
}

impl BuiltPath {
    fn new(path_bounds: RectF,
           view_box_bounds: RectF,
           fill_rule: FillRule,
           tiling_path_info: &TilingPathInfo)
           -> BuiltPath {
        let occludes = match *tiling_path_info {
            TilingPathInfo::Draw(ref draw_tiling_path_info) => {
                draw_tiling_path_info.paint_metadata.is_opaque &&
                    draw_tiling_path_info.blend_mode.occludes_backdrop()
            }
            TilingPathInfo::Clip => true,
        };

        let tile_map_bounds = if tiling_path_info.has_destructive_blend_mode() {
            view_box_bounds
        } else {
            path_bounds
        };

        BuiltPath {
            empty_tiles: vec![],
            single_mask_tiles: vec![],
            clip_tiles: vec![],
            solid_tiles: if occludes {
                SolidTiles::Occluders(vec![])
            } else {
                SolidTiles::Regular(vec![])
            },
            tiles: DenseTileMap::new(tiles::round_rect_out_to_tile_bounds(tile_map_bounds)),
            fill_rule,
        }
    }
}

impl Occluder {
    #[inline]
    pub(crate) fn new(coords: Vector2I) -> Occluder {
        Occluder { coords }
    }
}

struct CulledTiles {
    display_list: Vec<CulledDisplayItem>,
}

enum CulledDisplayItem {
    DrawTiles(TileBatch),
    PushRenderTarget(RenderTargetId),
    PopRenderTarget,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct TileStats {
    pub solid_tile_count: u32,
    pub alpha_tile_count: u32,
}

// Utilities for built objects

impl ObjectBuilder {
    pub(crate) fn new(path_bounds: RectF,
                      view_box_bounds: RectF,
                      fill_rule: FillRule,
                      tiling_path_info: &TilingPathInfo)
                      -> ObjectBuilder {
        ObjectBuilder {
            built_path: BuiltPath::new(path_bounds, view_box_bounds, fill_rule, tiling_path_info),
            bounds: path_bounds,
            fills: vec![],
        }
    }

    #[inline]
    pub(crate) fn tile_rect(&self) -> RectI {
        self.built_path.tiles.rect
    }

    fn add_fill(
        &mut self,
        scene_builder: &SceneBuilder,
        segment: LineSegment2F,
        tile_coords: Vector2I,
    ) {
        debug!("add_fill({:?} ({:?}))", segment, tile_coords);

        // Ensure this fill is in bounds. If not, cull it.
        if self.tile_coords_to_local_index(tile_coords).is_none() {
            return;
        }

        debug_assert_eq!(TILE_WIDTH, TILE_HEIGHT);

        // Compute the upper left corner of the tile.
        let tile_size = F32x4::splat(TILE_WIDTH as f32);
        let tile_upper_left = tile_coords.to_f32().0.to_f32x4().xyxy() * tile_size;

        // Convert to 4.8 fixed point.
        let segment = (segment.0 - tile_upper_left) * F32x4::splat(256.0);
        let (min, max) = (F32x4::default(), F32x4::splat((TILE_WIDTH * 256 - 1) as f32));
        let segment = segment.clamp(min, max).to_i32x4();
        let (from_x, from_y, to_x, to_y) = (segment[0], segment[1], segment[2], segment[3]);

        // Cull degenerate fills.
        if from_x == to_x {
            debug!("... culling!");
            return;
        }

        // Allocate a global tile if necessary.
        let alpha_tile_id = self.get_or_allocate_alpha_tile_index(scene_builder, tile_coords);

        // Pack whole pixels.
        let px = (segment & I32x4::splat(0xf00)).to_u32x4();
        let px = (px >> 8).to_i32x4() | (px >> 4).to_i32x4().yxwz();

        // Pack instance data.
        debug!("... OK, pushing");
        self.fills.push(FillBatchEntry {
            page: alpha_tile_id.page(),
            fill: Fill {
                px: LineSegmentU4 { from: px[0] as AlignedU8, to: px[2] as AlignedU8 },
                subpx: LineSegmentU8 {
                    from_x: from_x as u8,
                    from_y: from_y as u8,
                    to_x:   to_x   as u8,
                    to_y:   to_y   as u8,
                },
                alpha_tile_index: alpha_tile_id.tile() as AlignedU16,
            },
        });
    }

    fn get_or_allocate_alpha_tile_index(
        &mut self,
        scene_builder: &SceneBuilder,
        tile_coords: Vector2I,
    ) -> AlphaTileId {
        let local_tile_index = self.built_path.tiles.coords_to_index_unchecked(tile_coords);
        let alpha_tile_id = self.built_path.tiles.data[local_tile_index].alpha_tile_id;
        if alpha_tile_id.is_valid() {
            return alpha_tile_id;
        }

        let alpha_tile_id = AlphaTileId::new(&scene_builder.next_alpha_tile_indices, 0);
        self.built_path.tiles.data[local_tile_index].alpha_tile_id = alpha_tile_id;
        alpha_tile_id
    }

    pub(crate) fn add_active_fill(
        &mut self,
        scene_builder: &SceneBuilder,
        left: f32,
        right: f32,
        mut winding: i32,
        tile_coords: Vector2I,
    ) {
        let tile_origin_y = (tile_coords.y() * TILE_HEIGHT as i32) as f32;
        let left = vec2f(left, tile_origin_y);
        let right = vec2f(right, tile_origin_y);

        let segment = if winding < 0 {
            LineSegment2F::new(left, right)
        } else {
            LineSegment2F::new(right, left)
        };

        debug!(
            "... emitting active fill {} -> {} winding {} @ tile {:?}",
            left.x(),
            right.x(),
            winding,
            tile_coords
        );

        while winding != 0 {
            self.add_fill(scene_builder, segment, tile_coords);
            if winding < 0 {
                winding += 1
            } else {
                winding -= 1
            }
        }
    }

    pub(crate) fn generate_fill_primitives_for_line(
        &mut self,
        scene_builder: &SceneBuilder,
        mut segment: LineSegment2F,
        tile_y: i32,
    ) {
        debug!(
            "... generate_fill_primitives_for_line(): segment={:?} tile_y={} ({}-{})",
            segment,
            tile_y,
            tile_y as f32 * TILE_HEIGHT as f32,
            (tile_y + 1) as f32 * TILE_HEIGHT as f32
        );

        let winding = segment.from_x() > segment.to_x();
        let (segment_left, segment_right) = if !winding {
            (segment.from_x(), segment.to_x())
        } else {
            (segment.to_x(), segment.from_x())
        };

        let mut subsegment_x = (segment_left as i32 & !(TILE_WIDTH as i32 - 1)) as f32;
        while subsegment_x < segment_right {
            let (mut fill_from, mut fill_to) = (segment.from(), segment.to());
            let subsegment_x_next = subsegment_x + TILE_WIDTH as f32;
            if subsegment_x_next < segment_right {
                let x = subsegment_x_next;
                let point = Vector2F::new(x, segment.solve_y_for_x(x));
                if !winding {
                    fill_to = point;
                    segment = LineSegment2F::new(point, segment.to());
                } else {
                    fill_from = point;
                    segment = LineSegment2F::new(segment.from(), point);
                }
            }

            let fill_segment = LineSegment2F::new(fill_from, fill_to);
            let fill_tile_coords = vec2i(subsegment_x as i32 / TILE_WIDTH as i32, tile_y);
            self.add_fill(scene_builder, fill_segment, fill_tile_coords);

            subsegment_x = subsegment_x_next;
        }
    }

    #[inline]
    pub(crate) fn tile_coords_to_local_index(&self, coords: Vector2I) -> Option<u32> {
        self.built_path.tiles.coords_to_index(coords).map(|index| index as u32)
    }

    #[inline]
    pub(crate) fn local_tile_index_to_coords(&self, tile_index: u32) -> Vector2I {
        self.built_path.tiles.index_to_coords(tile_index as usize)
    }
}

impl<'a> PackedTile<'a> {
    pub(crate) fn add_to(&self,
                         tiles: &mut Vec<BuiltTile>,
                         clips: &mut Vec<BuiltClip>,
                         draw_tiling_path_info: &DrawTilingPathInfo,
                         scene_builder: &SceneBuilder) {
        let draw_tile_page = self.draw_tile.alpha_tile_id.page() as u16;
        let draw_tile_index = self.draw_tile.alpha_tile_id.tile() as u16;
        let draw_tile_backdrop = self.draw_tile.backdrop as i8;

        match self.clip_tile {
            None => {
                tiles.push(BuiltTile {
                    page: draw_tile_page,
                    tile: Tile::new_alpha(self.tile_coords,
                                          draw_tile_index,
                                          draw_tile_backdrop,
                                          draw_tiling_path_info),
                });
            }
            Some(clip_tile) => {
                let clip_tile_page = clip_tile.alpha_tile_id.page() as u16;
                let clip_tile_index = clip_tile.alpha_tile_id.tile() as u16;
                let clip_tile_backdrop = clip_tile.backdrop;

                let dest_tile_id = AlphaTileId::new(&scene_builder.next_alpha_tile_indices, 1);
                let dest_tile_page = dest_tile_id.page() as u16;
                let dest_tile_index = dest_tile_id.tile() as u16;

                clips.push(BuiltClip {
                    clip: Clip::new(dest_tile_index, draw_tile_index, draw_tile_backdrop),
                    key: ClipBatchKey {
                        src_page: draw_tile_page,
                        dest_page: dest_tile_page,
                        kind: ClipBatchKind::Draw,
                    },
                });
                clips.push(BuiltClip {
                    clip: Clip::new(dest_tile_index, clip_tile_index, clip_tile_backdrop),
                    key: ClipBatchKey {
                        src_page: clip_tile_page,
                        dest_page: dest_tile_page,
                        kind: ClipBatchKind::Clip,
                    },
                });
                tiles.push(BuiltTile {
                    page: dest_tile_page,
                    tile: Tile::new_alpha(self.tile_coords,
                                          dest_tile_index,
                                          0,
                                          draw_tiling_path_info),
                });
            }
        }
    }
}

impl Tile {
    #[inline]
    fn new_alpha(tile_origin: Vector2I,
                 draw_tile_index: u16,
                 draw_tile_backdrop: i8,
                 draw_tiling_path_info: &DrawTilingPathInfo)
                 -> Tile {
        let mask_0_uv = calculate_mask_uv(draw_tile_index);

        let mut ctrl = 0;
        match draw_tiling_path_info.fill_rule {
            FillRule::EvenOdd => ctrl |= TILE_CTRL_MASK_EVEN_ODD << TILE_CTRL_MASK_0_SHIFT,
            FillRule::Winding => ctrl |= TILE_CTRL_MASK_WINDING << TILE_CTRL_MASK_0_SHIFT,
        }

        Tile {
            tile_x: tile_origin.x() as AlignedI16,
            tile_y: tile_origin.y() as AlignedI16,
            mask_0_u: mask_0_uv.x() as AlignedU8,
            mask_0_v: mask_0_uv.y() as AlignedU8,
            mask_0_backdrop: draw_tile_backdrop as AlignedI8,
            ctrl: ctrl as AlignedU16,
            pad: 0,
            color: draw_tiling_path_info.paint_id.0 as AlignedU16,
        }
    }

    #[inline]
    pub fn tile_position(&self) -> Vector2I {
        vec2i(self.tile_x as i32, self.tile_y as i32)
    }
}

impl Clip {
    #[inline]
    fn new(dest_tile_index: u16, src_tile_index: u16, src_backdrop: i8) -> Clip {
        let dest_uv = calculate_mask_uv(dest_tile_index);
        let src_uv = calculate_mask_uv(src_tile_index);
        Clip {
            dest_u: dest_uv.x() as AlignedU8,
            dest_v: dest_uv.y() as AlignedU8,
            src_u: src_uv.x() as AlignedU8,
            src_v: src_uv.y() as AlignedU8,
            backdrop: src_backdrop as AlignedI8,
            pad_0: 0,
            pad_1: 0,
        }
    }
}

fn calculate_mask_uv(tile_index: u16) -> Vector2I {
    debug_assert_eq!(MASK_TILES_ACROSS, MASK_TILES_DOWN);
    let mask_u = tile_index as i32 % MASK_TILES_ACROSS as i32;
    let mask_v = tile_index as i32 / MASK_TILES_ACROSS as i32;
    vec2i(mask_u, mask_v)
}
