// pathfinder/canvas/src/text.rs
//
// Copyright © 2019 The Pathfinder Project Developers.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use crate::{CanvasRenderingContext2D, State, TextAlign, TextBaseline};
use font_kit::canvas::RasterizationOptions;
use font_kit::family_name::FamilyName;
use font_kit::handle::Handle;
use font_kit::hinting::HintingOptions;
use font_kit::loaders::default::Font;
use font_kit::properties::Properties;
use font_kit::source::{Source, SystemSource};
use font_kit::sources::mem::MemSource;
use pathfinder_geometry::transform2d::Transform2F;
use pathfinder_geometry::util;
use pathfinder_geometry::vector::{Vector2F, vec2f};
use pathfinder_renderer::paint::PaintId;
use pathfinder_text::{FontContext, FontRenderOptions, TextRenderMode};
use skribo::{FontCollection, FontFamily, FontRef, Layout, TextStyle};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

impl CanvasRenderingContext2D {
    pub fn fill_text(&mut self, string: &str, position: Vector2F) {
        let paint = self.current_state.resolve_paint(&self.current_state.fill_paint);
        let paint_id = self.canvas.scene.push_paint(&paint);
        self.fill_or_stroke_text(string, position, paint_id, TextRenderMode::Fill);
    }

    pub fn stroke_text(&mut self, string: &str, position: Vector2F) {
        let paint = self.current_state.resolve_paint(&self.current_state.stroke_paint);
        let paint_id = self.canvas.scene.push_paint(&paint);
        let render_mode = TextRenderMode::Stroke(self.current_state.resolve_stroke_style());
        self.fill_or_stroke_text(string, position, paint_id, render_mode);
    }

    pub fn measure_text(&self, string: &str) -> TextMetrics {
        let mut metrics = self.layout_text(string).metrics();
        metrics.make_origin_relative(&self.current_state);
        metrics
    }

    pub fn fill_layout(&mut self, layout: &Layout, transform: Transform2F) {
        let paint_id = self.canvas.scene.push_paint(&self.current_state.fill_paint);

        let clip_path = self.current_state.clip_path;
        let blend_mode = self.current_state.global_composite_operation.to_blend_mode();

        // TODO(pcwalton): Report errors.
        drop(self.canvas_font_context
                 .0
                 .borrow_mut()
                 .font_context
                 .push_layout(&mut self.canvas.scene,
                              &layout,
                              &TextStyle { size: self.current_state.font_size },
                              &FontRenderOptions {
                                  transform: transform * self.current_state.transform,
                                  render_mode: TextRenderMode::Fill,
                                  hinting_options: HintingOptions::None,
                                  clip_path,
                                  blend_mode,
                                  paint_id,
                              }));
    }

    fn fill_or_stroke_text(&mut self,
                           string: &str,
                           mut position: Vector2F,
                           paint_id: PaintId,
                           render_mode: TextRenderMode) {
        let layout = self.layout_text(string);

        let clip_path = self.current_state.clip_path;
        let blend_mode = self.current_state.global_composite_operation.to_blend_mode();

        position += layout.metrics().text_origin(&self.current_state);
        let transform = self.current_state.transform * Transform2F::from_translation(position);

        // TODO(pcwalton): Report errors.
        drop(self.canvas_font_context
                 .0
                 .borrow_mut()
                 .font_context
                 .push_layout(&mut self.canvas.scene,
                              &layout,
                              &TextStyle { size: self.current_state.font_size },
                              &FontRenderOptions {
                                  transform,
                                  render_mode,
                                  hinting_options: HintingOptions::None,
                                  clip_path,
                                  blend_mode,
                                  paint_id,
                              }));
    }

    fn layout_text(&self, string: &str) -> Layout {
        skribo::layout(&TextStyle { size: self.current_state.font_size },
                       &self.current_state.font_collection,
                       string)
    }

    // Text styles

    #[inline]
    pub fn font(&self) -> Arc<FontCollection> {
        self.current_state.font_collection.clone()
    }

    #[inline]
    pub fn set_font<FC>(&mut self, font_collection: FC) where FC: IntoFontCollection {
        let font_collection = font_collection.into_font_collection(&self.canvas_font_context);
        self.current_state.font_collection = font_collection; 
    }

    #[inline]
    pub fn font_size(&self) -> f32 {
        self.current_state.font_size
    }

    #[inline]
    pub fn set_font_size(&mut self, new_font_size: f32) {
        self.current_state.font_size = new_font_size;
    }

    #[inline]
    pub fn text_align(&self) -> TextAlign {
        self.current_state.text_align
    }

    #[inline]
    pub fn set_text_align(&mut self, new_text_align: TextAlign) {
        self.current_state.text_align = new_text_align;
    }

