use std::f32::consts::FRAC_PI_2;

use crate::{
    mesh::{primitives::dim3::triangle3d, Indices, Mesh, PerimeterSegment},
    render_asset::RenderAssetUsages,
};

use super::{Extrudable, MeshBuilder, Meshable};
use bevy_math::{
    primitives::{
        Annulus, Capsule2d, Circle, CircularSector, CircularSegment, Ellipse, Rectangle,
        RegularPolygon, Rhombus, Triangle2d, Triangle3d, WindingOrder,
    },
    FloatExt, Vec2,
};
use wgpu::PrimitiveTopology;

/// A builder used for creating a [`Mesh`] with a [`Circle`] shape.
#[derive(Clone, Copy, Debug)]
pub struct CircleMeshBuilder {
    /// The [`Circle`] shape.
    pub circle: Circle,
    /// The number of vertices used for the circle mesh.
    /// The default is `32`.
    #[doc(alias = "vertices")]
    pub resolution: u32,
}

impl Default for CircleMeshBuilder {
    fn default() -> Self {
        Self {
            circle: Circle::default(),
            resolution: 32,
        }
    }
}

impl CircleMeshBuilder {
    /// Creates a new [`CircleMeshBuilder`] from a given radius and vertex count.
    #[inline]
    pub const fn new(radius: f32, resolution: u32) -> Self {
        Self {
            circle: Circle { radius },
            resolution,
        }
    }

    /// Sets the number of vertices used for the circle mesh.
    #[inline]
    #[doc(alias = "vertices")]
    pub const fn resolution(mut self, resolution: u32) -> Self {
        self.resolution = resolution;
        self
    }
}

impl MeshBuilder for CircleMeshBuilder {
    fn build(&self) -> Mesh {
        Ellipse::new(self.circle.radius, self.circle.radius)
            .mesh()
            .resolution(self.resolution)
            .build()
    }
}

impl Extrudable for CircleMeshBuilder {
    fn perimeter(&self) -> Vec<PerimeterSegment> {
        vec![PerimeterSegment::Smooth {
            first_normal: Vec2::Y,
            last_normal: Vec2::Y,
            indices: (0..self.resolution).chain([0]).collect(),
        }]
    }
}

impl Meshable for Circle {
    type Output = CircleMeshBuilder;

    fn mesh(&self) -> Self::Output {
        CircleMeshBuilder {
            circle: *self,
            ..Default::default()
        }
    }
}

impl From<Circle> for Mesh {
    fn from(circle: Circle) -> Self {
        circle.mesh().build()
    }
}

/// Specifies how to generate UV-mappings for the [`CircularSector`] and [`CircularSegment`] shapes.
///
/// Currently the only variant is `Mask`, which is good for showing a portion of a texture that includes
/// the entire circle, particularly the same texture will be displayed with different fractions of a
/// complete circle.
///
/// It's expected that more will be added in the future, such as a variant that causes the texture to be
/// scaled to fit the bounding box of the shape, which would be good for packed textures only including the
/// portion of the circle that is needed to display.
#[derive(Copy, Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum CircularMeshUvMode {
    /// Treats the shape as a mask over a circle of equal size and radius,
    /// with the center of the circle at the center of the texture.
    Mask {
        /// Angle by which to rotate the shape when generating the UV map.
        angle: f32,
    },
}

impl Default for CircularMeshUvMode {
    fn default() -> Self {
        CircularMeshUvMode::Mask { angle: 0.0 }
    }
}

/// A builder used for creating a [`Mesh`] with a [`CircularSector`] shape.
///
/// The resulting mesh will have a UV-map such that the center of the circle is
/// at the center of the texture.
#[derive(Clone, Debug)]
pub struct CircularSectorMeshBuilder {
    /// The sector shape.
    pub sector: CircularSector,
    /// The number of vertices used for the arc portion of the sector mesh.
    /// The default is `32`.
    #[doc(alias = "vertices")]
    pub resolution: u32,
    /// The UV mapping mode
    pub uv_mode: CircularMeshUvMode,
}

impl Default for CircularSectorMeshBuilder {
    fn default() -> Self {
        Self {
            sector: CircularSector::default(),
            resolution: 32,
            uv_mode: CircularMeshUvMode::default(),
        }
    }
}

