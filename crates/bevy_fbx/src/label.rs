//! Labels that can be used to load part of an FBX asset

use bevy_asset::AssetPath;

/// Labels that can be used to load part of an FBX
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FbxAssetLabel {
    /// `Scene{}`: FBX Scene as a Bevy [`Scene`](bevy_scene::Scene)
    Scene(usize),
    /// `Mesh{}`: FBX Mesh as a Bevy [`Mesh`](bevy_mesh::Mesh)
    Mesh(usize),
    /// `Material{}`: FBX material as a Bevy [`StandardMaterial`](bevy_pbr::StandardMaterial)
    Material(usize),
    /// `Animation{}`: FBX animation as a Bevy [`AnimationClip`](bevy_animation::AnimationClip)
    Animation(usize),
    /// `Skeleton{}`: FBX skeleton as a Bevy [`Skeleton`](crate::Skeleton)
    Skeleton(usize),
    /// `DefaultMaterial`: fallback material used when no material is present
    DefaultMaterial,
}

impl core::fmt::Display for FbxAssetLabel {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            FbxAssetLabel::Scene(index) => f.write_str(&format!("Scene{index}")),
            FbxAssetLabel::Mesh(index) => f.write_str(&format!("Mesh{index}")),
            FbxAssetLabel::Material(index) => f.write_str(&format!("Material{index}")),
            FbxAssetLabel::Animation(index) => f.write_str(&format!("Animation{index}")),
            FbxAssetLabel::Skeleton(index) => f.write_str(&format!("Skeleton{index}")),
            FbxAssetLabel::DefaultMaterial => f.write_str("DefaultMaterial"),
        }
    }
}

impl FbxAssetLabel {
    /// Add this label to an asset path
    pub fn from_asset(&self, path: impl Into<AssetPath<'static>>) -> AssetPath<'static> {
        path.into().with_label(self.to_string())
    }
}