    #[inline]
    pub fn text_baseline(&self) -> TextBaseline {
        self.current_state.text_baseline
    }

    #[inline]
    pub fn set_text_baseline(&mut self, new_text_baseline: TextBaseline) {
        self.current_state.text_baseline = new_text_baseline;
    }
}

/// Represents the dimensions of a piece of text in the canvas.
#[derive(Clone, Copy, Debug)]
pub struct TextMetrics {
    /// The calculated width of a segment of inline text in pixels.
    pub width: f32,
    /// The distance from the alignment point given by the `text_align` state to the left side of
    /// the bounding rectangle of the given text, in pixels. The distance is measured parallel to
    /// the baseline.
    pub actual_bounding_box_left: f32,
    /// The distance from the alignment point given by the `text_align` state to the right side of
    /// the bounding rectangle of the given text, in pixels. The distance is measured parallel to
    /// the baseline.
    pub actual_bounding_box_right: f32,
    /// The distance from the horizontal line indicated by the `text_baseline` state to the top of
    /// the highest bounding rectangle of all the fonts used to render the text, in pixels.
    pub font_bounding_box_ascent: f32,
    /// The distance from the horizontal line indicated by the `text_baseline` state to the bottom
    /// of the highest bounding rectangle of all the fonts used to render the text, in pixels.
    pub font_bounding_box_descent: f32,
    /// The distance from the horizontal line indicated by the `text_baseline` state to the top of
    /// the bounding rectangle used to render the text, in pixels.
    pub actual_bounding_box_ascent: f32,
    /// The distance from the horizontal line indicated by the `text_baseline` state to the bottom
    /// of the bounding rectangle used to render the text, in pixels.
    pub actual_bounding_box_descent: f32,
    /// The distance from the horizontal line indicated by the `text_baseline` state to the top of
    /// the em square in the line box, in pixels.
    pub em_height_ascent: f32,
    /// The distance from the horizontal line indicated by the `text_baseline` state to the bottom
    /// of the em square in the line box, in pixels.
    pub em_height_descent: f32,
    /// The distance from the horizontal line indicated by the `text_baseline` state to the hanging
    /// baseline of the line box, in pixels.
    pub hanging_baseline: f32,
    /// The distance from the horizontal line indicated by the `text_baseline` state to the
    /// alphabetic baseline of the line box, in pixels.
    pub alphabetic_baseline: f32,
    /// The distance from the horizontal line indicated by the `text_baseline` state to the
    /// ideographic baseline of the line box, in pixels.
    pub ideographic_baseline: f32,
}

#[cfg(feature = "pf-text")]
#[derive(Clone)]
pub struct CanvasFontContext(pub(crate) Rc<RefCell<CanvasFontContextData>>);

pub(super) struct CanvasFontContextData {
    pub(super) font_context: FontContext<Font>,
    #[allow(dead_code)]
    pub(super) font_source: Arc<dyn Source>,
    #[allow(dead_code)]
    pub(super) default_font_collection: Arc<FontCollection>,
}

impl CanvasFontContext {
    pub fn new(font_source: Arc<dyn Source>) -> CanvasFontContext {
        let mut default_font_collection = FontCollection::new();
        if let Ok(default_font) = font_source.select_best_match(&[FamilyName::SansSerif],
                                                                &Properties::new()) {
            if let Ok(default_font) = default_font.load() {
                default_font_collection.add_family(FontFamily::new_from_font(default_font));
            }
        }

        CanvasFontContext(Rc::new(RefCell::new(CanvasFontContextData {
            font_source,
            default_font_collection: Arc::new(default_font_collection),
            font_context: FontContext::new(),
        })))
    }

    /// A convenience method to create a font context with the system source.
    /// This allows usage of fonts installed on the system.
    pub fn from_system_source() -> CanvasFontContext {
        CanvasFontContext::new(Arc::new(SystemSource::new()))
    }

    /// A convenience method to create a font context with a set of in-memory fonts.
    pub fn from_fonts<I>(fonts: I) -> CanvasFontContext where I: Iterator<Item = Handle> {
        CanvasFontContext::new(Arc::new(MemSource::from_fonts(fonts).unwrap()))
    }

    fn get_font_by_postscript_name(&self, postscript_name: &str) -> Font {
        let this = self.0.borrow();
        if let Some(cached_font) = this.font_context.get_cached_font(postscript_name) {
            return (*cached_font).clone();
        }
        this.font_source
            .select_by_postscript_name(postscript_name)
            .expect("Couldn't find a font with that PostScript name!")
            .load()
            .expect("Failed to load the font!")
    }
}

// Text layout utilities

