use crate::Font;
use bevy_render::{
    texture::{Texture, TextureType},
    Color,
};
use font_kit::{
    canvas::{Canvas, Format, RasterizationOptions},
    hinting::HintingOptions,
};
use pathfinder_geometry::transform2d::Transform2F;
use skribo::{LayoutSession, TextStyle};
use std::ops::Range;

struct TextSurface {
    width: usize,
    height: usize,
    pixels: Vec<u8>,
}

fn composite(a: u8, b: u8) -> u8 {
    let y = ((255 - a) as u16) * ((255 - b) as u16);
    let y = (y + (y >> 8) + 0x80) >> 8; // fast approx to round(y / 255)
    255 - (y as u8)
}

impl TextSurface {
    fn new(width: usize, height: usize) -> TextSurface {
        let pixels = vec![0; width * height];
        TextSurface {
            width,
            height,
            pixels,
        }
    }

    fn paint_from_canvas(&mut self, canvas: &Canvas, x: i32, y: i32) {
        let (cw, ch) = (canvas.size.x(), canvas.size.y());
        let (w, h) = (self.width as i32, self.height as i32);
        let y = y - ch;
        let xmin = 0.max(-x);
        let xmax = cw.min(w - x);
        let ymin = 0.max(-y);
        let ymax = ch.min(h - y);
        for yy in ymin..(ymax.max(ymin)) {
            for xx in xmin..(xmax.max(xmin)) {
                let pix = canvas.pixels[(cw * yy + xx) as usize];
                let dst_ix = ((y + yy) * w + x + xx) as usize;
                self.pixels[dst_ix] = composite(self.pixels[dst_ix], pix);
            }
        }
    }

    fn paint_layout_session<S: AsRef<str>>(
        &mut self,
        layout: &mut LayoutSession<S>,
        x: i32,
        y: i32,
        size: f32,
        range: Range<usize>,
    ) {
        for run in layout.iter_substr(range) {
            let font = run.font();
            for glyph in run.glyphs() {
                let glyph_id = glyph.glyph_id;
                let glyph_x = (glyph.offset.x() as i32) + x;
                let glyph_y = (glyph.offset.y() as i32) + y;
                let bounds = font
                    .font
                    .raster_bounds(
                        glyph_id,
                        size,
                        Transform2F::default(),
                        HintingOptions::None,
                        RasterizationOptions::GrayscaleAa,
                    )
                    .unwrap();
                if bounds.width() > 0 && bounds.height() > 0 {
                    let origin_adj = bounds.origin().to_f32();
                    let neg_origin = -origin_adj;
                    let mut canvas = Canvas::new(bounds.size(), Format::A8);
                    font.font
                        .rasterize_glyph(
                            &mut canvas,
                            glyph_id,
                            size,
                            Transform2F::from_translation(neg_origin),
                            HintingOptions::None,
                            RasterizationOptions::GrayscaleAa,
                        )
                        .unwrap();
                    self.paint_from_canvas(
                        &canvas,
                        glyph_x + bounds.origin_x(),
                        glyph_y - bounds.origin_y(),
                    );
                }
            }
        }
    }
}

pub fn render_text(font: &Font, text: &str, color: Color, width: usize, height: usize) -> Texture {
    let mut surface = TextSurface::new(width, height);
    let style = TextStyle {
        size: height as f32,
    };
    let offset = style.size * (font.metrics.ascent - font.metrics.cap_height)
        / font.metrics.units_per_em as f32;

    let mut layout = LayoutSession::create(&text, &style, &font.collection);
    surface.paint_layout_session(
        &mut layout,
        0,
        style.size as i32 - offset as i32,
        style.size,
        0..text.len(),
    );
    let color_u8 = [
        (color.r * 255.0) as u8,
        (color.g * 255.0) as u8,
        (color.b * 255.0) as u8,
    ];

    Texture::load(TextureType::Data(
        surface
            .pixels
            .iter()
            .map(|p| {
                vec![
                    color_u8[0],
                    color_u8[1],
                    color_u8[2],
                    (color.a * *p as f32) as u8,
                ]
            })
            .flatten()
            .collect::<Vec<u8>>(),
        surface.width,
        surface.height,
    ))
}
