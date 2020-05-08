// pathfinder/svg/src/lib.rs
//
// Copyright © 2019 The Pathfinder Project Developers.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Converts a subset of SVG to a Pathfinder scene.

#[macro_use]
extern crate bitflags;

use hashbrown::HashMap;
use pathfinder_color::ColorU;
use pathfinder_content::dash::OutlineDash;
use pathfinder_content::fill::FillRule;
use pathfinder_content::gradient::{ColorStop, Gradient};
use pathfinder_content::outline::Outline;
use pathfinder_content::segment::{Segment, SegmentFlags};
use pathfinder_content::stroke::{LineCap, LineJoin, OutlineStrokeToFill, StrokeStyle};
use pathfinder_content::transform::Transform2FPathIter;
use pathfinder_geometry::line_segment::LineSegment2F;
use pathfinder_geometry::rect::RectF;
use pathfinder_geometry::transform2d::Transform2F;
use pathfinder_geometry::vector::{Vector2F, vec2f};
use pathfinder_renderer::paint::Paint;
use pathfinder_renderer::scene::{ClipPath, ClipPathId, DrawPath, Scene};
use pathfinder_simd::default::F32x2;
use std::fmt::{Display, Formatter, Result as FormatResult};
use usvg::{BaseGradient, Color as SvgColor, FillRule as UsvgFillRule, LineCap as UsvgLineCap};
use usvg::{LineJoin as UsvgLineJoin, Node, NodeExt, NodeKind, Opacity, Paint as UsvgPaint};
use usvg::{PathSegment as UsvgPathSegment, Rect as UsvgRect, SpreadMethod, Stop};
use usvg::{Transform as UsvgTransform, Tree, Visibility};

const HAIRLINE_STROKE_WIDTH: f32 = 0.0333;

pub struct BuiltSVG {
    pub scene: Scene,
    pub result_flags: BuildResultFlags,
    pub clip_paths: HashMap<String, ClipPathId>,
    gradients: HashMap<String, GradientInfo>,
}

bitflags! {
    // NB: If you change this, make sure to update the `Display`
    // implementation as well.
    pub struct BuildResultFlags: u16 {
        const UNSUPPORTED_FILTER_NODE            = 0x0001;
        const UNSUPPORTED_IMAGE_NODE             = 0x0002;
        const UNSUPPORTED_MASK_NODE              = 0x0004;
        const UNSUPPORTED_PATTERN_NODE           = 0x0008;
        const UNSUPPORTED_NESTED_SVG_NODE        = 0x0010;
        const UNSUPPORTED_MULTIPLE_CLIP_PATHS    = 0x0020;
        const UNSUPPORTED_LINK_PAINT             = 0x0040;
        const UNSUPPORTED_FILTER_ATTR            = 0x0080;
        const UNSUPPORTED_MASK_ATTR              = 0x0100;
        const UNSUPPORTED_GRADIENT_SPREAD_METHOD = 0x0200;
    }
}

impl BuiltSVG {
    // TODO(pcwalton): Allow a global transform to be set.
    #[inline]
    pub fn from_tree(tree: &Tree) -> BuiltSVG {
        BuiltSVG::from_tree_and_scene(tree, Scene::new())
    }

    // TODO(pcwalton): Allow a global transform to be set.
    pub fn from_tree_and_scene(tree: &Tree, scene: Scene) -> BuiltSVG {
        // TODO(pcwalton): Maybe have a `SVGBuilder` type to hold the clip path IDs and other
        // transient data separate from `BuiltSVG`?
        let mut built_svg = BuiltSVG {
            scene,
            result_flags: BuildResultFlags::empty(),
            clip_paths: HashMap::new(),
            gradients: HashMap::new(),
        };

        let root = &tree.root();
        match *root.borrow() {
            NodeKind::Svg(ref svg) => {
                built_svg.scene.set_view_box(usvg_rect_to_euclid_rect(&svg.view_box.rect));
                for kid in root.children() {
                    built_svg.process_node(&kid, &State::new(), &mut None);
                }
            }
            _ => unreachable!(),
        }

        built_svg
    }

