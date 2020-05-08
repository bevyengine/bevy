// pathfinder/renderer/src/tiles.rs
//
// Copyright © 2019 The Pathfinder Project Developers.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use crate::builder::{BuiltPath, ObjectBuilder, Occluder, SceneBuilder, SolidTiles};
use crate::gpu_data::{AlphaTileId, TileObjectPrimitive};
use crate::paint::{PaintId, PaintMetadata};
use pathfinder_content::effects::BlendMode;
use pathfinder_content::fill::FillRule;
use pathfinder_content::outline::{Contour, Outline, PointIndex};
use pathfinder_content::segment::Segment;
use pathfinder_content::sorted_vector::SortedVector;
use pathfinder_geometry::line_segment::LineSegment2F;
use pathfinder_geometry::rect::{RectF, RectI};
use pathfinder_geometry::vector::{Vector2F, Vector2I, vec2f, vec2i};
use std::cmp::Ordering;
use std::mem;

// TODO(pcwalton): Make this configurable.
const FLATTENING_TOLERANCE: f32 = 0.1;

pub const TILE_WIDTH: u32 = 16;
pub const TILE_HEIGHT: u32 = 16;

pub(crate) struct Tiler<'a, 'b> {
    scene_builder: &'a SceneBuilder<'b, 'a>,
    pub(crate) object_builder: ObjectBuilder,
    outline: &'a Outline,
    path_info: TilingPathInfo<'a>,

    point_queue: SortedVector<QueuedEndpoint>,
    active_edges: SortedVector<ActiveEdge>,
    old_active_edges: Vec<ActiveEdge>,
}

#[derive(Clone, Copy)]
pub(crate) enum TilingPathInfo<'a> {
    Clip,
    Draw(DrawTilingPathInfo<'a>),
}

#[derive(Clone, Copy)]
pub(crate) struct DrawTilingPathInfo<'a> {
    pub(crate) paint_id: PaintId,
    pub(crate) paint_metadata: &'a PaintMetadata,
    pub(crate) blend_mode: BlendMode,
    pub(crate) built_clip_path: Option<&'a BuiltPath>,
    pub(crate) fill_rule: FillRule,
}

impl<'a, 'b> Tiler<'a, 'b> {
    #[allow(clippy::or_fun_call)]
    pub(crate) fn new(
        scene_builder: &'a SceneBuilder<'b, 'a>,
        outline: &'a Outline,
        fill_rule: FillRule,
        view_box: RectF,
        path_info: TilingPathInfo<'a>,
    ) -> Tiler<'a, 'b> {
        let bounds = outline
            .bounds()
            .intersection(view_box)
            .unwrap_or(RectF::default());
        let object_builder = ObjectBuilder::new(bounds, view_box, fill_rule, &path_info);

        Tiler {
            scene_builder,
            object_builder,
            outline,
            path_info,

            point_queue: SortedVector::new(),
            active_edges: SortedVector::new(),
            old_active_edges: vec![],
        }
    }

    pub(crate) fn generate_tiles(&mut self) {
        // Initialize the point queue.
        self.init_point_queue();

        // Reset active edges.
        self.active_edges.clear();
        self.old_active_edges.clear();

        // Generate strips.
        let tile_rect = self.object_builder.tile_rect();
        for strip_origin_y in tile_rect.min_y()..tile_rect.max_y() {
            self.generate_strip(strip_origin_y);
        }

        // Pack and cull.
        self.pack_and_cull();

        // Done!
        debug!("{:#?}", self.object_builder.built_path);
    }

    fn generate_strip(&mut self, strip_origin_y: i32) {
        // Process old active edges.
        self.process_old_active_edges(strip_origin_y);

        // Add new active edges.
        let strip_max_y = ((i32::from(strip_origin_y) + 1) * TILE_HEIGHT as i32) as f32;
        while let Some(queued_endpoint) = self.point_queue.peek() {
            // We're done when we see an endpoint that belongs to the next tile strip.
            //
            // Note that this test must be `>`, not `>=`, in order to make sure we don't miss
            // active edges that lie precisely on the tile strip boundary.
            if queued_endpoint.y > strip_max_y {
                break;
            }

            self.add_new_active_edge(strip_origin_y);
        }
    }

