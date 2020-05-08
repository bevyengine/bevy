// pathfinder/renderer/src/options.rs
//
// Copyright © 2019 The Pathfinder Project Developers.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Options that control how rendering is to be performed.

use crate::gpu_data::RenderCommand;
use pathfinder_geometry::rect::RectF;
use pathfinder_geometry::transform2d::Transform2F;
use pathfinder_geometry::transform3d::Perspective;
use pathfinder_geometry::vector::{Vector2F, Vector4F};
use pathfinder_content::clip::PolygonClipper3D;

pub trait RenderCommandListener: Send + Sync {
    fn send(&self, command: RenderCommand);
}

impl<F> RenderCommandListener for F
where
    F: Fn(RenderCommand) + Send + Sync,
{
    #[inline]
    fn send(&self, command: RenderCommand) {
        (*self)(command)
    }
}

/// Options that influence scene building.
#[derive(Clone, Default)]
pub struct BuildOptions {
    pub transform: RenderTransform,
    pub dilation: Vector2F,
    pub subpixel_aa_enabled: bool,
}

impl BuildOptions {
    pub(crate) fn prepare(self, bounds: RectF) -> PreparedBuildOptions {
        PreparedBuildOptions {
            transform: self.transform.prepare(bounds),
            dilation: self.dilation,
            subpixel_aa_enabled: self.subpixel_aa_enabled,
        }
    }
}

#[derive(Clone)]
pub enum RenderTransform {
    Transform2D(Transform2F),
    Perspective(Perspective),
}

impl Default for RenderTransform {
    #[inline]
    fn default() -> RenderTransform {
        RenderTransform::Transform2D(Transform2F::default())
    }
}

impl RenderTransform {
    fn prepare(&self, bounds: RectF) -> PreparedRenderTransform {
        let perspective = match self {
            RenderTransform::Transform2D(ref transform) => {
                if transform.is_identity() {
                    return PreparedRenderTransform::None;
                }
                return PreparedRenderTransform::Transform2D(*transform);
            }
            RenderTransform::Perspective(ref perspective) => *perspective,
        };

        let mut points = vec![
            bounds.origin().to_4d(),
            bounds.upper_right().to_4d(),
            bounds.lower_right().to_4d(),
            bounds.lower_left().to_4d(),
        ];
        debug!("-----");
        debug!("bounds={:?} ORIGINAL quad={:?}", bounds, points);
        for point in &mut points {
            *point = perspective.transform * *point;
        }
        debug!("... PERSPECTIVE quad={:?}", points);

        // Compute depth.
        let quad = [
            points[0].to_3d().to_4d(),
            points[1].to_3d().to_4d(),
            points[2].to_3d().to_4d(),
            points[3].to_3d().to_4d(),
        ];
        debug!("... PERSPECTIVE-DIVIDED points = {:?}", quad);

        points = PolygonClipper3D::new(points).clip();
        debug!("... CLIPPED quad={:?}", points);
        for point in &mut points {
            *point = point.to_3d().to_4d()
        }

        let inverse_transform = perspective.transform.inverse();
        let clip_polygon = points.into_iter()
                                 .map(|point| (inverse_transform * point).to_2d())
                                 .collect();
        return PreparedRenderTransform::Perspective {
            perspective,
            clip_polygon,
            quad,
        };
    }
}

pub(crate) struct PreparedBuildOptions {
    pub(crate) transform: PreparedRenderTransform,
    pub(crate) dilation: Vector2F,
    pub(crate) subpixel_aa_enabled: bool,
}

impl PreparedBuildOptions {
    #[inline]
    pub(crate) fn bounding_quad(&self) -> BoundingQuad {
        match self.transform {
            PreparedRenderTransform::Perspective { quad, .. } => quad,
            _ => [Vector4F::default(); 4],
        }
    }
}

pub(crate) type BoundingQuad = [Vector4F; 4];

pub(crate) enum PreparedRenderTransform {
    None,
    Transform2D(Transform2F),
    Perspective {
        perspective: Perspective,
        clip_polygon: Vec<Vector2F>,
        quad: [Vector4F; 4],
    },
}

impl PreparedRenderTransform {
    #[inline]
    pub(crate) fn is_2d(&self) -> bool {
        match *self {
            PreparedRenderTransform::Transform2D(_) => true,
            _ => false,
        }
    }
}