    fn process_node(&mut self,
                    node: &Node,
                    state: &State,
                    clip_outline: &mut Option<Outline>) {
        let mut state = (*state).clone();
        let node_transform = usvg_transform_to_transform_2d(&node.transform());
        state.transform = node_transform * state.transform;

        match *node.borrow() {
            NodeKind::Group(ref group) => {
                if group.filter.is_some() {
                    self.result_flags.insert(BuildResultFlags::UNSUPPORTED_FILTER_ATTR);
                }
                if group.mask.is_some() {
                    self.result_flags.insert(BuildResultFlags::UNSUPPORTED_MASK_ATTR);
                }

                if let Some(ref clip_path_name) = group.clip_path {
                    if let Some(clip_path_id) = self.clip_paths.get(clip_path_name) {
                        // TODO(pcwalton): Combine multiple clip paths if there's already one.
                        if state.clip_path.is_some() {
                            self.result_flags
                                .insert(BuildResultFlags::UNSUPPORTED_MULTIPLE_CLIP_PATHS);
                        }
                        state.clip_path = Some(*clip_path_id);
                    }
                }

                for kid in node.children() {
                    self.process_node(&kid, &state, clip_outline)
                }
            }
            NodeKind::Path(ref path) if state.path_destination == PathDestination::Clip => {
                // TODO(pcwalton): Multiple clip paths.
                let path = UsvgPathToSegments::new(path.data.iter().cloned());
                let path = Transform2FPathIter::new(path, &state.transform);
                if clip_outline.is_some() {
                    self.result_flags.insert(BuildResultFlags::UNSUPPORTED_MULTIPLE_CLIP_PATHS);
                }
                *clip_outline = Some(Outline::from_segments(path));
            }
            NodeKind::Path(ref path) if state.path_destination == PathDestination::Draw &&
                    path.visibility == Visibility::Visible => {
                if let Some(ref fill) = path.fill {
                    let path = UsvgPathToSegments::new(path.data.iter().cloned());
                    let outline = Outline::from_segments(path);

                    let name = format!("Fill({})", node.id());
                    self.push_draw_path(outline,
                                        name,
                                        &state,
                                        &fill.paint,
                                        fill.opacity,
                                        fill.rule);
                }

                if let Some(ref stroke) = path.stroke {
                    let stroke_style = StrokeStyle {
                        line_width: f32::max(stroke.width.value() as f32, HAIRLINE_STROKE_WIDTH),
                        line_cap: LineCap::from_usvg_line_cap(stroke.linecap),
                        line_join: LineJoin::from_usvg_line_join(stroke.linejoin,
                                                                 stroke.miterlimit.value() as f32),
                    };

                    let path = UsvgPathToSegments::new(path.data.iter().cloned());
                    let mut outline = Outline::from_segments(path);

                    if let Some(ref dash_array) = stroke.dasharray {
                        let dash_array: Vec<f32> = dash_array.iter().map(|&x| x as f32).collect();
                        let mut dash = OutlineDash::new(&outline, &dash_array, stroke.dashoffset);
                        dash.dash();
                        outline = dash.into_outline();
                    }

                    let mut stroke_to_fill = OutlineStrokeToFill::new(&outline, stroke_style);
                    stroke_to_fill.offset();
                    let outline = stroke_to_fill.into_outline();

                    let name = format!("Stroke({})", node.id());
                    self.push_draw_path(outline,
                                        name,
                                        &state,
                                        &stroke.paint,
                                        stroke.opacity,
                                        UsvgFillRule::NonZero);
                }
            }
            NodeKind::Path(..) => {}
            NodeKind::ClipPath(_) => {
                let mut clip_outline = None;
                state.path_destination = PathDestination::Clip;
                for kid in node.children() {
                    self.process_node(&kid, &state, &mut clip_outline);
                }

                if let Some(clip_outline) = clip_outline {
                    // FIXME(pcwalton): Is the winding fill rule correct to use?
                    let mut clip_path = ClipPath::new(clip_outline);
                    clip_path.set_name(format!("ClipPath({})", node.id()));
                    let clip_path_id = self.scene.push_clip_path(clip_path);
                    self.clip_paths.insert(node.id().to_owned(), clip_path_id);
                }
            }
            NodeKind::Defs => {
                // FIXME(pcwalton): This is wrong.
                state.path_destination = PathDestination::Defs;
                for kid in node.children() {
                    self.process_node(&kid, &state, clip_outline);
                }
            }
            NodeKind::LinearGradient(ref svg_linear_gradient) => {
                let from = vec2f(svg_linear_gradient.x1 as f32, svg_linear_gradient.y1 as f32);
                let to   = vec2f(svg_linear_gradient.x2 as f32, svg_linear_gradient.y2 as f32);
                let gradient = Gradient::linear_from_points(from, to);
                self.add_gradient(gradient,
                                  svg_linear_gradient.id.clone(),
                                  &svg_linear_gradient.base)
            }
            NodeKind::RadialGradient(ref svg_radial_gradient) => {
                let from = vec2f(svg_radial_gradient.fx as f32, svg_radial_gradient.fy as f32);
                let to   = vec2f(svg_radial_gradient.cx as f32, svg_radial_gradient.cy as f32);
                let radii = F32x2::new(0.0, svg_radial_gradient.r.value() as f32);
                let gradient = Gradient::radial(LineSegment2F::new(from, to), radii);
                self.add_gradient(gradient,
                                  svg_radial_gradient.id.clone(),
                                  &svg_radial_gradient.base)
            }
            NodeKind::Filter(..) => {
                self.result_flags
                    .insert(BuildResultFlags::UNSUPPORTED_FILTER_NODE);
            }
            NodeKind::Image(..) => {
                self.result_flags
                    .insert(BuildResultFlags::UNSUPPORTED_IMAGE_NODE);
            }
            NodeKind::Mask(..) => {
                self.result_flags
                    .insert(BuildResultFlags::UNSUPPORTED_MASK_NODE);
            }
            NodeKind::Pattern(..) => {
                self.result_flags
                    .insert(BuildResultFlags::UNSUPPORTED_PATTERN_NODE);
            }
            NodeKind::Svg(..) => {
                self.result_flags
                    .insert(BuildResultFlags::UNSUPPORTED_NESTED_SVG_NODE);
            }
        }
    }

