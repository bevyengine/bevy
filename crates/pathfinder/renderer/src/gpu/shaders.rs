// pathfinder/renderer/src/gpu/shaders.rs
//
// Copyright © 2020 The Pathfinder Project Developers.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use crate::gpu_data::Fill;
use pathfinder_gpu::{BufferData, BufferTarget, BufferUploadMode, Device, VertexAttrClass};
use pathfinder_gpu::{VertexAttrDescriptor, VertexAttrType};
use pathfinder_resources::ResourceLoader;

// TODO(pcwalton): Replace with `mem::size_of` calls?
const FILL_INSTANCE_SIZE: usize = 8;
const TILE_INSTANCE_SIZE: usize = 12;
const CLIP_TILE_INSTANCE_SIZE: usize = 8;

pub const MAX_FILLS_PER_BATCH: usize = 0x4000;

pub struct BlitVertexArray<D> where D: Device {
    pub vertex_array: D::VertexArray,
}

impl<D> BlitVertexArray<D> where D: Device {
    pub fn new(device: &D,
               blit_program: &BlitProgram<D>,
               quad_vertex_positions_buffer: &D::Buffer,
               quad_vertex_indices_buffer: &D::Buffer)
               -> BlitVertexArray<D> {
        let vertex_array = device.create_vertex_array();
        let position_attr = device.get_vertex_attr(&blit_program.program, "Position").unwrap();

        device.bind_buffer(&vertex_array, quad_vertex_positions_buffer, BufferTarget::Vertex);
        device.configure_vertex_attr(&vertex_array, &position_attr, &VertexAttrDescriptor {
            size: 2,
            class: VertexAttrClass::Int,
            attr_type: VertexAttrType::I16,
            stride: 4,
            offset: 0,
            divisor: 0,
            buffer_index: 0,
        });
        device.bind_buffer(&vertex_array, quad_vertex_indices_buffer, BufferTarget::Index);

        BlitVertexArray { vertex_array }
    }
}

pub struct FillVertexArray<D>
where
    D: Device,
{
    pub vertex_array: D::VertexArray,
    pub vertex_buffer: D::Buffer,
}

impl<D> FillVertexArray<D>
where
    D: Device,
{
    pub fn new(
        device: &D,
        fill_program: &FillProgram<D>,
        quad_vertex_positions_buffer: &D::Buffer,
        quad_vertex_indices_buffer: &D::Buffer,
    ) -> FillVertexArray<D> {
        let vertex_array = device.create_vertex_array();

        let vertex_buffer = device.create_buffer();
        let vertex_buffer_data: BufferData<Fill> = BufferData::Uninitialized(MAX_FILLS_PER_BATCH);
        device.allocate_buffer(
            &vertex_buffer,
            vertex_buffer_data,
            BufferTarget::Vertex,
            BufferUploadMode::Dynamic,
        );

        let tess_coord_attr = device.get_vertex_attr(&fill_program.program, "TessCoord").unwrap();
        let from_px_attr = device.get_vertex_attr(&fill_program.program, "FromPx").unwrap();
        let to_px_attr = device.get_vertex_attr(&fill_program.program, "ToPx").unwrap();
        let from_subpx_attr = device.get_vertex_attr(&fill_program.program, "FromSubpx").unwrap();
        let to_subpx_attr = device.get_vertex_attr(&fill_program.program, "ToSubpx").unwrap();
        let tile_index_attr = device.get_vertex_attr(&fill_program.program, "TileIndex").unwrap();

        device.bind_buffer(&vertex_array, quad_vertex_positions_buffer, BufferTarget::Vertex);
        device.configure_vertex_attr(&vertex_array, &tess_coord_attr, &VertexAttrDescriptor {
            size: 2,
            class: VertexAttrClass::Int,
            attr_type: VertexAttrType::U16,
            stride: 4,
            offset: 0,
            divisor: 0,
            buffer_index: 0,
        });
        device.bind_buffer(&vertex_array, &vertex_buffer, BufferTarget::Vertex);
        device.configure_vertex_attr(&vertex_array, &from_px_attr, &VertexAttrDescriptor {
            size: 1,
            class: VertexAttrClass::Int,
            attr_type: VertexAttrType::U8,
            stride: FILL_INSTANCE_SIZE,
            offset: 0,
            divisor: 1,
            buffer_index: 1,
        });
        device.configure_vertex_attr(&vertex_array, &to_px_attr, &VertexAttrDescriptor {
            size: 1,
            class: VertexAttrClass::Int,
            attr_type: VertexAttrType::U8,
            stride: FILL_INSTANCE_SIZE,
            offset: 1,
            divisor: 1,
            buffer_index: 1,
        });
        device.configure_vertex_attr(&vertex_array, &from_subpx_attr, &VertexAttrDescriptor {
            size: 2,
            class: VertexAttrClass::FloatNorm,
            attr_type: VertexAttrType::U8,
            stride: FILL_INSTANCE_SIZE,
            offset: 2,
            divisor: 1,
            buffer_index: 1,
        });
        device.configure_vertex_attr(&vertex_array, &to_subpx_attr, &VertexAttrDescriptor {
            size: 2,
            class: VertexAttrClass::FloatNorm,
            attr_type: VertexAttrType::U8,
            stride: FILL_INSTANCE_SIZE,
            offset: 4,
            divisor: 1,
            buffer_index: 1,
        });
        device.configure_vertex_attr(&vertex_array, &tile_index_attr, &VertexAttrDescriptor {
            size: 1,
            class: VertexAttrClass::Int,
            attr_type: VertexAttrType::U16,
            stride: FILL_INSTANCE_SIZE,
            offset: 6,
            divisor: 1,
            buffer_index: 1,
        });
        device.bind_buffer(&vertex_array, quad_vertex_indices_buffer, BufferTarget::Index);

        FillVertexArray { vertex_array, vertex_buffer }
    }
}

