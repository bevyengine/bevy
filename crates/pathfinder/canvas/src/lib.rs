// pathfinder/canvas/src/lib.rs
//
// Copyright © 2020 The Pathfinder Project Developers.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! A simple API for Pathfinder that mirrors a subset of HTML canvas.

pub use pathfinder_color::{ColorF, ColorU, rgbaf, rgbau, rgbf, rgbu};
pub use pathfinder_color::{color_slice_to_u8_slice, u8_slice_to_color_slice, u8_vec_to_color_vec};
pub use pathfinder_content::fill::FillRule;
pub use pathfinder_content::stroke::LineCap;
pub use pathfinder_content::outline::ArcDirection;
pub use pathfinder_geometry::rect::{RectF, RectI};
pub use pathfinder_geometry::transform2d::Transform2F;
pub use pathfinder_geometry::vector::{IntoVector2F, Vector2F, Vector2I, vec2f, vec2i};

use pathfinder_content::dash::OutlineDash;
use pathfinder_content::effects::{BlendMode, BlurDirection, PatternFilter};
use pathfinder_content::gradient::Gradient;
use pathfinder_content::outline::{Contour, Outline};
use pathfinder_content::pattern::Pattern;
use pathfinder_content::render_target::RenderTargetId;
use pathfinder_content::stroke::{LineJoin as StrokeLineJoin};
use pathfinder_content::stroke::{OutlineStrokeToFill, StrokeStyle};
use pathfinder_geometry::line_segment::LineSegment2F;
use pathfinder_renderer::paint::{Paint, PaintCompositeOp};
use pathfinder_renderer::scene::{ClipPath, ClipPathId, DrawPath, RenderTarget, Scene};
use std::borrow::Cow;
use std::default::Default;
use std::f32::consts::PI;
use std::f32;
use std::fmt::{Debug, Error as FmtError, Formatter};
use std::mem;
use std::sync::Arc;

pub use text::CanvasFontContext;

#[cfg(feature = "pf-text")]
use skribo::FontCollection;
#[cfg(not(feature = "pf-text"))]
use crate::text::FontCollection;

#[cfg(feature = "pf-text")]
pub use text::TextMetrics;

const HAIRLINE_STROKE_WIDTH: f32 = 0.0333;
const DEFAULT_FONT_SIZE: f32 = 10.0;

#[cfg(feature = "pf-text")]
mod text;

// For users who don't want text capability, include a tiny convenience stub.
#[cfg(not(feature = "pf-text"))]
mod text {
    #[derive(Clone)]
    pub struct CanvasFontContext;

    impl CanvasFontContext {
        pub fn from_system_source() -> Self {
            CanvasFontContext
        }
    }

    pub struct FontCollection;
}

#[cfg(test)]
mod tests;

pub struct Canvas {
    scene: Scene,
}

impl Canvas {
    #[inline]
    pub fn new(size: Vector2F) -> Canvas {
        let mut scene = Scene::new();
        scene.set_view_box(RectF::new(Vector2F::zero(), size));
        Canvas::from_scene(scene)
    }

    #[inline]
    pub fn from_scene(scene: Scene) -> Canvas {
        Canvas { scene }
    }

    #[inline]
    pub fn into_scene(self) -> Scene {
        self.scene
    }

    pub fn get_context_2d(self, canvas_font_context: CanvasFontContext)
                          -> CanvasRenderingContext2D {
        #[cfg(feature = "pf-text")]
        let default_font_collection =
            canvas_font_context.0.borrow().default_font_collection.clone();
        #[cfg(not(feature = "pf-text"))]
        let default_font_collection = Arc::new(FontCollection);
        CanvasRenderingContext2D {
            canvas: self,
            current_state: State::default(default_font_collection),
            saved_states: vec![],
            canvas_font_context,
        }
    }

    #[inline]
    pub fn size(&self) -> Vector2I {
        self.scene.view_box().size().ceil().to_i32()
    }
}