    fn add_gradient(&mut self,
                    mut gradient: Gradient,
                    id: String,
                    usvg_base_gradient: &BaseGradient) {
        for stop in &usvg_base_gradient.stops {
            gradient.add(ColorStop::from_usvg_stop(stop));
        }

        if usvg_base_gradient.spread_method != SpreadMethod::Pad {
            self.result_flags.insert(BuildResultFlags::UNSUPPORTED_GRADIENT_SPREAD_METHOD);
        }

        let transform = usvg_transform_to_transform_2d(&usvg_base_gradient.transform);

        // TODO(pcwalton): What should we do with `gradientUnits`?
        self.gradients.insert(id, GradientInfo { gradient, transform });
    }

    fn push_draw_path(&mut self,
                      mut outline: Outline,
                      name: String,
                      state: &State,
                      paint: &UsvgPaint,
                      opacity: Opacity,
                      fill_rule: UsvgFillRule) {
        outline.transform(&state.transform);
        let paint = Paint::from_svg_paint(paint,
                                          &state.transform,
                                          opacity,
                                          &self.gradients,
                                          &mut self.result_flags);
        let style = self.scene.push_paint(&paint);
        let fill_rule = FillRule::from_usvg_fill_rule(fill_rule);
        let mut path = DrawPath::new(outline, style);
        path.set_clip_path(state.clip_path);
        path.set_fill_rule(fill_rule);
        path.set_name(name);
        self.scene.push_path(path);
    }
}

