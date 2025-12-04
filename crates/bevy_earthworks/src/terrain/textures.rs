//! Terrain texture atlas system.
//!
//! This module provides texture atlas support for terrain materials,
//! allowing PBR textures instead of simple vertex colors.

use bevy_asset::{Assets, Handle};
use bevy_ecs::prelude::*;
use bevy_image::Image;
use bevy_math::Vec2;
use bevy_pbr::StandardMaterial;
use bevy_reflect::Reflect;

use super::materials::MaterialId;

/// UV region in a texture atlas.
#[derive(Clone, Copy, Debug, Default, Reflect)]
pub struct AtlasRegion {
    /// Top-left UV coordinate.
    pub min: Vec2,
    /// Bottom-right UV coordinate.
    pub max: Vec2,
}

impl AtlasRegion {
    /// Creates a new atlas region.
    pub fn new(min_u: f32, min_v: f32, max_u: f32, max_v: f32) -> Self {
        Self {
            min: Vec2::new(min_u, min_v),
            max: Vec2::new(max_u, max_v),
        }
    }

    /// Creates a region for a grid-based atlas.
    ///
    /// # Arguments
    /// * `col` - Column index (0-based, left to right)
    /// * `row` - Row index (0-based, top to bottom)
    /// * `cols` - Total columns in the atlas
    /// * `rows` - Total rows in the atlas
    pub fn from_grid(col: u32, row: u32, cols: u32, rows: u32) -> Self {
        let cell_width = 1.0 / cols as f32;
        let cell_height = 1.0 / rows as f32;

        Self {
            min: Vec2::new(col as f32 * cell_width, row as f32 * cell_height),
            max: Vec2::new((col + 1) as f32 * cell_width, (row + 1) as f32 * cell_height),
        }
    }

    /// Maps a local UV (0-1 range) to this atlas region.
    pub fn map_uv(&self, local_uv: Vec2) -> Vec2 {
        let size = self.max - self.min;
        self.min + local_uv * size
    }

    /// Maps a tiled UV to this atlas region with proper wrapping.
    pub fn map_uv_tiled(&self, local_uv: Vec2) -> Vec2 {
        let size = self.max - self.min;
        let wrapped = Vec2::new(local_uv.x.fract(), local_uv.y.fract());
        self.min + wrapped * size
    }
}

/// Configuration for a single terrain material's textures.
#[derive(Clone, Debug, Default, Reflect)]
pub struct TerrainMaterialTexture {
    /// Atlas region for the base color/albedo.
    pub albedo_region: AtlasRegion,
    /// Atlas region for the normal map (if using combined atlas).
    pub normal_region: Option<AtlasRegion>,
    /// UV scale factor for tiling (larger = more repetition).
    pub uv_scale: f32,
}

impl TerrainMaterialTexture {
    /// Creates a new terrain material texture config.
    pub fn new(region: AtlasRegion) -> Self {
        Self {
            albedo_region: region,
            normal_region: None,
            uv_scale: 1.0,
        }
    }

    /// Sets the UV scale factor.
    pub fn with_uv_scale(mut self, scale: f32) -> Self {
        self.uv_scale = scale;
        self
    }
}

/// Resource holding terrain texture atlas configuration.
///
/// # Usage
///
/// 1. Create your texture atlas image (e.g., 2x4 grid with 8 materials)
/// 2. Load it as a Bevy image asset
/// 3. Configure this resource with the proper UV regions
/// 4. The terrain meshing will use these UVs instead of vertex colors
///
/// # Example Atlas Layout (2x4 grid)
/// ```text
/// ┌────┬────┐
/// │Dirt│Clay│
/// ├────┼────┤
/// │Rock│Soil│
/// ├────┼────┤
/// │Grvl│Sand│
/// ├────┼────┤
/// │Watr│    │
/// └────┴────┘
/// ```
#[derive(Resource, Default, Reflect)]
pub struct TerrainTextureAtlas {
    /// Handle to the albedo texture atlas.
    pub albedo_atlas: Option<Handle<Image>>,
    /// Handle to the normal map atlas (optional).
    pub normal_atlas: Option<Handle<Image>>,
    /// Per-material texture configurations.
    pub materials: [Option<TerrainMaterialTexture>; 8],
    /// Whether textures are loaded and ready.
    pub ready: bool,
}

