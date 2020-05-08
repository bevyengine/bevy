// pathfinder/examples/canvas_nanovg/src/main.rs
//
// Copyright © 2020 The Pathfinder Project Developers.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use arrayvec::ArrayVec;
use font_kit::handle::Handle;
use font_kit::sources::mem::MemSource;
use image;
use pathfinder_canvas::{Canvas, CanvasFontContext, CanvasRenderingContext2D, LineJoin, Path2D};
use pathfinder_canvas::{TextAlign, TextBaseline};
use pathfinder_color::{ColorF, ColorU, rgbau, rgbf, rgbu};
use pathfinder_content::fill::FillRule;
use pathfinder_content::gradient::Gradient;
use pathfinder_content::outline::ArcDirection;
use pathfinder_content::pattern::{Image, Pattern};
use pathfinder_content::stroke::LineCap;
use pathfinder_geometry::angle;
use pathfinder_geometry::line_segment::LineSegment2F;
use pathfinder_geometry::rect::RectF;
use pathfinder_geometry::transform2d::Transform2F;
use pathfinder_geometry::util;
use pathfinder_geometry::vector::{Vector2F, vec2f, vec2i};
use pathfinder_gl::{GLDevice, GLVersion};
use pathfinder_renderer::concurrent::rayon::RayonExecutor;
use pathfinder_renderer::concurrent::scene_proxy::SceneProxy;
use pathfinder_renderer::gpu::options::{DestFramebuffer, RendererOptions};
use pathfinder_renderer::gpu::renderer::Renderer;
use pathfinder_renderer::options::BuildOptions;
use pathfinder_resources::ResourceLoader;
use pathfinder_resources::fs::FilesystemResourceLoader;
use pathfinder_simd::default::F32x2;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::video::GLProfile;
use std::collections::VecDeque;
use std::f32::consts::PI;
use std::iter;
use std::sync::Arc;
use std::time::Instant;

#[cfg(not(windows))]
use jemallocator;

#[cfg(not(windows))]
#[global_allocator]
static ALLOC: jemallocator::Jemalloc = jemallocator::Jemalloc;

const PI_2: f32 = PI * 2.0;
const FRAC_PI_2_3: f32 = PI * 2.0 / 3.0;

const WINDOW_WIDTH: i32 = 1024;
const WINDOW_HEIGHT: i32 = WINDOW_WIDTH * 3 / 4;

const GRAPH_WIDTH: f32 = 200.0;
const GRAPH_HEIGHT: f32 = 35.0;
const GRAPH_HISTORY_COUNT: usize = 100;

static FONT_NAME_REGULAR: &'static str = "Roboto-Regular";
static FONT_NAME_BOLD:    &'static str = "Roboto-Bold";
static FONT_NAME_EMOJI:   &'static str = "NotoEmoji";

static PARAGRAPH_TEXT: &'static str = "This is a longer chunk of text.

I would have used lorem ipsum, but she was busy jumping over the lazy dog with the fox and all \
the men who came to the aid of the party. 🎉";

static HOVER_TEXT: &'static str = "Hover your mouse over the text to see the calculated caret \
position.";

fn render_demo(context: &mut CanvasRenderingContext2D,
               mouse_position: Vector2F,
               window_size: Vector2F,
               time: f32,
               hidpi_factor: f32,
               data: &DemoData) {
    draw_eyes(context,
              RectF::new(vec2f(window_size.x() - 250.0, 50.0), vec2f(150.0, 100.0)),
              mouse_position,
              time);
    draw_paragraph(context, vec2f(window_size.x() - 450.0, 50.0), 150.0, mouse_position);
    draw_graph(context,
               RectF::new(window_size * vec2f(0.0, 0.5), window_size * vec2f(1.0, 0.5)),
               time);
    draw_color_wheel(context,
                     RectF::new(window_size - vec2f(300.0, 300.0), vec2f(250.0, 250.0)),
                     time,
                     hidpi_factor);
    draw_lines(context,
               RectF::new(vec2f(120.0, window_size.y() - 50.0), vec2f(600.0, 50.0)),
               time);
    draw_widths(context, vec2f(10.0, 50.0), 30.0);
    draw_caps(context, RectF::new(vec2f(10.0, 300.0), vec2f(30.0, 40.0)));
    draw_clip(context, vec2f(50.0, window_size.y() - 80.0), time);

    context.save();

    // Draw widgets.
    draw_window(context,
                "Widgets & Stuff",
                RectF::new(vec2f(50.0, 50.0), vec2f(300.0, 400.0)),
                hidpi_factor);
    let mut position = vec2f(60.0, 95.0);
    draw_search_box(context, "Search", RectF::new(position, vec2f(280.0, 25.0)), hidpi_factor);
    position += vec2f(0.0, 40.0);
    draw_dropdown(context, "Effects", RectF::new(position, vec2f(280.0, 28.0)));
    let popup_position = position + vec2f(0.0, 14.0);
    position += vec2f(0.0, 45.0);

    // Draw login form.
    draw_label(context, "Login", RectF::new(position, vec2f(280.0, 20.0)));
    position += vec2f(0.0, 25.0);
    draw_text_edit_box(context,
                       "E-mail address",
                       RectF::new(position, vec2f(280.0, 28.0)),
                       hidpi_factor);
    position += vec2f(0.0, 35.0);
    draw_text_edit_box(context,
                       "Password",
                       RectF::new(position, vec2f(280.0, 28.0)),
                       hidpi_factor);
    position += vec2f(0.0, 38.0);
    draw_check_box(context, "Remember me", RectF::new(position, vec2f(140.0, 28.0)), hidpi_factor);
    draw_button(context,
                Some("🚪"),
                "Sign In",
                RectF::new(position + vec2f(138.0, 0.0), vec2f(140.0, 28.0)),
                rgbu(0, 96, 128));
    position += vec2f(0.0, 45.0);

    // Draw slider form.
    draw_label(context, "Diameter", RectF::new(position, vec2f(280.0, 20.0)));
    position += vec2f(0.0, 25.0);
    draw_numeric_edit_box(context,
                          "123.00",
                          "px",
                          RectF::new(position + vec2f(180.0, 0.0), vec2f(100.0, 28.0)),
                          hidpi_factor);
    draw_slider(context, 0.4, RectF::new(position, vec2f(170.0, 28.0)), hidpi_factor);
    position += vec2f(0.0, 55.0);

    // Draw dialog box buttons.
    draw_button(context,
                Some("️❌"),
                "Delete",
                RectF::new(position, vec2f(160.0, 28.0)),
                rgbu(128, 16, 8));
    draw_button(context,
                None,
                "Cancel",
                RectF::new(position + vec2f(170.0, 0.0), vec2f(110.0, 28.0)),
                rgbau(0, 0, 0, 0));

    // Draw thumbnails.
    draw_thumbnails(context,
                    RectF::new(vec2f(365.0, popup_position.y() - 30.0), vec2f(160.0, 300.0)),
                    time,
                    hidpi_factor,
                    12,
                    &data.image);

    context.restore();
}