pub struct CanvasRenderingContext2D {
    canvas: Canvas,
    current_state: State,
    saved_states: Vec<State>,
    #[allow(dead_code)]
    canvas_font_context: CanvasFontContext,
}

impl CanvasRenderingContext2D {
    // Canvas accessors

    #[inline]
    pub fn canvas(&self) -> &Canvas {
        &self.canvas
    }

    #[inline]
    pub fn into_canvas(self) -> Canvas {
        self.canvas
    }

    // Drawing rectangles

    #[inline]
    pub fn fill_rect(&mut self, rect: RectF) {
        let mut path = Path2D::new();
        path.rect(rect);
        self.fill_path(path, FillRule::Winding);
    }

    #[inline]
    pub fn stroke_rect(&mut self, rect: RectF) {
        let mut path = Path2D::new();
        path.rect(rect);
        self.stroke_path(path);
    }

    pub fn clear_rect(&mut self, rect: RectF) {
        let mut path = Path2D::new();
        path.rect(rect);

        let paint = Paint::transparent_black();
        let paint = self.current_state.resolve_paint(&paint);
        let paint_id = self.canvas.scene.push_paint(&paint);

        let mut outline = path.into_outline();
        outline.transform(&self.current_state.transform);

        let mut path = DrawPath::new(outline, paint_id);
        path.set_blend_mode(BlendMode::Clear);
        self.canvas.scene.push_path(path);
    }

    // Line styles

    #[inline]
    pub fn set_line_width(&mut self, new_line_width: f32) {
        self.current_state.line_width = new_line_width
    }

    #[inline]
    pub fn set_line_cap(&mut self, new_line_cap: LineCap) {
        self.current_state.line_cap = new_line_cap
    }

    #[inline]
    pub fn set_line_join(&mut self, new_line_join: LineJoin) {
        self.current_state.line_join = new_line_join
    }

    #[inline]
    pub fn set_miter_limit(&mut self, new_miter_limit: f32) {
        self.current_state.miter_limit = new_miter_limit
    }

    #[inline]
    pub fn set_line_dash(&mut self, mut new_line_dash: Vec<f32>) {
        // Duplicate and concatenate if an odd number of dashes are present.
        if new_line_dash.len() % 2 == 1 {
            let mut real_line_dash = new_line_dash.clone();
            real_line_dash.extend(new_line_dash.into_iter());
            new_line_dash = real_line_dash;
        }

        self.current_state.line_dash = new_line_dash
    }

    #[inline]
    pub fn set_line_dash_offset(&mut self, new_line_dash_offset: f32) {
        self.current_state.line_dash_offset = new_line_dash_offset
    }

    // Fill and stroke styles

    #[inline]
    pub fn set_fill_style<FS>(&mut self, new_fill_style: FS) where FS: Into<FillStyle> {
        self.current_state.fill_paint = new_fill_style.into().into_paint();
    }

    #[inline]
    pub fn set_stroke_style<FS>(&mut self, new_stroke_style: FS) where FS: Into<FillStyle> {
        self.current_state.stroke_paint = new_stroke_style.into().into_paint();
    }

    // Shadows

    #[inline]
    pub fn shadow_blur(&self) -> f32 {
        self.current_state.shadow_blur
    }

    #[inline]
    pub fn set_shadow_blur(&mut self, new_shadow_blur: f32) {
        self.current_state.shadow_blur = new_shadow_blur;
    }

    #[inline]
    pub fn shadow_color(&self) -> ColorU {
        self.current_state.shadow_color
    }

    #[inline]
    pub fn set_shadow_color(&mut self, new_shadow_color: ColorU) {
        self.current_state.shadow_color = new_shadow_color;
    }

    #[inline]
    pub fn shadow_offset(&self) -> Vector2F {
        self.current_state.shadow_offset
    }

    #[inline]
    pub fn set_shadow_offset(&mut self, new_shadow_offset: Vector2F) {
        self.current_state.shadow_offset = new_shadow_offset;
    }

    // Drawing paths

