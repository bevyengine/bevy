use super::{GenerateTangentsError, Mesh};
use bevy_math::{Vec2, Vec3A};
use wgpu_types::{PrimitiveTopology, VertexFormat};

struct TriangleIndexIter<'a, I>(&'a mut I);

impl<'a, I> Iterator for TriangleIndexIter<'a, I>
where
    I: Iterator<Item = usize>,
{
    type Item = [usize; 3];
    fn next(&mut self) -> Option<[usize; 3]> {
        let i = &mut self.0;
        match (i.next(), i.next(), i.next()) {
            (Some(i1), Some(i2), Some(i3)) => Some([i1, i2, i3]),
            _ => None,
        }
    }
}

pub(crate) fn generate_tangents_for_mesh(
    mesh: &Mesh,
) -> Result<Vec<[f32; 4]>, GenerateTangentsError> {
    let positions = mesh.attribute(Mesh::ATTRIBUTE_POSITION);
    let normals = mesh.attribute(Mesh::ATTRIBUTE_NORMAL);
    let uvs = mesh.attribute(Mesh::ATTRIBUTE_UV_0);
    let indices = mesh.indices();
    let primitive_topology = mesh.primitive_topology();

    if primitive_topology != PrimitiveTopology::TriangleList {
        return Err(GenerateTangentsError::UnsupportedTopology(
            primitive_topology,
        ));
    }

    match (positions, normals, uvs, indices) {
        (None, _, _, _) => Err(GenerateTangentsError::MissingVertexAttribute(
            Mesh::ATTRIBUTE_POSITION.name,
        )),
        (_, None, _, _) => Err(GenerateTangentsError::MissingVertexAttribute(
            Mesh::ATTRIBUTE_NORMAL.name,
        )),
        (_, _, None, _) => Err(GenerateTangentsError::MissingVertexAttribute(
            Mesh::ATTRIBUTE_UV_0.name,
        )),
        (_, _, _, None) => Err(GenerateTangentsError::MissingIndices),
        (Some(positions), Some(normals), Some(uvs), Some(indices)) => {
            let positions = positions.as_float3().ok_or(
                GenerateTangentsError::InvalidVertexAttributeFormat(
                    Mesh::ATTRIBUTE_POSITION.name,
                    VertexFormat::Float32x3,
                ),
            )?;
            let normals =
                normals
                    .as_float3()
                    .ok_or(GenerateTangentsError::InvalidVertexAttributeFormat(
                        Mesh::ATTRIBUTE_NORMAL.name,
                        VertexFormat::Float32x3,
                    ))?;
            let uvs =
                uvs.as_float2()
                    .ok_or(GenerateTangentsError::InvalidVertexAttributeFormat(
                        Mesh::ATTRIBUTE_UV_0.name,
                        VertexFormat::Float32x2,
                    ))?;
            let vertex_count = positions.len();
            let mut tangents = vec![Vec3A::ZERO; vertex_count];
            let mut bi_tangents = vec![Vec3A::ZERO; vertex_count];

            for [i1, i2, i3] in TriangleIndexIter(&mut indices.iter()) {
                let v1 = Vec3A::from_array(positions[i1]);
                let v2 = Vec3A::from_array(positions[i2]);
                let v3 = Vec3A::from_array(positions[i3]);

                let w1 = Vec2::from(uvs[i1]);
                let w2 = Vec2::from(uvs[i2]);
                let w3 = Vec2::from(uvs[i3]);

                let delta_pos1 = v2 - v1;
                let delta_pos2 = v3 - v1;

                let delta_uv1 = w2 - w1;
                let delta_uv2 = w3 - w1;

                let determinant = delta_uv1.x * delta_uv2.y - delta_uv1.y * delta_uv2.x;

                // check for degenerate triangles
                if determinant.abs() > 1e-6 {
                    let r = 1.0 / determinant;
                    let tangent = (delta_pos1 * delta_uv2.y - delta_pos2 * delta_uv1.y) * r;
                    let bi_tangent = (delta_pos2 * delta_uv1.x - delta_pos1 * delta_uv2.x) * r;

                    tangents[i1] += tangent;
                    tangents[i2] += tangent;
                    tangents[i3] += tangent;

                    bi_tangents[i1] += bi_tangent;
                    bi_tangents[i2] += bi_tangent;
                    bi_tangents[i3] += bi_tangent;
                }
            }

            let mut result_tangents = Vec::with_capacity(vertex_count);

            for i in 0..vertex_count {
                let normal = Vec3A::from_array(normals[i]);
                let tangent = tangents[i].normalize();
                // Gram-Schmidt orthogonalization
                let tangent = (tangent - normal * normal.dot(tangent)).normalize();
                let bi_tangent = bi_tangents[i];
                let handedness = if normal.cross(tangent).dot(bi_tangent) > 0.0 {
                    1.0
                } else {
                    -1.0
                };
                // Both the gram-schmidt and mikktspace algorithms seem to assume left-handedness,
                // so we flip the sign to correct for this. The extra multiplication here is
                // negligible and it's done as a separate step to better document that it's
                // a deviation from the general algorithm. The generated mikktspace tangents are
                // also post processed to flip the sign.
                let handedness = handedness * -1.0;
                result_tangents.push([tangent.x, tangent.y, tangent.z, handedness]);
            }

            Ok(result_tangents)
        }
    }
}

