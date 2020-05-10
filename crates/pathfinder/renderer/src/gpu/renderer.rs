// pathfinder/renderer/src/gpu/renderer.rs
//
// Copyright © 2020 The Pathfinder Project Developers.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use crate::gpu::debug::DebugUIPresenter;
use crate::gpu::options::{DestFramebuffer, RendererOptions};
use crate::gpu::shaders::{BlitProgram, BlitVertexArray, ClearProgram, ClearVertexArray, ClipTileProgram, ClipTileVertexArray};
use crate::gpu::shaders::{CopyTileProgram, CopyTileVertexArray, FillProgram, FillVertexArray};
use crate::gpu::shaders::{MAX_FILLS_PER_BATCH, MAX_TILES_PER_BATCH, ReprojectionProgram};
use crate::gpu::shaders::{ReprojectionVertexArray, StencilProgram, StencilVertexArray};
use crate::gpu::shaders::{TileProgram, TileVertexArray};
use crate::gpu_data::{ClipBatch, ClipBatchKey, ClipBatchKind, Fill, FillBatchEntry, RenderCommand};
use crate::gpu_data::{TextureLocation, TextureMetadataEntry, TexturePageDescriptor, TexturePageId};
use crate::gpu_data::{Tile, TileBatchTexture};
use crate::options::BoundingQuad;
use crate::paint::PaintCompositeOp;
use crate::tiles::{TILE_HEIGHT, TILE_WIDTH};
use fxhash::FxHashMap;
use half::f16;
use pathfinder_color::{self as color, ColorF, ColorU};
use pathfinder_content::effects::{BlendMode, BlurDirection, DefringingKernel};
use pathfinder_content::effects::{Filter, PatternFilter};
use pathfinder_content::render_target::RenderTargetId;
use pathfinder_geometry::line_segment::LineSegment2F;
use pathfinder_geometry::rect::RectI;
use pathfinder_geometry::transform3d::Transform4F;
use pathfinder_geometry::util;
use pathfinder_geometry::{alignment::AlignedU16, vector::{Vector2F, Vector2I, Vector4F, vec2f, vec2i}};
use pathfinder_gpu::{BlendFactor, BlendOp, BlendState, BufferData, BufferTarget, BufferUploadMode};
use pathfinder_gpu::{ClearOps, ComputeDimensions, ComputeState, DepthFunc, DepthState, Device};
use pathfinder_gpu::{ImageAccess, ImageBinding, Primitive, RenderOptions, RenderState};
use pathfinder_gpu::{RenderTarget, StencilFunc, StencilState, TextureDataRef};
use pathfinder_gpu::{TextureFormat, UniformData};
use pathfinder_resources::ResourceLoader;
use pathfinder_simd::default::{F32x2, F32x4, I32x2};
use std::collections::VecDeque;
use std::f32;
use std::marker::PhantomData;
use std::mem;
use std::ops::{Add, Div};
use std::time::Duration;
use std::u32;

static QUAD_VERTEX_POSITIONS: [AlignedU16; 8] = [0, 0, 1, 0, 1, 1, 0, 1];
static QUAD_VERTEX_INDICES: [u32; 6] = [0, 1, 3, 1, 2, 3];

pub(crate) const MASK_TILES_ACROSS: u32 = 256;
pub(crate) const MASK_TILES_DOWN: u32 = 256;

// 1.0 / sqrt(2*pi)
const SQRT_2_PI_INV: f32 = 0.3989422804014327;

const TEXTURE_CACHE_SIZE: usize = 8;

const MIN_FILL_STORAGE_CLASS: usize = 14;   // 0x4000 entries, 128kB
const MIN_TILE_STORAGE_CLASS: usize = 10;   // 1024 entries, 12kB

const TEXTURE_METADATA_ENTRIES_PER_ROW: i32 = 128;
const TEXTURE_METADATA_TEXTURE_WIDTH:   i32 = TEXTURE_METADATA_ENTRIES_PER_ROW * 4;
const TEXTURE_METADATA_TEXTURE_HEIGHT:  i32 = 65536 / TEXTURE_METADATA_ENTRIES_PER_ROW;

// FIXME(pcwalton): Shrink this again!
const MASK_FRAMEBUFFER_WIDTH:  i32 = TILE_WIDTH as i32      * MASK_TILES_ACROSS as i32;
const MASK_FRAMEBUFFER_HEIGHT: i32 = TILE_HEIGHT as i32 / 4 * MASK_TILES_DOWN as i32;

const COMBINER_CTRL_COLOR_COMBINE_SRC_IN: i32 =     0x1;
const COMBINER_CTRL_COLOR_COMBINE_DEST_IN: i32 =    0x2;

const COMBINER_CTRL_FILTER_RADIAL_GRADIENT: i32 =   0x1;
const COMBINER_CTRL_FILTER_TEXT: i32 =              0x2;
const COMBINER_CTRL_FILTER_BLUR: i32 =              0x3;

const COMBINER_CTRL_COMPOSITE_NORMAL: i32 =         0x0;
const COMBINER_CTRL_COMPOSITE_MULTIPLY: i32 =       0x1;
const COMBINER_CTRL_COMPOSITE_SCREEN: i32 =         0x2;
const COMBINER_CTRL_COMPOSITE_OVERLAY: i32 =        0x3;
const COMBINER_CTRL_COMPOSITE_DARKEN: i32 =         0x4;
const COMBINER_CTRL_COMPOSITE_LIGHTEN: i32 =        0x5;
const COMBINER_CTRL_COMPOSITE_COLOR_DODGE: i32 =    0x6;
const COMBINER_CTRL_COMPOSITE_COLOR_BURN: i32 =     0x7;
const COMBINER_CTRL_COMPOSITE_HARD_LIGHT: i32 =     0x8;
const COMBINER_CTRL_COMPOSITE_SOFT_LIGHT: i32 =     0x9;
const COMBINER_CTRL_COMPOSITE_DIFFERENCE: i32 =     0xa;
const COMBINER_CTRL_COMPOSITE_EXCLUSION: i32 =      0xb;
const COMBINER_CTRL_COMPOSITE_HUE: i32 =            0xc;
const COMBINER_CTRL_COMPOSITE_SATURATION: i32 =     0xd;
const COMBINER_CTRL_COMPOSITE_COLOR: i32 =          0xe;
const COMBINER_CTRL_COMPOSITE_LUMINOSITY: i32 =     0xf;

const COMBINER_CTRL_COLOR_FILTER_SHIFT: i32 =       4;
const COMBINER_CTRL_COLOR_COMBINE_SHIFT: i32 =      6;
const COMBINER_CTRL_COMPOSITE_SHIFT: i32 =          8;

pub struct Renderer<D> where D: Device {
    // Device
    pub device: D,

    // Core data
    dest_framebuffer: DestFramebuffer<D>,
    options: RendererOptions,
    blit_program: BlitProgram<D>,
    clear_program: ClearProgram<D>,
    fill_program: FillProgram<D>,
    tile_program: TileProgram<D>,
    tile_copy_program: CopyTileProgram<D>,
    tile_clip_program: ClipTileProgram<D>,
    stencil_program: StencilProgram<D>,
    reprojection_program: ReprojectionProgram<D>,
    quad_vertex_positions_buffer: D::Buffer,
    quad_vertex_indices_buffer: D::Buffer,
    next_fills: Vec<i32>,
    fill_tile_map: Vec<i32>,
    texture_pages: Vec<Option<TexturePage<D>>>,
    render_targets: Vec<RenderTargetInfo>,
    render_target_stack: Vec<RenderTargetId>,
    area_lut_texture: D::Texture,
    gamma_lut_texture: D::Texture,

    // Frames
    front_frame: Frame<D>,
    back_frame: Frame<D>,
    front_frame_fence: Option<D::Fence>,

    // Rendering state
    texture_cache: TextureCache<D>,

    // Debug
    pub stats: RenderStats,
    current_cpu_build_time: Option<Duration>,
    current_timer: Option<PendingTimer<D>>,
    pending_timers: VecDeque<PendingTimer<D>>,
    timer_query_cache: TimerQueryCache<D>,
    pub debug_ui_presenter: DebugUIPresenter<D>,

    // Extra info
    flags: RendererFlags,
}

struct Frame<D> where D: Device {
    framebuffer_flags: FramebufferFlags,
    blit_vertex_array: BlitVertexArray<D>,
    clear_vertex_array: ClearVertexArray<D>,
    fill_vertex_storage_allocator: StorageAllocator<D, FillVertexStorage<D>>,
    tile_vertex_storage_allocator: StorageAllocator<D, TileVertexStorage<D>>,
    quads_vertex_indices_buffer: D::Buffer,
    quads_vertex_indices_length: usize,
    alpha_tile_pages: FxHashMap<u16, AlphaTilePage<D>>,
    tile_clip_vertex_array: ClipTileVertexArray<D>,
    stencil_vertex_array: StencilVertexArray<D>,
    reprojection_vertex_array: ReprojectionVertexArray<D>,
    dest_blend_framebuffer: D::Framebuffer,
    intermediate_dest_framebuffer: D::Framebuffer,
    texture_metadata_texture: D::Texture,
}