    #[inline]
    pub fn fill_path(&mut self, path: Path2D, fill_rule: FillRule) {
        self.push_path(path.into_outline(), PathOp::Fill, fill_rule);
    }

    #[inline]
    pub fn stroke_path(&mut self, path: Path2D) {
        let mut stroke_style = self.current_state.resolve_stroke_style();

        // The smaller scale is relevant here, as we multiply by it and want to ensure it is always
        // bigger than `HAIRLINE_STROKE_WIDTH`.
        let transform_scales = self.current_state.transform.extract_scale();
        let transform_scale = f32::min(transform_scales.x(), transform_scales.y());

        // Avoid the division in the normal case of sufficient thickness.
        if stroke_style.line_width * transform_scale < HAIRLINE_STROKE_WIDTH {
            stroke_style.line_width = HAIRLINE_STROKE_WIDTH / transform_scale;
        }

        let mut outline = path.into_outline();
        if !self.current_state.line_dash.is_empty() {
            let mut dash = OutlineDash::new(&outline,
                                            &self.current_state.line_dash,
                                            self.current_state.line_dash_offset);
            dash.dash();
            outline = dash.into_outline();
        }

        let mut stroke_to_fill = OutlineStrokeToFill::new(&outline, stroke_style);
        stroke_to_fill.offset();
        outline = stroke_to_fill.into_outline();

        self.push_path(outline, PathOp::Stroke, FillRule::Winding);
    }

    pub fn clip_path(&mut self, path: Path2D, fill_rule: FillRule) {
        let mut outline = path.into_outline();
        outline.transform(&self.current_state.transform);

        let mut clip_path = ClipPath::new(outline);
        clip_path.set_fill_rule(fill_rule);
        let clip_path_id = self.canvas.scene.push_clip_path(clip_path);

        self.current_state.clip_path = Some(clip_path_id);
    }

