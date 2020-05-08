// pathfinder/swf/src/lib.rs
//
// Copyright © 2019 The Pathfinder Project Developers.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use std::ops::Add;
use pathfinder_color::{ColorF, ColorU};
use pathfinder_content::fill::FillRule;
use pathfinder_content::outline::{Outline, Contour};
use pathfinder_content::stroke::{OutlineStrokeToFill, StrokeStyle};
use pathfinder_geometry::vector::vec2f;
use pathfinder_renderer::scene::{DrawPath, Scene};

use swf_types::tags::SetBackgroundColor;
use swf_types::{Tag, SRgb8, Movie};

use crate::shapes::{GraphicLayers, PaintOrLine};

mod shapes;

type SymbolId = u16;

// In swf, most values are specified in a fixed point format known as "twips" or twentieths of
// a pixel.  We store twips in their integer form, as if we were to convert them to floating point
// at the beginning of the pipeline it's easy to start running into precision errors when we add
// coordinate deltas and then try and compare coords for equality.

#[derive(Copy, Clone, Debug, PartialEq)]
struct Twips(i32);

impl Twips {
    // Divide twips by 20 to get the f32 value, just to be used once all processing
    // of the swf coords is completed and we want to output.
    fn as_f32(&self) -> f32 {
        self.0 as f32 / 20.0
    }
}

impl Add for Twips {
    type Output = Twips;
    fn add(self, rhs: Twips) -> Self {
        Twips(self.0 + rhs.0)
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
struct Point2<T> {
    x: T,
    y: T
}

impl Point2<Twips> {
    fn as_f32(self: Point2<Twips>) -> Point2<f32> {
        Point2 {
            x: self.x.as_f32(),
            y: self.y.as_f32(),
        }
    }
}

impl Add for Point2<Twips> {
    type Output = Self;
    fn add(self, rhs: Self) -> Self {
        Point2 { x: self.x + rhs.x, y: self.y + rhs.y }
    }
}

enum Symbol {
    Graphic(GraphicLayers),
    // Timeline, // TODO(jon)
}

pub struct Stage {
    // TODO(jon): Support some kind of lazy frames iterator.
    // frames: Timeline,
    background_color: SRgb8,
    width: i32,
    height: i32,
}

impl Stage {
    pub fn width(&self) -> i32 {
        self.width
    }

    pub fn height(&self) -> i32 {
        self.height
    }

    pub fn background_color(&self) -> ColorF {
        ColorU {
            r: self.background_color.r,
            g: self.background_color.g,
            b: self.background_color.b,
            a: 255,
        }.to_f32()
    }
}


pub struct SymbolLibrary(Vec<Symbol>);

impl SymbolLibrary {
    fn add_symbol(&mut self, symbol: Symbol) {
        self.0.push(symbol);
    }

    fn symbols(&self) -> &Vec<Symbol> {
        &self.0
    }
}

pub fn process_swf_tags(movie: &Movie) -> (SymbolLibrary, Stage) {
    let mut symbol_library = SymbolLibrary(Vec::new());
    let stage_width = Twips(movie.header.frame_size.x_max);
    let stage_height = Twips(movie.header.frame_size.y_max);
    // let num_frames = movie.header.frame_count;

    let mut stage = Stage {
        // frames: Timeline(Vec::new()), // TODO(jon)
        background_color: SRgb8 {
            r: 255,
            g: 255,
            b: 255
        },
        width: stage_width.as_f32() as i32,
        height: stage_height.as_f32() as i32,
    };

    for tag in &movie.tags {
        match tag {
            Tag::SetBackgroundColor(SetBackgroundColor { color }) => {
                stage.background_color = *color;
            },
            Tag::DefineShape(shape) => {
                symbol_library.add_symbol(Symbol::Graphic(shapes::decode_shape(shape)));
                // We will assume that symbol ids just go up, and are 1 based.
                let symbol_id: SymbolId = shape.id;
                debug_assert!(symbol_id as usize == symbol_library.0.len());
            }
            _ => ()
        }
    }
    (symbol_library, stage)
}

#[allow(irrefutable_let_patterns)]
pub fn draw_paths_into_scene(library: &SymbolLibrary, scene: &mut Scene) {
    for symbol in library.symbols() {
        // NOTE: Right now symbols only contain graphics.
        if let Symbol::Graphic(graphic) = symbol {
            for style_layer in graphic.layers() {
                let mut path = Outline::new();
                let paint_id = scene.push_paint(&style_layer.fill());

                for shape in style_layer.shapes() {
                    let mut contour = Contour::new();
                    let Point2 { x, y } = shape.outline.first().unwrap().from.as_f32();
                    contour.push_endpoint(vec2f(x, y));
                    for segment in &shape.outline {
                        let Point2 { x, y } = segment.to.as_f32();
                        match segment.ctrl {
                            Some(ctrl) => {
                                let Point2 { x: ctrl_x, y: ctrl_y } = ctrl.as_f32();
                                contour.push_quadratic(vec2f(ctrl_x, ctrl_y), vec2f(x, y));
                            }
                            None => {
                                contour.push_endpoint(vec2f(x, y));
                            },
                        }
                    }
                    if shape.is_closed() {
                        // NOTE: I'm not sure if this really does anything in this context,
                        // since all our closed shapes already have coincident start and end points.
                        contour.close();
                    }
                    path.push_contour(contour);
                }

                if let PaintOrLine::Line(line) = style_layer.kind() {
                    let mut stroke_to_fill = OutlineStrokeToFill::new(&path, StrokeStyle {
                        line_width: line.width.as_f32(),
                        line_cap: line.cap,
                        line_join: line.join,
                    });
                    stroke_to_fill.offset();
                    path = stroke_to_fill.into_outline();
                }

                let mut path = DrawPath::new(path, paint_id);
                path.set_fill_rule(FillRule::EvenOdd);
                scene.push_path(path);
            }
        }
    }
}