impl CircularSectorMeshBuilder {
    /// Creates a new [`CircularSectorMeshBuilder`] from a given sector
    #[inline]
    pub fn new(sector: CircularSector) -> Self {
        Self {
            sector,
            ..Self::default()
        }
    }

    /// Sets the number of vertices used for the sector mesh.
    #[inline]
    #[doc(alias = "vertices")]
    pub const fn resolution(mut self, resolution: u32) -> Self {
        self.resolution = resolution;
        self
    }

    /// Sets the uv mode used for the sector mesh
    #[inline]
    pub const fn uv_mode(mut self, uv_mode: CircularMeshUvMode) -> Self {
        self.uv_mode = uv_mode;
        self
    }
}

impl MeshBuilder for CircularSectorMeshBuilder {
    fn build(&self) -> Mesh {
        let resolution = self.resolution as usize;
        let mut indices = Vec::with_capacity((resolution - 1) * 3);
        let mut positions = Vec::with_capacity(resolution + 1);
        let normals = vec![[0.0, 0.0, 1.0]; resolution + 1];
        let mut uvs = Vec::with_capacity(resolution + 1);

        let CircularMeshUvMode::Mask { angle: uv_angle } = self.uv_mode;

        // Push the center of the circle.
        positions.push([0.0; 3]);
        uvs.push([0.5; 2]);

        let first_angle = FRAC_PI_2 - self.sector.half_angle();
        let last_angle = FRAC_PI_2 + self.sector.half_angle();
        let last_i = (self.resolution - 1) as f32;
        for i in 0..self.resolution {
            let angle = f32::lerp(first_angle, last_angle, i as f32 / last_i);

            // Compute the vertex
            let vertex = self.sector.radius() * Vec2::from_angle(angle);
            // Compute the UV coordinate by taking the modified angle's unit vector, negating the Y axis, and rescaling and centering it at (0.5, 0.5).
            // We accomplish the Y axis flip by negating the angle.
            let uv =
                Vec2::from_angle(-(angle + uv_angle)).mul_add(Vec2::splat(0.5), Vec2::splat(0.5));

            positions.push([vertex.x, vertex.y, 0.0]);
            uvs.push([uv.x, uv.y]);
        }

        for i in 1..self.resolution {
            // Index 0 is the center.
            indices.extend_from_slice(&[0, i, i + 1]);
        }

        Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::default(),
        )
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
        .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs)
        .with_inserted_indices(Indices::U32(indices))
    }
}

impl Extrudable for CircularSectorMeshBuilder {
    fn perimeter(&self) -> Vec<PerimeterSegment> {
        let (sin, cos) = self.sector.arc.half_angle.sin_cos();
        let first_normal = Vec2::new(sin, cos);
        let last_normal = Vec2::new(-sin, cos);
        vec![
            PerimeterSegment::Flat {
                indices: vec![self.resolution, 0, 1],
            },
            PerimeterSegment::Smooth {
                first_normal,
                last_normal,
                indices: (1..=self.resolution).collect(),
            },
        ]
    }
}

impl Meshable for CircularSector {
    type Output = CircularSectorMeshBuilder;

    fn mesh(&self) -> Self::Output {
        CircularSectorMeshBuilder {
            sector: *self,
            ..Default::default()
        }
    }
}

impl From<CircularSector> for Mesh {
    /// Converts this sector into a [`Mesh`] using a default [`CircularSectorMeshBuilder`].
    ///
    /// See the documentation of [`CircularSectorMeshBuilder`] for more details.
    fn from(sector: CircularSector) -> Self {
        sector.mesh().build()
    }
}

/// A builder used for creating a [`Mesh`] with a [`CircularSegment`] shape.
///
/// The resulting mesh will have a UV-map such that the center of the circle is
/// at the center of the texture.
#[derive(Clone, Copy, Debug)]
pub struct CircularSegmentMeshBuilder {
    /// The segment shape.
    pub segment: CircularSegment,
    /// The number of vertices used for the arc portion of the segment mesh.
    /// The default is `32`.
    #[doc(alias = "vertices")]
    pub resolution: u32,
    /// The UV mapping mode
    pub uv_mode: CircularMeshUvMode,
}