    fn pack_and_cull(&mut self) {
        let draw_tiling_path_info = match self.path_info {
            TilingPathInfo::Clip => return,
            TilingPathInfo::Draw(draw_tiling_path_info) => draw_tiling_path_info,
        };

        let blend_mode_is_destructive = draw_tiling_path_info.blend_mode.is_destructive();

        for (draw_tile_index, draw_tile) in self.object_builder
                                                .built_path
                                                .tiles
                                                .data
                                                .iter()
                                                .enumerate() {
            let packed_tile = PackedTile::new(draw_tile_index as u32,
                                              draw_tile,
                                              &draw_tiling_path_info,
                                              &self.object_builder);

            match packed_tile.tile_type {
                TileType::Solid => {
                    match self.object_builder.built_path.solid_tiles {
                        SolidTiles::Occluders(ref mut occluders) => {
                            occluders.push(Occluder::new(packed_tile.tile_coords));
                        }
                        SolidTiles::Regular(ref mut solid_tiles) => {
                            packed_tile.add_to(solid_tiles,
                                               &mut self.object_builder.built_path.clip_tiles,
                                               &draw_tiling_path_info,
                                               &self.scene_builder);
                        }
                    }
                }
                TileType::SingleMask => {
                    debug_assert_ne!(packed_tile.draw_tile.alpha_tile_id.page(), !0);
                    packed_tile.add_to(&mut self.object_builder.built_path.single_mask_tiles,
                                       &mut self.object_builder.built_path.clip_tiles,
                                       &draw_tiling_path_info,
                                       &self.scene_builder);
                }
                TileType::Empty if blend_mode_is_destructive => {
                    packed_tile.add_to(&mut self.object_builder.built_path.empty_tiles,
                                       &mut self.object_builder.built_path.clip_tiles,
                                       &draw_tiling_path_info,
                                       &self.scene_builder);
                }
                TileType::Empty => {
                    // Just cull.
                }
            }
        }
    }

