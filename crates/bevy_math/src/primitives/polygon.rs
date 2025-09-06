#[cfg(feature = "alloc")]
use {
    super::{Measured2d, Triangle2d},
    alloc::{collections::BTreeMap, vec::Vec},
    core::cmp::Ordering,
};

use crate::Vec2;

#[derive(Debug, Clone, Copy)]
#[cfg(feature = "alloc")]
enum Endpoint {
    Left,
    Right,
}

/// An event in the [`EventQueue`] is either the left or right vertex of an edge of the polygon.
///
/// Events are ordered so that any event `e1` which is to the left of another event `e2` is less than that event.
/// If `e1.position().x == e2.position().x` the events are ordered from bottom to top.
///
/// This is the order expected by the [`SweepLine`].
#[cfg(feature = "alloc")]
#[derive(Debug, Clone, Copy)]
struct SweepLineEvent {
    segment: Segment,
    /// Type of the vertex (left or right)
    endpoint: Endpoint,
}

#[cfg(feature = "alloc")]
impl SweepLineEvent {
    fn position(&self) -> Vec2 {
        match self.endpoint {
            Endpoint::Left => self.segment.left,
            Endpoint::Right => self.segment.right,
        }
    }
}

#[cfg(feature = "alloc")]
impl PartialEq for SweepLineEvent {
    fn eq(&self, other: &Self) -> bool {
        self.position() == other.position()
    }
}

#[cfg(feature = "alloc")]
impl Eq for SweepLineEvent {}

#[cfg(feature = "alloc")]
impl PartialOrd for SweepLineEvent {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

#[cfg(feature = "alloc")]
impl Ord for SweepLineEvent {
    fn cmp(&self, other: &Self) -> Ordering {
        xy_order(self.position(), other.position())
    }
}

/// Orders 2D points according to the order expected by the sweep line and event queue from -X to +X and then -Y to Y.
#[cfg(feature = "alloc")]
fn xy_order(a: Vec2, b: Vec2) -> Ordering {
    a.x.total_cmp(&b.x).then_with(|| a.y.total_cmp(&b.y))
}

/// The event queue holds an ordered list of all events the [`SweepLine`] will encounter when checking the current polygon.
#[cfg(feature = "alloc")]
#[derive(Debug, Clone)]
struct EventQueue {
    events: Vec<SweepLineEvent>,
}
#[cfg(feature = "alloc")]
impl EventQueue {
    /// Initialize a new `EventQueue` with all events from the polygon represented by `vertices`.
    ///
    /// The events in the event queue will be ordered.
    fn new(vertices: &[Vec2]) -> Self {
        if vertices.is_empty() {
            return Self { events: Vec::new() };
        }

        let mut events = Vec::with_capacity(vertices.len() * 2);
        for i in 0..vertices.len() {
            let v1 = vertices[i];
            let v2 = *vertices.get(i + 1).unwrap_or(&vertices[0]);
            let (left, right) = if xy_order(v1, v2) == Ordering::Less {
                (v1, v2)
            } else {
                (v2, v1)
            };

            let segment = Segment {
                edge_index: i,
                left,
                right,
            };
            events.push(SweepLineEvent {
                segment,
                endpoint: Endpoint::Left,
            });
            events.push(SweepLineEvent {
                segment,
                endpoint: Endpoint::Right,
            });
        }

        events.sort();

        Self { events }
    }
}

/// Represents a segment or rather an edge of the polygon in the [`SweepLine`].
///
/// Segments are ordered from bottom to top based on their left vertices if possible.
/// If their y values are identical, the segments are ordered based on the y values of their right vertices.
#[derive(Debug, Clone, Copy)]
#[cfg(feature = "alloc")]
struct Segment {
    edge_index: usize,
    left: Vec2,
    right: Vec2,
}

#[cfg(feature = "alloc")]
impl PartialEq for Segment {
    fn eq(&self, other: &Self) -> bool {
        self.edge_index == other.edge_index
    }
}

#[cfg(feature = "alloc")]
impl Eq for Segment {}

#[cfg(feature = "alloc")]
impl PartialOrd for Segment {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

#[cfg(feature = "alloc")]
impl Ord for Segment {
    fn cmp(&self, other: &Self) -> Ordering {
        self.left
            .y
            .total_cmp(&other.left.y)
            .then_with(|| self.right.y.total_cmp(&other.right.y))
    }
}

/// Holds information about which segment is above and which is below a given [`Segment`]
#[cfg(feature = "alloc")]
#[derive(Debug, Clone, Copy)]
struct SegmentOrder {
    above: Option<usize>,
    below: Option<usize>,
}

/// A sweep line allows for an efficient search for intersections between [segments](`Segment`).
///
/// It can be thought of as a vertical line sweeping from -X to +X across the polygon that keeps track of the order of the segments
/// the sweep line is intersecting at any given moment.
#[derive(Debug, Clone)]
#[cfg(feature = "alloc")]
struct SweepLine<'a> {
    vertices: &'a [Vec2],
    tree: BTreeMap<Segment, SegmentOrder>,
}
#[cfg(feature = "alloc")]
impl<'a> SweepLine<'a> {
    const fn new(vertices: &'a [Vec2]) -> Self {
        Self {
            vertices,
            tree: BTreeMap::new(),
        }
    }

