use bevy_asset::{embedded_path, Asset, AssetPath};
use bevy_color::{Color, LinearRgba};
use bevy_mesh::{Mesh, MeshVertexBufferLayoutRef};
use bevy_pbr::{Material, MaterialPipeline, MaterialPipelineKey};
use bevy_reflect::{std_traits::ReflectDefault, Reflect};
use bevy_render::render_resource::{
    AsBindGroup, RenderPipelineDescriptor, ShaderType, SpecializedMeshPipelineError,
};
use bevy_shader::ShaderRef;

use crate::{ATTRIBUTE_HAIR_PARAMS, ATTRIBUTE_HAIR_STRAND, ATTRIBUTE_HAIR_TANGENT};

fn shader_ref(path: std::path::PathBuf) -> ShaderRef {
    ShaderRef::Path(AssetPath::from_path_buf(path).with_source("embedded"))
}

/// A material for rendering [`HairStrands`](crate::HairStrands) ribbons.
///
/// Shading follows the Kajiya-Kay / Scheuermann model: a wrapped diffuse term
/// plus two anisotropic specular highlights whose tangents are shifted along
/// the ribbon normal, so the primary (uncolored) and secondary (colored)
/// highlights separate as they do on real hair. Every light in the scene
/// contributes, and shadows, fog and tonemapping behave like `StandardMaterial`.
///
/// Ribbons are widened in the vertex shader between [`root_width`](Self::root_width)
/// and [`tip_width`](Self::tip_width) and always face the camera. A ribbon
/// thinner than a pixel is drawn a pixel wide and its true width carried as
/// coverage, which alpha-to-coverage turns into samples under multisampling
/// and an ordered dither into a keep-or-discard without it, so fine strands
/// neither shimmer nor vanish. The material is opaque and double-sided.
///
/// Indirect light comes from wherever `StandardMaterial` takes it: the ambient
/// colour, an environment map, an irradiance volume and screen-space ambient
/// occlusion. Deep in the hair mass less light gets in
/// ([`volume_occlusion`](Self::volume_occlusion)), baked per point from the
/// density of strands around it when the mesh is built.
///
/// Far away, where a strand would be thinner than
/// [`min_pixel_width`](Self::min_pixel_width), only a fraction of the strands
/// is drawn, widened to keep the same coverage, so distant hair costs a
/// fraction of the fragments and shadow maps thin by their own resolution.
#[derive(Asset, AsBindGroup, Reflect, Debug, Clone, PartialEq)]
#[reflect(Default, Debug, Clone, PartialEq)]
#[uniform(0, HairMaterialUniform)]
pub struct HairMaterial {
    /// Diffuse albedo of the strands.
    pub base_color: Color,
    /// Tint of the primary specular highlight. On real hair this is close to the
    /// light color (white), sitting slightly root-ward of the secondary highlight.
    pub specular_color: Color,
    /// Tint of the secondary specular highlight. This is light that has entered
    /// and exited the strand, so it usually carries the hair color.
    pub secondary_specular_color: Color,
    /// Ribbon width, in world units, at the root of each strand.
    pub root_width: f32,
    /// Ribbon width, in world units, at the tip of each strand.
    pub tip_width: f32,
    /// How far the primary highlight's tangent is shifted along the ribbon
    /// normal. Negative values move it toward the root.
    pub specular_shift: f32,
    /// Sharpness of the primary highlight; higher is tighter.
    pub specular_exponent: f32,
    /// How far the secondary highlight's tangent is shifted along the ribbon
    /// normal. Usually opposite in sign to [`specular_shift`](Self::specular_shift).
    pub secondary_shift: f32,
    /// Sharpness of the secondary highlight; usually broader than the primary.
    pub secondary_exponent: f32,
    /// Brightness multiplier at the root of each strand (`1.0` at the tip),
    /// approximating the occlusion from neighbouring strands. `1.0` disables it.
    pub root_occlusion: f32,
    /// Per-strand random brightness variation, `0.0` for none. `0.2` varies
    /// each strand's albedo by up to ±20%.
    pub color_variation: f32,
    /// Level of detail: once a strand's root would be drawn thinner than this
    /// many pixels (in the main view or a shadow map), only the fraction of
    /// the strands that keeps the survivors this wide is drawn, each widened
    /// to cover for the rest. Never fewer than [`MIN_KEEP_FRACTION`] of them
    /// are kept. `0.0` draws every strand at every distance.
    pub min_pixel_width: f32,
    /// How much light the interior of the hair mass loses, `0.0` for none:
    /// a point that is fully buried in strands is darkened by this fraction,
    /// one at the surface not at all. Applies to every light, direct and
    /// indirect, as the crate's stand-in for hair shadowing hair.
    pub volume_occlusion: f32,
}

/// The smallest fraction of an entity's strands that level of detail keeps,
/// so the survivors are never widened by more than its reciprocal. Mirrored
/// in `hair_functions.wesl`.
pub const MIN_KEEP_FRACTION: f32 = 0.125;

