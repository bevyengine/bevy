//! Shared ray-reconstruction guide textures used by DLSS-RR and MetalFX Temporal Denoised Scaler.

use bevy_ecs::component::Component;
use bevy_render::texture::CachedTexture;

/// Marker inserted by DLSS-RR / MetalFX prepare when a camera needs Solari guide buffers.
#[derive(Component, Clone, Copy, Default)]
pub struct RayReconstructionDenoiser;

/// Guide buffers produced by Solari (or similar) for neural ray-reconstruction denoisers.
#[derive(Component)]
pub struct ViewRayReconstructionGuideTextures {
    pub diffuse_albedo: CachedTexture,
    pub specular_albedo: CachedTexture,
    /// World/view-space normals in RGB; perceptual roughness is packed in A for DLSS.
    pub normal_roughness: CachedTexture,
    /// Hardware depth copied for DLSS, with virtual depth written for replaced mirrors.
    ///
    /// MetalFX samples the depth prepass instead: this copy is `R32Float`, not a depth format.
    pub depth: CachedTexture,
    /// Specular / reflection motion vectors (DLSS-RR). MetalFX uses the prepass motion vectors.
    pub specular_motion_vectors: CachedTexture,
    /// Perceptual roughness in R, for MetalFX. DLSS reads roughness from `normal_roughness.a`.
    pub roughness: CachedTexture,
}

/// Backward-compatible alias for DLSS integration sites.
pub type ViewDlssRayReconstructionTextures = ViewRayReconstructionGuideTextures;
