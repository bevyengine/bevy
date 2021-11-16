use wgpu::PrimitiveTopology;

use crate::mesh::{Indices, Mesh};
use std::f32::consts::PI;

/// A sphere made of sectors and stacks.
#[allow(clippy::upper_case_acronyms)]
#[derive(Debug, Clone, Copy)]
pub struct UVSphere {
    /// The radius of the sphere.
    pub radius: f32,
    /// Longitudinal sectors
    pub sectors: usize,
    /// Latitudinal stacks
    pub stacks: usize,
}

impl Default for UVSphere {
    fn default() -> Self {
        Self {
            radius: 1.0,
            sectors: 36,
            stacks: 18,
        }
    }
}

impl From<UVSphere> for Mesh {
    fn from(sphere: UVSphere) -> Self {
        // Largely inspired from http://www.songho.ca/opengl/gl_sphere.html

        let sectors = sphere.sectors as f32;
        let stacks = sphere.stacks as f32;
        let length_inv = 1. / sphere.radius;
        let sector_step = 2. * PI / sectors;
        let stack_step = PI / stacks;

        let mut vertices: Vec<[f32; 3]> = Vec::with_capacity(sphere.stacks * sphere.sectors);
        let mut normals: Vec<[f32; 3]> = Vec::with_capacity(sphere.stacks * sphere.sectors);
        let mut uvs: Vec<[f32; 2]> = Vec::with_capacity(sphere.stacks * sphere.sectors);
        let mut indices: Vec<u32> = Vec::with_capacity(sphere.stacks * sphere.sectors * 2 * 3);

        for i in 0..sphere.stacks + 1 {
            let stack_angle = PI / 2. - (i as f32) * stack_step;
            let xy = sphere.radius * stack_angle.cos();
            let z = sphere.radius * stack_angle.sin();

            for j in 0..sphere.sectors + 1 {
                let sector_angle = (j as f32) * sector_step;
                let x = xy * sector_angle.cos();
                let y = xy * sector_angle.sin();

                vertices.push([x, y, z]);
                normals.push([x * length_inv, y * length_inv, z * length_inv]);
                uvs.push([(j as f32) / sectors, (i as f32) / stacks]);
            }
        }

        // indices
        //  k1--k1+1
        //  |  / |
        //  | /  |
        //  k2--k2+1
        for i in 0..sphere.stacks {
            let mut k1 = i * (sphere.sectors + 1);
            let mut k2 = k1 + sphere.sectors + 1;
            for _j in 0..sphere.sectors {
                if i != 0 {
                    indices.push(k1 as u32);
                    indices.push(k2 as u32);
                    indices.push((k1 + 1) as u32);
                }
                if i != sphere.stacks - 1 {
                    indices.push((k1 + 1) as u32);
                    indices.push(k2 as u32);
                    indices.push((k2 + 1) as u32);
                }
                k1 += 1;
                k2 += 1;
            }
        }

        let mut mesh = Mesh::new(PrimitiveTopology::TriangleList);
        mesh.set_indices(Some(Indices::U32(indices)));
        mesh.set_attribute(Mesh::ATTRIBUTE_POSITION, vertices);
        mesh.set_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
        mesh.set_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
        mesh
    }
}
