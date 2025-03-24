//! This module contains the basic building blocks of Bevy's UI

mod button;
mod image;
mod label;
mod text;

use crate::Node;

use bevy_color::{Color, Srgba};
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::{bundle::Bundle, component::Component};
use bevy_math::Vec2;
use bevy_reflect::Reflect;

pub use button::*;
pub use image::*;
pub use label::*;
pub use text::*;

pub enum Style {
    Color(ColorStyle),
    Border(BorderStyle),
}

/// A full color palette in use across
#[derive(Component)]
pub struct ColorStyle {
    pub background: Color,
    pub text: Color,
    pub primary: Color,
    pub secondary: Color,
    pub success: Color,
    pub failure: Color,
    pub note: Color,
}

impl Default for ColorStyle {
    fn default() -> Self {
        // Solarized Light, stolen from iced-rs :p
        Self {
            background: Srgba::rgb_u8(0x00, 0x2b, 0x36).into(), // base03
            text: Srgba::rgb_u8(0x65, 0x7b, 0x83).into(),       // base00
            primary: Srgba::rgb_u8(0x2a, 0xa1, 0x98).into(),    // cyan
            secondary: Srgba::rgb_u8(0x2a, 0xa1, 0x98).into(),  // cyan
            success: Srgba::rgb_u8(0x85, 0x99, 0x00).into(),    // green
            failure: Srgba::rgb_u8(0xb5, 0x89, 0x00).into(),    // yellow
            note: Srgba::rgb_u8(0xdc, 0x32, 0x2f).into(),       // red
        }
    }
}

#[derive(Component, Default)]
pub struct BorderStyle {
    pub color: Color,
    pub size: Vec2,
    pub geom: BorderGeometry,
}

#[derive(Default)]
pub enum BorderGeometry {
    Dotted,
    Dashed,
    Solid,
    Double,
    Groove,
    Ridge,
    Outset,
    #[default]
    None,
    Hidden,
}

/// A stack of style changes that can be pushed into or "rolled back" to previous ones
#[derive(Component, Default, Reflect)]
pub struct Rollback<T>(Vec<T>);

impl<T> Rollback<T> {
    pub fn push(&mut self, t: T) {
        self.0.push(t);
    }

    pub fn roll_back(&mut self) -> Option<T> {
        self.0.pop()
    }
}

#[derive(Bundle, Deref, DerefMut)]
pub struct WidgetBundle {
    #[deref]
    pub node: Node,
    pub color: ColorStyle,
    pub border: BorderStyle,
}

/*

Node styling
- CSS layout parameters
- Theme/ palette
- Local style overrides

Interactions
- Focus
- Hover
- Click
- Touch (?)
- Drag
- DragInternal (?)
- Keyboard



*/
