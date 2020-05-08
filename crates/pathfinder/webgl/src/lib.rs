// pathfinder/webgl/src/lib.rs
//
// Copyright © 2020 The Pathfinder Project Developers.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! A WebGL implementation of the device abstraction.

#[macro_use]
extern crate log;

use pathfinder_geometry::rect::RectI;
use pathfinder_geometry::vector::Vector2I;
use pathfinder_gpu::{BlendFactor, BlendOp, BufferData, BufferTarget, RenderTarget};
use pathfinder_gpu::{BufferUploadMode, ClearOps, DepthFunc, Device, Primitive, RenderOptions};
use pathfinder_gpu::{RenderState, ShaderKind, StencilFunc, TextureData, TextureDataRef};
use pathfinder_gpu::{TextureFormat, TextureSamplingFlags, UniformData, VertexAttrClass};
use pathfinder_gpu::{VertexAttrDescriptor, VertexAttrType};
use pathfinder_resources::ResourceLoader;
use std::mem;
use std::str;
use std::time::Duration;
use wasm_bindgen::{JsCast, JsValue};
use web_sys::WebGl2RenderingContext as WebGl;
use js_sys::{Uint8Array, Uint16Array, Float32Array, Object};

pub struct WebGlDevice {
    context: web_sys::WebGl2RenderingContext,
}

impl WebGlDevice {
    pub fn new(context: web_sys::WebGl2RenderingContext) -> Self {
        context.get_extension("EXT_color_buffer_float").unwrap();
        WebGlDevice { context }
    }

    // Error checking

    #[cfg(debug_assertions)]
    fn ck(&self) {
        let mut num_errors = 0;
        loop {
            let err = self.context.get_error();
            println!(
                "GL error: 0x{:x} ({})",
                err,
                match err {
                    WebGl::NO_ERROR => break,
                    WebGl::INVALID_ENUM => "INVALID_ENUM",
                    WebGl::INVALID_VALUE => "INVALID_VALUE",
                    WebGl::INVALID_OPERATION => "INVALID_OPERATION",
                    WebGl::INVALID_FRAMEBUFFER_OPERATION => "INVALID_FRAMEBUFFER_OPERATION",
                    WebGl::OUT_OF_MEMORY => "OUT_OF_MEMORY",
                    _ => "Unknown",
                }
            );
            num_errors += 1;
        }
        if num_errors > 0 {
            panic!("aborting due to {} errors", num_errors);
        }
    }

    #[cfg(not(debug_assertions))]
    #[inline]
    fn ck(&self) {}

    #[inline]
    fn bind_texture(&self, texture: &WebGlTexture, unit: u32) {
        self.context.active_texture(WebGl::TEXTURE0 + unit);
        self.context
            .bind_texture(WebGl::TEXTURE_2D, Some(&texture.texture));
    }

    #[inline]
    fn unbind_texture(&self, unit: u32) {
        self.context.active_texture(WebGl::TEXTURE0 + unit);
        self.context.bind_texture(WebGl::TEXTURE_2D, None);
    }

    #[inline]
    fn bind_render_target(&self, attachment: &RenderTarget<WebGlDevice>) {
        let framebuffer = match *attachment {
            RenderTarget::Default => None,
            RenderTarget::Framebuffer(framebuffer) => Some(framebuffer),
        };
        self.context
            .bind_framebuffer(WebGl::FRAMEBUFFER, framebuffer.map(|f| &f.framebuffer));
    }

    #[inline]
    fn bind_vertex_array(&self, vertex_array: &WebGlVertexArray) {
        self.context
            .bind_vertex_array(Some(&vertex_array.gl_vertex_array));
        self.ck();
    }