    fn push_path(&mut self, mut outline: Outline, path_op: PathOp, fill_rule: FillRule) {
        let paint = self.current_state.resolve_paint(match path_op {
            PathOp::Fill => &self.current_state.fill_paint,
            PathOp::Stroke => &self.current_state.stroke_paint,
        });
        let paint_id = self.canvas.scene.push_paint(&paint);

        let transform = self.current_state.transform;
        let clip_path = self.current_state.clip_path;
        let blend_mode = self.current_state.global_composite_operation.to_blend_mode();

        outline.transform(&transform);

        if !self.current_state.shadow_color.is_fully_transparent() {
            let mut outline = outline.clone();
            outline.transform(&Transform2F::from_translation(self.current_state.shadow_offset));

            let shadow_blur_info =
                push_shadow_blur_render_targets_if_needed(&mut self.canvas.scene,
                                                          &self.current_state,
                                                          outline.bounds());

            if let Some(ref shadow_blur_info) = shadow_blur_info {
                outline.transform(&Transform2F::from_translation(-shadow_blur_info.bounds
                                                                                  .origin()
                                                                                  .to_f32()));
            }

            // Per spec the shadow must respect the alpha of the shadowed path, but otherwise have
            // the color of the shadow paint.
            let mut shadow_paint = (*paint).clone();
            let shadow_base_alpha = shadow_paint.base_color().a;
            let mut shadow_color = self.current_state.shadow_color.to_f32();
            shadow_color.set_a(shadow_color.a() * shadow_base_alpha as f32 / 255.0);
            shadow_paint.set_base_color(shadow_color.to_u8());
            if let &mut Some(ref mut shadow_paint_overlay) = shadow_paint.overlay_mut() {
                shadow_paint_overlay.set_composite_op(PaintCompositeOp::DestIn);
            }
            let shadow_paint_id = self.canvas.scene.push_paint(&shadow_paint);

            let mut path = DrawPath::new(outline, shadow_paint_id);
            if shadow_blur_info.is_none() {
                path.set_clip_path(clip_path);
            }
            path.set_fill_rule(fill_rule);
            path.set_blend_mode(blend_mode);
            self.canvas.scene.push_path(path);

            composite_shadow_blur_render_targets_if_needed(&mut self.canvas.scene,
                                                           shadow_blur_info,
                                                           clip_path);
        }

        let mut path = DrawPath::new(outline, paint_id);
        path.set_clip_path(clip_path);
        path.set_fill_rule(fill_rule);
        path.set_blend_mode(blend_mode);
        self.canvas.scene.push_path(path);

        fn push_shadow_blur_render_targets_if_needed(scene: &mut Scene,
                                                     current_state: &State,
                                                     outline_bounds: RectF)
                                                    -> Option<ShadowBlurRenderTargetInfo> {
            if current_state.shadow_blur == 0.0 {
                return None;
            }

            let sigma = current_state.shadow_blur * 0.5;
            let bounds = outline_bounds.dilate(sigma * 3.0).round_out().to_i32();

            let render_target_y = RenderTarget::new(bounds.size(), String::new());
            let render_target_id_y = scene.push_render_target(render_target_y);
            let render_target_x = RenderTarget::new(bounds.size(), String::new());
            let render_target_id_x = scene.push_render_target(render_target_x);

            Some(ShadowBlurRenderTargetInfo {
                id_x: render_target_id_x,
                id_y: render_target_id_y,
                bounds,
                sigma,
            })
        }

        fn composite_shadow_blur_render_targets_if_needed(scene: &mut Scene,
                                                          info: Option<ShadowBlurRenderTargetInfo>,
                                                          clip_path: Option<ClipPathId>) {
            let info = match info {
                None => return,
                Some(info) => info,
            };

            let mut paint_x = Pattern::from_render_target(info.id_x, info.bounds.size());
            let mut paint_y = Pattern::from_render_target(info.id_y, info.bounds.size());
            paint_y.apply_transform(Transform2F::from_translation(info.bounds.origin().to_f32()));

            let sigma = info.sigma;
            paint_x.set_filter(Some(PatternFilter::Blur { direction: BlurDirection::X, sigma }));
            paint_y.set_filter(Some(PatternFilter::Blur { direction: BlurDirection::Y, sigma }));

            let paint_id_x = scene.push_paint(&Paint::from_pattern(paint_x));
            let paint_id_y = scene.push_paint(&Paint::from_pattern(paint_y));

            // TODO(pcwalton): Apply clip as necessary.
            let outline_x = Outline::from_rect(RectF::new(vec2f(0.0, 0.0),
                                                        info.bounds.size().to_f32()));
            let path_x = DrawPath::new(outline_x, paint_id_x);
            let outline_y = Outline::from_rect(info.bounds.to_f32());
            let mut path_y = DrawPath::new(outline_y, paint_id_y);
            path_y.set_clip_path(clip_path);

            scene.pop_render_target();
            scene.push_path(path_x);
            scene.pop_render_target();
            scene.push_path(path_y);
        }

    }

    // Transformations

    #[inline]
    pub fn rotate(&mut self, angle: f32) {
        self.current_state.transform *= Transform2F::from_rotation(angle)
    }

    #[inline]
    pub fn scale<S>(&mut self, scale: S) where S: IntoVector2F {
        self.current_state.transform *= Transform2F::from_scale(scale)
    }

    #[inline]
    pub fn translate(&mut self, offset: Vector2F) {
        self.current_state.transform *= Transform2F::from_translation(offset)
    }

    #[inline]
    pub fn transform(&self) -> Transform2F {
        self.current_state.transform
    }

    #[inline]
    pub fn set_transform(&mut self, new_transform: &Transform2F) {
        self.current_state.transform = *new_transform;
    }

    #[inline]
    pub fn reset_transform(&mut self) {
        self.current_state.transform = Transform2F::default();
    }

    // Compositing

    #[inline]
    pub fn global_alpha(&self) -> f32 {
        self.current_state.global_alpha
    }

    #[inline]
    pub fn set_global_alpha(&mut self, new_global_alpha: f32) {
        self.current_state.global_alpha = new_global_alpha;
    }

