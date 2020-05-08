// pathfinder/demo/common/src/device.rs
//
// Copyright © 2019 The Pathfinder Project Developers.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! GPU rendering code specifically for the demo.

use pathfinder_gpu::{BufferTarget, Device, VertexAttrClass, VertexAttrDescriptor, VertexAttrType};
use pathfinder_resources::ResourceLoader;

pub struct GroundProgram<D>
where
    D: Device,
{
    pub program: D::Program,
    pub transform_uniform: D::Uniform,
    pub gridline_count_uniform: D::Uniform,
    pub ground_color_uniform: D::Uniform,
    pub gridline_color_uniform: D::Uniform,
}

impl<D> GroundProgram<D>
where
    D: Device,
{
    pub fn new(device: &D, resources: &dyn ResourceLoader) -> GroundProgram<D> {
        let program = device.create_program(resources, "demo_ground");
        let transform_uniform = device.get_uniform(&program, "Transform");
        let gridline_count_uniform = device.get_uniform(&program, "GridlineCount");
        let ground_color_uniform = device.get_uniform(&program, "GroundColor");
        let gridline_color_uniform = device.get_uniform(&program, "GridlineColor");
        GroundProgram {
            program,
            transform_uniform,
            gridline_count_uniform,
            ground_color_uniform,
            gridline_color_uniform,
        }
    }
}

pub struct GroundVertexArray<D>
where
    D: Device,
{
    pub vertex_array: D::VertexArray,
}

impl<D> GroundVertexArray<D>
where
    D: Device,
{
    pub fn new(
        device: &D,
        ground_program: &GroundProgram<D>,
        quad_vertex_positions_buffer: &D::Buffer,
        quad_vertex_indices_buffer: &D::Buffer,
    ) -> GroundVertexArray<D> {
        let vertex_array = device.create_vertex_array();

        let position_attr = device.get_vertex_attr(&ground_program.program, "Position").unwrap();

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

        GroundVertexArray { vertex_array }
    }
}
