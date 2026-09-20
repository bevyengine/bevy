use bevy_asset::{
    prelude::AssetChanged, AsAssetId, Asset, AssetId, Assets, Handle, RenderAssetUsages,
};
use bevy_camera::{primitives::Aabb, visibility::NoAutoAabb};
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::{
    component::Component,
    query::{Changed, Or},
    reflect::ReflectComponent,
    system::{Query, Res, ResMut},
    template::FromTemplate,
};
use bevy_math::{Vec3, Vec3A};
use bevy_mesh::{
    Indices, Mesh, Mesh3d, MeshVertexAttribute, PrimitiveTopology, VertexAttributeValues,
    VertexFormat,
};
use bevy_pbr::MeshMaterial3d;
use bevy_reflect::{std_traits::ReflectDefault, Reflect};
use derive_more::derive::From;
use tracing::warn;

use crate::HairMaterial;

/// Per-vertex strand tangent (the direction the strand travels at this control
/// point, in local space), packed as `Snorm8x4` with `w` unused. Used by the
/// vertex shader to orient the ribbon and by the fragment shader for
/// anisotropic shading.
///
/// A "high" random id is used to avoid collisions with other custom attributes.
/// See [`MeshVertexAttribute`] for more info.
pub const ATTRIBUTE_HAIR_TANGENT: MeshVertexAttribute = MeshVertexAttribute::new(
    "Hair_Tangent",
    0x4841_4952_5441_4e47,
    VertexFormat::Snorm8x4,
);

/// Per-vertex strand parameters, packed as `Unorm8x4`:
///
/// * `x` — which side of the ribbon this vertex is on (`0` or `255`, read as `-1.0` or `1.0`),
/// * `y` — normalized arc-length position along the strand (`0` at the root, `255` at the tip),
/// * `z` — a per-strand pseudo-random value, stable for a given strand index,
/// * `w` — unused.
pub const ATTRIBUTE_HAIR_PARAMS: MeshVertexAttribute =
    MeshVertexAttribute::new("Hair_Params", 0x4841_4952_5041_524d, VertexFormat::Unorm8x4);

/// A single hair strand: a polyline of control points in the local space of the
/// entity that renders it, ordered from root to tip.
#[derive(Reflect, Clone, Debug, Default, PartialEq)]
#[reflect(Default, Clone, PartialEq)]
pub struct HairStrand {
    /// Control points from root to tip. Strands with fewer than two points are
    /// skipped when building the mesh.
    pub points: Vec<Vec3>,
}

impl HairStrand {
    /// Creates a strand from a list of control points ordered root to tip.
    pub fn new(points: impl Into<Vec<Vec3>>) -> Self {
        Self {
            points: points.into(),
        }
    }

    /// Total polyline length of the strand.
    pub fn length(&self) -> f32 {
        self.points.windows(2).map(|w| w[0].distance(w[1])).sum()
    }
}

impl From<Vec<Vec3>> for HairStrand {
    fn from(points: Vec<Vec3>) -> Self {
        Self { points }
    }
}

/// A collection of [`HairStrand`]s that is rendered as a set of thin,
/// view-facing ribbons.
///
/// Attach a [`Handle<HairStrands>`] to an entity through [`HairStrands3d`] and
/// give it a [`MeshMaterial3d<HairMaterial>`](bevy_pbr::MeshMaterial3d) to draw it.
/// The ribbon mesh is (re)built automatically whenever the asset changes.
///
/// # Example
///
/// ```
/// # use bevy_hair_strands::{HairStrand, HairStrands};
/// # use bevy_math::Vec3;
/// let strands = HairStrands::from_iter([
///     HairStrand::new([Vec3::ZERO, Vec3::Y * 0.5, Vec3::new(0.2, 0.8, 0.0)]),
///     HairStrand::new([Vec3::X * 0.1, Vec3::new(0.1, 0.6, 0.1)]),
/// ]);
/// assert_eq!(strands.len(), 2);
/// assert_eq!(strands.point_count(), 5);
/// ```
#[derive(Asset, Reflect, Clone, Debug, Default, PartialEq)]
#[reflect(Default, Clone, PartialEq)]
pub struct HairStrands {
    /// The strands, in local space.
    pub strands: Vec<HairStrand>,
}

impl HairStrands {
    /// Creates an empty set of strands.
    pub const fn new() -> Self {
        Self {
            strands: Vec::new(),
        }
    }