fn draw_eyes(context: &mut CanvasRenderingContext2D,
             rect: RectF,
             mouse_position: Vector2F,
             time: f32) {
    let eyes_radii = rect.size() * vec2f(0.23, 0.5);
    let eyes_left_position = rect.origin() + eyes_radii;
    let eyes_right_position = rect.origin() + vec2f(rect.width() - eyes_radii.x(), eyes_radii.y());
    let eyes_center = f32::min(eyes_radii.x(), eyes_radii.y()) * 0.5;
    let blink = 1.0 - f32::powf((time * 0.5).sin(), 200.0) * 0.8;

    let mut gradient = Gradient::linear(
        LineSegment2F::new(vec2f(0.0, rect.height() * 0.5),
                           rect.size() * vec2f(0.1, 1.0)) + rect.origin());
    gradient.add_color_stop(rgbau(0, 0, 0, 32), 0.0);
    gradient.add_color_stop(rgbau(0, 0, 0, 16), 1.0);
    let mut path = Path2D::new();
    path.ellipse(eyes_left_position  + vec2f(3.0, 16.0), eyes_radii, 0.0, 0.0, PI_2);
    path.ellipse(eyes_right_position + vec2f(3.0, 16.0), eyes_radii, 0.0, 0.0, PI_2);
    context.set_fill_style(gradient);
    context.fill_path(path, FillRule::Winding);

    let mut gradient =
        Gradient::linear(LineSegment2F::new(vec2f(0.0, rect.height() * 0.25),
                                            rect.size() * vec2f(0.1, 1.0)) + rect.origin());
    gradient.add_color_stop(rgbu(220, 220, 220), 0.0);
    gradient.add_color_stop(rgbu(128, 128, 128), 1.0);
    let mut path = Path2D::new();
    path.ellipse(eyes_left_position, eyes_radii, 0.0, 0.0, PI_2);
    path.ellipse(eyes_right_position, eyes_radii, 0.0, 0.0, PI_2);
    context.set_fill_style(gradient);
    context.fill_path(path, FillRule::Winding);

    let mut delta = (mouse_position - eyes_right_position) / (eyes_radii * 10.0);
    let distance = delta.length();
    if distance > 1.0 {
        delta *= 1.0 / distance;
    }
    delta *= eyes_radii * vec2f(0.4, 0.5);
    let mut path = Path2D::new();
    path.ellipse(eyes_left_position + delta + vec2f(0.0, eyes_radii.y() * 0.25 * (1.0 - blink)),
                 vec2f(eyes_center, eyes_center * blink),
                 0.0,
                 0.0,
                 PI_2);
    path.ellipse(eyes_right_position + delta + vec2f(0.0, eyes_radii.y() * 0.25 * (1.0 - blink)),
                 vec2f(eyes_center, eyes_center * blink),
                 0.0,
                 0.0,
                 PI_2);
    context.set_fill_style(rgbu(32, 32, 32));
    context.fill_path(path, FillRule::Winding);

    let gloss_position = eyes_left_position - eyes_radii * vec2f(0.25, 0.5);
    let gloss_radii = F32x2::new(0.1, 0.75) * F32x2::splat(eyes_radii.x());
    let mut gloss = Gradient::radial(gloss_position, gloss_radii);
    gloss.add_color_stop(rgbau(255, 255, 255, 128), 0.0);
    gloss.add_color_stop(rgbau(255, 255, 255, 0), 1.0);
    context.set_fill_style(gloss);
    let mut path = Path2D::new();
    path.ellipse(eyes_left_position, eyes_radii, 0.0, 0.0, PI_2);
    context.fill_path(path, FillRule::Winding);

    let gloss_position = eyes_right_position - eyes_radii * vec2f(0.25, 0.5);
    let mut gloss = Gradient::radial(gloss_position, gloss_radii);
    gloss.add_color_stop(rgbau(255, 255, 255, 128), 0.0);
    gloss.add_color_stop(rgbau(255, 255, 255, 0), 1.0);
    context.set_fill_style(gloss);
    let mut path = Path2D::new();
    path.ellipse(eyes_right_position, eyes_radii, 0.0, 0.0, PI_2);
    context.fill_path(path, FillRule::Winding);
}

fn draw_paragraph(context: &mut CanvasRenderingContext2D,
                  origin: Vector2F,
                  line_width: f32,
                  mouse_position: Vector2F) {
    const MAIN_LINE_HEIGHT: f32 = 24.0;

    context.save();

    context.set_font(&[FONT_NAME_REGULAR, FONT_NAME_EMOJI][..]);
    context.set_font_size(18.0);
    context.set_fill_style(ColorU::white());
    context.set_text_align(TextAlign::Left);
    context.set_text_baseline(TextBaseline::Alphabetic);
    let main_text = MultilineTextBox::new(context,
                                          PARAGRAPH_TEXT,
                                          origin + vec2f(0.0, 24.0),
                                          line_width);
    let main_text_hit_location = main_text.hit_test(context, mouse_position);

    for (main_text_line_index, main_text_line) in main_text.lines.iter().enumerate() {
        let bg_alpha = match main_text_hit_location {
            Some(ref main_text_hit_location) if
                    main_text_hit_location.line_index == main_text_line_index as u32 => {
                64
            }
            _ => 16,
        };
        main_text_line.draw(context, rgbau(255, 255, 255, bg_alpha), ColorU::white());
    }

    if let Some(text_location) = main_text_hit_location {
        let caret_position = main_text.char_position(context, text_location);
        context.set_fill_style(rgbau(255, 192, 0, 255));
        context.fill_rect(RectF::new(caret_position, vec2f(1.0, MAIN_LINE_HEIGHT)));

        let line_bounds = main_text.lines[text_location.line_index as usize].bounds();
        let gutter_origin = line_bounds.origin() + vec2f(-10.0, MAIN_LINE_HEIGHT * 0.5);

        context.set_font_size(12.0);
        context.set_text_align(TextAlign::Right);
        context.set_text_baseline(TextBaseline::Middle);
        context.set_fill_style(rgbau(255, 192, 0, 255));

        let gutter_text = format!("{}", text_location.line_index + 1);
        let gutter_text_metrics = context.measure_text(&gutter_text);

        let gutter_text_bounds =
            RectF::from_points(vec2f(gutter_text_metrics.actual_bounding_box_left,
                                     -gutter_text_metrics.font_bounding_box_ascent),
                               vec2f(gutter_text_metrics.actual_bounding_box_right,
                                     -gutter_text_metrics.font_bounding_box_descent));
        let gutter_path_bounds = gutter_text_bounds.dilate(vec2f(4.0, 2.0));
        let gutter_path_radius = gutter_path_bounds.width() * 0.5 - 1.0;
        let path = create_rounded_rect_path(gutter_path_bounds + gutter_origin,
                                            gutter_path_radius);
        context.fill_path(path, FillRule::Winding);

        context.set_fill_style(rgbau(32, 32, 32, 255));
        context.fill_text(&gutter_text, gutter_origin);
    }

    // Fade out the tooltip when close to it.
    context.set_font_size(11.0);
    context.set_text_align(TextAlign::Left);
    context.set_text_baseline(TextBaseline::Alphabetic);
    let tooltip_origin = main_text.bounds.lower_left() + vec2f(0.0, 38.0);
    let tooltip = MultilineTextBox::new(context, HOVER_TEXT, tooltip_origin, 150.0);
    let mouse_vector = mouse_position.clamp(tooltip.bounds.origin(),
                                            tooltip.bounds.lower_right()) - mouse_position;
    context.set_global_alpha(util::clamp(mouse_vector.length() / 30.0, 0.0, 1.0));

    // Draw tooltip background.
    context.set_fill_style(rgbau(220, 220, 220, 255));
    let mut path = create_rounded_rect_path(tooltip.bounds.dilate(2.0), 3.0);
    path.move_to(vec2f(tooltip.bounds.center().x(), tooltip.bounds.origin_y() - 10.0));
    path.line_to(vec2f(tooltip.bounds.center().x() + 7.0, tooltip.bounds.origin_y() + 1.0));
    path.line_to(vec2f(tooltip.bounds.center().x() - 7.0, tooltip.bounds.origin_y() + 1.0));
    context.fill_path(path, FillRule::Winding);

    // Draw tooltip.
    context.set_fill_style(rgbau(0, 0, 0, 220));
    tooltip.draw(context, rgbau(0, 0, 0, 0), rgbau(0, 0, 0, 220));

    context.restore();
}

// This is nowhere near correct line layout, but it suffices to more or less match what NanoVG
// does.

struct MultilineTextBox {
    lines: Vec<Line>,
    bounds: RectF,
}

struct Line {
    words: Vec<Word>,
    origin: Vector2F,
    ascent: f32,
    descent: f32,
    width: f32,
    max_width: f32,
}

struct Word {
    text: String,
    origin_x: f32,
}

#[derive(Clone, Copy, Debug)]
struct TextLocation {
    line_index: u32,
    line_location: LineLocation,
}

#[derive(Clone, Copy, Debug)]
struct LineLocation {
    word_index: u32,
    char_index: u32,
}

impl MultilineTextBox {
    fn new(context: &mut CanvasRenderingContext2D,
           text: &str,
           mut origin: Vector2F,
           max_width: f32)
           -> MultilineTextBox {
        const LINE_SPACING: f32 = 3.0;

        let a_b_measure = context.measure_text("A B");
        let space_width = a_b_measure.width - context.measure_text("AB").width;
        let line_height = a_b_measure.em_height_ascent - a_b_measure.em_height_descent +
            LINE_SPACING;

        let mut text: VecDeque<VecDeque<_>> = text.split('\n').map(|paragraph| {
            paragraph.split(' ').map(|word| word.to_owned()).collect()
        }).collect();
        let mut lines = vec![];
        let mut bounds = None;

        while let Some(mut paragraph) = text.pop_front() {
            while !paragraph.is_empty() {
                let mut line = Line::new(origin, max_width);
                line.layout(context, &mut paragraph, space_width);

                origin += vec2f(0.0, line_height);
                match bounds {
                    None => bounds = Some(line.bounds()),
                    Some(ref mut bounds) => *bounds = bounds.union_rect(line.bounds()),
                }

                lines.push(line);
            }
        }

        MultilineTextBox { bounds: bounds.unwrap_or_default(), lines }
    }

