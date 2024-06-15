use bevy_color::Color;
use bevy_gizmos::{config::GizmoConfigGroup, prelude::Gizmos};
use bevy_math::{Vec2, Vec2Swizzles};
use bevy_reflect::Reflect;
use bevy_transform::prelude::GlobalTransform;
use bevy_utils::HashMap;

use super::{CameraQuery, LayoutRect};

// Function used here so we don't need to redraw lines that are fairly close to each other.
fn approx_eq(compared: f32, other: f32) -> bool {
    (compared - other).abs() < 0.001
}

fn rect_border_axis(rect: LayoutRect) -> (f32, f32, f32, f32) {
    let pos = rect.pos;
    let size = rect.size;
    let offset = pos + size;
    (pos.x, offset.x, pos.y, offset.y)
}

#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Debug)]
enum Dir {
    Start,
    End,
}
impl Dir {
    const fn increments(self) -> i64 {
        match self {
            Dir::Start => 1,
            Dir::End => -1,
        }
    }
}
impl From<i64> for Dir {
    fn from(value: i64) -> Self {
        if value.is_positive() {
            Dir::Start
        } else {
            Dir::End
        }
    }
}
/// Collection of axis aligned "lines" (actually just their coordinate on
/// a given axis).
#[derive(Debug, Clone)]
struct DrawnLines {
    lines: HashMap<i64, Dir>,
    width: f32,
}
#[allow(clippy::cast_precision_loss, clippy::cast_possible_truncation)]
impl DrawnLines {
    fn new(width: f32) -> Self {
        DrawnLines {
            lines: HashMap::new(),
            width,
        }
    }
    /// Return `value` offset by as many `increment`s as necessary to make it
    /// not overlap with already drawn lines.
    fn inset(&self, value: f32) -> f32 {
        let scaled = value / self.width;
        let fract = scaled.fract();
        let mut on_grid = scaled.floor() as i64;
        for _ in 0..10 {
            let Some(dir) = self.lines.get(&on_grid) else {
                break;
            };
            // TODO(clean): This fixes a panic, but I'm not sure how valid this is
            let Some(added) = on_grid.checked_add(dir.increments()) else {
                break;
            };
            on_grid = added;
        }
        ((on_grid as f32) + fract) * self.width
    }
    /// Remove a line from the collection of drawn lines.
    ///
    /// Typically, we only care for pre-existing lines when drawing the children
    /// of a container, nothing more. So we remove it after we are done with
    /// the children.
    fn remove(&mut self, value: f32, increment: i64) {
        let mut on_grid = (value / self.width).floor() as i64;
        loop {
            // TODO(clean): This fixes a panic, but I'm not sure how valid this is
            let Some(next_cell) = on_grid.checked_add(increment) else {
                return;
            };
            if !self.lines.contains_key(&next_cell) {
                self.lines.remove(&on_grid);
                return;
            }
            on_grid = next_cell;
        }
    }
    /// Add a line from the collection of drawn lines.
    fn add(&mut self, value: f32, increment: i64) {
        let mut on_grid = (value / self.width).floor() as i64;
        loop {
            let old_value = self.lines.insert(on_grid, increment.into());
            if old_value.is_none() {
                return;
            }
            // TODO(clean): This fixes a panic, but I'm not sure how valid this is
            let Some(added) = on_grid.checked_add(increment) else {
                return;
            };
            on_grid = added;
        }
    }
}

#[derive(GizmoConfigGroup, Reflect, Default)]
pub struct UiGizmosDebug;

pub(super) struct InsetGizmo<'w, 's> {
    draw: Gizmos<'w, 's, UiGizmosDebug>,
    cam: CameraQuery<'w, 's>,
    known_y: DrawnLines,
    known_x: DrawnLines,
}
impl<'w, 's> InsetGizmo<'w, 's> {
    pub(super) fn new(
        draw: Gizmos<'w, 's, UiGizmosDebug>,
        cam: CameraQuery<'w, 's>,
        line_width: f32,
    ) -> Self {
        InsetGizmo {
            draw,
            cam,
            known_y: DrawnLines::new(line_width),
            known_x: DrawnLines::new(line_width),
        }
    }
    fn relative(&self, mut position: Vec2) -> Vec2 {
        let zero = GlobalTransform::IDENTITY;
        let Ok(cam) = self.cam.get_single() else {
            return Vec2::ZERO;
        };
        if let Some(new_position) = cam.world_to_viewport(&zero, position.extend(0.)) {
            position = new_position;
        };
        position.xy()
    }
    fn line_2d(&mut self, mut start: Vec2, mut end: Vec2, color: Color) {
        if approx_eq(start.x, end.x) {
            start.x = self.known_x.inset(start.x);
            end.x = start.x;
        } else if approx_eq(start.y, end.y) {
            start.y = self.known_y.inset(start.y);
            end.y = start.y;
        }
        let (start, end) = (self.relative(start), self.relative(end));
        self.draw.line_2d(start, end, color);
    }
    pub(super) fn set_scope(&mut self, rect: LayoutRect) {
        let (left, right, top, bottom) = rect_border_axis(rect);
        self.known_x.add(left, 1);
        self.known_x.add(right, -1);
        self.known_y.add(top, 1);
        self.known_y.add(bottom, -1);
    }
    pub(super) fn clear_scope(&mut self, rect: LayoutRect) {
        let (left, right, top, bottom) = rect_border_axis(rect);
        self.known_x.remove(left, 1);
        self.known_x.remove(right, -1);
        self.known_y.remove(top, 1);
        self.known_y.remove(bottom, -1);
    }
    pub(super) fn rect_2d(&mut self, rect: LayoutRect, color: Color) {
        let (left, right, top, bottom) = rect_border_axis(rect);
        if approx_eq(left, right) {
            self.line_2d(Vec2::new(left, top), Vec2::new(left, bottom), color);
        } else if approx_eq(top, bottom) {
            self.line_2d(Vec2::new(left, top), Vec2::new(right, top), color);
        } else {
            let inset_x = |v| self.known_x.inset(v);
            let inset_y = |v| self.known_y.inset(v);
            let (left, right) = (inset_x(left), inset_x(right));
            let (top, bottom) = (inset_y(top), inset_y(bottom));
            let strip = [
                Vec2::new(left, top),
                Vec2::new(left, bottom),
                Vec2::new(right, bottom),
                Vec2::new(right, top),
                Vec2::new(left, top),
            ];
            self.draw
                .linestrip_2d(strip.map(|v| self.relative(v)), color);
        }
    }
}