    /// Adds a strand.
    pub fn push(&mut self, strand: impl Into<HairStrand>) {
        self.strands.push(strand.into());
    }

    /// Number of strands.
    pub fn len(&self) -> usize {
        self.strands.len()
    }

    /// Whether there are no strands.
    pub fn is_empty(&self) -> bool {
        self.strands.is_empty()
    }

    /// Total number of control points across all strands.
    pub fn point_count(&self) -> usize {
        self.strands.iter().map(|s| s.points.len()).sum()
    }

    /// Iterator over the strands that can actually be rendered (at least two points).
    fn renderable(&self) -> impl Iterator<Item = &HairStrand> {
        self.strands.iter().filter(|s| s.points.len() >= 2)
    }

    /// Computes the local-space bounding box of all control points, grown by
    /// `padding` on every side to account for the ribbon width added on the GPU.
    ///
    /// Returns `None` if there are no renderable strands.
    pub fn aabb(&self, padding: f32) -> Option<Aabb> {
        let mut min = Vec3A::splat(f32::MAX);
        let mut max = Vec3A::splat(f32::MIN);
        let mut any = false;
        for point in self.renderable().flat_map(|s| s.points.iter()) {
            let p = Vec3A::from(*point);
            min = min.min(p);
            max = max.max(p);
            any = true;
        }
        any.then(|| {
            let padding = Vec3A::splat(padding.max(0.0));
            Aabb::from_min_max((min - padding).into(), (max + padding).into())
        })
    }

    /// Builds the ribbon [`Mesh`] for these strands.
    ///
    /// Every control point produces two vertices (one for each edge of the
    /// ribbon) that share the same position; the vertex shader pushes them apart
    /// perpendicular to both the strand tangent and the view direction. Each
    /// segment between consecutive control points becomes two triangles.
    ///
    /// The mesh carries [`Mesh::ATTRIBUTE_POSITION`], [`ATTRIBUTE_HAIR_TANGENT`]
    /// and [`ATTRIBUTE_HAIR_PARAMS`] (20 bytes a vertex), and is intended to be
    /// drawn with [`HairMaterial`](crate::HairMaterial).
    ///
    /// The mesh is for the render world only: once uploaded, its data is not
    /// kept in [`Assets<Mesh>`], since the strands themselves are the source
    /// of truth and the bounds come from [`Self::aabb`]. Rebuild it from the
    /// strands rather than reading it back.
    pub fn to_mesh(&self) -> Mesh {
        let point_count: usize = self.renderable().map(|s| s.points.len()).sum();
        let vertex_count = point_count * 2;

        let mut positions: Vec<[f32; 3]> = Vec::with_capacity(vertex_count);
        let mut tangents: Vec<[i8; 4]> = Vec::with_capacity(vertex_count);
        let mut params: Vec<[u8; 4]> = Vec::with_capacity(vertex_count);
        let mut indices: Vec<u32> = Vec::with_capacity(point_count.saturating_sub(1) * 6);

        for (strand_index, strand) in self.renderable().enumerate() {
            let points = &strand.points;
            let seed = pack_unorm8(strand_seed(strand_index));
            let total_length = strand.length();

            let base = positions.len() as u32;
            let mut arc_length = 0.0;
            let mut last_tangent = Vec3::Y;

            for (i, point) in points.iter().enumerate() {
                if i > 0 {
                    arc_length += points[i - 1].distance(*point);
                }
                // Central differences in the interior, one-sided at the ends.
                let prev = points[i.saturating_sub(1)];
                let next = points[(i + 1).min(points.len() - 1)];
                let tangent = (next - prev).try_normalize().unwrap_or(last_tangent);
                last_tangent = tangent;
                let tangent = pack_snorm8x4(tangent);

                let t = if total_length > 0.0 {
                    arc_length / total_length
                } else {
                    i as f32 / (points.len() - 1) as f32
                };
                let t = pack_unorm8(t);

                let position = point.to_array();
                for side in [0, 255] {
                    positions.push(position);
                    tangents.push(tangent);
                    params.push([side, t, seed, 0]);
                }
            }

            for i in 0..(points.len() as u32 - 1) {
                let a = base + i * 2;
                let b = a + 1;
                let c = a + 2;
                let d = a + 3;
                indices.extend_from_slice(&[a, b, c, b, d, c]);
            }
        }

        Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::RENDER_WORLD,
        )
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
        .with_inserted_attribute(
            ATTRIBUTE_HAIR_TANGENT,
            VertexAttributeValues::Snorm8x4(tangents),
        )
        .with_inserted_attribute(
            ATTRIBUTE_HAIR_PARAMS,
            VertexAttributeValues::Unorm8x4(params),
        )
        .with_inserted_indices(Indices::U32(indices))
    }
}

