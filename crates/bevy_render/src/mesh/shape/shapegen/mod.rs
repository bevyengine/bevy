use slice::Slice::{self, *};

use crate::mesh::{Indices, VertexAttributeValues};
use crate::{Mesh, PrimitiveTopology};

mod interpolation;
mod slice;

use bevy_utils::HashMap;
pub use interpolation::*;

struct Edge {
    points: Vec<u32>,
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

#[derive(Clone, Debug)]
enum TriangleContents {
    None,
    One(u32),
    Three {
        a: u32,
        b: u32,
        c: u32,
    },
    Six {
        a: u32,
        b: u32,
        c: u32,
        ab: u32,
        bc: u32,
        ca: u32,
    },
    More {
        a: u32,
        b: u32,
        c: u32,
        // Separated into three `my_side_length` segments
        // to save on extra allocations.
        sides: Vec<u32>,
        my_side_length: u32,
    },
}

impl TriangleContents {
    fn one(ab: Slice<u32>, bc: Slice<u32>, points: &mut Attributes) -> Self {
        assert_eq!(ab.len(), bc.len());
        assert_eq!(ab.len(), 2);
        let index = points.len() as u32;
        points.extend_default(1);
        TriangleContents::One(index)
    }

    fn three(&mut self, ab: Slice<u32>, bc: Slice<u32>, ca: Slice<u32>, points: &mut Attributes) {
        use TriangleContents::*;

        assert_eq!(ab.len(), bc.len());
        assert_eq!(ab.len(), ca.len());
        assert_eq!(ab.len(), 3);

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

    fn six(&mut self, ab: Slice<u32>, bc: Slice<u32>, ca: Slice<u32>, points: &mut Attributes) {
        use TriangleContents::*;

        assert_eq!(ab.len(), bc.len());
        assert_eq!(ab.len(), ca.len());
        assert_eq!(ab.len(), 4);

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

    pub fn calculate<I: AttributeInterpolator<A>, A: Copy>(
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
                self.subdivide(ab, bc, ca, points)
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

                let side_length = *my_side_length as usize;

                let ab = &sides[..side_length];
                let bc = &sides[side_length..side_length * 2];
                let ca = &sides[side_length * 2..];

                Option::Some((Forward(ab), Forward(bc), Forward(ca)))
            }
        }
    }

    pub fn idx_ab(&self, idx: usize) -> u32 {
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

    pub fn idx_bc(&self, idx: usize) -> u32 {
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

    pub fn idx_ca(&self, idx: usize) -> u32 {
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

    pub fn add_indices(&self, buffer: &mut Vec<u32>, next: Option<&Self>) {
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
                buffer.extend_from_slice(&[a, ab, ca]);
                buffer.extend_from_slice(&[ab, b, bc]);
                buffer.extend_from_slice(&[bc, c, ca]);

                buffer.extend_from_slice(&[ab, bc, ca]);
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
        return;
    }

    let last_idx = ab.len() - 1;

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
    buffer.extend_from_slice(&[
        ab[last_idx],
        contents.idx_ab(last_idx - 1),
        ab[last_idx - 1],
    ]);

    buffer.extend_from_slice(&[
        bc[last_idx],
        contents.idx_bc(last_idx - 1),
        bc[last_idx - 1],
    ]);

    buffer.extend_from_slice(&[
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

    fn subdivide(&mut self, edges: &mut [Edge], points: &mut Attributes) {
        let side_length = self.subdivide_edges(edges, points) + 1;

        if side_length > 2 {
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
                self.contents.push(last);
            }
        }
    }

    pub fn calculate<I: AttributeInterpolator<A>, A: Copy>(
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

        if self.contents.len() != 0 {
            assert!(result.is_none());
        }
    }

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
                    contents: vec![]
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