impl<D> Renderer<D> where D: Device {
    pub fn new(device: D,
               resources: &dyn ResourceLoader,
               dest_framebuffer: DestFramebuffer<D>,
               options: RendererOptions)
               -> Renderer<D> {
        let blit_program = BlitProgram::new(&device, resources);
        let clear_program = ClearProgram::new(&device, resources);
        let fill_program = FillProgram::new(&device, resources, &options);
        let tile_program = TileProgram::new(&device, resources);
        let tile_copy_program = CopyTileProgram::new(&device, resources);
        let tile_clip_program = ClipTileProgram::new(&device, resources);
        let stencil_program = StencilProgram::new(&device, resources);
        let reprojection_program = ReprojectionProgram::new(&device, resources);

        let area_lut_texture =
            device.create_texture_from_png(resources, "area-lut", TextureFormat::RGBA8);
        let gamma_lut_texture =
            device.create_texture_from_png(resources, "gamma-lut", TextureFormat::R8);

        let quad_vertex_positions_buffer = device.create_buffer(BufferUploadMode::Static);
        device.allocate_buffer(&quad_vertex_positions_buffer,
                               BufferData::Memory(&QUAD_VERTEX_POSITIONS),
                               BufferTarget::Vertex);
        let quad_vertex_indices_buffer = device.create_buffer(BufferUploadMode::Static);
        device.allocate_buffer(&quad_vertex_indices_buffer,
                               BufferData::Memory(&QUAD_VERTEX_INDICES),
                               BufferTarget::Index);

        let window_size = dest_framebuffer.window_size(&device);

        let timer_query_cache = TimerQueryCache::new(&device);
        let debug_ui_presenter = DebugUIPresenter::new(&device, resources, window_size);

        let front_frame = Frame::new(&device,
                                     &blit_program,
                                     &clear_program,
                                     &tile_clip_program,
                                     &reprojection_program,
                                     &stencil_program,
                                     &quad_vertex_positions_buffer,
                                     &quad_vertex_indices_buffer,
                                     window_size);
        let back_frame = Frame::new(&device,
                                    &blit_program,
                                    &clear_program,
                                    &tile_clip_program,
                                    &reprojection_program,
                                    &stencil_program,
                                    &quad_vertex_positions_buffer,
                                    &quad_vertex_indices_buffer,
                                    window_size);

        Renderer {
            device,

            dest_framebuffer,
            options,
            blit_program,
            clear_program,
            fill_program,
            tile_program,
            tile_copy_program,
            tile_clip_program,
            quad_vertex_positions_buffer,
            quad_vertex_indices_buffer,
            next_fills: vec![],
            fill_tile_map: vec![-1; 256 * 256],
            texture_pages: vec![],
            render_targets: vec![],
            render_target_stack: vec![],

            front_frame,
            back_frame,
            front_frame_fence: None,

            area_lut_texture,
            gamma_lut_texture,

            stencil_program,

            reprojection_program,

            stats: RenderStats::default(),
            current_cpu_build_time: None,
            current_timer: None,
            pending_timers: VecDeque::new(),
            timer_query_cache,
            debug_ui_presenter,

            texture_cache: TextureCache::new(),

            flags: RendererFlags::empty(),
        }
    }

    pub fn begin_scene(&mut self) {
        self.back_frame.framebuffer_flags = FramebufferFlags::empty();
        for alpha_tile_page in self.back_frame.alpha_tile_pages.values_mut() {
            alpha_tile_page.framebuffer_is_dirty = false;
        }

        self.device.begin_commands();
        self.current_timer = Some(PendingTimer::new());
        self.stats = RenderStats::default();
    }

    pub fn render_command(&mut self, command: &RenderCommand) {
        debug!("render command: {:?}", command);
        match *command {
            RenderCommand::Start { bounding_quad, path_count, needs_readable_framebuffer } => {
                self.start_rendering(bounding_quad, path_count, needs_readable_framebuffer);
            }
            RenderCommand::AllocateTexturePage { page_id, ref descriptor } => {
                self.allocate_texture_page(page_id, descriptor)
            }
            RenderCommand::UploadTexelData { ref texels, location } => {
                self.upload_texel_data(texels, location)
            }
            RenderCommand::DeclareRenderTarget { id, location } => {
                self.declare_render_target(id, location)
            }
            RenderCommand::UploadTextureMetadata(ref metadata) => {
                self.upload_texture_metadata(metadata)
            }
            RenderCommand::AddFills(ref fills) => self.add_fills(fills),
            RenderCommand::FlushFills => {
                let page_indices: Vec<_> =
                    self.back_frame.alpha_tile_pages.keys().cloned().collect();
                for page_index in page_indices {
                    self.draw_buffered_fills(page_index)
                }
            }
            RenderCommand::ClipTiles(ref batches) => {
                batches.iter().for_each(|batch| self.draw_clip_batch(batch))
            }
            RenderCommand::BeginTileDrawing => {}
            RenderCommand::PushRenderTarget(render_target_id) => {
                self.push_render_target(render_target_id)
            }
            RenderCommand::PopRenderTarget => self.pop_render_target(),
            RenderCommand::DrawTiles(ref batch) => {
                let count = batch.tiles.len();
                self.stats.alpha_tile_count += count;
                let storage_id = self.upload_tiles(&batch.tiles);
                self.draw_tiles(batch.tile_page,
                                count as u32,
                                storage_id,
                                batch.color_texture,
                                batch.blend_mode,
                                batch.filter)
            }
            RenderCommand::Finish { cpu_build_time } => {
                self.stats.cpu_build_time = cpu_build_time;
            }
        }
    }

    pub fn end_scene(&mut self) {
        self.clear_dest_framebuffer_if_necessary();
        self.blit_intermediate_dest_framebuffer_if_necessary();

        let old_front_frame_fence = self.front_frame_fence.take();
        self.front_frame_fence = Some(self.device.add_fence());
        self.device.end_commands();

        self.back_frame.fill_vertex_storage_allocator.end_frame();
        self.back_frame.tile_vertex_storage_allocator.end_frame();

        if let Some(timer) = self.current_timer.take() {
            self.pending_timers.push_back(timer);
        }
        self.current_cpu_build_time = None;

        if let Some(old_front_frame_fence) = old_front_frame_fence {
            self.device.wait_for_fence(&old_front_frame_fence);
        }

        mem::swap(&mut self.front_frame, &mut self.back_frame);
    }

    fn start_rendering(&mut self,
                       bounding_quad: BoundingQuad,
                       path_count: usize,
                       mut needs_readable_framebuffer: bool) {
        if let DestFramebuffer::Other(_) = self.dest_framebuffer {
            needs_readable_framebuffer = false;
        }

        if self.flags.contains(RendererFlags::USE_DEPTH) {
            self.draw_stencil(&bounding_quad);
        }
        self.stats.path_count = path_count;

        self.flags.set(RendererFlags::INTERMEDIATE_DEST_FRAMEBUFFER_NEEDED,
                       needs_readable_framebuffer);

        self.render_targets.clear();
    }

    pub fn draw_debug_ui(&self) {
        self.debug_ui_presenter.draw(&self.device);
    }

    pub fn shift_rendering_time(&mut self) -> Option<RenderTime> {
        if let Some(mut pending_timer) = self.pending_timers.pop_front() {
            for old_query in pending_timer.poll(&self.device) {
                self.timer_query_cache.free(old_query);
            }
            if let Some(gpu_time) = pending_timer.total_time() {
                return Some(RenderTime { gpu_time })
            }
            self.pending_timers.push_front(pending_timer);
        }
        None
    }

    #[inline]
    pub fn dest_framebuffer(&self) -> &DestFramebuffer<D> {
        &self.dest_framebuffer
    }

    #[inline]
    pub fn replace_dest_framebuffer(
        &mut self,
        new_dest_framebuffer: DestFramebuffer<D>,
    ) -> DestFramebuffer<D> {
        mem::replace(&mut self.dest_framebuffer, new_dest_framebuffer)
    }

    #[inline]
    pub fn set_options(&mut self, new_options: RendererOptions) {
        self.options = new_options
    }

    #[inline]
    pub fn set_main_framebuffer_size(&mut self, new_framebuffer_size: Vector2I) {
        self.debug_ui_presenter.ui_presenter.set_framebuffer_size(new_framebuffer_size);
    }

    #[inline]
    pub fn disable_depth(&mut self) {
        self.flags.remove(RendererFlags::USE_DEPTH);
    }

    #[inline]
    pub fn enable_depth(&mut self) {
        self.flags.insert(RendererFlags::USE_DEPTH);
    }

    #[inline]
    pub fn quad_vertex_positions_buffer(&self) -> &D::Buffer {
        &self.quad_vertex_positions_buffer
    }

    #[inline]
    pub fn quad_vertex_indices_buffer(&self) -> &D::Buffer {
        &self.quad_vertex_indices_buffer
    }

    fn allocate_texture_page(&mut self,
                             page_id: TexturePageId,
                             descriptor: &TexturePageDescriptor) {
        // Fill in IDs up to the requested page ID.
        let page_index = page_id.0 as usize;
        while self.texture_pages.len() < page_index + 1 {
            self.texture_pages.push(None);
        }

        // Clear out any existing texture.
        if let Some(old_texture_page) = self.texture_pages[page_index].take() {
            let old_texture = self.device.destroy_framebuffer(old_texture_page.framebuffer);
            self.texture_cache.release_texture(old_texture);
        }

        // Allocate texture.
        let texture_size = descriptor.size;
        let texture = self.texture_cache.create_texture(&mut self.device,
                                                        TextureFormat::RGBA8,
                                                        texture_size);
        let framebuffer = self.device.create_framebuffer(texture);
        self.texture_pages[page_index] = Some(TexturePage {
            framebuffer,
            must_preserve_contents: false,
        });
    }

    fn upload_texel_data(&mut self, texels: &[ColorU], location: TextureLocation) {
        let texture_page = self.texture_pages[location.page.0 as usize]
                               .as_mut()
                               .expect("Texture page not allocated yet!");
        let texture = self.device.framebuffer_texture(&texture_page.framebuffer);
        let texels = color::color_slice_to_u8_slice(texels);
        self.device.upload_to_texture(texture, location.rect, TextureDataRef::U8(texels));
        texture_page.must_preserve_contents = true;
    }

    fn declare_render_target(&mut self,
                             render_target_id: RenderTargetId,
                             location: TextureLocation) {
        while self.render_targets.len() < render_target_id.render_target as usize + 1 {
            self.render_targets.push(RenderTargetInfo {
                location: TextureLocation { page: TexturePageId(!0), rect: RectI::default() },
            });
        }
        let mut render_target = &mut self.render_targets[render_target_id.render_target as usize];
        debug_assert_eq!(render_target.location.page, TexturePageId(!0));
        render_target.location = location;
    }

