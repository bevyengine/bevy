// pathfinder/demo/common/src/renderer.rs
//
// Copyright © 2019 The Pathfinder Project Developers.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Rendering functionality for the demo.

use crate::camera::{Camera, Mode};
use crate::window::{View, Window};
use crate::{BackgroundColor, DemoApp, UIVisibility};
use image::ColorType;
use pathfinder_color::{ColorF, ColorU};
use pathfinder_gpu::{ClearOps, DepthFunc, DepthState, Device, Primitive, RenderOptions};
use pathfinder_gpu::{RenderState, RenderTarget, TextureData, TextureFormat, UniformData};
use pathfinder_geometry::rect::RectI;
use pathfinder_geometry::transform3d::Transform4F;
use pathfinder_geometry::vector::{Vector2I, Vector4F};
use pathfinder_renderer::gpu::options::{DestFramebuffer, RendererOptions};
use pathfinder_renderer::options::RenderTransform;
use std::path::PathBuf;

const GROUND_SOLID_COLOR: ColorU = ColorU {
    r: 80,
    g: 80,
    b: 80,
    a: 255,
};

const GROUND_LINE_COLOR: ColorU = ColorU {
    r: 127,
    g: 127,
    b: 127,
    a: 255,
};

const GRIDLINE_COUNT: i32 = 10;

impl<W> DemoApp<W> where W: Window {
    pub fn prepare_frame_rendering(&mut self) -> u32 {
        // Make the context current.
        let view = self.ui_model.mode.view(0);
        self.window.make_current(view);

        // Set up framebuffers.
        let window_size = self.window_size.device_size();
        let mode = self.camera.mode();
        let scene_count = match mode {
            Mode::VR => {
                let viewport = self.window.viewport(View::Stereo(0));
                if self.scene_framebuffer.is_none()
                    || self.renderer.device.texture_size(
                        &self
                            .renderer
                            .device
                            .framebuffer_texture(self.scene_framebuffer.as_ref().unwrap()),
                    ) != viewport.size()
                {
                    let scene_texture = self
                        .renderer
                        .device
                        .create_texture(TextureFormat::RGBA8, viewport.size());
                    self.scene_framebuffer =
                        Some(self.renderer.device.create_framebuffer(scene_texture));
                }
                self.renderer
                    .replace_dest_framebuffer(DestFramebuffer::Other(
                        self.scene_framebuffer.take().unwrap(),
                    ));
                2
            }
            _ => {
                self.renderer
                    .replace_dest_framebuffer(DestFramebuffer::Default {
                        viewport: self.window.viewport(View::Mono),
                        window_size,
                    });
                1
            }
        };

        // Clear to the appropriate color.
        let clear_color = match mode {
            Mode::TwoD => Some(self.ui_model.background_color().to_f32()),
            Mode::ThreeD => None,
            Mode::VR => Some(ColorF::transparent_black()),
        };
        self.renderer.set_options(RendererOptions { background_color: clear_color });

        scene_count
    }

    pub fn draw_scene(&mut self) {
        self.renderer.device.begin_commands();

        let view = self.ui_model.mode.view(0);
        self.window.make_current(view);

        if self.camera.mode() != Mode::VR {
            self.draw_environment(0);
        }

        self.renderer.device.end_commands();

        self.render_vector_scene();

        // Reattach default framebuffer.
        if self.camera.mode() == Mode::VR {
            if let DestFramebuffer::Other(scene_framebuffer) =
                self.renderer
                    .replace_dest_framebuffer(DestFramebuffer::Default {
                        viewport: self.window.viewport(View::Mono),
                        window_size: self.window_size.device_size(),
                    })
            {
                self.scene_framebuffer = Some(scene_framebuffer);
            }
        }
    }

    pub fn begin_compositing(&mut self) {
        self.renderer.device.begin_commands();
    }

    pub fn composite_scene(&mut self, render_scene_index: u32) {
        let (eye_transforms, scene_transform, modelview_transform) = match self.camera {
            Camera::ThreeD {
                ref eye_transforms,
                ref scene_transform,
                ref modelview_transform,
                ..
            } if eye_transforms.len() > 1 => (eye_transforms, scene_transform, modelview_transform),
            _ => return,
        };

        debug!(
            "scene_transform.perspective={:?}",
            scene_transform.perspective
        );
        debug!(
            "scene_transform.modelview_to_eye={:?}",
            scene_transform.modelview_to_eye
        );
        debug!("modelview transform={:?}", modelview_transform);

        let viewport = self.window.viewport(View::Stereo(render_scene_index));
        self.window.make_current(View::Stereo(render_scene_index));

        self.renderer.replace_dest_framebuffer(DestFramebuffer::Default {
            viewport,
            window_size: self.window_size.device_size(),
        });

        self.draw_environment(render_scene_index);

        let scene_framebuffer = self.scene_framebuffer.as_ref().unwrap();
        let scene_texture = self.renderer.device.framebuffer_texture(scene_framebuffer);

        let mut quad_scale = self.scene_metadata.view_box.size().to_4d();
        quad_scale.set_z(1.0);
        let quad_scale_transform = Transform4F::from_scale(quad_scale);

        let scene_transform_matrix = scene_transform.perspective *
            scene_transform.modelview_to_eye *
            modelview_transform.to_transform() *
            quad_scale_transform;

        let eye_transform = &eye_transforms[render_scene_index as usize];
        let eye_transform_matrix = eye_transform.perspective *
            eye_transform.modelview_to_eye *
            modelview_transform.to_transform() *
            quad_scale_transform;

        debug!(
            "eye transform({}).modelview_to_eye={:?}",
            render_scene_index, eye_transform.modelview_to_eye
        );
        debug!(
            "eye transform_matrix({})={:?}",
            render_scene_index, eye_transform_matrix
        );
        debug!("---");

        self.renderer.reproject_texture(
            scene_texture,
            &scene_transform_matrix.transform,
            &eye_transform_matrix.transform,
        );
    }