    #[inline]
    fn unbind_vertex_array(&self) {
        self.context.bind_vertex_array(None);
        self.ck();
    }
    #[inline]
    fn set_uniform(&self, uniform: &WebGlUniform, data: &UniformData) {
        let location = uniform.location.as_ref();

        match *data {
            UniformData::Float(value) => {
                self.context.uniform1f(location, value);
                self.ck();
            }
            UniformData::Int(value) => {
                self.context.uniform1i(location, value);
                self.ck();
            }
            UniformData::Mat2(data) => {
                self.context.uniform_matrix2fv_with_f32_array(
                    location,
                    false,
                    &[data.x(), data.y(), data.z(), data.w()],
                );
            }
            UniformData::Mat4(data) => {
                self.context.uniform_matrix4fv_with_f32_array(
                    location,
                    false,
                    &[
                        data[0].x(),
                        data[0].y(),
                        data[0].z(),
                        data[0].w(),
                        data[1].x(),
                        data[1].y(),
                        data[1].z(),
                        data[1].w(),
                        data[2].x(),
                        data[2].y(),
                        data[2].z(),
                        data[2].w(),
                        data[3].x(),
                        data[3].y(),
                        data[3].z(),
                        data[3].w(),
                    ],
                );
            }
            UniformData::Vec2(data) => {
                self.context.uniform2f(location, data.x(), data.y());
                self.ck();
            }
            UniformData::Vec3(data) => {
                self.context.uniform3f(location, data[0], data[1], data[2]);
                self.ck();
            }
            UniformData::Vec4(data) => {
                self.context
                    .uniform4f(location, data.x(), data.y(), data.z(), data.w());
                self.ck();
            }
            UniformData::IVec2(data) => {
                self.context.uniform2i(location, data[0], data[1]);
                self.ck();
            }
            UniformData::IVec3(data) => {
                self.context.uniform3i(location, data[0], data[1], data[2]);
                self.ck();
            }
            UniformData::TextureUnit(unit) => {
                self.context.uniform1i(location, unit as i32);
                self.ck();
            }
        }
    }
    fn set_render_state(&self, render_state: &RenderState<WebGlDevice>) {
        self.bind_render_target(render_state.target);

        let (origin, size) = (render_state.viewport.origin(), render_state.viewport.size());
        self.context
            .viewport(origin.x(), origin.y(), size.x(), size.y());

        if render_state.options.clear_ops.has_ops() {
            self.clear(&render_state.options.clear_ops);
        }

        self.context
            .use_program(Some(&render_state.program.gl_program));
        self.context
            .bind_vertex_array(Some(&render_state.vertex_array.gl_vertex_array));
        for (texture_unit, texture) in render_state.textures.iter().enumerate() {
            self.bind_texture(texture, texture_unit as u32);
        }

        for (uniform, data) in render_state.uniforms {
            self.set_uniform(uniform, data);
        }
        self.set_render_options(&render_state.options);
    }

    fn set_render_options(&self, render_options: &RenderOptions) {
        match render_options.blend {
            None => {
                self.context.disable(WebGl::BLEND);
                self.ck();
            }
            Some(blend) => {
                let func = match blend.op {
                    BlendOp::Add => WebGl::FUNC_ADD,
                    BlendOp::Subtract => WebGl::FUNC_SUBTRACT,
                    BlendOp::ReverseSubtract => WebGl::FUNC_REVERSE_SUBTRACT,
                    BlendOp::Max => WebGl::MAX,
                    BlendOp::Min => WebGl::MIN,
                };
                self.context.blend_equation(func);
                self.ck();

                let func = |f| match f {
                    BlendFactor::Zero => WebGl::ZERO,
                    BlendFactor::One => WebGl::ONE,
                    BlendFactor::SrcAlpha => WebGl::SRC_ALPHA,
                    BlendFactor::OneMinusSrcAlpha => WebGl::ONE_MINUS_SRC_ALPHA,
                    BlendFactor::DestAlpha => WebGl::DST_ALPHA,
                    BlendFactor::OneMinusDestAlpha => WebGl::ONE_MINUS_DST_ALPHA,
                    BlendFactor::DestColor => WebGl::DST_COLOR,
                };

                self.context.blend_func_separate(
                    func(blend.src_rgb_factor),
                    func(blend.dest_rgb_factor),
                    func(blend.src_alpha_factor),
                    func(blend.dest_alpha_factor),
                );
                self.context.enable(WebGl::BLEND);
                self.ck();
            }
        }

        // Set depth.
        match render_options.depth {
            None => {
                self.context.disable(WebGl::DEPTH_TEST);
                self.ck();
            }
            Some(ref state) => {
                self.context.depth_func(state.func.to_gl_depth_func());
                self.ck();
                self.context.depth_mask(state.write as bool);
                self.ck();
                self.context.enable(WebGl::DEPTH_TEST);
                self.ck();
            }
        }

        // Set stencil.
        match render_options.stencil {
            None => {
                self.context.disable(WebGl::STENCIL_TEST);
                self.ck();
            }
            Some(ref state) => {
                self.context.stencil_func(
                    state.func.to_gl_stencil_func(),
                    state.reference as i32,
                    state.mask,
                );
                self.ck();
                let (pass_action, write_mask) = if state.write {
                    (WebGl::REPLACE, state.mask)
                } else {
                    (WebGl::KEEP, 0)
                };
                self.context
                    .stencil_op(WebGl::KEEP, WebGl::KEEP, pass_action);
                self.ck();
                self.context.stencil_mask(write_mask);
                self.context.enable(WebGl::STENCIL_TEST);
                self.ck();
            }
        }

        // Set color mask.
        let color_mask = render_options.color_mask as bool;
        self.context
            .color_mask(color_mask, color_mask, color_mask, color_mask);
        self.ck();
    }

