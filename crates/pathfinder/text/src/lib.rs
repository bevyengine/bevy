// pathfinder/text/src/lib.rs
//
// Copyright © 2020 The Pathfinder Project Developers.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use font_kit::error::GlyphLoadingError;
use font_kit::hinting::HintingOptions;
use font_kit::loader::Loader;
use font_kit::loaders::default::Font as DefaultLoader;
use font_kit::outline::OutlineSink;
use pathfinder_content::effects::BlendMode;
use pathfinder_content::outline::{Contour, Outline};
use pathfinder_content::stroke::{OutlineStrokeToFill, StrokeStyle};
use pathfinder_geometry::line_segment::LineSegment2F;
use pathfinder_geometry::transform2d::Transform2F;
use pathfinder_geometry::vector::{Vector2F, vec2f};
use pathfinder_renderer::paint::PaintId;
use pathfinder_renderer::scene::{ClipPathId, DrawPath, Scene};
use skribo::{FontCollection, Layout, TextStyle};
use std::collections::HashMap;
use std::mem;

#[derive(Clone)]
pub struct FontContext<F> where F: Loader {
    font_info: HashMap<String, FontInfo<F>>, 
}

#[derive(Clone)]
struct FontInfo<F> where F: Loader {
    font: F,
    outline_cache: HashMap<GlyphId, Outline>,
}

#[derive(Clone, Copy)]
pub struct FontRenderOptions {
    pub transform: Transform2F,
    pub render_mode: TextRenderMode,
    pub hinting_options: HintingOptions,
    pub clip_path: Option<ClipPathId>,
    pub blend_mode: BlendMode,
    pub paint_id: PaintId,
}

impl Default for FontRenderOptions {
    #[inline]
    fn default() -> FontRenderOptions {
        FontRenderOptions {
            transform: Transform2F::default(),
            render_mode: TextRenderMode::Fill,
            hinting_options: HintingOptions::None,
            clip_path: None,
            blend_mode: BlendMode::SrcOver,
            paint_id: PaintId(0),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Debug, Eq, Hash)]
pub struct GlyphId(pub u32);

impl<F> FontContext<F> where F: Loader {
    #[inline]
    pub fn new() -> FontContext<F> {
        FontContext { font_info: HashMap::new() }
    }

    pub fn push_glyph(&mut self,
                      scene: &mut Scene,
                      font: &F,
                      glyph_id: GlyphId,
                      render_options: &FontRenderOptions)
                      -> Result<(), GlyphLoadingError> {
        let font_key = font.postscript_name();
        let metrics = font.metrics();

        // Insert the font into the cache if needed.
        if let Some(ref font_key) = font_key {
            if !self.font_info.contains_key(&*font_key) {
                self.font_info.insert((*font_key).clone(), FontInfo::new((*font).clone()));
            }
        }

        // See if we have a cached outline.
        //
        // TODO(pcwalton): Cache hinted outlines too.
        let mut cached_outline = None;
        let can_cache_outline = font_key.is_some() &&
            render_options.hinting_options == HintingOptions::None;
        if can_cache_outline {
            if let Some(ref font_info) = self.font_info.get(&*font_key.as_ref().unwrap()) {
                if let Some(ref outline) = font_info.outline_cache.get(&glyph_id) {
                    cached_outline = Some((*outline).clone());
                }
            }
        }

        let mut outline = match cached_outline {
            Some(mut cached_outline) => {
                let scale = 1.0 / metrics.units_per_em as f32;
                cached_outline.transform(&(render_options.transform *
                                           Transform2F::from_scale(scale)));
                cached_outline
            }
            None => {
                let transform = if can_cache_outline {
                    Transform2F::from_scale(metrics.units_per_em as f32)
                } else {
                    render_options.transform
                };
                let mut outline_builder = OutlinePathBuilder::new(&transform);
                font.outline(glyph_id.0, render_options.hinting_options, &mut outline_builder)?;
                let mut outline = outline_builder.build();
                if can_cache_outline {
                    let font_key = font_key.as_ref().unwrap();
                    let font_info = self.font_info.get_mut(&*font_key).unwrap();
                    font_info.outline_cache.insert(glyph_id, outline.clone());
                    let scale = 1.0 / metrics.units_per_em as f32;
                    outline.transform(&(render_options.transform *
                                        Transform2F::from_scale(scale)));
                }
                outline
            }
        };

        if let TextRenderMode::Stroke(stroke_style) = render_options.render_mode {
            let mut stroke_to_fill = OutlineStrokeToFill::new(&outline, stroke_style);
            stroke_to_fill.offset();
            outline = stroke_to_fill.into_outline();
        }

        let mut path = DrawPath::new(outline, render_options.paint_id);
        path.set_clip_path(render_options.clip_path);
        path.set_blend_mode(render_options.blend_mode);

        scene.push_path(path);
        Ok(())
    }

