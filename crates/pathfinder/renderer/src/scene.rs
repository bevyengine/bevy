// pathfinder/renderer/src/scene.rs
//
// Copyright © 2019 The Pathfinder Project Developers.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! A set of paths to be rendered.

use crate::builder::SceneBuilder;
use crate::concurrent::executor::Executor;
use crate::options::{BuildOptions, PreparedBuildOptions};
use crate::options::{PreparedRenderTransform, RenderCommandListener};
use crate::paint::{MergedPaletteInfo, Paint, PaintId, PaintInfo, Palette};
use pathfinder_content::effects::BlendMode;
use pathfinder_content::fill::FillRule;
use pathfinder_content::outline::Outline;
use pathfinder_content::render_target::RenderTargetId;
use pathfinder_geometry::rect::RectF;
use pathfinder_geometry::transform2d::Transform2F;
use pathfinder_geometry::vector::{Vector2I, vec2f};
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT_SCENE_ID: AtomicUsize = AtomicUsize::new(0);

#[derive(Clone)]
pub struct Scene {
    pub(crate) display_list: Vec<DisplayItem>,
    pub(crate) paths: Vec<DrawPath>,
    pub(crate) clip_paths: Vec<ClipPath>,
    palette: Palette,
    bounds: RectF,
    view_box: RectF,
    id: SceneId,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SceneId(pub u32);

impl Scene {
    #[inline]
    pub fn new() -> Scene {
        let scene_id = SceneId(NEXT_SCENE_ID.fetch_add(1, Ordering::Relaxed) as u32);
        Scene {
            display_list: vec![],
            paths: vec![],
            clip_paths: vec![],
            palette: Palette::new(scene_id),
            bounds: RectF::default(),
            view_box: RectF::default(),
            id: scene_id,
        }
    }

    pub fn push_path(&mut self, path: DrawPath) {
        let path_index = self.paths.len() as u32;
        self.paths.push(path);
        self.push_path_with_index(path_index);
    }

    fn push_path_with_index(&mut self, path_index: u32) {
        self.bounds = self.bounds.union_rect(self.paths[path_index as usize].outline.bounds());

        if let Some(DisplayItem::DrawPaths {
            start_index: _,
            ref mut end_index
        }) = self.display_list.last_mut() {
            *end_index = path_index + 1;
        } else {
            self.display_list.push(DisplayItem::DrawPaths {
                start_index: path_index,
                end_index: path_index + 1,
            });
        }
    }

    pub fn push_clip_path(&mut self, clip_path: ClipPath) -> ClipPathId {
        self.bounds = self.bounds.union_rect(clip_path.outline.bounds());
        let clip_path_id = ClipPathId(self.clip_paths.len() as u32);
        self.clip_paths.push(clip_path);
        clip_path_id
    }

    pub fn push_render_target(&mut self, render_target: RenderTarget) -> RenderTargetId {
        let render_target_id = self.palette.push_render_target(render_target);
        self.display_list.push(DisplayItem::PushRenderTarget(render_target_id));
        render_target_id
    }

    pub fn pop_render_target(&mut self) {
        self.display_list.push(DisplayItem::PopRenderTarget);
    }