impl Default for CircularSegmentMeshBuilder {
    fn default() -> Self {
        Self {
            segment: CircularSegment::default(),
            resolution: 32,
            uv_mode: CircularMeshUvMode::default(),
        }
    }
}

impl CircularSegmentMeshBuilder {
    /// Creates a new [`CircularSegmentMeshBuilder`] from a given segment
    #[inline]
    pub fn new(segment: CircularSegment) -> Self {
        Self {
            segment,
            ..Self::default()
        }
    }

    /// Sets the number of vertices used for the segment mesh.
    #[inline]
    #[doc(alias = "vertices")]
    pub const fn resolution(mut self, resolution: u32) -> Self {
        self.resolution = resolution;
        self
    }

    /// Sets the uv mode used for the segment mesh
    #[inline]
    pub const fn uv_mode(mut self, uv_mode: CircularMeshUvMode) -> Self {
        self.uv_mode = uv_mode;
        self
    }
}

impl MeshBuilder for CircularSegmentMeshBuilder {
    fn build(&self) -> Mesh {
        let resolution = self.resolution as usize;
        let mut indices = Vec::with_capacity((resolution - 1) * 3);
        let mut positions = Vec::with_capacity(resolution + 1);
        let normals = vec![[0.0, 0.0, 1.0]; resolution + 1];
        let mut uvs = Vec::with_capacity(resolution + 1);

        let CircularMeshUvMode::Mask { angle: uv_angle } = self.uv_mode;

        // Push the center of the chord.
        let midpoint_vertex = self.segment.chord_midpoint();
        positions.push([midpoint_vertex.x, midpoint_vertex.y, 0.0]);
        // Compute the UV coordinate of the midpoint vertex.
        // This is similar to the computation inside the loop for the arc vertices,
        // but the vertex angle is PI/2, and we must scale by the ratio of the apothem to the radius
        // to correctly position the vertex.
        let midpoint_uv = Vec2::from_angle(-uv_angle - FRAC_PI_2).mul_add(
            Vec2::splat(0.5 * (self.segment.apothem() / self.segment.radius())),
            Vec2::splat(0.5),
        );
        uvs.push([midpoint_uv.x, midpoint_uv.y]);

        let first_angle = FRAC_PI_2 - self.segment.half_angle();
        let last_angle = FRAC_PI_2 + self.segment.half_angle();
        let last_i = (self.resolution - 1) as f32;
        for i in 0..self.resolution {
            let angle = f32::lerp(first_angle, last_angle, i as f32 / last_i);

            // Compute the vertex
            let vertex = self.segment.radius() * Vec2::from_angle(angle);
            // Compute the UV coordinate by taking the modified angle's unit vector, negating the Y axis, and rescaling and centering it at (0.5, 0.5).
            // We accomplish the Y axis flip by negating the angle.
            let uv =
                Vec2::from_angle(-(angle + uv_angle)).mul_add(Vec2::splat(0.5), Vec2::splat(0.5));

            positions.push([vertex.x, vertex.y, 0.0]);
            uvs.push([uv.x, uv.y]);
        }

        for i in 1..self.resolution {
            // Index 0 is the midpoint of the chord.
            indices.extend_from_slice(&[0, i, i + 1]);
        }

        Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::default(),
        )
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
        .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs)
        .with_inserted_indices(Indices::U32(indices))
    }
}

impl Extrudable for CircularSegmentMeshBuilder {
    fn perimeter(&self) -> Vec<PerimeterSegment> {
        let (sin, cos) = self.segment.arc.half_angle.sin_cos();
        let first_normal = Vec2::new(sin, cos);
        let last_normal = Vec2::new(-sin, cos);
        vec![
            PerimeterSegment::Flat {
                indices: vec![self.resolution, 0, 1],
            },
            PerimeterSegment::Smooth {
                first_normal,
                last_normal,
                indices: (1..=self.resolution).collect(),
            },
        ]
    }
}

impl Meshable for CircularSegment {
    type Output = CircularSegmentMeshBuilder;

    fn mesh(&self) -> Self::Output {
        CircularSegmentMeshBuilder {
            segment: *self,
            ..Default::default()
        }
    }
}