    fn draw(&self, context: &mut CanvasRenderingContext2D, bg_color: ColorU, fg_color: ColorU) {
        for line in &self.lines {
            line.draw(context, bg_color, fg_color);
        }
    }

    fn hit_test(&self, context: &CanvasRenderingContext2D, mouse_position: Vector2F)
                -> Option<TextLocation> {
        for (line_index, line) in self.lines.iter().enumerate() {
            if line.bounds().contains_point(mouse_position) {
                if let Some(line_location) = line.hit_test(context, mouse_position) {
                    return Some(TextLocation { line_index: line_index as u32, line_location });
                }
            }
        }
        None
    }

    fn char_position(&self, context: &CanvasRenderingContext2D, text_location: TextLocation)
                     -> Vector2F {
        let line = &self.lines[text_location.line_index as usize];
        line.bounds().origin() + vec2f(line.char_position(context, text_location.line_location),
                                       0.0)
    }
}

impl Line {
    fn new(origin: Vector2F, max_width: f32) -> Line {
        Line { words: vec![], origin, ascent: 0.0, descent: 0.0, width: 0.0, max_width }
    }

    fn layout(&mut self,
              context: &mut CanvasRenderingContext2D,
              text: &mut VecDeque<String>,
              space_width: f32) {
        while let Some(word) = text.pop_front() {
            let mut word_origin_x = self.width;
            if self.width > 0.0 {
                word_origin_x += space_width;
            }

            let word_metrics = context.measure_text(&word);
            let new_line_width = word_origin_x + word_metrics.width;
            if self.width != 0.0 && new_line_width > self.max_width {
                text.push_front(word);
                return;
            }

            self.words.push(Word { text: word, origin_x: word_origin_x });
            self.width = new_line_width;
            self.ascent = self.ascent.max(word_metrics.em_height_ascent);
            self.descent = self.descent.min(word_metrics.em_height_descent);
        }
    }

    fn draw(&self, context: &mut CanvasRenderingContext2D, bg_color: ColorU, fg_color: ColorU) {
        context.set_text_align(TextAlign::Left);
        context.set_text_baseline(TextBaseline::Alphabetic);

        if !bg_color.is_fully_transparent() {
            context.set_fill_style(bg_color);
            context.fill_rect(self.bounds());
        }

        context.set_fill_style(fg_color);
        for word in &self.words {
            context.fill_text(&word.text, self.origin + vec2f(word.origin_x, 0.0));
        }
    }

    fn bounds(&self) -> RectF {
        RectF::new(self.origin - vec2f(0.0, self.ascent),
                   vec2f(self.width, self.ascent - self.descent))
    }

    fn hit_test(&self, context: &CanvasRenderingContext2D, mut mouse_position: Vector2F)
                -> Option<LineLocation> {
        let bounds = self.bounds();
        mouse_position -= bounds.origin();
        if mouse_position.y() < 0.0 || mouse_position.y() > bounds.height() {
            return None;
        }

        // FIXME(pcwalton): This doesn't quite handle spaces correctly.
        for (word_index, word) in self.words.iter().enumerate().rev() {
            if word.origin_x <= mouse_position.x() {
                return Some(LineLocation {
                    word_index: word_index as u32,
                    char_index: word.hit_test(context, mouse_position.x()),
                });
            }
        }

        None
    }

    fn char_position(&self, context: &CanvasRenderingContext2D, line_location: LineLocation)
                     -> f32 {
        let word = &self.words[line_location.word_index as usize];
        word.origin_x + word.char_position(context, line_location.char_index)
    }
}

impl Word {
    fn hit_test(&self, context: &CanvasRenderingContext2D, position_x: f32) -> u32 {
        let (mut char_start_x, mut prev_char_index) = (self.origin_x, 0);
        for char_index in self.text
                              .char_indices()
                              .map(|(index, _)| index)
                              .skip(1)
                              .chain(iter::once(self.text.len())) {
            let char_end_x = self.origin_x + context.measure_text(&self.text[0..char_index]).width;
            if position_x <= (char_start_x + char_end_x) * 0.5 {
                return prev_char_index;
            }
            char_start_x = char_end_x;
            prev_char_index = char_index as u32;
        }
        return self.text.len() as u32;
    }

    fn char_position(&self, context: &CanvasRenderingContext2D, char_index: u32) -> f32 {
        context.measure_text(&self.text[0..(char_index as usize)]).width
    }
}

fn draw_graph(context: &mut CanvasRenderingContext2D, rect: RectF, time: f32) {
    let sample_spread = rect.width() / 5.0;

    let samples = [
        (1.0 + f32::sin(time * 1.2345  + f32::cos(time * 0.33457) * 0.44)) * 0.5,
        (1.0 + f32::sin(time * 0.68363 + f32::cos(time * 1.30)    * 1.55)) * 0.5,
        (1.0 + f32::sin(time * 1.1642  + f32::cos(time * 0.33457) * 1.24)) * 0.5,
        (1.0 + f32::sin(time * 0.56345 + f32::cos(time * 1.63)    * 0.14)) * 0.5,
        (1.0 + f32::sin(time * 1.6245  + f32::cos(time * 0.254)   * 0.3))  * 0.5,
        (1.0 + f32::sin(time * 0.345   + f32::cos(time * 0.03)    * 0.6))  * 0.5,
    ];

    let sample_scale = vec2f(sample_spread, rect.height() * 0.8);
    let sample_points: ArrayVec<[Vector2F; 6]> = samples.iter()
                                                        .enumerate()
                                                        .map(|(index, &sample)| {
        rect.origin() + vec2f(index as f32, sample) * sample_scale
    }).collect();

    // Draw graph background.
    let mut background = Gradient::linear(
        LineSegment2F::new(vec2f(0.0, 0.0), vec2f(0.0, rect.height())) + rect.origin());
    background.add_color_stop(rgbau(0, 160, 192, 0),  0.0);
    background.add_color_stop(rgbau(0, 160, 192, 64), 1.0);
    context.set_fill_style(background);
    let mut path = create_graph_path(&sample_points, sample_spread, Vector2F::zero());
    path.line_to(rect.lower_right());
    path.line_to(rect.lower_left());
    context.fill_path(path, FillRule::Winding);

    // Draw graph line shadow.
    context.set_stroke_style(rgbau(0, 0, 0, 32));
    context.set_line_width(3.0);
    let path = create_graph_path(&sample_points, sample_spread, vec2f(0.0, 2.0));
    context.stroke_path(path);

    // Draw graph line.
    context.set_stroke_style(rgbu(0, 160, 192));
    context.set_line_width(3.0);
    let path = create_graph_path(&sample_points, sample_spread, Vector2F::zero());
    context.stroke_path(path);

    // Draw sample position highlights.
    for &sample_point in &sample_points {
        let gradient_center = sample_point + vec2f(0.0, 2.0);
        let mut background = Gradient::radial(gradient_center, F32x2::new(3.0, 8.0));
        background.add_color_stop(rgbau(0, 0, 0, 32), 0.0);
        background.add_color_stop(rgbau(0, 0, 0, 0),  1.0);
        context.set_fill_style(background);
        context.fill_rect(RectF::new(sample_point + vec2f(-10.0, -10.0 + 2.0), vec2f(20.0, 20.0)));
    }

    // Draw sample positions.
    context.set_fill_style(rgbu(0, 160, 192));
    let mut path = Path2D::new();
    for &sample_point in &sample_points {
        path.ellipse(sample_point, vec2f(4.0, 4.0), 0.0, 0.0, PI_2);
    }
    context.fill_path(path, FillRule::Winding);
    context.set_fill_style(rgbu(220, 220, 220));
    let mut path = Path2D::new();
    for &sample_point in &sample_points {
        path.ellipse(sample_point, vec2f(2.0, 2.0), 0.0, 0.0, PI_2);
    }
    context.fill_path(path, FillRule::Winding);

    // Reset state.
    context.set_line_width(1.0);
}