    pub fn append_scene(&mut self, scene: Scene) {
        let MergedPaletteInfo {
            render_target_mapping,
            paint_mapping,
        } = self.palette.append_palette(scene.palette);

        // Merge clip paths.
        let mut clip_path_mapping = Vec::with_capacity(scene.clip_paths.len());
        for clip_path in scene.clip_paths {
            clip_path_mapping.push(self.clip_paths.len());
            self.clip_paths.push(clip_path);
        }

        // Merge draw paths.
        let mut draw_path_mapping = Vec::with_capacity(scene.paths.len());
        for draw_path in scene.paths {
            draw_path_mapping.push(self.paths.len() as u32);
            self.paths.push(DrawPath {
                outline: draw_path.outline,
                paint: paint_mapping[&draw_path.paint],
                clip_path: draw_path.clip_path.map(|clip_path_id| {
                    ClipPathId(clip_path_mapping[clip_path_id.0 as usize] as u32)
                }),
                fill_rule: draw_path.fill_rule,
                blend_mode: draw_path.blend_mode,
                name: draw_path.name,
            });
        }

        // Merge display items.
        for display_item in scene.display_list {
            match display_item {
                DisplayItem::PushRenderTarget(old_render_target_id) => {
                    let new_render_target_id = render_target_mapping[&old_render_target_id];
                    self.display_list.push(DisplayItem::PushRenderTarget(new_render_target_id));
                }
                DisplayItem::PopRenderTarget => {
                    self.display_list.push(DisplayItem::PopRenderTarget);
                }
                DisplayItem::DrawPaths {
                    start_index: old_start_path_index,
                    end_index: old_end_path_index,
                } => {
                    for old_path_index in old_start_path_index..old_end_path_index {
                        self.push_path_with_index(draw_path_mapping[old_path_index as usize])
                    }
                }
            }
        }
    }

    #[inline]
    pub fn build_paint_info(&mut self, render_transform: Transform2F) -> PaintInfo {
        self.palette.build_paint_info(render_transform)
    }

    #[allow(clippy::trivially_copy_pass_by_ref)]
    pub fn push_paint(&mut self, paint: &Paint) -> PaintId {
        self.palette.push_paint(paint)
    }

    #[inline]
    pub fn path_count(&self) -> usize {
        self.paths.len()
    }

    #[inline]
    pub fn bounds(&self) -> RectF {
        self.bounds
    }

    #[inline]
    pub fn set_bounds(&mut self, new_bounds: RectF) {
        self.bounds = new_bounds;
    }

    #[inline]
    pub fn view_box(&self) -> RectF {
        self.view_box
    }

    #[inline]
    pub fn set_view_box(&mut self, new_view_box: RectF) {
        self.view_box = new_view_box;
    }

    pub(crate) fn apply_render_options(
        &self,
        original_outline: &Outline,
        options: &PreparedBuildOptions,
    ) -> Outline {
        let effective_view_box = self.effective_view_box(options);

        let mut outline;
        match options.transform {
            PreparedRenderTransform::Perspective {
                ref perspective,
                ref clip_polygon,
                ..
            } => {
                if original_outline.is_outside_polygon(clip_polygon) {
                    outline = Outline::new();
                } else {
                    outline = (*original_outline).clone();
                    outline.close_all_contours();
                    outline.clip_against_polygon(clip_polygon);
                    outline.apply_perspective(perspective);

                    // TODO(pcwalton): Support subpixel AA in 3D.
                }
            }
            _ => {
                // TODO(pcwalton): Short circuit.
                outline = (*original_outline).clone();
                outline.close_all_contours();
                if options.transform.is_2d() || options.subpixel_aa_enabled {
                    let mut transform = match options.transform {
                        PreparedRenderTransform::Transform2D(transform) => transform,
                        PreparedRenderTransform::None => Transform2F::default(),
                        PreparedRenderTransform::Perspective { .. } => unreachable!(),
                    };
                    if options.subpixel_aa_enabled {
                        transform *= Transform2F::from_scale(vec2f(3.0, 1.0))
                    }
                    outline.transform(&transform);
                }
                outline.clip_against_rect(effective_view_box);
            }
        }

        if !options.dilation.is_zero() {
            outline.dilate(options.dilation);
        }

        // TODO(pcwalton): Fold this into previous passes to avoid unnecessary clones during
        // monotonic conversion.
        outline.prepare_for_tiling(self.effective_view_box(options));
        outline
    }

    #[inline]
    pub(crate) fn effective_view_box(&self, render_options: &PreparedBuildOptions) -> RectF {
        if render_options.subpixel_aa_enabled {
            self.view_box * vec2f(3.0, 1.0)
        } else {
            self.view_box
        }
    }

    #[inline]
    pub fn build<E>(&mut self,
                    options: BuildOptions,
                    listener: Box<dyn RenderCommandListener>,
                    executor: &E)
                    where E: Executor {
        let prepared_options = options.prepare(self.bounds);
        SceneBuilder::new(self, &prepared_options, listener).build(executor)
    }