    #[inline]
    pub fn global_composite_operation(&self) -> CompositeOperation {
        self.current_state.global_composite_operation
    }

    #[inline]
    pub fn set_global_composite_operation(&mut self, new_composite_operation: CompositeOperation) {
        self.current_state.global_composite_operation = new_composite_operation;
    }

    // Drawing images

    #[inline]
    pub fn draw_image<I, L>(&mut self, image: I, dest_location: L)
                            where I: CanvasImageSource, L: CanvasImageDestLocation {
        let pattern = image.to_pattern(self, Transform2F::default());
        let src_rect = RectF::new(vec2f(0.0, 0.0), pattern.size().to_f32());
        self.draw_subimage(pattern, src_rect, dest_location)
    }

    pub fn draw_subimage<I, L>(&mut self, image: I, src_location: RectF, dest_location: L)
                               where I: CanvasImageSource, L: CanvasImageDestLocation {
        let dest_size = dest_location.size().unwrap_or(src_location.size());
        let scale = dest_size / src_location.size();
        let offset = dest_location.origin() - src_location.origin();
        let transform = Transform2F::from_scale(scale).translate(offset);

        let pattern = image.to_pattern(self, transform);
        let old_fill_paint = self.current_state.fill_paint.clone();
        self.set_fill_style(pattern);
        self.fill_rect(RectF::new(dest_location.origin(), dest_size));
        self.current_state.fill_paint = old_fill_paint;
    }

    // Image smoothing

    #[inline]
    pub fn image_smoothing_enabled(&self) -> bool {
        self.current_state.image_smoothing_enabled
    }

    #[inline]
    pub fn set_image_smoothing_enabled(&mut self, enabled: bool) {
        self.current_state.image_smoothing_enabled = enabled
    }

    #[inline]
    pub fn image_smoothing_quality(&self) -> ImageSmoothingQuality {
        self.current_state.image_smoothing_quality
    }

    #[inline]
    pub fn set_image_smoothing_quality(&mut self, new_quality: ImageSmoothingQuality) {
        self.current_state.image_smoothing_quality = new_quality
    }

    // The canvas state

    #[inline]
    pub fn save(&mut self) {
        self.saved_states.push(self.current_state.clone());
    }

    #[inline]
    pub fn restore(&mut self) {
        if let Some(state) = self.saved_states.pop() {
            self.current_state = state;
        }
    }

    // Extensions

    pub fn create_pattern_from_canvas(&mut self, canvas: Canvas, transform: Transform2F)
                                      -> Pattern {
        let subscene_size = canvas.size();
        let subscene = canvas.into_scene();
        let render_target = RenderTarget::new(subscene_size, String::new());
        let render_target_id = self.canvas.scene.push_render_target(render_target);
        self.canvas.scene.append_scene(subscene);
        self.canvas.scene.pop_render_target();

        let mut pattern = Pattern::from_render_target(render_target_id, subscene_size);
        pattern.apply_transform(transform);
        pattern
    }
}

#[derive(Clone)]
struct State {
    transform: Transform2F,
    font_collection: Arc<FontCollection>,
    font_size: f32,
    line_width: f32,
    line_cap: LineCap,
    line_join: LineJoin,
    miter_limit: f32,
    line_dash: Vec<f32>,
    line_dash_offset: f32,
    fill_paint: Paint,
    stroke_paint: Paint,
    shadow_color: ColorU,
    shadow_blur: f32,
    shadow_offset: Vector2F,
    text_align: TextAlign,
    text_baseline: TextBaseline,
    image_smoothing_enabled: bool,
    image_smoothing_quality: ImageSmoothingQuality,
    global_alpha: f32,
    global_composite_operation: CompositeOperation,
    clip_path: Option<ClipPathId>,
}

