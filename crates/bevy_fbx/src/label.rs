//! Labels that can be used to load part of an FBX asset

use bevy_asset::AssetPath;

/// Labels that can be used to load part of an FBX asset
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
    /// `AnimationStack{}`: FBX animation stack with multiple layers
    AnimationStack(usize),
    /// `Skeleton{}`: FBX skeleton for skeletal animation
    Skeleton(usize),
    /// `Node{}`: Individual FBX node in the scene hierarchy
    Node(usize),
    /// `Light{}`: FBX light definition
    Light(usize),
    /// `Camera{}`: FBX camera definition
    Camera(usize),
    /// `Texture{}`: FBX texture reference
    Texture(usize),
    /// `DefaultScene`: Main scene with all objects
    DefaultScene,
    /// `DefaultMaterial`: Fallback material used when no material is present
    DefaultMaterial,
    /// `RootNode`: Root node of the scene hierarchy
    RootNode,
}

impl core::fmt::Display for FbxAssetLabel {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            FbxAssetLabel::Scene(index) => f.write_str(&format!("Scene{index}")),
            FbxAssetLabel::Mesh(index) => f.write_str(&format!("Mesh{index}")),
            FbxAssetLabel::Material(index) => f.write_str(&format!("Material{index}")),
            FbxAssetLabel::Animation(index) => f.write_str(&format!("Animation{index}")),
            FbxAssetLabel::AnimationStack(index) => f.write_str(&format!("AnimationStack{index}")),
            FbxAssetLabel::Skeleton(index) => f.write_str(&format!("Skeleton{index}")),
            FbxAssetLabel::Node(index) => f.write_str(&format!("Node{index}")),
            FbxAssetLabel::Light(index) => f.write_str(&format!("Light{index}")),
            FbxAssetLabel::Camera(index) => f.write_str(&format!("Camera{index}")),
            FbxAssetLabel::Texture(index) => f.write_str(&format!("Texture{index}")),
            FbxAssetLabel::DefaultScene => f.write_str("DefaultScene"),
            FbxAssetLabel::DefaultMaterial => f.write_str("DefaultMaterial"),
            FbxAssetLabel::RootNode => f.write_str("RootNode"),
        }
    }
}

impl FbxAssetLabel {
    /// Add this label to an asset path
    pub fn from_asset(&self, path: impl Into<AssetPath<'static>>) -> AssetPath<'static> {
        path.into().with_label(self.to_string())
    }
}