impl TextMetrics {
    fn text_origin(&self, state: &State) -> Vector2F {
        let x = match state.text_align {
            TextAlign::Left => 0.0,
            TextAlign::Right => -self.width,
            TextAlign::Center => -0.5 * self.width,
        };

        let y = match state.text_baseline {
            TextBaseline::Alphabetic => 0.0,
            TextBaseline::Top => self.em_height_ascent,
            TextBaseline::Middle => util::lerp(self.em_height_ascent, self.em_height_descent, 0.5),
            TextBaseline::Bottom => self.em_height_descent,
            TextBaseline::Ideographic => self.ideographic_baseline,
            TextBaseline::Hanging => self.hanging_baseline,
        };

        vec2f(x, y)
    }

    fn make_origin_relative(&mut self, state: &State) {
        let text_origin = self.text_origin(state);
        self.actual_bounding_box_left += text_origin.x();
        self.actual_bounding_box_right += text_origin.x();
        self.font_bounding_box_ascent -= text_origin.y();
        self.font_bounding_box_descent -= text_origin.y();
        self.actual_bounding_box_ascent -= text_origin.y();
        self.actual_bounding_box_descent -= text_origin.y();
        self.em_height_ascent -= text_origin.y();
        self.em_height_descent -= text_origin.y();
        self.hanging_baseline -= text_origin.y();
        self.alphabetic_baseline -= text_origin.y();
        self.ideographic_baseline -= text_origin.y();
    }
}

pub trait LayoutExt {
    fn metrics(&self) -> TextMetrics;
    fn width(&self) -> f32;
    fn actual_bounding_box_left(&self) -> f32;
    fn actual_bounding_box_right(&self) -> f32;
    fn hanging_baseline(&self) -> f32;
    fn ideographic_baseline(&self) -> f32;
}

impl LayoutExt for Layout {
    // NB: This does not return origin-relative values. To get those, call `make_origin_relative()`
    // afterward.
    fn metrics(&self) -> TextMetrics {
        let (mut em_height_ascent, mut em_height_descent) = (0.0, 0.0);
        let (mut font_bounding_box_ascent, mut font_bounding_box_descent) = (0.0, 0.0);
        let (mut actual_bounding_box_ascent, mut actual_bounding_box_descent) = (0.0, 0.0);

        let mut last_font: Option<Arc<Font>> = None;
        for glyph in &self.glyphs {
            match last_font {
                Some(ref last_font) if Arc::ptr_eq(&last_font, &glyph.font.font) => {}
                _ => {
                    let font = glyph.font.font.clone();

                    let font_metrics = font.metrics();
                    let scale_factor = self.size / font_metrics.units_per_em as f32;
                    em_height_ascent = (font_metrics.ascent * scale_factor).max(em_height_ascent);
                    em_height_descent =
                        (font_metrics.descent * scale_factor).min(em_height_descent);
                    font_bounding_box_ascent = (font_metrics.bounding_box.max_y() *
                                                scale_factor).max(font_bounding_box_ascent);
                    font_bounding_box_descent = (font_metrics.bounding_box.min_y() *
                                                 scale_factor).min(font_bounding_box_descent);

                    last_font = Some(font);
                }
            }

            let font = last_font.as_ref().unwrap();
            let glyph_rect = font.raster_bounds(glyph.glyph_id,
                                                self.size,
                                                Transform2F::default(),
                                                HintingOptions::None,
                                                RasterizationOptions::GrayscaleAa).unwrap();
            actual_bounding_box_ascent =
                (glyph_rect.max_y() as f32).max(actual_bounding_box_ascent);
            actual_bounding_box_descent =
                (glyph_rect.min_y() as f32).min(actual_bounding_box_descent);
        }

        TextMetrics {
            width: self.width(),
            actual_bounding_box_left: self.actual_bounding_box_left(),
            actual_bounding_box_right: self.actual_bounding_box_right(),
            font_bounding_box_ascent,
            font_bounding_box_descent,
            actual_bounding_box_ascent,
            actual_bounding_box_descent,
            em_height_ascent,
            em_height_descent,
            alphabetic_baseline: 0.0,
            hanging_baseline: self.hanging_baseline(),
            ideographic_baseline: self.ideographic_baseline(),
        }
    }

    fn width(&self) -> f32 {
        let last_glyph = match self.glyphs.last() {
            None => return 0.0,
            Some(last_glyph) => last_glyph,
        };

        let glyph_id = last_glyph.glyph_id;
        let font_metrics = last_glyph.font.font.metrics();
        let scale_factor = self.size / font_metrics.units_per_em as f32;
        let glyph_rect = last_glyph.font.font.typographic_bounds(glyph_id).unwrap();
        last_glyph.offset.x() + glyph_rect.max_x() * scale_factor
    }

