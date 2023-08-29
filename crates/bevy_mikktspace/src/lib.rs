//! Mikktspace ported to *safe* Rust.
//!
//! This library is based on Morten S. Mikkelsen's original tangent space algorithm
//! implementation, written in C. The original source code can be found at
//! <https://archive.blender.org/wiki/index.php/Dev:Shading/Tangent_Space_Normal_Maps>
//! and includes the following licence:
//!
//! > Copyright (C) 2011 by Morten S. Mikkelsen
//! >
//! > This software is provided 'as-is', without any express or implied
//! > warranty.  In no event will the authors be held liable for any damages
//! > arising from the use of this software.
//! >
//! > Permission is granted to anyone to use this software for any purpose,
//! > including commercial applications, and to alter it and redistribute it
//! > freely, subject to the following restrictions:
//! >
//! > 1. The origin of this software must not be misrepresented; you must not
//! > claim that you wrote the original software. If you use this software
//! > in a product, an acknowledgment in the product documentation would be
//! > appreciated but is not required.
//! >
//! > 2. Altered source versions must be plainly marked as such, and must not be
//! > misrepresented as being the original software.
//! >
//! > 3. This notice may not be removed or altered from any source distribution.

use bitflags::bitflags;
use glam::{Vec2, Vec3};

mod geometry;

pub use geometry::Geometry;

/// Generates tangents for the input geometry.
///
/// # Errors
///
/// Returns `false` if the geometry is unsuitable for tangent generation including,
/// but not limited to, lack of vertices.
pub fn generate_tangents<G: Geometry>(geometry: &mut G) -> bool {
    generate_tangents_with(geometry, 180.0)
}

fn generate_tangents_with<G: Geometry>(geometry: &mut G, angular_threshold: f32) -> bool {
    // count triangles on supported faces
    let mut triangles_count = 0;
    for face in 0..geometry.num_faces() {
        let verts = geometry.num_vertices_of_face(face);
        if verts == 3 {
            triangles_count += 1;
        } else if verts == 4 {
            triangles_count += 2;
        }
    }

    // make an initial triangle --> face index list
    let (mut triangles_info, mut vertex_indices, tspaces_count) =
        generate_initial_vertex_indices(geometry, triangles_count);

    // make a welded index list of identical positions and attributes (pos, norm, texc)
    generate_shared_vertices_index_list(&mut vertex_indices, geometry, triangles_count);

    // Mark all degenerate triangles
    let total_triangles_count = triangles_count;
    let mut degen_triangles_count = 0;
    for triangle in 0..total_triangles_count {
        let i0 = vertex_indices[triangle * 3];
        let i1 = vertex_indices[triangle * 3 + 1];
        let i2 = vertex_indices[triangle * 3 + 2];
        let p0 = get_position(geometry, i0 as usize);
        let p1 = get_position(geometry, i1 as usize);
        let p2 = get_position(geometry, i2 as usize);

        if p0 == p1 || p0 == p2 || p1 == p2 {
            triangles_info[triangle]
                .flags
                .set(TriangleFlags::MARK_DEGENERATE, true);
            degen_triangles_count += 1;
        }
    }
    triangles_count = total_triangles_count - degen_triangles_count;

    // mark all triangle pairs that belong to a quad with only one
    // good triangle. These need special treatment in DegenEpilogue().
    // Additionally, move all good triangles to the start of
    // pTriInfos[] and piTriListIn[] without changing order and
    // put the degenerate triangles last.
    degen_prologue(
        &mut triangles_info,
        &mut vertex_indices,
        triangles_count as i32,
        total_triangles_count as i32,
    );

    // evaluate triangle level attributes and neighbor list
    init_tri_info(
        geometry,
        &mut triangles_info,
        &vertex_indices,
        triangles_count,
    );

    // based on the 4 rules, identify groups based on connectivity
    let max_groups = triangles_count * 3;
    let mut groups = vec![Group::zero(); max_groups];
    let mut face_indices_buffer = vec![0; triangles_count * 3];

    let active_groups = build_4_rule_groups(
        &mut triangles_info,
        &mut groups,
        &mut face_indices_buffer,
        &vertex_indices,
        triangles_count as i32,
    );

    let mut tspaces = vec![
        TSpace {
            os: Vec3::new(1.0, 0.0, 0.0),
            mag_s: 1.0,
            ot: Vec3::new(0.0, 1.0, 0.0),
            mag_t: 1.0,
            ..TSpace::zero()
        };
        tspaces_count as usize
    ];

    // make tspaces, each group is split up into subgroups if necessary
    // based on fAngularThreshold. Finally a tangent space is made for
    // every resulting subgroup
    let thres_cos = (angular_threshold * std::f32::consts::PI / 180.0).cos();
    generate_tspaces(
        geometry,
        &mut tspaces,
        &triangles_info,
        &mut groups,
        active_groups,
        &vertex_indices,
        thres_cos,
        &face_indices_buffer,
    );

    // degenerate quads with one good triangle will be fixed by copying a space from
    // the good triangle to the coinciding vertex.
    // all other degenerate triangles will just copy a space from any good triangle
    // with the same welded index in piTriListIn[].
    degen_epilogue(
        geometry,
        &mut tspaces,
        &mut triangles_info,
        &vertex_indices,
        triangles_count as i32,
        total_triangles_count as i32,
    );

    let mut index = 0;
    for face in 0..geometry.num_faces() {
        let verts_0 = geometry.num_vertices_of_face(face);
        if !(verts_0 != 3 && verts_0 != 4) {
            // I've decided to let degenerate triangles and group-with-anythings
            // vary between left/right hand coordinate systems at the vertices.
            // All healthy triangles on the other hand are built to always be either or.

            // set data
            for i in 0..verts_0 {
                let tspace = &tspaces[index];
                let tang = Vec3::new(tspace.os.x, tspace.os.y, tspace.os.z);
                let bitang = Vec3::new(tspace.ot.x, tspace.ot.y, tspace.ot.z);
                geometry.set_tangent(
                    tang.into(),
                    bitang.into(),
                    tspace.mag_s,
                    tspace.mag_t,
                    tspace.orient,
                    face,
                    i,
                );
                index += 1;
            }
        }
    }

    true
}

#[derive(Copy, Clone)]
struct TriangleInfo {
    face_neighbors: [i32; 3],
    assigned_group: [usize; 3],
    os: Vec3,
    ot: Vec3,
    mag_s: f32,
    mag_t: f32,
    /// Index of the face this triangle maps to, in the original faces.
    original_face_index: i32,
    flags: TriangleFlags,
    /// Offset of the first vertex of this triangle, in the original vertices.
    vertex_offset: i32,
    /// Offsets of the vertices of this triangle, relative to the triangle index, last always 0.
    vertex_indices: [u8; 4],
}

bitflags! {
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    struct TriangleFlags: u32 {
        const MARK_DEGENERATE = 0b00000001;
        const QUAD_ONE_DEGEN_TRI = 0b00000010;
        const GROUP_WITH_ANY = 0b00000100;
        const ORIENT_PRESERVING = 0b00001000;
    }
}

impl TriangleInfo {
    fn zero() -> Self {
        Self {
            face_neighbors: [0, 0, 0],
            assigned_group: [usize::MAX, usize::MAX, usize::MAX],
            os: Default::default(),
            ot: Default::default(),
            mag_s: 0.0,
            mag_t: 0.0,
            original_face_index: 0,
            flags: TriangleFlags::empty(),
            vertex_offset: 0,
            vertex_indices: [0, 0, 0, 0],
        }
    }
}