    fn upload_texture_metadata(&mut self, metadata: &[TextureMetadataEntry]) {
        let padded_texel_size =
            (util::alignup_i32(metadata.len() as i32, TEXTURE_METADATA_ENTRIES_PER_ROW) *
             TEXTURE_METADATA_TEXTURE_WIDTH * 4) as usize;
        let mut texels = Vec::with_capacity(padded_texel_size);
        for entry in metadata {
            let base_color = entry.base_color.to_f32();
            texels.extend_from_slice(&[
                f16::from_f32(entry.color_0_transform.m11()),
                f16::from_f32(entry.color_0_transform.m21()),
                f16::from_f32(entry.color_0_transform.m12()),
                f16::from_f32(entry.color_0_transform.m22()),
                f16::from_f32(entry.color_0_transform.m13()),
                f16::from_f32(entry.color_0_transform.m23()),
                f16::default(),
                f16::default(),
                f16::from_f32(base_color.r()),
                f16::from_f32(base_color.g()),
                f16::from_f32(base_color.b()),
                f16::from_f32(base_color.a()),
                f16::default(),
                f16::default(),
                f16::default(),
                f16::default(),
            ]);
        }
        while texels.len() < padded_texel_size {
            texels.push(f16::default())
        }

        let texture = &mut self.back_frame.texture_metadata_texture;
        let width = TEXTURE_METADATA_TEXTURE_WIDTH;
        let height = texels.len() as i32 / (4 * TEXTURE_METADATA_TEXTURE_WIDTH);
        let rect = RectI::new(Vector2I::zero(), Vector2I::new(width, height));
        self.device.upload_to_texture(texture, rect, TextureDataRef::F16(&texels));
    }

    fn upload_tiles(&mut self, tiles: &[Tile]) -> StorageID {
        debug_assert!(tiles.len() <= MAX_TILES_PER_BATCH);

        let tile_program = &self.tile_program;
        let tile_copy_program = &self.tile_copy_program;
        let quad_vertex_positions_buffer = &self.quad_vertex_positions_buffer;
        let quad_vertex_indices_buffer = &self.quad_vertex_indices_buffer;
        let storage_id = self.back_frame.tile_vertex_storage_allocator.allocate(&self.device,
                                                                                tiles.len() as u64,
                                                                                |device, size| {
            TileVertexStorage::new(size,
                                   device,
                                   tile_program,
                                   tile_copy_program,
                                   quad_vertex_positions_buffer,
                                   quad_vertex_indices_buffer)
        });

        let vertex_buffer = &self.back_frame
                                 .tile_vertex_storage_allocator
                                 .get(storage_id)
                                 .vertex_buffer;
        self.device.upload_to_buffer(vertex_buffer, 0, tiles, BufferTarget::Vertex);

        self.ensure_index_buffer(tiles.len());

        storage_id
    }

    fn ensure_index_buffer(&mut self, mut length: usize) {
        length = length.next_power_of_two();
        if self.back_frame.quads_vertex_indices_length >= length {
            return;
        }

        // TODO(pcwalton): Generate these with SIMD.
        let mut indices: Vec<u32> = Vec::with_capacity(length * 6);
        for index in 0..(length as u32) {
            indices.extend_from_slice(&[
                index * 4 + 0, index * 4 + 1, index * 4 + 2,
                index * 4 + 1, index * 4 + 3, index * 4 + 2,
            ]);
        }

        self.device.allocate_buffer(&self.back_frame.quads_vertex_indices_buffer,
                                    BufferData::Memory(&indices),
                                    BufferTarget::Index);

        self.back_frame.quads_vertex_indices_length = length;
    }

    fn add_fills(&mut self, fill_batch: &[FillBatchEntry]) {
        if fill_batch.is_empty() {
            return;
        }

        self.stats.fill_count += fill_batch.len();

        // We have to make sure we don't split batches across draw calls, or else the compute
        // shader path, which expects to see all the fills belonging to one tile in the same
        // batch, will break.

        let mut pages_touched = vec![];
        for fill_batch_entry in fill_batch {
            let page_index = fill_batch_entry.page;
            if !self.back_frame.alpha_tile_pages.contains_key(&page_index) {
                let alpha_tile_page = AlphaTilePage::new(&mut self.device);
                self.back_frame.alpha_tile_pages.insert(page_index, alpha_tile_page);
            }

            let page = self.back_frame.alpha_tile_pages.get_mut(&page_index).unwrap();
            if page.pending_fills.is_empty() {
                pages_touched.push(page_index);
            }
            page.pending_fills.push(fill_batch_entry.fill);
        }

        for page_index in pages_touched {
            if self.back_frame.alpha_tile_pages[&page_index].buffered_fills.len() +
                    self.back_frame.alpha_tile_pages[&page_index].pending_fills.len() >
                    MAX_FILLS_PER_BATCH {
                self.draw_buffered_fills(page_index);
            }

            let page = self.back_frame.alpha_tile_pages.get_mut(&page_index).unwrap();
            for fill in &page.pending_fills {
                page.buffered_fills.push(*fill);
            }
            page.pending_fills.clear();
        }
    }

    fn draw_buffered_fills(&mut self, page: u16) {
        match self.fill_program {
            FillProgram::Raster(_) => self.draw_buffered_fills_via_raster(page),
            FillProgram::Compute(_) => self.draw_buffered_fills_via_compute(page),
        }
    }

    fn draw_buffered_fills_via_raster(&mut self, page: u16) {
        let fill_raster_program = match self.fill_program {
            FillProgram::Raster(ref fill_raster_program) => fill_raster_program,
            _ => unreachable!(),
        };

        let mask_viewport = self.mask_viewport();

        let alpha_tile_page = self.back_frame
                                  .alpha_tile_pages
                                  .get_mut(&page)
                                  .expect("Where's the alpha tile page?");
        let buffered_fills = &mut alpha_tile_page.buffered_fills;
        if buffered_fills.is_empty() {
            return;
        }

        let storage_id = {
            let fill_program = &self.fill_program;
            let quad_vertex_positions_buffer = &self.quad_vertex_positions_buffer;
            let quad_vertex_indices_buffer = &self.quad_vertex_indices_buffer;
            self.back_frame
                .fill_vertex_storage_allocator
                .allocate(&self.device, MAX_FILLS_PER_BATCH as u64, |device, size| {
                FillVertexStorage::new(size,
                                       device,
                                       fill_program,
                                       quad_vertex_positions_buffer,
                                       quad_vertex_indices_buffer)
            })
        };
        let fill_vertex_storage = self.back_frame.fill_vertex_storage_allocator.get(storage_id);

        let fill_vertex_array = match fill_vertex_storage.auxiliary {
            FillVertexStorageAuxiliary::Raster { ref vertex_array } => vertex_array,
            _ => unreachable!(),
        };

        self.device.upload_to_buffer(&fill_vertex_storage.vertex_buffer,
                                     0,
                                     &buffered_fills,
                                     BufferTarget::Vertex);

        let mut clear_color = None;
        if !alpha_tile_page.framebuffer_is_dirty {
            clear_color = Some(ColorF::default());
        };

        let timer_query = self.timer_query_cache.alloc(&self.device);
        self.device.begin_timer_query(&timer_query);

        debug_assert!(buffered_fills.len() <= u32::MAX as usize);
        self.device.draw_elements_instanced(6, buffered_fills.len() as u32, &RenderState {
            target: &RenderTarget::Framebuffer(&alpha_tile_page.framebuffer),
            program: &fill_raster_program.program,
            vertex_array: &fill_vertex_array.vertex_array,
            primitive: Primitive::Triangles,
            textures: &[&self.area_lut_texture],
            uniforms: &[
                (&fill_raster_program.framebuffer_size_uniform,
                 UniformData::Vec2(F32x2::new(MASK_FRAMEBUFFER_WIDTH as f32,
                                              MASK_FRAMEBUFFER_HEIGHT as f32))),
                (&fill_raster_program.tile_size_uniform,
                 UniformData::Vec2(F32x2::new(TILE_WIDTH as f32, TILE_HEIGHT as f32))),
                (&fill_raster_program.area_lut_uniform, UniformData::TextureUnit(0)),
            ],
            images: &[],
            viewport: mask_viewport,
            options: RenderOptions {
                blend: Some(BlendState {
                    src_rgb_factor: BlendFactor::One,
                    src_alpha_factor: BlendFactor::One,
                    dest_rgb_factor: BlendFactor::One,
                    dest_alpha_factor: BlendFactor::One,
                    ..BlendState::default()
                }),
                clear_ops: ClearOps { color: clear_color, ..ClearOps::default() },
                ..RenderOptions::default()
            },
        });

        self.device.end_timer_query(&timer_query);
        self.current_timer.as_mut().unwrap().fill_times.push(TimerFuture::new(timer_query));

        alpha_tile_page.framebuffer_is_dirty = true;
        buffered_fills.clear();
    }

