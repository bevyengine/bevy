//! This example demonstrates how to generate a mesh.

#![allow(clippy::bool_assert_comparison, clippy::useless_conversion)]

use glam::{Vec2, Vec3};

type Face = [u32; 3];

#[derive(Debug)]
struct Vertex {
    position: Vec3,
    normal: Vec3,
    tex_coord: Vec2,
}

struct Mesh {
    faces: Vec<Face>,
    vertices: Vec<Vertex>,
}

fn vertex(mesh: &Mesh, face: usize, vert: usize) -> &Vertex {
    let vs: &[u32; 3] = &mesh.faces[face];
    &mesh.vertices[vs[vert] as usize]
}

impl bevy_mikktspace::Geometry for Mesh {
    fn num_faces(&self) -> usize {
        self.faces.len()
    }

    fn num_vertices_of_face(&self, _face: usize) -> usize {
        3
    }

    fn position(&self, face: usize, vert: usize) -> [f32; 3] {
        vertex(self, face, vert).position.into()
    }

    fn normal(&self, face: usize, vert: usize) -> [f32; 3] {
        vertex(self, face, vert).normal.into()
    }

    fn tex_coord(&self, face: usize, vert: usize) -> [f32; 2] {
        vertex(self, face, vert).tex_coord.into()
    }

    fn set_tangent_encoded(&mut self, tangent: [f32; 4], face: usize, vert: usize) {
        println!(
            "{face}-{vert}: v: {v:?}, vn: {vn:?}, vt: {vt:?}, vx: {vx:?}",
            face = face,
            vert = vert,
            v = vertex(self, face, vert).position,
            vn = vertex(self, face, vert).normal,
            vt = vertex(self, face, vert).tex_coord,
            vx = tangent,
        );
    }
}