impl FromIterator<HairStrand> for HairStrands {
    fn from_iter<I: IntoIterator<Item = HairStrand>>(iter: I) -> Self {
        Self {
            strands: iter.into_iter().collect(),
        }
    }
}

/// A well-distributed pseudo-random value in `[0, 1)` for a strand index
/// (a golden-ratio low-discrepancy sequence), so neighbouring strands get
/// noticeably different shading variation, and any prefix of the range
/// `[0, f)` picks an even spread of the strands (which is how level of
/// detail thins them).
fn strand_seed(index: usize) -> f32 {
    const GOLDEN_RATIO_CONJUGATE: f64 = 0.618_033_988_749_895;
    ((index as f64 + 1.0) * GOLDEN_RATIO_CONJUGATE).fract() as f32
}

/// A value in `[0, 1]` as the GPU reads a `Unorm8`.
fn pack_unorm8(value: f32) -> u8 {
    (value.clamp(0.0, 1.0) * 255.0).round() as u8
}

/// A unit vector as the GPU reads a `Snorm8x4` (`w` unused).
fn pack_snorm8x4(v: Vec3) -> [i8; 4] {
    let pack = |x: f32| (x.clamp(-1.0, 1.0) * 127.0).round() as i8;
    [pack(v.x), pack(v.y), pack(v.z), 0]
}

/// A component that renders a [`HairStrands`] asset on this entity.
///
/// The ribbon [`Mesh3d`] and [`Aabb`] are generated and kept up to date by
/// [`HairStrandsPlugin`](crate::HairStrandsPlugin); do not set them yourself.
/// Pair with a [`MeshMaterial3d<HairMaterial>`](bevy_pbr::MeshMaterial3d).
///
/// # Example
///
/// ```ignore
/// commands.spawn((
///     HairStrands3d(hair_strands_handle),
///     MeshMaterial3d(hair_materials.add(HairMaterial::default())),
///     Transform::from_xyz(0.0, 1.0, 0.0),
/// ));
/// ```
#[derive(
    Component, FromTemplate, Clone, Debug, Default, Deref, DerefMut, Reflect, PartialEq, Eq, From,
)]
#[reflect(Component, Default, Clone, PartialEq)]
#[require(Mesh3d, NoAutoAabb, Aabb)]
pub struct HairStrands3d(pub Handle<HairStrands>);

impl AsAssetId for HairStrands3d {
    type Asset = HairStrands;

    fn as_asset_id(&self) -> AssetId<Self::Asset> {
        self.id()
    }
}

/// How far the bounds of an entity reach beyond its control points: its
/// [`HairStrandsBoundsPadding`] if it has one, else what its
/// [`HairMaterial`] can widen a ribbon by, else
/// [`HairStrandsBoundsPadding::DEFAULT`] while the material is not loaded.
fn bounds_padding(
    padding: Option<&HairStrandsBoundsPadding>,
    material: Option<&MeshMaterial3d<HairMaterial>>,
    materials: &Assets<HairMaterial>,
) -> f32 {
    if let Some(padding) = padding {
        return padding.0;
    }
    material
        .and_then(|material| materials.get(&material.0))
        .map_or(
            HairStrandsBoundsPadding::DEFAULT,
            HairMaterial::bounds_padding,
        )
}