fn draw_color_wheel(context: &mut CanvasRenderingContext2D,
                    rect: RectF,
                    time: f32,
                    hidpi_factor: f32) {
    let hue = (time * 0.12).sin() * PI_2;

    context.save();

    let center = rect.center();
    let outer_radius = f32::min(rect.width(), rect.height()) * 0.5 - 5.0;
    let inner_radius = outer_radius - 20.0;

    // Half a pixel arc length in radians.
    let half_arc_len = 0.5 / outer_radius;

    // Draw outer circle.
    for segment in 0..6 {
        let start_angle = segment       as f32 / 6.0 * PI_2 - half_arc_len;
        let end_angle   = (segment + 1) as f32 / 6.0 * PI_2 + half_arc_len;
        let line = LineSegment2F::new(vec2f(f32::cos(start_angle), f32::sin(start_angle)),
                                      vec2f(f32::cos(end_angle),   f32::sin(end_angle)));
        let scale = util::lerp(inner_radius, outer_radius, 0.5);
        let mut gradient = Gradient::linear(line * scale + center);
        let start_color = ColorF::from_hsl(start_angle, 1.0, 0.55).to_u8();
        let end_color   = ColorF::from_hsl(end_angle,   1.0, 0.55).to_u8();
        gradient.add_color_stop(start_color, 0.0);
        gradient.add_color_stop(end_color,   1.0);
        context.set_fill_style(gradient);
        let mut path = Path2D::new();
        path.arc(center, inner_radius, start_angle, end_angle,   ArcDirection::CW);
        path.arc(center, outer_radius, end_angle,   start_angle, ArcDirection::CCW);
        path.close_path();
        context.fill_path(path, FillRule::Winding);
    }

    // Stroke outer circle.
    context.set_stroke_style(rgbau(0, 0, 0, 64));
    context.set_line_width(1.0);
    let mut path = Path2D::new();
    path.ellipse(center, inner_radius - 0.5, 0.0, 0.0, PI_2);
    path.ellipse(center, outer_radius + 0.5, 0.0, 0.0, PI_2);
    context.stroke_path(path);

    // Prepare to draw the selector.
    context.save();
    context.translate(center);
    context.rotate(hue);

    // Draw marker.
    context.set_shadow_blur(4.0 * hidpi_factor);
    context.set_shadow_color(rgbu(0, 0, 0));
    context.set_shadow_offset(vec2f(0.0, 0.0));
    context.set_stroke_style(rgbau(255, 255, 255, 192));
    context.set_line_width(2.0);
    context.stroke_rect(RectF::new(vec2f(inner_radius - 1.0, -3.0),
                                   vec2f(outer_radius - inner_radius + 2.0, 6.0)));
    context.set_shadow_color(ColorU::transparent_black());

    // Draw center triangle.
    let triangle_radius = inner_radius - 6.0;
    let triangle_vertex_a = vec2f(triangle_radius, 0.0);
    let triangle_vertex_b = vec2f(FRAC_PI_2_3.cos(), FRAC_PI_2_3.sin()) * triangle_radius;
    let triangle_vertex_c = vec2f((-FRAC_PI_2_3).cos(), (-FRAC_PI_2_3).sin()) * triangle_radius;
    let mut gradient_0 = Gradient::linear_from_points(triangle_vertex_a, triangle_vertex_b);
    gradient_0.add_color_stop(ColorF::from_hsl(hue, 1.0, 0.5).to_u8(), 0.0);
    gradient_0.add_color_stop(ColorU::white(), 1.0);
    let mut gradient_1 =
        Gradient::linear_from_points(triangle_vertex_a.lerp(triangle_vertex_b, 0.5),
                                     triangle_vertex_c);
    gradient_1.add_color_stop(ColorU::transparent_black(), 0.0);
    gradient_1.add_color_stop(ColorU::black(),             1.0);
    let mut path = Path2D::new();
    path.move_to(triangle_vertex_a);
    path.line_to(triangle_vertex_b);
    path.line_to(triangle_vertex_c);
    path.close_path();
    context.set_fill_style(gradient_0);
    context.fill_path(path.clone(), FillRule::Winding);
    context.set_fill_style(gradient_1);
    context.fill_path(path.clone(), FillRule::Winding);
    context.set_stroke_style(rgbau(0, 0, 0, 64));
    context.stroke_path(path);

    // Stroke the selection circle on the triangle.
    let selection_circle_center = vec2f(FRAC_PI_2_3.cos(), FRAC_PI_2_3.sin()) * triangle_radius *
        vec2f(0.3, 0.4);
    context.set_stroke_style(rgbau(255, 255, 255, 192));
    context.set_line_width(2.0);
    let mut path = Path2D::new();
    path.ellipse(selection_circle_center, vec2f(5.0, 5.0), 0.0, 0.0, PI_2);
    context.stroke_path(path);

    // Fill the selection circle.
    let mut gradient = Gradient::radial(selection_circle_center, F32x2::new(7.0, 9.0));
    gradient.add_color_stop(rgbau(0, 0, 0, 64), 0.0);
    gradient.add_color_stop(rgbau(0, 0, 0, 0),  1.0);
    context.set_fill_style(gradient);
    let mut path = Path2D::new();
    path.rect(RectF::new(selection_circle_center - vec2f(20.0, 20.0), vec2f(40.0, 40.0)));
    path.ellipse(selection_circle_center, vec2f(7.0, 7.0), 0.0, 0.0, PI_2);
    context.fill_path(path, FillRule::EvenOdd);

    context.restore();
    context.restore();
}

fn draw_lines(context: &mut CanvasRenderingContext2D, rect: RectF, time: f32) {
    const PADDING: f32 = 5.0;

    let spacing = rect.width() / 9.0 - PADDING * 2.0;

    context.save();

    let points = [
        vec2f(-spacing * 0.25 + f32::cos(time * 0.3)  * spacing * 0.5,
              f32::sin(time * 0.3)  * spacing * 0.5),
        vec2f(-spacing * 0.25, 0.0),
        vec2f( spacing * 0.25, 0.0),
        vec2f( spacing * 0.25 + f32::cos(time * -0.3) * spacing * 0.5,
              f32::sin(time * -0.3) * spacing * 0.5),
    ];

    for (cap_index, &cap) in [LineCap::Butt, LineCap::Round, LineCap::Square].iter().enumerate() {
        for (join_index, &join) in [
            LineJoin::Miter, LineJoin::Round, LineJoin::Bevel
        ].iter().enumerate() {
            let origin = rect.origin() +
                vec2f(0.5, -0.5) * spacing +
                vec2f((cap_index * 3 + join_index) as f32 / 9.0 * rect.width(), 0.0) +
                PADDING;

            context.set_line_cap(cap);
            context.set_line_join(join);
            context.set_line_width(spacing * 0.3);
            context.set_stroke_style(rgbau(0, 0, 0, 160));

            let mut path = Path2D::new();
            path.move_to(points[0] + origin);
            path.line_to(points[1] + origin);
            path.line_to(points[2] + origin);
            path.line_to(points[3] + origin);
            context.stroke_path(path.clone());

            context.set_line_cap(LineCap::Butt);
            context.set_line_join(LineJoin::Bevel);
            context.set_line_width(1.0);
            context.set_stroke_style(rgbu(0, 192, 255));

            context.stroke_path(path);
        }
    }

    context.restore();
}

fn draw_widths(context: &mut CanvasRenderingContext2D, mut origin: Vector2F, width: f32) {
    context.save();
    context.set_stroke_style(rgbau(0, 0, 0, 255));

    for index in 0..20 {
        context.set_line_width((index as f32 + 0.5) * 0.1);
        let mut path = Path2D::new();
        path.move_to(origin);
        path.line_to(origin + vec2f(1.0, 0.3) * width);
        context.stroke_path(path);
        origin += vec2f(0.0, 10.0);
    }

    context.restore();
}

fn draw_caps(context: &mut CanvasRenderingContext2D, rect: RectF) {
    const LINE_WIDTH: f32 = 8.0;

    context.save();

    context.set_fill_style(rgbau(255, 255, 255, 32));
    context.fill_rect(rect.dilate(vec2f(LINE_WIDTH / 2.0, 0.0)));
    context.fill_rect(rect);

    context.set_line_width(LINE_WIDTH);
    for (cap_index, &cap) in [LineCap::Butt, LineCap::Round, LineCap::Square].iter().enumerate() {
        context.set_line_cap(cap);
        context.set_stroke_style(ColorU::black());
        let offset = cap_index as f32 * 10.0 + 5.0;
        let mut path = Path2D::new();
        path.move_to(rect.origin()      + vec2f(0.0, offset));
        path.line_to(rect.upper_right() + vec2f(0.0, offset));
        context.stroke_path(path);
    }

    context.restore();
}