    fn process_old_active_edges(&mut self, tile_y: i32) {
        let mut current_tile_x = self.object_builder.tile_rect().min_x();
        let mut current_subtile_x = 0.0;
        let mut current_winding = 0;

        debug_assert!(self.old_active_edges.is_empty());
        mem::swap(&mut self.old_active_edges, &mut self.active_edges.array);

        // FIXME(pcwalton): Yuck.
        let mut last_segment_x = -9999.0;

        let tile_top = (i32::from(tile_y) * TILE_HEIGHT as i32) as f32;

        debug!("---------- tile y {}({}) ----------", tile_y, tile_top);
        debug!("old active edges: {:#?}", self.old_active_edges);

        for mut active_edge in self.old_active_edges.drain(..) {
            // Determine x-intercept and winding.
            let segment_x = active_edge.crossing.x();
            let edge_winding =
                if active_edge.segment.baseline.from_y() < active_edge.segment.baseline.to_y() {
                    1
                } else {
                    -1
                };

            debug!(
                "tile Y {}({}): segment_x={} edge_winding={} current_tile_x={} \
                 current_subtile_x={} current_winding={}",
                tile_y,
                tile_top,
                segment_x,
                edge_winding,
                current_tile_x,
                current_subtile_x,
                current_winding
            );
            debug!(
                "... segment={:#?} crossing={:?}",
                active_edge.segment, active_edge.crossing
            );

            // FIXME(pcwalton): Remove this debug code!
            debug_assert!(segment_x >= last_segment_x);
            last_segment_x = segment_x;

            // Do initial subtile fill, if necessary.
            let segment_tile_x = f32::floor(segment_x) as i32 / TILE_WIDTH as i32;
            if current_tile_x < segment_tile_x && current_subtile_x > 0.0 {
                let current_x =
                    (i32::from(current_tile_x) * TILE_WIDTH as i32) as f32 + current_subtile_x;
                let tile_right_x = ((i32::from(current_tile_x) + 1) * TILE_WIDTH as i32) as f32;
                let current_tile_coords = vec2i(current_tile_x, tile_y);
                self.object_builder.add_active_fill(
                    self.scene_builder,
                    current_x,
                    tile_right_x,
                    current_winding,
                    current_tile_coords,
                );
                current_tile_x += 1;
                current_subtile_x = 0.0;
            }

            // Move over to the correct tile, filling in as we go.
            while current_tile_x < segment_tile_x {
                debug!(
                    "... emitting backdrop {} @ tile {}",
                    current_winding, current_tile_x
                );
                let current_tile_coords = vec2i(current_tile_x, tile_y);
                if let Some(tile_index) = self.object_builder
                                              .tile_coords_to_local_index(current_tile_coords) {
                    // FIXME(pcwalton): Handle winding overflow.
                    self.object_builder.built_path.tiles.data[tile_index as usize].backdrop =
                        current_winding as i8;
                }

                current_tile_x += 1;
                current_subtile_x = 0.0;
            }

            // Do final subtile fill, if necessary.
            debug_assert_eq!(current_tile_x, segment_tile_x);
            let segment_subtile_x =
                segment_x - (i32::from(current_tile_x) * TILE_WIDTH as i32) as f32;
            if segment_subtile_x > current_subtile_x {
                let current_x =
                    (i32::from(current_tile_x) * TILE_WIDTH as i32) as f32 + current_subtile_x;
                let current_tile_coords = vec2i(current_tile_x, tile_y);
                self.object_builder.add_active_fill(
                    self.scene_builder,
                    current_x,
                    segment_x,
                    current_winding,
                    current_tile_coords,
                );
                current_subtile_x = segment_subtile_x;
            }

            // Update winding.
            current_winding += edge_winding;

            // Process the edge.
            debug!("about to process existing active edge {:#?}", active_edge);
            debug_assert!(f32::abs(active_edge.crossing.y() - tile_top) < 0.1);
            active_edge.process(self.scene_builder, &mut self.object_builder, tile_y);
            if !active_edge.segment.is_none() {
                self.active_edges.push(active_edge);
            }
        }
    }

    fn add_new_active_edge(&mut self, tile_y: i32) {
        let outline = &self.outline;
        let point_index = self.point_queue.pop().unwrap().point_index;

        let contour = &outline.contours()[point_index.contour() as usize];

        // TODO(pcwalton): Could use a bitset of processed edges…
        let prev_endpoint_index = contour.prev_endpoint_index_of(point_index.point());
        let next_endpoint_index = contour.next_endpoint_index_of(point_index.point());

        debug!(
            "adding new active edge, tile_y={} point_index={} prev={} next={} pos={:?} \
             prevpos={:?} nextpos={:?}",
            tile_y,
            point_index.point(),
            prev_endpoint_index,
            next_endpoint_index,
            contour.position_of(point_index.point()),
            contour.position_of(prev_endpoint_index),
            contour.position_of(next_endpoint_index)
        );

        if contour.point_is_logically_above(point_index.point(), prev_endpoint_index) {
            debug!("... adding prev endpoint");

            process_active_segment(
                contour,
                prev_endpoint_index,
                &mut self.active_edges,
                self.scene_builder,
                &mut self.object_builder,
                tile_y,
            );

            self.point_queue.push(QueuedEndpoint {
                point_index: PointIndex::new(point_index.contour(), prev_endpoint_index),
                y: contour.position_of(prev_endpoint_index).y(),
            });

            debug!("... done adding prev endpoint");
        }

        if contour.point_is_logically_above(point_index.point(), next_endpoint_index) {
            debug!(
                "... adding next endpoint {} -> {}",
                point_index.point(),
                next_endpoint_index
            );

            process_active_segment(
                contour,
                point_index.point(),
                &mut self.active_edges,
                self.scene_builder,
                &mut self.object_builder,
                tile_y,
            );

            self.point_queue.push(QueuedEndpoint {
                point_index: PointIndex::new(point_index.contour(), next_endpoint_index),
                y: contour.position_of(next_endpoint_index).y(),
            });

            debug!("... done adding next endpoint");
        }
    }