/// Rebuilds the ribbon mesh of every entity whose [`HairStrands3d`] component
/// or [`HairStrands`] asset changed, and refreshes its [`Aabb`].
///
/// The bounding box is padded so ribbons that are widened in the vertex
/// shader are not culled at the edges of the view: by the entity's
/// [`HairStrandsBoundsPadding`], or else by what its [`HairMaterial`] needs.
pub fn update_hair_strand_meshes(
    mut query: Query<
        (
            &HairStrands3d,
            &mut Mesh3d,
            &mut Aabb,
            Option<&HairStrandsBoundsPadding>,
            Option<&MeshMaterial3d<HairMaterial>>,
        ),
        Or<(Changed<HairStrands3d>, AssetChanged<HairStrands3d>)>,
    >,
    strands: Res<Assets<HairStrands>>,
    materials: Res<Assets<HairMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    for (hair, mut mesh3d, mut aabb, padding, material) in &mut query {
        let Some(hair) = strands.get(&hair.0) else {
            // Not loaded yet; `AssetChanged` will fire once it is.
            continue;
        };
        let mesh = hair.to_mesh();

        // Reuse the mesh asset we generated earlier, if any, instead of churning
        // handles. (Its data has gone to the GPU, but the asset is still there
        // to be replaced.)
        if mesh3d.0 != Handle::default() && meshes.get(&mesh3d.0).is_some() {
            if let Err(err) = meshes.insert(&mesh3d.0, mesh) {
                warn!("Failed to update hair strand mesh: {err}");
            }
        } else {
            mesh3d.0 = meshes.add(mesh);
        }

        *aabb = hair
            .aabb(bounds_padding(padding, material, &materials))
            .unwrap_or_default();
    }
}

/// Refreshes the [`Aabb`] of entities whose [`HairStrandsBoundsPadding`] or
/// [`HairMaterial`] changed without rebuilding their mesh.
pub fn update_hair_strand_bounds(
    mut query: Query<
        (
            &HairStrands3d,
            &mut Aabb,
            Option<&HairStrandsBoundsPadding>,
            Option<&MeshMaterial3d<HairMaterial>>,
        ),
        Or<(
            Changed<HairStrandsBoundsPadding>,
            Changed<MeshMaterial3d<HairMaterial>>,
            AssetChanged<MeshMaterial3d<HairMaterial>>,
        )>,
    >,
    strands: Res<Assets<HairStrands>>,
    materials: Res<Assets<HairMaterial>>,
) {
    for (hair, mut aabb, padding, material) in &mut query {
        if let Some(hair) = strands.get(&hair.0) {
            *aabb = hair
                .aabb(bounds_padding(padding, material, &materials))
                .unwrap_or_default();
        }
    }
}

/// How far, in local units, to grow the [`Aabb`] of a [`HairStrands3d`] entity
/// beyond its control points.
///
/// Ribbons are widened on the GPU, so the bounds computed from the control
/// points alone would be slightly too small. Without this component the
/// padding is taken from the entity's [`HairMaterial`]
/// ([`HairMaterial::bounds_padding`]); set it yourself when that is not
/// right, for instance on an entity whose transform is scaled (the ribbon
/// width is in world units, the padding in local ones).
#[derive(Component, Clone, Copy, Debug, Reflect, PartialEq)]
#[reflect(Component, Default, Clone, PartialEq)]
pub struct HairStrandsBoundsPadding(pub f32);

impl HairStrandsBoundsPadding {
    /// Padding used while an entity has neither the component nor a loaded
    /// [`HairMaterial`].
    pub const DEFAULT: f32 = 0.05;
}

