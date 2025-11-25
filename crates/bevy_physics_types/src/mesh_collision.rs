//! Mesh collision attributes control how a mesh is converted into a collider.
//! Various approximation methods are available to optimize collision performance vs. accuracy tradeoffs.

usd_enum! {
    /// Determines the mesh's collision approximation method.
    ColliderFromMeshApproximation {
        /// "none" - The mesh geometry is used directly as a collider without any
        /// approximation.
        ColliderFromMesh = "none",
        /// "convexDecomposition" - A convex mesh decomposition is performed, resulting
        /// in a set of convex mesh colliders.
        ColliderFromMeshConvexDecomposition  = "convexDecomposition",
        /// "convexHull" - A convex hull of the mesh is generated and used as the
        /// collider.
        ColliderFromMeshConvexHull = "convexHull",
        /// "boundingSphere" - A bounding sphere is computed around the mesh and used
        /// as a collider.
        ColliderFromMeshBoundingSphere = "boundingSphere",
        /// "boundingCube" - An optimally fitting box collider is computed around the
        /// mesh.
        ColliderFromMeshBoundingCube = "boundingCube",
        /// "meshSimplification" - A mesh simplification step is performed, resulting
        /// in a simplified triangle mesh collider.
        ColliderFromMeshSimplification = "meshSimplification",
    }
    apiName = "approximation"
    displayName = "Approximation"
}