    pub fn paths<'a>(&'a self) -> PathIter {
        PathIter {
            scene: self,
            pos: 0
        }
    }
}

pub struct PathIter<'a> {
    scene: &'a Scene,
    pos: usize
}

impl<'a> Iterator for PathIter<'a> {
    type Item = (&'a Paint, &'a Outline, &'a str);
    fn next(&mut self) -> Option<Self::Item> {
        let item = self.scene.paths.get(self.pos).map(|path_object| {
            (
                self.scene.palette.paints.get(path_object.paint.0 as usize).unwrap(),
                &path_object.outline,
                &*path_object.name
            )
        });
        self.pos += 1;
        item
    }
}

#[derive(Clone, Debug)]
pub struct DrawPath {
    outline: Outline,
    paint: PaintId,
    clip_path: Option<ClipPathId>,
    fill_rule: FillRule,
    blend_mode: BlendMode,
    name: String,
}

#[derive(Clone, Debug)]
pub struct ClipPath {
    outline: Outline,
    fill_rule: FillRule,
    name: String,
}

#[derive(Clone, Copy, Debug)]
pub struct ClipPathId(pub u32);

#[derive(Clone, Debug)]
pub struct RenderTarget {
    size: Vector2I,
    name: String,
}

/// Drawing commands.
#[derive(Clone, Debug)]
pub enum DisplayItem {
    /// Draws paths to the render target on top of the stack.
    DrawPaths { start_index: u32, end_index: u32 },

    /// Pushes a render target onto the top of the stack.
    PushRenderTarget(RenderTargetId),

    /// Pops a render target from the stack.
    PopRenderTarget,
}

impl DrawPath {
    #[inline]
    pub fn new(outline: Outline, paint: PaintId) -> DrawPath {
        DrawPath {
            outline,
            paint,
            clip_path: None,
            fill_rule: FillRule::Winding,
            blend_mode: BlendMode::SrcOver,
            name: String::new(),
        }
    }

    #[inline]
    pub fn outline(&self) -> &Outline {
        &self.outline
    }

    #[inline]
    pub(crate) fn clip_path(&self) -> Option<ClipPathId> {
        self.clip_path
    }

    #[inline]
    pub fn set_clip_path(&mut self, new_clip_path: Option<ClipPathId>) {
        self.clip_path = new_clip_path
    }

    #[inline]
    pub(crate) fn paint(&self) -> PaintId {
        self.paint
    }

    #[inline]
    pub(crate) fn fill_rule(&self) -> FillRule {
        self.fill_rule
    }

    #[inline]
    pub fn set_fill_rule(&mut self, new_fill_rule: FillRule) {
        self.fill_rule = new_fill_rule
    }

    #[inline]
    pub(crate) fn blend_mode(&self) -> BlendMode {
        self.blend_mode
    }

    #[inline]
    pub fn set_blend_mode(&mut self, new_blend_mode: BlendMode) {
        self.blend_mode = new_blend_mode
    }

    #[inline]
    pub fn set_name(&mut self, new_name: String) {
        self.name = new_name
    }
}

impl ClipPath {
    #[inline]
    pub fn new(outline: Outline) -> ClipPath {
        ClipPath { outline, fill_rule: FillRule::Winding, name: String::new() }
    }

    #[inline]
    pub fn outline(&self) -> &Outline {
        &self.outline
    }

    #[inline]
    pub(crate) fn fill_rule(&self) -> FillRule {
        self.fill_rule
    }

    #[inline]
    pub fn set_fill_rule(&mut self, new_fill_rule: FillRule) {
        self.fill_rule = new_fill_rule
    }

    #[inline]
    pub fn set_name(&mut self, new_name: String) {
        self.name = new_name
    }
}

impl RenderTarget {
    #[inline]
    pub fn new(size: Vector2I, name: String) -> RenderTarget {
        RenderTarget { size, name }
    }

    #[inline]
    pub fn size(&self) -> Vector2I {
        self.size
    }
}