    fn reset_render_state(&self, render_state: &RenderState<WebGlDevice>) {
        self.reset_render_options(&render_state.options);
        for texture_unit in 0..(render_state.textures.len() as u32) {
            self.unbind_texture(texture_unit);
        }
        self.context.use_program(None);
        self.unbind_vertex_array();
    }

    fn reset_render_options(&self, render_options: &RenderOptions) {
        if render_options.blend.is_some() {
            self.context.disable(WebGl::BLEND);
        }

        if render_options.depth.is_some() {
            self.context.disable(WebGl::DEPTH_TEST);
        }

        if render_options.stencil.is_some() {
            self.context.stencil_mask(!0);
            self.context.disable(WebGl::STENCIL_TEST);
        }

        self.context.color_mask(true, true, true, true);
        self.ck();
    }

    #[inline]
    fn clear(&self, ops: &ClearOps) {
        let mut flags = 0;
        if let Some(color) = ops.color {
            self.context.color_mask(true, true, true, true);
            self.context
                .clear_color(color.r(), color.g(), color.b(), color.a());
            flags |= WebGl::COLOR_BUFFER_BIT;
        }
        if let Some(depth) = ops.depth {
            self.context.depth_mask(true);
            self.context.clear_depth(depth as _);
            flags |= WebGl::DEPTH_BUFFER_BIT;
        }
        if let Some(stencil) = ops.stencil {
            self.context.stencil_mask(!0);
            self.context.clear_stencil(stencil as i32);
            flags |= WebGl::STENCIL_BUFFER_BIT;
        }
        if flags != 0 {
            self.context.clear(flags);
        }
    }

    fn preprocess(&self, source: &[u8], version: &str) -> String {
        let source = std::str::from_utf8(source).unwrap();
        let mut output = String::new();

        let mut pos = 0;
        while let Some(index) = source[pos..].find("{{") {
            let index = index + pos;
            if index > pos {
                output.push_str(&source[pos..index]);
            }
            let end_index = index + 2 + source[index + 2..].find("}").unwrap();
            assert_eq!(&source[end_index + 1..end_index + 2], "}");
            let ident = &source[index + 2..end_index];
            if ident == "version" {
                output.push_str(version);
            } else {
                panic!("unknown template variable: `{}`", ident);
            }
            pos = end_index + 2;
        }
        output.push_str(&source[pos..]);
        /*
        for (line_nr, line) in output.lines().enumerate() {
            debug!("{:3}: {}", line_nr + 1, line);
        }
        */
        output
    }
}

fn slice_to_u8<T>(slice: &[T]) -> &[u8] {
    unsafe {
        std::slice::from_raw_parts(
            slice.as_ptr() as *const u8,
            slice.len() * mem::size_of::<T>(),
        )
    }
}

// this function is unsafe due to the underlying UintXArray::view
unsafe fn check_and_extract_data(
    data_ref: TextureDataRef,
    minimum_size: Vector2I,
    format: TextureFormat,
) -> Object {
    let channels = match (format, data_ref) {
        (TextureFormat::R8, TextureDataRef::U8(_)) => 1,
        (TextureFormat::RGBA8, TextureDataRef::U8(_)) => 4,
        (TextureFormat::RGBA16F, TextureDataRef::F16(_)) => 4,
        (TextureFormat::RGBA32F, TextureDataRef::F32(_)) => 4,
        _ => panic!("Unimplemented texture format!"),
    };

    let area = minimum_size.x() as usize * minimum_size.y() as usize;

    match data_ref {
        TextureDataRef::U8(data) => {
            assert!(data.len() >= area * channels);
            Uint8Array::view(data).unchecked_into()
        }
        TextureDataRef::F16(data) => {
            assert!(data.len() >= area * channels);
            Uint16Array::view_mut_raw(data.as_ptr() as *mut u16, data.len()).unchecked_into()
        }
        TextureDataRef::F32(data) => {
            assert!(data.len() >= area * channels);
            Float32Array::view(data).unchecked_into()
        }
    }
}