    fn draw_buffered_fills_via_compute(&mut self, page: u16) {
        let fill_compute_program = match self.fill_program {
            FillProgram::Compute(ref fill_compute_program) => fill_compute_program,
            _ => unreachable!(),
        };

        let alpha_tile_page = self.back_frame
                                  .alpha_tile_pages
                                  .get_mut(&page)
                                  .expect("Where's the alpha tile page?");
        let buffered_fills = &mut alpha_tile_page.buffered_fills;
        if buffered_fills.is_empty() {
            return;
        }

        let storage_id = {
            let fill_program = &self.fill_program;
            let quad_vertex_positions_buffer = &self.quad_vertex_positions_buffer;
            let quad_vertex_indices_buffer = &self.quad_vertex_indices_buffer;
            self.back_frame.fill_vertex_storage_allocator.allocate(&self.device,
                                                                   MAX_FILLS_PER_BATCH as u64,
                                                                   |device, size| {
                FillVertexStorage::new(size,
                                       device,
                                       fill_program,
                                       quad_vertex_positions_buffer,
                                       quad_vertex_indices_buffer)
            })
        };
        let fill_vertex_storage = self.back_frame.fill_vertex_storage_allocator.get(storage_id);

        let (tile_map_buffer, next_fills_buffer) = match fill_vertex_storage.auxiliary {
            FillVertexStorageAuxiliary::Compute { ref tile_map_buffer, ref next_fills_buffer } => {
                (tile_map_buffer, next_fills_buffer)
            }
            _ => unreachable!(),
        };

        // Initialize the tile map and fill linked list buffers.
        self.fill_tile_map.iter_mut().for_each(|entry| *entry = -1);
        while self.next_fills.len() < buffered_fills.len() {
            self.next_fills.push(-1);
        }

        // Create a linked list running through all our fills.
        let (mut first_fill_tile, mut last_fill_tile) = (256 * 256, 0);
        for (fill_index, fill) in buffered_fills.iter().enumerate() {
            let fill_tile_index = fill.alpha_tile_index as usize;
            self.next_fills[fill_index as usize] = self.fill_tile_map[fill_tile_index];
            self.fill_tile_map[fill_tile_index] = fill_index as i32;
            first_fill_tile = first_fill_tile.min(fill_tile_index as u32);
            last_fill_tile = last_fill_tile.max(fill_tile_index as u32);
        }
        let fill_tile_count = last_fill_tile - first_fill_tile + 1;

        self.device.upload_to_buffer(&fill_vertex_storage.vertex_buffer,
                                     0,
                                     &buffered_fills,
                                     BufferTarget::Storage);
        self.device.upload_to_buffer(next_fills_buffer,
                                     0,
                                     &self.next_fills,
                                     BufferTarget::Storage);
        self.device.upload_to_buffer(tile_map_buffer,
                                     0,
                                     &self.fill_tile_map,
                                     BufferTarget::Storage);

        let image_binding = ImageBinding {
            texture: self.device.framebuffer_texture(&alpha_tile_page.framebuffer),
            access: ImageAccess::Write,
        };

        let timer_query = self.timer_query_cache.alloc(&self.device);
        self.device.begin_timer_query(&timer_query);

        debug_assert!(buffered_fills.len() <= u32::MAX as usize);
        let dimensions = ComputeDimensions { x: 1, y: 1, z: fill_tile_count as u32 };
        self.device.dispatch_compute(dimensions, &ComputeState {
            program: &fill_compute_program.program,
            textures: &[&self.area_lut_texture],
            images: &[image_binding],
            uniforms: &[
                (&fill_compute_program.area_lut_uniform, UniformData::TextureUnit(0)),
                (&fill_compute_program.dest_uniform, UniformData::ImageUnit(0)),
                (&fill_compute_program.first_tile_index_uniform,
                 UniformData::Int(first_fill_tile as i32)),
            ],
            storage_buffers: &[
                (&fill_compute_program.fills_storage_buffer, &fill_vertex_storage.vertex_buffer),
                (&fill_compute_program.next_fills_storage_buffer, next_fills_buffer),
                (&fill_compute_program.fill_tile_map_storage_buffer, tile_map_buffer),
            ],
        });

        self.device.end_timer_query(&timer_query);
        self.current_timer.as_mut().unwrap().fill_times.push(TimerFuture::new(timer_query));

        alpha_tile_page.framebuffer_is_dirty = true;
        buffered_fills.clear();
    }

    fn draw_clip_batch(&mut self, batch: &ClipBatch) {
        if batch.clips.is_empty() {
            return;
        }

        let ClipBatchKey { dest_page, src_page, kind } = batch.key;

        self.device.allocate_buffer(&self.back_frame.tile_clip_vertex_array.vertex_buffer,
                                    BufferData::Memory(&batch.clips),
                                    BufferTarget::Vertex);

        if !self.back_frame.alpha_tile_pages.contains_key(&dest_page) {
            let alpha_tile_page = AlphaTilePage::new(&mut self.device);
            self.back_frame.alpha_tile_pages.insert(dest_page, alpha_tile_page);
        }

        let mut clear_color = None;
        if !self.back_frame.alpha_tile_pages[&dest_page].framebuffer_is_dirty {
            clear_color = Some(ColorF::default());
        };

        let blend = match kind {
            ClipBatchKind::Draw => None,
            ClipBatchKind::Clip => {
                Some(BlendState {
                    src_rgb_factor: BlendFactor::One,
                    src_alpha_factor: BlendFactor::One,
                    dest_rgb_factor: BlendFactor::One,
                    dest_alpha_factor: BlendFactor::One,
                    op: BlendOp::Min,
                })
            }
        };

        let mask_viewport = self.mask_viewport();

        let timer_query = self.timer_query_cache.alloc(&self.device);
        self.device.begin_timer_query(&timer_query);

        {
            let dest_framebuffer = &self.back_frame.alpha_tile_pages[&dest_page].framebuffer;
            let src_framebuffer = &self.back_frame.alpha_tile_pages[&src_page].framebuffer;
            let src_texture = self.device.framebuffer_texture(&src_framebuffer);

            debug_assert!(batch.clips.len() <= u32::MAX as usize);
            self.device.draw_elements_instanced(6, batch.clips.len() as u32, &RenderState {
                target: &RenderTarget::Framebuffer(dest_framebuffer),
                program: &self.tile_clip_program.program,
                vertex_array: &self.back_frame.tile_clip_vertex_array.vertex_array,
                primitive: Primitive::Triangles,
                textures: &[src_texture],
                images: &[],
                uniforms: &[(&self.tile_clip_program.src_uniform, UniformData::TextureUnit(0))],
                viewport: mask_viewport,
                options: RenderOptions {
                    blend,
                    clear_ops: ClearOps { color: clear_color, ..ClearOps::default() },
                    ..RenderOptions::default()
                },
            });

            self.device.end_timer_query(&timer_query);
            self.current_timer.as_mut().unwrap().fill_times.push(TimerFuture::new(timer_query));
        }

        self.back_frame
            .alpha_tile_pages
            .get_mut(&dest_page)
            .unwrap()
            .framebuffer_is_dirty = true;
    }

    fn tile_transform(&self) -> Transform4F {
        let draw_viewport = self.draw_viewport().size().to_f32();
        let scale = Vector4F::new(2.0 / draw_viewport.x(), -2.0 / draw_viewport.y(), 1.0, 1.0);
        Transform4F::from_scale(scale).translate(Vector4F::new(-1.0, 1.0, 0.0, 1.0))
    }

    fn draw_tiles(&mut self,
                  tile_page: u16,
                  tile_count: u32,
                  storage_id: StorageID,
                  color_texture_0: Option<TileBatchTexture>,
                  blend_mode: BlendMode,
                  filter: Filter) {
        // TODO(pcwalton): Disable blend for solid tiles.

        let needs_readable_framebuffer = blend_mode.needs_readable_framebuffer();
        if needs_readable_framebuffer {
            self.copy_alpha_tiles_to_dest_blend_texture(tile_count, storage_id);
        }

        let clear_color = self.clear_color_for_draw_operation();
        let draw_viewport = self.draw_viewport();

        let timer_query = self.timer_query_cache.alloc(&self.device);
        self.device.begin_timer_query(&timer_query);

        let mut textures = vec![&self.back_frame.texture_metadata_texture];
        let mut uniforms = vec![
            (&self.tile_program.transform_uniform,
             UniformData::Mat4(self.tile_transform().to_columns())),
            (&self.tile_program.tile_size_uniform,
             UniformData::Vec2(F32x2::new(TILE_WIDTH as f32, TILE_HEIGHT as f32))),
            (&self.tile_program.framebuffer_size_uniform,
             UniformData::Vec2(draw_viewport.size().to_f32().0)),
            (&self.tile_program.texture_metadata_uniform, UniformData::TextureUnit(0)),
            (&self.tile_program.texture_metadata_size_uniform,
             UniformData::IVec2(I32x2::new(TEXTURE_METADATA_TEXTURE_WIDTH,
                                           TEXTURE_METADATA_TEXTURE_HEIGHT))),
        ];

        if needs_readable_framebuffer {
            uniforms.push((&self.tile_program.dest_texture_uniform,
                           UniformData::TextureUnit(textures.len() as u32)));
            textures.push(self.device
                              .framebuffer_texture(&self.back_frame.dest_blend_framebuffer));
        }

        if let Some(alpha_tile_page) = self.back_frame.alpha_tile_pages.get(&tile_page) {
            uniforms.push((&self.tile_program.mask_texture_0_uniform,
                           UniformData::TextureUnit(textures.len() as u32)));
            uniforms.push((&self.tile_program.mask_texture_size_0_uniform,
                           UniformData::Vec2(F32x2::new(MASK_FRAMEBUFFER_WIDTH as f32,
                                                        MASK_FRAMEBUFFER_HEIGHT as f32))));
            textures.push(self.device.framebuffer_texture(&alpha_tile_page.framebuffer));
        }

        // TODO(pcwalton): Refactor.
        let mut ctrl = 0;
        match color_texture_0 {
            Some(color_texture) => {
                let color_texture_page = self.texture_page(color_texture.page);
                let color_texture_size = self.device.texture_size(color_texture_page).to_f32();
                self.device.set_texture_sampling_mode(color_texture_page,
                                                    color_texture.sampling_flags);
                uniforms.push((&self.tile_program.color_texture_0_uniform,
                               UniformData::TextureUnit(textures.len() as u32)));
                uniforms.push((&self.tile_program.color_texture_size_0_uniform,
                               UniformData::Vec2(color_texture_size.0)));
                textures.push(color_texture_page);

                ctrl |= color_texture.composite_op.to_combine_mode() <<
                    COMBINER_CTRL_COLOR_COMBINE_SHIFT;
            }
            None => {
                uniforms.push((&self.tile_program.color_texture_size_0_uniform,
                               UniformData::Vec2(F32x2::default())));
            }
        }

        ctrl |= blend_mode.to_composite_ctrl() << COMBINER_CTRL_COMPOSITE_SHIFT;

        match filter {
            Filter::None => self.set_uniforms_for_no_filter(&mut uniforms),
            Filter::RadialGradient { line, radii, uv_origin } => {
                ctrl |= COMBINER_CTRL_FILTER_RADIAL_GRADIENT << COMBINER_CTRL_COLOR_FILTER_SHIFT;
                self.set_uniforms_for_radial_gradient_filter(&mut uniforms, line, radii, uv_origin)
            }
            Filter::PatternFilter(PatternFilter::Text {
                fg_color,
                bg_color,
                defringing_kernel,
                gamma_correction,
            }) => {
                ctrl |= COMBINER_CTRL_FILTER_TEXT << COMBINER_CTRL_COLOR_FILTER_SHIFT;
                self.set_uniforms_for_text_filter(&mut textures,
                                                  &mut uniforms,
                                                  fg_color,
                                                  bg_color,
                                                  defringing_kernel,
                                                  gamma_correction);
            }
            Filter::PatternFilter(PatternFilter::Blur { direction, sigma }) => {
                ctrl |= COMBINER_CTRL_FILTER_BLUR << COMBINER_CTRL_COLOR_FILTER_SHIFT;
                self.set_uniforms_for_blur_filter(&mut uniforms, direction, sigma);
            }
        }

        uniforms.push((&self.tile_program.ctrl_uniform, UniformData::Int(ctrl)));

        let vertex_array = &self.back_frame
                                .tile_vertex_storage_allocator
                                .get(storage_id)
                                .tile_vertex_array
                                .vertex_array;

        self.device.draw_elements_instanced(6, tile_count, &RenderState {
            target: &self.draw_render_target(),
            program: &self.tile_program.program,
            vertex_array,
            primitive: Primitive::Triangles,
            textures: &textures,
            images: &[],
            uniforms: &uniforms,
            viewport: draw_viewport,
            options: RenderOptions {
                blend: blend_mode.to_blend_state(),
                stencil: self.stencil_state(),
                clear_ops: ClearOps { color: clear_color, ..ClearOps::default() },
                ..RenderOptions::default()
            },
        });

        self.device.end_timer_query(&timer_query);
        self.current_timer.as_mut().unwrap().tile_times.push(TimerFuture::new(timer_query));

        self.preserve_draw_framebuffer();
    }