pub struct TileVertexArray<D> where D: Device {
    pub vertex_array: D::VertexArray,
}

impl<D> TileVertexArray<D> where D: Device {
    pub fn new(device: &D,
               tile_program: &TileProgram<D>,
               tile_vertex_buffer: &D::Buffer,
               quad_vertex_positions_buffer: &D::Buffer,
               quad_vertex_indices_buffer: &D::Buffer)
               -> TileVertexArray<D> {
        let vertex_array = device.create_vertex_array();

        let tile_offset_attr =
            device.get_vertex_attr(&tile_program.program, "TileOffset").unwrap();
        let tile_origin_attr =
            device.get_vertex_attr(&tile_program.program, "TileOrigin").unwrap();
        let mask_0_tex_coord_attr =
            device.get_vertex_attr(&tile_program.program, "MaskTexCoord0").unwrap();
        let mask_backdrop_attr =
            device.get_vertex_attr(&tile_program.program, "MaskBackdrop").unwrap();
        let color_attr = device.get_vertex_attr(&tile_program.program, "Color").unwrap();
        let tile_ctrl_attr = device.get_vertex_attr(&tile_program.program, "TileCtrl").unwrap();

        device.bind_buffer(&vertex_array, quad_vertex_positions_buffer, BufferTarget::Vertex);
        device.configure_vertex_attr(&vertex_array, &tile_offset_attr, &VertexAttrDescriptor {
            size: 2,
            class: VertexAttrClass::Int,
            attr_type: VertexAttrType::I16,
            stride: 4,
            offset: 0,
            divisor: 0,
            buffer_index: 0,
        });
        device.bind_buffer(&vertex_array, tile_vertex_buffer, BufferTarget::Vertex);
        device.configure_vertex_attr(&vertex_array, &tile_origin_attr, &VertexAttrDescriptor {
            size: 2,
            class: VertexAttrClass::Int,
            attr_type: VertexAttrType::I16,
            stride: TILE_INSTANCE_SIZE,
            offset: 0,
            divisor: 1,
            buffer_index: 1,
        });
        device.configure_vertex_attr(&vertex_array, &mask_0_tex_coord_attr, &VertexAttrDescriptor {
            size: 2,
            class: VertexAttrClass::Int,
            attr_type: VertexAttrType::U8,
            stride: TILE_INSTANCE_SIZE,
            offset: 4,
            divisor: 1,
            buffer_index: 1,
        });
        device.configure_vertex_attr(&vertex_array, &mask_backdrop_attr, &VertexAttrDescriptor {
            size: 2,
            class: VertexAttrClass::Int,
            attr_type: VertexAttrType::I8,
            stride: TILE_INSTANCE_SIZE,
            offset: 6,
            divisor: 1,
            buffer_index: 1,
        });
        device.configure_vertex_attr(&vertex_array, &color_attr, &VertexAttrDescriptor {
            size: 1,
            class: VertexAttrClass::Int,
            attr_type: VertexAttrType::I16,
            stride: TILE_INSTANCE_SIZE,
            offset: 8,
            divisor: 1,
            buffer_index: 1,
        });
        device.configure_vertex_attr(&vertex_array, &tile_ctrl_attr, &VertexAttrDescriptor {
            size: 1,
            class: VertexAttrClass::Int,
            attr_type: VertexAttrType::I16,
            stride: TILE_INSTANCE_SIZE,
            offset: 10,
            divisor: 1,
            buffer_index: 1,
        });
        device.bind_buffer(&vertex_array, quad_vertex_indices_buffer, BufferTarget::Index);

        TileVertexArray { vertex_array }
    }
}