#[cfg(test)]
mod tests {
    use bevy_math::{primitives::*, Vec2, Vec3, Vec3A};

    use crate::{Mesh, TangentCalculationStrategy};

    // The tangents should be very close for simple shapes
    fn compare_tangents(mut mesh: Mesh) {
        let hq_tangents: Vec<[f32; 4]> = {
            mesh.remove_attribute(Mesh::ATTRIBUTE_TANGENT);
            mesh.compute_tangents(TangentCalculationStrategy::HighQuality)
                .expect("compute_tangents(HighQuality)");
            mesh.attribute(Mesh::ATTRIBUTE_TANGENT)
                .expect("hq_tangents.attribute(tangent)")
                .as_float4()
                .expect("hq_tangents.as_float4")
                .to_vec()
        };

        let fa_tangents: Vec<[f32; 4]> = {
            mesh.remove_attribute(Mesh::ATTRIBUTE_TANGENT);
            mesh.compute_tangents(TangentCalculationStrategy::FastApproximation)
                .expect("compute_tangents(FastApproximation)");
            mesh.attribute(Mesh::ATTRIBUTE_TANGENT)
                .expect("fa_tangents.attribute(tangent)")
                .as_float4()
                .expect("fa_tangents.as_float4")
                .to_vec()
        };

        for (hq, fa) in hq_tangents.iter().zip(fa_tangents.iter()) {
            assert_eq!(hq[3], fa[3], "handedness");
            let hq = Vec3A::from_slice(hq);
            let fa = Vec3A::from_slice(fa);
            let angle = hq.angle_between(fa);
            let threshold = 15.0f32.to_radians();
            assert!(
                angle < threshold,
                "tangents differ significantly: hq = {:?}, fa = {:?}, angle.to_degrees() = {}",
                hq,
                fa,
                angle.to_degrees()
            );
        }
    }

    #[test]
    fn cuboid() {
        compare_tangents(Mesh::from(Cuboid::new(10.0, 10.0, 10.0)));
    }

    #[test]
    fn capsule3d() {
        compare_tangents(Mesh::from(Capsule3d::new(10.0, 10.0)));
    }

    #[test]
    fn plane3d() {
        compare_tangents(Mesh::from(Plane3d::new(Vec3::Y, Vec2::splat(10.0))));
    }

    #[test]
    fn cylinder() {
        compare_tangents(Mesh::from(Cylinder::new(10.0, 10.0)));
    }

    #[test]
    fn torus() {
        compare_tangents(Mesh::from(Torus::new(10.0, 100.0)));
    }

    #[test]
    fn rhombus() {
        compare_tangents(Mesh::from(Rhombus::new(10.0, 100.0)));
    }

    #[test]
    fn tetrahedron() {
        compare_tangents(Mesh::from(Tetrahedron::default()));
    }

    #[test]
    fn cone() {
        compare_tangents(Mesh::from(Cone::new(10.0, 100.0)));
    }
}
