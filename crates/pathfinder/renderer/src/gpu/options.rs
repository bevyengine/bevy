// pathfinder/renderer/src/gpu/options.rs
//
// Copyright © 2019 The Pathfinder Project Developers.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use pathfinder_color::ColorF;
use pathfinder_geometry::rect::RectI;
use pathfinder_geometry::vector::Vector2I;
use pathfinder_gpu::Device;

/// Options that influence rendering.
#[derive(Default)]
pub struct RendererOptions {
    pub background_color: Option<ColorF>,
    pub no_compute: bool,
}

#[derive(Clone)]
pub enum DestFramebuffer<D> where D: Device {
    Default {
        viewport: RectI,
        window_size: Vector2I,
    },
    Other(D::Framebuffer),
}

impl<D> Default for DestFramebuffer<D> where D: Device {
    #[inline]
    fn default() -> DestFramebuffer<D> {
        DestFramebuffer::Default { viewport: RectI::default(), window_size: Vector2I::default() }
    }
}

impl<D> DestFramebuffer<D>
where
    D: Device,
{
    #[inline]
    pub fn full_window(window_size: Vector2I) -> DestFramebuffer<D> {
        let viewport = RectI::new(Vector2I::default(), window_size);
        DestFramebuffer::Default { viewport, window_size }
    }

    #[inline]
    pub fn window_size(&self, device: &D) -> Vector2I {
        match *self {
            DestFramebuffer::Default { window_size, .. } => window_size,
            DestFramebuffer::Other(ref framebuffer) => {
                device.texture_size(device.framebuffer_texture(framebuffer))
            }
        }
    }
}