impl TerrainTextureAtlas {
    /// Creates a new empty terrain texture atlas.
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a standard 2x4 grid atlas layout.
    ///
    /// Materials are arranged:
    /// - Row 0: Dirt (0,0), Clay (1,0)
    /// - Row 1: Rock (0,1), Topsoil (1,1)
    /// - Row 2: Gravel (0,2), Sand (1,2)
    /// - Row 3: Water (0,3), Reserved (1,3)
    pub fn with_standard_layout() -> Self {
        let mut atlas = Self::new();

        // Dirt (material 1)
        atlas.materials[MaterialId::Dirt as usize] = Some(
            TerrainMaterialTexture::new(AtlasRegion::from_grid(0, 0, 2, 4))
                .with_uv_scale(2.0)
        );

        // Clay (material 2)
        atlas.materials[MaterialId::Clay as usize] = Some(
            TerrainMaterialTexture::new(AtlasRegion::from_grid(1, 0, 2, 4))
                .with_uv_scale(2.0)
        );

        // Rock (material 3)
        atlas.materials[MaterialId::Rock as usize] = Some(
            TerrainMaterialTexture::new(AtlasRegion::from_grid(0, 1, 2, 4))
                .with_uv_scale(1.5)
        );

        // Topsoil (material 4)
        atlas.materials[MaterialId::Topsoil as usize] = Some(
            TerrainMaterialTexture::new(AtlasRegion::from_grid(1, 1, 2, 4))
                .with_uv_scale(3.0)
        );

        // Gravel (material 5)
        atlas.materials[MaterialId::Gravel as usize] = Some(
            TerrainMaterialTexture::new(AtlasRegion::from_grid(0, 2, 2, 4))
                .with_uv_scale(2.5)
        );

        // Sand (material 6)
        atlas.materials[MaterialId::Sand as usize] = Some(
            TerrainMaterialTexture::new(AtlasRegion::from_grid(1, 2, 2, 4))
                .with_uv_scale(2.0)
        );

        // Water (material 7)
        atlas.materials[MaterialId::Water as usize] = Some(
            TerrainMaterialTexture::new(AtlasRegion::from_grid(0, 3, 2, 4))
                .with_uv_scale(1.0)
        );

        atlas
    }

    /// Sets the albedo atlas texture.
    pub fn with_albedo(mut self, handle: Handle<Image>) -> Self {
        self.albedo_atlas = Some(handle);
        self
    }

    /// Sets the normal atlas texture.
    pub fn with_normals(mut self, handle: Handle<Image>) -> Self {
        self.normal_atlas = Some(handle);
        self
    }

    /// Gets the texture config for a material, if any.
    pub fn get_material(&self, material: MaterialId) -> Option<&TerrainMaterialTexture> {
        self.materials.get(material as usize).and_then(|m| m.as_ref())
    }

    /// Checks if textures are configured for a material.
    pub fn has_texture(&self, material: MaterialId) -> bool {
        self.get_material(material).is_some() && self.albedo_atlas.is_some()
    }

    /// Converts this atlas configuration to an AtlasUvConfig for mesh generation.
    ///
    /// The returned config can be safely sent to async mesh tasks since it
    /// contains only UV data (no asset handles).
    pub fn to_uv_config(&self) -> super::meshing::AtlasUvConfig {
        let mut config = super::meshing::AtlasUvConfig::new();

        // Copy UV regions for each material
        for (i, mat_config) in self.materials.iter().enumerate() {
            if let Some(mat) = mat_config {
                if let Some(material_id) = MaterialId::from_u8(i as u8) {
                    config.set_material(material_id, mat.albedo_region, mat.uv_scale);
                }
            }
        }

        // Only enable if we have an albedo atlas
        if self.albedo_atlas.is_some() && self.ready {
            config.enabled = true;
        }

        config
    }
}

/// Creates a PBR terrain material using the texture atlas.
pub fn create_terrain_material(
    materials: &mut Assets<StandardMaterial>,
    atlas: &TerrainTextureAtlas,
) -> Handle<StandardMaterial> {
    let mut material = StandardMaterial {
        perceptual_roughness: 0.9,
        metallic: 0.0,
        reflectance: 0.3,
        ..Default::default()
    };

    // Set albedo texture if available
    if let Some(ref albedo) = atlas.albedo_atlas {
        material.base_color_texture = Some(albedo.clone());
    }

    // Set normal map if available
    if let Some(ref normals) = atlas.normal_atlas {
        material.normal_map_texture = Some(normals.clone());
    }

    materials.add(material)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_atlas_region_from_grid() {
        let region = AtlasRegion::from_grid(0, 0, 2, 4);
        assert_eq!(region.min, Vec2::new(0.0, 0.0));
        assert_eq!(region.max, Vec2::new(0.5, 0.25));

        let region = AtlasRegion::from_grid(1, 1, 2, 4);
        assert_eq!(region.min, Vec2::new(0.5, 0.25));
        assert_eq!(region.max, Vec2::new(1.0, 0.5));
    }

    #[test]
    fn test_uv_mapping() {
        let region = AtlasRegion::new(0.0, 0.0, 0.5, 0.25);

        // Top-left corner
        let mapped = region.map_uv(Vec2::new(0.0, 0.0));
        assert_eq!(mapped, Vec2::new(0.0, 0.0));

        // Bottom-right corner
        let mapped = region.map_uv(Vec2::new(1.0, 1.0));
        assert_eq!(mapped, Vec2::new(0.5, 0.25));

        // Center
        let mapped = region.map_uv(Vec2::new(0.5, 0.5));
        assert_eq!(mapped, Vec2::new(0.25, 0.125));
    }

    #[test]
    fn test_standard_layout() {
        let atlas = TerrainTextureAtlas::with_standard_layout();

        assert!(atlas.get_material(MaterialId::Dirt).is_some());
        assert!(atlas.get_material(MaterialId::Rock).is_some());
        assert!(atlas.get_material(MaterialId::Sand).is_some());
        assert!(atlas.get_material(MaterialId::Air).is_none());
    }
}