    fn init_point_queue(&mut self) {
        // Find MIN points.
        self.point_queue.clear();
        for (contour_index, contour) in self.outline.contours().iter().enumerate() {
            let contour_index = contour_index as u32;
            let mut cur_endpoint_index = 0;
            let mut prev_endpoint_index = contour.prev_endpoint_index_of(cur_endpoint_index);
            let mut next_endpoint_index = contour.next_endpoint_index_of(cur_endpoint_index);
            loop {
                if contour.point_is_logically_above(cur_endpoint_index, prev_endpoint_index)
                    && contour.point_is_logically_above(cur_endpoint_index, next_endpoint_index)
                {
                    self.point_queue.push(QueuedEndpoint {
                        point_index: PointIndex::new(contour_index, cur_endpoint_index),
                        y: contour.position_of(cur_endpoint_index).y(),
                    });
                }

                if cur_endpoint_index >= next_endpoint_index {
                    break;
                }

                prev_endpoint_index = cur_endpoint_index;
                cur_endpoint_index = next_endpoint_index;
                next_endpoint_index = contour.next_endpoint_index_of(cur_endpoint_index);
            }
        }
    }
}

impl<'a> TilingPathInfo<'a> {
    pub(crate) fn has_destructive_blend_mode(&self) -> bool {
        match *self {
            TilingPathInfo::Draw(ref draw_tiling_path_info) => {
                draw_tiling_path_info.blend_mode.is_destructive()
            }
            TilingPathInfo::Clip => false,
        }
    }
}

pub(crate) struct PackedTile<'a> {
    pub(crate) tile_type: TileType,
    pub(crate) tile_coords: Vector2I,
    pub(crate) draw_tile: &'a TileObjectPrimitive,
    pub(crate) clip_tile: Option<&'a TileObjectPrimitive>,
}

#[derive(Clone, Copy, PartialEq)]
pub(crate) enum TileType {
    Solid,
    Empty,
    SingleMask,
}

impl<'a> PackedTile<'a> {
    fn new(draw_tile_index: u32,
           draw_tile: &'a TileObjectPrimitive,
           draw_tiling_path_info: &DrawTilingPathInfo<'a>,
           object_builder: &ObjectBuilder)
           -> PackedTile<'a> {
        let tile_coords = object_builder.local_tile_index_to_coords(draw_tile_index as u32);

        // First, if the draw tile is empty, cull it regardless of clip.
        if draw_tile.is_solid() {
            match (object_builder.built_path.fill_rule, draw_tile.backdrop) {
                (FillRule::Winding, 0) => {
                    return PackedTile {
                        tile_type: TileType::Empty,
                        tile_coords,
                        draw_tile,
                        clip_tile: None,
                    };
                }
                (FillRule::Winding, _) => {}
                (FillRule::EvenOdd, backdrop) if backdrop % 2 == 0 => {
                    return PackedTile {
                        tile_type: TileType::Empty,
                        tile_coords,
                        draw_tile,
                        clip_tile: None,
                    };
                }
                (FillRule::EvenOdd, _) => {}
            }
        }

