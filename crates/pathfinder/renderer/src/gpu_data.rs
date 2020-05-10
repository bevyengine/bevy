// pathfinder/renderer/src/gpu_data.rs
//
// Copyright © 2020 The Pathfinder Project Developers.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Packed data ready to be sent to the GPU.

use crate::builder::{ALPHA_TILES_PER_LEVEL, ALPHA_TILE_LEVEL_COUNT};
use crate::options::BoundingQuad;
use crate::paint::PaintCompositeOp;
use pathfinder_color::ColorU;
use pathfinder_content::effects::{BlendMode, Filter};
use pathfinder_content::render_target::RenderTargetId;
use pathfinder_geometry::alignment::{AlignedI8, AlignedU8, AlignedI16, AlignedU16};
use pathfinder_geometry::line_segment::{LineSegmentU4, LineSegmentU8};
use pathfinder_geometry::rect::RectI;
use pathfinder_geometry::transform2d::Transform2F;
use pathfinder_geometry::vector::Vector2I;
use pathfinder_gpu::TextureSamplingFlags;
use std::fmt::{Debug, Formatter, Result as DebugResult};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;
use std::u32;

pub const TILE_CTRL_MASK_MASK:     i32 = 0x3;
pub const TILE_CTRL_MASK_WINDING:  i32 = 0x1;
pub const TILE_CTRL_MASK_EVEN_ODD: i32 = 0x2;

pub const TILE_CTRL_MASK_0_SHIFT:  i32 = 0;

pub enum RenderCommand {
    // Starts rendering a frame.
    Start {
        /// The number of paths that will be rendered.
        path_count: usize,

        /// A bounding quad for the scene.
        bounding_quad: BoundingQuad,

        /// Whether the framebuffer we're rendering to must be readable.
        ///
        /// This is needed if a path that renders directly to the output framebuffer (i.e. not to a
        /// render target) uses one of the more exotic blend modes.
        needs_readable_framebuffer: bool,
    },

    // Allocates a texture page.
    AllocateTexturePage { page_id: TexturePageId, descriptor: TexturePageDescriptor },

    // Uploads data to a texture page.
    UploadTexelData { texels: Arc<Vec<ColorU>>, location: TextureLocation },

    // Associates a render target with a texture page.
    //
    // TODO(pcwalton): Add a rect to this so we can render to subrects of a page.
    DeclareRenderTarget { id: RenderTargetId, location: TextureLocation },

    // Upload texture metadata.
    UploadTextureMetadata(Vec<TextureMetadataEntry>),

    // Adds fills to the queue.
    AddFills(Vec<FillBatchEntry>),

    // Flushes the queue of fills.
    FlushFills,

    // Renders clips to the mask tile.
    ClipTiles(Vec<ClipBatch>),

    // Pushes a render target onto the stack. Draw commands go to the render target on top of the
    // stack.
    PushRenderTarget(RenderTargetId),

    // Pops a render target from the stack.
    PopRenderTarget,

    // Marks that tile compositing is about to begin.
    BeginTileDrawing,

    // Draws a batch of tiles to the render target on top of the stack.
    DrawTiles(TileBatch),

    // Presents a rendered frame.
    Finish { cpu_build_time: Duration },
}

#[derive(Clone, Copy, PartialEq, Debug, Default)]
pub struct TexturePageId(pub u32);

#[derive(Clone, Copy, Debug)]
pub struct TexturePageDescriptor {
    pub size: Vector2I,
}

#[derive(Clone, Copy, PartialEq, Debug, Default)]
pub struct TextureLocation {
    pub page: TexturePageId,
    pub rect: RectI,
}

#[derive(Clone, Debug)]
pub struct TileBatch {
    pub tiles: Vec<Tile>,
    pub color_texture: Option<TileBatchTexture>,
    pub filter: Filter,
    pub blend_mode: BlendMode,
    pub tile_page: u16,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TileBatchTexture {
    pub page: TexturePageId,
    pub sampling_flags: TextureSamplingFlags,
    pub composite_op: PaintCompositeOp,
}

#[derive(Clone, Copy, Debug)]
pub struct FillObjectPrimitive {
    pub px: LineSegmentU4,
    pub subpx: LineSegmentU8,
    pub tile_x: i16,
    pub tile_y: i16,
}

#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct TileObjectPrimitive {
    pub alpha_tile_id: AlphaTileId,
    pub backdrop: i8,
}

