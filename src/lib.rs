mod generated;

use nalgebra::{Vector2, Vector3};

/// The interface by which mikktspace interacts with your geometry.
pub trait Geometry {
    /// Returns the number of faces.
    fn num_faces(&self) -> usize;

    /// Returns the number of vertices of a face.
    fn num_vertices_of_face(&self, face: usize) -> usize;

    /// Returns the position of a vertex.
    fn position(&self, face: usize, vert: usize) -> [f32; 3];

    /// Returns the normal of a vertex.
    fn normal(&self, face: usize, vert: usize) -> [f32; 3];

    /// Returns the texture coordinate of a vertex.
    fn tex_coord(&self, face: usize, vert: usize) -> [f32; 2];

    /// Sets a vertex' generated tangent.
    fn set_tangent(
        &mut self,
        tangent: [f32; 3],
        _bi_tangent: [f32; 3],
        _f_mag_s: f32,
        _f_mag_t: f32,
        bi_tangent_preserves_orientation: bool,
        face: usize,
        vert: usize,
    ) {
        let sign = if bi_tangent_preserves_orientation {
            1.0
        } else {
            -1.0
        };
        self.set_tangent_encoded([tangent[0], tangent[1], tangent[2], sign], face, vert);
    }

    /// Sets a vertex' generated tangent with the bi-tangent encoded as the W component in the
    /// tangent. The W component marks if the bi-tangent is flipped. This will only be called if
    /// `set_tangent` is not implemented.
    fn set_tangent_encoded(&mut self, _tangent: [f32; 4], _face: usize, _vert: usize) {}
}

/// Default (recommended) Angular Threshold is 180 degrees, which means threshold disabled.
pub fn generate_tangents_default<I: Geometry>(geometry: &mut I) -> bool {
    unsafe { generated::genTangSpace(geometry, 180.0) }
}

fn get_position<I: Geometry>(geometry: &mut I, index: usize) -> Vector3<f32> {
    let (face, vert) = index_to_face_vert(index);
    geometry.position(face, vert).into()
}

fn get_tex_coord<I: Geometry>(geometry: &mut I, index: usize) -> Vector3<f32> {
    let (face, vert) = index_to_face_vert(index);
    let tex_coord: Vector2<f32> = geometry.tex_coord(face, vert).into();
    tex_coord.insert_row(2, 1.0)
}

fn get_normal<I: Geometry>(geometry: &mut I, index: usize) -> Vector3<f32> {
    let (face, vert) = index_to_face_vert(index);
    geometry.normal(face, vert).into()
}

fn index_to_face_vert(index: usize) -> (usize, usize) {
    (index >> 2, index & 0x3)
}

fn face_vert_to_index(face: usize, vert: usize) -> usize {
    face << 2 | vert & 0x3
}