impl From<CircularSegment> for Mesh {
    /// Converts this sector into a [`Mesh`] using a default [`CircularSegmentMeshBuilder`].
    ///
    /// See the documentation of [`CircularSegmentMeshBuilder`] for more details.
    fn from(segment: CircularSegment) -> Self {
        segment.mesh().build()
    }
}

/// A builder used for creating a [`Mesh`] with a [`RegularPolygon`] shape.
pub struct RegularPolygonMeshBuilder {
    circumradius: f32,
    sides: u32,
}
impl Meshable for RegularPolygon {
    type Output = RegularPolygonMeshBuilder;

    fn mesh(&self) -> Self::Output {
        Self::Output {
            circumradius: self.circumcircle.radius,
            sides: self.sides,
        }
    }
}

impl MeshBuilder for RegularPolygonMeshBuilder {
    fn build(&self) -> Mesh {
        // The ellipse mesh is just a regular polygon with two radii
        Ellipse::new(self.circumradius, self.circumradius)
            .mesh()
            .resolution(self.sides)
            .build()
    }
}

impl Extrudable for RegularPolygonMeshBuilder {
    fn perimeter(&self) -> Vec<PerimeterSegment> {
        vec![PerimeterSegment::Flat {
            indices: (0..self.sides).chain([0]).collect(),
        }]
    }
}

impl From<RegularPolygon> for Mesh {
    fn from(polygon: RegularPolygon) -> Self {
        polygon.mesh().build()
    }
}

/// A builder used for creating a [`Mesh`] with an [`Ellipse`] shape.
#[derive(Clone, Copy, Debug)]
pub struct EllipseMeshBuilder {
    /// The [`Ellipse`] shape.
    pub ellipse: Ellipse,
    /// The number of vertices used for the ellipse mesh.
    /// The default is `32`.
    #[doc(alias = "vertices")]
    pub resolution: u32,
}

impl Default for EllipseMeshBuilder {
    fn default() -> Self {
        Self {
            ellipse: Ellipse::default(),
            resolution: 32,
        }
    }
}

impl EllipseMeshBuilder {
    /// Creates a new [`EllipseMeshBuilder`] from a given half width and half height and a vertex count.
    #[inline]
    pub const fn new(half_width: f32, half_height: f32, resolution: u32) -> Self {
        Self {
            ellipse: Ellipse::new(half_width, half_height),
            resolution,
        }
    }

    /// Sets the number of vertices used for the ellipse mesh.
    #[inline]
    #[doc(alias = "vertices")]
    pub const fn resolution(mut self, resolution: u32) -> Self {
        self.resolution = resolution;
        self
    }
}

impl MeshBuilder for EllipseMeshBuilder {
    fn build(&self) -> Mesh {
        let resolution = self.resolution as usize;
        let mut indices = Vec::with_capacity((resolution - 2) * 3);
        let mut positions = Vec::with_capacity(resolution);
        let normals = vec![[0.0, 0.0, 1.0]; resolution];
        let mut uvs = Vec::with_capacity(resolution);

        // Add pi/2 so that there is a vertex at the top (sin is 1.0 and cos is 0.0)
        let start_angle = std::f32::consts::FRAC_PI_2;
        let step = std::f32::consts::TAU / self.resolution as f32;

        for i in 0..self.resolution {
            // Compute vertex position at angle theta
            let theta = start_angle + i as f32 * step;
            let (sin, cos) = theta.sin_cos();
            let x = cos * self.ellipse.half_size.x;
            let y = sin * self.ellipse.half_size.y;

            positions.push([x, y, 0.0]);
            uvs.push([0.5 * (cos + 1.0), 1.0 - 0.5 * (sin + 1.0)]);
        }

        for i in 1..(self.resolution - 1) {
            indices.extend_from_slice(&[0, i, i + 1]);
        }

        Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::default(),
        )
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
        .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs)
        .with_inserted_indices(Indices::U32(indices))
    }
}

impl Extrudable for EllipseMeshBuilder {
    fn perimeter(&self) -> Vec<PerimeterSegment> {
        vec![PerimeterSegment::Smooth {
            first_normal: Vec2::Y,
            last_normal: Vec2::Y,
            indices: (0..self.resolution).chain([0]).collect(),
        }]
    }
}

impl Meshable for Ellipse {
    type Output = EllipseMeshBuilder;