/// Generate initial triangles and vertex indices, from original geometry.
fn generate_initial_vertex_indices<G: Geometry>(
    geometry: &mut G,
    triangles_count: usize,
) -> (Vec<TriangleInfo>, Vec<i32>, i32) {
    let mut triangles_info = vec![TriangleInfo::zero(); triangles_count];
    let mut vertex_indices = vec![0i32; 3 * triangles_count];

    let mut vertex_offset = 0;
    let mut triangle_index = 0;

    for face_index in 0..geometry.num_faces() {
        let face_vertices_count = geometry.num_vertices_of_face(face_index);

        // Only generate for tris or quads
        if face_vertices_count != 3 && face_vertices_count != 4 {
            continue;
        }

        triangles_info[triangle_index].original_face_index = face_index as i32;
        triangles_info[triangle_index].vertex_offset = vertex_offset;

        if face_vertices_count == 3 {
            // For tris

            triangles_info[triangle_index].vertex_indices = [0, 1, 2, 0];
            vertex_indices[triangle_index * 3] = face_vertex_to_index(face_index, 0) as i32;
            vertex_indices[triangle_index * 3 + 1] = face_vertex_to_index(face_index, 1) as i32;
            vertex_indices[triangle_index * 3 + 2] = face_vertex_to_index(face_index, 2) as i32;

            triangle_index += 1;
        } else {
            // For quads

            triangles_info[triangle_index + 1].original_face_index = face_index as i32;
            triangles_info[triangle_index + 1].vertex_offset = vertex_offset;

            let i0 = face_vertex_to_index(face_index, 0);
            let i1 = face_vertex_to_index(face_index, 1);
            let i2 = face_vertex_to_index(face_index, 2);
            let i3 = face_vertex_to_index(face_index, 3);

            // Figure out the best cut for the quad
            let t0 = get_tex_coord(geometry, i0);
            let t1 = get_tex_coord(geometry, i1);
            let t2 = get_tex_coord(geometry, i2);
            let t3 = get_tex_coord(geometry, i3);
            let length_squared_02: f32 = (t2 - t0).length_squared();
            let length_squared_13: f32 = (t3 - t1).length_squared();
            let quad_diagonal_is_02;

            if length_squared_02 < length_squared_13 {
                quad_diagonal_is_02 = true;
            } else if length_squared_13 < length_squared_02 {
                quad_diagonal_is_02 = false;
            } else {
                let p0 = get_position(geometry, i0);
                let p1 = get_position(geometry, i1);
                let p2 = get_position(geometry, i2);
                let p3 = get_position(geometry, i3);
                let length_squared_02_0: f32 = (p2 - p0).length_squared();
                let length_squared_13_0: f32 = (p3 - p1).length_squared();
                quad_diagonal_is_02 = length_squared_13_0 >= length_squared_02_0;
            }

            // Apply indices for the cut we determined
            if quad_diagonal_is_02 {
                triangles_info[triangle_index].vertex_indices = [0, 1, 2, 0];
                vertex_indices[triangle_index * 3] = i0 as i32;
                vertex_indices[triangle_index * 3 + 1] = i1 as i32;
                vertex_indices[triangle_index * 3 + 2] = i2 as i32;
                triangle_index += 1;

                triangles_info[triangle_index].vertex_indices = [0, 2, 3, 0];
                vertex_indices[triangle_index * 3] = i0 as i32;
                vertex_indices[triangle_index * 3 + 1] = i2 as i32;
                vertex_indices[triangle_index * 3 + 2] = i3 as i32;
                triangle_index += 1;
            } else {
                triangles_info[triangle_index].vertex_indices = [0, 1, 3, 0];
                vertex_indices[triangle_index * 3] = i0 as i32;
                vertex_indices[triangle_index * 3 + 1] = i1 as i32;
                vertex_indices[triangle_index * 3 + 2] = i3 as i32;
                triangle_index += 1;

                triangles_info[triangle_index].vertex_indices = [1, 2, 3, 0];
                vertex_indices[triangle_index * 3] = i1 as i32;
                vertex_indices[triangle_index * 3 + 1] = i2 as i32;
                vertex_indices[triangle_index * 3 + 2] = i3 as i32;
                triangle_index += 1;
            }
        }

        vertex_offset += face_vertices_count as i32;
    }

    for face_info in &mut triangles_info {
        face_info.flags = TriangleFlags::empty();
    }

    (triangles_info, vertex_indices, vertex_offset)
}

// Mikktspace uses indices internally to refer to and identify vertices, these utility functions
// make it easier to work with these indices.

/// Generate a vertex index for the Nth vertex of the Nth face.
fn face_vertex_to_index(face_index: usize, vertex: usize) -> usize {
    face_index << 2 | vertex & 0x3
}

/// Reverse of `face_vertex_to_index`.
fn index_to_face_vertex(index: usize) -> (usize, usize) {
    (index >> 2, index & 0x3)
}

fn get_position<G: Geometry>(geometry: &mut G, index: usize) -> Vec3 {
    let (face, vert) = index_to_face_vertex(index);
    geometry.position(face, vert).into()
}

fn get_tex_coord<G: Geometry>(geometry: &mut G, index: usize) -> Vec3 {
    let (face, vert) = index_to_face_vertex(index);
    let tex_coord: Vec2 = geometry.tex_coord(face, vert).into();
    tex_coord.extend(1.0)
}

fn get_normal<G: Geometry>(geometry: &mut G, index: usize) -> Vec3 {
    let (face, vert) = index_to_face_vertex(index);
    geometry.normal(face, vert).into()
}

#[derive(Copy, Clone)]
struct TSpace {
    os: Vec3,
    mag_s: f32,
    ot: Vec3,
    mag_t: f32,
    counter: i32,
    orient: bool,
}

impl TSpace {
    fn zero() -> Self {
        Self {
            os: Default::default(),
            mag_s: 0.0,
            ot: Default::default(),
            mag_t: 0.0,
            counter: 0,
            orient: false,
        }
    }
}

// To avoid visual errors (distortions/unwanted hard edges in lighting), when using sampled normal
// maps, the normal map sampler must use the exact inverse of the pixel shader transformation.
// The most efficient transformation we can possibly do in the pixel shader is
// achieved by using, directly, the "unnormalized" interpolated tangent, bitangent and vertex
// normal: vT, vB and vN.
// pixel shader (fast transform out)
// vNout = normalize( vNt.x * vT + vNt.y * vB + vNt.z * vN );
// where vNt is the tangent space normal. The normal map sampler must likewise use the
// interpolated and "unnormalized" tangent, bitangent and vertex normal to be compliant with the
// pixel shader.
// sampler does (exact inverse of pixel shader):
// float3 row0 = cross(vB, vN);
// float3 row1 = cross(vN, vT);
// float3 row2 = cross(vT, vB);
// float fSign = dot(vT, row0)<0 ? -1 : 1;
// vNt = normalize( fSign * float3(dot(vNout,row0), dot(vNout,row1), dot(vNout,row2)) );
// where vNout is the sampled normal in some chosen 3D space.
//
// Should you choose to reconstruct the bitangent in the pixel shader instead
// of the vertex shader, as explained earlier, then be sure to do this in the normal map sampler
// also.
// Finally, beware of quad triangulations. If the normal map sampler doesn't use the same
// triangulation of
// quads as your renderer then problems will occur since the interpolated tangent spaces will differ
// eventhough the vertex level tangent spaces match. This can be solved either by triangulating
// before
// sampling/exporting or by using the order-independent choice of diagonal for splitting quads
// suggested earlier.
// However, this must be used both by the sampler and your tools/rendering pipeline.
// internal structure

