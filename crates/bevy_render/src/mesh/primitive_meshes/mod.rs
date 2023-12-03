use super::{Indices, Mesh, PrimitiveTopology};
use bevy_math::{
    primitives::{Circle, Cuboid, Rectangle, RegularPolygon, Triangle2d, WindingOrder},
    Vec2,
};

impl From<Cuboid> for Mesh {
    fn from(cuboid: Cuboid) -> Self {
        let min = -cuboid.half_extents;
        let max = cuboid.half_extents;

        // suppose Y-up right hand, and camera look from +z to -z
        let vertices = &[
            // Front
            ([min.x, min.y, max.z], [0.0, 0.0, 1.0], [0.0, 0.0]),
            ([max.x, min.y, max.z], [0.0, 0.0, 1.0], [1.0, 0.0]),
            ([max.x, max.y, max.z], [0.0, 0.0, 1.0], [1.0, 1.0]),
            ([min.x, max.y, max.z], [0.0, 0.0, 1.0], [0.0, 1.0]),
            // Back
            ([min.x, max.y, min.z], [0.0, 0.0, -1.0], [1.0, 0.0]),
            ([max.x, max.y, min.z], [0.0, 0.0, -1.0], [0.0, 0.0]),
            ([max.x, min.y, min.z], [0.0, 0.0, -1.0], [0.0, 1.0]),
            ([min.x, min.y, min.z], [0.0, 0.0, -1.0], [1.0, 1.0]),
            // Right
            ([max.x, min.y, min.z], [1.0, 0.0, 0.0], [0.0, 0.0]),
            ([max.x, max.y, min.z], [1.0, 0.0, 0.0], [1.0, 0.0]),
            ([max.x, max.y, max.z], [1.0, 0.0, 0.0], [1.0, 1.0]),
            ([max.x, min.y, max.z], [1.0, 0.0, 0.0], [0.0, 1.0]),
            // Left
            ([min.x, min.y, max.z], [-1.0, 0.0, 0.0], [1.0, 0.0]),
            ([min.x, max.y, max.z], [-1.0, 0.0, 0.0], [0.0, 0.0]),
            ([min.x, max.y, min.z], [-1.0, 0.0, 0.0], [0.0, 1.0]),
            ([min.x, min.y, min.z], [-1.0, 0.0, 0.0], [1.0, 1.0]),
            // Top
            ([max.x, max.y, min.z], [0.0, 1.0, 0.0], [1.0, 0.0]),
            ([min.x, max.y, min.z], [0.0, 1.0, 0.0], [0.0, 0.0]),
            ([min.x, max.y, max.z], [0.0, 1.0, 0.0], [0.0, 1.0]),
            ([max.x, max.y, max.z], [0.0, 1.0, 0.0], [1.0, 1.0]),
            // Bottom
            ([max.x, min.y, max.z], [0.0, -1.0, 0.0], [0.0, 0.0]),
            ([min.x, min.y, max.z], [0.0, -1.0, 0.0], [1.0, 0.0]),
            ([min.x, min.y, min.z], [0.0, -1.0, 0.0], [1.0, 1.0]),
            ([max.x, min.y, min.z], [0.0, -1.0, 0.0], [0.0, 1.0]),
        ];

        let positions: Vec<_> = vertices.iter().map(|(p, _, _)| *p).collect();
        let normals: Vec<_> = vertices.iter().map(|(_, n, _)| *n).collect();
        let uvs: Vec<_> = vertices.iter().map(|(_, _, uv)| *uv).collect();

        let indices = Indices::U32(vec![
            0, 1, 2, 2, 3, 0, // front
            4, 5, 6, 6, 7, 4, // back
            8, 9, 10, 10, 11, 8, // right
            12, 13, 14, 14, 15, 12, // left
            16, 17, 18, 18, 19, 16, // top
            20, 21, 22, 22, 23, 20, // bottom
        ]);

        Mesh::new(PrimitiveTopology::TriangleList)
            .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
            .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
            .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs)
            .with_indices(Some(indices))
    }
}

pub trait MeshableRectangle {
    fn mesh(&self, flip: bool) -> Mesh;
}