    fn mesh(&self) -> Self::Output {
        EllipseMeshBuilder {
            ellipse: *self,
            ..Default::default()
        }
    }
}

impl From<Ellipse> for Mesh {
    fn from(ellipse: Ellipse) -> Self {
        ellipse.mesh().build()
    }
}

/// A builder for creating a [`Mesh`] with an [`Annulus`] shape.
pub struct AnnulusMeshBuilder {
    /// The [`Annulus`] shape.
    pub annulus: Annulus,

    /// The number of vertices used in constructing each concentric circle of the annulus mesh.
    /// The default is `32`.
    pub resolution: u32,
}

impl Default for AnnulusMeshBuilder {
    fn default() -> Self {
        Self {
            annulus: Annulus::default(),
            resolution: 32,
        }
    }
}

impl AnnulusMeshBuilder {
    /// Create an [`AnnulusMeshBuilder`] with the given inner radius, outer radius, and angular vertex count.
    #[inline]
    pub fn new(inner_radius: f32, outer_radius: f32, resolution: u32) -> Self {
        Self {
            annulus: Annulus::new(inner_radius, outer_radius),
            resolution,
        }
    }

    /// Sets the number of vertices used in constructing the concentric circles of the annulus mesh.
    #[inline]
    pub fn resolution(mut self, resolution: u32) -> Self {
        self.resolution = resolution;
        self
    }
}

impl MeshBuilder for AnnulusMeshBuilder {
    fn build(&self) -> Mesh {
        let inner_radius = self.annulus.inner_circle.radius;
        let outer_radius = self.annulus.outer_circle.radius;

        let num_vertices = (self.resolution as usize + 1) * 2;
        let mut indices = Vec::with_capacity(self.resolution as usize * 6);
        let mut positions = Vec::with_capacity(num_vertices);
        let mut uvs = Vec::with_capacity(num_vertices);
        let normals = vec![[0.0, 0.0, 1.0]; num_vertices];

        // We have one more set of vertices than might be naïvely expected;
        // the vertices at `start_angle` are duplicated for the purposes of UV
        // mapping. Here, each iteration places a pair of vertices at a fixed
        // angle from the center of the annulus.
        let start_angle = std::f32::consts::FRAC_PI_2;
        let step = std::f32::consts::TAU / self.resolution as f32;
        for i in 0..=self.resolution {
            let theta = start_angle + i as f32 * step;
            let (sin, cos) = theta.sin_cos();
            let inner_pos = [cos * inner_radius, sin * inner_radius, 0.];
            let outer_pos = [cos * outer_radius, sin * outer_radius, 0.];
            positions.push(inner_pos);
            positions.push(outer_pos);

            // The first UV direction is radial and the second is angular;
            // i.e., a single UV rectangle is stretched around the annulus, with
            // its top and bottom meeting as the circle closes. Lines of constant
            // U map to circles, and lines of constant V map to radial line segments.
            let inner_uv = [0., i as f32 / self.resolution as f32];
            let outer_uv = [1., i as f32 / self.resolution as f32];
            uvs.push(inner_uv);
            uvs.push(outer_uv);
        }

        // Adjacent pairs of vertices form two triangles with each other; here,
        // we are just making sure that they both have the right orientation,
        // which is the CCW order of
        // `inner_vertex` -> `outer_vertex` -> `next_outer` -> `next_inner`
        for i in 0..self.resolution {
            let inner_vertex = 2 * i;
            let outer_vertex = 2 * i + 1;
            let next_inner = inner_vertex + 2;
            let next_outer = outer_vertex + 2;
            indices.extend_from_slice(&[inner_vertex, outer_vertex, next_outer]);
            indices.extend_from_slice(&[next_outer, next_inner, inner_vertex]);
        }

        Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::default(),
        )
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
        .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs)
        .with_inserted_indices(Indices::U32(indices))
    }
}

impl Extrudable for AnnulusMeshBuilder {
    fn perimeter(&self) -> Vec<PerimeterSegment> {
        let vert_count = 2 * self.resolution;
        vec![
            PerimeterSegment::Smooth {
                first_normal: Vec2::NEG_Y,
                last_normal: Vec2::NEG_Y,
                indices: (0..vert_count).step_by(2).chain([0]).rev().collect(), // Inner hole
            },
            PerimeterSegment::Smooth {
                first_normal: Vec2::Y,
                last_normal: Vec2::Y,
                indices: (1..vert_count).step_by(2).chain([1]).collect(), // Outer perimeter
            },
        ]
    }
}