fn make_cube() -> Mesh {
    struct ControlPoint {
        uv: [f32; 2],
        dir: [f32; 3],
    }
    let mut faces = Vec::new();
    let mut ctl_pts = Vec::new();
    let mut vertices = Vec::new();

    // +x plane
    {
        let base = ctl_pts.len() as u32;
        faces.push([base, base + 1, base + 4]);
        faces.push([base + 1, base + 2, base + 4]);
        faces.push([base + 2, base + 3, base + 4]);
        faces.push([base + 3, base, base + 4]);
        ctl_pts.push(ControlPoint {
            uv: [0.0, 0.0],
            dir: [1.0, -1.0, 1.0],
        });
        ctl_pts.push(ControlPoint {
            uv: [0.0, 1.0],
            dir: [1.0, -1.0, -1.0],
        });
        ctl_pts.push(ControlPoint {
            uv: [1.0, 1.0],
            dir: [1.0, 1.0, -1.0],
        });
        ctl_pts.push(ControlPoint {
            uv: [1.0, 0.0],
            dir: [1.0, 1.0, 1.0],
        });
        ctl_pts.push(ControlPoint {
            uv: [0.5, 0.5],
            dir: [1.0, 0.0, 0.0],
        });
    }

    // -x plane
    {
        let base = ctl_pts.len() as u32;
        faces.push([base, base + 1, base + 4]);
        faces.push([base + 1, base + 2, base + 4]);
        faces.push([base + 2, base + 3, base + 4]);
        faces.push([base + 3, base, base + 4]);
        ctl_pts.push(ControlPoint {
            uv: [1.0, 0.0],
            dir: [-1.0, 1.0, 1.0],
        });
        ctl_pts.push(ControlPoint {
            uv: [1.0, 1.0],
            dir: [-1.0, 1.0, -1.0],
        });
        ctl_pts.push(ControlPoint {
            uv: [0.0, 1.0],
            dir: [-1.0, -1.0, -1.0],
        });
        ctl_pts.push(ControlPoint {
            uv: [0.0, 0.0],
            dir: [-1.0, -1.0, 1.0],
        });
        ctl_pts.push(ControlPoint {
            uv: [0.5, 0.5],
            dir: [-1.0, 0.0, 0.0],
        });
    }

    // +y plane
    {
        let base = ctl_pts.len() as u32;
        faces.push([base, base + 1, base + 4]);
        faces.push([base + 1, base + 2, base + 4]);
        faces.push([base + 2, base + 3, base + 4]);
        faces.push([base + 3, base, base + 4]);
        ctl_pts.push(ControlPoint {
            uv: [0.0, 0.0],
            dir: [1.0, 1.0, 1.0],
        });
        ctl_pts.push(ControlPoint {
            uv: [0.0, 1.0],
            dir: [1.0, 1.0, -1.0],
        });
        ctl_pts.push(ControlPoint {
            uv: [0.0, 1.0],
            dir: [-1.0, 1.0, -1.0],
        });
        ctl_pts.push(ControlPoint {
            uv: [0.0, 0.0],
            dir: [-1.0, 1.0, 1.0],
        });
        ctl_pts.push(ControlPoint {
            uv: [0.0, 0.5],
            dir: [0.0, 1.0, 0.0],
        });
    }

    // -y plane
    {
        let base = ctl_pts.len() as u32;
        faces.push([base, base + 1, base + 4]);
        faces.push([base + 1, base + 2, base + 4]);
        faces.push([base + 2, base + 3, base + 4]);
        faces.push([base + 3, base, base + 4]);
        ctl_pts.push(ControlPoint {
            uv: [0.0, 0.0],
            dir: [-1.0, -1.0, 1.0],
        });
        ctl_pts.push(ControlPoint {
            uv: [0.0, 1.0],
            dir: [-1.0, -1.0, -1.0],
        });
        ctl_pts.push(ControlPoint {
            uv: [0.0, 1.0],
            dir: [1.0, -1.0, -1.0],
        });
        ctl_pts.push(ControlPoint {
            uv: [0.0, 0.0],
            dir: [1.0, -1.0, 1.0],
        });
        ctl_pts.push(ControlPoint {
            uv: [0.0, 0.5],
            dir: [0.0, -1.0, 0.0],
        });
    }

    // +z plane
    {
        let base = ctl_pts.len() as u32;
        faces.push([base, base + 1, base + 4]);
        faces.push([base + 1, base + 2, base + 4]);
        faces.push([base + 2, base + 3, base + 4]);
        faces.push([base + 3, base, base + 4]);
        ctl_pts.push(ControlPoint {
            uv: [0.0, 0.0],
            dir: [-1.0, 1.0, 1.0],
        });
        ctl_pts.push(ControlPoint {
            uv: [0.0, 1.0],
            dir: [-1.0, -1.0, 1.0],
        });
        ctl_pts.push(ControlPoint {
            uv: [1.0, 1.0],
            dir: [1.0, -1.0, 1.0],
        });
        ctl_pts.push(ControlPoint {
            uv: [1.0, 0.0],
            dir: [1.0, 1.0, 1.0],
        });
        ctl_pts.push(ControlPoint {
            uv: [0.5, 0.5],
            dir: [0.0, 0.0, 1.0],
        });
    }

    // -z plane
    {
        let base = ctl_pts.len() as u32;
        faces.push([base, base + 1, base + 4]);
        faces.push([base + 1, base + 2, base + 4]);
        faces.push([base + 2, base + 3, base + 4]);
        faces.push([base + 3, base, base + 4]);
        ctl_pts.push(ControlPoint {
            uv: [1.0, 0.0],
            dir: [1.0, 1.0, -1.0],
        });
        ctl_pts.push(ControlPoint {
            uv: [1.0, 1.0],
            dir: [1.0, -1.0, -1.0],
        });
        ctl_pts.push(ControlPoint {
            uv: [0.0, 1.0],
            dir: [-1.0, -1.0, -1.0],
        });
        ctl_pts.push(ControlPoint {
            uv: [0.0, 0.0],
            dir: [-1.0, 1.0, -1.0],
        });
        ctl_pts.push(ControlPoint {
            uv: [0.5, 0.5],
            dir: [0.0, 0.0, -1.0],
        });
    }

    for pt in ctl_pts {
        let p: Vec3 = pt.dir.into();
        let n: Vec3 = p.normalize();
        let t: Vec2 = pt.uv.into();
        vertices.push(Vertex {
            position: (p / 2.0).into(),
            normal: n.into(),
            tex_coord: t.into(),
        });
    }

    Mesh { faces, vertices }
}

fn main() {
    let mut cube = make_cube();
    let ret = bevy_mikktspace::generate_tangents(&mut cube);
    assert_eq!(true, ret);
}
