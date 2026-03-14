//! Mesh collision approximation methods.
//!
//! Arbitrary meshes in real-time physics come with performance tradeoffs that
//! users generally need control over. The [`ColliderFromMeshApproximation`] trait
//! and its variants specify how a mesh should be converted into a collision shape.
//!
//! ## Usage
//!
//! This API is applied to mesh geometry alongside [`CollisionEnabled`](crate::collision::CollisionEnabled).
//! Simple primitives (Sphere, Cylinder, Cube, Cone, Capsule) can be used
//! directly without approximation.
//!
//! ## Approximation Methods
//!
//! Different approximations offer various tradeoffs between accuracy and performance:
//!
//! - **None** ([`ColliderFromMesh`]): Use mesh geometry directly. Highest accuracy
//!   but typically lowest performance.
//! - **Convex Decomposition** ([`ColliderFromMeshConvexDecomposition`]): Generate
//!   a set of convex hulls. Good balance for complex concave shapes.
//! - **Convex Hull** ([`ColliderFromMeshConvexHull`]): Single convex hull wrapping
//!   the mesh. Fast but may not accurately represent concave features.
//! - **Bounding Sphere** ([`ColliderFromMeshBoundingSphere`]): Fastest, least accurate.
//! - **Bounding Box** ([`ColliderFromMeshBoundingCube`]): Axis-aligned or optimally
//!   oriented box. Very fast.
//! - **Mesh Simplification** ([`ColliderFromMeshSimplification`]): Simplified
//!   triangle mesh with reduced polygon count.
//!
//! ## Subdivision Consideration
//!
//! The mesh's `subdivisionScheme` attribute can cause features like corners and
//! edges to be removed from the graphical representation. To ensure the physics
//! representation matches the visual appearance, this attribute should be accounted
//! for when generating physics colliders.
//!
//! ## Custom Collision Meshes
//!
//! For explicit control, create a custom collision mesh as a sibling to the
//! render mesh, set its `purpose` to "guide" (so it doesn't render), and apply
//! [`CollisionEnabled`](crate::collision::CollisionEnabled) with the `none`
//! approximation.
//!
//! ## Fallback Behavior
//!
//! If an implementation doesn't support a particular approximation, it should
//! fall back to the most similar supported option.

make_enum! {
    /// Determines the mesh's collision approximation method.
    ///
    /// This trait is implemented by marker components that specify how a mesh
    /// should be converted into a physics collider.
    ColliderFromMeshApproximation {
        /// Use the mesh geometry directly without any approximation.
        ///
        /// Provides the highest accuracy collision representation but typically
        /// has the lowest performance. Best for static environment geometry
        /// or when precise collision is required.
        ColliderFromMesh = "none",

        /// Perform convex mesh decomposition.
        ///
        /// The mesh is automatically decomposed into a set of convex hulls.
        /// This provides a good balance between accuracy and performance for
        /// complex concave shapes like furniture or vehicles.
        ColliderFromMeshConvexDecomposition = "convexDecomposition",

        /// Generate a single convex hull around the mesh.
        ///
        /// Fast and simple, but does not accurately represent concave features.
        /// The resulting shape will be the smallest convex volume containing
        /// all mesh vertices.
        ColliderFromMeshConvexHull = "convexHull",

        /// Compute a bounding sphere around the mesh.
        ///
        /// The fastest approximation but least accurate. Useful for rough
        /// collision detection or objects that are approximately spherical.
        ColliderFromMeshBoundingSphere = "boundingSphere",

        /// Compute an optimally fitting box collider.
        ///
        /// May be axis-aligned or oriented for best fit depending on
        /// implementation. Very fast and suitable for box-like objects.
        ColliderFromMeshBoundingCube = "boundingCube",

        /// Perform mesh simplification.
        ///
        /// Generates a simplified triangle mesh with reduced polygon count.
        /// Balances accuracy with performance better than exact mesh but
        /// less aggressively than convex approximations.
        ColliderFromMeshSimplification = "meshSimplification",
    }
    apiName = "approximation"
    displayName = "Approximation"
}