impl State {
    fn default(default_font_collection: Arc<FontCollection>) -> State {
        State {
            transform: Transform2F::default(),
            font_collection: default_font_collection,
            font_size: DEFAULT_FONT_SIZE,
            line_width: 1.0,
            line_cap: LineCap::Butt,
            line_join: LineJoin::Miter,
            miter_limit: 10.0,
            line_dash: vec![],
            line_dash_offset: 0.0,
            fill_paint: Paint::black(),
            stroke_paint: Paint::black(),
            shadow_color: ColorU::transparent_black(),
            shadow_blur: 0.0,
            shadow_offset: Vector2F::zero(),
            text_align: TextAlign::Left,
            text_baseline: TextBaseline::Alphabetic,
            image_smoothing_enabled: true,
            image_smoothing_quality: ImageSmoothingQuality::Low,
            global_alpha: 1.0,
            global_composite_operation: CompositeOperation::SourceOver,
            clip_path: None,
        }
    }

    fn resolve_paint<'a>(&self, paint: &'a Paint) -> Cow<'a, Paint> {
        let mut must_copy = !self.transform.is_identity() || self.global_alpha < 1.0;
        if !must_copy {
            if let Some(ref pattern) = paint.pattern() {
                must_copy = self.image_smoothing_enabled != pattern.smoothing_enabled()
            }
        }

        if !must_copy {
            return Cow::Borrowed(paint);
        }

        let mut paint = (*paint).clone();
        paint.apply_transform(&self.transform);

        let mut base_color = paint.base_color().to_f32();
        base_color.set_a(base_color.a() * self.global_alpha);
        paint.set_base_color(base_color.to_u8());

        if let Some(ref mut pattern) = paint.pattern_mut() {
            pattern.set_smoothing_enabled(self.image_smoothing_enabled);
        }
        Cow::Owned(paint)
    }

    fn resolve_stroke_style(&self) -> StrokeStyle {
        StrokeStyle {
            line_width: self.line_width,
            line_cap: self.line_cap,
            line_join: match self.line_join {
                LineJoin::Miter => StrokeLineJoin::Miter(self.miter_limit),
                LineJoin::Bevel => StrokeLineJoin::Bevel,
                LineJoin::Round => StrokeLineJoin::Round,
            },
        }
    }
}

#[derive(Clone)]
pub struct Path2D {
    outline: Outline,
    current_contour: Contour,
}

impl Path2D {
    #[inline]
    pub fn new() -> Path2D {
        Path2D { outline: Outline::new(), current_contour: Contour::new() }
    }

    #[inline]
    pub fn close_path(&mut self) {
        self.current_contour.close();
    }

    #[inline]
    pub fn move_to(&mut self, to: Vector2F) {
        // TODO(pcwalton): Cull degenerate contours.
        self.flush_current_contour();
        self.current_contour.push_endpoint(to);
    }

    #[inline]
    pub fn line_to(&mut self, to: Vector2F) {
        self.current_contour.push_endpoint(to);
    }

    #[inline]
    pub fn quadratic_curve_to(&mut self, ctrl: Vector2F, to: Vector2F) {
        self.current_contour.push_quadratic(ctrl, to);
    }

    #[inline]
    pub fn bezier_curve_to(&mut self, ctrl0: Vector2F, ctrl1: Vector2F, to: Vector2F) {
        self.current_contour.push_cubic(ctrl0, ctrl1, to);
    }

    #[inline]
    pub fn arc(&mut self,
               center: Vector2F,
               radius: f32,
               start_angle: f32,
               end_angle: f32,
               direction: ArcDirection) {
        let transform = Transform2F::from_scale(radius).translate(center);
        self.current_contour.push_arc(&transform, start_angle, end_angle, direction);
    }