    fn copy_alpha_tiles_to_dest_blend_texture(&mut self, tile_count: u32, storage_id: StorageID) {
        let draw_viewport = self.draw_viewport();

        let mut textures = vec![];
        let mut uniforms = vec![
            (&self.tile_copy_program.transform_uniform,
             UniformData::Mat4(self.tile_transform().to_columns())),
            (&self.tile_copy_program.tile_size_uniform,
             UniformData::Vec2(F32x2::new(TILE_WIDTH as f32, TILE_HEIGHT as f32))),
        ];

        let draw_framebuffer = match self.draw_render_target() {
            RenderTarget::Framebuffer(framebuffer) => framebuffer,
            RenderTarget::Default => panic!("Can't copy alpha tiles from default framebuffer!"),
        };
        let draw_texture = self.device.framebuffer_texture(&draw_framebuffer);

        uniforms.push((&self.tile_copy_program.src_uniform,
                       UniformData::TextureUnit(textures.len() as u32)));
        textures.push(draw_texture);
        uniforms.push((&self.tile_copy_program.framebuffer_size_uniform,
                       UniformData::Vec2(draw_viewport.size().to_f32().0)));

        let vertex_array = &self.back_frame
                                .tile_vertex_storage_allocator
                                .get(storage_id)
                                .tile_copy_vertex_array
                                .vertex_array;

        self.device.draw_elements(tile_count * 6, &RenderState {
            target: &RenderTarget::Framebuffer(&self.back_frame.dest_blend_framebuffer),
            program: &self.tile_copy_program.program,
            vertex_array,
            primitive: Primitive::Triangles,
            textures: &textures,
            images: &[],
            uniforms: &uniforms,
            viewport: draw_viewport,
            options: RenderOptions {
                clear_ops: ClearOps {
                    color: Some(ColorF::new(1.0, 0.0, 0.0, 1.0)),
                    ..ClearOps::default()
                },
                ..RenderOptions::default()
            },
        });
    }

    fn draw_stencil(&mut self, quad_positions: &[Vector4F]) {
        self.device.allocate_buffer(&self.back_frame.stencil_vertex_array.vertex_buffer,
                                    BufferData::Memory(quad_positions),
                                    BufferTarget::Vertex);

        // Create indices for a triangle fan. (This is OK because the clipped quad should always be
        // convex.)
        let mut indices: Vec<u32> = vec![];
        for index in 1..(quad_positions.len() as u32 - 1) {
            indices.extend_from_slice(&[0, index as u32, index + 1]);
        }
        self.device.allocate_buffer(&self.back_frame.stencil_vertex_array.index_buffer,
                                    BufferData::Memory(&indices),
                                    BufferTarget::Index);

        self.device.draw_elements(indices.len() as u32, &RenderState {
            target: &self.draw_render_target(),
            program: &self.stencil_program.program,
            vertex_array: &self.back_frame.stencil_vertex_array.vertex_array,
            primitive: Primitive::Triangles,
            textures: &[],
            images: &[],
            uniforms: &[],
            viewport: self.draw_viewport(),
            options: RenderOptions {
                // FIXME(pcwalton): Should we really write to the depth buffer?
                depth: Some(DepthState { func: DepthFunc::Less, write: true }),
                stencil: Some(StencilState {
                    func: StencilFunc::Always,
                    reference: 1,
                    mask: 1,
                    write: true,
                }),
                color_mask: false,
                clear_ops: ClearOps { stencil: Some(0), ..ClearOps::default() },
                ..RenderOptions::default()
            },
        });
    }

    pub fn reproject_texture(
        &mut self,
        texture: &D::Texture,
        old_transform: &Transform4F,
        new_transform: &Transform4F,
    ) {
        let clear_color = self.clear_color_for_draw_operation();

        self.device.draw_elements(6, &RenderState {
            target: &self.draw_render_target(),
            program: &self.reprojection_program.program,
            vertex_array: &self.back_frame.reprojection_vertex_array.vertex_array,
            primitive: Primitive::Triangles,
            textures: &[texture],
            images: &[],
            uniforms: &[
                (&self.reprojection_program.old_transform_uniform,
                 UniformData::from_transform_3d(old_transform)),
                (&self.reprojection_program.new_transform_uniform,
                 UniformData::from_transform_3d(new_transform)),
                (&self.reprojection_program.texture_uniform, UniformData::TextureUnit(0)),
            ],
            viewport: self.draw_viewport(),
            options: RenderOptions {
                blend: BlendMode::SrcOver.to_blend_state(),
                depth: Some(DepthState { func: DepthFunc::Less, write: false, }),
                clear_ops: ClearOps { color: clear_color, ..ClearOps::default() },
                ..RenderOptions::default()
            },
        });

        self.preserve_draw_framebuffer();
    }

    pub fn draw_render_target(&self) -> RenderTarget<D> {
        match self.render_target_stack.last() {
            Some(&render_target_id) => {
                let texture_page_id = self.render_target_location(render_target_id).page;
                let framebuffer = self.texture_page_framebuffer(texture_page_id);
                RenderTarget::Framebuffer(framebuffer)
            }
            None => {
                if self.flags.contains(RendererFlags::INTERMEDIATE_DEST_FRAMEBUFFER_NEEDED) {
                    RenderTarget::Framebuffer(&self.back_frame.intermediate_dest_framebuffer)
                } else {
                    match self.dest_framebuffer {
                        DestFramebuffer::Default { .. } => RenderTarget::Default,
                        DestFramebuffer::Other(ref framebuffer) => {
                            RenderTarget::Framebuffer(framebuffer)
                        }
                    }
                }
            }
        }
    }

    fn push_render_target(&mut self, render_target_id: RenderTargetId) {
        self.render_target_stack.push(render_target_id);
    }

    fn pop_render_target(&mut self) {
        self.render_target_stack.pop().expect("Render target stack underflow!");
    }

    fn set_uniforms_for_no_filter<'a>(&'a self,
                                      uniforms: &mut Vec<(&'a D::Uniform, UniformData)>) {
        uniforms.extend_from_slice(&[
            (&self.tile_program.filter_params_0_uniform, UniformData::Vec4(F32x4::default())),
            (&self.tile_program.filter_params_1_uniform, UniformData::Vec4(F32x4::default())),
            (&self.tile_program.filter_params_2_uniform, UniformData::Vec4(F32x4::default())),
        ]);
    }

    fn set_uniforms_for_radial_gradient_filter<'a>(
            &'a self,
            uniforms: &mut Vec<(&'a D::Uniform, UniformData)>,
            line: LineSegment2F,
            radii: F32x2,
            uv_origin: Vector2F) {
        uniforms.extend_from_slice(&[
            (&self.tile_program.filter_params_0_uniform,
             UniformData::Vec4(line.from().0.concat_xy_xy(line.vector().0))),
            (&self.tile_program.filter_params_1_uniform,
             UniformData::Vec4(radii.concat_xy_xy(uv_origin.0))),
            (&self.tile_program.filter_params_2_uniform, UniformData::Vec4(F32x4::default())),
        ]);
    }

    fn set_uniforms_for_text_filter<'a>(&'a self,
                                        textures: &mut Vec<&'a D::Texture>,
                                        uniforms: &mut Vec<(&'a D::Uniform, UniformData)>,
                                        fg_color: ColorF,
                                        bg_color: ColorF,
                                        defringing_kernel: Option<DefringingKernel>,
                                        gamma_correction: bool) {
        let gamma_lut_texture_unit = textures.len() as u32;
        textures.push(&self.gamma_lut_texture);

        match defringing_kernel {
            Some(ref kernel) => {
                uniforms.push((&self.tile_program.filter_params_0_uniform,
                               UniformData::Vec4(F32x4::from_slice(&kernel.0))));
            }
            None => {
                uniforms.push((&self.tile_program.filter_params_0_uniform,
                               UniformData::Vec4(F32x4::default())));
            }
        }

        let mut params_2 = fg_color.0;
        params_2.set_w(gamma_correction as i32 as f32);

        uniforms.extend_from_slice(&[
            (&self.tile_program.gamma_lut_uniform,
             UniformData::TextureUnit(gamma_lut_texture_unit)),
            (&self.tile_program.filter_params_1_uniform, UniformData::Vec4(bg_color.0)),
            (&self.tile_program.filter_params_2_uniform, UniformData::Vec4(params_2)),
        ]);

    }

