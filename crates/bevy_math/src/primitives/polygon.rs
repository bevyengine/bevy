extern crate alloc;
use alloc::collections::BTreeMap;
use core::cmp::Ordering;

use crate::Vec2;

use super::{Measured2d, Triangle2d};

#[derive(Debug, Clone, Copy)]
enum Endpoint {
    Left,
    Right,
}

/// An event in the [`EventQueue`] is either the left or right end of an edge of the polygon.
#[derive(Debug, Clone, Copy)]
struct Event {
    /// Event vertex
    position: Vec2,
    /// Index of the edge in the polygon
    edge_index: usize,
    /// Type of the vertex (left or right)
    ty: Endpoint,
}
impl PartialEq for Event {
    fn eq(&self, other: &Self) -> bool {
        self.position == other.position
    }
}
impl Eq for Event {}
impl PartialOrd for Event {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for Event {
    fn cmp(&self, other: &Self) -> Ordering {
        xy_order(self.position, other.position)
    }
}

/// Orders 2D points according to the order expected by the sweepline and event queue from -X to +X and then -Y to Y.
fn xy_order(a: Vec2, b: Vec2) -> Ordering {
    match a.x.total_cmp(&b.x) {
        Ordering::Equal => a.y.total_cmp(&b.y),
        ord => ord,
    }
}

/// The event queue holds an ordered list of all events the [`Sweepline`] will encounter when checking the current polygon.
#[derive(Debug, Clone)]
struct EventQueue {
    events: Vec<Event>,
}
impl EventQueue {
    /// Initialize a new `EventQueue` with all events from the polygon represented by `vertices`.
    ///
    /// The events will be ordered
    fn new(vertices: &[Vec2]) -> Self {
        if vertices.is_empty() {
            return Self { events: Vec::new() };
        }

        let mut events = Vec::with_capacity(vertices.len() * 2);
        for i in 0..vertices.len() {
            let v1 = vertices[i];
            let v2 = *vertices.get(i + 1).unwrap_or(&vertices[0]);
            let (ty1, ty2) = if xy_order(v1, v2) == Ordering::Less {
                (Endpoint::Left, Endpoint::Right)
            } else {
                (Endpoint::Right, Endpoint::Left)
            };

            events.push(Event {
                edge_index: i,
                position: v1,
                ty: ty1,
            });
            events.push(Event {
                edge_index: i,
                position: v2,
                ty: ty2,
            });
        }

        events.sort();

        Self { events }
    }
}

/// Represents a segment in the [`Sweepline`]
#[derive(Debug, Clone, Copy)]
struct Segment {
    edge_index: usize,
    left: Vec2,
    right: Vec2,
}
impl PartialEq for Segment {
    fn eq(&self, other: &Self) -> bool {
        self.edge_index == other.edge_index
    }
}
impl Eq for Segment {}
impl PartialOrd for Segment {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for Segment {
    fn cmp(&self, other: &Self) -> Ordering {
        match self.left.y.total_cmp(&other.left.y) {
            Ordering::Equal => self.right.y.total_cmp(&other.right.y),
            ord => ord,
        }
    }
}

/// Holds information about which segment is above and which is below a given [`Segment`]
#[derive(Debug, Clone, Copy)]
struct SegmentOrder {
    above: Option<usize>,
    below: Option<usize>,
}

/// A sweep line allows for efficient an efficient search for intersections between [segments](`Segment`).
///
/// It can be thought of as a vertical line sweeping from -X to +X across the polygon that keeps track of the order of the segments
/// the sweep line is intersecting at any given moment.
#[derive(Debug, Clone)]
struct SweepLine<'a> {
    vertices: &'a [Vec2],
    tree: BTreeMap<Segment, SegmentOrder>,
}
impl<'a> SweepLine<'a> {
    fn new(vertices: &'a [Vec2]) -> Self {
        Self {
            vertices,
            tree: BTreeMap::new(),
        }
    }

    /// Determine whther the given edges of the polygon intersect.
    fn intersects(&self, edge1: Option<usize>, edge2: Option<usize>) -> bool {
        let Some(edge1) = edge1 else {
            return false;
        };
        let Some(edge2) = edge2 else {
            return false;
        };

        // All adjacent edges intersect at their shared vertex
        // but these intersections do not count so we ignore them here.
        // Likewise a segment will always intersect itself / an identical edge.
        if edge1 == edge2
            || (edge1 + 1) % self.vertices.len() == edge2
            || (edge2 + 1) % self.vertices.len() == edge1
        {
            return false;
        }

        let s11 = self.vertices[edge1];
        let s12 = *self.vertices.get(edge1 + 1).unwrap_or(&self.vertices[0]);
        let s21 = self.vertices[edge2];
        let s22 = *self.vertices.get(edge2 + 1).unwrap_or(&self.vertices[0]);

        // When both points of the second edge are on the same side of the first edge, no intersection is possible.
        if point_side(s11, s12, s21) * point_side(s11, s12, s22) > 0.0 {
            return false;
        }
        if point_side(s21, s22, s11) * point_side(s21, s22, s12) > 0.0 {
            return false;
        }

        true
    }

