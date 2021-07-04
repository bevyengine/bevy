use slice::Slice::{self, *};

use crate::mesh::{Indices, VertexAttributeValues};
use crate::{Mesh, PrimitiveTopology};

mod interpolation;
mod slice;

use bevy_utils::HashMap;
pub use interpolation::*;

<<<<<<< HEAD
struct Edge {
    points: Vec<u32>,
=======
/// Describes an edge on the original mesh.
struct Edge {
    /// Indices of points along the edge.
    points: Vec<u32>,
    /// Whether or not this edge has already been
    /// subdivided in this pass.
>>>>>>> temp
    done: bool,
}

impl Default for Edge {
    fn default() -> Self {
        Self {
            points: Vec::new(),
            done: true,
        }
    }
}

<<<<<<< HEAD
#[derive(Clone, Debug)]
enum TriangleContents {
    None,
    One(u32),
    Three {
        a: u32,
        b: u32,
        c: u32,
    },
=======
/// Describes a "layer" of a triangle in terms
/// of the indices of the points inside.
#[derive(Clone, Debug)]
enum TriangleContents {
    /// No contents.
    None,

    /// One point inside.
    One(u32),

    /// Three points inside, in a triangle.
    ///
    /// `a`, `b`, and `c` go in the same winding
    /// as the rest of the mesh.
    Three { a: u32, b: u32, c: u32 },

    /// Six points, one on each vertex of a triangle,
    /// and one in the middle of each edge of a triangle.
    ///
    /// Once again, `a`, `b` and `c`, along with their
    /// intermediaries go in the same winding as the rest
    /// of the mesh.
>>>>>>> temp
    Six {
        a: u32,
        b: u32,
        c: u32,
        ab: u32,
        bc: u32,
        ca: u32,
    },
<<<<<<< HEAD
=======

