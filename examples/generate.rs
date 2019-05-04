use nalgebra::{Point2, Point3, Vector3, Vector4};

pub type Face = [u32; 3];

#[derive(Debug)]
struct Vertex {
    position: Point3<f32>,
    normal: Vector3<f32>,
    tex_coord: Point2<f32>,
}

struct Mesh {
    faces: Vec<Face>,
    vertices: Vec<Vertex>,
}

fn vertex(mesh: &Mesh, face: usize, vert: usize) -> &Vertex {
    let vs: &[u32; 3] = &mesh.faces[face];
    &mesh.vertices[vs[vert] as usize]
}

impl mikktspace::Geometry for Mesh {
    fn get_num_faces(&self) -> usize {
        self.faces.len()
    }

    fn get_num_vertices_of_face(&self, _face: usize) -> usize {
        3
    }

    fn get_position(&self, face: usize, vert: usize) -> Point3<f32> {
        vertex(self, face, vert).position
    }

    fn get_normal(&self, face: usize, vert: usize) -> Vector3<f32> {
        vertex(self, face, vert).normal
    }

    fn get_tex_coord(&self, face: usize, vert: usize) -> Point2<f32> {
        vertex(self, face, vert).tex_coord
    }

    fn set_tangent_encoded(&mut self, tangent: Vector4<f32>, face: usize, vert: usize) {
        println!(
            "{face}-{vert}: v: {v:?}, vn: {vn:?}, vt: {vt:?}, vx: {vx:?}",
            face = face,
            vert = vert,
            v = vertex(self, face, vert).position.coords.data,
            vn = vertex(self, face, vert).normal.data,
            vt = vertex(self, face, vert).tex_coord.coords.data,
            vx = tangent.data,
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
        let p: Point3<f32> = pt.dir.into();
        let n: Vector3<f32> = p.coords.normalize();
        let t: Point2<f32> = pt.uv.into();
        vertices.push(Vertex {
            position: (p / 2.0).into(),
            normal: n.into(),
            tex_coord: t.into(),
        });
    }

    Mesh {
        faces,
        vertices,
    }
}

fn main() {
    let mut cube = make_cube();
    let ret = mikktspace::generate_tangents_default(&mut cube);
    assert_eq!(true, ret);
}