    fn set_uniforms_for_blur_filter<'a>(&'a self,
                                        uniforms: &mut Vec<(&'a D::Uniform, UniformData)>,
                                        direction: BlurDirection,
                                        sigma: f32) {
        let sigma_inv = 1.0 / sigma;
        let gauss_coeff_x = SQRT_2_PI_INV * sigma_inv;
        let gauss_coeff_y = f32::exp(-0.5 * sigma_inv * sigma_inv);
        let gauss_coeff_z = gauss_coeff_y * gauss_coeff_y;

        let src_offset = match direction {
            BlurDirection::X => vec2f(1.0, 0.0),
            BlurDirection::Y => vec2f(0.0, 1.0),
        };

        let support = f32::ceil(1.5 * sigma) * 2.0;

        uniforms.extend_from_slice(&[
            (&self.tile_program.filter_params_0_uniform,
             UniformData::Vec4(src_offset.0.concat_xy_xy(F32x2::new(support, 0.0)))),
            (&self.tile_program.filter_params_1_uniform,
             UniformData::Vec4(F32x4::new(gauss_coeff_x, gauss_coeff_y, gauss_coeff_z, 0.0))),
            (&self.tile_program.filter_params_2_uniform, UniformData::Vec4(F32x4::default())),
        ]);
    }

    fn clear_dest_framebuffer_if_necessary(&mut self) {
        let background_color = match self.options.background_color {
            None => return,
            Some(background_color) => background_color,
        };

        if self.back_frame
               .framebuffer_flags
               .contains(FramebufferFlags::DEST_FRAMEBUFFER_IS_DIRTY) {
            return;
        }

        let main_viewport = self.main_viewport();
        let uniforms = [
            (&self.clear_program.rect_uniform, UniformData::Vec4(main_viewport.to_f32().0)),
            (&self.clear_program.framebuffer_size_uniform,
             UniformData::Vec2(main_viewport.size().to_f32().0)),
            (&self.clear_program.color_uniform, UniformData::Vec4(background_color.0)),
        ];

        self.device.draw_elements(6, &RenderState {
            target: &RenderTarget::Default,
            program: &self.clear_program.program,
            vertex_array: &self.back_frame.clear_vertex_array.vertex_array,
            primitive: Primitive::Triangles,
            textures: &[],
            images: &[],
            uniforms: &uniforms[..],
            viewport: main_viewport,
            options: RenderOptions::default(),
        });
    }

    fn blit_intermediate_dest_framebuffer_if_necessary(&mut self) {
        if !self.flags.contains(RendererFlags::INTERMEDIATE_DEST_FRAMEBUFFER_NEEDED) {
            return;
        }

        let main_viewport = self.main_viewport();

        let uniforms = [(&self.blit_program.src_uniform, UniformData::TextureUnit(0))];
        let textures = [
            (self.device.framebuffer_texture(&self.back_frame.intermediate_dest_framebuffer))
        ];

        self.device.draw_elements(6, &RenderState {
            target: &RenderTarget::Default,
            program: &self.blit_program.program,
            vertex_array: &self.back_frame.blit_vertex_array.vertex_array,
            primitive: Primitive::Triangles,
            textures: &textures[..],
            images: &[],
            uniforms: &uniforms[..],
            viewport: main_viewport,
            options: RenderOptions {
                clear_ops: ClearOps {
                    color: Some(ColorF::new(0.0, 0.0, 0.0, 1.0)),
                    ..ClearOps::default()
                },
                ..RenderOptions::default()
            },
        });
    }

    fn stencil_state(&self) -> Option<StencilState> {
        if !self.flags.contains(RendererFlags::USE_DEPTH) {
            return None;
        }

        Some(StencilState {
            func: StencilFunc::Equal,
            reference: 1,
            mask: 1,
            write: false,
        })
    }

    fn clear_color_for_draw_operation(&self) -> Option<ColorF> {
        let must_preserve_contents = match self.render_target_stack.last() {
            Some(&render_target_id) => {
                let texture_page = self.render_target_location(render_target_id).page;
                self.texture_pages[texture_page.0 as usize]
                    .as_ref()
                    .expect("Draw target texture page not allocated!")
                    .must_preserve_contents
            }
            None => {
                self.back_frame
                    .framebuffer_flags
                    .contains(FramebufferFlags::DEST_FRAMEBUFFER_IS_DIRTY)
            }
        };

        if must_preserve_contents {
            None
        } else if self.render_target_stack.is_empty() {
            self.options.background_color
        } else {
            Some(ColorF::default())
        }
    }

    fn preserve_draw_framebuffer(&mut self) {
        match self.render_target_stack.last() {
            Some(&render_target_id) => {
                let texture_page = self.render_target_location(render_target_id).page;
                self.texture_pages[texture_page.0 as usize]
                    .as_mut()
                    .expect("Draw target texture page not allocated!")
                    .must_preserve_contents = true;
            }
            None => {
                self.back_frame
                    .framebuffer_flags
                    .insert(FramebufferFlags::DEST_FRAMEBUFFER_IS_DIRTY);
            }
        }
    }

    pub fn draw_viewport(&self) -> RectI {
        match self.render_target_stack.last() {
            Some(&render_target_id) => self.render_target_location(render_target_id).rect,
            None => self.main_viewport(),
        }
    }

    fn main_viewport(&self) -> RectI {
        match self.dest_framebuffer {
            DestFramebuffer::Default { viewport, .. } => viewport,
            DestFramebuffer::Other(ref framebuffer) => {
                let size = self
                    .device
                    .texture_size(self.device.framebuffer_texture(framebuffer));
                RectI::new(Vector2I::default(), size)
            }
        }
    }

    fn mask_viewport(&self) -> RectI {
        RectI::new(Vector2I::zero(), vec2i(MASK_FRAMEBUFFER_WIDTH, MASK_FRAMEBUFFER_HEIGHT))
    }

    fn render_target_location(&self, render_target_id: RenderTargetId) -> TextureLocation {
        self.render_targets[render_target_id.render_target as usize].location
    }

    fn texture_page_framebuffer(&self, id: TexturePageId) -> &D::Framebuffer {
        &self.texture_pages[id.0 as usize]
             .as_ref()
             .expect("Texture page not allocated!")
             .framebuffer
    }

    fn texture_page(&self, id: TexturePageId) -> &D::Texture {
        self.device.framebuffer_texture(&self.texture_page_framebuffer(id))
    }
}

impl<D> Frame<D> where D: Device {
    // FIXME(pcwalton): This signature shouldn't be so big. Make a struct.
    fn new(device: &D,
           blit_program: &BlitProgram<D>,
           clear_program: &ClearProgram<D>,
           tile_clip_program: &ClipTileProgram<D>,
           reprojection_program: &ReprojectionProgram<D>,
           stencil_program: &StencilProgram<D>,
           quad_vertex_positions_buffer: &D::Buffer,
           quad_vertex_indices_buffer: &D::Buffer,
           window_size: Vector2I)
           -> Frame<D> {
        let quads_vertex_indices_buffer = device.create_buffer(BufferUploadMode::Dynamic);

        let blit_vertex_array = BlitVertexArray::new(device,
                                                     &blit_program,
                                                     &quad_vertex_positions_buffer,
                                                     &quad_vertex_indices_buffer);
        let clear_vertex_array = ClearVertexArray::new(device,
                                                       &clear_program,
                                                       &quad_vertex_positions_buffer,
                                                       &quad_vertex_indices_buffer);
        let tile_clip_vertex_array = ClipTileVertexArray::new(device,
                                                              &tile_clip_program,
                                                              &quad_vertex_positions_buffer,
                                                              &quad_vertex_indices_buffer);
        let reprojection_vertex_array = ReprojectionVertexArray::new(device,
                                                                     &reprojection_program,
                                                                     &quad_vertex_positions_buffer,
                                                                     &quad_vertex_indices_buffer);
        let stencil_vertex_array = StencilVertexArray::new(device, &stencil_program);

        let fill_vertex_storage_allocator = StorageAllocator::new(MIN_FILL_STORAGE_CLASS);
        let tile_vertex_storage_allocator = StorageAllocator::new(MIN_TILE_STORAGE_CLASS);

        let texture_metadata_texture_size = vec2i(TEXTURE_METADATA_TEXTURE_WIDTH,
                                                  TEXTURE_METADATA_TEXTURE_HEIGHT);
        let texture_metadata_texture = device.create_texture(TextureFormat::RGBA16F,
                                                             texture_metadata_texture_size);

        let intermediate_dest_texture = device.create_texture(TextureFormat::RGBA8, window_size);
        let intermediate_dest_framebuffer = device.create_framebuffer(intermediate_dest_texture);

        let dest_blend_texture = device.create_texture(TextureFormat::RGBA8, window_size);
        let dest_blend_framebuffer = device.create_framebuffer(dest_blend_texture);

        Frame {
            blit_vertex_array,
            clear_vertex_array,
            tile_vertex_storage_allocator,
            fill_vertex_storage_allocator,
            tile_clip_vertex_array,
            reprojection_vertex_array,
            stencil_vertex_array,
            quads_vertex_indices_buffer,
            quads_vertex_indices_length: 0,
            alpha_tile_pages: FxHashMap::default(),
            texture_metadata_texture,
            intermediate_dest_framebuffer,
            dest_blend_framebuffer,
            framebuffer_flags: FramebufferFlags::empty(),
        }
    }
}

// Buffer management

struct StorageAllocator<D, S> where D: Device {
    buckets: Vec<StorageAllocatorBucket<S>>,
    min_size_class: usize,
    phantom: PhantomData<D>,
}

struct StorageAllocatorBucket<S> {
    free: Vec<S>,
    in_use: Vec<S>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct StorageID {
    bucket: usize,
    index: usize,
}

impl<D, S> StorageAllocator<D, S> where D: Device {
    fn new(min_size_class: usize) -> StorageAllocator<D, S> {
        StorageAllocator { buckets: vec![], min_size_class, phantom: PhantomData }
    }

