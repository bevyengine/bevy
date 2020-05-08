// pathfinder/swf/src/shapes.rs
//
// Copyright © 2019 The Pathfinder Project Developers.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use crate::{Twips, Point2};

use pathfinder_color::ColorU;
use pathfinder_content::stroke::{LineJoin, LineCap};
use pathfinder_renderer::paint::Paint;
use std::cmp::Ordering;
use std::mem;
use swf_types::tags::DefineShape;
use swf_types::{CapStyle, FillStyle, JoinStyle, LineStyle, ShapeRecord, StraightSRgba8, Vector2D};
use swf_types::{fill_styles, join_styles, shape_records};

#[derive(Clone, Copy, Debug)]
pub(crate) struct LineSegment {
    pub(crate) from: Point2<Twips>,
    pub(crate) to: Point2<Twips>,
    pub(crate) ctrl: Option<Point2<Twips>>,
}

impl LineSegment {
    fn reverse(&mut self) {
        let tmp = self.from;
        self.from = self.to;
        self.to = tmp;
    }
}

#[derive(Copy, Clone, PartialEq, Debug)]
pub(crate) enum LineDirection {
    Left,
    Right,
}

impl LineDirection {
    fn reverse(&mut self) {
        *self = match self {
            LineDirection::Right => LineDirection::Left,
            LineDirection::Left => LineDirection::Right
        };
    }
}

#[derive(Clone, Debug)]
pub(crate) struct Shape {
    pub(crate) outline: Vec<LineSegment>, // Could be Vec<(start, end)>
    direction: LineDirection,
    reversed: bool,
}

impl Shape {
    pub fn new_with_direction(direction: LineDirection) -> Shape {
        Shape {
            direction,
            outline: Vec::new(),
            reversed: false,
        }
    }

    fn prepend_shape(&mut self, shape: &mut Shape) {
        shape.append_shape(&self);
        mem::swap(&mut self.outline, &mut shape.outline);
    }

    fn append_shape(&mut self, shape: &Shape) {
        self.outline.extend_from_slice(&shape.outline);
    }

    fn add_line_segment(&mut self, segment: LineSegment) {
        self.outline.push(segment);
    }

    #[inline]
    fn len(&self) -> usize {
        self.outline.len()
    }

    #[inline]
    fn first(&self) -> LineSegment {
        *self.outline.first().unwrap()
    }

    #[inline]
    fn last(&self) -> LineSegment {
        *self.outline.last().unwrap()
    }

    #[inline]
    fn comes_before(&self, other: &Shape) -> bool {
        self.last().to == other.first().from
    }

    #[inline]
    fn comes_after(&self, other: &Shape) -> bool {
        self.first().from == other.last().to
    }

    #[inline]
    pub(crate) fn is_closed(&self) -> bool {
        self.len() > 1 && self.comes_after(self)
    }

    fn reverse(&mut self) {
        self.reversed = !self.reversed;
        self.direction.reverse();
        for segment in &mut self.outline {
            segment.reverse();
        }
        self.outline.reverse();
    }
}

pub(crate) struct SwfLineStyle {
    color: Paint,
    pub(crate) width: Twips,
    pub(crate) join: LineJoin,
    pub(crate) cap: LineCap,
}

pub(crate) enum PaintOrLine {
    Paint(Paint),
    Line(SwfLineStyle),
}

pub(crate) struct StyleLayer {
    fill: PaintOrLine,
    // TODO(jon): Maybe shapes are actually slices into a single buffer, then we don't
    // need to realloc anything, we're just shuffling shapes around?
    shapes: Vec<Shape>,
}

impl StyleLayer {
    pub(crate) fn kind(&self) -> &PaintOrLine {
        &self.fill
    }

    fn is_fill(&self) -> bool {
        match &self.fill {
            PaintOrLine::Paint(_) => true,
            PaintOrLine::Line(_) => false,
        }
    }

    pub(crate) fn fill(&self) -> &Paint {
        match &self.fill {
            PaintOrLine::Paint(ref paint) => paint,
            PaintOrLine::Line(line) => &line.color,
        }
    }

    fn push_new_shape(&mut self, direction: LineDirection) {
        if let Some(prev_shape) = self.shapes.last_mut() {
            // Check that the previous shape was actually used, otherwise reuse it.
            if prev_shape.len() != 0 {
                self.shapes.push(Shape::new_with_direction(direction))
            } else {
                prev_shape.direction = direction;
            }
        } else {
            self.shapes.push(Shape::new_with_direction(direction))
        }
    }

    pub(crate) fn shapes(&self) -> &Vec<Shape> {
        &self.shapes
    }

    fn shapes_mut(&mut self) -> &mut Vec<Shape> {
        &mut self.shapes
    }