impl Device for WebGlDevice {
    type Buffer = WebGlBuffer;
    type Framebuffer = WebGlFramebuffer;
    type Program = WebGlProgram;
    type Shader = WebGlShader;
    type Texture = WebGlTexture;
    type TextureDataReceiver = ();
    type TimerQuery = WebGlTimerQuery;
    type Uniform = WebGlUniform;
    type VertexArray = WebGlVertexArray;
    type VertexAttr = WebGlVertexAttr;

    fn create_texture(&self, format: TextureFormat, size: Vector2I) -> WebGlTexture {
        let texture = self.context.create_texture().unwrap();
        let texture = WebGlTexture {
            texture,
            format,
            size,
            context: self.context.clone(),
        };
        self.bind_texture(&texture, 0);
        self.context
            .tex_image_2d_with_i32_and_i32_and_i32_and_format_and_type_and_opt_u8_array(
                WebGl::TEXTURE_2D,
                0,
                format.gl_internal_format() as i32,
                size.x(),
                size.y(),
                0,
                format.gl_format(),
                format.gl_type(),
                None,
            )
            .unwrap();

        self.set_texture_sampling_mode(&texture, TextureSamplingFlags::empty());
        texture
    }

    fn create_texture_from_data(
        &self,
        format: TextureFormat,
        size: Vector2I,
        data_ref: TextureDataRef,
    ) -> WebGlTexture {
        let data = unsafe {
            check_and_extract_data(data_ref, size, format)
        };

        let texture = self.context.create_texture().unwrap();
        let texture = WebGlTexture {
            texture,
            format,
            size,
            context: self.context.clone(),
        };

        self.bind_texture(&texture, 0);
        self.context
            .tex_image_2d_with_i32_and_i32_and_i32_and_format_and_type_and_opt_array_buffer_view(
                WebGl::TEXTURE_2D,
                0,
                format.gl_internal_format() as i32,
                size.x(),
                size.y(),
                0,
                format.gl_format(),
                format.gl_type(),
                Some(&data),
            )
            .unwrap();

        self.set_texture_sampling_mode(&texture, TextureSamplingFlags::empty());
        texture
    }

    #[inline]
    fn texture_format(&self, texture: &Self::Texture) -> TextureFormat {
        texture.format
    }

    fn create_shader_from_source(
        &self,
        name: &str,
        source: &[u8],
        kind: ShaderKind,
    ) -> WebGlShader {
        let glsl_version_spec = "300 es";

        let source = self.preprocess(source, glsl_version_spec);

        let gl_shader_kind = match kind {
            ShaderKind::Vertex => WebGl::VERTEX_SHADER,
            ShaderKind::Fragment => WebGl::FRAGMENT_SHADER,
        };

        let gl_shader = self
            .context
            .create_shader(gl_shader_kind)
            .expect("could not create shader");
        self.context.shader_source(&gl_shader, &source);
        self.context.compile_shader(&gl_shader);
        let compile_status = self
            .context
            .get_shader_parameter(&gl_shader, WebGl::COMPILE_STATUS);
        if !compile_status.as_bool().unwrap_or(false) {
            if let Some(info_log) = self.context.get_shader_info_log(&gl_shader) {
                info!("Shader info log:\n{}", info_log);
            }
            panic!("{:?} shader '{}' compilation failed", kind, name);
        }

        WebGlShader { gl_shader }
    }

    fn create_program_from_shaders(
        &self,
        _resources: &dyn ResourceLoader,
        name: &str,
        vertex_shader: WebGlShader,
        fragment_shader: WebGlShader,
    ) -> WebGlProgram {
        let gl_program = self
            .context
            .create_program()
            .expect("unable to create program object");
        self.context
            .attach_shader(&gl_program, &vertex_shader.gl_shader);
        self.context
            .attach_shader(&gl_program, &fragment_shader.gl_shader);
        self.context.link_program(&gl_program);
        if !self
            .context
            .get_program_parameter(&gl_program, WebGl::LINK_STATUS)
            .as_bool()
            .unwrap_or(false)
        {
            if let Some(info_log) = self.context.get_program_info_log(&gl_program) {
                info!("Program info log for {}:\n{}", name, info_log);
            }
            panic!("Program {:?} linking failed", name);
        }

        WebGlProgram {
            context: self.context.clone(),
            gl_program,
        }
    }