#[derive(Copy, Clone)]
struct Group {
    face_indices_len: usize,
    /// Index of the first face index in the buffer.
    face_indices_index: usize,
    vertex_representative: i32,
    orient_preservering: bool,
}

impl Group {
    fn zero() -> Self {
        Self {
            face_indices_len: 0,
            face_indices_index: usize::MAX,
            vertex_representative: 0,
            orient_preservering: false,
        }
    }
}

#[derive(Clone)]
struct SubGroup {
    faces_count: i32,
    tri_members: Vec<i32>,
}

impl SubGroup {
    fn zero() -> Self {
        Self {
            faces_count: 0,
            tri_members: Vec::new(),
        }
    }
}

#[derive(Copy, Clone)]
struct Edge {
    i0: i32,
    i1: i32,
    f: i32,
}

impl Edge {
    fn zero() -> Self {
        Self { i0: 0, i1: 0, f: 0 }
    }

    fn channel(&self, i: i32) -> i32 {
        [self.i0, self.i1, self.f][i as usize]
    }
}

#[derive(Copy, Clone)]
struct TmpVert {
    vert: [f32; 3],
    index: i32,
}

impl TmpVert {
    fn zero() -> Self {
        Self {
            vert: [0.0, 0.0, 0.0],
            index: 0,
        }
    }
}