    fn current_shape_mut(&mut self) -> &mut Shape {
        self.shapes.last_mut().unwrap()
    }

    fn consolidate_edges(&mut self) {
        // Reverse left fill shape fragments in place.
        {
            self.shapes
                .iter_mut()
                .filter(|frag| frag.direction == LineDirection::Left)
                .for_each(|frag| frag.reverse());
        }

        // Sort shapes into [closed...open]
        if self.is_fill() {
            // I think sorting is only necessary when we want to have closed shapes,
            // lines don't really need this?
            self.shapes.sort_unstable_by(|a, b| {
                match (a.is_closed(), b.is_closed()) {
                    (true, true) | (false, false) => Ordering::Equal,
                    (true, false) => Ordering::Less,
                    (false, true) => Ordering::Greater,
                }
            });
        }

        // A cursor at the index of the first unclosed shape, if any.
        let first_open_index = self.shapes
            .iter()
            .position(|frag| !frag.is_closed());

        if let Some(first_open_index) = first_open_index {
            if self.shapes.len() - first_open_index >= 2 {
                // TODO(jon): This might be sped up by doing it in a way that we don't have
                // to allocate more vecs?
                // Also, maybe avoid path reversal, and just flag the path as reversed and iterate it
                // backwards.
                let unmatched_pieces = find_matches(first_open_index, &mut self.shapes, false);
                if let Some(mut unmatched_pieces) = unmatched_pieces {
                    if self.is_fill() {
                        // If they didn't match before, they're probably parts of inner shapes
                        // and should be reversed again so they have correct winding
                        let unclosed = find_matches(0, &mut unmatched_pieces, true);
                        // If it's a shape we should always be able to close it.
                        debug_assert!(unclosed.is_none());
                    }
                    for dropped in &mut unmatched_pieces {
                        dropped.reverse();
                    }
                    self.shapes.extend_from_slice(&unmatched_pieces);
                }
                // FIXME(jon): Sometimes we don't get the correct winding of internal closed shapes,
                // need to figure out why this happens.
            }
        }
    }
}


fn get_new_styles<'a>(
    fills: &'a Vec<FillStyle>,
    lines: &'a Vec<LineStyle>
) -> impl Iterator<Item=PaintOrLine> + 'a {
    // This enforces the order that fills and line groupings are added in.
    // Fills always come first.
    fills.iter().filter_map(|fill_style| {
        match fill_style {
            FillStyle::Solid(
                fill_styles::Solid {
                    color: StraightSRgba8 {
                        r,
                        g,
                        b,
                        a
                    }
                }
            ) => {
                Some(PaintOrLine::Paint(Paint::from_color(ColorU { r: *r, g: *g, b: *b, a: *a })))
            }
            _ => unimplemented!("Unimplemented fill style")
        }
    }).chain(
        lines.iter().filter_map(|LineStyle {
            width,
            fill,
            join,
            start_cap,
            end_cap: _,
            /*
            TODO(jon): Handle these cases?
            pub no_h_scale: bool,
            pub no_v_scale: bool,
            pub no_close: bool,
            pub pixel_hinting: bool,
            */
            ..
        }| {
            if let FillStyle::Solid(fill_styles::Solid {
                color: StraightSRgba8 {
                    r,
                    g,
                    b,
                    a
                }
            }) = fill {
                // NOTE: PathFinder doesn't support different cap styles for start and end of
                // strokes, so lets assume that they're always the same for the inputs we care about.
                // Alternately, we split a line in two with a diff cap style for each.
                // assert_eq!(start_cap, end_cap);
                Some(PaintOrLine::Line(SwfLineStyle {
                    width: Twips(*width as i32),
                    color: Paint::from_color(ColorU { r: *r, g: *g, b: *b, a: *a }),
                    join: match join {
                        JoinStyle::Bevel => LineJoin::Bevel,
                        JoinStyle::Round => LineJoin::Round,
                        JoinStyle::Miter(join_styles::Miter { limit }) => {
                            LineJoin::Miter(*limit as f32)
                        },
                    },
                    cap: match start_cap {
                        CapStyle::None => LineCap::Butt,
                        CapStyle::Square => LineCap::Square,
                        CapStyle::Round => LineCap::Round,
                    },
                }))
            } else {
                unimplemented!("unimplemented line fill style");
            }
        })
    )
}