    #[inline]
    fn create_vertex_array(&self) -> WebGlVertexArray {
        WebGlVertexArray {
            context: self.context.clone(),
            gl_vertex_array: self.context.create_vertex_array().unwrap(),
        }
    }

    fn get_vertex_attr(&self, program: &WebGlProgram, name: &str) -> Option<WebGlVertexAttr> {
        let name = format!("a{}", name);
        let attr = self.context.get_attrib_location(&program.gl_program, &name);
        if attr < 0 {
            return None;
        }
        Some(WebGlVertexAttr { attr: attr as u32 })
    }

    fn get_uniform(&self, program: &WebGlProgram, name: &str) -> WebGlUniform {
        let name = format!("u{}", name);
        let location = self
            .context
            .get_uniform_location(&program.gl_program, &name);
        self.ck();
        WebGlUniform { location: location }
    }

    fn configure_vertex_attr(
        &self,
        vertex_array: &WebGlVertexArray,
        attr: &WebGlVertexAttr,
        descriptor: &VertexAttrDescriptor,
    ) {
        debug_assert_ne!(descriptor.stride, 0);

        self.context
            .bind_vertex_array(Some(&vertex_array.gl_vertex_array));

        let attr_type = descriptor.attr_type.to_gl_type();
        match descriptor.class {
            VertexAttrClass::Float | VertexAttrClass::FloatNorm => {
                let normalized = descriptor.class == VertexAttrClass::FloatNorm;
                self.context.vertex_attrib_pointer_with_i32(
                    attr.attr,
                    descriptor.size as i32,
                    attr_type,
                    normalized,
                    descriptor.stride as i32,
                    descriptor.offset as i32,
                );
            }
            VertexAttrClass::Int => {
                self.context.vertex_attrib_i_pointer_with_i32(
                    attr.attr,
                    descriptor.size as i32,
                    attr_type,
                    descriptor.stride as i32,
                    descriptor.offset as i32,
                );
            }
        }

        self.context
            .vertex_attrib_divisor(attr.attr, descriptor.divisor);
        self.context.enable_vertex_attrib_array(attr.attr);
        self.context.bind_vertex_array(None);
    }

    fn create_framebuffer(&self, texture: WebGlTexture) -> WebGlFramebuffer {
        debug!(
            "texture size = {:?}, format = {:?}",
            texture.size, texture.format
        );
        let gl_framebuffer = self.context.create_framebuffer().unwrap();
        self.context
            .bind_framebuffer(WebGl::FRAMEBUFFER, Some(&gl_framebuffer));
        self.bind_texture(&texture, 0);

        self.context.framebuffer_texture_2d(
            WebGl::FRAMEBUFFER,
            WebGl::COLOR_ATTACHMENT0,
            WebGl::TEXTURE_2D,
            Some(&texture.texture),
            0,
        );
        self.ck();
        match self.context.check_framebuffer_status(WebGl::FRAMEBUFFER) {
            WebGl::FRAMEBUFFER_COMPLETE => {}
            WebGl::FRAMEBUFFER_INCOMPLETE_ATTACHMENT => panic!("FRAMEBUFFER_INCOMPLETE_ATTACHMENT"),
            WebGl::FRAMEBUFFER_INCOMPLETE_MISSING_ATTACHMENT => {
                panic!("FRAMEBUFFER_INCOMPLETE_MISSING_ATTACHMENT")
            }
            WebGl::FRAMEBUFFER_INCOMPLETE_DIMENSIONS => panic!("FRAMEBUFFER_INCOMPLETE_DIMENSIONS"),
            WebGl::FRAMEBUFFER_UNSUPPORTED => panic!("FRAMEBUFFER_UNSUPPORTED"),
            WebGl::FRAMEBUFFER_INCOMPLETE_MULTISAMPLE => {
                panic!("FRAMEBUFFER_INCOMPLETE_MULTISAMPLE")
            }
            WebGl::RENDERBUFFER_SAMPLES => panic!("RENDERBUFFER_SAMPLES"),
            code => panic!("unknown code {}", code),
        }

        WebGlFramebuffer {
            framebuffer: gl_framebuffer,
            texture,
        }
    }

    fn destroy_framebuffer(&self, framebuffer: Self::Framebuffer) -> Self::Texture {
        self.context
            .delete_framebuffer(Some(&framebuffer.framebuffer));
        framebuffer.texture
    }