fn degen_epilogue<G: Geometry>(
    geometry: &mut G,
    tspaces: &mut [TSpace],
    triangles_info: &mut [TriangleInfo],
    vertex_indices: &[i32],
    triangles_count: i32,
    total_triangles_count: i32,
) {
    // deal with degenerate triangles
    // punishment for degenerate triangles is O(N^2)
    for t in triangles_count..total_triangles_count {
        // degenerate triangles on a quad with one good triangle are skipped
        // here but processed in the next loop
        let skip = triangles_info[t as usize]
            .flags
            .contains(TriangleFlags::QUAD_ONE_DEGEN_TRI);

        if !skip {
            for i in 0..3 {
                let index1: i32 = vertex_indices[(t * 3i32 + i) as usize];
                let mut not_found: bool = true;
                let mut j: i32 = 0i32;
                while not_found && j < 3i32 * triangles_count {
                    let index2: i32 = vertex_indices[j as usize];
                    if index1 == index2 {
                        not_found = false;
                    } else {
                        j += 1;
                    }
                }
                if !not_found {
                    let tri: i32 = j / 3i32;
                    let vert: i32 = j % 3i32;
                    let src_vert: i32 =
                        triangles_info[tri as usize].vertex_indices[vert as usize] as i32;
                    let src_offs: i32 = triangles_info[tri as usize].vertex_offset;
                    let dst_vert: i32 =
                        triangles_info[t as usize].vertex_indices[i as usize] as i32;
                    let dst_offs: i32 = triangles_info[t as usize].vertex_offset;
                    tspaces[(dst_offs + dst_vert) as usize] =
                        tspaces[(src_offs + src_vert) as usize];
                }
            }
        }
    }

    // deal with degenerate quads with one good triangle
    for t in 0..triangles_count {
        // this triangle belongs to a quad where the
        // other triangle is degenerate
        if triangles_info[t as usize]
            .flags
            .contains(TriangleFlags::QUAD_ONE_DEGEN_TRI)
        {
            let pv = triangles_info[t as usize].vertex_indices;
            let flag = (1 << pv[0]) | (1 << pv[1]) | (1 << pv[2]);
            let mut missing_index: i32 = 0i32;
            if flag & 2i32 == 0i32 {
                missing_index = 1i32;
            } else if flag & 4i32 == 0i32 {
                missing_index = 2i32;
            } else if flag & 8i32 == 0i32 {
                missing_index = 3i32;
            }
            let org_f = triangles_info[t as usize].original_face_index;
            let v_dst_p = get_position(
                geometry,
                face_vertex_to_index(org_f as usize, missing_index as usize),
            );

            let mut not_found = true;
            let mut i_0 = 0i32;
            while not_found && i_0 < 3i32 {
                let vert: i32 = pv[i_0 as usize] as i32;
                let v_src_p = get_position(
                    geometry,
                    face_vertex_to_index(org_f as usize, vert as usize),
                );
                if v_src_p == v_dst_p {
                    let offs: i32 = triangles_info[t as usize].vertex_offset;
                    tspaces[(offs + missing_index) as usize] = tspaces[(offs + vert) as usize];
                    not_found = false;
                } else {
                    i_0 += 1;
                }
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn generate_tspaces<G: Geometry>(
    geometry: &mut G,
    tspaces: &mut [TSpace],
    triangles_info: &[TriangleInfo],
    groups: &mut [Group],
    active_groups: i32,
    vertex_indices: &[i32],
    thres_cos: f32,
    face_indices_buffer: &[i32],
) {
    let mut max_faces_count: usize = 0;
    for g in 0..active_groups {
        if max_faces_count < groups[g as usize].face_indices_len {
            max_faces_count = groups[g as usize].face_indices_len;
        }
    }
    if max_faces_count == 0 {
        return;
    }

    let mut sub_group_tspace = vec![TSpace::zero(); max_faces_count];
    let mut uni_sub_groups = vec![SubGroup::zero(); max_faces_count];
    let mut tmp_members = vec![0i32; max_faces_count];

    for g in 0..active_groups {
        let group = &mut groups[g as usize];
        let mut unique_sub_groups = 0;
        for i in 0..group.face_indices_len as i32 {
            let offset = group.face_indices_index + i as usize;
            let f: i32 = face_indices_buffer[offset];

            let mut index: i32 = -1i32;
            let mut tmp_group: SubGroup = SubGroup {
                faces_count: 0,
                tri_members: Vec::new(),
            };
            if triangles_info[f as usize].assigned_group[0] == g as usize {
                index = 0i32;
            } else if triangles_info[f as usize].assigned_group[1] == g as usize {
                index = 1i32;
            } else if triangles_info[f as usize].assigned_group[2] == g as usize {
                index = 2i32;
            }
            let vertex_index = vertex_indices[(f * 3 + index) as usize];
            let n = get_normal(geometry, vertex_index as usize);
            let mut v_os =
                triangles_info[f as usize].os - (n.dot(triangles_info[f as usize].os) * n);
            let mut v_ot =
                triangles_info[f as usize].ot - (n.dot(triangles_info[f as usize].ot) * n);
            if v_not_zero(v_os) {
                v_os = normalize(v_os);
            }
            if v_not_zero(v_ot) {
                v_ot = normalize(v_ot);
            }
            let of_1 = triangles_info[f as usize].original_face_index;
            let mut members = 0;
            for j in 0..group.face_indices_len as i32 {
                let offset = group.face_indices_index + j as usize;
                let t: i32 = face_indices_buffer[offset];

                let of_2: i32 = triangles_info[t as usize].original_face_index;
                let mut v_os2 =
                    triangles_info[t as usize].os - (n.dot(triangles_info[t as usize].os) * n);
                let mut v_ot2 =
                    triangles_info[t as usize].ot - (n.dot(triangles_info[t as usize].ot) * n);
                if v_not_zero(v_os2) {
                    v_os2 = normalize(v_os2);
                }
                if v_not_zero(v_ot2) {
                    v_ot2 = normalize(v_ot2);
                }

                let flags = triangles_info[f as usize].flags | triangles_info[t as usize].flags;
                let any = flags.contains(TriangleFlags::GROUP_WITH_ANY);
                // make sure triangles which belong to the same quad are joined.
                let same_org_face: bool = of_1 == of_2;
                let cos_s: f32 = v_os.dot(v_os2);
                let cos_t: f32 = v_ot.dot(v_ot2);
                if any || same_org_face || cos_s > thres_cos && cos_t > thres_cos {
                    let fresh0 = members;
                    members += 1;
                    tmp_members[fresh0] = t;
                }
            }
            if members > 1 {
                let seed: u32 = 39871946i32 as u32;
                quick_sort(&mut tmp_members, 0i32, (members - 1) as i32, seed);
            }
            tmp_group.faces_count = members as i32;
            tmp_group.tri_members = tmp_members.clone();
            let mut found = false;
            let mut l = 0;
            while l < unique_sub_groups && !found {
                found = compare_sub_groups(&tmp_group, &uni_sub_groups[l]);
                if !found {
                    l += 1;
                }
            }
            if !found {
                uni_sub_groups[unique_sub_groups].faces_count = members as i32;
                uni_sub_groups[unique_sub_groups].tri_members = tmp_group.tri_members.clone();

                sub_group_tspace[unique_sub_groups] = eval_tspace(
                    geometry,
                    &tmp_group.tri_members,
                    members as i32,
                    vertex_indices,
                    triangles_info,
                    group.vertex_representative,
                );
                unique_sub_groups += 1;
            }
            let offs = triangles_info[f as usize].vertex_offset as usize;
            let vert = triangles_info[f as usize].vertex_indices[index as usize] as usize;

            let tspaces_out = &mut tspaces[offs + vert];
            if tspaces_out.counter == 1i32 {
                *tspaces_out = avg_tspace(tspaces_out, &sub_group_tspace[l]);
                tspaces_out.counter = 2i32;
                tspaces_out.orient = group.orient_preservering;
            } else {
                *tspaces_out = sub_group_tspace[l];
                tspaces_out.counter = 1i32;
                tspaces_out.orient = group.orient_preservering;
            }
        }
    }
}

fn avg_tspace(tspace0: &TSpace, tspace1: &TSpace) -> TSpace {
    let mut ts_res: TSpace = TSpace {
        os: Vec3::new(0.0, 0.0, 0.0),
        mag_s: 0.,
        ot: Vec3::new(0.0, 0.0, 0.0),
        mag_t: 0.,
        counter: 0,
        orient: false,
    };
    if tspace0.mag_s == tspace1.mag_s
        && tspace0.mag_t == tspace1.mag_t
        && tspace0.os == tspace1.os
        && tspace0.ot == tspace1.ot
    {
        ts_res.mag_s = tspace0.mag_s;
        ts_res.mag_t = tspace0.mag_t;
        ts_res.os = tspace0.os;
        ts_res.ot = tspace0.ot;
    } else {
        ts_res.mag_s = 0.5f32 * (tspace0.mag_s + tspace1.mag_s);
        ts_res.mag_t = 0.5f32 * (tspace0.mag_t + tspace1.mag_t);
        ts_res.os = tspace0.os + tspace1.os;
        ts_res.ot = tspace0.ot + tspace1.ot;
        if v_not_zero(ts_res.os) {
            ts_res.os = normalize(ts_res.os);
        }
        if v_not_zero(ts_res.ot) {
            ts_res.ot = normalize(ts_res.ot);
        }
    }
    ts_res
}

fn normalize(v: Vec3) -> Vec3 {
    (1.0 / v.length()) * v
}

fn v_not_zero(v: Vec3) -> bool {
    not_zero(v.x) || not_zero(v.y) || not_zero(v.z)
}

#[allow(clippy::excessive_precision)]
fn not_zero(fx: f32) -> bool {
    fx.abs() > 1.17549435e-38f32
}

fn eval_tspace<G: Geometry>(
    geometry: &mut G,
    face_indices_buffer: &[i32],
    faces_count: i32,
    vertex_indices: &[i32],
    triangles_info: &[TriangleInfo],
    vertex_representative: i32,
) -> TSpace {
    let mut res: TSpace = TSpace {
        os: Vec3::new(0.0, 0.0, 0.0),
        mag_s: 0.,
        ot: Vec3::new(0.0, 0.0, 0.0),
        mag_t: 0.,
        counter: 0,
        orient: false,
    };
    let mut angle_sum: f32 = 0i32 as f32;
    res.os.x = 0.0f32;
    res.os.y = 0.0f32;
    res.os.z = 0.0f32;
    res.ot.x = 0.0f32;
    res.ot.y = 0.0f32;
    res.ot.z = 0.0f32;
    res.mag_s = 0i32 as f32;
    res.mag_t = 0i32 as f32;
    for face in 0..faces_count {
        let f: i32 = face_indices_buffer[face as usize];

        // only valid triangles get to add their contribution
        if !triangles_info[f as usize]
            .flags
            .contains(TriangleFlags::GROUP_WITH_ANY)
        {
            let mut i: i32 = -1i32;
            if vertex_indices[(3i32 * f) as usize] == vertex_representative {
                i = 0i32;
            } else if vertex_indices[(3i32 * f + 1i32) as usize] == vertex_representative {
                i = 1i32;
            } else if vertex_indices[(3i32 * f + 2i32) as usize] == vertex_representative {
                i = 2i32;
            }
            let index = vertex_indices[(3i32 * f + i) as usize];
            let n = get_normal(geometry, index as usize);
            let mut v_os =
                triangles_info[f as usize].os - (n.dot(triangles_info[f as usize].os) * n);
            let mut v_ot =
                triangles_info[f as usize].ot - (n.dot(triangles_info[f as usize].ot) * n);
            if v_not_zero(v_os) {
                v_os = normalize(v_os);
            }
            if v_not_zero(v_ot) {
                v_ot = normalize(v_ot);
            }
            let i2 = vertex_indices[(3i32 * f + if i < 2i32 { i + 1i32 } else { 0i32 }) as usize];
            let i1 = vertex_indices[(3i32 * f + i) as usize];
            let i0 = vertex_indices[(3i32 * f + if i > 0i32 { i - 1i32 } else { 2i32 }) as usize];
            let p0 = get_position(geometry, i0 as usize);
            let p1 = get_position(geometry, i1 as usize);
            let p2 = get_position(geometry, i2 as usize);
            let v1 = p0 - p1;
            let v2 = p2 - p1;
            let mut v1 = v1 - (n.dot(v1) * n);
            if v_not_zero(v1) {
                v1 = normalize(v1);
            }
            let mut v2 = v2 - (n.dot(v2) * n);
            if v_not_zero(v2) {
                v2 = normalize(v2);
            }
            let cos = v1.dot(v2);

            let cos = if cos > 1.0 {
                1.0
            } else if cos < -1.0 {
                -1.0
            } else {
                cos
            };
            let angle = (cos as f64).acos() as f32;
            let mag_s = triangles_info[f as usize].mag_s;
            let mag_t = triangles_info[f as usize].mag_t;
            res.os += angle * v_os;
            res.ot += angle * v_ot;
            res.mag_s += angle * mag_s;
            res.mag_t += angle * mag_t;
            angle_sum += angle;
        }
    }
    if v_not_zero(res.os) {
        res.os = normalize(res.os);
    }
    if v_not_zero(res.ot) {
        res.ot = normalize(res.ot);
    }
    if angle_sum > 0i32 as f32 {
        res.mag_s /= angle_sum;
        res.mag_t /= angle_sum;
    }
    res
}

fn compare_sub_groups(pg1: &SubGroup, pg2: &SubGroup) -> bool {
    let mut still_same: bool = true;
    let mut i = 0;
    if pg1.faces_count != pg2.faces_count {
        return false;
    }
    while i < pg1.faces_count as usize && still_same {
        still_same = pg1.tri_members[i] == pg2.tri_members[i];
        if still_same {
            i += 1;
        }
    }
    still_same
}

fn quick_sort(sort_buffer: &mut [i32], left: i32, right: i32, mut seed: u32) {
    // Random
    let mut t: u32 = seed & 31i32 as u32;
    t = seed.rotate_left(t) | seed.rotate_right((32i32 as u32).wrapping_sub(t));
    seed = seed.wrapping_add(t).wrapping_add(3i32 as u32);
    // Random end

    let mut l = left;
    let mut r = right;
    let n = r - l + 1i32;
    let index = seed.wrapping_rem(n as u32) as i32;
    let mid = sort_buffer[(index + l) as usize];
    loop {
        while sort_buffer[l as usize] < mid {
            l += 1;
        }
        while sort_buffer[r as usize] > mid {
            r -= 1;
        }
        if l <= r {
            sort_buffer.swap(l as usize, r as usize);
            l += 1;
            r -= 1;
        }
        if l > r {
            break;
        }
    }
    if left < r {
        quick_sort(sort_buffer, left, r, seed);
    }
    if l < right {
        quick_sort(sort_buffer, l, right, seed);
    }
}

fn build_4_rule_groups(
    triangles_info: &mut [TriangleInfo],
    groups: &mut [Group],
    face_indices_buffer: &mut [i32],
    vertex_indices: &[i32],
    triangles_count: i32,
) -> i32 {
    let max_groups: i32 = triangles_count * 3i32;
    let mut active_groups: i32 = 0i32;
    let mut offset: i32 = 0i32;
    for f in 0..triangles_count {
        for i in 0..3i32 {
            // if not assigned to a group
            if !triangles_info[f as usize]
                .flags
                .contains(TriangleFlags::GROUP_WITH_ANY)
                && triangles_info[f as usize].assigned_group[i as usize] == usize::MAX
            {
                let vert_index: i32 = vertex_indices[(f * 3i32 + i) as usize];
                assert!(active_groups < max_groups);

                let group_index = active_groups as usize;
                triangles_info[f as usize].assigned_group[i as usize] = group_index;
                let group = &mut groups[group_index];
                group.vertex_representative = vert_index;
                group.orient_preservering = triangles_info[f as usize]
                    .flags
                    .contains(TriangleFlags::ORIENT_PRESERVING);
                group.face_indices_len = 0;
                group.face_indices_index = offset as usize;
                active_groups += 1;

                add_tri_to_group(face_indices_buffer, group, f);
                let or_pre = triangles_info[f as usize]
                    .flags
                    .contains(TriangleFlags::ORIENT_PRESERVING);
                let neigh_index_l = triangles_info[f as usize].face_neighbors[i as usize];
                let neigh_index_r = triangles_info[f as usize].face_neighbors
                    [(if i > 0i32 { i - 1i32 } else { 2i32 }) as usize];
                if neigh_index_l >= 0i32 {
                    let answer: bool = assign_recur(
                        vertex_indices,
                        triangles_info,
                        neigh_index_l,
                        group,
                        group_index,
                        face_indices_buffer,
                    );
                    let or_pre2: bool = triangles_info[neigh_index_l as usize]
                        .flags
                        .contains(TriangleFlags::ORIENT_PRESERVING);
                    let diff: bool = or_pre != or_pre2;
                    assert!(answer || diff);
                }
                if neigh_index_r >= 0i32 {
                    let answer: bool = assign_recur(
                        vertex_indices,
                        triangles_info,
                        neigh_index_r,
                        group,
                        group_index,
                        face_indices_buffer,
                    );
                    let or_pre_2: bool = triangles_info[neigh_index_r as usize]
                        .flags
                        .contains(TriangleFlags::ORIENT_PRESERVING);
                    let diff: bool = or_pre != or_pre_2;
                    assert!(answer || diff);
                }

                // update offset
                offset += group.face_indices_len as i32;

                // since the groups are disjoint a triangle can never
                // belong to more than 3 groups. Subsequently something
                // is completely screwed if this assertion ever hits.
                assert!(offset <= max_groups);
            }
        }
    }

    active_groups
}

fn assign_recur(
    vertex_indices: &[i32],
    triangles_info: &mut [TriangleInfo],
    my_tri_index: i32,
    group: &mut Group,
    group_index: usize,
    face_indices_buffer: &mut [i32],
) -> bool {
    let my_tri_info = &mut triangles_info[my_tri_index as usize];

    // track down vertex
    let vert_rep: i32 = group.vertex_representative;
    let offset = 3 * my_tri_index as usize;
    let mut i: i32 = -1;
    if vertex_indices[offset] == vert_rep {
        i = 0;
    } else if vertex_indices[offset + 1] == vert_rep {
        i = 1;
    } else if vertex_indices[offset + 2] == vert_rep {
        i = 2;
    }
    assert!((0..3).contains(&i));

    // early out
    if my_tri_info.assigned_group[i as usize] == group_index {
        return true;
    }
    if !my_tri_info.assigned_group[i as usize] == usize::MAX {
        return false;
    }

    if my_tri_info.flags.contains(TriangleFlags::GROUP_WITH_ANY) {
        // first to group with a group-with-anything triangle
        // determines it's orientation.
        // This is the only existing order dependency in the code!!
        if my_tri_info.assigned_group[0] == usize::MAX
            && my_tri_info.assigned_group[1] == usize::MAX
            && my_tri_info.assigned_group[2] == usize::MAX
        {
            my_tri_info
                .flags
                .set(TriangleFlags::ORIENT_PRESERVING, group.orient_preservering);
        }
    }

    let orient: bool = my_tri_info.flags.contains(TriangleFlags::ORIENT_PRESERVING);
    if orient != group.orient_preservering {
        return false;
    }

    add_tri_to_group(face_indices_buffer, group, my_tri_index);
    my_tri_info.assigned_group[i as usize] = group_index;

    let neigh_index_l = my_tri_info.face_neighbors[i as usize];
    let neigh_index_r = my_tri_info.face_neighbors[(if i > 0 { i - 1 } else { 2 }) as usize];
    if neigh_index_l >= 0 {
        assign_recur(
            vertex_indices,
            triangles_info,
            neigh_index_l,
            group,
            group_index,
            face_indices_buffer,
        );
    }
    if neigh_index_r >= 0 {
        assign_recur(
            vertex_indices,
            triangles_info,
            neigh_index_r,
            group,
            group_index,
            face_indices_buffer,
        );
    }

    true
}

fn add_tri_to_group(face_indices_buffer: &mut [i32], group: &mut Group, tri_index: i32) {
    let offset = group.face_indices_index + group.face_indices_len;
    face_indices_buffer[offset] = tri_index;
    group.face_indices_len += 1;
}

fn init_tri_info<G: Geometry>(
    geometry: &mut G,
    triangles_info: &mut [TriangleInfo],
    vertex_indices: &[i32],
    triangles_count: usize,
) {
    let mut t = 0;

    // generate neighbor info list
    #[allow(clippy::needless_range_loop)]
    for f in 0..triangles_count {
        for i in 0..3 {
            triangles_info[f].face_neighbors[i as usize] = -1i32;
            triangles_info[f].assigned_group[i as usize] = usize::MAX;
            triangles_info[f].os.x = 0.0f32;
            triangles_info[f].os.y = 0.0f32;
            triangles_info[f].os.z = 0.0f32;
            triangles_info[f].ot.x = 0.0f32;
            triangles_info[f].ot.y = 0.0f32;
            triangles_info[f].ot.z = 0.0f32;
            triangles_info[f].mag_s = 0i32 as f32;
            triangles_info[f].mag_t = 0i32 as f32;

            // assumed bad
            triangles_info[f].flags |= TriangleFlags::GROUP_WITH_ANY;
        }
    }

    // evaluate first order derivatives
    for f in 0..triangles_count {
        let v1 = get_position(geometry, vertex_indices[f * 3] as usize);
        let v2 = get_position(geometry, vertex_indices[f * 3 + 1] as usize);
        let v3 = get_position(geometry, vertex_indices[f * 3 + 2] as usize);
        let t1 = get_tex_coord(geometry, vertex_indices[f * 3] as usize);
        let t2 = get_tex_coord(geometry, vertex_indices[f * 3 + 1] as usize);
        let t3 = get_tex_coord(geometry, vertex_indices[f * 3 + 2] as usize);
        let t21x: f32 = t2.x - t1.x;
        let t21y: f32 = t2.y - t1.y;
        let t31x: f32 = t3.x - t1.x;
        let t31y: f32 = t3.y - t1.y;
        let d1 = v2 - v1;
        let d2 = v3 - v1;
        let signed_arena_stx2: f32 = t21x * t31y - t21y * t31x;
        let os = (t31y * d1) - (t21y * d2);
        let ot = (-t31x * d1) + (t21x * d2);

        triangles_info[f]
            .flags
            .set(TriangleFlags::ORIENT_PRESERVING, signed_arena_stx2 > 0f32);

        if not_zero(signed_arena_stx2) {
            let abs_arena: f32 = signed_arena_stx2.abs();
            let len_os: f32 = os.length();
            let len_ot: f32 = ot.length();
            let s: f32 = if !triangles_info[f]
                .flags
                .contains(TriangleFlags::ORIENT_PRESERVING)
            {
                -1.0f32
            } else {
                1.0f32
            };
            if not_zero(len_os) {
                triangles_info[f].os = (s / len_os) * os;
            }
            if not_zero(len_ot) {
                triangles_info[f].ot = (s / len_ot) * ot;
            }

            // evaluate magnitudes prior to normalization of vOs and vOt
            triangles_info[f].mag_s = len_os / abs_arena;
            triangles_info[f].mag_t = len_ot / abs_arena;

            // if this is a good triangle
            if not_zero(triangles_info[f].mag_s) && not_zero(triangles_info[f].mag_t) {
                triangles_info[f]
                    .flags
                    .remove(TriangleFlags::GROUP_WITH_ANY);
            }
        }
    }

    // force otherwise healthy quads to a fixed orientation
    while t < triangles_count - 1 {
        let fo_a: i32 = triangles_info[t].original_face_index;
        let fo_b: i32 = triangles_info[t + 1].original_face_index;
        if fo_a == fo_b {
            let is_deg_a: bool = triangles_info[t]
                .flags
                .contains(TriangleFlags::MARK_DEGENERATE);
            let is_deg_b: bool = triangles_info[t + 1]
                .flags
                .contains(TriangleFlags::MARK_DEGENERATE);

            // bad triangles should already have been removed by
            // DegenPrologue(), but just in case check bIsDeg_a and bIsDeg_a are false
            if !(is_deg_a || is_deg_b) {
                let orient_a: bool = triangles_info[t]
                    .flags
                    .contains(TriangleFlags::ORIENT_PRESERVING);
                let orient_b: bool = triangles_info[t + 1]
                    .flags
                    .contains(TriangleFlags::ORIENT_PRESERVING);

                // if this happens the quad has extremely bad mapping!!
                if orient_a != orient_b {
                    let mut choose_orient_first_tri: bool = false;
                    if triangles_info[t + 1]
                        .flags
                        .contains(TriangleFlags::GROUP_WITH_ANY)
                        || calc_tex_area(geometry, vertex_indices, t * 3)
                            >= calc_tex_area(geometry, vertex_indices, (t + 1) * 3)
                    {
                        choose_orient_first_tri = true;
                    }

                    // force match
                    let t0 = if choose_orient_first_tri { t } else { t + 1 };
                    let t1_0 = if choose_orient_first_tri { t + 1 } else { t };

                    triangles_info[t1_0].flags.set(
                        TriangleFlags::ORIENT_PRESERVING,
                        triangles_info[t0]
                            .flags
                            .contains(TriangleFlags::ORIENT_PRESERVING),
                    );
                }
            }
            t += 2;
        } else {
            t += 1;
        }
    }

    let mut edges = vec![Edge::zero(); triangles_count * 3];
    build_neighbors_fast(
        triangles_info,
        &mut edges,
        vertex_indices,
        triangles_count as i32,
    );
}

fn build_neighbors_fast(
    triangles_info: &mut [TriangleInfo],
    edges: &mut [Edge],
    vertex_indices: &[i32],
    triangles_count: i32,
) {
    // build array of edges
    // could replace with a random seed?
    let seed: u32 = 39871946i32 as u32;
    for f in 0..triangles_count {
        for i in 0..3 {
            let i0: i32 = vertex_indices[(f * 3i32 + i) as usize];
            let i1: i32 =
                vertex_indices[(f * 3i32 + if i < 2i32 { i + 1i32 } else { 0i32 }) as usize];
            edges[(f * 3i32 + i) as usize].i0 = if i0 < i1 { i0 } else { i1 };
            edges[(f * 3i32 + i) as usize].i1 = if i0 >= i1 { i0 } else { i1 };
            edges[(f * 3i32 + i) as usize].f = f;
        }
    }

    // sort over all edges by i0, this is the pricy one.
    quick_sort_edges(edges, 0i32, triangles_count * 3i32 - 1i32, 0i32, seed);

    // sub sort over i1, should be fast.
    // could replace this with a 64 bit int sort over (i0,i1)
    // with i0 as msb in the quicksort call above.
    let entries = triangles_count * 3;
    let mut cur_start_index = 0;
    for i in 1..entries {
        if edges[cur_start_index as usize].i0 != edges[i as usize].i0 {
            let l: i32 = cur_start_index;
            let r: i32 = i - 1i32;
            cur_start_index = i;
            quick_sort_edges(edges, l, r, 1i32, seed);
        }
    }

    // sub sort over f, which should be fast.
    // this step is to remain compliant with BuildNeighborsSlow() when
    // more than 2 triangles use the same edge (such as a butterfly topology).
    cur_start_index = 0i32;
    for i in 1..entries {
        if edges[cur_start_index as usize].i0 != edges[i as usize].i0
            || edges[cur_start_index as usize].i1 != edges[i as usize].i1
        {
            let l_0: i32 = cur_start_index;
            let r_0: i32 = i - 1;
            cur_start_index = i;
            quick_sort_edges(edges, l_0, r_0, 2, seed);
        }
    }

    // pair up, adjacent triangles
    for i in 0..entries {
        let i0_0: i32 = edges[i as usize].i0;
        let i1_0: i32 = edges[i as usize].i1;
        let f_0: i32 = edges[i as usize].f;
        let mut i0_a: i32 = 0;
        let mut i1_a: i32 = 0;
        let mut edgenum_a: i32 = 0;
        let mut edgenum_b: i32 = 0;
        get_edge(
            &mut i0_a,
            &mut i1_a,
            &mut edgenum_a,
            vertex_indices,
            (f_0 * 3i32) as usize,
            i0_0,
            i1_0,
        );
        let unassigned_a = triangles_info[f_0 as usize].face_neighbors[edgenum_a as usize] == -1;
        if unassigned_a {
            // get true index ordering
            let mut j: i32 = i + 1i32;
            let mut not_found: bool = true;
            while j < entries
                && i0_0 == edges[j as usize].i0
                && i1_0 == edges[j as usize].i1
                && not_found
            {
                let mut i0_b: i32 = 0;
                let mut i1_b: i32 = 0;
                let t = edges[j as usize].f;
                get_edge(
                    &mut i1_b,
                    &mut i0_b,
                    &mut edgenum_b,
                    vertex_indices,
                    (t * 3i32) as usize,
                    edges[j as usize].i0,
                    edges[j as usize].i1,
                );
                let unassigned_b =
                    triangles_info[t as usize].face_neighbors[edgenum_b as usize] == -1;
                if i0_a == i0_b && i1_a == i1_b && unassigned_b {
                    not_found = false;
                } else {
                    j += 1;
                }
            }
            if !not_found {
                let t_0: i32 = edges[j as usize].f;
                triangles_info[f_0 as usize].face_neighbors[edgenum_a as usize] = t_0;
                triangles_info[t_0 as usize].face_neighbors[edgenum_b as usize] = f_0;
            }
        }
    }
}

fn get_edge(
    i0_out: &mut i32,
    i1_out: &mut i32,
    edgenum_out: &mut i32,
    indices: &[i32],
    offset: usize,
    i0_in: i32,
    i1_in: i32,
) {
    let indices = &indices[offset..offset + 3];

    *edgenum_out = -1i32;
    if indices[0] == i0_in || indices[0] == i1_in {
        if indices[1] == i0_in || indices[1] == i1_in {
            *edgenum_out = 0i32;
            *i0_out = indices[0];
            *i1_out = indices[1];
        } else {
            *edgenum_out = 2i32;
            *i0_out = indices[2];
            *i1_out = indices[0];
        }
    } else {
        *edgenum_out = 1i32;
        *i0_out = indices[1];
        *i1_out = indices[2];
    };
}

fn quick_sort_edges(sort_buffer: &mut [Edge], left: i32, right: i32, channel: i32, mut seed: u32) {
    // early out
    let elems: i32 = right - left + 1i32;
    if elems < 2 {
        return;
    }
    if elems == 2 {
        if sort_buffer[left as usize].channel(channel)
            > sort_buffer[right as usize].channel(channel)
        {
            sort_buffer.swap(left as usize, right as usize);
        }
        return;
    }

    // Random
    let mut t = seed & 31i32 as u32;
    t = seed.rotate_left(t) | seed.rotate_right((32i32 as u32).wrapping_sub(t));
    seed = seed.wrapping_add(t).wrapping_add(3i32 as u32);
    // Random end

    let mut l = left;
    let mut r = right;
    let n = r - l + 1i32;
    let index = seed.wrapping_rem(n as u32) as i32;
    let mid = sort_buffer[(index + l) as usize].channel(channel);
    loop {
        while sort_buffer[l as usize].channel(channel) < mid {
            l += 1;
        }
        while sort_buffer[r as usize].channel(channel) > mid {
            r -= 1;
        }
        if l <= r {
            sort_buffer.swap(l as usize, r as usize);
            l += 1;
            r -= 1;
        }
        if l > r {
            break;
        }
    }
    if left < r {
        quick_sort_edges(sort_buffer, left, r, channel, seed);
    }
    if l < right {
        quick_sort_edges(sort_buffer, l, right, channel, seed);
    };
}

// returns the texture area times 2
fn calc_tex_area<G: Geometry>(geometry: &mut G, indices: &[i32], start: usize) -> f32 {
    let t1 = get_tex_coord(geometry, indices[start] as usize);
    let t2 = get_tex_coord(geometry, indices[start + 1] as usize);
    let t3 = get_tex_coord(geometry, indices[start + 2] as usize);
    let t21x: f32 = t2.x - t1.x;
    let t21y: f32 = t2.y - t1.y;
    let t31x: f32 = t3.x - t1.x;
    let t31y: f32 = t3.y - t1.y;
    let signed_area_stx2: f32 = t21x * t31y - t21y * t31x;
    if signed_area_stx2 < 0i32 as f32 {
        -signed_area_stx2
    } else {
        signed_area_stx2
    }
}

// degen triangles
fn degen_prologue(
    triangles_info: &mut [TriangleInfo],
    vertex_indices: &mut [i32],
    triangles_count: i32,
    total_triangles_count: i32,
) {
    // locate quads with only one good triangle
    let mut t: i32 = 0i32;
    while t < total_triangles_count - 1i32 {
        let fo_a: i32 = triangles_info[t as usize].original_face_index;
        let fo_b: i32 = triangles_info[(t + 1i32) as usize].original_face_index;
        if fo_a == fo_b {
            let is_deg_a: bool = triangles_info[t as usize]
                .flags
                .contains(TriangleFlags::MARK_DEGENERATE);
            let is_deg_b: bool = triangles_info[(t + 1) as usize]
                .flags
                .contains(TriangleFlags::MARK_DEGENERATE);
            if is_deg_a ^ is_deg_b {
                triangles_info[t as usize].flags |= TriangleFlags::QUAD_ONE_DEGEN_TRI;
                triangles_info[(t + 1i32) as usize].flags |= TriangleFlags::QUAD_ONE_DEGEN_TRI;
            }
            t += 2i32;
        } else {
            t += 1;
        }
    }

    // reorder list so all degen triangles are moved to the back
    // without reordering the good triangles
    let mut next_good_triangle_search_index = 1i32;
    t = 0i32;
    let mut still_finding_good_ones = true;
    while t < triangles_count && still_finding_good_ones {
        let is_good: bool = !triangles_info[t as usize]
            .flags
            .contains(TriangleFlags::MARK_DEGENERATE);
        if is_good {
            if next_good_triangle_search_index < t + 2i32 {
                next_good_triangle_search_index = t + 2i32;
            }
        } else {
            // search for the first good triangle.
            let mut just_a_degenerate: bool = true;
            while just_a_degenerate && next_good_triangle_search_index < total_triangles_count {
                let is_good_0: bool = !triangles_info[next_good_triangle_search_index as usize]
                    .flags
                    .contains(TriangleFlags::MARK_DEGENERATE);
                if is_good_0 {
                    just_a_degenerate = false;
                } else {
                    next_good_triangle_search_index += 1;
                }
            }
            let t0 = t;
            let t1 = next_good_triangle_search_index;
            next_good_triangle_search_index += 1;

            // swap triangle t0 and t1
            if !just_a_degenerate {
                for i in 0..3 {
                    vertex_indices.swap((t0 * 3 + i) as usize, (t1 * 3 + i) as usize);
                }
                triangles_info.swap(t0 as usize, t1 as usize);
            } else {
                still_finding_good_ones = false;
            }
        }
        if still_finding_good_ones {
            t += 1;
        }
    }
}

fn generate_shared_vertices_index_list<G: Geometry>(
    vertex_indices: &mut [i32],
    geometry: &mut G,
    triangles_count: usize,
) {
    let mut min = get_position(geometry, 0);
    let mut max = min;

    #[allow(clippy::needless_range_loop)]
    for i in 1..triangles_count * 3 {
        let index: i32 = vertex_indices[i];
        let p = get_position(geometry, index as usize);
        if min.x > p.x {
            min.x = p.x;
        } else if max.x < p.x {
            max.x = p.x;
        }
        if min.y > p.y {
            min.y = p.y;
        } else if max.y < p.y {
            max.y = p.y;
        }
        if min.z > p.z {
            min.z = p.z;
        } else if max.z < p.z {
            max.z = p.z;
        }
    }
    let dim = max - min;
    let mut channel = 0i32;
    let mut f_min = min.x;
    let mut f_max = max.x;
    if dim.y > dim.x && dim.y > dim.z {
        channel = 1i32;
        f_min = min.y;
        f_max = max.y;
    } else if dim.z > dim.x {
        channel = 2i32;
        f_min = min.z;
        f_max = max.z;
    }

    let mut hash_table = vec![0i32; triangles_count * 3];
    let mut hash_offsets = vec![0i32; G_CELLS];
    let mut hash_count = vec![0i32; G_CELLS];
    let mut hash_count2 = vec![0i32; G_CELLS];

    #[allow(clippy::needless_range_loop)]
    for i in 0..triangles_count * 3 {
        let index_0: i32 = vertex_indices[i];
        let p_0 = get_position(geometry, index_0 as usize);
        let val: f32 = if channel == 0i32 {
            p_0.x
        } else if channel == 1i32 {
            p_0.y
        } else {
            p_0.z
        };
        let cell = find_grid_cell(f_min, f_max, val);
        hash_count[cell] += 1;
    }
    hash_offsets[0] = 0i32;
    let mut k = 1;
    while k < G_CELLS {
        hash_offsets[k] = hash_offsets[k - 1] + hash_count[k - 1];
        k += 1;
    }
    #[allow(clippy::needless_range_loop)]
    for i in 0..triangles_count * 3 {
        let index_1: i32 = vertex_indices[i];
        let p_1 = get_position(geometry, index_1 as usize);
        let val_0: f32 = if channel == 0i32 {
            p_1.x
        } else if channel == 1i32 {
            p_1.y
        } else {
            p_1.z
        };
        let cell_0 = find_grid_cell(f_min, f_max, val_0);
        hash_table[(hash_offsets[cell_0] + hash_count2[cell_0]) as usize] = i as i32;
        hash_count2[cell_0] += 1;
    }
    k = 0;
    while k < G_CELLS {
        k += 1;
    }
    let mut max_count = hash_count[0] as usize;
    k = 1;
    while k < G_CELLS {
        if max_count < hash_count[k] as usize {
            max_count = hash_count[k] as usize;
        }
        k += 1;
    }
    let mut tmp_vert = vec![TmpVert::zero(); max_count];
    k = 0;
    while k < G_CELLS {
        // extract table of cell k and amount of entries in it
        let table_0_offset = hash_offsets[k] as usize;
        let entries = hash_count[k] as usize;
        if entries >= 2 {
            let mut e = 0;
            while e < entries {
                let i_0: i32 = hash_table[table_0_offset + e];
                let p_2 = get_position(geometry, vertex_indices[i_0 as usize] as usize);
                tmp_vert[e].vert[0] = p_2.x;
                tmp_vert[e].vert[1] = p_2.y;
                tmp_vert[e].vert[2] = p_2.z;
                tmp_vert[e].index = i_0;
                e += 1;
            }
            merge_verts_fast(
                geometry,
                vertex_indices,
                &mut tmp_vert,
                0i32,
                (entries - 1) as i32,
            );
        }
        k += 1;
    }
}

fn merge_verts_fast<G: Geometry>(
    geometry: &mut G,
    vertex_indices: &mut [i32],
    tmp_vert: &mut [TmpVert],
    l_in: i32,
    r_in: i32,
) {
    // make bbox
    let mut min: [f32; 3] = [0.; 3];
    let mut max: [f32; 3] = [0.; 3];
    for c in 0..3 {
        min[c as usize] = tmp_vert[l_in as usize].vert[c as usize];
        max[c as usize] = min[c as usize];
    }
    let mut l = l_in + 1i32;
    while l <= r_in {
        for c in 0..3 {
            if min[c as usize] > tmp_vert[l as usize].vert[c as usize] {
                min[c as usize] = tmp_vert[l as usize].vert[c as usize];
            } else if max[c as usize] < tmp_vert[l as usize].vert[c as usize] {
                max[c as usize] = tmp_vert[l as usize].vert[c as usize];
            }
        }
        l += 1;
    }
    let dx = max[0usize] - min[0usize];
    let dy = max[1usize] - min[1usize];
    let dz = max[2usize] - min[2usize];
    let mut channel = 0i32;
    if dy > dx && dy > dz {
        channel = 1i32;
    } else if dz > dx {
        channel = 2i32;
    }
    let sep = 0.5f32 * (max[channel as usize] + min[channel as usize]);
    if sep >= max[channel as usize] || sep <= min[channel as usize] {
        l = l_in;
        while l <= r_in {
            let i: i32 = tmp_vert[l as usize].index;
            let index: i32 = vertex_indices[i as usize];
            let p = get_position(geometry, index as usize);
            let n = get_normal(geometry, index as usize);
            let t = get_tex_coord(geometry, index as usize);
            let mut not_found: bool = true;
            let mut l2: i32 = l_in;
            let mut i2rec: i32 = -1i32;
            while l2 < l && not_found {
                let i2: i32 = tmp_vert[l2 as usize].index;
                let index2: i32 = vertex_indices[i2 as usize];
                let p2 = get_position(geometry, index2 as usize);
                let n2 = get_normal(geometry, index2 as usize);
                let t2 = get_tex_coord(geometry, index2 as usize);
                i2rec = i2;
                if p.x == p2.x
                    && p.y == p2.y
                    && p.z == p2.z
                    && n.x == n2.x
                    && n.y == n2.y
                    && n.z == n2.z
                    && t.x == t2.x
                    && t.y == t2.y
                    && t.z == t2.z
                {
                    not_found = false;
                } else {
                    l2 += 1;
                }
            }
            if !not_found {
                vertex_indices[i as usize] = vertex_indices[i2rec as usize];
            }
            l += 1;
        }
    } else {
        let mut l: i32 = l_in;
        let mut r: i32 = r_in;
        while l < r {
            let mut ready_left_swap: bool = false;
            let mut ready_right_swap: bool = false;
            while !ready_left_swap && l < r {
                ready_left_swap = tmp_vert[l as usize].vert[channel as usize] >= sep;
                if !ready_left_swap {
                    l += 1;
                }
            }
            while !ready_right_swap && l < r {
                ready_right_swap = tmp_vert[r as usize].vert[channel as usize] < sep;
                if !ready_right_swap {
                    r -= 1;
                }
            }
            if ready_left_swap && ready_right_swap {
                tmp_vert.swap(l as usize, r as usize);
                l += 1;
                r -= 1;
            }
        }
        if l == r {
            let ready_right_swap_0: bool = tmp_vert[r as usize].vert[channel as usize] < sep;
            if ready_right_swap_0 {
                l += 1;
            } else {
                r -= 1;
            }
        }
        if l_in < r {
            merge_verts_fast(geometry, vertex_indices, tmp_vert, l_in, r);
        }
        if l < r_in {
            merge_verts_fast(geometry, vertex_indices, tmp_vert, l, r_in);
        }
    };
}

const G_CELLS: usize = 2048;

// it is IMPORTANT that this function is called to evaluate the hash since
// inlining could potentially reorder instructions and generate different
// results for the same effective input value fVal.
#[inline(never)]
fn find_grid_cell(min: f32, max: f32, val: f32) -> usize {
    let f_index = G_CELLS as f32 * ((val - min) / (max - min));
    let i_index = f_index as isize;
    if i_index < G_CELLS as isize {
        if i_index >= 0 {
            i_index as usize
        } else {
            0
        }
    } else {
        G_CELLS - 1
    }
}