#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct TextureMetadataEntry {
    pub color_0_transform: Transform2F,
    pub base_color: ColorU,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct FillBatchEntry {
    pub fill: Fill,
    pub page: u16,
}

#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct Fill {
    pub subpx: LineSegmentU8,
    pub px: LineSegmentU4,
    pub alpha_tile_index: AlignedU16,
}

#[derive(Clone, Debug)]
pub struct ClipBatch {
    pub clips: Vec<Clip>,
    pub key: ClipBatchKey,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct ClipBatchKey {
    pub dest_page: u16,
    pub src_page: u16,
    pub kind: ClipBatchKind,
}

// Order is significant here.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ClipBatchKind {
    Draw,
    Clip,
}

#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct Clip {
    pub dest_u: AlignedU8,
    pub dest_v: AlignedU8,
    pub src_u: AlignedU8,
    pub src_v: AlignedU8,
    pub backdrop: AlignedI8,
    pub pad_0: AlignedU8,
    pub pad_1: AlignedU16,
}

#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct Tile {
    pub tile_x: AlignedI16,
    pub tile_y: AlignedI16,
    pub mask_0_u: AlignedU8,
    pub mask_0_v: AlignedU8,
    pub mask_0_backdrop: AlignedI8,
    pub pad: AlignedU8,
    pub color: AlignedU16,
    pub ctrl: AlignedU16,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub struct AlphaTileId(pub u32);

impl AlphaTileId {
    #[inline]
    pub fn new(next_alpha_tile_index: &[AtomicUsize; ALPHA_TILE_LEVEL_COUNT], level: usize) 
               -> AlphaTileId {
        let alpha_tile_index = next_alpha_tile_index[level].fetch_add(1, Ordering::Relaxed);
        debug_assert!(alpha_tile_index < ALPHA_TILES_PER_LEVEL);
        AlphaTileId((level * ALPHA_TILES_PER_LEVEL + alpha_tile_index) as u32)
    }

    #[inline]
    pub fn invalid() -> AlphaTileId {
        AlphaTileId(!0)
    }

    #[inline]
    pub fn page(self) -> u16 {
        (self.0 >> 16) as u16
    }

    #[inline]
    pub fn tile(self) -> u16 {
        (self.0 & 0xffff) as u16
    }

    #[inline]
    pub fn is_valid(self) -> bool {
        self.0 < !0
    }
}

impl Debug for RenderCommand {
    fn fmt(&self, formatter: &mut Formatter) -> DebugResult {
        match *self {
            RenderCommand::Start { .. } => write!(formatter, "Start"),
            RenderCommand::AllocateTexturePage { page_id, descriptor: _ } => {
                write!(formatter, "AllocateTexturePage({})", page_id.0)
            }
            RenderCommand::UploadTexelData { ref texels, location } => {
                write!(formatter, "UploadTexelData(x{:?}, {:?})", texels.len(), location)
            }
            RenderCommand::DeclareRenderTarget { id, location } => {
                write!(formatter, "DeclareRenderTarget({:?}, {:?})", id, location)
            }
            RenderCommand::UploadTextureMetadata(ref metadata) => {
                write!(formatter, "UploadTextureMetadata(x{})", metadata.len())
            }
            RenderCommand::AddFills(ref fills) => {
                write!(formatter, "AddFills(x{})", fills.len())
            }
            RenderCommand::FlushFills => write!(formatter, "FlushFills"),
            RenderCommand::ClipTiles(ref batches) => {
                write!(formatter, "ClipTiles(x{})", batches.len())
            }
            RenderCommand::PushRenderTarget(render_target_id) => {
                write!(formatter, "PushRenderTarget({:?})", render_target_id)
            }
            RenderCommand::PopRenderTarget => write!(formatter, "PopRenderTarget"),
            RenderCommand::BeginTileDrawing => write!(formatter, "BeginTileDrawing"),
            RenderCommand::DrawTiles(ref batch) => {
                write!(formatter,
                       "DrawTiles(x{}, C0 {:?}, {:?})",
                       batch.tiles.len(),
                       batch.color_texture,
                       batch.blend_mode)
            }
            RenderCommand::Finish { cpu_build_time } => {
                write!(formatter, "Finish({} ms)", cpu_build_time.as_secs_f64() * 1000.0)
            }
        }
    }
}