    fn allocate<F>(&mut self, device: &D, size: u64, allocator: F) -> StorageID
                   where D: Device, F: FnOnce(&D, u64) -> S {
        let size_class = (64 - (size.leading_zeros() as usize)).max(self.min_size_class);
        let bucket_index = size_class - self.min_size_class;
        while self.buckets.len() < bucket_index + 1 {
            self.buckets.push(StorageAllocatorBucket { free: vec![], in_use: vec![] });
        }

        let bucket = &mut self.buckets[bucket_index];
        match bucket.free.pop() {
            Some(storage) => bucket.in_use.push(storage),
            None => bucket.in_use.push(allocator(device, 1 << size_class as u64)),
        }
        StorageID { bucket: bucket_index, index: bucket.in_use.len() - 1 }
    }

    fn get(&self, storage_id: StorageID) -> &S {
        &self.buckets[storage_id.bucket].in_use[storage_id.index]
    }

    fn end_frame(&mut self) {
        for bucket in &mut self.buckets {
            bucket.free.extend(mem::replace(&mut bucket.in_use, vec![]).into_iter())
        }
    }
}

struct FillVertexStorage<D> where D: Device {
    vertex_buffer: D::Buffer,
    auxiliary: FillVertexStorageAuxiliary<D>,
}

enum FillVertexStorageAuxiliary<D> where D: Device {
    Raster { vertex_array: FillVertexArray<D> },
    Compute {
        next_fills_buffer: D::Buffer,
        tile_map_buffer: D::Buffer,
    },
}

struct TileVertexStorage<D> where D: Device {
    tile_vertex_array: TileVertexArray<D>,
    tile_copy_vertex_array: CopyTileVertexArray<D>,
    vertex_buffer: D::Buffer,
}

impl<D> FillVertexStorage<D> where D: Device {
    fn new(size: u64,
           device: &D,
           fill_program: &FillProgram<D>,
           quad_vertex_positions_buffer: &D::Buffer,
           quad_vertex_indices_buffer: &D::Buffer)
           -> FillVertexStorage<D> {
        let vertex_buffer = device.create_buffer(BufferUploadMode::Dynamic);
        let vertex_buffer_data: BufferData<Fill> = BufferData::Uninitialized(size as usize);
        device.allocate_buffer(&vertex_buffer, vertex_buffer_data, BufferTarget::Vertex);

        let auxiliary = match *fill_program {
            FillProgram::Raster(ref fill_raster_program) => {
                FillVertexStorageAuxiliary::Raster {
                    vertex_array: FillVertexArray::new(device,
                                                       fill_raster_program,
                                                       &vertex_buffer,
                                                       quad_vertex_positions_buffer,
                                                       quad_vertex_indices_buffer),
                }
            }
            FillProgram::Compute(_) => {
                let next_fills_buffer = device.create_buffer(BufferUploadMode::Dynamic);
                let tile_map_buffer = device.create_buffer(BufferUploadMode::Dynamic);
                let next_fills_buffer_data: BufferData<i32> =
                    BufferData::Uninitialized(size as usize);
                let tile_map_buffer_data: BufferData<i32> =
                    BufferData::Uninitialized(256 * 256);
                device.allocate_buffer(&next_fills_buffer,
                                       next_fills_buffer_data,
                                       BufferTarget::Storage);
                device.allocate_buffer(&tile_map_buffer,
                                       tile_map_buffer_data,
                                       BufferTarget::Storage);
                FillVertexStorageAuxiliary::Compute { next_fills_buffer, tile_map_buffer }
            }
        };

        FillVertexStorage { vertex_buffer, auxiliary }
    }
}

impl<D> TileVertexStorage<D> where D: Device {
    fn new(size: u64,
           device: &D,
           tile_program: &TileProgram<D>,
           tile_copy_program: &CopyTileProgram<D>,
           quad_vertex_positions_buffer: &D::Buffer,
           quad_vertex_indices_buffer: &D::Buffer)
           -> TileVertexStorage<D> {
        let vertex_buffer = device.create_buffer(BufferUploadMode::Dynamic);
        device.allocate_buffer::<Tile>(&vertex_buffer,
                                       BufferData::Uninitialized(size as usize),
                                       BufferTarget::Vertex);
        let tile_vertex_array = TileVertexArray::new(device,
                                                     &tile_program,
                                                     &vertex_buffer,
                                                     &quad_vertex_positions_buffer,
                                                     &quad_vertex_indices_buffer);
        let tile_copy_vertex_array = CopyTileVertexArray::new(device,
                                                              &tile_copy_program,
                                                              &vertex_buffer,
                                                              &quad_vertex_indices_buffer);
        TileVertexStorage { vertex_buffer, tile_vertex_array, tile_copy_vertex_array }
    }
}

// Render stats

#[derive(Clone, Copy, Debug, Default)]
pub struct RenderStats {
    pub path_count: usize,
    pub fill_count: usize,
    pub alpha_tile_count: usize,
    pub solid_tile_count: usize,
    pub cpu_build_time: Duration,
}

impl Add<RenderStats> for RenderStats {
    type Output = RenderStats;
    fn add(self, other: RenderStats) -> RenderStats {
        RenderStats {
            path_count: self.path_count + other.path_count,
            solid_tile_count: self.solid_tile_count + other.solid_tile_count,
            alpha_tile_count: self.alpha_tile_count + other.alpha_tile_count,
            fill_count: self.fill_count + other.fill_count,
            cpu_build_time: self.cpu_build_time + other.cpu_build_time,
        }
    }
}

impl Div<usize> for RenderStats {
    type Output = RenderStats;
    fn div(self, divisor: usize) -> RenderStats {
        RenderStats {
            path_count: self.path_count / divisor,
            solid_tile_count: self.solid_tile_count / divisor,
            alpha_tile_count: self.alpha_tile_count / divisor,
            fill_count: self.fill_count / divisor,
            cpu_build_time: self.cpu_build_time / divisor as u32,
        }
    }
}

struct TimerQueryCache<D> where D: Device {
    free_queries: Vec<D::TimerQuery>,
}

struct PendingTimer<D> where D: Device {
    fill_times: Vec<TimerFuture<D>>,
    tile_times: Vec<TimerFuture<D>>,
}

enum TimerFuture<D> where D: Device {
    Pending(D::TimerQuery),
    Resolved(Duration),
}

impl<D> TimerQueryCache<D> where D: Device {
    fn new(_: &D) -> TimerQueryCache<D> {
        TimerQueryCache { free_queries: vec![] }
    }

    fn alloc(&mut self, device: &D) -> D::TimerQuery {
        self.free_queries.pop().unwrap_or_else(|| device.create_timer_query())
    }

    fn free(&mut self, old_query: D::TimerQuery) {
        self.free_queries.push(old_query);
    }
}

impl<D> PendingTimer<D> where D: Device {
    fn new() -> PendingTimer<D> {
        PendingTimer { fill_times: vec![], tile_times: vec![] }
    }

    fn poll(&mut self, device: &D) -> Vec<D::TimerQuery> {
        let mut old_queries = vec![];
        for future in self.fill_times.iter_mut().chain(self.tile_times.iter_mut()) {
            if let Some(old_query) = future.poll(device) {
                old_queries.push(old_query)
            }
        }
        old_queries
    }

    fn total_time(&self) -> Option<Duration> {
        let mut total = Duration::default();
        for future in self.fill_times.iter().chain(self.tile_times.iter()) {
            match *future {
                TimerFuture::Pending(_) => return None,
                TimerFuture::Resolved(time) => total += time,
            }
        }
        Some(total)
    }
}

impl<D> TimerFuture<D> where D: Device {
    fn new(query: D::TimerQuery) -> TimerFuture<D> {
        TimerFuture::Pending(query)
    }

