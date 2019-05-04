mod generated;

use nalgebra::{Point2, Point3, Vector3, Vector4};

/// The interface by which mikktspace interacts with your geometry.
pub trait Geometry {
    /// Returns the number of faces.
    fn get_num_faces(&self) -> usize;

    /// Returns the number of vertices of a face.
    fn get_num_vertices_of_face(&self, face: usize) -> usize;

    /// Returns the position of a vertex.
    fn get_position(&self, face: usize, vert: usize) -> Point3<f32>;

    /// Returns the normal of a vertex.
    fn get_normal(&self, face: usize, vert: usize) -> Vector3<f32>;

    /// Returns the texture coordinate of a vertex.
    fn get_tex_coord(&self, face: usize, vert: usize) -> Point2<f32>;

    /// Sets a vertex' generated tangent.
    fn set_tangent(
        &mut self,
        tangent: Vector3<f32>,
        _bi_tangent: Vector3<f32>,
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
        self.set_tangent_encoded(tangent.insert_row(3, sign), face, vert);
    }

    /// Sets a vertex' generated tangent with the bi-tangent encoded as the W component in the
    /// tangent. The W component marks if the bi-tangent is flipped. This will only be called if
    /// `set_tangent` is not implemented.
    fn set_tangent_encoded(&mut self, _tangent: Vector4<f32>, _face: usize, _vert: usize) {}
}

/// Default (recommended) Angular Threshold is 180 degrees, which means threshold disabled.
pub fn generate_tangents_default<I: Geometry>(geometry: &mut I) -> bool {
    unsafe { generated::genTangSpace(geometry, 180.0) }
}

fn get_position<I: Geometry>(geometry: &mut I, index: usize) -> Vector3<f32> {
    let (face, vert) = index_to_face_vert(index);
    geometry.get_position(face, vert).coords
}

fn get_tex_coord<I: Geometry>(geometry: &mut I, index: usize) -> Vector3<f32> {
    let (face, vert) = index_to_face_vert(index);
    geometry.get_tex_coord(face, vert).coords.insert_row(2, 1.0)
}

fn get_normal<I: Geometry>(geometry: &mut I, index: usize) -> Vector3<f32> {
    let (face, vert) = index_to_face_vert(index);
    geometry.get_normal(face, vert)
}

fn index_to_face_vert(index: usize) -> (usize, usize) {
    (index >> 2, index & 0x3)
}

fn face_vert_to_index(face: usize, vert: usize) -> usize {
    face << 2 | vert & 0x3
}