    fn create_buffer(&self) -> WebGlBuffer {
        let buffer = self.context.create_buffer().unwrap();
        WebGlBuffer {
            buffer,
            context: self.context.clone(),
        }
    }

    fn allocate_buffer<T>(
        &self,
        buffer: &WebGlBuffer,
        data: BufferData<T>,
        target: BufferTarget,
        mode: BufferUploadMode,
    ) {
        let target = match target {
            BufferTarget::Vertex => WebGl::ARRAY_BUFFER,
            BufferTarget::Index => WebGl::ELEMENT_ARRAY_BUFFER,
        };
        self.context.bind_buffer(target, Some(&buffer.buffer));
        self.ck();
        let usage = mode.to_gl_usage();
        match data {
            BufferData::Uninitialized(len) => {
                self.context
                    .buffer_data_with_i32(target, (len * mem::size_of::<T>()) as i32, usage)
            }
            BufferData::Memory(buffer) => {
                self.context
                    .buffer_data_with_u8_array(target, slice_to_u8(buffer), usage)
            }
        }
    }

    #[inline]
    fn framebuffer_texture<'f>(&self, framebuffer: &'f Self::Framebuffer) -> &'f Self::Texture {
        &framebuffer.texture
    }

    #[inline]
    fn texture_size(&self, texture: &Self::Texture) -> Vector2I {
        texture.size
    }

    fn set_texture_sampling_mode(&self, texture: &Self::Texture, flags: TextureSamplingFlags) {
        self.bind_texture(texture, 0);
        self.context
            .tex_parameteri(WebGl::TEXTURE_2D,
                            WebGl::TEXTURE_MIN_FILTER,
                            if flags.contains(TextureSamplingFlags::NEAREST_MIN) {
                                WebGl::NEAREST as i32
                            } else {
                                WebGl::LINEAR as i32
                            });
        self.context
            .tex_parameteri(WebGl::TEXTURE_2D,
                            WebGl::TEXTURE_MAG_FILTER,
                            if flags.contains(TextureSamplingFlags::NEAREST_MAG) {
                                WebGl::NEAREST as i32
                            } else {
                                WebGl::LINEAR as i32
                            });
        self.context
            .tex_parameteri(WebGl::TEXTURE_2D,
                            WebGl::TEXTURE_WRAP_S,
                            if flags.contains(TextureSamplingFlags::REPEAT_U) {
                                WebGl::REPEAT as i32
                            } else {
                                WebGl::CLAMP_TO_EDGE as i32
                            });
        self.context
            .tex_parameteri(WebGl::TEXTURE_2D,
                            WebGl::TEXTURE_WRAP_T,
                            if flags.contains(TextureSamplingFlags::REPEAT_V) {
                                WebGl::REPEAT as i32
                            } else {
                                WebGl::CLAMP_TO_EDGE as i32
                            });
    }

    fn upload_to_texture(&self, texture: &WebGlTexture, rect: RectI, data_ref: TextureDataRef) {
        let data = unsafe {
            check_and_extract_data(data_ref, rect.size(), texture.format)
        };
        assert!(rect.size().x() >= 0);
        assert!(rect.size().y() >= 0);
        assert!(rect.max_x() <= texture.size.x());
        assert!(rect.max_y() <= texture.size.y());

        self.bind_texture(texture, 0);
        if rect.origin() == Vector2I::default() && rect.size() == texture.size {
            self.context
                .tex_image_2d_with_i32_and_i32_and_i32_and_format_and_type_and_opt_array_buffer_view(
                    WebGl::TEXTURE_2D,
                    0,
                    texture.format.gl_internal_format() as i32,
                    rect.width(),
                    rect.height(),
                    0,
                    texture.format.gl_format(),
                    texture.format.gl_type(),
                    Some(&data),
                )
                .unwrap();
        } else {
            self.context
                .tex_sub_image_2d_with_i32_and_i32_and_u32_and_type_and_opt_array_buffer_view(
                    WebGl::TEXTURE_2D,
                    0,
                    rect.origin().x(),
                    rect.origin().y(),
                    rect.width(),
                    rect.height(),
                    texture.format.gl_format(),
                    texture.format.gl_type(),
                    Some(&data),
                )
                .unwrap();
        }

        self.set_texture_sampling_mode(&texture, TextureSamplingFlags::empty());
    }

    fn read_pixels(&self, _render_target: &RenderTarget<WebGlDevice>, _viewport: RectI) -> () {
        panic!("read_pixels is not supported");
    }

    fn begin_commands(&self) {
        // TODO(pcwalton): Add some checks in debug mode to make sure render commands are bracketed
        // by these?
    }

    fn end_commands(&self) {
        self.context.flush();
    }

    fn draw_arrays(&self, index_count: u32, render_state: &RenderState<Self>) {
        self.set_render_state(render_state);
        self.context.draw_arrays(
            render_state.primitive.to_gl_primitive(),
            0,
            index_count as i32,
        );
        self.reset_render_state(render_state);
    }

    fn draw_elements(&self, index_count: u32, render_state: &RenderState<Self>) {
        self.set_render_state(render_state);
        self.context.draw_elements_with_i32(
            render_state.primitive.to_gl_primitive(),
            index_count as i32,
            WebGl::UNSIGNED_INT,
            0,
        );
        self.reset_render_state(render_state);
    }

    fn draw_elements_instanced(
        &self,
        index_count: u32,
        instance_count: u32,
        render_state: &RenderState<Self>,
    ) {
        self.set_render_state(render_state);
        self.context.draw_elements_instanced_with_i32(
            render_state.primitive.to_gl_primitive(),
            index_count as i32,
            WebGl::UNSIGNED_INT,
            0,
            instance_count as i32,
        );
        self.reset_render_state(render_state);
    }

    #[inline]
    fn create_timer_query(&self) -> WebGlTimerQuery {
        // FIXME use performance timers
        WebGlTimerQuery {}
    }

    #[inline]
    fn begin_timer_query(&self, _query: &Self::TimerQuery) {
        // FIXME use performance timers
    }

    #[inline]
    fn end_timer_query(&self, _: &Self::TimerQuery) {
        // FIXME use performance timers
    }

    #[inline]
    fn try_recv_timer_query(&self, _query: &WebGlTimerQuery) -> Option<Duration> {
        None
    }

    #[inline]
    fn recv_timer_query(&self, _query: &WebGlTimerQuery) -> Duration {
        Duration::from_millis(0)
    }
    fn try_recv_texture_data(&self, _receiver: &Self::TextureDataReceiver) -> Option<TextureData> {
        None
    }
    fn recv_texture_data(&self, _receiver: &Self::TextureDataReceiver) -> TextureData {
        unimplemented!()
    }

    #[inline]
    fn bind_buffer(
        &self,
        vertex_array: &WebGlVertexArray,
        buffer: &WebGlBuffer,
        target: BufferTarget,
    ) {
        self.bind_vertex_array(vertex_array);
        self.context
            .bind_buffer(target.to_gl_target(), Some(&buffer.buffer));
        self.unbind_vertex_array();
    }

    #[inline]
    fn create_shader(
        &self,
        resources: &dyn ResourceLoader,
        name: &str,
        kind: ShaderKind,
    ) -> Self::Shader {
        let suffix = match kind {
            ShaderKind::Vertex => 'v',
            ShaderKind::Fragment => 'f',
        };
        let path = format!("shaders/gl3/{}.{}s.glsl", name, suffix);
        self.create_shader_from_source(name, &resources.slurp(&path).unwrap(), kind)
    }
}