impl MeshableRectangle for Rectangle {
    fn mesh(&self, flip: bool) -> Mesh {
        let (u_left, u_right) = if flip { (1.0, 0.0) } else { (0.0, 1.0) };
        let [hw, hh] = [self.half_width, self.half_height];
        let vertices = [
            ([-hw, -hh, 0.0], [0.0, 0.0, 1.0], [u_left, 1.0]),
            ([-hw, hh, 0.0], [0.0, 0.0, 1.0], [u_left, 0.0]),
            ([hw, hh, 0.0], [0.0, 0.0, 1.0], [u_right, 0.0]),
            ([hw, -hh, 0.0], [0.0, 0.0, 1.0], [u_right, 1.0]),
        ];

        let indices = Indices::U32(vec![0, 2, 1, 0, 3, 2]);

        let positions: Vec<_> = vertices.iter().map(|(p, _, _)| *p).collect();
        let normals: Vec<_> = vertices.iter().map(|(_, n, _)| *n).collect();
        let uvs: Vec<_> = vertices.iter().map(|(_, _, uv)| *uv).collect();

        Mesh::new(PrimitiveTopology::TriangleList)
            .with_indices(Some(indices))
            .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
            .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
            .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs)
    }
}

impl From<Rectangle> for Mesh {
    fn from(rectangle: Rectangle) -> Self {
        rectangle.mesh(false)
    }
}

impl From<RegularPolygon> for Mesh {
    fn from(polygon: RegularPolygon) -> Self {
        let sides = polygon.sides;

        debug_assert!(sides > 2, "RegularPolygon requires at least 3 sides.");

        let mut positions = Vec::with_capacity(sides);
        let mut normals = Vec::with_capacity(sides);
        let mut uvs = Vec::with_capacity(sides);

        let step = std::f32::consts::TAU / sides as f32;
        for i in 0..sides {
            let theta = std::f32::consts::FRAC_PI_2 - i as f32 * step;
            let (sin, cos) = theta.sin_cos();

            positions.push([
                cos * polygon.circumcircle.radius,
                sin * polygon.circumcircle.radius,
                0.0,
            ]);
            normals.push([0.0, 0.0, 1.0]);
            uvs.push([0.5 * (cos + 1.0), 1.0 - 0.5 * (sin + 1.0)]);
        }

        let mut indices = Vec::with_capacity((sides - 2) * 3);
        for i in 1..(sides as u32 - 1) {
            indices.extend_from_slice(&[0, i + 1, i]);
        }

        Mesh::new(PrimitiveTopology::TriangleList)
            .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
            .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
            .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs)
            .with_indices(Some(Indices::U32(indices)))
    }
}

pub trait MeshableCircle {
    fn mesh(&self, vertices: usize) -> Mesh;
}

impl MeshableCircle for Circle {
    fn mesh(&self, vertices: usize) -> Mesh {
        Mesh::from(RegularPolygon::new(self.radius, vertices))
    }
}

impl From<Circle> for Mesh {
    fn from(circle: Circle) -> Self {
        circle.mesh(64)
    }
}

impl From<Triangle2d> for Mesh {
    fn from(triangle: Triangle2d) -> Self {
        let [a, b, c] = triangle.vertices;
        let max = a.min(b).min(c).abs().max(a.max(b).max(c)) * Vec2::new(1.0, -1.0);
        let [norm_a, norm_b, norm_c] = [(a) / max, (b) / max, (c) / max];
        let vertices = [
            (a.extend(0.0), [0.0, 0.0, 1.0], norm_a / 2.0 + 0.5),
            (b.extend(0.0), [0.0, 0.0, 1.0], norm_b / 2.0 + 0.5),
            (c.extend(0.0), [0.0, 0.0, 1.0], norm_c / 2.0 + 0.5),
        ];

        let indices = if triangle.winding_order() == WindingOrder::CounterClockwise {
            Indices::U32(vec![0, 1, 2])
        } else {
            Indices::U32(vec![0, 2, 1])
        };

        let positions: Vec<_> = vertices.iter().map(|(p, _, _)| *p).collect();
        let normals: Vec<_> = vertices.iter().map(|(_, n, _)| *n).collect();
        let uvs: Vec<_> = vertices.iter().map(|(_, _, uv)| *uv).collect();

        Mesh::new(PrimitiveTopology::TriangleList)
            .with_indices(Some(indices))
            .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
            .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
            .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs)
    }
}