impl Display for BuildResultFlags {
    fn fmt(&self, formatter: &mut Formatter) -> FormatResult {
        if self.is_empty() {
            return Ok(());
        }

        let mut first = true;
        for (bit, name) in NAMES.iter().enumerate() {
            if (self.bits() >> bit) & 1 == 0 {
                continue;
            }
            if !first {
                formatter.write_str(", ")?;
            } else {
                first = false;
            }
            formatter.write_str(name)?;
        }

        return Ok(());

        // Must match the order in `BuildResultFlags`.
        static NAMES: &'static [&'static str] = &[
            "<filter>",
            "<image>",
            "<mask>",
            "<pattern>",
            "nested <svg>",
            "multiple clip paths",
            "non-color paint",
            "filter attribute",
            "mask attribute",
            "gradient spread method",
        ];
    }
}

trait PaintExt {
    fn from_svg_paint(svg_paint: &UsvgPaint,
                      transform: &Transform2F,
                      opacity: Opacity,
                      gradients: &HashMap<String, GradientInfo>,
                      result_flags: &mut BuildResultFlags)
                      -> Self;
}

impl PaintExt for Paint {
    #[inline]
    fn from_svg_paint(svg_paint: &UsvgPaint,
                      transform: &Transform2F,
                      opacity: Opacity,
                      gradients: &HashMap<String, GradientInfo>,
                      result_flags: &mut BuildResultFlags)
                      -> Paint {
        let mut paint;
        match *svg_paint {
            UsvgPaint::Color(color) => paint = Paint::from_color(ColorU::from_svg_color(color)),
            UsvgPaint::Link(ref id) => {
                match gradients.get(id) {
                    Some(ref gradient_info) => {
                        paint = Paint::from_gradient(gradient_info.gradient.clone());
                        paint.apply_transform(&(*transform * gradient_info.transform));
                    }
                    None => {
                        // TODO(pcwalton)
                        result_flags.insert(BuildResultFlags::UNSUPPORTED_LINK_PAINT);
                        paint = Paint::from_color(ColorU::black());
                    }
                }
            }
        }

        let mut base_color = paint.base_color().to_f32();
        base_color.set_a(base_color.a() * opacity.value() as f32);
        paint.set_base_color(base_color.to_u8());

        paint
    }
}

fn usvg_rect_to_euclid_rect(rect: &UsvgRect) -> RectF {
    RectF::new(vec2f(rect.x() as f32, rect.y() as f32),
               vec2f(rect.width() as f32, rect.height() as f32))
}

fn usvg_transform_to_transform_2d(transform: &UsvgTransform) -> Transform2F {
    Transform2F::row_major(transform.a as f32, transform.c as f32, transform.e as f32,
                           transform.b as f32, transform.d as f32, transform.f as f32)
}

struct UsvgPathToSegments<I>
where
    I: Iterator<Item = UsvgPathSegment>,
{
    iter: I,
    first_subpath_point: Vector2F,
    last_subpath_point: Vector2F,
    just_moved: bool,
}

impl<I> UsvgPathToSegments<I>
where
    I: Iterator<Item = UsvgPathSegment>,
{
    fn new(iter: I) -> UsvgPathToSegments<I> {
        UsvgPathToSegments {
            iter,
            first_subpath_point: Vector2F::zero(),
            last_subpath_point: Vector2F::zero(),
            just_moved: false,
        }
    }
}