pub struct CopyTileVertexArray<D> where D: Device {
    pub vertex_array: D::VertexArray,
}

impl<D> CopyTileVertexArray<D> where D: Device {
    pub fn new(
        device: &D,
        copy_tile_program: &CopyTileProgram<D>,
        copy_tile_vertex_buffer: &D::Buffer,
        quads_vertex_indices_buffer: &D::Buffer,
    ) -> CopyTileVertexArray<D> {
        let vertex_array = device.create_vertex_array();

        let tile_position_attr =
            device.get_vertex_attr(&copy_tile_program.program, "TilePosition").unwrap();

        device.bind_buffer(&vertex_array, copy_tile_vertex_buffer, BufferTarget::Vertex);
        device.configure_vertex_attr(&vertex_array, &tile_position_attr, &VertexAttrDescriptor {
            size: 2,
            class: VertexAttrClass::Int,
            attr_type: VertexAttrType::I16,
            stride: TILE_INSTANCE_SIZE,
            offset: 0,
            divisor: 0,
            buffer_index: 0,
        });
        device.bind_buffer(&vertex_array, quads_vertex_indices_buffer, BufferTarget::Index);

        CopyTileVertexArray { vertex_array }
    }
}

pub struct ClipTileVertexArray<D> where D: Device {
    pub vertex_array: D::VertexArray,
    pub vertex_buffer: D::Buffer,
}

impl<D> ClipTileVertexArray<D> where D: Device {
    pub fn new(device: &D,
               clip_tile_program: &ClipTileProgram<D>,
               quad_vertex_positions_buffer: &D::Buffer,
               quad_vertex_indices_buffer: &D::Buffer)
               -> ClipTileVertexArray<D> {
        let vertex_array = device.create_vertex_array();
        let vertex_buffer = device.create_buffer();

        let tile_offset_attr =
            device.get_vertex_attr(&clip_tile_program.program, "TileOffset").unwrap();
        let dest_tile_origin_attr =
            device.get_vertex_attr(&clip_tile_program.program, "DestTileOrigin").unwrap();
        let src_tile_origin_attr =
            device.get_vertex_attr(&clip_tile_program.program, "SrcTileOrigin").unwrap();
        let src_backdrop_attr =
            device.get_vertex_attr(&clip_tile_program.program, "SrcBackdrop").unwrap();

        device.bind_buffer(&vertex_array, quad_vertex_positions_buffer, BufferTarget::Vertex);
        device.configure_vertex_attr(&vertex_array, &tile_offset_attr, &VertexAttrDescriptor {
            size: 2,
            class: VertexAttrClass::Int,
            attr_type: VertexAttrType::I16,
            stride: 4,
            offset: 0,
            divisor: 0,
            buffer_index: 0,
        });
        device.bind_buffer(&vertex_array, &vertex_buffer, BufferTarget::Vertex);
        device.configure_vertex_attr(&vertex_array, &dest_tile_origin_attr, &VertexAttrDescriptor {
            size: 2,
            class: VertexAttrClass::Int,
            attr_type: VertexAttrType::U8,
            stride: CLIP_TILE_INSTANCE_SIZE,
            offset: 0,
            divisor: 1,
            buffer_index: 1,
        });
        device.configure_vertex_attr(&vertex_array, &src_tile_origin_attr, &VertexAttrDescriptor {
            size: 2,
            class: VertexAttrClass::Int,
            attr_type: VertexAttrType::U8,
            stride: CLIP_TILE_INSTANCE_SIZE,
            offset: 2,
            divisor: 1,
            buffer_index: 1,
        });
        device.configure_vertex_attr(&vertex_array, &src_backdrop_attr, &VertexAttrDescriptor {
            size: 1,
            class: VertexAttrClass::Int,
            attr_type: VertexAttrType::I8,
            stride: CLIP_TILE_INSTANCE_SIZE,
            offset: 4,
            divisor: 1,
            buffer_index: 1,
        });
        device.bind_buffer(&vertex_array, quad_vertex_indices_buffer, BufferTarget::Index);

        ClipTileVertexArray { vertex_array, vertex_buffer }
    }
}


