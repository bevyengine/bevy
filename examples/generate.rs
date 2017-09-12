extern crate cgmath;
extern crate mikktspace;

use cgmath::prelude::*;

pub type Face = [u32; 3];
pub type Vec2 = [f32; 2];
pub type Vec3 = [f32; 3];

#[derive(Debug)]
struct Vertex {
    position: Vec3,
    normal: Vec3,
    tex_coord: Vec2,
}

fn make_cube() -> (Vec<Face>, Vec<Vertex>) {
    struct ControlPoint {
        uv: Vec2,
        dir: Vec3,
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
        ctl_pts.push(ControlPoint { uv: [0.0, 0.0], dir: [1.0, -1.0, 1.0] });
        ctl_pts.push(ControlPoint { uv: [0.0, 1.0], dir: [1.0, -1.0, -1.0] });
        ctl_pts.push(ControlPoint { uv: [1.0, 1.0], dir: [1.0, 1.0, -1.0] });
        ctl_pts.push(ControlPoint { uv: [1.0, 0.0], dir: [1.0, 1.0, 1.0] });
        ctl_pts.push(ControlPoint { uv: [0.5, 0.5], dir: [1.0, 0.0, 0.0] });
    }

    // -x plane
    {
        let base = ctl_pts.len() as u32;
        faces.push([base, base + 1, base + 4]);
        faces.push([base + 1, base + 2, base + 4]);
        faces.push([base + 2, base + 3, base + 4]);
        faces.push([base + 3, base, base + 4]);
        ctl_pts.push(ControlPoint { uv: [1.0, 0.0], dir: [-1.0, 1.0, 1.0] });
        ctl_pts.push(ControlPoint { uv: [1.0, 1.0], dir: [-1.0, 1.0, -1.0] });
        ctl_pts.push(ControlPoint { uv: [0.0, 1.0], dir: [-1.0, -1.0, -1.0] });
        ctl_pts.push(ControlPoint { uv: [0.0, 0.0], dir: [-1.0, -1.0, 1.0] });
        ctl_pts.push(ControlPoint { uv: [0.5, 0.5], dir: [-1.0, 0.0, 0.0] });
    }

    // +y plane
    {
        let base = ctl_pts.len() as u32;
        faces.push([base, base + 1, base + 4]);
        faces.push([base + 1, base + 2, base + 4]);
        faces.push([base + 2, base + 3, base + 4]);
        faces.push([base + 3, base, base + 4]);
        ctl_pts.push(ControlPoint { uv: [0.0, 0.0], dir: [1.0, 1.0, 1.0] });
        ctl_pts.push(ControlPoint { uv: [0.0, 1.0], dir: [1.0, 1.0, -1.0] });
        ctl_pts.push(ControlPoint { uv: [0.0, 1.0], dir: [-1.0, 1.0, -1.0] });
        ctl_pts.push(ControlPoint { uv: [0.0, 0.0], dir: [-1.0, 1.0, 1.0] });
        ctl_pts.push(ControlPoint { uv: [0.0, 0.5], dir: [0.0, 1.0, 0.0] });
    }

    // -y plane
    {
        let base = ctl_pts.len() as u32;
        faces.push([base, base + 1, base + 4]);
        faces.push([base + 1, base + 2, base + 4]);
        faces.push([base + 2, base + 3, base + 4]);
        faces.push([base + 3, base, base + 4]);
        ctl_pts.push(ControlPoint { uv: [0.0, 0.0], dir: [-1.0, -1.0, 1.0] });
        ctl_pts.push(ControlPoint { uv: [0.0, 1.0], dir: [-1.0, -1.0, -1.0] });
        ctl_pts.push(ControlPoint { uv: [0.0, 1.0], dir: [1.0, -1.0, -1.0] });
        ctl_pts.push(ControlPoint { uv: [0.0, 0.0], dir: [1.0, -1.0, 1.0] });
        ctl_pts.push(ControlPoint { uv: [0.0, 0.5], dir: [0.0, -1.0, 0.0] });
    }

    // +z plane
    {
        let base = ctl_pts.len() as u32;
        faces.push([base, base + 1, base + 4]);
        faces.push([base + 1, base + 2, base + 4]);
        faces.push([base + 2, base + 3, base + 4]);
        faces.push([base + 3, base, base + 4]);
        ctl_pts.push(ControlPoint { uv: [0.0, 0.0], dir: [-1.0, 1.0, 1.0] });
        ctl_pts.push(ControlPoint { uv: [0.0, 1.0], dir: [-1.0, -1.0, 1.0] });
        ctl_pts.push(ControlPoint { uv: [1.0, 1.0], dir: [1.0, -1.0, 1.0] });
        ctl_pts.push(ControlPoint { uv: [1.0, 0.0], dir: [1.0, 1.0, 1.0] });
        ctl_pts.push(ControlPoint { uv: [0.5, 0.5], dir: [0.0, 0.0, 1.0] });
    }

    // -z plane
    {
        let base = ctl_pts.len() as u32;
        faces.push([base, base + 1, base + 4]);
        faces.push([base + 1, base + 2, base + 4]);
        faces.push([base + 2, base + 3, base + 4]);
        faces.push([base + 3, base, base + 4]);
        ctl_pts.push(ControlPoint { uv: [1.0, 0.0], dir: [1.0, 1.0, -1.0] });
        ctl_pts.push(ControlPoint { uv: [1.0, 1.0], dir: [1.0, -1.0, -1.0] });
        ctl_pts.push(ControlPoint { uv: [0.0, 1.0], dir: [-1.0, -1.0, -1.0] });
        ctl_pts.push(ControlPoint { uv: [0.0, 0.0], dir: [-1.0, 1.0, -1.0] });
        ctl_pts.push(ControlPoint { uv: [0.5, 0.5], dir: [0.0, 0.0, -1.0] });
    }

    for pt in ctl_pts {
        let p: cgmath::Vector3<f32> = pt.dir.into();
        let n: cgmath::Vector3<f32> = p.normalize();
        let t: cgmath::Vector2<f32> = pt.uv.into();
        vertices.push(Vertex {
            position: (p / 2.0).into(),
            normal: n.into(),
            tex_coord: t.into(),
        });
    }

    (faces, vertices)
}

fn main() {
    let (faces, vertices) = make_cube();
    let vertex = |face, vert| {
        let vs: &[u32; 3] = &faces[face];
        &vertices[vs[vert] as usize]
    };
    let vertices_per_face = || 3;
    let face_count = || faces.len();
    let position = |face, vert| &vertex(face, vert).position;
    let normal = |face, vert| &vertex(face, vert).normal;
    let tex_coord = |face, vert| &vertex(face, vert).tex_coord;

    {
        let mut i = 0;
        let mut set_tangent = |face, vert, tangent| {
            println!(
                "{index}: v: {v:?}, vn: {vn:?}, vt: {vt:?}, vx: {vx:?}",
                index = i,
                v = position(face, vert),
                vn = normal(face, vert),
                vt = tex_coord(face, vert),
                vx = tangent,
            );
            i += 1;
        };
        let ret = mikktspace::generate_tangents(
            &vertices_per_face,
            &face_count,
            &position,
            &normal,
            &tex_coord,
            &mut set_tangent,
        );
        assert_eq!(true, ret);
    }
}
