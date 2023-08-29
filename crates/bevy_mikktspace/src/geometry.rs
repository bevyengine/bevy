/// Interface for the mikktspace algorithm to query information about your geometry.
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

    /// Sets the generated tangent for a vertex.
    /// Leave this function unimplemented if you are implementing
    /// `set_tangent_encoded`.
    #[allow(clippy::too_many_arguments)]
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

    /// Sets the generated tangent for a vertex with its bi-tangent encoded as the 'W' (4th)
    /// component in the tangent. The 'W' component marks if the bi-tangent is flipped. This
    /// is called by the default implementation of `set_tangent`; therefore, this function will
    /// not be called by the crate unless `set_tangent` is unimplemented.
    fn set_tangent_encoded(&mut self, _tangent: [f32; 4], _face: usize, _vert: usize) {}
}