    /// Add a new event to the sweep line
    fn add(&mut self, e: Event) -> SegmentOrder {
        let s = self.segment_from_event(e);

        let above = if let Some((next_s, next_ord)) = self.tree.range_mut(s..).next() {
            next_ord.below.replace(e.edge_index);
            Some(next_s.edge_index)
        } else {
            None
        };
        let below = if let Some((prev_s, prev_ord)) = self.tree.range_mut(..s).next_back() {
            prev_ord.above.replace(e.edge_index);
            Some(prev_s.edge_index)
        } else {
            None
        };

        let s_ord = SegmentOrder { above, below };
        self.tree.insert(s, s_ord);
        s_ord
    }

    /// Get the segment order for the event `e`.
    ///
    /// If `e` has not been added to the [`SweepLine`] `None` will be returned.
    fn find(&self, e: Event) -> Option<&SegmentOrder> {
        let s = self.segment_from_event(e);

        self.tree.get(&s)
    }

    /// Remove an event from the [`SweepLine`].
    fn remove(&mut self, e: Event) {
        let s = self.segment_from_event(e);

        let Some(nd) = self.tree.get(&s).copied() else {
            return;
        };

        if let Some((_, above_ord)) = self.tree.range_mut(s..).next() {
            above_ord.below = nd.below;
        }
        if let Some((_, below_ord)) = self.tree.range_mut(..s).next_back() {
            below_ord.above = nd.above;
        }

        self.tree.remove(&s);
    }

    fn segment_from_event(&self, e: Event) -> Segment {
        let v1 = self.vertices[e.edge_index];
        let v2 = *self
            .vertices
            .get(e.edge_index + 1)
            .unwrap_or(&self.vertices[0]);
        let (left, right) = if xy_order(v1, v2) == Ordering::Less {
            (v1, v2)
        } else {
            (v2, v1)
        };

        Segment {
            edge_index: e.edge_index,
            left,
            right,
        }
    }
}

/// Test what side of the line through `p1` and `p2` `q` is.
///
/// The result will be `0` if the `q` is on the segment, negative for one side and positive for the other.
#[inline(always)]
fn point_side(p0: Vec2, p1: Vec2, q: Vec2) -> f32 {
    (p1.x - p0.x) * (q.y - p0.y) - (q.x - p0.x) * (p1.y - p0.y)
}

/// Tests whether the `vertices` describe a simple polygon.
/// The last vertex must not be equal to the first vertex.
///
/// A polygon is simple if it is not self intersecting and not self tangent.
/// As such, no two edges of the polygon may cross each other and each vertex must not lie on another edge.
///
/// Any 'polygon' with less than three vertices is simple.
///
/// The algorithm used is the Shamos-Hoey algorithm, a simplified version of the Bentley-Ottman algorithm.
/// This function will run in O(n * log n)
pub fn is_polygon_simple(vertices: &[Vec2]) -> bool {
    if vertices.len() < 3 {
        return true;
    }
    if vertices.len() == 3 {
        return Triangle2d::new(vertices[0], vertices[1], vertices[2]).area() > 0.0;
    }

    let eq = EventQueue::new(vertices);
    let mut sl = SweepLine::new(vertices);

    for e in eq.events {
        match e.ty {
            Endpoint::Left => {
                let s = sl.add(e);
                if sl.intersects(Some(e.edge_index), s.above)
                    || sl.intersects(Some(e.edge_index), s.below)
                {
                    return false;
                }
            }
            Endpoint::Right => {
                if let Some(s) = sl.find(e) {
                    if sl.intersects(s.above, s.below) {
                        return false;
                    }
                    sl.remove(e);
                }
            }
        }
    }

    true
}

#[cfg(test)]
mod tests {
    use crate::{primitives::polygon::is_polygon_simple, Vec2};

    #[test]
    fn complex_polygon() {
        // A square with one side punching through the opposite side.
        let verts = [Vec2::ZERO, Vec2::X, Vec2::ONE, Vec2::Y, Vec2::new(2.0, 0.5)];
        assert!(!is_polygon_simple(&verts));

        // A square with a vertex from one side touching the opposite side.
        let verts = [Vec2::ZERO, Vec2::X, Vec2::ONE, Vec2::Y, Vec2::new(1.0, 0.5)];
        assert!(!is_polygon_simple(&verts));

        // A square with one side touching the opposite side.
        let verts = [
            Vec2::ZERO,
            Vec2::X,
            Vec2::ONE,
            Vec2::Y,
            Vec2::new(1.0, 0.6),
            Vec2::new(1.0, 0.4),
        ];
        assert!(!is_polygon_simple(&verts));

        // Four points lying on a line
        let verts = [Vec2::ONE, Vec2::new(3., 2.), Vec2::new(5., 3.), Vec2::NEG_X];
        assert!(!is_polygon_simple(&verts));

        // Three points lying on a line
        let verts = [Vec2::ONE, Vec2::new(3., 2.), Vec2::NEG_X];
        assert!(!is_polygon_simple(&verts));

        // Two identical points and one other point
        let verts = [Vec2::ONE, Vec2::ONE, Vec2::NEG_X];
        assert!(!is_polygon_simple(&verts));

        // Two triangles with one shared side
        let verts = [Vec2::ZERO, Vec2::X, Vec2::Y, Vec2::ONE, Vec2::X, Vec2::Y];
        assert!(!is_polygon_simple(&verts));
    }

    #[test]
    fn simple_polygon() {
        // A square
        let verts = [Vec2::ZERO, Vec2::X, Vec2::ONE, Vec2::Y];
        assert!(is_polygon_simple(&verts));

        let verts = [];
        assert!(is_polygon_simple(&verts));
    }
}