impl Meshable for Annulus {
    type Output = AnnulusMeshBuilder;

    fn mesh(&self) -> Self::Output {
        AnnulusMeshBuilder {
            annulus: *self,
            ..Default::default()
        }
    }
}

impl From<Annulus> for Mesh {
    fn from(annulus: Annulus) -> Self {
        annulus.mesh().build()
    }
}

pub struct RhombusMeshBuilder {
    half_diagonals: Vec2,
}

impl MeshBuilder for RhombusMeshBuilder {
    fn build(&self) -> Mesh {
        let [hhd, vhd] = [self.half_diagonals.x, self.half_diagonals.y];
        let positions = vec![
            [hhd, 0.0, 0.0],
            [-hhd, 0.0, 0.0],
            [0.0, vhd, 0.0],
            [0.0, -vhd, 0.0],
        ];
        let normals = vec![[0.0, 0.0, 1.0]; 4];
        let uvs = vec![[1.0, 0.5], [0.0, 0.5], [0.5, 0.0], [0.5, 1.0]];
        let indices = Indices::U32(vec![1, 0, 2, 1, 3, 0]);

        Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::default(),
        )
        .with_inserted_indices(indices)
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
        .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs)
    }
}

impl Extrudable for RhombusMeshBuilder {
    fn perimeter(&self) -> Vec<PerimeterSegment> {
        vec![PerimeterSegment::Flat {
            indices: vec![0, 2, 1, 3, 0],
        }]
    }
}

impl Meshable for Rhombus {
    type Output = RhombusMeshBuilder;

    fn mesh(&self) -> Self::Output {
        Self::Output {
            half_diagonals: self.half_diagonals,
        }
    }
}

impl From<Rhombus> for Mesh {
    fn from(rhombus: Rhombus) -> Self {
        rhombus.mesh().build()
    }
}

/// A builder used for creating a [`Mesh`] with a [`Triangle2d`] shape.
pub struct Triangle2dMeshBuilder {
    triangle: Triangle2d,
}
impl Meshable for Triangle2d {
    type Output = Triangle2dMeshBuilder;

    fn mesh(&self) -> Self::Output {
        Self::Output { triangle: *self }
    }
}
impl MeshBuilder for Triangle2dMeshBuilder {
    fn build(&self) -> Mesh {
        let vertices_3d = self.triangle.vertices.map(|v| v.extend(0.));

        let positions: Vec<_> = vertices_3d.into();
        let normals = vec![[0.0, 0.0, 1.0]; 3];

        let uvs: Vec<_> = triangle3d::uv_coords(&Triangle3d::new(
            vertices_3d[0],
            vertices_3d[1],
            vertices_3d[2],
        ))
        .into();

        let is_ccw = self.triangle.winding_order() == WindingOrder::CounterClockwise;
        let indices = if is_ccw {
            Indices::U32(vec![0, 1, 2])
        } else {
            Indices::U32(vec![2, 1, 0])
        };

        Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::default(),
        )
        .with_inserted_indices(indices)
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
        .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs)
    }
}

impl Extrudable for Triangle2dMeshBuilder {
    fn perimeter(&self) -> Vec<PerimeterSegment> {
        let is_ccw = self.triangle.winding_order() == WindingOrder::CounterClockwise;
        if is_ccw {
            vec![PerimeterSegment::Flat {
                indices: vec![0, 1, 2, 0],
            }]
        } else {
            vec![PerimeterSegment::Flat {
                indices: vec![2, 1, 0, 2],
            }]
        }
    }
}

impl From<Triangle2d> for Mesh {
    fn from(triangle: Triangle2d) -> Self {
        triangle.mesh().build()
    }
}

/// A builder used for creating a [`Mesh`] with a [`Rectangle`] shape.
pub struct RectangleMeshBuilder {
    half_size: Vec2,
}