pub(crate) fn decode_shape(shape: &DefineShape) -> GraphicLayers {
    let DefineShape {
        shape,
        // id,
        // has_fill_winding, NOTE(jon): Could be important for some inputs?
        // has_non_scaling_strokes,
        // has_scaling_strokes,
        ..
    } = shape;
    let mut graphic = GraphicLayers::new();
    let mut current_line_style = None;
    let mut current_left_fill = None;
    let mut current_right_fill = None;
    let mut prev_pos = None;

    let mut some_fill_set = false;
    let mut both_fills_set;
    let mut both_fills_same = false;
    let mut both_fills_set_and_same = false;

    // Create style groups for initially specified fills and lines.
    for fills_or_line in get_new_styles(&shape.initial_styles.fill, &shape.initial_styles.line) {
        match fills_or_line {
            PaintOrLine::Paint(fill) => graphic.begin_fill_style(fill),
            PaintOrLine::Line(line) => graphic.begin_line_style(line),
        }
    }

    for record in &shape.records {
        match record {
            ShapeRecord::StyleChange(
                shape_records::StyleChange {
                    move_to,
                    new_styles,
                    line_style,
                    left_fill,
                    right_fill,
                }
            ) => {
                // Start a whole new style grouping.
                if let Some(new_style) = new_styles {
                    // Consolidate current style grouping and begin a new one.
                    graphic.end_style_group();
                    graphic.begin_style_group();
                    for fills_or_line in get_new_styles(&new_style.fill, &new_style.line) {
                        match fills_or_line {
                            PaintOrLine::Paint(fill) => graphic.begin_fill_style(fill),
                            PaintOrLine::Line(line) => graphic.begin_line_style(line),
                        }
                    }
                }

                // If there's a change in right fill
                if let Some(fill_id) = right_fill {
                    if *fill_id == 0 {
                        current_right_fill = None;
                    } else {
                        current_right_fill = Some(*fill_id);
                        graphic
                            .with_fill_style_mut(*fill_id)
                            .unwrap()
                            .push_new_shape(LineDirection::Right);
                    }
                }
                // If there's a change in left fill
                if let Some(fill_id) = left_fill {
                    if *fill_id == 0 {
                        current_left_fill = None;
                    } else {
                        current_left_fill = Some(*fill_id);
                        graphic
                            .with_fill_style_mut(*fill_id)
                            .unwrap()
                            .push_new_shape(LineDirection::Left);
                    }
                }

                some_fill_set = current_left_fill.is_some() || current_right_fill.is_some();
                both_fills_set = current_left_fill.is_some() && current_right_fill.is_some();
                both_fills_same = current_left_fill == current_right_fill;
                both_fills_set_and_same = both_fills_set && both_fills_same;

                // If there's a change in line style
                if let Some(style_id) = line_style {
                    if *style_id == 0 {
                        current_line_style = None;
                    } else {
                        current_line_style = Some(*style_id);
                        graphic
                            .with_line_style_mut(*style_id)
                            .unwrap()
                            .push_new_shape(LineDirection::Right);
                    }
                }

                // Move to, start new shape fragments with the current styles.
                if let Some(Vector2D { x, y }) = move_to {
                    let to: Point2<Twips> = Point2 { x: Twips(*x), y: Twips(*y) };
                    prev_pos = Some(to);

                    // If we didn't start a new shape for the current fill due to a fill
                    // style change earlier, we definitely want to start a new shape now,
                    // since each move_to command indicates a new shape fragment.
                    if let Some(current_right_fill) = current_right_fill {
                        graphic
                            .with_fill_style_mut(current_right_fill)
                            .unwrap()
                            .push_new_shape(LineDirection::Right);
                    }
                    if let Some(current_left_fill) = current_left_fill {
                        graphic
                            .with_fill_style_mut(current_left_fill)
                            .unwrap()
                            .push_new_shape(LineDirection::Left);
                    }
                    if let Some(current_line_style) = current_line_style {
                        // TODO(jon): Does the direction of this line depend on the current
                        // fill directions?
                        graphic
                            .with_line_style_mut(current_line_style)
                            .unwrap()
                            .push_new_shape(LineDirection::Right);
                    }
                }
            },
            ShapeRecord::Edge(
                shape_records::Edge {
                    delta,
                    control_delta,
                }
            ) => {
                let from = prev_pos.unwrap();
                let to = Point2 {
                    x: from.x + Twips(delta.x),
                    y: from.y + Twips(delta.y)
                };
                prev_pos = Some(to);
                let new_segment = LineSegment {
                    from,
                    to,
                    ctrl: control_delta.map(|Vector2D { x, y }| {
                        Point2 {
                            x: from.x + Twips(x),
                            y: from.y + Twips(y),
                        }
                    }),
                };
                if some_fill_set && !both_fills_same {
                    for fill_id in [
                        current_right_fill,
                        current_left_fill
                    ].iter() {
                        if let Some(fill_id) = fill_id {
                            graphic
                                .with_fill_style_mut(*fill_id)
                                .unwrap()
                                .current_shape_mut()
                                .add_line_segment(new_segment);
                        }
                    }
                } else if both_fills_set_and_same {
                    for (fill_id, direction) in [
                        (current_right_fill, LineDirection::Right),
                        (current_left_fill, LineDirection::Left)
                    ].iter() {
                        // NOTE: If both left and right fill are set the same,
                        // then we don't record the edge as part of the current shape;
                        // it's will just be an internal stroke inside an otherwise solid
                        // shape, and recording these edges as part of the shape means that
                        // we can't determine the closed shape outline later.
                        if let Some(fill_id) = fill_id {
                            graphic
                                .with_fill_style_mut(*fill_id)
                                .unwrap()
                                .push_new_shape(*direction);
                        }
                    }
                }
                if let Some(current_line_style) = current_line_style {
                    graphic
                        .with_line_style_mut(current_line_style)
                        .unwrap()
                        .current_shape_mut()
                        .add_line_segment(new_segment);
                }
            }
        }
    }
    // NOTE: Consolidate current group of styles, joining edges of shapes/strokes where
    // possible and forming closed shapes.  In swf, all filled shapes should always be closed,
    // so there will always be a solution for joining shape line segments together so that
    // the start point and end point are coincident.
    graphic.end_style_group();
    graphic
}