fn draw_clip(context: &mut CanvasRenderingContext2D, origin: Vector2F, time: f32) {
    context.save();

    // Draw first rect.
    let original_transform = context.transform();
    let transform_a = original_transform *
        Transform2F::from_rotation(angle::angle_from_degrees(5.0)).translate(origin);
    context.set_transform(&transform_a);
    context.set_fill_style(rgbu(255, 0, 0));
    let mut clip_path_a = Path2D::new();
    let clip_rect_a = RectF::new(vec2f(-20.0, -20.0), vec2f(60.0, 40.0));
    clip_path_a.rect(clip_rect_a);
    context.fill_path(clip_path_a, FillRule::Winding);

    // Draw second rectangle with no clip.
    let transform_b = transform_a * Transform2F::from_rotation(time).translate(vec2f(40.0, 0.0));
    context.set_transform(&transform_b);
    context.set_fill_style(rgbau(255, 128, 0, 64));
    let fill_rect = RectF::new(vec2f(-20.0, -10.0), vec2f(60.0, 30.0));
    context.fill_rect(fill_rect);

    // Draw second rectangle with clip.
    let mut clip_path_b = Path2D::new();
    let clip_rect_b = (transform_b.inverse() * transform_a * clip_rect_a).intersection(fill_rect)
                                                                         .unwrap_or_default();
    clip_path_b.rect(clip_rect_b);
    context.clip_path(clip_path_b, FillRule::Winding);
    context.set_fill_style(rgbu(255, 128, 0));
    context.fill_rect(fill_rect);

    context.restore();
}

fn draw_window(context: &mut CanvasRenderingContext2D,  
               title: &str,
               rect: RectF,
               hidpi_factor: f32) {
    const CORNER_RADIUS: f32 = 3.0;

    context.save();

    // Draw window with shadow.
    context.set_fill_style(rgbau(28, 30, 34, 160));
    context.set_shadow_blur(10.0 * hidpi_factor);
    context.set_shadow_offset(vec2f(0.0, 2.0));
    context.set_shadow_color(rgbau(0, 0, 0, 128));
    context.fill_path(create_rounded_rect_path(rect, CORNER_RADIUS), FillRule::Winding);
    context.set_shadow_color(rgbau(0, 0, 0, 0));

    // Header.
    let mut header_gradient =
        Gradient::linear(LineSegment2F::new(Vector2F::zero(), vec2f(0.0, 15.0)) + rect.origin());
    header_gradient.add_color_stop(rgbau(255, 255, 255, 8),  0.0);
    header_gradient.add_color_stop(rgbau(0,   0,   0,   16), 1.0);
    context.set_fill_style(header_gradient);
    context.fill_path(create_rounded_rect_path(RectF::new(rect.origin() + vec2f(1.0, 1.0),
                                                          vec2f(rect.width() - 2.0, 30.0)),
                                              CORNER_RADIUS - 1.0),
                      FillRule::Winding);
    let mut path = Path2D::new();
    path.move_to(rect.origin() + vec2f(0.5, 30.5));
    path.line_to(rect.origin() + vec2f(rect.width() - 0.5, 30.5));
    context.set_stroke_style(rgbau(0, 0, 0, 32));
    context.stroke_path(path);

    context.set_font(FONT_NAME_BOLD);
    context.set_font_size(15.0);
    context.set_text_align(TextAlign::Center);
    context.set_text_baseline(TextBaseline::Middle);
    context.set_fill_style(rgbau(220, 220, 220, 160));
    context.set_shadow_blur(2.0 * hidpi_factor);
    context.set_shadow_offset(vec2f(0.0, 1.0));
    context.set_shadow_color(rgbu(0, 0, 0));
    context.fill_text(title, rect.origin() + vec2f(rect.width() * 0.5, 16.0));

    context.restore();
}

fn draw_search_box(context: &mut CanvasRenderingContext2D,
                   text: &str,
                   rect: RectF,
                   hidpi_factor: f32) {
    let corner_radius = rect.height() * 0.5 - 1.0;

    let path = create_rounded_rect_path(rect, corner_radius);
    context.set_fill_style(rgbau(0, 0, 0, 16));
    context.fill_path(path.clone(), FillRule::Winding);
    context.save();
    context.clip_path(path, FillRule::Winding);
    let shadow_path = create_rounded_rect_path(rect + vec2f(0.0, 1.5), corner_radius);
    context.set_shadow_blur(5.0 * hidpi_factor);
    context.set_shadow_offset(vec2f(0.0, 0.0));
    context.set_shadow_color(rgbau(0, 0, 0, 92));
    context.set_stroke_style(rgbau(0, 0, 0, 92));
    context.set_line_width(1.0);
    context.stroke_path(shadow_path);
    context.restore();

    context.set_font_size(rect.height() * 0.5);
    context.set_font(FONT_NAME_EMOJI);
    context.set_fill_style(rgbau(255, 255, 255, 64));
    context.set_text_align(TextAlign::Center);
    context.set_text_baseline(TextBaseline::Middle);
    context.fill_text("🔍", rect.origin() + Vector2F::splat(rect.height() * 0.55));

    context.set_font(FONT_NAME_REGULAR);
    context.set_font_size(17.0);
    context.set_fill_style(rgbau(255, 255, 255, 32));
    context.set_text_align(TextAlign::Left);
    context.set_text_baseline(TextBaseline::Middle);
    context.fill_text(text, rect.origin() + vec2f(1.05, 0.5) * rect.height());

    context.set_font_size(rect.height() * 0.5);
    context.set_font(FONT_NAME_EMOJI);
    context.set_text_align(TextAlign::Center);
    context.fill_text("️❌", rect.upper_right() + vec2f(-1.0, 1.0) * (rect.height() * 0.55));
}

fn draw_dropdown(context: &mut CanvasRenderingContext2D, text: &str, rect: RectF) {
    const CORNER_RADIUS: f32 = 4.0;

    let mut background_gradient = Gradient::linear_from_points(rect.origin(), rect.lower_left());
    background_gradient.add_color_stop(rgbau(255, 255, 255, 16), 0.0);
    background_gradient.add_color_stop(rgbau(0,   0,   0,   16), 1.0);
    context.set_fill_style(background_gradient);
    context.fill_path(create_rounded_rect_path(rect.contract(1.0), CORNER_RADIUS - 1.0),
                      FillRule::Winding);

    context.set_stroke_style(rgbau(0, 0, 0, 48));
    context.stroke_path(create_rounded_rect_path(rect.contract(0.5), CORNER_RADIUS - 0.5));

    context.set_font(FONT_NAME_REGULAR);
    context.set_font_size(17.0);
    context.set_fill_style(rgbau(255, 255, 255, 160));
    context.set_text_align(TextAlign::Left);
    context.set_text_baseline(TextBaseline::Middle);
    context.fill_text(text, rect.origin() + vec2f(0.3, 0.5) * rect.height());

    // Draw chevron. This is a glyph in the original, but I don't want to grab an icon font just
    // for this.
    context.save();
    context.translate(rect.upper_right() + vec2f(-0.5, 0.33) * rect.height());
    context.scale(0.1);
    context.set_fill_style(rgbau(255, 255, 255, 64));
    let mut path = Path2D::new();
    path.move_to(vec2f(0.0,  100.0));
    path.line_to(vec2f(32.8, 50.0));
    path.line_to(vec2f(0.0,  0.0));
    path.line_to(vec2f(22.1, 0.0));
    path.line_to(vec2f(54.2, 50.0));
    path.line_to(vec2f(22.1, 100.0));
    path.close_path();
    context.fill_path(path, FillRule::Winding);
    context.restore();
}

fn draw_label(context: &mut CanvasRenderingContext2D, text: &str, rect: RectF) {
    context.set_font(FONT_NAME_REGULAR);
    context.set_font_size(15.0);
    context.set_fill_style(rgbau(255, 255, 255, 128));
    context.set_text_align(TextAlign::Left);
    context.set_text_baseline(TextBaseline::Middle);
    context.fill_text(text, rect.origin() + vec2f(0.0, rect.height() * 0.5));
}

fn draw_edit_box(context: &mut CanvasRenderingContext2D, rect: RectF, hidpi_factor: f32) {
    const CORNER_RADIUS: f32 = 4.0;

    context.save();
    let path = create_rounded_rect_path(rect.contract(1.0), CORNER_RADIUS - 1.0);
    context.set_fill_style(rgbau(255, 255, 255, 32));
    context.fill_path(path.clone(), FillRule::Winding);
    context.clip_path(path.clone(), FillRule::Winding);
    context.set_line_width(1.0);
    context.set_shadow_blur(2.0 * hidpi_factor);
    context.set_shadow_color(rgbau(32, 32, 32, 92));
    context.set_shadow_offset(vec2f(0.0, 1.0));
    context.set_stroke_style(rgbau(32, 32, 32, 92));
    context.stroke_path(path);
    context.restore();

    context.set_stroke_style(rgbau(0, 0, 0, 48));
    context.stroke_path(create_rounded_rect_path(rect.contract(0.5), CORNER_RADIUS - 0.5));
}