    fn actual_bounding_box_left(&self) -> f32 {
        let first_glyph = match self.glyphs.get(0) {
            None => return 0.0,
            Some(first_glyph) => first_glyph,
        };

        let glyph_id = first_glyph.glyph_id;
        let font_metrics = first_glyph.font.font.metrics();
        let scale_factor = self.size / font_metrics.units_per_em as f32;
        let glyph_rect = first_glyph.font
                                    .font
                                    .raster_bounds(glyph_id,
                                                   font_metrics.units_per_em as f32,
                                                   Transform2F::default(),
                                                   HintingOptions::None,
                                                   RasterizationOptions::GrayscaleAa).unwrap();
        first_glyph.offset.x() + glyph_rect.min_x() as f32 * scale_factor
    }

    fn actual_bounding_box_right(&self) -> f32 {
        let last_glyph = match self.glyphs.last() {
            None => return 0.0,
            Some(last_glyph) => last_glyph,
        };

        let glyph_id = last_glyph.glyph_id;
        let font_metrics = last_glyph.font.font.metrics();
        let scale_factor = self.size / font_metrics.units_per_em as f32;
        let glyph_rect = last_glyph.font
                                   .font
                                   .raster_bounds(glyph_id,
                                                  font_metrics.units_per_em as f32,
                                                  Transform2F::default(),
                                                  HintingOptions::None,
                                                  RasterizationOptions::GrayscaleAa).unwrap();
        last_glyph.offset.x() + glyph_rect.max_x() as f32 * scale_factor
    }

    fn hanging_baseline(&self) -> f32 {
        // TODO(pcwalton)
        0.0
    }

    fn ideographic_baseline(&self) -> f32 {
        // TODO(pcwalton)
        0.0
    }
}

/// Various things that can be conveniently converted into font collections for use with
/// `CanvasRenderingContext2D::set_font()`.
pub trait IntoFontCollection {
    fn into_font_collection(self, font_context: &CanvasFontContext) -> Arc<FontCollection>;
}

impl IntoFontCollection for Arc<FontCollection> {
    #[inline]
    fn into_font_collection(self, _: &CanvasFontContext) -> Arc<FontCollection> {
        self
    }
}

impl IntoFontCollection for FontFamily {
    #[inline]
    fn into_font_collection(self, _: &CanvasFontContext) -> Arc<FontCollection> {
        let mut font_collection = FontCollection::new();
        font_collection.add_family(self);
        Arc::new(font_collection)
    }
}

impl IntoFontCollection for Vec<FontFamily> {
    #[inline]
    fn into_font_collection(self, _: &CanvasFontContext) -> Arc<FontCollection> {
        let mut font_collection = FontCollection::new();
        for family in self {
            font_collection.add_family(family);
        }
        Arc::new(font_collection)
    }
}

/*
impl IntoFontCollection for Handle {
    #[inline]
    fn into_font_collection(self, context: &CanvasFontContext) -> Arc<FontCollection> {
        self.load().expect("Failed to load the font!").into_font_collection(context)
    }
}

impl<'a> IntoFontCollection for &'a [Handle] {
    #[inline]
    fn into_font_collection(self, context: &CanvasFontContext) -> Arc<FontCollection> {
        let mut font_collection = FontCollection::new();
        for handle in self {
            let postscript_name = handle.postscript_name();

            let font = handle.load().expect("Failed to load the font!");
            font_collection.add_family(FontFamily::new_from_font(font));
        }
        Arc::new(font_collection)
    }
}
*/

impl IntoFontCollection for Font {
    #[inline]
    fn into_font_collection(self, context: &CanvasFontContext) -> Arc<FontCollection> {
        FontFamily::new_from_font(self).into_font_collection(context)
    }
}

impl<'a> IntoFontCollection for &'a [Font] {
    #[inline]
    fn into_font_collection(self, context: &CanvasFontContext) -> Arc<FontCollection> {
        let mut family = FontFamily::new();
        for font in self {
            family.add_font(FontRef::new((*font).clone()))
        }
        family.into_font_collection(context)
    }
}

impl<'a> IntoFontCollection for &'a str {
    #[inline]
    fn into_font_collection(self, context: &CanvasFontContext) -> Arc<FontCollection> {
        context.get_font_by_postscript_name(self).into_font_collection(context)
    }
}

impl<'a, 'b> IntoFontCollection for &'a [&'b str] {
    #[inline]
    fn into_font_collection(self, context: &CanvasFontContext) -> Arc<FontCollection> {
        let mut font_collection = FontCollection::new();
        for postscript_name in self {
            let font = context.get_font_by_postscript_name(postscript_name);
            font_collection.add_family(FontFamily::new_from_font(font));
        }
        Arc::new(font_collection)
    }
}