    /// Determine whether the given edges of the polygon intersect.
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

    /// Add a new segment to the sweep line
    fn add(&mut self, s: Segment) -> SegmentOrder {
        let above = if let Some((next_s, next_ord)) = self.tree.range_mut(s..).next() {
            next_ord.below.replace(s.edge_index);
            Some(next_s.edge_index)
        } else {
            None
        };
        let below = if let Some((prev_s, prev_ord)) = self.tree.range_mut(..s).next_back() {
            prev_ord.above.replace(s.edge_index);
            Some(prev_s.edge_index)
        } else {
            None
        };

        let s_ord = SegmentOrder { above, below };
        self.tree.insert(s, s_ord);
        s_ord
    }

    /// Get the segment order for the given segment.
    ///
    /// If `s` has not been added to the [`SweepLine`] `None` will be returned.
    fn find(&self, s: &Segment) -> Option<&SegmentOrder> {
        self.tree.get(s)
    }

    /// Remove `s` from the [`SweepLine`].
    fn remove(&mut self, s: &Segment) {
        let Some(s_ord) = self.tree.get(s).copied() else {
            return;
        };

        if let Some((_, above_ord)) = self.tree.range_mut(s..).next() {
            above_ord.below = s_ord.below;
        }
        if let Some((_, below_ord)) = self.tree.range_mut(..s).next_back() {
            below_ord.above = s_ord.above;
        }

        self.tree.remove(s);
    }
}

/// Test what side of the line through `p1` and `p2` `q` is.
///
/// The result will be `0` if the `q` is on the segment, negative for one side and positive for the other.
#[cfg_attr(
    not(feature = "alloc"),
    expect(
        dead_code,
        reason = "this function is only used with the alloc feature"
    )
)]
#[inline(always)]
const fn point_side(p1: Vec2, p2: Vec2, q: Vec2) -> f32 {
    (p2.x - p1.x) * (q.y - p1.y) - (q.x - p1.x) * (p2.y - p1.y)
}

/// Tests whether the `vertices` describe a simple polygon.
/// The last vertex must not be equal to the first vertex.
///
/// A polygon is simple if it is not self intersecting and not self tangent.
/// As such, no two edges of the polygon may cross each other and each vertex must not lie on another edge.
///
/// Any 'polygon' with less than three vertices is simple.
///
/// The algorithm used is the Shamos-Hoey algorithm, a version of the Bentley-Ottman algorithm adapted to only detect whether any intersections exist.
/// This function will run in O(n * log n)
#[cfg(feature = "alloc")]
pub fn is_polygon_simple(vertices: &[Vec2]) -> bool {
    if vertices.len() < 3 {
        return true;
    }
    if vertices.len() == 3 {
        return Triangle2d::new(vertices[0], vertices[1], vertices[2]).area() > 0.0;
    }

    let event_queue = EventQueue::new(vertices);
    let mut sweep_line = SweepLine::new(vertices);

    for e in event_queue.events {
        match e.endpoint {
            Endpoint::Left => {
                let s = sweep_line.add(e.segment);
                if sweep_line.intersects(Some(e.segment.edge_index), s.above)
                    || sweep_line.intersects(Some(e.segment.edge_index), s.below)
                {
                    return false;
                }
            }
            Endpoint::Right => {
                if let Some(s) = sweep_line.find(&e.segment) {
                    if sweep_line.intersects(s.above, s.below) {
                        return false;
                    }
                    sweep_line.remove(&e.segment);
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