    #[inline]
    pub fn arc_to(&mut self, ctrl: Vector2F, to: Vector2F, radius: f32) {
        // FIXME(pcwalton): What should we do if there's no initial point?
        let from = self.current_contour.last_position().unwrap_or_default();
        let (v0, v1) = (from - ctrl, to - ctrl);
        let (vu0, vu1) = (v0.normalize(), v1.normalize());
        let hypot = radius / f32::sqrt(0.5 * (1.0 - vu0.dot(vu1)));
        let bisector = vu0 + vu1;
        let center = ctrl + bisector * (hypot / bisector.length());

        let transform = Transform2F::from_scale(radius).translate(center);
        let chord = LineSegment2F::new(vu0.yx() * vec2f(-1.0,  1.0), vu1.yx() * vec2f( 1.0, -1.0));

        // FIXME(pcwalton): Is clockwise direction correct?
        self.current_contour.push_arc_from_unit_chord(&transform, chord, ArcDirection::CW);
    }

    pub fn rect(&mut self, rect: RectF) {
        self.flush_current_contour();
        self.current_contour.push_endpoint(rect.origin());
        self.current_contour.push_endpoint(rect.upper_right());
        self.current_contour.push_endpoint(rect.lower_right());
        self.current_contour.push_endpoint(rect.lower_left());
        self.current_contour.close();
    }

    pub fn ellipse<A>(&mut self,
                      center: Vector2F,
                      axes: A,
                      rotation: f32,
                      start_angle: f32,
                      end_angle: f32)
                      where A: IntoVector2F {
        self.flush_current_contour();

        let transform = Transform2F::from_scale(axes).rotate(rotation).translate(center);
        self.current_contour.push_arc(&transform, start_angle, end_angle, ArcDirection::CW);

        if end_angle - start_angle >= 2.0 * PI {
            self.current_contour.close();
        }
    }

    // https://html.spec.whatwg.org/multipage/canvas.html#dom-path2d-addpath
    pub fn add_path(&mut self, mut path: Path2D, transform: &Transform2F) {
        self.flush_current_contour();
        path.flush_current_contour();
        path.outline.transform(transform);
        let last_contour = path.outline.pop_contour();
        for contour in path.outline.into_contours() {
            self.outline.push_contour(contour);
        }
        self.current_contour = last_contour.unwrap_or_else(Contour::new);
    }

    pub fn into_outline(mut self) -> Outline {
        self.flush_current_contour();
        self.outline
    }

    fn flush_current_contour(&mut self) {
        if !self.current_contour.is_empty() {
            self.outline.push_contour(mem::replace(&mut self.current_contour, Contour::new()));
        }
    }
}

#[derive(Clone)]
pub enum FillStyle {
    Color(ColorU),
    Gradient(Gradient),
    Pattern(Pattern),
}