impl MeshBuilder for RectangleMeshBuilder {
    fn build(&self) -> Mesh {
        let [hw, hh] = [self.half_size.x, self.half_size.y];
        let positions = vec![
            [hw, hh, 0.0],
            [-hw, hh, 0.0],
            [-hw, -hh, 0.0],
            [hw, -hh, 0.0],
        ];
        let normals = vec![[0.0, 0.0, 1.0]; 4];
        let uvs = vec![[1.0, 0.0], [0.0, 0.0], [0.0, 1.0], [1.0, 1.0]];
        let indices = Indices::U32(vec![0, 1, 2, 0, 2, 3]);

        Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::default(),
        )
        .with_inserted_indices(indices)
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
        .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs)
    }
}

impl Extrudable for RectangleMeshBuilder {
    fn perimeter(&self) -> Vec<PerimeterSegment> {
        vec![PerimeterSegment::Flat {
            indices: vec![0, 1, 2, 3, 0],
        }]
    }
}

impl Meshable for Rectangle {
    type Output = RectangleMeshBuilder;

    fn mesh(&self) -> Self::Output {
        RectangleMeshBuilder {
            half_size: self.half_size,
        }
    }
}

impl From<Rectangle> for Mesh {
    fn from(rectangle: Rectangle) -> Self {
        rectangle.mesh().build()
    }
}

/// A builder used for creating a [`Mesh`] with a [`Capsule2d`] shape.
#[derive(Clone, Copy, Debug)]
pub struct Capsule2dMeshBuilder {
    /// The [`Capsule2d`] shape.
    pub capsule: Capsule2d,
    /// The number of vertices used for one hemicircle.
    /// The total number of vertices for the capsule mesh will be two times the resolution.
    ///
    /// The default is `16`.
    pub resolution: u32,
}

impl Default for Capsule2dMeshBuilder {
    fn default() -> Self {
        Self {
            capsule: Capsule2d::default(),
            resolution: 16,
        }
    }
}

impl Capsule2dMeshBuilder {
    /// Creates a new [`Capsule2dMeshBuilder`] from a given radius, length, and the number of vertices
    /// used for one hemicircle. The total number of vertices for the capsule mesh will be two times the resolution.
    #[inline]
    pub fn new(radius: f32, length: f32, resolution: u32) -> Self {
        Self {
            capsule: Capsule2d::new(radius, length),
            resolution,
        }
    }

    /// Sets the number of vertices used for one hemicircle.
    /// The total number of vertices for the capsule mesh will be two times the resolution.
    #[inline]
    pub const fn resolution(mut self, resolution: u32) -> Self {
        self.resolution = resolution;
        self
    }
}

impl MeshBuilder for Capsule2dMeshBuilder {
    fn build(&self) -> Mesh {
        // The resolution is the number of vertices for one semicircle
        let resolution = self.resolution;
        let vertex_count = 2 * resolution;

        // Six extra indices for the two triangles between the hemicircles
        let mut indices = Vec::with_capacity((resolution as usize - 2) * 2 * 3 + 6);
        let mut positions = Vec::with_capacity(vertex_count as usize);
        let normals = vec![[0.0, 0.0, 1.0]; vertex_count as usize];
        let mut uvs = Vec::with_capacity(vertex_count as usize);

        let radius = self.capsule.radius;
        let step = std::f32::consts::TAU / vertex_count as f32;

        // If the vertex count is even, offset starting angle of top semicircle by half a step
        // to position the vertices evenly.
        let start_angle = if vertex_count % 2 == 0 {
            step / 2.0
        } else {
            0.0
        };

        // How much the hemicircle radius is of the total half-height of the capsule.
        // This is used to prevent the UVs from stretching between the hemicircles.
        let radius_frac = self.capsule.radius / (self.capsule.half_length + self.capsule.radius);

        // Create top semicircle
        for i in 0..resolution {
            // Compute vertex position at angle theta
            let theta = start_angle + i as f32 * step;
            let (sin, cos) = theta.sin_cos();
            let (x, y) = (cos * radius, sin * radius + self.capsule.half_length);

            positions.push([x, y, 0.0]);
            uvs.push([0.5 * (cos + 1.0), radius_frac * (1.0 - 0.5 * (sin + 1.0))]);
        }

        // Add top semicircle indices
        for i in 1..resolution - 1 {
            indices.extend_from_slice(&[0, i, i + 1]);
        }

        // Add indices for top left triangle of the part between the hemicircles
        indices.extend_from_slice(&[0, resolution - 1, resolution]);

        // Create bottom semicircle
        for i in resolution..vertex_count {
            // Compute vertex position at angle theta
            let theta = start_angle + i as f32 * step;
            let (sin, cos) = theta.sin_cos();
            let (x, y) = (cos * radius, sin * radius - self.capsule.half_length);

            positions.push([x, y, 0.0]);
            uvs.push([0.5 * (cos + 1.0), 1.0 - radius_frac * 0.5 * (sin + 1.0)]);
        }

        // Add bottom semicircle indices
        for i in 1..resolution - 1 {
            indices.extend_from_slice(&[resolution, resolution + i, resolution + i + 1]);
        }

        // Add indices for bottom right triangle of the part between the hemicircles
        indices.extend_from_slice(&[resolution, vertex_count - 1, 0]);

        Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::default(),
        )
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
        .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs)
        .with_inserted_indices(Indices::U32(indices))
    }
}