pub struct BlitProgram<D> where D: Device {
    pub program: D::Program,
    pub src_uniform: D::Uniform,
}

impl<D> BlitProgram<D> where D: Device {
    pub fn new(device: &D, resources: &dyn ResourceLoader) -> BlitProgram<D> {
        let program = device.create_program(resources, "blit");
        let src_uniform = device.get_uniform(&program, "Src");
        BlitProgram { program, src_uniform }
    }
}

pub struct FillProgram<D>
where
    D: Device,
{
    pub program: D::Program,
    pub framebuffer_size_uniform: D::Uniform,
    pub tile_size_uniform: D::Uniform,
    pub area_lut_uniform: D::Uniform,
}

impl<D> FillProgram<D>
where
    D: Device,
{
    pub fn new(device: &D, resources: &dyn ResourceLoader) -> FillProgram<D> {
        let program = device.create_program(resources, "fill");
        let framebuffer_size_uniform = device.get_uniform(&program, "FramebufferSize");
        let tile_size_uniform = device.get_uniform(&program, "TileSize");
        let area_lut_uniform = device.get_uniform(&program, "AreaLUT");
        FillProgram {
            program,
            framebuffer_size_uniform,
            tile_size_uniform,
            area_lut_uniform,
        }
    }
}

pub struct TileProgram<D> where D: Device {
    pub program: D::Program,
    pub transform_uniform: D::Uniform,
    pub tile_size_uniform: D::Uniform,
    pub texture_metadata_uniform: D::Uniform,
    pub texture_metadata_size_uniform: D::Uniform,
    pub dest_texture_uniform: D::Uniform,
    pub color_texture_0_uniform: D::Uniform,
    pub color_texture_1_uniform: D::Uniform,
    pub mask_texture_0_uniform: D::Uniform,
    pub gamma_lut_uniform: D::Uniform,
    pub color_texture_0_size_uniform: D::Uniform,
    pub filter_params_0_uniform: D::Uniform,
    pub filter_params_1_uniform: D::Uniform,
    pub filter_params_2_uniform: D::Uniform,
    pub framebuffer_size_uniform: D::Uniform,
    pub ctrl_uniform: D::Uniform,
}

impl<D> TileProgram<D> where D: Device {
    pub fn new(device: &D, resources: &dyn ResourceLoader) -> TileProgram<D> {
        let program = device.create_program(resources, "tile");
        let transform_uniform = device.get_uniform(&program, "Transform");
        let tile_size_uniform = device.get_uniform(&program, "TileSize");
        let texture_metadata_uniform = device.get_uniform(&program, "TextureMetadata");
        let texture_metadata_size_uniform = device.get_uniform(&program, "TextureMetadataSize");
        let dest_texture_uniform = device.get_uniform(&program, "DestTexture");
        let color_texture_0_uniform = device.get_uniform(&program, "ColorTexture0");
        let color_texture_1_uniform = device.get_uniform(&program, "ColorTexture1");
        let mask_texture_0_uniform = device.get_uniform(&program, "MaskTexture0");
        let gamma_lut_uniform = device.get_uniform(&program, "GammaLUT");
        let color_texture_0_size_uniform = device.get_uniform(&program, "ColorTexture0Size");
        let filter_params_0_uniform = device.get_uniform(&program, "FilterParams0");
        let filter_params_1_uniform = device.get_uniform(&program, "FilterParams1");
        let filter_params_2_uniform = device.get_uniform(&program, "FilterParams2");
        let framebuffer_size_uniform = device.get_uniform(&program, "FramebufferSize");
        let ctrl_uniform = device.get_uniform(&program, "Ctrl");
        TileProgram {
            program,
            transform_uniform,
            tile_size_uniform,
            texture_metadata_uniform,
            texture_metadata_size_uniform,
            dest_texture_uniform,
            color_texture_0_uniform,
            color_texture_1_uniform,
            mask_texture_0_uniform,
            gamma_lut_uniform,
            color_texture_0_size_uniform,
            filter_params_0_uniform,
            filter_params_1_uniform,
            filter_params_2_uniform,
            framebuffer_size_uniform,
            ctrl_uniform,
        }
    }
}

pub struct CopyTileProgram<D> where D: Device {
    pub program: D::Program,
    pub transform_uniform: D::Uniform,
    pub tile_size_uniform: D::Uniform,
    pub framebuffer_size_uniform: D::Uniform,
    pub src_uniform: D::Uniform,
}