        // Figure out what clip tile we need, if any.
        let clip_tile = match draw_tiling_path_info.built_clip_path {
            None => None,
            Some(built_clip_path) => {
                match built_clip_path.tiles.get(tile_coords) {
                    None => {
                        // This tile is outside of the bounds of the clip path entirely. We can
                        // cull it.
                        return PackedTile {
                            tile_type: TileType::Empty,
                            tile_coords,
                            draw_tile,
                            clip_tile: None,
                        };
                    }
                    Some(clip_tile) if clip_tile.is_solid() => {
                        if clip_tile.backdrop != 0 {
                            // The clip tile is fully opaque, so this tile isn't clipped at
                            // all.
                            None
                        } else {
                            // This tile is completely clipped out. Cull it.
                            return PackedTile {
                                tile_type: TileType::Empty,
                                tile_coords,
                                draw_tile,
                                clip_tile: None,
                            };
                        }
                    }
                    Some(clip_tile) => Some(clip_tile),
                }
            }
        };

        // Choose a tile type.
        match clip_tile {
            None if draw_tile.is_solid() => {
                // This is a solid tile that completely occludes the background.
                PackedTile { tile_type: TileType::Solid, tile_coords, draw_tile, clip_tile }
            }
            None => {
                // We have a draw tile and no clip tile.
                PackedTile {
                    tile_type: TileType::SingleMask,
                    tile_coords,
                    draw_tile,
                    clip_tile: None,
                }
            }
            Some(clip_tile) if draw_tile.is_solid() => {
                // We have a solid draw tile and a clip tile. This is effectively the same as
                // having a draw tile and no clip tile.
                //
                // FIXME(pcwalton): This doesn't preserve the fill rule of the clip path!
                PackedTile {
                    tile_type: TileType::SingleMask,
                    tile_coords,
                    draw_tile: clip_tile,
                    clip_tile: None,
                }
            }
            Some(clip_tile) => {
                // We have both a draw and clip mask. Composite them together.
                PackedTile {
                    tile_type: TileType::SingleMask,
                    tile_coords,
                    draw_tile,
                    clip_tile: Some(clip_tile),
                }
            }
        }
    }
}

pub fn round_rect_out_to_tile_bounds(rect: RectF) -> RectI {
    (rect * vec2f(1.0 / TILE_WIDTH as f32, 1.0 / TILE_HEIGHT as f32)).round_out().to_i32()
}

fn process_active_segment(
    contour: &Contour,
    from_endpoint_index: u32,
    active_edges: &mut SortedVector<ActiveEdge>,
    builder: &SceneBuilder,
    object_builder: &mut ObjectBuilder,
    tile_y: i32,
) {
    let mut active_edge = ActiveEdge::from_segment(&contour.segment_after(from_endpoint_index));
    debug!("... process_active_segment({:#?})", active_edge);
    active_edge.process(builder, object_builder, tile_y);
    if !active_edge.segment.is_none() {
        debug!("... ... pushing resulting active edge: {:#?}", active_edge);
        active_edges.push(active_edge);
    }
}

// Queued endpoints

#[derive(PartialEq)]
struct QueuedEndpoint {
    point_index: PointIndex,
    y: f32,
}

impl Eq for QueuedEndpoint {}

impl PartialOrd<QueuedEndpoint> for QueuedEndpoint {
    fn partial_cmp(&self, other: &QueuedEndpoint) -> Option<Ordering> {
        // NB: Reversed!
        (other.y, other.point_index).partial_cmp(&(self.y, self.point_index))
    }
}

// Active edges

#[derive(Clone, PartialEq, Debug)]
struct ActiveEdge {
    segment: Segment,
    // TODO(pcwalton): Shrink `crossing` down to just one f32?
    crossing: Vector2F,
}

impl ActiveEdge {
    fn from_segment(segment: &Segment) -> ActiveEdge {
        let crossing = if segment.baseline.from_y() < segment.baseline.to_y() {
            segment.baseline.from()
        } else {
            segment.baseline.to()
        };
        ActiveEdge::from_segment_and_crossing(segment, crossing)
    }

    fn from_segment_and_crossing(segment: &Segment, crossing: Vector2F) -> ActiveEdge {
        ActiveEdge { segment: *segment, crossing }
    }

