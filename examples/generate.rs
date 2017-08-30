extern crate cgmath;
extern crate mikktspace;

use cgmath::prelude::*;

pub type Face = [u32; 3];
pub type Vec2 = [f32; 2];
pub type Vec3 = [f32; 3];
pub type Vec4 = [f32; 4];

#[derive(Debug)]
struct Vertex {
    position: Vec3,
    normal: Vec3,
    tex_coord: Vec2,
}

#[derive(Debug)]
struct NewVertex {
    position: Vec3,
    normal: Vec3,
    tex_coord: Vec2,
    tangent: Vec4,
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
    //println!("{:#?}", faces);
    //println!("{:#?}", vertices);

    let vertex = |face, vert| {
        let vs: &[u32; 3] = &faces[face % faces.len()];
        println!("reading {}, {}", face, vert);
        &vertices[vs[vert] as usize % vertices.len()]
    };
    let vertices_per_face = || 3;
    let face_count = || faces.len();
    let position = |face, vert| &vertex(face, vert).position;
    let normal = |face, vert| &vertex(face, vert).normal;
    let tex_coord = |face, vert| &vertex(face, vert).tex_coord;

    let mut new_vertices = Vec::new();

    {
        let mut set_tangent = |face, vert, tangent| {
            println!("setting {}, {}", face, vert);
            new_vertices.push(NewVertex {
                position: *position(face, vert),
                normal: *normal(face, vert),
                tex_coord: *tex_coord(face, vert),
                tangent: tangent,
            });
        };
        let ret = mikktspace::generate(
            &vertices_per_face,
            &face_count,
            &position,
            &normal,
            &tex_coord,
            &mut set_tangent,
        );
        assert_eq!(true, ret);
    }
    println!("{:#?}", new_vertices);
}