    /// More than 6 points in a layer. The contents of this
    /// layer are the next element in the `Vec` which it was
    /// stored in.
>>>>>>> temp
    More {
        a: u32,
        b: u32,
        c: u32,
<<<<<<< HEAD
        // Separated into three `my_side_length` segments
        // to save on extra allocations.
        sides: Vec<u32>,
=======
        /// Contains the indices of the sides adjacent to eachother
        /// in memory:
        ///
        /// ```ignore
        /// [ab1, ab2, ... abN, bc1, bc2, ... bcN, ca1, ca2, ... caN]
        /// ```
        ///
        /// Where `N` is `my_side_length`.
        sides: Vec<u32>,
        /// The number of points inside an edge.
>>>>>>> temp
        my_side_length: u32,
    },
}

impl TriangleContents {
<<<<<<< HEAD
    fn one(ab: Slice<u32>, bc: Slice<u32>, points: &mut Attributes) -> Self {
        assert_eq!(ab.len(), bc.len());
        assert_eq!(ab.len(), 2);
=======
    /// Creates a `One` variant.
    fn one(points: &mut Attributes) -> Self {
>>>>>>> temp
        let index = points.len() as u32;
        points.extend_default(1);
        TriangleContents::One(index)
    }

<<<<<<< HEAD
    fn three(&mut self, ab: Slice<u32>, bc: Slice<u32>, ca: Slice<u32>, points: &mut Attributes) {
        use TriangleContents::*;

        assert_eq!(ab.len(), bc.len());
        assert_eq!(ab.len(), ca.len());
        assert_eq!(ab.len(), 3);

=======
    /// Turns the current `One` variant into a `Three`
    /// variant, reusing the previous point.
    fn three(&mut self, points: &mut Attributes) {
        use TriangleContents::*;

>>>>>>> temp
        match self {
            One(x) => {
                points.extend_default(2);

                *self = Three {
                    a: *x,
                    b: points.len() as u32 - 2,
                    c: points.len() as u32 - 1,
                };
            }
            _ => panic!("Self is {:?} while it should be One", self),
        }
    }

<<<<<<< HEAD
    fn six(&mut self, ab: Slice<u32>, bc: Slice<u32>, ca: Slice<u32>, points: &mut Attributes) {
        use TriangleContents::*;

        assert_eq!(ab.len(), bc.len());
        assert_eq!(ab.len(), ca.len());
        assert_eq!(ab.len(), 4);

=======
    /// Turns the current `Three` variant into a `Six`
    /// variant, reusing the previous points.
    fn six(&mut self, points: &mut Attributes) {
        use TriangleContents::*;

>>>>>>> temp
        match self {
            Three {
                a: a_index,
                b: b_index,
                c: c_index,
            } => {
                points.extend_default(3);

                *self = Six {
                    a: *a_index,
                    b: *b_index,
                    c: *c_index,
                    ab: points.len() as u32 - 3,
                    bc: points.len() as u32 - 2,
                    ca: points.len() as u32 - 1,
                };
            }
            _ => panic!("Found {:?} whereas a Three was expected", self),
        }
    }

<<<<<<< HEAD
    pub fn calculate<I: AttributeInterpolator<A>, A: Copy>(
=======
    /// Actually performs the calculation of the new points
    /// for a single attribute, assuming there are no more
    /// subdivisions left to perform.
    ///
    /// `ab`, `bc`, and `ca` are the edge indices of the previous
    /// layer.
    ///
    /// `interpolator` performs the interpolation between
    /// vertices as necessary.
    ///
    /// This returns the next set of `ab`, `bc` and `ca`, or
    /// `None` if there should not be a next layer after this.
    fn calculate<I: AttributeInterpolator<A>, A: Copy>(
>>>>>>> temp
        &self,
        ab: Slice<u32>,
        bc: Slice<u32>,
        ca: Slice<u32>,
        interpolator: &mut I,
        attributes: &mut [A],
    ) -> Option<(Slice<u32>, Slice<u32>, Slice<u32>)> {
        use TriangleContents::*;
        match self {
            None => Option::None,
            One(p) => {
                let p1 = attributes[ab[0] as usize];
                let p2 = attributes[bc[1] as usize];

                attributes[*p as usize] = interpolator.interpolate_half(p1, p2);

                Option::None
            }
            Three { a, b, c } => {
                let ab = attributes[ab[1] as usize];
                let bc = attributes[bc[1] as usize];
                let ca = attributes[ca[1] as usize];

                let a_v = interpolator.interpolate_half(ab, ca);
                let b_v = interpolator.interpolate_half(bc, ab);
                let c_v = interpolator.interpolate_half(ca, bc);

                attributes[*a as usize] = a_v;
                attributes[*b as usize] = b_v;
                attributes[*c as usize] = c_v;

                Option::None
            }
            Six {
                a,
                b,
                c,
                ab: ab_idx,
                bc: bc_idx,
                ca: ca_idx,
            } => {
                let aba = attributes[ab[1] as usize];
                let abb = attributes[ab[2] as usize];
                let bcb = attributes[bc[1] as usize];
                let bcc = attributes[bc[2] as usize];
                let cac = attributes[ca[1] as usize];
                let caa = attributes[ca[2] as usize];

                let a_v = interpolator.interpolate_half(aba, caa);
                let b_v = interpolator.interpolate_half(abb, bcb);
                let c_v = interpolator.interpolate_half(bcc, cac);

                let ab_v = interpolator.interpolate_half(a_v, b_v);
                let bc_v = interpolator.interpolate_half(b_v, c_v);
                let ca_v = interpolator.interpolate_half(c_v, a_v);

                attributes[*a as usize] = a_v;
                attributes[*b as usize] = b_v;
                attributes[*c as usize] = c_v;
                attributes[*ab_idx as usize] = ab_v;
                attributes[*bc_idx as usize] = bc_v;
                attributes[*ca_idx as usize] = ca_v;

                Option::None
            }
            More {
                a,
                b,
                c,
                sides,
                my_side_length,
            } => {
                let outer_len = ab.len();

                let aba = attributes[ab[1] as usize];
                let abb = attributes[ab[outer_len - 2] as usize];
                let bcb = attributes[bc[1] as usize];
                let bcc = attributes[bc[outer_len - 2] as usize];
                let cac = attributes[ca[1] as usize];
                let caa = attributes[ca[outer_len - 2] as usize];

                attributes[*a as usize] = interpolator.interpolate_half(aba, caa);
                attributes[*b as usize] = interpolator.interpolate_half(abb, bcb);
                attributes[*c as usize] = interpolator.interpolate_half(bcc, cac);

                let side_length = *my_side_length as usize;

                let ab = &sides[..side_length];
                let bc = &sides[side_length..side_length * 2];
                let ca = &sides[side_length * 2..];

                interpolator.interpolate_multiple(
                    attributes[*a as usize],
                    attributes[*b as usize],
                    ab,
                    attributes,
                );
                interpolator.interpolate_multiple(
                    attributes[*b as usize],
                    attributes[*c as usize],
                    bc,
                    attributes,
                );
                interpolator.interpolate_multiple(
                    attributes[*c as usize],
                    attributes[*a as usize],
                    ca,
                    attributes,
                );

                Option::Some((Forward(ab), Forward(bc), Forward(ca)))
            }
        }
    }

<<<<<<< HEAD
    pub fn subdivide(
        &mut self,
        ab: Slice<u32>,
        bc: Slice<u32>,
        ca: Slice<u32>,
        points: &mut Attributes,
    ) -> Option<(Slice<u32>, Slice<u32>, Slice<u32>)> {
        use TriangleContents::*;
        assert_eq!(ab.len(), bc.len());
        assert_eq!(ab.len(), ca.len());
        assert!(ab.len() >= 2);
        match self {
            None => {
                *self = Self::one(ab, bc, points);
                Option::None
            }
            One(_) => {
                self.three(ab, bc, ca, points);
                Option::None
            }
            Three { .. } => {
                self.six(ab, bc, ca, points);
                Option::None
            }
=======
    /// Should this layer have another layer after
    /// it?
    fn should_have_next(&self) -> bool {
        use TriangleContents::*;

        matches!(self, More { .. })
    }

    /// Performs subdivision, only recording indices,
    /// without actually calculating the new attributes.
    fn subdivide(&mut self, points: &mut Attributes) {
        use TriangleContents::*;
        match self {
            None => *self = Self::one(points),
            One(_) => self.three(points),
            Three { .. } => self.six(points),
>>>>>>> temp
            &mut Six {
                a,
                b,
                c,
                ab: ab_idx,
                bc: bc_idx,
                ca: ca_idx,
            } => {
                *self = More {
                    a,
                    b,
                    c,
                    sides: vec![ab_idx, bc_idx, ca_idx],
                    my_side_length: 1,
                };
<<<<<<< HEAD
                self.subdivide(ab, bc, ca, points)
=======
                self.subdivide(points)
>>>>>>> temp
            }
            &mut More {
                a: _,
                b: _,
                c: _,
                ref mut sides,
                ref mut my_side_length,
            } => {
                points.extend_default(3);
                let len = points.len() as u32;
                sides.extend_from_slice(&[len - 3, len - 2, len - 1]);
                *my_side_length += 1;
<<<<<<< HEAD

                let side_length = *my_side_length as usize;

                let ab = &sides[..side_length];
                let bc = &sides[side_length..side_length * 2];
                let ca = &sides[side_length * 2..];

                Option::Some((Forward(ab), Forward(bc), Forward(ca)))
=======
>>>>>>> temp
            }
        }
    }

<<<<<<< HEAD
    pub fn idx_ab(&self, idx: usize) -> u32 {
=======
    /// Useful when constructing the new indices.
    ///
    /// This indexes the whole edge, including ending
    /// vertices.
    fn idx_ab(&self, idx: usize) -> u32 {
>>>>>>> temp
        use TriangleContents::*;
        match self {
            None => panic!("Invalid Index, len is 0, but got {}", idx),
            One(x) => {
                if idx != 0 {
                    panic!("Invalid Index, len is 1, but got {}", idx);
                } else {
                    *x
                }
            }
            Three { a, b, .. } => *[a, b][idx],
            Six { a, b, ab, .. } => *[a, ab, b][idx],
            &More {
                a,
                b,
                ref sides,
                my_side_length,
                ..
            } => match idx {
                0 => a,
                x if (1..(my_side_length as usize + 1)).contains(&x) => sides[x - 1],
                x if x == my_side_length as usize + 1 => b,
                _ => panic!(
                    "Invalid Index, len is {}, but got {}",
                    my_side_length + 2,
                    idx
                ),
            },
        }
    }

<<<<<<< HEAD
    pub fn idx_bc(&self, idx: usize) -> u32 {
=======
    /// See `idx_ab`.
    fn idx_bc(&self, idx: usize) -> u32 {
>>>>>>> temp
        use TriangleContents::*;
        match self {
            None => panic!("Invalid Index, len is 0, but got {}", idx),
            One(x) => {
                if idx != 0 {
                    panic!("Invalid Index, len is 1, but got {}", idx);
                } else {
                    *x
                }
            }
            Three { c, b, .. } => *[b, c][idx],
            Six { b, c, bc, .. } => *[b, bc, c][idx],
            &More {
                b,
                c,
                ref sides,
                my_side_length,
                ..
            } => match idx {
                0 => b,
                x if (1..(my_side_length as usize + 1)).contains(&x) => {
                    sides[my_side_length as usize + (x - 1)]
                }
                x if x == my_side_length as usize + 1 => c,
                _ => panic!(
                    "Invalid Index, len is {}, but got {}",
                    my_side_length + 2,
                    idx
                ),
            },
        }
    }

<<<<<<< HEAD
    pub fn idx_ca(&self, idx: usize) -> u32 {
=======
    /// See `idx_ab`.
    fn idx_ca(&self, idx: usize) -> u32 {
>>>>>>> temp
        use TriangleContents::*;
        match self {
            None => panic!("Invalid Index, len is 0, but got {}", idx),
            One(x) => {
                if idx != 0 {
                    panic!("Invalid Index, len is 1, but got {}", idx);
                } else {
                    *x
                }
            }
            Three { c, a, .. } => *[c, a][idx],
            Six { c, a, ca, .. } => *[c, ca, a][idx],
            &More {
                c,
                a,
                ref sides,
                my_side_length,
                ..
            } => match idx {
                0 => c,
                x if (1..(my_side_length as usize + 1)).contains(&x) => {
                    sides[my_side_length as usize * 2 + x - 1]
                }
                x if x == my_side_length as usize + 1 => a,
                _ => panic!(
                    "Invalid Index, len is {}, but got {}",
                    my_side_length + 2,
                    idx
                ),
            },
        }
    }

<<<<<<< HEAD
    pub fn add_indices(&self, buffer: &mut Vec<u32>, next: Option<&Self>) {
=======
    /// Adds the indices of the triangles mostly associated with
    /// this layer into the buffer.
    ///
    /// Although this takes an optional reference to the next
    /// layer, it does not add the triangles associated mostly with
    /// that layer.
    fn add_indices(&self, buffer: &mut Vec<u32>, next: Option<&Self>) {
>>>>>>> temp
        use TriangleContents::*;
        match self {
            None | One(_) => {}
            &Three { a, b, c } => buffer.extend_from_slice(&[a, b, c]),
            &Six {
                a,
                b,
                c,
                ab,
                bc,
                ca,
            } => {
<<<<<<< HEAD
                buffer.extend_from_slice(&[a, ab, ca]);
                buffer.extend_from_slice(&[ab, b, bc]);
                buffer.extend_from_slice(&[bc, c, ca]);

                buffer.extend_from_slice(&[ab, bc, ca]);
=======
                #[rustfmt::skip]
                buffer.extend_from_slice(&[
                    a, ab, ca,
                    ab, b, bc,
                    bc, c, ca,

                    ab, bc, ca,
                ]);
>>>>>>> temp
            }
            &More {
                a,
                b,
                c,
                ref sides,
                my_side_length,
            } => {
                let next = next.unwrap();
                let my_side_length = my_side_length as usize;
                let ab = &sides[0..my_side_length];
                let bc = &sides[my_side_length..my_side_length * 2];
                let ca = &sides[my_side_length * 2..];

                // Contents are always stored forward.
                add_indices_triangular(
                    a,
                    b,
                    c,
                    Forward(ab),
                    Forward(bc),
                    Forward(ca),
                    next,
                    buffer,
                );
            }
        }
    }
}

// The logic in this function has been worked out mostly on pen and paper
// and therefore it is difficult to read.
//
// Hush, bot. It has exactly how many arguments it should.
#[allow(clippy::too_many_arguments)]
fn add_indices_triangular(
    a: u32,
    b: u32,
    c: u32,
    ab: Slice<u32>,
    bc: Slice<u32>,
    ca: Slice<u32>,
    contents: &TriangleContents,
    buffer: &mut Vec<u32>,
) {
    let subdivisions = ab.len();
    if subdivisions == 0 {
        buffer.extend_from_slice(&[a, b, c]);
        return;
    } else if subdivisions == 1 {
<<<<<<< HEAD
        buffer.extend_from_slice(&[a, ab[0], ca[0]]);
        buffer.extend_from_slice(&[b, bc[0], ab[0]]);
        buffer.extend_from_slice(&[c, ca[0], bc[0]]);
        buffer.extend_from_slice(&[ab[0], bc[0], ca[0]]);
        return;
    } else if subdivisions == 2 {
        buffer.extend_from_slice(&[a, ab[0], ca[1]]);
        buffer.extend_from_slice(&[b, bc[0], ab[1]]);
        buffer.extend_from_slice(&[c, ca[0], bc[1]]);

        buffer.extend_from_slice(&[ab[1], contents.idx_ab(0), ab[0]]);
        buffer.extend_from_slice(&[bc[1], contents.idx_ab(0), bc[0]]);
        buffer.extend_from_slice(&[ca[1], contents.idx_ab(0), ca[0]]);

        buffer.extend_from_slice(&[ab[0], contents.idx_ab(0), ca[1]]);
        buffer.extend_from_slice(&[bc[0], contents.idx_ab(0), ab[1]]);
        buffer.extend_from_slice(&[ca[0], contents.idx_ab(0), bc[1]]);
=======
        #[rustfmt::skip]
        buffer.extend_from_slice(&[
            a, ab[0], ca[0],
            b, bc[0], ab[0],
            c, ca[0], bc[0],
            ab[0], bc[0], ca[0],
        ]);
        return;
    } else if subdivisions == 2 {
        let center = contents.idx_ab(0);
        #[rustfmt::skip]
        buffer.extend_from_slice(&[
            a, ab[0], ca[1],
            b, bc[0], ab[1],
            c, ca[0], bc[1],

            ab[1], center, ab[0],
            bc[1], center, bc[0],
            ca[1], center, ca[0],

            ab[0], center, ca[1],
            bc[0], center, ab[1],
            ca[0], center, bc[1],
        ]);
>>>>>>> temp
        return;
    }

    let last_idx = ab.len() - 1;

<<<<<<< HEAD
    buffer.extend_from_slice(&[a, ab[0], ca[last_idx]]);
    buffer.extend_from_slice(&[b, bc[0], ab[last_idx]]);
    buffer.extend_from_slice(&[c, ca[0], bc[last_idx]]);

    buffer.extend_from_slice(&[ab[0], contents.idx_ab(0), ca[last_idx]]);
    buffer.extend_from_slice(&[bc[0], contents.idx_bc(0), ab[last_idx]]);
    buffer.extend_from_slice(&[ca[0], contents.idx_ca(0), bc[last_idx]]);

    for i in 0..last_idx - 1 {
        // Exclude special case: last_idx - 1.
        // AB
        buffer.extend_from_slice(&[ab[i], ab[i + 1], contents.idx_ab(i)]);
        buffer.extend_from_slice(&[ab[i + 1], contents.idx_ab(i + 1), contents.idx_ab(i)]);
        // BC
        buffer.extend_from_slice(&[bc[i], bc[i + 1], contents.idx_bc(i)]);
        buffer.extend_from_slice(&[bc[i + 1], contents.idx_bc(i + 1), contents.idx_bc(i)]);
        // CA
        buffer.extend_from_slice(&[ca[i], ca[i + 1], contents.idx_ca(i)]);
        buffer.extend_from_slice(&[ca[i + 1], contents.idx_ca(i + 1), contents.idx_ca(i)]);
    }

    // Deal with special case: last_idx - 1
=======
    #[rustfmt::skip]
    buffer.extend_from_slice(&[
        a, ab[0], ca[last_idx],
        b, bc[0], ab[last_idx],
        c, ca[0], bc[last_idx],

        ab[0], contents.idx_ab(0), ca[last_idx],
        bc[0], contents.idx_bc(0), ab[last_idx],
        ca[0], contents.idx_ca(0), bc[last_idx],
    ]);

    for i in 0..last_idx - 1 {
        #[rustfmt::skip]
        buffer.extend_from_slice(&[
            // Exclude special case: last_idx - 1.
            // AB
            ab[i], ab[i + 1], contents.idx_ab(i),
            ab[i + 1], contents.idx_ab(i + 1), contents.idx_ab(i),

            // BC
            bc[i], bc[i + 1], contents.idx_bc(i),
            bc[i + 1], contents.idx_bc(i + 1), contents.idx_bc(i),

            // CA
            ca[i], ca[i + 1], contents.idx_ca(i),
            ca[i + 1], contents.idx_ca(i + 1), contents.idx_ca(i),
        ]);
    }

    // Deal with special case: last_idx - 1
    #[rustfmt::skip]
>>>>>>> temp
    buffer.extend_from_slice(&[
        ab[last_idx],
        contents.idx_ab(last_idx - 1),
        ab[last_idx - 1],
<<<<<<< HEAD
    ]);

    buffer.extend_from_slice(&[
        bc[last_idx],
        contents.idx_bc(last_idx - 1),
        bc[last_idx - 1],
    ]);

    buffer.extend_from_slice(&[
=======

        bc[last_idx],
        contents.idx_bc(last_idx - 1),
        bc[last_idx - 1],

>>>>>>> temp
        ca[last_idx],
        contents.idx_ca(last_idx - 1),
        ca[last_idx - 1],
    ]);
}

// This could in theory just be another TriangleContents,
// but we need to special-case the outermost triangles because
// they share subdivided edges with surrounding ones
// and we don't want to subdivide the same edge twice in a
// single subdivision pass.
#[derive(Clone, Debug)]
struct Triangle {
    pub a: u32,
    pub b: u32,
    pub c: u32,
    pub ab_edge: usize,
    pub bc_edge: usize,
    pub ca_edge: usize,
    pub ab_forward: bool,
    pub bc_forward: bool,
    pub ca_forward: bool,

    pub contents: Vec<TriangleContents>,
}

impl Triangle {
<<<<<<< HEAD
    pub const fn new(
        a: u32,
        b: u32,
        c: u32,
        ab_edge: usize,
        bc_edge: usize,
        ca_edge: usize,
    ) -> Self {
        Self {
            a,
            b,
            c,
            ab_edge,
            bc_edge,
            ca_edge,

            ab_forward: false,
            bc_forward: false,
            ca_forward: false,

            contents: Vec::new(),
        }
    }

=======
    /// Subdivides the edges, adding indices to them.
>>>>>>> temp
    fn subdivide_edges(&mut self, edges: &mut [Edge], points: &mut Attributes) -> usize {
        let mut divide = |edge_idx: usize| {
            if !edges[edge_idx].done {
                edges[edge_idx].points.push(points.len() as u32);
                points.extend_default(1);

                edges[edge_idx].done = true;
            }
        };

        divide(self.ab_edge);
        divide(self.bc_edge);
        divide(self.ca_edge);

        edges[self.ab_edge].points.len()
    }

<<<<<<< HEAD
=======
    /// Subdivides the edges and contents of the triangle,
    /// without calculating the new values for the points inside.
    ///
    /// To calculate the values for the various attributes of the
    /// points inside, call the `calculate` function with the
    /// adequate interpolator and attribute list.
>>>>>>> temp
    fn subdivide(&mut self, edges: &mut [Edge], points: &mut Attributes) {
        let side_length = self.subdivide_edges(edges, points) + 1;

        if side_length > 2 {
<<<<<<< HEAD
            let abbcca = self.get_edge_slices(edges);

            let result = self
                .contents
                .iter_mut()
                .fold(Some(abbcca), |abbcca, layer| {
                    let (ab, bc, ca) = abbcca.unwrap();
                    layer.subdivide(ab, bc, ca, points)
                });

            if let Some((ab, bc, ca)) = result {
                let mut last = TriangleContents::None;
                let result = last.subdivide(ab, bc, ca, points);
                debug_assert!(result.is_none());
=======
            self.contents
                .iter_mut()
                .for_each(|layer| layer.subdivide(points));

            if self
                .contents
                .last()
                .map(|x| x.should_have_next())
                // For when starting the contents, we
                // will have to assume true. Remember `side_length > 2`.
                .unwrap_or(true)
            {
                let mut last = TriangleContents::None;
                last.subdivide(points);
>>>>>>> temp
                self.contents.push(last);
            }
        }
    }

<<<<<<< HEAD
    pub fn calculate<I: AttributeInterpolator<A>, A: Copy>(
=======
    /// Calculates the values for the attributes of the vertices along
    /// the edges of the triangle, and the contents of the triangle.
    ///
    /// Will skip edges on the triangle which have already had their
    /// attributes interpolated.
    fn calculate<I: AttributeInterpolator<A>, A: Copy>(
>>>>>>> temp
        &mut self,
        interpolator: &mut I,
        attributes: &mut [A],
        edges: &mut [Edge],
    ) {
        let mut calculate = |p1: u32, p2: u32, edge_idx: usize| {
            if !edges[edge_idx].done {
                interpolator.interpolate_multiple(
                    attributes[p1 as usize],
                    attributes[p2 as usize],
                    &edges[edge_idx].points,
                    attributes,
                );

                edges[edge_idx].done = true;
            }
        };

        calculate(self.a, self.b, self.ab_edge);
        calculate(self.b, self.c, self.bc_edge);
        calculate(self.c, self.a, self.ca_edge);

        let abbcca = self.get_edge_slices(edges);

        let result = self.contents.iter().fold(Some(abbcca), |abbcca, layer| {
            let (ab, bc, ca) = abbcca.unwrap();
            layer.calculate(ab, bc, ca, interpolator, attributes)
        });

<<<<<<< HEAD
        if self.contents.len() != 0 {
=======
        if !self.contents.is_empty() {
>>>>>>> temp
            assert!(result.is_none());
        }
    }

<<<<<<< HEAD
=======
    /// Adds the resulting indices associated with the triangles
    /// inside of this "chunk" triangle into the buffer.
    ///
    /// Preserves winding from the source mesh.
>>>>>>> temp
    fn add_indices(&self, buffer: &mut Vec<u32>, edges: &[Edge]) {
        let (ab, bc, ca) = self.get_edge_slices(edges);

        add_indices_triangular(
            self.a,
            self.b,
            self.c,
            ab,
            bc,
            ca,
            self.contents.first().unwrap_or(&TriangleContents::None),
            buffer,
        );

        for (i, layer) in self.contents.iter().enumerate() {
            let next_layer = self.contents.get(i + 1);
            layer.add_indices(buffer, next_layer);
        }
    }

<<<<<<< HEAD
=======
    /// Gets the `Slice`s associated with the edges
    /// of this triangle. `Slice` is used instead of
    /// `[T]` since sometimes we need the data to be
    /// read backwards instead of forwards.
>>>>>>> temp
    fn get_edge_slices<'a>(
        &'_ self,
        edges: &'a [Edge],
    ) -> (Slice<'a, u32>, Slice<'a, u32>, Slice<'a, u32>) {
        let ab = if self.ab_forward {
            Forward(&edges[self.ab_edge].points)
        } else {
            Backward(&edges[self.ab_edge].points)
        };
        let bc = if self.bc_forward {
            Forward(&edges[self.bc_edge].points)
        } else {
            Backward(&edges[self.bc_edge].points)
        };
        let ca = if self.ca_forward {
            Forward(&edges[self.ca_edge].points)
        } else {
            Backward(&edges[self.ca_edge].points)
        };

        (ab, bc, ca)
    }
}

/// Deals with the attributes in an attribute-agnostic way.
struct Attributes<'a> {
<<<<<<< HEAD

=======
>>>>>>> temp
    /// The current length of the attributes.
    pub len: usize,

    /// The attributes, with their names.
    pub attributes: Vec<(&'a str, &'a mut VertexAttributeValues)>,

    /// The number of extra, default attributes to add.
    pub tail: usize,
}

impl<'a> Attributes<'a> {
    pub fn new(len: usize, attributes: Vec<(&'a str, &'a mut VertexAttributeValues)>) -> Self {
        Self {
            len,
            attributes,
            tail: 0,
        }
    }

    /// Length of the attributes as if the tail had
    /// already been applied.
    pub fn len(&self) -> usize {
        self.len + self.tail
    }

    /// Lazily adds `len` default values to the end of each
    /// attribute list.
    pub fn extend_default(&mut self, len: usize) {
        self.tail += len;
    }

    /// Extends all of the attributes associated with the
    /// mesh by `self.tail` default values.
    pub fn apply_tail(&mut self) {
        macro_rules! fill_default {
            ($len:expr, $list:expr, $t:ty) => {
                $list.extend((0..$len).map::<$t, _>(|_| Default::default()))
            };
        }

        for (_, i) in &mut self.attributes {
            match i {
<<<<<<< HEAD
                VertexAttributeValues::Int(x) => fill_default!(self.tail, x, i32),
                VertexAttributeValues::Int2(x) => fill_default!(self.tail, x, [i32; 2]),
                VertexAttributeValues::Int3(x) => fill_default!(self.tail, x, [i32; 3]),
                VertexAttributeValues::Int4(x) => fill_default!(self.tail, x, [i32; 4]),
                VertexAttributeValues::Uint(x) => fill_default!(self.tail, x, u32),
                VertexAttributeValues::Uint2(x) => fill_default!(self.tail, x, [u32; 2]),
                VertexAttributeValues::Uint3(x) => fill_default!(self.tail, x, [u32; 3]),
                VertexAttributeValues::Uint4(x) => fill_default!(self.tail, x, [u32; 4]),
                VertexAttributeValues::Float(x) => fill_default!(self.tail, x, f32),
                VertexAttributeValues::Float2(x) => fill_default!(self.tail, x, [f32; 2]),
                VertexAttributeValues::Float3(x) => fill_default!(self.tail, x, [f32; 3]),
                VertexAttributeValues::Float4(x) => fill_default!(self.tail, x, [f32; 4]),
                VertexAttributeValues::Uchar4Norm(x) => fill_default!(self.tail, x, [u8; 4]),
=======
                VertexAttributeValues::Float32(x) => fill_default!(self.tail, x, f32),
                VertexAttributeValues::Sint32(x) => fill_default!(self.tail, x, i32),
                VertexAttributeValues::Uint32(x) => fill_default!(self.tail, x, u32),
                VertexAttributeValues::Float32x2(x) => fill_default!(self.tail, x, [f32; 2]),
                VertexAttributeValues::Sint32x2(x) => fill_default!(self.tail, x, [i32; 2]),
                VertexAttributeValues::Uint32x2(x) => fill_default!(self.tail, x, [u32; 2]),
                VertexAttributeValues::Float32x3(x) => fill_default!(self.tail, x, [f32; 3]),
                VertexAttributeValues::Sint32x3(x) => fill_default!(self.tail, x, [i32; 3]),
                VertexAttributeValues::Uint32x3(x) => fill_default!(self.tail, x, [u32; 3]),
                VertexAttributeValues::Float32x4(x) => fill_default!(self.tail, x, [f32; 4]),
                VertexAttributeValues::Sint32x4(x) => fill_default!(self.tail, x, [i32; 4]),
                VertexAttributeValues::Uint32x4(x) => fill_default!(self.tail, x, [u32; 4]),
                VertexAttributeValues::Sint16x2(x) => fill_default!(self.tail, x, [i16; 2]),
                VertexAttributeValues::Snorm16x2(x) => fill_default!(self.tail, x, [i16; 2]),
                VertexAttributeValues::Uint16x2(x) => fill_default!(self.tail, x, [u16; 2]),
                VertexAttributeValues::Unorm16x2(x) => fill_default!(self.tail, x, [u16; 2]),
                VertexAttributeValues::Sint16x4(x) => fill_default!(self.tail, x, [i16; 4]),
                VertexAttributeValues::Snorm16x4(x) => fill_default!(self.tail, x, [i16; 4]),
                VertexAttributeValues::Uint16x4(x) => fill_default!(self.tail, x, [u16; 4]),
                VertexAttributeValues::Unorm16x4(x) => fill_default!(self.tail, x, [u16; 4]),
                VertexAttributeValues::Sint8x2(x) => fill_default!(self.tail, x, [i8; 2]),
                VertexAttributeValues::Snorm8x2(x) => fill_default!(self.tail, x, [i8; 2]),
                VertexAttributeValues::Uint8x2(x) => fill_default!(self.tail, x, [u8; 2]),
                VertexAttributeValues::Unorm8x2(x) => fill_default!(self.tail, x, [u8; 2]),
                VertexAttributeValues::Sint8x4(x) => fill_default!(self.tail, x, [i8; 4]),
                VertexAttributeValues::Snorm8x4(x) => fill_default!(self.tail, x, [i8; 4]),
                VertexAttributeValues::Uint8x4(x) => fill_default!(self.tail, x, [u8; 4]),
                VertexAttributeValues::Unorm8x4(x) => fill_default!(self.tail, x, [u8; 4]),
>>>>>>> temp
            }
        }

        self.tail = 0;
    }

    /// Calculates the values of each new index after subdivison
    /// using the interpolator specified. This will query the
    /// interpolator for the adequate attribute-specific
    /// interpolator depending on the name and type.
    ///
    /// For more information see [`Interpolator`].
    pub fn calculate<I: Interpolator>(
        &mut self,
        triangles: &mut [Triangle],
        edges: &mut [Edge],
        mut interpolator: I,
    ) {
        fn calculate_specific<I: AttributeInterpolator<A>, A: Copy>(
            triangles: &mut [Triangle],
            edges: &mut [Edge],
            mut interpolator: I,
            attributes: &mut [A],
        ) {
            for triangle in triangles {
                triangle.calculate(&mut interpolator, attributes, edges);
            }
            edges.iter_mut().for_each(|Edge { done, .. }| *done = false);
        }

<<<<<<< HEAD
        for (name, attr) in &mut self.attributes {
            match attr {
                VertexAttributeValues::Int(x) => {
                    calculate_specific(triangles, edges, interpolator.int(name), x)
                }
                VertexAttributeValues::Int2(x) => {
                    calculate_specific(triangles, edges, interpolator.int2(name), x)
                }
                VertexAttributeValues::Int3(x) => {
                    calculate_specific(triangles, edges, interpolator.int3(name), x)
                }
                VertexAttributeValues::Int4(x) => {
                    calculate_specific(triangles, edges, interpolator.int4(name), x)
                }
                VertexAttributeValues::Uint(x) => {
                    calculate_specific(triangles, edges, interpolator.uint(name), x)
                }
                VertexAttributeValues::Uint2(x) => {
                    calculate_specific(triangles, edges, interpolator.uint2(name), x)
                }
                VertexAttributeValues::Uint3(x) => {
                    calculate_specific(triangles, edges, interpolator.uint3(name), x)
                }
                VertexAttributeValues::Uint4(x) => {
                    calculate_specific(triangles, edges, interpolator.uint4(name), x)
                }
                VertexAttributeValues::Float(x) => {
                    calculate_specific(triangles, edges, interpolator.float(name), x)
                }
                VertexAttributeValues::Float2(x) => {
                    calculate_specific(triangles, edges, interpolator.float2(name), x)
                }
                VertexAttributeValues::Float3(x) => {
                    calculate_specific(triangles, edges, interpolator.float3(name), x)
                }
                VertexAttributeValues::Float4(x) => {
                    calculate_specific(triangles, edges, interpolator.float4(name), x)
                }
                VertexAttributeValues::Uchar4Norm(x) => {
                    calculate_specific(triangles, edges, interpolator.uchar4norm(name), x)
                }
            }
        }
=======
        macro_rules! select_right_type {
            ($val:ident; $triangles:ident; $edges:ident; $interpolator:ident; $name:ident; $(($type_name:ident: $fn_name:ident)),*$(,)?) => {
                match $val {
                    $(
                        VertexAttributeValues::$type_name(x) => {
                            calculate_specific($triangles, $edges, $interpolator.$fn_name($name), x)
                        }
                    ),*
                }
            }
        }
        for (name, attr) in &mut self.attributes {
            select_right_type! {
                attr; triangles; edges; interpolator; name;
                (Float32: float32),
                (Sint32: sint32),
                (Uint32: uint32),
                (Float32x2: float32x2),
                (Sint32x2: sint32x2),
                (Uint32x2: uint32x2),
                (Float32x3: float32x3),
                (Sint32x3: sint32x3),
                (Uint32x3: uint32x3),
                (Float32x4: float32x4),
                (Sint32x4: sint32x4),
                (Uint32x4: uint32x4),
                (Sint16x2: sint16x2),
                (Snorm16x2: snorm16x2),
                (Uint16x2: uint16x2),
                (Unorm16x2: unorm16x2),
                (Sint16x4: sint16x4),
                (Snorm16x4: snorm16x4),
                (Uint16x4: uint16x4),
                (Unorm16x4: unorm16x4),
                (Sint8x2: sint8x2),
                (Snorm8x2: snorm8x2),
                (Uint8x2: uint8x2),
                (Unorm8x2: unorm8x2),
                (Sint8x4: sint8x4),
                (Snorm8x4: snorm8x4),
                (Uint8x4: uint8x4),
                (Unorm8x4: unorm8x4),
            }
        }
>>>>>>> temp
    }
}

/// If there are already indices present in the mesh, return those
/// as `u32`s. Otherwise, return `0..len`. The boolean indicates if
/// the default indices (`0..len`) were generated.
fn get_indices(mesh: &mut Mesh, len: usize) -> (Vec<u32>, bool) {
    if let Some(indices) = mesh.indices() {
        let i = match indices {
            Indices::U16(x) => x.iter().map(|x| *x as _).collect(),
            Indices::U32(x) => x.clone(),
        };
        (i, false)
    } else {
        ((0..len as u32).collect(), true)
    }
}

/// Groups indices into triangles and edges.
///
/// `is_iota` is used to indicate if the indices
/// are linear starting from 0: `[0, 1, 2, 3, 4, 5, etc.]`.
fn generate_triangles(indices: &[u32], is_iota: bool) -> (Box<[Triangle]>, Box<[Edge]>) {
    assert_eq!(indices.len() % 3, 0);
    if is_iota {
        let triangles = indices
            .chunks(3)
<<<<<<< HEAD
            .map(|x| {
                Triangle {
                    a: x[0],
                    b: x[1],
                    c: x[2],
                    ab_edge: x[0] as usize,
                    bc_edge: x[1] as usize,
                    ca_edge: x[2] as usize,
                    ab_forward: true,
                    bc_forward: true,
                    ca_forward: true,
                    contents: vec![],
                }
=======
            .map(|x| Triangle {
                a: x[0],
                b: x[1],
                c: x[2],
                ab_edge: x[0] as usize,
                bc_edge: x[1] as usize,
                ca_edge: x[2] as usize,
                ab_forward: true,
                bc_forward: true,
                ca_forward: true,
                contents: vec![],
>>>>>>> temp
            })
            .collect::<Vec<_>>();
        let edges = indices
            .iter()
            .map(|_| Edge {
                points: Vec::new(),
                done: false,
            })
            .collect::<Vec<_>>();

        (triangles.into(), edges.into())
    } else {
        let mut edges = Vec::new();
        let mut edge_map = HashMap::default();

        let mut make_edge = |i, j| {
            // If we happen to find `i, j`, then that means we
            // are looking a triangle with winding opposite to a triangle
            // it borders. If such a case arises, we ignore it
            // and just add another edge.
            let index = edge_map.get(&(j, i));
            match index {
                Some(x) => (*x, false),
                None => {
                    let x = edges.len();
                    edge_map.insert((i, j), x);
                    edges.push(Edge {
                        points: Vec::new(),
                        done: false,
                    });
                    (x, true)
                }
            }
        };

        let triangles = indices
            .chunks(3)
            .map(move |x| {
                let [a, b, c] = [x[0], x[1], x[2]];
                let (ab, ab_forward) = make_edge(a, b);
                let (bc, bc_forward) = make_edge(b, c);
                let (ca, ca_forward) = make_edge(c, a);
                Triangle {
                    a,
                    b,
                    c,
                    ab_edge: ab,
                    bc_edge: bc,
                    ca_edge: ca,
                    ab_forward,
                    bc_forward,
                    ca_forward,
<<<<<<< HEAD
                    contents: vec![]
=======
                    contents: vec![],
>>>>>>> temp
                }
            })
            .collect::<Vec<_>>();

        (triangles.into(), edges.into())
    }
}

/// Subdivides a mesh N times, progressing through a series of
/// triangular numbers associated with each original triangle.
///
/// ![Series of triangular dots](https://nzmaths.co.nz/sites/default/files/images/uploads/users/3/triangular.PNG)
pub(crate) fn subdivide<I: Interpolator>(
    mesh: &mut Mesh,
    iterations: usize,
    interpolator: I,
) -> Option<()> {
    if iterations == 0 {
        return Some(());
    }

    match mesh.primitive_topology() {
        PrimitiveTopology::LineList
        | PrimitiveTopology::LineStrip
        | PrimitiveTopology::PointList => {
            // Unsupported. todo?
            return None;
        }
        PrimitiveTopology::TriangleStrip => {
            // Unsupported.
            // This is inherently incompatible with
            // this algorithm.
            // Perhaps write an algo to unwrap triangle
            // strips into triangle lists?
            return None;
        }
        _ => {}
    }

    let len = mesh
        .attribute_iter()
        .min_by(|(_, x), (_, y)| x.len().cmp(&y.len()));
    let len = if let Some(x) = len {
        x.1.len()
    } else {
        // There are no attributes attached to this mesh.
        return Some(());
    };

    let (mut indices, is_iota) = get_indices(mesh, len);

    let attributes = mesh
        .attribute_iter_mut()
        .filter(|(_, x)| x.len() == len)
        .collect::<Vec<_>>();

    let mut attributes = Attributes::new(len, attributes);
    let (mut triangles, mut edges) = generate_triangles(&indices, is_iota);

    for _ in 0..iterations {
        for triangle in &mut triangles[..] {
            triangle.subdivide(&mut edges, &mut attributes);
        }

        edges.iter_mut().for_each(|Edge { done, .. }| *done = false);
    }

    attributes.apply_tail();
    attributes.calculate(&mut triangles[..], &mut edges[..], interpolator);

    indices.clear();
    for triangle in &*triangles {
        triangle.add_indices(&mut indices, &*edges);
    }

    drop(attributes);
    mesh.set_indices(Some(Indices::U32(indices)));

    Some(())
}