fn draw_text_edit_box(context: &mut CanvasRenderingContext2D,
                      text: &str,
                      rect: RectF,
                      hidpi_factor: f32) {
    draw_edit_box(context, rect, hidpi_factor);

    context.set_font(FONT_NAME_REGULAR);
    context.set_font_size(17.0);
    context.set_fill_style(rgbau(255, 255, 255, 64));
    context.set_text_align(TextAlign::Left);
    context.set_text_baseline(TextBaseline::Middle);
    context.fill_text(text, rect.origin() + vec2f(0.3, 0.5) * rect.height());
}

fn draw_numeric_edit_box(context: &mut CanvasRenderingContext2D,
                         value: &str,
                         unit: &str,
                         rect: RectF,
                         hidpi_factor: f32) {
    draw_edit_box(context, rect, hidpi_factor);

    context.set_font(FONT_NAME_REGULAR);
    context.set_font_size(15.0);
    let unit_width = context.measure_text(unit).width;

    context.set_fill_style(rgbau(255, 255, 255, 64));
    context.set_text_align(TextAlign::Right);
    context.set_text_baseline(TextBaseline::Middle);
    context.fill_text(unit, rect.upper_right() + vec2f(-0.3, 0.5) * rect.height());

    context.set_font_size(17.0);
    context.set_fill_style(rgbau(255, 255, 255, 128));
    context.set_text_align(TextAlign::Right);
    context.set_text_baseline(TextBaseline::Middle);
    context.fill_text(value, rect.upper_right() + vec2f(-unit_width - rect.height() * 0.5,
                                                        rect.height() * 0.5));
}

fn draw_check_box(context: &mut CanvasRenderingContext2D,
                  text: &str,
                  rect: RectF,
                  hidpi_factor: f32) {
    const CORNER_RADIUS: f32 = 3.0;

    context.set_font(FONT_NAME_REGULAR);
    context.set_font_size(15.0);
    context.set_fill_style(rgbau(255, 255, 255, 160));
    context.set_text_align(TextAlign::Left);
    context.set_text_baseline(TextBaseline::Middle);
    context.fill_text(text, rect.origin() + vec2f(28.0, rect.height() * 0.5));

    context.save();
    let check_box_rect = RectF::new(vec2f(rect.origin_x(), rect.center().y().floor() - 9.0),
                                    vec2f(20.0, 20.0)).contract(1.0);
    let check_box_path = create_rounded_rect_path(check_box_rect, CORNER_RADIUS);
    context.set_fill_style(rgbau(0, 0, 0, 32));
    context.fill_path(check_box_path.clone(), FillRule::Winding);
    context.clip_path(check_box_path, FillRule::Winding);
    context.set_line_width(1.0);
    context.set_stroke_style(rgbau(0, 0, 0, 92));
    context.set_shadow_color(rgbau(0, 0, 0, 92));
    context.set_shadow_blur(1.5 * hidpi_factor);
    context.set_shadow_offset(vec2f(0.0, 0.0));
    let shadow_path = create_rounded_rect_path(check_box_rect + vec2f(0.0, 1.0), CORNER_RADIUS);
    context.stroke_path(shadow_path);
    context.restore();

    context.set_font(FONT_NAME_EMOJI);
    context.set_font_size(17.0);
    context.set_fill_style(rgbau(255, 255, 255, 128));
    context.set_text_align(TextAlign::Center);
    context.fill_text("✔︎", check_box_rect.center());
}

fn draw_button(context: &mut CanvasRenderingContext2D,
               pre_icon: Option<&str>,
               text: &str,
               rect: RectF,
               color: ColorU) {
    const CORNER_RADIUS: f32 = 4.0;

    let path = create_rounded_rect_path(rect.contract(1.0), CORNER_RADIUS - 1.0);
    if color != ColorU::transparent_black() {
        context.set_fill_style(color);
        context.fill_path(path.clone(), FillRule::Winding);
    }
    let alpha = if color == ColorU::transparent_black() { 16 } else { 32 };
    let mut background_gradient = Gradient::linear_from_points(rect.origin(), rect.lower_left());
    background_gradient.add_color_stop(rgbau(255, 255, 255, alpha), 0.0);
    background_gradient.add_color_stop(rgbau(0,   0,   0,   alpha), 1.0);
    context.set_fill_style(background_gradient);
    context.fill_path(path, FillRule::Winding);

    context.set_stroke_style(rgbau(0, 0, 0, 48));
    context.stroke_path(create_rounded_rect_path(rect.contract(0.5), CORNER_RADIUS - 0.5));

    context.set_font(FONT_NAME_BOLD);
    context.set_font_size(17.0);
    let text_width = context.measure_text(text).width;

    let icon_width;
    match pre_icon {
        None => icon_width = 0.0,
        Some(icon) => {
            context.set_font_size(rect.height() * 0.7);
            context.set_font(FONT_NAME_EMOJI);
            icon_width = context.measure_text(icon).width + rect.height() * 0.15;
            context.set_fill_style(rgbau(255, 255, 255, 96));
            context.set_text_align(TextAlign::Left);
            context.set_text_baseline(TextBaseline::Middle);
            context.fill_text(icon,
                              rect.center() - vec2f(text_width * 0.5 + icon_width * 0.75, 0.0));
        }
    }

    context.set_font(FONT_NAME_BOLD);
    context.set_font_size(17.0);
    let text_origin = rect.center() + vec2f(icon_width * 0.25 - text_width * 0.5, 0.0);
    context.set_text_align(TextAlign::Left);
    context.set_text_baseline(TextBaseline::Middle);
    context.set_shadow_color(rgbau(0, 0, 0, 160));
    context.set_shadow_offset(vec2f(0.0, -1.0));
    context.set_shadow_blur(0.0);
    context.set_fill_style(rgbau(255, 255, 255, 160));
    context.fill_text(text, text_origin);
    context.set_shadow_color(ColorU::transparent_black());
}

fn draw_slider(context: &mut CanvasRenderingContext2D,
               value: f32,
               rect: RectF,
               hidpi_factor: f32) {
    let (center_y, knob_radius) = (rect.center().y().floor(), (rect.height() * 0.25).floor());

    context.save();

    // Draw track.
    context.save();
    let track_rect = RectF::new(vec2f(rect.origin_x(), center_y - 2.0), vec2f(rect.width(), 4.0));
    let track_path = create_rounded_rect_path(track_rect, 2.0);
    context.clip_path(track_path.clone(), FillRule::Winding);
    context.set_shadow_blur(2.0 * hidpi_factor);
    context.set_shadow_color(rgbau(0, 0, 0, 32));
    context.set_shadow_offset(vec2f(0.0, 1.0));
    context.set_fill_style(rgbau(0, 0, 0, 32));
    context.fill_path(track_path, FillRule::Winding);
    context.restore();

    // Fill knob.
    let knob_position = vec2f(rect.origin_x() + (value * rect.width()).floor(), center_y);
    let mut background_gradient =
        Gradient::linear_from_points(knob_position - vec2f(0.0, knob_radius),
                                     knob_position + vec2f(0.0, knob_radius));
    background_gradient.add_color_stop(rgbau(255, 255, 255, 16), 0.0);
    background_gradient.add_color_stop(rgbau(0,   0,   0,   16), 1.0);
    let mut path = Path2D::new();
    path.ellipse(knob_position, knob_radius - 1.0, 0.0, 0.0, PI_2);
    context.set_fill_style(rgbu(40, 43, 48));
    context.set_shadow_blur(6.0 * hidpi_factor);
    context.set_shadow_color(rgbau(0, 0, 0, 128));
    context.set_shadow_offset(vec2f(0.0, 1.0));
    context.fill_path(path.clone(), FillRule::Winding);
    context.set_shadow_color(rgbau(0, 0, 0, 0));
    context.set_fill_style(background_gradient);
    context.fill_path(path, FillRule::Winding);

    // Outline knob.
    let mut path = Path2D::new();
    path.ellipse(knob_position, knob_radius - 0.5, 0.0, 0.0, PI_2);
    context.set_stroke_style(rgbau(0, 0, 0, 92));
    context.stroke_path(path);

    context.restore();
}