pub struct WebGlVertexArray {
    context: web_sys::WebGl2RenderingContext,
    pub gl_vertex_array: web_sys::WebGlVertexArrayObject,
}

impl Drop for WebGlVertexArray {
    #[inline]
    fn drop(&mut self) {
        self.context
            .delete_vertex_array(Some(&self.gl_vertex_array));
    }
}

pub struct WebGlVertexAttr {
    attr: u32,
}

pub struct WebGlFramebuffer {
    pub framebuffer: web_sys::WebGlFramebuffer,
    pub texture: WebGlTexture,
}

pub struct WebGlBuffer {
    context: web_sys::WebGl2RenderingContext,
    pub buffer: web_sys::WebGlBuffer,
}

impl Drop for WebGlBuffer {
    fn drop(&mut self) {
        self.context.delete_buffer(Some(&self.buffer));
    }
}

#[derive(Debug)]
pub struct WebGlUniform {
    location: Option<web_sys::WebGlUniformLocation>,
}

pub struct WebGlProgram {
    context: web_sys::WebGl2RenderingContext,
    pub gl_program: web_sys::WebGlProgram,
}

impl Drop for WebGlProgram {
    fn drop(&mut self) {
        self.context.delete_program(Some(&self.gl_program));
    }
}

pub struct WebGlShader {
    gl_shader: web_sys::WebGlShader,
}