fn find_matches(
    mut first_open_index: usize,
    shapes: &mut Vec<Shape>,
    reverse: bool
) -> Option<Vec<Shape>> {
    let mut dropped_pieces = None;
    while first_open_index < shapes.len() {
        // Take the last unclosed value, and try to join it onto
        // one of the other unclosed values.
        let mut last = shapes.pop().unwrap();
        if reverse {
            last.reverse();
        }
        let mut found_match = false;
        for i in first_open_index..shapes.len() {
            let fragment = &mut shapes[i];
            if last.comes_after(fragment) {
                // NOTE(jon): We do realloc quite a bit here, I wonder if it's worth trying
                // to avoid that?  Could do it with another level of indirection, where an outline
                // is a list of fragments.

                // println!("app ({}, {})", last.reversed, fragment.reversed);
                fragment.append_shape(&last);
                found_match = true;
            } else if last.comes_before(fragment) {
                // println!("pre ({}, {})", last.reversed, fragment.reversed);
                fragment.prepend_shape(&mut last);
                found_match = true;
            }
            if found_match {
                if fragment.is_closed() {
                    // Move the shape that was just closed to the left side of the current slice,
                    // and advance the cursor.
                    shapes.swap(first_open_index, i);
                    first_open_index += 1;
                }
                break;
            }
        }
        if !found_match {
            // Have we tried matching a reversed version of this segment?
            // move last back onto the array, it will never be closed, presumably because
            // it's a set of line segments rather than a shape that needs to be closed.
            let dropped_pieces: &mut Vec<Shape> = dropped_pieces.get_or_insert(Vec::new());
            dropped_pieces.push(last);
        }
    }
    dropped_pieces
}

pub(crate) struct GraphicLayers {
    style_layers: Vec<StyleLayer>,
    base_layer_offset: usize,
    stroke_layer_offset: Option<usize>,
}

impl GraphicLayers {
    fn new() -> GraphicLayers {
        GraphicLayers { style_layers: Vec::new(), stroke_layer_offset: None, base_layer_offset: 0 }
    }

    fn begin_style_group(&mut self) {
        self.stroke_layer_offset = None;
        self.base_layer_offset = self.style_layers.len();
    }

    fn begin_fill_style(&mut self, fill: Paint) {
        self.style_layers.push(StyleLayer { fill: PaintOrLine::Paint(fill), shapes: Vec::new() })
    }

    fn begin_line_style(&mut self, line: SwfLineStyle) {
        if self.stroke_layer_offset.is_none() {
            self.stroke_layer_offset = Some(self.style_layers.len());
        }
        self.style_layers.push(StyleLayer { fill: PaintOrLine::Line(line), shapes: Vec::new() })
    }

    fn with_fill_style_mut(&mut self, fill_id: usize) -> Option<&mut StyleLayer> {
        self.style_layers.get_mut(self.base_layer_offset + fill_id - 1)
    }

    fn with_line_style_mut(&mut self, line_id: usize) -> Option<&mut StyleLayer> {
        self.style_layers.get_mut((self.stroke_layer_offset.unwrap() + line_id) - 1)
    }

    pub(crate) fn layers(&self) -> &Vec<StyleLayer> {
        &self.style_layers
    }

    fn end_style_group(&mut self) {
        for style_layer in &mut self.style_layers[self.base_layer_offset..] {
            // There can be an unused style group at the end of each layer, which we should remove.
            if let Some(last) = style_layer.shapes().last() {
                if last.len() == 0 {
                    style_layer.shapes_mut().pop();
                }
            }
            style_layer.consolidate_edges();
        }
    }
}