impl Default for HairStrandsBoundsPadding {
    fn default() -> Self {
        Self(Self::DEFAULT)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::MIN_KEEP_FRACTION;

    fn two_strands() -> HairStrands {
        HairStrands::from_iter([
            HairStrand::new([Vec3::ZERO, Vec3::Y, Vec3::Y * 2.0]),
            HairStrand::new([Vec3::X, Vec3::new(1.0, 1.0, 0.0)]),
        ])
    }

    #[test]
    fn mesh_topology() {
        let mesh = two_strands().to_mesh();
        // 5 control points -> 10 vertices; 3 segments -> 6 triangles -> 18 indices.
        assert_eq!(mesh.count_vertices(), 10);
        let Some(Indices::U32(indices)) = mesh.indices() else {
            panic!("expected u32 indices");
        };
        assert_eq!(indices.len(), 18);
        assert!(indices.iter().all(|&i| i < 10));
        // Second strand starts at vertex 6.
        assert_eq!(&indices[12..18], &[6, 7, 8, 7, 9, 8]);
        // Position, tangent and params: 12 + 4 + 4 bytes a vertex.
        assert_eq!(mesh.get_vertex_size(), 20);
        assert_eq!(mesh.asset_usage, RenderAssetUsages::RENDER_WORLD);
    }

    #[test]
    fn params_and_tangents() {
        let mesh = two_strands().to_mesh();
        let Some(VertexAttributeValues::Unorm8x4(params)) = mesh.attribute(ATTRIBUTE_HAIR_PARAMS)
        else {
            panic!("missing params");
        };
        let Some(VertexAttributeValues::Snorm8x4(tangents)) =
            mesh.attribute(ATTRIBUTE_HAIR_TANGENT)
        else {
            panic!("missing tangents");
        };
        // Sides alternate, t runs root->tip by arc length, seed is constant per strand.
        assert_eq!(params[0], [0, 0, params[0][2], 0]);
        assert_eq!(params[1], [255, 0, params[0][2], 0]);
        assert_eq!(params[2][1], 128);
        assert_eq!(params[5][1], 255);
        assert_eq!(params[5][2], params[0][2]);
        assert_ne!(params[6][2], params[0][2]);
        // First strand is straight up.
        assert_eq!(tangents[0], [0, 127, 0, 0]);
        assert_eq!(tangents[3], [0, 127, 0, 0]);
    }

    #[test]
    fn packing() {
        assert_eq!(pack_unorm8(0.0), 0);
        assert_eq!(pack_unorm8(1.0), 255);
        assert_eq!(pack_unorm8(2.0), 255);
        assert_eq!(pack_snorm8x4(Vec3::NEG_X), [-127, 0, 0, 0]);
        let diagonal = pack_snorm8x4(Vec3::ONE.normalize());
        assert_eq!(diagonal, [73, 73, 73, 0]);
        // Seeds are spread over the whole byte, so any prefix of the range
        // picks an even spread of the strands.
        let seeds: Vec<u8> = (0..16).map(|i| pack_unorm8(strand_seed(i))).collect();
        let kept = seeds.iter().filter(|&&s| s < 128).count();
        assert!((6..=10).contains(&kept), "{seeds:?}");
    }

    #[test]
    fn skips_degenerate_strands() {
        let mut strands = two_strands();
        strands.push(HairStrand::new([Vec3::ZERO]));
        strands.push(HairStrand::default());
        assert_eq!(strands.to_mesh().count_vertices(), 10);
        assert_eq!(strands.point_count(), 6);
    }

    #[test]
    fn coincident_points_do_not_produce_nan_tangents() {
        let strands = HairStrands::from_iter([HairStrand::new([Vec3::ZERO, Vec3::ZERO, Vec3::Y])]);
        let mesh = strands.to_mesh();
        let Some(VertexAttributeValues::Snorm8x4(tangents)) =
            mesh.attribute(ATTRIBUTE_HAIR_TANGENT)
        else {
            panic!("missing tangents");
        };
        // The first point takes the next segment's direction: straight up.
        assert_eq!(tangents[0], [0, 127, 0, 0]);
    }

    #[test]
    fn aabb_is_padded() {
        let aabb = two_strands().aabb(0.5).unwrap();
        assert_eq!(aabb.min(), Vec3A::new(-0.5, -0.5, -0.5));
        assert_eq!(aabb.max(), Vec3A::new(1.5, 2.5, 0.5));
        assert!(HairStrands::new().aabb(0.5).is_none());
    }

    #[test]
    fn padding_comes_from_the_material() {
        let mut materials = Assets::<HairMaterial>::default();
        let wide = HairMaterial {
            root_width: 0.02,
            tip_width: 0.01,
            min_pixel_width: 0.0,
            ..Default::default()
        };
        assert_eq!(wide.bounds_padding(), 0.01);
        let thinned = HairMaterial {
            min_pixel_width: 1.0,
            ..wide.clone()
        };
        assert_eq!(thinned.bounds_padding(), 0.01 / MIN_KEEP_FRACTION);
        let handle = MeshMaterial3d(materials.add(wide));
        assert_eq!(bounds_padding(None, Some(&handle), &materials), 0.01);
        // An explicit component wins; no material at all falls back.
        let explicit = HairStrandsBoundsPadding(0.3);
        assert_eq!(
            bounds_padding(Some(&explicit), Some(&handle), &materials),
            0.3
        );
        assert_eq!(
            bounds_padding(None, None, &materials),
            HairStrandsBoundsPadding::DEFAULT
        );
        let unloaded = MeshMaterial3d(Handle::<HairMaterial>::default());
        assert_eq!(
            bounds_padding(None, Some(&unloaded), &materials),
            HairStrandsBoundsPadding::DEFAULT
        );
    }
}
