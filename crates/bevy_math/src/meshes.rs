use crate::primitives::Triangle3d;
use crate::Vec3;

/// A mesh which is nothing more than a collection of triangles with no face adjacency information.
pub struct TriangleMesh {
    /// The faces of this mesh: a collection of triangles.
    pub faces: Vec<Triangle3d>,
}

impl TriangleMesh {
    /// Create a new [`TriangleMesh`] from a collection of triangular faces.
    pub fn new(faces: impl Into<Vec<Triangle3d>>) -> Self {
        Self {
            faces: faces.into(),
        }
    }
}

/// A triangular surface mesh with indexed faces, allowing face adjacencies to be recovered.
pub struct IndexedFaceMesh {
    vertices: Vec<Vec3>, // should these fields be public?
    faces: Vec<[usize; 3]>,
}

impl IndexedFaceMesh {
    /// Create a new [`IndexedFaceMesh`] from a collection of vertices and a collection of faces.
    /// Here, each element of `faces` is a set of indices into `vertices`.
    pub fn new(vertices: impl Into<Vec<Vec3>>, faces: impl Into<Vec<[usize; 3]>>) -> Self {
        Self {
            vertices: vertices.into(),
            faces: faces.into(),
        }
    }

    /// Build a face from the indices of its vertices.
    #[inline]
    fn build_face_triangle(vertices: &[Vec3], indices: [usize; 3]) -> Triangle3d {
        let vertices = indices.map(|v| vertices[v as usize]);
        Triangle3d { vertices }
    }

    /// Get the face at the provided `index` as a triangle. Returns `None` if the index is
    /// out of bounds.
    pub fn face_triangle(&self, index: usize) -> Option<Triangle3d> {
        self.faces
            .get(index)
            .map(|indices| Self::build_face_triangle(&self.vertices, *indices))
    }

    /// Get the collection of all faces of this [`IndexedFaceMesh`] as triangles.
    pub fn face_triangles(&self) -> Vec<Triangle3d> {
        self.faces
            .iter()
            .map(|indices| Self::build_face_triangle(&self.vertices, *indices))
            .collect()
    }
}

impl From<&IndexedFaceMesh> for TriangleMesh {
    fn from(mesh: &IndexedFaceMesh) -> Self {
        Self::new(mesh.face_triangles())
    }
}