    fn process(&mut self,
               builder: &SceneBuilder,
               object_builder: &mut ObjectBuilder,
               tile_y: i32) {
        let tile_bottom = ((i32::from(tile_y) + 1) * TILE_HEIGHT as i32) as f32;
        debug!(
            "process_active_edge({:#?}, tile_y={}({}))",
            self, tile_y, tile_bottom
        );

        let mut segment = self.segment;
        let winding = segment.baseline.y_winding();

        if segment.is_line() {
            let line_segment = segment.as_line_segment();
            self.segment =
                match self.process_line_segment(line_segment, builder, object_builder, tile_y) {
                    Some(lower_part) => Segment::line(lower_part),
                    None => Segment::none(),
                };
            return;
        }

        // TODO(pcwalton): Don't degree elevate!
        if !segment.is_cubic() {
            segment = segment.to_cubic();
        }

        // If necessary, draw initial line.
        if self.crossing.y() < segment.baseline.min_y() {
            let first_line_segment =
                LineSegment2F::new(self.crossing, segment.baseline.upper_point()).orient(winding);
            if self.process_line_segment(first_line_segment, builder, object_builder, tile_y)
                   .is_some() {
                return;
            }
        }

        let mut oriented_segment = segment.orient(winding);
        loop {
            let mut split_t = 1.0;
            let mut before_segment = oriented_segment;
            let mut after_segment = None;

            while !before_segment
                .as_cubic_segment()
                .is_flat(FLATTENING_TOLERANCE)
            {
                let next_t = 0.5 * split_t;
                let (before, after) = oriented_segment.as_cubic_segment().split(next_t);
                before_segment = before;
                after_segment = Some(after);
                split_t = next_t;
            }

            debug!(
                "... tile_y={} winding={} segment={:?} t={} before_segment={:?}
                    after_segment={:?}",
                tile_y, winding, segment, split_t, before_segment, after_segment
            );

            let line = before_segment.baseline.orient(winding);
            match self.process_line_segment(line, builder, object_builder, tile_y) {
                Some(lower_part) if split_t == 1.0 => {
                    self.segment = Segment::line(lower_part);
                    return;
                }
                None if split_t == 1.0 => {
                    self.segment = Segment::none();
                    return;
                }
                Some(_) => {
                    self.segment = after_segment.unwrap().orient(winding);
                    return;
                }
                None => oriented_segment = after_segment.unwrap(),
            }
        }
    }

    fn process_line_segment(
        &mut self,
        line_segment: LineSegment2F,
        builder: &SceneBuilder,
        object_builder: &mut ObjectBuilder,
        tile_y: i32,
    ) -> Option<LineSegment2F> {
        let tile_bottom = ((i32::from(tile_y) + 1) * TILE_HEIGHT as i32) as f32;
        debug!(
            "process_line_segment({:?}, tile_y={}) tile_bottom={}",
            line_segment, tile_y, tile_bottom
        );

        if line_segment.max_y() <= tile_bottom {
            object_builder.generate_fill_primitives_for_line(builder, line_segment, tile_y);
            return None;
        }

        let (upper_part, lower_part) = line_segment.split_at_y(tile_bottom);
        object_builder.generate_fill_primitives_for_line(builder, upper_part, tile_y);
        self.crossing = lower_part.upper_point();
        Some(lower_part)
    }
}

impl PartialOrd<ActiveEdge> for ActiveEdge {
    fn partial_cmp(&self, other: &ActiveEdge) -> Option<Ordering> {
        self.crossing.x().partial_cmp(&other.crossing.x())
    }
}

impl Default for TileObjectPrimitive {
    #[inline]
    fn default() -> TileObjectPrimitive {
        TileObjectPrimitive { backdrop: 0, alpha_tile_id: AlphaTileId::invalid() }
    }
}

impl TileObjectPrimitive {
    #[inline]
    pub fn is_solid(&self) -> bool { !self.alpha_tile_id.is_valid() }
}