fn draw_thumbnails(context: &mut CanvasRenderingContext2D,
                   rect: RectF,
                   time: f32,
                   hidpi_factor: f32,
                   image_count: usize,
                   image: &Image) {
    const CORNER_RADIUS: f32 = 3.0;
    const THUMB_HEIGHT: f32 = 60.0;
    const ARROW_Y_POSITION: f32 = 30.5;
    const IMAGES_ACROSS: usize = 4;

    let stack_height = image_count as f32 * 0.5 * (THUMB_HEIGHT + 10.0) + 10.0;
    let scroll_height = rect.height() / stack_height * (rect.height() - 8.0);
    let scroll_y = (1.0 + f32::cos(time * 0.5)) * 0.5;
    let load_y = (1.0 - f32::cos(time * 0.2)) * 0.5;
    let image_y_scale = 1.0 / (image_count as f32 - 1.0);

    context.save();

    // Draw window.
    let mut path = create_rounded_rect_path(rect, CORNER_RADIUS);
    path.move_to(rect.origin() + vec2f(-10.0, ARROW_Y_POSITION));
    path.line_to(rect.origin() + vec2f(1.0, ARROW_Y_POSITION - 11.0));
    path.line_to(rect.origin() + vec2f(1.0, ARROW_Y_POSITION + 11.0));
    context.set_fill_style(rgbu(200, 200, 200));
    context.set_shadow_blur(20.0 * hidpi_factor);
    context.set_shadow_offset(vec2f(0.0, 4.0));
    context.set_shadow_color(rgbau(0, 0, 0, 64));
    context.fill_path(path, FillRule::Winding);
    context.set_shadow_color(rgbau(0, 0, 0, 0));

    // Draw images.

    context.save();
    let mut clip_path = Path2D::new();
    clip_path.rect(rect);
    context.clip_path(clip_path, FillRule::Winding);
    context.translate(vec2f(0.0, -scroll_y * (stack_height - rect.height())));

    for image_index in 0..image_count {
        let image_origin = rect.origin() + vec2f(10.0, 10.0) +
            vec2i(image_index as i32 % 2, image_index as i32 / 2).to_f32() * (THUMB_HEIGHT + 10.0);
        let image_rect = RectF::new(image_origin, Vector2F::splat(THUMB_HEIGHT)); 

        // Draw shadow.
        let shadow_path = create_rounded_rect_path(image_rect.dilate(1.0) + vec2f(0.0, 1.0), 5.0);
        context.set_fill_style(rgbu(200, 200, 200));
        context.set_shadow_blur(3.0 * hidpi_factor);
        context.set_shadow_offset(vec2f(0.0, 0.0));
        context.set_shadow_color(rgbau(0, 0, 0, 255));
        context.fill_path(shadow_path, FillRule::Winding);
        context.set_shadow_color(rgbau(0, 0, 0, 0));

        let image_y = image_index as f32 * image_y_scale;
        let alpha = util::clamp((load_y - image_y) / image_y_scale, 0.0, 1.0);
        if alpha < 1.0 {
            draw_spinner(context, image_rect.center(), THUMB_HEIGHT * 0.25, time);
        }

        let image_path = create_rounded_rect_path(image_rect, 5.0);
        let image_coord = vec2i((image_index % IMAGES_ACROSS) as i32,
                                (image_index / IMAGES_ACROSS) as i32);
        let pattern_transform = Transform2F::from_translation(image_rect.origin()) *
            Transform2F::from_scale(0.5) *
            Transform2F::from_translation(-image_coord.to_f32() * (THUMB_HEIGHT * 2.0 + 2.0) -
                                          1.0);
        let mut pattern = Pattern::from_image((*image).clone());
        pattern.apply_transform(pattern_transform);
        context.set_fill_style(pattern);
        context.set_global_alpha(alpha);
        context.fill_path(image_path, FillRule::Winding);
        context.set_global_alpha(1.0);

        context.set_stroke_style(rgbau(255, 255, 255, 192));
        context.stroke_path(create_rounded_rect_path(image_rect.dilate(0.5), 3.5));
    }

    context.restore();

    // Draw fade-away gradients.

    let mut fade_gradient = Gradient::linear_from_points(rect.origin(),
                                                         rect.origin() + vec2f(0.0, 6.0));
    fade_gradient.add_color_stop(rgbau(200, 200, 200, 255), 0.0);
    fade_gradient.add_color_stop(rgbau(200, 200, 200, 0),   1.0);
    context.set_fill_style(fade_gradient);
    context.fill_rect(RectF::new(rect.origin() + vec2f(4.0, 0.0), vec2f(rect.width() - 8.0, 6.0)));

    let mut fade_gradient = Gradient::linear_from_points(rect.lower_left(),
                                                         rect.lower_left() - vec2f(0.0, 6.0));
    fade_gradient.add_color_stop(rgbau(200, 200, 200, 255), 0.0);
    fade_gradient.add_color_stop(rgbau(200, 200, 200, 0),   1.0);
    context.set_fill_style(fade_gradient);
    context.fill_rect(RectF::new(rect.lower_left() + vec2f(4.0, -6.0),
                                 vec2f(rect.width() - 8.0, 6.0)));

    // Draw scroll bar.

    context.save();
    let scroll_bar_rect = RectF::new(rect.upper_right() + vec2f(-12.0, 4.0),
                                     vec2f(8.0, rect.height() - 8.0));
    let path = create_rounded_rect_path(scroll_bar_rect, CORNER_RADIUS);
    context.set_fill_style(rgbau(0, 0, 0, 32));
    context.fill_path(path.clone(), FillRule::Winding);
    context.clip_path(path, FillRule::Winding);
    context.set_stroke_style(rgbau(0, 0, 0, 92));
    context.set_shadow_offset(vec2f(0.0, 0.0));
    context.set_shadow_color(rgbau(0, 0, 0, 92));
    context.set_shadow_blur(4.0 * hidpi_factor);
    let shadow_path = create_rounded_rect_path(scroll_bar_rect + vec2f(0.0, 1.0), CORNER_RADIUS);
    context.stroke_path(shadow_path);
    context.set_shadow_color(rgbau(0, 0, 0, 0));
    context.restore();

    let knob_rect = RectF::new(
        scroll_bar_rect.origin() + vec2f(0.0, (rect.height() - 8.0 - scroll_height) * scroll_y),
        vec2f(8.0, scroll_height));
    context.set_fill_style(rgbu(220, 220, 220));
    let path = create_rounded_rect_path(knob_rect.contract(1.0), 3.0);
    context.fill_path(path.clone(), FillRule::Winding);
    context.clip_path(path, FillRule::Winding);
    context.set_stroke_style(rgbu(128, 128, 128));
    context.set_line_width(1.0);
    let shadow_path = create_rounded_rect_path(knob_rect, 3.0);
    context.set_shadow_blur(2.0 * hidpi_factor);
    context.set_shadow_color(rgbu(128, 128, 128));
    context.set_shadow_offset(vec2f(0.0, 0.0));
    context.stroke_path(shadow_path);

    context.restore();
}

fn draw_spinner(context: &mut CanvasRenderingContext2D, center: Vector2F, radius: f32, time: f32) {
    let (start_angle, end_angle) = (time * 6.0, PI + time * 6.0);
    let (outer_radius, inner_radius) = (radius, radius * 0.75);
    let average_radius = util::lerp(outer_radius, inner_radius, 0.5);

    context.save();

    let mut path = Path2D::new();
    path.arc(center, outer_radius, start_angle, end_angle, ArcDirection::CW);
    path.arc(center, inner_radius, end_angle, start_angle, ArcDirection::CCW);
    path.close_path();
    set_linear_gradient_fill_style(
        context,
        center + vec2f(outer_radius.cos(), outer_radius.sin()) * average_radius,
        center + vec2f(inner_radius.cos(), inner_radius.sin()) * average_radius,
        rgbau(0, 0, 0, 0),
        rgbau(0, 0, 0, 128));
    context.fill_path(path, FillRule::Winding);

    context.restore();
}

struct PerfGraph {
    style: GraphStyle,
    values: VecDeque<f32>,
    name: &'static str,
}

impl PerfGraph {
    fn new(style: GraphStyle, name: &'static str) -> PerfGraph {
        PerfGraph { style, name, values: VecDeque::new() }
    }

    fn push(&mut self, frame_time: f32) {
        if self.values.len() == GRAPH_HISTORY_COUNT {
            self.values.pop_front();
        }
        self.values.push_back(frame_time);
    }

    fn render(&self, context: &mut CanvasRenderingContext2D, origin: Vector2F) {
        let rect = RectF::new(origin, vec2f(GRAPH_WIDTH, GRAPH_HEIGHT));
        context.set_fill_style(rgbau(0, 0, 0, 128));
        context.fill_rect(rect);

        let mut path = Path2D::new();
        path.move_to(rect.lower_left());

        let scale = vec2f(rect.width() / (GRAPH_HISTORY_COUNT as f32 - 1.0), rect.height());
        for (index, value) in self.values.iter().enumerate() {
            let mut value = *value;
            if self.style == GraphStyle::FPS && value != 0.0 {
                value = 1.0 / value;
            }
            value = (value * self.style.scale()).min(self.style.max());
            let point = rect.lower_left() + vec2f(index as f32, -value / self.style.max()) * scale;
            path.line_to(point);
        }

        path.line_to(rect.lower_left() + vec2f(self.values.len() as f32 - 1.0, 0.0) * scale);
        context.set_fill_style(rgbau(255, 192, 0, 128));
        context.fill_path(path, FillRule::Winding);

        context.set_font(FONT_NAME_REGULAR);
        context.set_text_baseline(TextBaseline::Top);

        if !self.name.is_empty() {
            context.set_font_size(12.0);
            context.set_text_align(TextAlign::Left);
            context.set_fill_style(rgbau(240, 240, 240, 192));
            context.fill_text(self.name, origin + vec2f(3.0, 3.0));
        }

        context.set_font_size(15.0);
        context.set_text_align(TextAlign::Right);
        context.set_fill_style(rgbau(240, 240, 240, 255));
        self.draw_label(context, self.style, rect.upper_right() + vec2f(-3.0, 3.0));

        if self.style == GraphStyle::FPS {
            context.set_text_baseline(TextBaseline::Alphabetic);
            context.set_fill_style(rgbau(240, 240, 240, 160));
            self.draw_label(context, GraphStyle::MS, rect.lower_right() + vec2f(-3.0, -3.0));
        }
    }