impl Extrudable for Capsule2dMeshBuilder {
    fn perimeter(&self) -> Vec<PerimeterSegment> {
        let resolution = self.resolution;
        let top_semi_indices = (0..resolution).collect();
        let bottom_semi_indices = (resolution..(2 * resolution)).collect();
        vec![
            PerimeterSegment::Smooth {
                first_normal: Vec2::X,
                last_normal: Vec2::NEG_X,
                indices: top_semi_indices,
            }, // Top semi-circle
            PerimeterSegment::Flat {
                indices: vec![resolution - 1, resolution],
            }, // Left edge
            PerimeterSegment::Smooth {
                first_normal: Vec2::NEG_X,
                last_normal: Vec2::X,
                indices: bottom_semi_indices,
            }, // Bottom semi-circle
            PerimeterSegment::Flat {
                indices: vec![2 * resolution - 1, 0],
            }, // Right edge
        ]
    }
}

impl Meshable for Capsule2d {
    type Output = Capsule2dMeshBuilder;

    fn mesh(&self) -> Self::Output {
        Capsule2dMeshBuilder {
            capsule: *self,
            ..Default::default()
        }
    }
}

impl From<Capsule2d> for Mesh {
    fn from(capsule: Capsule2d) -> Self {
        capsule.mesh().build()
    }
}

#[cfg(test)]
mod tests {
    use bevy_math::primitives::RegularPolygon;

    use crate::mesh::{Mesh, VertexAttributeValues};

    /// Sin/cos and multiplication computations result in numbers like 0.4999999.
    /// Round these to numbers we expect like 0.5.
    fn fix_floats<const N: usize>(points: &mut [[f32; N]]) {
        for point in points.iter_mut() {
            for coord in point.iter_mut() {
                let round = (*coord * 2.).round() / 2.;
                if (*coord - round).abs() < 0.00001 {
                    *coord = round;
                }
            }
        }
    }

    #[test]
    fn test_regular_polygon() {
        let mut mesh = Mesh::from(RegularPolygon::new(7.0, 4));

        let Some(VertexAttributeValues::Float32x3(mut positions)) =
            mesh.remove_attribute(Mesh::ATTRIBUTE_POSITION)
        else {
            panic!("Expected positions f32x3");
        };
        let Some(VertexAttributeValues::Float32x2(mut uvs)) =
            mesh.remove_attribute(Mesh::ATTRIBUTE_UV_0)
        else {
            panic!("Expected uvs f32x2");
        };
        let Some(VertexAttributeValues::Float32x3(normals)) =
            mesh.remove_attribute(Mesh::ATTRIBUTE_NORMAL)
        else {
            panic!("Expected normals f32x3");
        };

        fix_floats(&mut positions);
        fix_floats(&mut uvs);

        assert_eq!(
            [
                [0.0, 7.0, 0.0],
                [-7.0, 0.0, 0.0],
                [0.0, -7.0, 0.0],
                [7.0, 0.0, 0.0],
            ],
            &positions[..]
        );

        // Note V coordinate increases in the opposite direction to the Y coordinate.
        assert_eq!([[0.5, 0.0], [0.0, 0.5], [0.5, 1.0], [1.0, 0.5],], &uvs[..]);

        assert_eq!(&[[0.0, 0.0, 1.0]; 4], &normals[..]);
    }
}
