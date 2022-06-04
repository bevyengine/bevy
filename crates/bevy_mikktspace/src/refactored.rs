//! The contents of this file are a combination of transpilation and human
//! modification to Morten S. Mikkelsen's original tangent space algorithm
//! implementation written in C. The original source code can be found at
//! <https://archive.blender.org/wiki/index.php/Dev:Shading/Tangent_Space_Normal_Maps>
//! and includes the following licence:
//!
//! Copyright (C) 2011 by Morten S. Mikkelsen
//!
//! This software is provided 'as-is', without any express or implied
//! warranty.  In no event will the authors be held liable for any damages
//! arising from the use of this software.
//!
//! Permission is granted to anyone to use this software for any purpose,
//! including commercial applications, and to alter it and redistribute it
//! freely, subject to the following restrictions:
//!
//! 1. The origin of this software must not be misrepresented; you must not
//! claim that you wrote the original software. If you use this software
//! in a product, an acknowledgment in the product documentation would be
//! appreciated but is not required.
//!
//! 2. Altered source versions must be plainly marked as such, and must not be
//! misrepresented as being the original software.
//!
//! 3. This notice may not be removed or altered from any source distribution.

use std::{
    cmp::Ordering,
    collections::{btree_map::Entry, BTreeMap},
};

use glam::{Vec2, Vec3};

use crate::Geometry;

pub fn generate_tangents(geometry: &mut impl Geometry) {
    let triangle_count = geometry.num_faces();
    if triangle_count == 0 {
        // Nothing to do - further steps can now assume at least one face exists.
        return;
    }
    let vertex_indices = find_shared_vertices(geometry);
    // Ignore degenerate triangles for now
}

fn find_shared_vertices(geometry: &impl Geometry) -> Vec<usize> {
    let mut vertex_indices = BTreeMap::<VertexInfo, usize>::new();
    let mut vertices = Vec::<usize>::new();
    for face in 0..geometry.num_faces() {
        for vert in 0..3 {
            let info = VertexInfo {
                position: geometry.position(face, vert),
                normal: geometry.normal(face, vert),
                tex_coord: geometry.tex_coord(face, vert),
            };
            match vertex_indices.entry(info) {
                Entry::Vacant(vacant) => {
                    let new_idx = vertices.len();
                    vacant.insert(new_idx);
                    vertices.push(new_idx);
                }
                Entry::Occupied(o) => vertices.push(*o.get()),
            }
        }
    }
    vertices
}

fn build_groups() {}

fn build_neighbours() {}

#[derive(PartialEq, Clone, Copy)]
struct VertexInfo {
    position: Vec3,
    normal: Vec3,
    tex_coord: Vec2,
}

impl Eq for VertexInfo {}

impl Ord for VertexInfo {
    fn cmp(&self, other: &Self) -> Ordering {
        self.partial_cmp(other).unwrap_or(Ordering::Less)
    }
}

impl PartialOrd for VertexInfo {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        match self.position.partial_cmp(&other.position) {
            Some(Ordering::Equal) => {}
            ord => return ord,
        }
        match self.normal.partial_cmp(&other.normal) {
            Some(Ordering::Equal) => {}
            ord => return ord,
        }
        self.tex_coord.partial_cmp(&other.tex_coord)
    }
}