    fn draw_label(&self,
                  context: &mut CanvasRenderingContext2D,
                  style: GraphStyle,
                  origin: Vector2F) {
        let mut average = self.average();
        if style == GraphStyle::FPS && average != 0.0 {
            average = 1.0 / average;
        }
        average *= style.scale();
        context.fill_text(&format!("{}{}", average, style.label()), origin);
    }

    fn average(&self) -> f32 {
        let mut sum: f32 = self.values.iter().sum();
        if !self.values.is_empty() {
            sum /= self.values.len() as f32;
        }
        sum
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
enum GraphStyle {
    FPS,
    MS,
}

impl GraphStyle {
    fn scale(self) -> f32 {
        match self {
            GraphStyle::FPS => 1.0,
            GraphStyle::MS => 1000.0,
        }
    }

    fn max(self) -> f32 {
        match self {
            GraphStyle::MS => 20.0,
            GraphStyle::FPS => 80.0,
        }
    }

    fn label(self) -> &'static str {
        match self {
            GraphStyle::FPS => " FPS",
            GraphStyle::MS => " ms",
        }
    }
}

fn set_linear_gradient_fill_style(context: &mut CanvasRenderingContext2D,
                                  from_position: Vector2F,
                                  to_position: Vector2F,
                                  from_color: ColorU,
                                  to_color: ColorU) {
    let mut gradient = Gradient::linear(LineSegment2F::new(from_position, to_position));
    gradient.add_color_stop(from_color, 0.0);
    gradient.add_color_stop(to_color, 1.0);
    context.set_fill_style(gradient);
}

fn create_graph_path(sample_points: &[Vector2F], sample_spread: f32, offset: Vector2F) -> Path2D {
    let mut path = Path2D::new();
    path.move_to(sample_points[0] + vec2f(0.0, 2.0));
    for pair in sample_points.windows(2) {
        path.bezier_curve_to(pair[0] + offset + vec2f(sample_spread * 0.5, 0.0),
                             pair[1] + offset - vec2f(sample_spread * 0.5, 0.0),
                             pair[1] + offset);
    }
    path
}

fn create_rounded_rect_path(rect: RectF, radius: f32) -> Path2D {
    let mut path = Path2D::new();
    path.move_to(rect.origin() + vec2f(radius, 0.0));
    path.arc_to(rect.upper_right(), rect.upper_right() + vec2f(0.0,  radius), radius);
    path.arc_to(rect.lower_right(), rect.lower_right() + vec2f(-radius, 0.0), radius);
    path.arc_to(rect.lower_left(),  rect.lower_left()  + vec2f(0.0, -radius), radius);
    path.arc_to(rect.origin(),      rect.origin()      + vec2f(radius,  0.0), radius);
    path.close_path();
    path
}

struct DemoData {
    image: Image,
}

impl DemoData {
    fn load(resources: &dyn ResourceLoader) -> DemoData {
        let data = resources.slurp("textures/example-nanovg.png").unwrap();
        let image = image::load_from_memory(&data).unwrap().to_rgba();
        let image = Image::from_image_buffer(image);
        DemoData { image }
    }
}

fn main() {
    // Set up SDL2.
    let sdl_context = sdl2::init().unwrap();
    let video = sdl_context.video().unwrap();

    // Make sure we have at least a GL 3.0 context. Pathfinder requires this.
    let gl_attributes = video.gl_attr();
    gl_attributes.set_context_profile(GLProfile::Core);
    gl_attributes.set_context_version(3, 3);

    // Open a window.
    let window_size = vec2i(WINDOW_WIDTH, WINDOW_HEIGHT);
    let window =
        video.window("NanoVG example port", window_size.x() as u32, window_size.y() as u32)
             .opengl()
             .allow_highdpi()
             .build()
             .unwrap();

    // Create the GL context, and make it current.
    let gl_context = window.gl_create_context().unwrap();
    gl::load_with(|name| video.gl_get_proc_address(name) as *const _);
    window.gl_make_current(&gl_context).unwrap();

    // Get the real window size (for HiDPI).
    let (drawable_width, drawable_height) = window.drawable_size();
    let drawable_size = vec2i(drawable_width as i32, drawable_height as i32);
    let hidpi_factor = drawable_size.x() as f32 / window_size.x() as f32;

    // Load demo data.
    let resources = FilesystemResourceLoader::locate();
    let font_data = vec![
        Handle::from_memory(Arc::new(resources.slurp("fonts/Roboto-Regular.ttf").unwrap()), 0),
        Handle::from_memory(Arc::new(resources.slurp("fonts/Roboto-Bold.ttf").unwrap()), 0),
        Handle::from_memory(Arc::new(resources.slurp("fonts/NotoEmoji-Regular.ttf").unwrap()), 0),
    ];
    let demo_data = DemoData::load(&resources);

    // Create a Pathfinder renderer.
    let mut renderer = Renderer::new(GLDevice::new(GLVersion::GL3, 0),
                                     &resources,
                                     DestFramebuffer::full_window(drawable_size),
                                     RendererOptions {
                                         background_color: Some(rgbf(0.3, 0.3, 0.32)),
                                     });

    // Initialize font state.
    let font_source = Arc::new(MemSource::from_fonts(font_data.into_iter()).unwrap());
    let font_context = CanvasFontContext::new(font_source.clone());

    // Initialize general state.
    let mut event_pump = sdl_context.event_pump().unwrap();
    let mut mouse_position = Vector2F::zero();
    let start_time = Instant::now();

    // Initialize performance graphs.
    let mut fps_graph = PerfGraph::new(GraphStyle::FPS, "Frame Time");
    let mut cpu_graph = PerfGraph::new(GraphStyle::MS, "CPU Time");
    let mut gpu_graph = PerfGraph::new(GraphStyle::MS, "GPU Time");

    // Enter the main loop.
    loop {
        // Make a canvas.
        let mut context = Canvas::new(drawable_size.to_f32()).get_context_2d(font_context.clone());

        // Start performance timing.
        let frame_start_time = Instant::now();
        let frame_start_elapsed_time = (frame_start_time - start_time).as_secs_f32();

        // Render the demo.
        context.scale(hidpi_factor);
        render_demo(&mut context,
                    mouse_position,
                    window_size.to_f32(),
                    frame_start_elapsed_time,
                    hidpi_factor,
                    &demo_data);

        // Render performance graphs.
        let cpu_frame_elapsed_time = (Instant::now() - frame_start_time).as_secs_f32();
        fps_graph.render(&mut context, vec2f(5.0, 5.0));
        cpu_graph.render(&mut context, vec2f(210.0, 5.0));
        gpu_graph.render(&mut context, vec2f(415.0, 5.0));

        // Render the canvas to screen.
        let canvas = context.into_canvas();
        let scene = SceneProxy::from_scene(canvas.into_scene(), RayonExecutor);
        scene.build_and_render(&mut renderer, BuildOptions::default());
        window.gl_swap_window();

        // Add stats to performance graphs.
        if let Some(gpu_time) = renderer.shift_rendering_time() {
            let cpu_build_time = renderer.stats.cpu_build_time.as_secs_f32();
            let gpu_time = gpu_time.gpu_time.as_secs_f32();
            fps_graph.push(cpu_frame_elapsed_time + cpu_build_time.max(gpu_time));
            cpu_graph.push(cpu_frame_elapsed_time + cpu_build_time);
            gpu_graph.push(gpu_time);
        }

        for event in event_pump.poll_iter() {
            match event {
                Event::Quit {..} | Event::KeyDown { keycode: Some(Keycode::Escape), .. } => return,
                Event::MouseMotion { x, y, .. } => mouse_position = vec2i(x, y).to_f32(),
                _ => {}
            }
        }
    }
}