impl HairMaterial {
    /// How far, in world units, a ribbon of this material can reach beyond
    /// its control points: half the widest strand, times what level of
    /// detail may widen it by. [`HairStrands3d`](crate::HairStrands3d)
    /// entities without a [`HairStrandsBoundsPadding`](crate::HairStrandsBoundsPadding)
    /// have their bounds padded by this.
    pub fn bounds_padding(&self) -> f32 {
        let widening = if self.min_pixel_width > 0.0 {
            1.0 / MIN_KEEP_FRACTION
        } else {
            1.0
        };
        0.5 * self.root_width.max(self.tip_width) * widening
    }
}

impl Default for HairMaterial {
    fn default() -> Self {
        Self {
            base_color: Color::srgb(0.35, 0.2, 0.08),
            specular_color: Color::srgb(0.7, 0.65, 0.6),
            secondary_specular_color: Color::srgb(0.55, 0.35, 0.15),
            root_width: 0.01,
            tip_width: 0.003,
            specular_shift: -0.15,
            specular_exponent: 120.0,
            secondary_shift: 0.15,
            secondary_exponent: 24.0,
            root_occlusion: 0.35,
            color_variation: 0.15,
            min_pixel_width: 1.0,
            volume_occlusion: 0.5,
        }
    }
}

/// The GPU representation of a [`HairMaterial`]. Field meanings match the
/// material; colors are linear.
#[derive(Clone, Default, ShaderType)]
pub struct HairMaterialUniform {
    /// [`HairMaterial::base_color`] in linear space.
    pub base_color: LinearRgba,
    /// [`HairMaterial::specular_color`] in linear space.
    pub specular_color: LinearRgba,
    /// [`HairMaterial::secondary_specular_color`] in linear space.
    pub secondary_specular_color: LinearRgba,
    /// [`HairMaterial::root_width`].
    pub root_width: f32,
    /// [`HairMaterial::tip_width`].
    pub tip_width: f32,
    /// [`HairMaterial::specular_shift`].
    pub specular_shift: f32,
    /// [`HairMaterial::specular_exponent`].
    pub specular_exponent: f32,
    /// [`HairMaterial::secondary_shift`].
    pub secondary_shift: f32,
    /// [`HairMaterial::secondary_exponent`].
    pub secondary_exponent: f32,
    /// [`HairMaterial::root_occlusion`].
    pub root_occlusion: f32,
    /// [`HairMaterial::color_variation`].
    pub color_variation: f32,
    /// [`HairMaterial::min_pixel_width`].
    pub min_pixel_width: f32,
    /// [`HairMaterial::volume_occlusion`].
    pub volume_occlusion: f32,
}

impl From<&HairMaterial> for HairMaterialUniform {
    fn from(material: &HairMaterial) -> Self {
        Self {
            base_color: material.base_color.to_linear(),
            specular_color: material.specular_color.to_linear(),
            secondary_specular_color: material.secondary_specular_color.to_linear(),
            root_width: material.root_width,
            tip_width: material.tip_width,
            specular_shift: material.specular_shift,
            specular_exponent: material.specular_exponent,
            secondary_shift: material.secondary_shift,
            secondary_exponent: material.secondary_exponent,
            root_occlusion: material.root_occlusion,
            color_variation: material.color_variation,
            min_pixel_width: material.min_pixel_width,
            volume_occlusion: material.volume_occlusion,
        }
    }
}

impl Material for HairMaterial {
    fn vertex_shader() -> ShaderRef {
        shader_ref(embedded_path!("hair.wesl"))
    }

    fn fragment_shader() -> ShaderRef {
        shader_ref(embedded_path!("hair.wesl"))
    }

    fn prepass_vertex_shader() -> ShaderRef {
        shader_ref(embedded_path!("hair_prepass.wesl"))
    }

    fn specialize(
        _pipeline: &MaterialPipeline,
        descriptor: &mut RenderPipelineDescriptor,
        layout: &MeshVertexBufferLayoutRef,
        _key: MaterialPipelineKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        let mut attributes = vec![
            Mesh::ATTRIBUTE_POSITION.at_shader_location(0),
            ATTRIBUTE_HAIR_TANGENT.at_shader_location(1),
            ATTRIBUTE_HAIR_PARAMS.at_shader_location(2),
        ];
        // Strands with their own colour or width carry a fourth attribute;
        // the shaders read it only when told it is there.
        if layout.0.contains(ATTRIBUTE_HAIR_STRAND) {
            attributes.push(ATTRIBUTE_HAIR_STRAND.at_shader_location(3));
            descriptor
                .vertex
                .shader_defs
                .push("HAIR_STRAND_ATTRIBUTES".into());
            if let Some(fragment) = descriptor.fragment.as_mut() {
                fragment.shader_defs.push("HAIR_STRAND_ATTRIBUTES".into());
            }
        }
        let vertex_layout = layout.0.get_layout(&attributes)?;
        descriptor.vertex.buffers = vec![vertex_layout];
        // In the main pass under multisampling, the coverage of a ribbon
        // thinner than a pixel goes out as alpha and becomes samples. The
        // prepass and shadow passes draw the true width and have no alpha.
        let prepass = descriptor
            .label
            .as_deref()
            .is_some_and(|label| label.starts_with("prepass"));
        if !prepass && descriptor.multisample.count > 1 {
            descriptor.multisample.alpha_to_coverage_enabled = true;
        }
        // Ribbons are flat and always face the camera, so backface culling
        // would only ever remove geometry we want to keep.
        descriptor.primitive.cull_mode = None;
        Ok(())
    }
}