    // Draws the ground, if applicable.
    fn draw_environment(&self, render_scene_index: u32) {
        let frame = &self.current_frame.as_ref().unwrap();

        let perspective = match frame.transform {
            RenderTransform::Transform2D(..) => return,
            RenderTransform::Perspective(perspective) => perspective,
        };

        if self.ui_model.background_color == BackgroundColor::Transparent {
            return;
        }

        let ground_scale = self.scene_metadata.view_box.max_x() * 2.0;

        let mut offset = self.scene_metadata.view_box.lower_right().to_4d();
        offset.set_z(ground_scale);
        offset = offset * Vector4F::new(-0.5, 1.0, -0.5, 1.0);
        let base_transform = perspective.transform * Transform4F::from_translation(offset);

        // Fill ground.
        let transform = base_transform *
            Transform4F::from_scale(Vector4F::new(ground_scale, 1.0, ground_scale, 1.0));

        // Don't clear the first scene after drawing it.
        let clear_color = if render_scene_index == 0 {
            Some(self.ui_model.background_color().to_f32())
        } else {
            None
        };

        self.renderer.device.draw_elements(6, &RenderState {
            target: &self.renderer.draw_render_target(),
            program: &self.ground_program.program,
            vertex_array: &self.ground_vertex_array.vertex_array,
            primitive: Primitive::Triangles,
            textures: &[],
            uniforms: &[
                (&self.ground_program.transform_uniform,
                 UniformData::from_transform_3d(&transform)),
                (&self.ground_program.ground_color_uniform,
                 UniformData::Vec4(GROUND_SOLID_COLOR.to_f32().0)),
                (&self.ground_program.gridline_color_uniform,
                 UniformData::Vec4(GROUND_LINE_COLOR.to_f32().0)),
                (&self.ground_program.gridline_count_uniform, UniformData::Int(GRIDLINE_COUNT)),
            ],
            viewport: self.renderer.draw_viewport(),
            options: RenderOptions {
                depth: Some(DepthState { func: DepthFunc::Less, write: true }),
                clear_ops: ClearOps { color: clear_color, depth: Some(1.0), stencil: Some(0) },
                ..RenderOptions::default()
            },
        });
    }

    fn render_vector_scene(&mut self) {
        if self.ui_model.mode == Mode::TwoD {
            self.renderer.disable_depth();
        } else {
            self.renderer.enable_depth();
        }

        self.renderer.begin_scene();

        // Issue render commands!
        for command in self.render_command_stream.as_mut().unwrap() {
            self.renderer.render_command(&command);
        }

        self.current_frame
            .as_mut()
            .unwrap()
            .scene_stats
            .push(self.renderer.stats);
        self.renderer.end_scene();
    }

    pub fn take_raster_screenshot(&mut self, path: PathBuf) {
        let drawable_size = self.window_size.device_size();
        let viewport = RectI::new(Vector2I::default(), drawable_size);
        let texture_data_receiver =
            self.renderer.device.read_pixels(&RenderTarget::Default, viewport);
        let pixels = match self.renderer.device.recv_texture_data(&texture_data_receiver) {
            TextureData::U8(pixels) => pixels,
            _ => panic!("Unexpected pixel format for default framebuffer!"),
        };
        image::save_buffer(
            path,
            &pixels,
            drawable_size.x() as u32,
            drawable_size.y() as u32,
            ColorType::Rgba8,
        )
        .unwrap();
    }

    pub fn draw_debug_ui(&mut self) {
        if self.options.ui == UIVisibility::None {
            return;
        }

        let viewport = self.window.viewport(View::Mono);
        self.window.make_current(View::Mono);
        self.renderer.replace_dest_framebuffer(DestFramebuffer::Default {
            viewport,
            window_size: self.window_size.device_size(),
        });

        self.renderer.draw_debug_ui();
    }
}