pub struct WebGlTexture {
    context: web_sys::WebGl2RenderingContext,
    texture: web_sys::WebGlTexture,
    pub size: Vector2I,
    pub format: TextureFormat,
}
impl Drop for WebGlTexture {
    fn drop(&mut self) {
        self.context.delete_texture(Some(&self.texture));
    }
}

pub struct WebGlTimerQuery {}

trait BufferTargetExt {
    fn to_gl_target(self) -> u32;
}

impl BufferTargetExt for BufferTarget {
    fn to_gl_target(self) -> u32 {
        match self {
            BufferTarget::Vertex => WebGl::ARRAY_BUFFER,
            BufferTarget::Index => WebGl::ELEMENT_ARRAY_BUFFER,
        }
    }
}

trait BufferUploadModeExt {
    fn to_gl_usage(self) -> u32;
}

impl BufferUploadModeExt for BufferUploadMode {
    fn to_gl_usage(self) -> u32 {
        match self {
            BufferUploadMode::Static => WebGl::STATIC_DRAW,
            BufferUploadMode::Dynamic => WebGl::DYNAMIC_DRAW,
        }
    }
}

trait DepthFuncExt {
    fn to_gl_depth_func(self) -> u32;
}

impl DepthFuncExt for DepthFunc {
    fn to_gl_depth_func(self) -> u32 {
        match self {
            DepthFunc::Less => WebGl::LESS,
            DepthFunc::Always => WebGl::ALWAYS,
        }
    }
}

trait PrimitiveExt {
    fn to_gl_primitive(self) -> u32;
}

impl PrimitiveExt for Primitive {
    fn to_gl_primitive(self) -> u32 {
        match self {
            Primitive::Triangles => WebGl::TRIANGLES,
            Primitive::Lines => WebGl::LINES,
        }
    }
}

trait StencilFuncExt {
    fn to_gl_stencil_func(self) -> u32;
}

impl StencilFuncExt for StencilFunc {
    fn to_gl_stencil_func(self) -> u32 {
        match self {
            StencilFunc::Always => WebGl::ALWAYS,
            StencilFunc::Equal => WebGl::EQUAL,
        }
    }
}

trait TextureFormatExt {
    fn gl_internal_format(self) -> u32;
    fn gl_format(self) -> u32;
    fn gl_type(self) -> u32;
}

impl TextureFormatExt for TextureFormat {
    fn gl_internal_format(self) -> u32 {
        match self {
            TextureFormat::R8 => WebGl::R8,
            TextureFormat::R16F => WebGl::R16F,
            TextureFormat::RGBA8 => WebGl::RGBA,
            TextureFormat::RGBA16F => WebGl::RGBA16F,
            TextureFormat::RGBA32F => WebGl::RGBA32F,
        }
    }

    fn gl_format(self) -> u32 {
        match self {
            TextureFormat::R8 | TextureFormat::R16F => WebGl::RED,
            TextureFormat::RGBA8 | TextureFormat::RGBA16F | TextureFormat::RGBA32F => WebGl::RGBA,
        }
    }

    fn gl_type(self) -> u32 {
        match self {
            TextureFormat::R8 | TextureFormat::RGBA8 => WebGl::UNSIGNED_BYTE,
            TextureFormat::R16F | TextureFormat::RGBA16F => WebGl::HALF_FLOAT,
            TextureFormat::RGBA32F => WebGl::FLOAT,
        }
    }
}

trait VertexAttrTypeExt {
    fn to_gl_type(self) -> u32;
}

impl VertexAttrTypeExt for VertexAttrType {
    fn to_gl_type(self) -> u32 {
        match self {
            VertexAttrType::F32 => WebGl::FLOAT,
            VertexAttrType::I16 => WebGl::SHORT,
            VertexAttrType::I8 => WebGl::BYTE,
            VertexAttrType::U16 => WebGl::UNSIGNED_SHORT,
            VertexAttrType::U8 => WebGl::UNSIGNED_BYTE,
        }
    }
}

/// The version/dialect of OpenGL we should render with.
#[derive(Clone, Copy)]
#[repr(u32)]
pub enum GLVersion {
    /// OpenGL 3.0+, core profile.
    GL3 = 0,
    /// OpenGL ES 3.0+.
    GLES3 = 1,
}