    fn poll(&mut self, device: &D) -> Option<D::TimerQuery> {
        let duration = match *self {
            TimerFuture::Pending(ref query) => device.try_recv_timer_query(query),
            TimerFuture::Resolved(_) => None,
        };
        match duration {
            None => None,
            Some(duration) => {
                match mem::replace(self, TimerFuture::Resolved(duration)) {
                    TimerFuture::Resolved(_) => unreachable!(),
                    TimerFuture::Pending(old_query) => Some(old_query),
                }
            }
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct RenderTime {
    pub gpu_time: Duration,
}

impl Default for RenderTime {
    #[inline]
    fn default() -> RenderTime {
        RenderTime { gpu_time: Duration::new(0, 0) }
    }
}

impl Add<RenderTime> for RenderTime {
    type Output = RenderTime;

    #[inline]
    fn add(self, other: RenderTime) -> RenderTime {
        RenderTime { gpu_time: self.gpu_time + other.gpu_time }
    }
}

impl Div<usize> for RenderTime {
    type Output = RenderTime;

    #[inline]
    fn div(self, divisor: usize) -> RenderTime {
        RenderTime { gpu_time: self.gpu_time / divisor as u32 }
    }
}

bitflags! {
    struct FramebufferFlags: u8 {
        const MASK_FRAMEBUFFER_IS_DIRTY = 0x01;
        const DEST_FRAMEBUFFER_IS_DIRTY = 0x02;
    }
}

struct TextureCache<D> where D: Device {
    textures: Vec<D::Texture>,
}

impl<D> TextureCache<D> where D: Device {
    fn new() -> TextureCache<D> {
        TextureCache { textures: vec![] }
    }

    fn create_texture(&mut self, device: &mut D, format: TextureFormat, size: Vector2I)
                      -> D::Texture {
        for index in 0..self.textures.len() {
            if device.texture_size(&self.textures[index]) == size &&
                    device.texture_format(&self.textures[index]) == format {
                return self.textures.remove(index);
            }
        }

        device.create_texture(format, size)
    }

    fn release_texture(&mut self, texture: D::Texture) {
        if self.textures.len() == TEXTURE_CACHE_SIZE {
            self.textures.pop();
        }
        self.textures.insert(0, texture);
    }
}

struct TexturePage<D> where D: Device {
    framebuffer: D::Framebuffer,
    must_preserve_contents: bool,
}

struct RenderTargetInfo {
    location: TextureLocation,
}

trait ToBlendState {
    fn to_blend_state(self) -> Option<BlendState>;
}

impl ToBlendState for BlendMode {
    fn to_blend_state(self) -> Option<BlendState> {
        match self {
            BlendMode::Clear => {
                Some(BlendState {
                    src_rgb_factor: BlendFactor::Zero,
                    dest_rgb_factor: BlendFactor::Zero,
                    src_alpha_factor: BlendFactor::Zero,
                    dest_alpha_factor: BlendFactor::Zero,
                    ..BlendState::default()
                })
            }
            BlendMode::SrcOver => {
                Some(BlendState {
                    src_rgb_factor: BlendFactor::One,
                    dest_rgb_factor: BlendFactor::OneMinusSrcAlpha,
                    src_alpha_factor: BlendFactor::One,
                    dest_alpha_factor: BlendFactor::OneMinusSrcAlpha,
                    ..BlendState::default()
                })
            }
            BlendMode::DestOver => {
                Some(BlendState {
                    src_rgb_factor: BlendFactor::OneMinusDestAlpha,
                    dest_rgb_factor: BlendFactor::One,
                    src_alpha_factor: BlendFactor::OneMinusDestAlpha,
                    dest_alpha_factor: BlendFactor::One,
                    ..BlendState::default()
                })
            }
            BlendMode::SrcIn => {
                Some(BlendState {
                    src_rgb_factor: BlendFactor::DestAlpha,
                    dest_rgb_factor: BlendFactor::Zero,
                    src_alpha_factor: BlendFactor::DestAlpha,
                    dest_alpha_factor: BlendFactor::Zero,
                    ..BlendState::default()
                })
            }
            BlendMode::DestIn => {
                Some(BlendState {
                    src_rgb_factor: BlendFactor::Zero,
                    dest_rgb_factor: BlendFactor::SrcAlpha,
                    src_alpha_factor: BlendFactor::Zero,
                    dest_alpha_factor: BlendFactor::SrcAlpha,
                    ..BlendState::default()
                })
            }
            BlendMode::SrcOut => {
                Some(BlendState {
                    src_rgb_factor: BlendFactor::OneMinusDestAlpha,
                    dest_rgb_factor: BlendFactor::Zero,
                    src_alpha_factor: BlendFactor::OneMinusDestAlpha,
                    dest_alpha_factor: BlendFactor::Zero,
                    ..BlendState::default()
                })
            }
            BlendMode::DestOut => {
                Some(BlendState {
                    src_rgb_factor: BlendFactor::Zero,
                    dest_rgb_factor: BlendFactor::OneMinusSrcAlpha,
                    src_alpha_factor: BlendFactor::Zero,
                    dest_alpha_factor: BlendFactor::OneMinusSrcAlpha,
                    ..BlendState::default()
                })
            }
            BlendMode::SrcAtop => {
                Some(BlendState {
                    src_rgb_factor: BlendFactor::DestAlpha,
                    dest_rgb_factor: BlendFactor::OneMinusSrcAlpha,
                    src_alpha_factor: BlendFactor::DestAlpha,
                    dest_alpha_factor: BlendFactor::OneMinusSrcAlpha,
                    ..BlendState::default()
                })
            }
            BlendMode::DestAtop => {
                Some(BlendState {
                    src_rgb_factor: BlendFactor::OneMinusDestAlpha,
                    dest_rgb_factor: BlendFactor::SrcAlpha,
                    src_alpha_factor: BlendFactor::OneMinusDestAlpha,
                    dest_alpha_factor: BlendFactor::SrcAlpha,
                    ..BlendState::default()
                })
            }
            BlendMode::Xor => {
                Some(BlendState {
                    src_rgb_factor: BlendFactor::OneMinusDestAlpha,
                    dest_rgb_factor: BlendFactor::OneMinusSrcAlpha,
                    src_alpha_factor: BlendFactor::OneMinusDestAlpha,
                    dest_alpha_factor: BlendFactor::OneMinusSrcAlpha,
                    ..BlendState::default()
                })
            }
            BlendMode::Lighter => {
                Some(BlendState {
                    src_rgb_factor: BlendFactor::One,
                    dest_rgb_factor: BlendFactor::One,
                    src_alpha_factor: BlendFactor::One,
                    dest_alpha_factor: BlendFactor::One,
                    ..BlendState::default()
                })
            }
            BlendMode::Copy |
            BlendMode::Darken |
            BlendMode::Lighten |
            BlendMode::Multiply |
            BlendMode::Screen |
            BlendMode::HardLight |
            BlendMode::Overlay |
            BlendMode::ColorDodge |
            BlendMode::ColorBurn |
            BlendMode::SoftLight |
            BlendMode::Difference |
            BlendMode::Exclusion |
            BlendMode::Hue |
            BlendMode::Saturation |
            BlendMode::Color |
            BlendMode::Luminosity => {
                // Blending is done manually in the shader.
                None
            }
        }
    }
}

pub trait BlendModeExt {
    fn needs_readable_framebuffer(self) -> bool;
}

impl BlendModeExt for BlendMode {
    fn needs_readable_framebuffer(self) -> bool {
        match self {
            BlendMode::Clear |
            BlendMode::SrcOver |
            BlendMode::DestOver |
            BlendMode::SrcIn |
            BlendMode::DestIn |
            BlendMode::SrcOut |
            BlendMode::DestOut |
            BlendMode::SrcAtop |
            BlendMode::DestAtop |
            BlendMode::Xor |
            BlendMode::Lighter |
            BlendMode::Copy => false,
            BlendMode::Lighten |
            BlendMode::Darken |
            BlendMode::Multiply |
            BlendMode::Screen |
            BlendMode::HardLight |
            BlendMode::Overlay |
            BlendMode::ColorDodge |
            BlendMode::ColorBurn |
            BlendMode::SoftLight |
            BlendMode::Difference |
            BlendMode::Exclusion |
            BlendMode::Hue |
            BlendMode::Saturation |
            BlendMode::Color |
            BlendMode::Luminosity => true,
        }
    }
}

struct AlphaTilePage<D> where D: Device {
    buffered_fills: Vec<Fill>,
    pending_fills: Vec<Fill>,
    framebuffer: D::Framebuffer,
    framebuffer_is_dirty: bool,
}

impl<D> AlphaTilePage<D> where D: Device {
    fn new(device: &mut D) -> AlphaTilePage<D> {
        let framebuffer_size = vec2i(MASK_FRAMEBUFFER_WIDTH, MASK_FRAMEBUFFER_HEIGHT);
        let framebuffer_texture = device.create_texture(TextureFormat::RGBA16F, framebuffer_size);
        let framebuffer = device.create_framebuffer(framebuffer_texture);
        AlphaTilePage {
            buffered_fills: vec![],
            pending_fills: vec![],
            framebuffer,
            framebuffer_is_dirty: false,
        }
    }
}

bitflags! {
    struct RendererFlags: u8 {
        // Whether we need a depth buffer.
        const USE_DEPTH = 0x01;
        // Whether an intermediate destination framebuffer is needed.
        //
        // This will be true if any exotic blend modes are used at the top level (not inside a
        // render target), *and* the output framebuffer is the default framebuffer.
        const INTERMEDIATE_DEST_FRAMEBUFFER_NEEDED = 0x02;
    }
}

trait ToCompositeCtrl {
    fn to_composite_ctrl(&self) -> i32;
}

impl ToCompositeCtrl for BlendMode {
    fn to_composite_ctrl(&self) -> i32 {
        match *self {
            BlendMode::SrcOver |
            BlendMode::SrcAtop |
            BlendMode::DestOver |
            BlendMode::DestOut |
            BlendMode::Xor |
            BlendMode::Lighter |
            BlendMode::Clear |
            BlendMode::Copy |
            BlendMode::SrcIn |
            BlendMode::SrcOut |
            BlendMode::DestIn |
            BlendMode::DestAtop => COMBINER_CTRL_COMPOSITE_NORMAL,
            BlendMode::Multiply => COMBINER_CTRL_COMPOSITE_MULTIPLY,
            BlendMode::Darken => COMBINER_CTRL_COMPOSITE_DARKEN,
            BlendMode::Lighten => COMBINER_CTRL_COMPOSITE_LIGHTEN,
            BlendMode::Screen => COMBINER_CTRL_COMPOSITE_SCREEN,
            BlendMode::Overlay => COMBINER_CTRL_COMPOSITE_OVERLAY,
            BlendMode::ColorDodge => COMBINER_CTRL_COMPOSITE_COLOR_DODGE,
            BlendMode::ColorBurn => COMBINER_CTRL_COMPOSITE_COLOR_BURN,
            BlendMode::HardLight => COMBINER_CTRL_COMPOSITE_HARD_LIGHT,
            BlendMode::SoftLight => COMBINER_CTRL_COMPOSITE_SOFT_LIGHT,
            BlendMode::Difference => COMBINER_CTRL_COMPOSITE_DIFFERENCE,
            BlendMode::Exclusion => COMBINER_CTRL_COMPOSITE_EXCLUSION,
            BlendMode::Hue => COMBINER_CTRL_COMPOSITE_HUE,
            BlendMode::Saturation => COMBINER_CTRL_COMPOSITE_SATURATION,
            BlendMode::Color => COMBINER_CTRL_COMPOSITE_COLOR,
            BlendMode::Luminosity => COMBINER_CTRL_COMPOSITE_LUMINOSITY,
        }
    }
}

trait ToCombineMode {
    fn to_combine_mode(self) -> i32;
}

impl ToCombineMode for PaintCompositeOp {
    fn to_combine_mode(self) -> i32 {
        match self {
            PaintCompositeOp::DestIn => COMBINER_CTRL_COLOR_COMBINE_DEST_IN,
            PaintCompositeOp::SrcIn => COMBINER_CTRL_COLOR_COMBINE_SRC_IN,
        }
    }
}
