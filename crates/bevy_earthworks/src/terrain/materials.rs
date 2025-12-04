//! Material definitions for voxels.

use bevy_color::Color;
use bevy_reflect::Reflect;
use serde::{Deserialize, Serialize};

/// Material identifier for voxels.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
#[repr(u8)]
pub enum MaterialId {
    /// Empty space / air.
    #[default]
    Air = 0,
    /// Standard dirt.
    Dirt = 1,
    /// Clay material.
    Clay = 2,
    /// Rock / stone.
    Rock = 3,
    /// Topsoil (nutrient-rich surface layer).
    Topsoil = 4,
    /// Gravel.
    Gravel = 5,
    /// Sand.
    Sand = 6,
    /// Water (for future use).
    Water = 7,
}

impl MaterialId {
    /// Returns the display name of this material.
    pub const fn name(&self) -> &'static str {
        match self {
            MaterialId::Air => "Air",
            MaterialId::Dirt => "Dirt",
            MaterialId::Clay => "Clay",
            MaterialId::Rock => "Rock",
            MaterialId::Topsoil => "Topsoil",
            MaterialId::Gravel => "Gravel",
            MaterialId::Sand => "Sand",
            MaterialId::Water => "Water",
        }
    }

    /// Returns the base color for this material.
    pub fn color(&self) -> Color {
        match self {
            MaterialId::Air => Color::NONE,
            // Rich brown dirt - earthy and warm
            MaterialId::Dirt => Color::srgb(0.6, 0.45, 0.3),
            // Orange-brown clay - distinct from dirt
            MaterialId::Clay => Color::srgb(0.75, 0.5, 0.35),
            // Gray rock with slight blue tint
            MaterialId::Rock => Color::srgb(0.45, 0.45, 0.5),
            // Dark rich topsoil with green tint (grass)
            MaterialId::Topsoil => Color::srgb(0.35, 0.45, 0.25),
            // Light gray gravel
            MaterialId::Gravel => Color::srgb(0.55, 0.55, 0.5),
            // Warm tan sand
            MaterialId::Sand => Color::srgb(0.9, 0.8, 0.55),
            // Blue water
            MaterialId::Water => Color::srgba(0.2, 0.5, 0.8, 0.8),
        }
    }

    /// Returns the density of this material in kg/m³.
    pub const fn density(&self) -> f32 {
        match self {
            MaterialId::Air => 1.2,
            MaterialId::Dirt => 1500.0,
            MaterialId::Clay => 1800.0,
            MaterialId::Rock => 2500.0,
            MaterialId::Topsoil => 1200.0,
            MaterialId::Gravel => 1800.0,
            MaterialId::Sand => 1600.0,
            MaterialId::Water => 1000.0,
        }
    }

    /// Returns whether this material is solid.
    pub const fn is_solid(&self) -> bool {
        !matches!(self, MaterialId::Air)
    }

    /// Converts from a u8 value.
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0 => Some(MaterialId::Air),
            1 => Some(MaterialId::Dirt),
            2 => Some(MaterialId::Clay),
            3 => Some(MaterialId::Rock),
            4 => Some(MaterialId::Topsoil),
            5 => Some(MaterialId::Gravel),
            6 => Some(MaterialId::Sand),
            7 => Some(MaterialId::Water),
            _ => None,
        }
    }
}

impl From<MaterialId> for u8 {
    fn from(m: MaterialId) -> Self {
        m as u8
    }
}