impl FillStyle {
    fn into_paint(self) -> Paint {
        match self {
            FillStyle::Color(color) => Paint::from_color(color),
            FillStyle::Gradient(gradient) => Paint::from_gradient(gradient),
            FillStyle::Pattern(pattern) => Paint::from_pattern(pattern),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum TextAlign {
    Left,
    Right,
    Center,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum TextBaseline {
    Alphabetic,
    Top,
    Hanging,
    Middle,
    Ideographic,
    Bottom,
}

// We duplicate `pathfinder_content::stroke::LineJoin` here because the HTML canvas API treats the
// miter limit as part of the canvas state, while the native Pathfinder API treats the miter limit
// as part of the line join. Pathfinder's choice is more logical, because the miter limit is
// specific to miter joins. In this API, however, for compatibility we go with the HTML canvas
// semantics.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum LineJoin {
    Miter,
    Bevel,
    Round,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum CompositeOperation {
    SourceOver,
    SourceIn,
    SourceOut,
    SourceAtop,
    DestinationOver,
    DestinationIn,
    DestinationOut,
    DestinationAtop,
    Lighter,
    Copy,
    Xor,
    Multiply,
    Screen,
    Overlay,
    Darken,
    Lighten,
    ColorDodge,
    ColorBurn,
    HardLight,
    SoftLight,
    Difference,
    Exclusion,
    Hue,
    Saturation,
    Color,
    Luminosity,
}

impl CompositeOperation {
    fn to_blend_mode(self) -> BlendMode {
        match self {
            CompositeOperation::Copy => BlendMode::Copy,
            CompositeOperation::SourceAtop => BlendMode::SrcAtop,
            CompositeOperation::DestinationOver => BlendMode::DestOver,
            CompositeOperation::DestinationOut => BlendMode::DestOut,
            CompositeOperation::Xor => BlendMode::Xor,
            CompositeOperation::Lighter => BlendMode::Lighter,
            CompositeOperation::Multiply => BlendMode::Multiply,
            CompositeOperation::Screen => BlendMode::Screen,
            CompositeOperation::Overlay => BlendMode::Overlay,
            CompositeOperation::Darken => BlendMode::Darken,
            CompositeOperation::Lighten => BlendMode::Lighten,
            CompositeOperation::ColorDodge => BlendMode::ColorDodge,
            CompositeOperation::ColorBurn => BlendMode::ColorBurn,
            CompositeOperation::HardLight => BlendMode::HardLight,
            CompositeOperation::SoftLight => BlendMode::SoftLight,
            CompositeOperation::Difference => BlendMode::Difference,
            CompositeOperation::Exclusion => BlendMode::Exclusion,
            CompositeOperation::Hue => BlendMode::Hue,
            CompositeOperation::Saturation => BlendMode::Saturation,
            CompositeOperation::Color => BlendMode::Color,
            CompositeOperation::Luminosity => BlendMode::Luminosity,
            CompositeOperation::SourceOver => BlendMode::SrcOver,
            CompositeOperation::SourceIn => BlendMode::SrcIn,
            CompositeOperation::SourceOut => BlendMode::SrcOut,
            CompositeOperation::DestinationIn => BlendMode::DestIn,
            CompositeOperation::DestinationAtop => BlendMode::DestAtop,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ImageSmoothingQuality {
    Low,
    Medium,
    High,
}

pub trait CanvasImageSource {
    fn to_pattern(self, dest_context: &mut CanvasRenderingContext2D, transform: Transform2F)
                  -> Pattern;
}

pub trait CanvasImageDestLocation {
    fn origin(&self) -> Vector2F;
    fn size(&self) -> Option<Vector2F>;
}

impl CanvasImageSource for Pattern {
    #[inline]
    fn to_pattern(mut self, _: &mut CanvasRenderingContext2D, transform: Transform2F) -> Pattern {
        self.apply_transform(transform);
        self
    }
}

impl CanvasImageSource for Canvas {
    #[inline]
    fn to_pattern(self, dest_context: &mut CanvasRenderingContext2D, transform: Transform2F)
                  -> Pattern {
        dest_context.create_pattern_from_canvas(self, transform)
    }
}

impl CanvasImageDestLocation for RectF {
    #[inline]
    fn origin(&self) -> Vector2F {
        RectF::origin(*self)
    }
    #[inline]
    fn size(&self) -> Option<Vector2F> {
        Some(RectF::size(*self))
    }
}

impl CanvasImageDestLocation for Vector2F {
    #[inline]
    fn origin(&self) -> Vector2F {
        *self
    }
    #[inline]
    fn size(&self) -> Option<Vector2F> {
        None
    }
}

impl Debug for Path2D {
    fn fmt(&self, formatter: &mut Formatter) -> Result<(), FmtError> {
        self.clone().into_outline().fmt(formatter)
    }
}

impl From<ColorU> for FillStyle {
    #[inline]
    fn from(color: ColorU) -> FillStyle {
        FillStyle::Color(color)
    }
}

impl From<Gradient> for FillStyle {
    #[inline]
    fn from(gradient: Gradient) -> FillStyle {
        FillStyle::Gradient(gradient)
    }
}

impl From<Pattern> for FillStyle {
    #[inline]
    fn from(pattern: Pattern) -> FillStyle {
        FillStyle::Pattern(pattern)
    }
}

struct ShadowBlurRenderTargetInfo {
    id_x: RenderTargetId,
    id_y: RenderTargetId,
    bounds: RectI,
    sigma: f32,
}

enum PathOp {
    Fill,
    Stroke,
}