    /// Attempts to look up a font in the font cache.
    #[inline]
    pub fn get_cached_font(&self, postscript_name: &str) -> Option<&F> {
        self.font_info.get(postscript_name).map(|font_info| &font_info.font)
    }
}

impl FontContext<DefaultLoader> {
    pub fn push_layout(&mut self,
                       scene: &mut Scene,
                       layout: &Layout,
                       style: &TextStyle,
                       render_options: &FontRenderOptions)
                       -> Result<(), GlyphLoadingError> {
        for glyph in &layout.glyphs {
            let offset = glyph.offset;
            let font = &*glyph.font.font;
            // FIXME(pcwalton): Cache this!
            let scale = style.size / (font.metrics().units_per_em as f32);
            let scale = vec2f(scale, -scale);
            let render_options = FontRenderOptions {
                transform: render_options.transform *
                    Transform2F::from_scale(scale).translate(offset),
                ..*render_options
            };
            self.push_glyph(scene, font, GlyphId(glyph.glyph_id), &render_options)?;
        }
        Ok(())
    }

    #[inline]
    pub fn push_text(&mut self,
                     scene: &mut Scene,
                     text: &str,
                     style: &TextStyle,
                     collection: &FontCollection,
                     render_options: &FontRenderOptions)
                     -> Result<(), GlyphLoadingError> {
        let layout = skribo::layout(style, collection, text);
        self.push_layout(scene, &layout, style, render_options)
    }
}

impl<F> FontInfo<F> where F: Loader {
    fn new(font: F) -> FontInfo<F> {
        FontInfo { font, outline_cache: HashMap::new() }
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum TextRenderMode {
    Fill,
    Stroke(StrokeStyle),
}

struct OutlinePathBuilder {
    outline: Outline,
    current_contour: Contour,
    transform: Transform2F,
}

impl OutlinePathBuilder {
    fn new(transform: &Transform2F) -> OutlinePathBuilder {
        OutlinePathBuilder {
            outline: Outline::new(),
            current_contour: Contour::new(),
            transform: *transform,
        }
    }

    fn flush_current_contour(&mut self) {
        if !self.current_contour.is_empty() {
            self.outline.push_contour(mem::replace(&mut self.current_contour, Contour::new()));
        }
    }

    fn build(mut self) -> Outline {
        self.flush_current_contour();
        self.outline
    }
}

impl OutlineSink for OutlinePathBuilder {
    fn move_to(&mut self, to: Vector2F) {
        self.flush_current_contour();
        self.current_contour.push_endpoint(self.transform * to);
    }

    fn line_to(&mut self, to: Vector2F) {
        self.current_contour.push_endpoint(self.transform * to);
    }

    fn quadratic_curve_to(&mut self, ctrl: Vector2F, to: Vector2F) {
        self.current_contour.push_quadratic(self.transform * ctrl, self.transform * to);
    }

    fn cubic_curve_to(&mut self, ctrl: LineSegment2F, to: Vector2F) {
        self.current_contour.push_cubic(self.transform * ctrl.from(),
                                        self.transform * ctrl.to(),
                                        self.transform * to);
    }

    fn close(&mut self) {
        self.current_contour.close();
    }
}