impl<I> Iterator for UsvgPathToSegments<I>
where
    I: Iterator<Item = UsvgPathSegment>,
{
    type Item = Segment;

    fn next(&mut self) -> Option<Segment> {
        match self.iter.next()? {
            UsvgPathSegment::MoveTo { x, y } => {
                let to = vec2f(x as f32, y as f32);
                self.first_subpath_point = to;
                self.last_subpath_point = to;
                self.just_moved = true;
                self.next()
            }
            UsvgPathSegment::LineTo { x, y } => {
                let to = vec2f(x as f32, y as f32);
                let mut segment = Segment::line(LineSegment2F::new(self.last_subpath_point, to));
                if self.just_moved {
                    segment.flags.insert(SegmentFlags::FIRST_IN_SUBPATH);
                }
                self.last_subpath_point = to;
                self.just_moved = false;
                Some(segment)
            }
            UsvgPathSegment::CurveTo {
                x1,
                y1,
                x2,
                y2,
                x,
                y,
            } => {
                let ctrl0 = vec2f(x1 as f32, y1 as f32);
                let ctrl1 = vec2f(x2 as f32, y2 as f32);
                let to = vec2f(x as f32, y as f32);
                let mut segment = Segment::cubic(LineSegment2F::new(self.last_subpath_point, to),
                                                 LineSegment2F::new(ctrl0, ctrl1));
                if self.just_moved {
                    segment.flags.insert(SegmentFlags::FIRST_IN_SUBPATH);
                }
                self.last_subpath_point = to;
                self.just_moved = false;
                Some(segment)
            }
            UsvgPathSegment::ClosePath => {
                let mut segment = Segment::line(LineSegment2F::new(
                    self.last_subpath_point,
                    self.first_subpath_point,
                ));
                segment.flags.insert(SegmentFlags::CLOSES_SUBPATH);
                self.just_moved = false;
                self.last_subpath_point = self.first_subpath_point;
                Some(segment)
            }
        }
    }
}

trait ColorUExt {
    fn from_svg_color(svg_color: SvgColor) -> Self;
}

impl ColorUExt for ColorU {
    #[inline]
    fn from_svg_color(svg_color: SvgColor) -> ColorU {
        ColorU { r: svg_color.red, g: svg_color.green, b: svg_color.blue, a: !0 }
    }
}

trait LineCapExt {
    fn from_usvg_line_cap(usvg_line_cap: UsvgLineCap) -> Self;
}

impl LineCapExt for LineCap {
    #[inline]
    fn from_usvg_line_cap(usvg_line_cap: UsvgLineCap) -> LineCap {
        match usvg_line_cap {
            UsvgLineCap::Butt => LineCap::Butt,
            UsvgLineCap::Round => LineCap::Round,
            UsvgLineCap::Square => LineCap::Square,
        }
    }
}

trait LineJoinExt {
    fn from_usvg_line_join(usvg_line_join: UsvgLineJoin, miter_limit: f32) -> Self;
}

impl LineJoinExt for LineJoin {
    #[inline]
    fn from_usvg_line_join(usvg_line_join: UsvgLineJoin, miter_limit: f32) -> LineJoin {
        match usvg_line_join {
            UsvgLineJoin::Miter => LineJoin::Miter(miter_limit),
            UsvgLineJoin::Round => LineJoin::Round,
            UsvgLineJoin::Bevel => LineJoin::Bevel,
        }
    }
}

trait FillRuleExt {
    fn from_usvg_fill_rule(usvg_fill_rule: UsvgFillRule) -> Self;
}

impl FillRuleExt for FillRule {
    #[inline]
    fn from_usvg_fill_rule(usvg_fill_rule: UsvgFillRule) -> FillRule {
        match usvg_fill_rule {
            UsvgFillRule::NonZero => FillRule::Winding,
            UsvgFillRule::EvenOdd => FillRule::EvenOdd,
        }
    }
}

trait ColorStopExt {
    fn from_usvg_stop(usvg_stop: &Stop) -> Self;
}

impl ColorStopExt for ColorStop {
    fn from_usvg_stop(usvg_stop: &Stop) -> ColorStop {
        let mut color = ColorU::from_svg_color(usvg_stop.color);
        color.a = (usvg_stop.opacity.value() * 255.0) as u8;
        ColorStop::new(color, usvg_stop.offset.value() as f32)
    }
}

#[derive(Clone)]
struct State {
    // Where paths are being appended to.
    path_destination: PathDestination,
    // The current transform.
    transform: Transform2F,
    // The current clip path in effect.
    clip_path: Option<ClipPathId>,
}

impl State {
    fn new() -> State {
        State {
            path_destination: PathDestination::Draw,
            transform: Transform2F::default(),
            clip_path: None,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
enum PathDestination {
    Draw,
    Defs,
    Clip,
}

struct GradientInfo {
    gradient: Gradient,
    transform: Transform2F,
}