impl<D> CopyTileProgram<D> where D: Device {
    pub fn new(device: &D, resources: &dyn ResourceLoader) -> CopyTileProgram<D> {
        let program = device.create_program(resources, "tile_copy");
        let transform_uniform = device.get_uniform(&program, "Transform");
        let tile_size_uniform = device.get_uniform(&program, "TileSize");
        let framebuffer_size_uniform = device.get_uniform(&program, "FramebufferSize");
        let src_uniform = device.get_uniform(&program, "Src");
        CopyTileProgram {
            program,
            transform_uniform,
            tile_size_uniform,
            framebuffer_size_uniform,
            src_uniform,
        }
    }
}

pub struct ClipTileProgram<D> where D: Device {
    pub program: D::Program,
    pub src_uniform: D::Uniform,
}

impl<D> ClipTileProgram<D> where D: Device {
    pub fn new(device: &D, resources: &dyn ResourceLoader) -> ClipTileProgram<D> {
        let program = device.create_program(resources, "tile_clip");
        let src_uniform = device.get_uniform(&program, "Src");
        ClipTileProgram { program, src_uniform }
    }
}

pub struct StencilProgram<D>
where
    D: Device,
{
    pub program: D::Program,
}

impl<D> StencilProgram<D>
where
    D: Device,
{
    pub fn new(device: &D, resources: &dyn ResourceLoader) -> StencilProgram<D> {
        let program = device.create_program(resources, "stencil");
        StencilProgram { program }
    }
}

pub struct StencilVertexArray<D>
where
    D: Device,
{
    pub vertex_array: D::VertexArray,
    pub vertex_buffer: D::Buffer,
    pub index_buffer: D::Buffer,
}

impl<D> StencilVertexArray<D>
where
    D: Device,
{
    pub fn new(device: &D, stencil_program: &StencilProgram<D>) -> StencilVertexArray<D> {
        let vertex_array = device.create_vertex_array();
        let (vertex_buffer, index_buffer) = (device.create_buffer(), device.create_buffer());

        let position_attr = device.get_vertex_attr(&stencil_program.program, "Position").unwrap();

        device.bind_buffer(&vertex_array, &vertex_buffer, BufferTarget::Vertex);
        device.configure_vertex_attr(&vertex_array, &position_attr, &VertexAttrDescriptor {
            size: 3,
            class: VertexAttrClass::Float,
            attr_type: VertexAttrType::F32,
            stride: 4 * 4,
            offset: 0,
            divisor: 0,
            buffer_index: 0,
        });
        device.bind_buffer(&vertex_array, &index_buffer, BufferTarget::Index);

        StencilVertexArray { vertex_array, vertex_buffer, index_buffer }
    }
}

pub struct ReprojectionProgram<D>
where
    D: Device,
{
    pub program: D::Program,
    pub old_transform_uniform: D::Uniform,
    pub new_transform_uniform: D::Uniform,
    pub texture_uniform: D::Uniform,
}

impl<D> ReprojectionProgram<D>
where
    D: Device,
{
    pub fn new(device: &D, resources: &dyn ResourceLoader) -> ReprojectionProgram<D> {
        let program = device.create_program(resources, "reproject");
        let old_transform_uniform = device.get_uniform(&program, "OldTransform");
        let new_transform_uniform = device.get_uniform(&program, "NewTransform");
        let texture_uniform = device.get_uniform(&program, "Texture");

        ReprojectionProgram {
            program,
            old_transform_uniform,
            new_transform_uniform,
            texture_uniform,
        }
    }
}

pub struct ReprojectionVertexArray<D>
where
    D: Device,
{
    pub vertex_array: D::VertexArray,
}

impl<D> ReprojectionVertexArray<D>
where
    D: Device,
{
    pub fn new(
        device: &D,
        reprojection_program: &ReprojectionProgram<D>,
        quad_vertex_positions_buffer: &D::Buffer,
        quad_vertex_indices_buffer: &D::Buffer,
    ) -> ReprojectionVertexArray<D> {
        let vertex_array = device.create_vertex_array();
        let position_attr = device.get_vertex_attr(&reprojection_program.program, "Position")
                                  .unwrap();

        device.bind_buffer(&vertex_array, quad_vertex_positions_buffer, BufferTarget::Vertex);
        device.configure_vertex_attr(&vertex_array, &position_attr, &VertexAttrDescriptor {
            size: 2,
            class: VertexAttrClass::Int,
            attr_type: VertexAttrType::I16,
            stride: 4,
            offset: 0,
            divisor: 0,
            buffer_index: 0,
        });
        device.bind_buffer(&vertex_array, quad_vertex_indices_buffer, BufferTarget::Index);

        ReprojectionVertexArray { vertex_array }
    }
}
