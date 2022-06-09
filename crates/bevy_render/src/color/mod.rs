mod colorspace;

pub use colorspace::*;

use crate::color::{HslRepresentation, SrgbColorSpace};
use bevy_math::{Vec3, Vec4};
use bevy_reflect::{FromReflect, Reflect, ReflectDeserialize};
use serde::{Deserialize, Serialize};
use std::ops::{Add, AddAssign, Mul, MulAssign};
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Reflect, FromReflect)]
#[reflect(PartialEq, Serialize, Deserialize)]
pub enum Color {
    /// sRGBA color
    Rgba {
        /// Red component. [0.0, 1.0]
        red: f32,
        /// Green component. [0.0, 1.0]
        green: f32,
        /// Blue component. [0.0, 1.0]
        blue: f32,
        /// Alpha component. [0.0, 1.0]
        alpha: f32,
    },
    /// RGBA color in the Linear sRGB colorspace (often colloquially referred to as "linear", "RGB", or "linear RGB").
    RgbaLinear {
        /// Red component. [0.0, 1.0]
        red: f32,
        /// Green component. [0.0, 1.0]
        green: f32,
        /// Blue component. [0.0, 1.0]
        blue: f32,
        /// Alpha component. [0.0, 1.0]
        alpha: f32,
    },
    /// HSL (hue, saturation, lightness) color with an alpha channel
    Hsla {
        /// Hue component. [0.0, 360.0]
        hue: f32,
        /// Saturation component. [0.0, 1.0]
        saturation: f32,
        /// Lightness component. [0.0, 1.0]
        lightness: f32,
        /// Alpha component. [0.0, 1.0]
        alpha: f32,
    },
}

// List of HTML colors sourced from https://www.w3schools.com/tags/ref_colornames.asp.
// Extended color keywords license: https://www.w3.org/Consortium/Legal/2015/copyright-software-and-document.
impl Color {
    /// <div style="background-color:rgb(94%, 97%, 100%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const ALICE_BLUE: Color = Color::rgb(0.94, 0.97, 1.0);
    /// <div style="background-color:rgb(98%, 92%, 84%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const ANTIQUE_WHITE: Color = Color::rgb(0.98, 0.92, 0.84);
    /// <div style="background-color:rgb(0%, 100%, 100%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const AQUA: Color = Color::rgb(0.0, 1.0, 1.0);
    /// <div style="background-color:rgb(49%, 100%, 83%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const AQUAMARINE: Color = Color::rgb(0.49, 1.0, 0.83);
    /// <div style="background-color:rgb(94%, 100%, 100%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const AZURE: Color = Color::rgb(0.94, 1.0, 1.0);
    /// <div style="background-color:rgb(96%, 96%, 86%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const BEIGE: Color = Color::rgb(0.96, 0.96, 0.86);
    /// <div style="background-color:rgb(100%, 89%, 77%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const BISQUE: Color = Color::rgb(1.0, 0.89, 0.77);
    /// <div style="background-color:rgb(0%, 0%, 0%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const BLACK: Color = Color::rgb(0.0, 0.0, 0.0);
    /// <div style="background-color:rgb(100%, 92%, 80%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const BLANCHED_ALMOND: Color = Color::rgb(1.0, 0.92, 0.8);
    /// <div style="background-color:rgb(0%, 0%, 100%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const BLUE: Color = Color::rgb(0.0, 0.0, 1.0);
    /// <div style="background-color:rgb(54%, 17%, 89%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const BLUE_VIOLET: Color = Color::rgb(0.54, 0.17, 0.89);
    /// <div style="background-color:rgb(65%, 16%, 16%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const BROWN: Color = Color::rgb(0.65, 0.16, 0.16);
    /// <div style="background-color:rgb(87%, 72%, 53%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const BURLY_WOOD: Color = Color::rgb(0.87, 0.72, 0.53);
    /// <div style="background-color:rgb(37%, 62%, 63%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const CADET_BLUE: Color = Color::rgb(0.37, 0.62, 0.63);
    /// <div style="background-color:rgb(50%, 100%, 0%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const CHARTREUSE: Color = Color::rgb(0.5, 1.0, 0.0);
    /// <div style="background-color:rgb(82%, 41%, 12%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const CHOCOLATE: Color = Color::rgb(0.82, 0.41, 0.12);
    /// <div style="background-color:rgb(100%, 50%, 31%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const CORAL: Color = Color::rgb(1.0, 0.5, 0.31);
    /// <div style="background-color:rgb(39%, 58%, 93%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const CORNFLOWER_BLUE: Color = Color::rgb(0.39, 0.58, 0.93);
    /// <div style="background-color:rgb(100%, 97%, 86%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const CORNSILK: Color = Color::rgb(1.0, 0.97, 0.86);
    /// <div style="background-color:rgb(86%, 8%, 24%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const CRIMSON: Color = Color::rgb(0.86, 0.08, 0.24);
    /// <div style="background-color:rgb(0%, 100%, 100%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const CYAN: Color = Color::rgb(0.0, 1.0, 1.0);
    /// <div style="background-color:rgb(0%, 0%, 55%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const DARK_BLUE: Color = Color::rgb(0.0, 0.0, 0.55);
    /// <div style="background-color:rgb(0%, 55%, 55%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const DARK_CYAN: Color = Color::rgb(0.0, 0.55, 0.55);
    /// <div style="background-color:rgb(72%, 53%, 4%); width: 10px; padding: 10px; border: 1px solid"></div>
    pub const DARK_GOLDEN_ROD: Color = Color::rgb(0.72, 0.53, 0.04);
    /// <div style="background-color:rgb(25%, 25%, 25%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const DARK_GRAY: Color = Color::rgb(0.25, 0.25, 0.25);
    /// <div style="background-color:rgb(0%, 50%, 0%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const DARK_GREEN: Color = Color::rgb(0.0, 0.5, 0.0);
    /// <div style="background-color:rgb(74%, 72%, 42%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const DARK_KHAKI: Color = Color::rgb(0.74, 0.72, 0.42);
    /// <div style="background-color:rgb(55%, 0%, 55%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const DARK_MAGENTA: Color = Color::rgb(0.55, 0.0, 0.55);
    /// <div style="background-color:rgb(33%, 42%, 18%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const DARK_OLIVE_GREEN: Color = Color::rgb(0.33, 0.42, 0.18);
    /// <div style="background-color:rgb(100%, 55%, 0%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const DARK_ORANGE: Color = Color::rgb(1.0, 0.55, 0.0);
    /// <div style="background-color:rgb(60%, 20%, 80%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const DARK_ORCHID: Color = Color::rgb(0.6, 0.2, 0.8);
    /// <div style="background-color:rgb(55%, 0%, 0%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const DARK_RED: Color = Color::rgb(0.55, 0.0, 0.0);
    /// <div style="background-color:rgb(91%, 59%, 48%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const DARK_SALMON: Color = Color::rgb(0.91, 0.59, 0.48);
    /// <div style="background-color:rgb(56%, 74%, 56%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const DARK_SEA_GREEN: Color = Color::rgb(0.56, 0.74, 0.56);
    /// <div style="background-color:rgb(28%, 24%, 55%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const DARK_SLATE_BLUE: Color = Color::rgb(0.28, 0.24, 0.55);
    /// <div style="background-color:rgb(17%, 31%, 31%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const DARK_SLATE_GRAY: Color = Color::rgb(0.18, 0.31, 0.31);
    /// <div style="background-color:rgb(0%, 81%, 82%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const DARK_TURQUOISE: Color = Color::rgb(0.0, 0.81, 0.82);
    /// <div style="background-color:rgb(58%, 0%, 83%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const DARK_VIOLET: Color = Color::rgb(0.58, 0.0, 0.83);
    /// <div style="background-color:rgb(100%, 8%, 58%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const DEEP_PINK: Color = Color::rgb(1.0, 0.08, 0.58);
    /// <div style="background-color:rgb(0%, 75%, 100%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const DEEP_SKY_BLUE: Color = Color::rgb(0.0, 0.75, 1.0);
    /// <div style="background-color:rgb(70%, 13%, 13%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const FIRE_BRICK: Color = Color::rgb(0.7, 0.13, 0.13);
    /// <div style="background-color:rgb(100%, 98%, 94%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const FLORAL_WHITE: Color = Color::rgb(1.0, 0.98, 0.94);
    /// <div style="background-color:rgb(13%, 55%, 13%); width: 10px; padding: 10px; border 1px solid;"></div>
    pub const FOREST_GREEN: Color = Color::rgb(0.13, 0.55, 0.13);
    /// <div style="background-color:rgb(100%, 0%, 100%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const FUCHSIA: Color = Color::rgb(1.0, 0.0, 1.0);
    /// <div style="background-color:rgb(86%, 86%, 86%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const GAINSBORO: Color = Color::rgb(0.86, 0.86, 0.86);
    /// <div style="background-color:rgb(97%, 97%, 100%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const CHOST_WHITE: Color = Color::rgb(0.97, 0.97, 1.0);
    /// <div style="background-color:rgb(100%, 84%, 0%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const GOLD: Color = Color::rgb(1.0, 0.84, 0.0);
    /// <div style="background-color:rgb(85%, 65%, 13%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const GOLDEN_ROD: Color = Color::rgb(0.85, 0.65, 0.13);
    /// <div style="background-color:rgb(50%, 50%, 50%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const GRAY: Color = Color::rgb(0.5, 0.5, 0.5);
    /// <div style="background-color:rgb(0%, 100%, 0%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const GREEN: Color = Color::rgb(0.0, 1.0, 0.0);
    /// <div style="background-color:rgb(68%, 100%, 18%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const GREEN_YELLOW: Color = Color::rgb(0.68, 1.0, 0.18);
    /// <div style="background-color:rgb(94%, 100%, 94%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const HONEY_DEW: Color = Color::rgb(0.94, 1.0, 0.94);
    /// <div style="background-color:rgb(100%, 41%, 71%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const HOT_PINK: Color = Color::rgb(1.0, 0.41, 0.71);
    /// <div style="background-color:rgb(80%, 36%, 36%); width: 10px; padding: 10px; border: 1px solid"></div>
    pub const INDIAN_RED: Color = Color::rgb(0.8, 0.36, 0.36);
    /// <div style="background-color:rgb(28%, 0%, 51%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const INDIGO: Color = Color::rgb(0.29, 0.0, 0.51);
    /// <div style="background-color:rgb(100%, 100%, 94%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const IVORY: Color = Color::rgb(1.0, 1.0, 0.94);
    /// <div style="background-color:rgb(94%, 90%, 55%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const KHAKI: Color = Color::rgb(0.94, 0.9, 0.55);
    /// <div style="background-color:rgb(90%, 90%, 98%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const LAVENDER: Color = Color::rgb(0.9, 0.9, 0.98);
    /// <div style="background-color:rgb(100%, 94%, 96%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const LAVENDER_BLUSH: Color = Color::rgb(1.0, 0.94, 0.96);
    /// <div style="background-color:rgb(49%, 99%, 0%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const LAWN_GREEN: Color = Color::rgb(0.49, 0.99, 0.0);
    /// <div style="background-color:rgb(100%, 98%, 80%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const LEMON_CHIFFON: Color = Color::rgb(1.0, 0.98, 0.8);
    /// <div style="background-color:rgb(68%, 85%, 90%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const LIGHT_BLUE: Color = Color::rgb(0.68, 0.85, 0.9);
    /// <div style="background-color:rgb(94%, 50%, 50%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const LIGHT_CORAL: Color = Color::rgb(0.94, 0.5, 0.5);
    /// <div style="background-color:rgb(88%, 100%, 100%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const LIGHT_CYAN: Color = Color::rgb(0.88, 1.0, 1.0);
    /// <div style="background-color:rgb(98%, 98%, 82%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const LIGHT_GOLDEN_ROD_YELLOW: Color = Color::rgb(0.98, 0.98, 0.82);
    /// <div style="background-color:rgb(83%, 83%, 83%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const LIGHT_GRAY: Color = Color::rgb(0.83, 0.83, 0.83);
    /// <div style="background-color:rgb(56%, 93%, 56%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const LIGHT_GREEN: Color = Color::rgb(0.56, 0.93, 0.56);
    /// <div style="background-color:rgb(100%, 71%, 76%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const LIGHT_PINK: Color = Color::rgb(1.0, 0.71, 1.0);
    /// <div style="background-color:rgb(100%, 63%, 48%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const LIGHT_SALMON: Color = Color::rgb(1.0, 0.63, 0.48);
    /// <div style="background-color:rgb(13%, 70%, 67%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const LIGHT_SEA_GREEN: Color = Color::rgb(0.13, 0.7, 0.67);
    /// <div style="background-color:rgb(53%, 81%, 98%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const LIGHT_SKY_BLUE: Color = Color::rgb(0.53, 0.81, 0.98);
    /// <div style="background-color:rgb(47%, 53%, 60%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const LIGHT_SLATE_GRAY: Color = Color::rgb(0.47, 0.53, 0.6);
    /// <div style="background-color:rgb(69%, 77%, 87%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const LIGHT_STEEL_BLUE: Color = Color::rgb(0.69, 0.77, 0.87);
    /// <div style="background-color:rgb(100%, 100%, 88%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const LIGHT_YELLOW: Color = Color::rgb(1.0, 1.0, 0.88);
    /// <div style="background-color:rgb(0%, 100%, 0%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const LIME: Color = Color::rgb(0.0, 1.0, 0.0);
    /// <div style="background-color:rgb(20%, 80%, 20%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const LIME_GREEN: Color = Color::rgb(0.2, 0.8, 0.2);
    /// <div style="background-color:rgb(98%, 94%, 90%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const LINEN: Color = Color::rgb(0.98, 0.94, 0.9);
    /// <div style="background-color:rgb(100%, 0%, 100%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const MAGENTA: Color = Color::rgb(1.0, 0.0, 1.0);
    /// <div style="background-color:rgb(50%, 0%, 0%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const MAROON: Color = Color::rgb(0.5, 0.0, 0.0);
    /// <div style="background-color:rgb(40%, 80%, 67%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const MEDIUM_AQUA_MARINE: Color = Color::rgb(0.4, 0.8, 0.67);
    /// <div style="background-color:rgb(0%, 0%, 80%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const MEDIUM_BLUE: Color = Color::rgb(0.0, 0.0, 0.8);
    /// <div style="background-color:rgb(73%, 33%, 83%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const MEDIUM_ORCHID: Color = Color::rgb(0.73, 0.33, 0.83);
    /// <div style="background-color:rgb(58%, 44%, 86%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const MEDIUM_PURPLE: Color = Color::rgb(0.58, 0.44, 0.86);
    /// <div style="background-color:rgb(24%, 70%, 44%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const MEDIUM_SEA_GREEN: Color = Color::rgb(0.24, 0.7, 0.44);
    /// <div style="background-color:rgb(48%, 41%, 93%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const MEDIUM_SLATE_BLUE: Color = Color::rgb(0.48, 0.41, 0.93);
    /// <div style="background-color:rgb(0%, 98%, 60%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const MEDIUM_SPRING_GREEN: Color = Color::rgb(0.0, 0.98, 0.6);
    /// <div style="background-color:rgb(28%, 82%, 80%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const MEDIUM_TURQUOISE: Color = Color::rgb(0.28, 0.82, 0.8);
    /// <div style="background-color:rgb(78%, 8%, 52%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const MEDIUM_VIOLET_RED: Color = Color::rgb(0.78, 0.08, 0.52);
    /// <div style="background-color:rgb(10%, 10%, 44%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const MIDNIGHT_BLUE: Color = Color::rgb(0.1, 0.1, 0.44);
    /// <div style="background-color:rgb(98%, 100%, 98%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const MINT_CREAM: Color = Color::rgb(0.96, 1.0, 0.98);
    /// <div style="background-color:rgb(100%, 89%, 88%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const MISTY_ROSE: Color = Color::rgb(1.0, 0.89, 0.88);
    /// <div style="background-color:rgb(100%, 89%, 71%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const MOCCASIN: Color = Color::rgb(1.0, 0.89, 0.71);
    /// <div style="background-color:rgb(100%, 87%, 68%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const NAVAJO_WHITE: Color = Color::rgb(1.0, 0.87, 0.68);
    /// <div style="background-color:rgb(0%, 0%, 50%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const NAVY: Color = Color::rgb(0.0, 0.0, 0.5);
    /// <div style="background-color:rgba(0%, 0%, 0%, 0%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const NONE: Color = Color::rgba(0.0, 0.0, 0.0, 0.0);
    /// <div style="background-color:rgb(99%, 96%, 90%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const OLD_LACE: Color = Color::rgb(0.99, 0.96, 0.9);
    /// <div style="background-color:rgb(50%, 50%, 0%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const OLIVE: Color = Color::rgb(0.5, 0.5, 0.0);
    /// <div style="background-color:rgb(42%, 56%, 14%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const OLIVE_DRAB: Color = Color::rgb(0.42, 0.56, 0.14);
    /// <div style="background-color:rgb(100%, 65%, 0%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const ORANGE: Color = Color::rgb(1.0, 0.65, 0.0);
    /// <div style="background-color:rgb(100%, 27%, 0%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const ORANGE_RED: Color = Color::rgb(1.0, 0.27, 0.0);
    /// <div style="background-color:rgb(85%, 44%, 84%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const ORCHID: Color = Color::rgb(0.85, 0.44, 0.84);
    /// <div style="background-color:rgb(93%, 91%, 67%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const PALE_GOLDEN_ROD: Color = Color::rgb(0.93, 0.91, 0.67);
    /// <div style="background-color:rgb(60%, 98%, 60%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const PALE_GREEN: Color = Color::rgb(0.6, 0.98, 0.6);
    /// <div style="background-color:rgb(69%, 93%, 93%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const PALE_TURQUOISE: Color = Color::rgb(0.69, 0.93, 0.93);
    /// <div style="background-color:rgb(86%, 44%, 58%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const PALE_VIOLET_RED: Color = Color::rgb(0.86, 0.44, 0.58);
    /// <div style="background-color:rgb(100%, 94%, 84%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const PAPAYA_WHIP: Color = Color::rgb(1.0, 0.94, 0.84);
    /// <div style="background-color:rgb(100%, 85%, 73%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const PEACH_PUFF: Color = Color::rgb(1.0, 0.85, 0.73);
    /// <div style="background-color:rgb(80%, 52%, 25%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const PERU: Color = Color::rgb(0.8, 0.52, 0.25);
    /// <div style="background-color:rgb(100%, 75%, 80%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const PINK: Color = Color::rgb(1.0, 0.75, 0.8);
    /// <div style="background-color:rgb(87%, 63%, 87%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const PLUM: Color = Color::rgb(0.87, 0.63, 0.87);
    /// <div style="background-color:rgb(69%, 88%, 90%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const POWDER_BLUE: Color = Color::rgb(0.69, 0.88, 0.9);
    /// <div style="background-color:rgb(50%, 0%, 50%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const PURPLE: Color = Color::rgb(0.5, 0.0, 0.5);
    /// <div style="background-color:rgb(40%, 20%, 60%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const REBECCA_PURPLE: Color = Color::rgb(0.4, 0.2, 0.6);
    /// <div style="background-color:rgb(100%, 0%, 0%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const RED: Color = Color::rgb(1.0, 0.0, 0.0);
    /// <div style="background-color:rgb(75%, 56%, 56%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const ROSY_BROWN: Color = Color::rgb(0.74, 0.56, 0.56);
    /// <div style="background-color:rgb(25%, 41%, 88%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const ROYAL_BLUE: Color = Color::rgb(0.25, 0.41, 0.88);
    /// <div style="background-color:rgb(55%, 27%, 7%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const SADDLE_BROWN: Color = Color::rgb(0.55, 0.27, 0.07);
    /// <div style="background-color:rgb(98%, 50%, 45%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const SALMON: Color = Color::rgb(0.98, 0.5, 0.45);
    /// <div style="background-color:rgb(96%, 64%, 38%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const SANDY_BROWN: Color = Color::rgb(0.96, 0.64, 0.38);
    /// <div style="background-color:rgb(18%, 55%, 34%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const SEA_GREEN: Color = Color::rgb(0.18, 0.55, 0.34);
    /// <div style="background-color:rgb(100%, 96%, 93%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const SEA_SHELL: Color = Color::rgb(1.0, 0.96, 0.93);
    /// <div style="background-color:rgb(63%, 32%, 18%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const SIENNA: Color = Color::rgb(0.63, 0.32, 0.18);
    /// <div style="background-color:rgb(75%, 75%, 75%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const SILVER: Color = Color::rgb(0.75, 0.75, 0.75);
    /// <div style="background-color:rgb(53%, 81%, 92%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const SKY_BLUE: Color = Color::rgb(0.53, 0.81, 0.92);
    /// <div style="background-color:rgb(42%, 35%, 80%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const SLATE_BLUE: Color = Color::rgb(0.42, 0.35, 0.8);
    /// <div style="background-color:rgb(44%, 50%, 56%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const SLATE_GRAY: Color = Color::rgb(0.44, 0.5, 0.56);
    /// <div style="background-color:rgb(100%, 98%, 98%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const SNOW: Color = Color::rgb(1.0, 0.98, 0.98);
    /// <div style="background-color:rgb(0%, 100%, 50%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const SPRING_GREEN: Color = Color::rgb(0.0, 1.0, 0.5);
    /// <div style="background-color:rgb(27%, 51%, 71%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const STEEL_BLUE: Color = Color::rgb(0.27, 0.51, 0.71);
    /// <div style="background-color:rgb(82%, 71%, 55%); width: 10px; padding: 10px; border: 1px; solid;"></div>
    pub const TAN: Color = Color::rgb(0.82, 0.71, 0.55);
    /// <div style="background-color:rgb(0%, 50%, 50%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const TEAL: Color = Color::rgb(0.0, 0.5, 0.5);
    /// <div style="background-color:rgb(85%, 75%, 85%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const THISTLE: Color = Color::rgb(0.85, 0.75, 0.85);
    /// <div style="background-color:rgb(100%, 39%, 28%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const TOMATO: Color = Color::rgb(1.0, 0.39, 0.28);
    /// <div style="background-color:rgb(25%, 88%, 82%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const TURQUOISE: Color = Color::rgb(0.25, 0.88, 0.82);
    /// <div style="background-color:rgb(93%, 51%, 93%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const VIOLET: Color = Color::rgb(0.93, 0.51, 0.93);
    /// <div style="background-color:rgb(96%, 87%, 70%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const WHEAT: Color = Color::rgb(0.96, 0.87, 0.7);
    /// <div style="background-color:rgb(100%, 100%, 100%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const WHITE: Color = Color::rgb(1.0, 1.0, 1.0);
    /// <div style="background-color:rgb(96%, 96%, 96%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const WHITE_SMOKE: Color = Color::rgb(0.96, 0.96, 0.96);
    /// <div style="background-color:rgb(100%, 100%, 0%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const YELLOW: Color = Color::rgb(1.0, 1.0, 0.0);
    /// <div style="background-color:rgb(60%, 80%, 20%); width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const YELLOW_GREEN: Color = Color::rgb(0.6, 0.8, 0.2);

    /// New `Color` from sRGB colorspace.
    pub const fn rgb(r: f32, g: f32, b: f32) -> Color {
        Color::Rgba {
            red: r,
            green: g,
            blue: b,
            alpha: 1.0,
        }
    }

    /// New `Color` from sRGB colorspace.
    pub const fn rgba(r: f32, g: f32, b: f32, a: f32) -> Color {
        Color::Rgba {
            red: r,
            green: g,
            blue: b,
            alpha: a,
        }
    }

    /// New `Color` from linear RGB colorspace.
    pub const fn rgb_linear(r: f32, g: f32, b: f32) -> Color {
        Color::RgbaLinear {
            red: r,
            green: g,
            blue: b,
            alpha: 1.0,
        }
    }

    /// New `Color` from linear RGB colorspace.
    pub const fn rgba_linear(r: f32, g: f32, b: f32, a: f32) -> Color {
        Color::RgbaLinear {
            red: r,
            green: g,
            blue: b,
            alpha: a,
        }
    }

    /// New `Color` with HSL representation in sRGB colorspace.
    pub const fn hsl(hue: f32, saturation: f32, lightness: f32) -> Color {
        Color::Hsla {
            hue,
            saturation,
            lightness,
            alpha: 1.0,
        }
    }

    /// New `Color` with HSL representation in sRGB colorspace.
    pub const fn hsla(hue: f32, saturation: f32, lightness: f32, alpha: f32) -> Color {
        Color::Hsla {
            hue,
            saturation,
            lightness,
            alpha,
        }
    }

    /// New `Color` from sRGB colorspace.
    pub fn hex<T: AsRef<str>>(hex: T) -> Result<Color, HexColorError> {
        let hex = hex.as_ref();

        // RGB
        if hex.len() == 3 {
            let mut data = [0; 6];
            for (i, ch) in hex.chars().enumerate() {
                data[i * 2] = ch as u8;
                data[i * 2 + 1] = ch as u8;
            }
            return decode_rgb(&data);
        }

        // RGBA
        if hex.len() == 4 {
            let mut data = [0; 8];
            for (i, ch) in hex.chars().enumerate() {
                data[i * 2] = ch as u8;
                data[i * 2 + 1] = ch as u8;
            }
            return decode_rgba(&data);
        }

        // RRGGBB
        if hex.len() == 6 {
            return decode_rgb(hex.as_bytes());
        }

        // RRGGBBAA
        if hex.len() == 8 {
            return decode_rgba(hex.as_bytes());
        }

        Err(HexColorError::Length)
    }

    /// New `Color` from sRGB colorspace.
    pub fn rgb_u8(r: u8, g: u8, b: u8) -> Color {
        Color::rgba_u8(r, g, b, u8::MAX)
    }

    // Float operations in const fn are not stable yet
    // see https://github.com/rust-lang/rust/issues/57241
    /// New `Color` from sRGB colorspace.
    pub fn rgba_u8(r: u8, g: u8, b: u8, a: u8) -> Color {
        Color::rgba(
            r as f32 / u8::MAX as f32,
            g as f32 / u8::MAX as f32,
            b as f32 / u8::MAX as f32,
            a as f32 / u8::MAX as f32,
        )
    }

    /// Get red in sRGB colorspace.
    pub fn r(&self) -> f32 {
        match self.as_rgba() {
            Color::Rgba { red, .. } => red,
            _ => unreachable!(),
        }
    }

    /// Get green in sRGB colorspace.
    pub fn g(&self) -> f32 {
        match self.as_rgba() {
            Color::Rgba { green, .. } => green,
            _ => unreachable!(),
        }
    }

    /// Get blue in sRGB colorspace.
    pub fn b(&self) -> f32 {
        match self.as_rgba() {
            Color::Rgba { blue, .. } => blue,
            _ => unreachable!(),
        }
    }

    /// Set red in sRGB colorspace.
    pub fn set_r(&mut self, r: f32) -> &mut Self {
        *self = self.as_rgba();
        match self {
            Color::Rgba { red, .. } => *red = r,
            _ => unreachable!(),
        }
        self
    }

    /// Set green in sRGB colorspace.
    pub fn set_g(&mut self, g: f32) -> &mut Self {
        *self = self.as_rgba();
        match self {
            Color::Rgba { green, .. } => *green = g,
            _ => unreachable!(),
        }
        self
    }

    /// Set blue in sRGB colorspace.
    pub fn set_b(&mut self, b: f32) -> &mut Self {
        *self = self.as_rgba();
        match self {
            Color::Rgba { blue, .. } => *blue = b,
            _ => unreachable!(),
        }
        self
    }

    /// Get alpha.
    pub fn a(&self) -> f32 {
        match self {
            Color::Rgba { alpha, .. }
            | Color::RgbaLinear { alpha, .. }
            | Color::Hsla { alpha, .. } => *alpha,
        }
    }

    /// Set alpha.
    pub fn set_a(&mut self, a: f32) -> &mut Self {
        match self {
            Color::Rgba { alpha, .. }
            | Color::RgbaLinear { alpha, .. }
            | Color::Hsla { alpha, .. } => {
                *alpha = a;
            }
        }
        self
    }

    /// Converts a `Color` to variant `Color::Rgba`
    pub fn as_rgba(self: &Color) -> Color {
        match self {
            Color::Rgba { .. } => *self,
            Color::RgbaLinear {
                red,
                green,
                blue,
                alpha,
            } => Color::Rgba {
                red: red.linear_to_nonlinear_srgb(),
                green: green.linear_to_nonlinear_srgb(),
                blue: blue.linear_to_nonlinear_srgb(),
                alpha: *alpha,
            },
            Color::Hsla {
                hue,
                saturation,
                lightness,
                alpha,
            } => {
                let [red, green, blue] =
                    HslRepresentation::hsl_to_nonlinear_srgb(*hue, *saturation, *lightness);
                Color::Rgba {
                    red,
                    green,
                    blue,
                    alpha: *alpha,
                }
            }
        }
    }

    /// Converts a `Color` to variant `Color::RgbaLinear`
    pub fn as_rgba_linear(self: &Color) -> Color {
        match self {
            Color::Rgba {
                red,
                green,
                blue,
                alpha,
            } => Color::RgbaLinear {
                red: red.nonlinear_to_linear_srgb(),
                green: green.nonlinear_to_linear_srgb(),
                blue: blue.nonlinear_to_linear_srgb(),
                alpha: *alpha,
            },
            Color::RgbaLinear { .. } => *self,
            Color::Hsla {
                hue,
                saturation,
                lightness,
                alpha,
            } => {
                let [red, green, blue] =
                    HslRepresentation::hsl_to_nonlinear_srgb(*hue, *saturation, *lightness);
                Color::RgbaLinear {
                    red: red.nonlinear_to_linear_srgb(),
                    green: green.nonlinear_to_linear_srgb(),
                    blue: blue.nonlinear_to_linear_srgb(),
                    alpha: *alpha,
                }
            }
        }
    }

    /// Converts a `Color` to variant `Color::Hsla`
    pub fn as_hsla(self: &Color) -> Color {
        match self {
            Color::Rgba {
                red,
                green,
                blue,
                alpha,
            } => {
                let (hue, saturation, lightness) =
                    HslRepresentation::nonlinear_srgb_to_hsl([*red, *green, *blue]);
                Color::Hsla {
                    hue,
                    saturation,
                    lightness,
                    alpha: *alpha,
                }
            }
            Color::RgbaLinear {
                red,
                green,
                blue,
                alpha,
            } => {
                let (hue, saturation, lightness) = HslRepresentation::nonlinear_srgb_to_hsl([
                    red.linear_to_nonlinear_srgb(),
                    green.linear_to_nonlinear_srgb(),
                    blue.linear_to_nonlinear_srgb(),
                ]);
                Color::Hsla {
                    hue,
                    saturation,
                    lightness,
                    alpha: *alpha,
                }
            }
            Color::Hsla { .. } => *self,
        }
    }

    /// Converts a `Color` to a `[f32; 4]` from sRGB colorspace
    pub fn as_rgba_f32(self: Color) -> [f32; 4] {
        match self {
            Color::Rgba {
                red,
                green,
                blue,
                alpha,
            } => [red, green, blue, alpha],
            Color::RgbaLinear {
                red,
                green,
                blue,
                alpha,
            } => [
                red.linear_to_nonlinear_srgb(),
                green.linear_to_nonlinear_srgb(),
                blue.linear_to_nonlinear_srgb(),
                alpha,
            ],
            Color::Hsla {
                hue,
                saturation,
                lightness,
                alpha,
            } => {
                let [red, green, blue] =
                    HslRepresentation::hsl_to_nonlinear_srgb(hue, saturation, lightness);
                [red, green, blue, alpha]
            }
        }
    }

    /// Converts a `Color` to a `[f32; 4]` from linear RGB colorspace
    #[inline]
    pub fn as_linear_rgba_f32(self: Color) -> [f32; 4] {
        match self {
            Color::Rgba {
                red,
                green,
                blue,
                alpha,
            } => [
                red.nonlinear_to_linear_srgb(),
                green.nonlinear_to_linear_srgb(),
                blue.nonlinear_to_linear_srgb(),
                alpha,
            ],
            Color::RgbaLinear {
                red,
                green,
                blue,
                alpha,
            } => [red, green, blue, alpha],
            Color::Hsla {
                hue,
                saturation,
                lightness,
                alpha,
            } => {
                let [red, green, blue] =
                    HslRepresentation::hsl_to_nonlinear_srgb(hue, saturation, lightness);
                [
                    red.nonlinear_to_linear_srgb(),
                    green.nonlinear_to_linear_srgb(),
                    blue.nonlinear_to_linear_srgb(),
                    alpha,
                ]
            }
        }
    }

    /// Converts a `Color` to a `[f32; 4]` from HSL colorspace
    pub fn as_hsla_f32(self: Color) -> [f32; 4] {
        match self {
            Color::Rgba {
                red,
                green,
                blue,
                alpha,
            } => {
                let (hue, saturation, lightness) =
                    HslRepresentation::nonlinear_srgb_to_hsl([red, green, blue]);
                [hue, saturation, lightness, alpha]
            }
            Color::RgbaLinear {
                red,
                green,
                blue,
                alpha,
            } => {
                let (hue, saturation, lightness) = HslRepresentation::nonlinear_srgb_to_hsl([
                    red.linear_to_nonlinear_srgb(),
                    green.linear_to_nonlinear_srgb(),
                    blue.linear_to_nonlinear_srgb(),
                ]);
                [hue, saturation, lightness, alpha]
            }
            Color::Hsla {
                hue,
                saturation,
                lightness,
                alpha,
            } => [hue, saturation, lightness, alpha],
        }
    }

    /// Converts Color to a u32 from sRGB colorspace.
    ///
    /// Maps the RGBA channels in RGBA order to a little-endian byte array (GPUs are little-endian).
    /// A will be the most significant byte and R the least significant.
    pub fn as_rgba_u32(self: Color) -> u32 {
        match self {
            Color::Rgba {
                red,
                green,
                blue,
                alpha,
            } => u32::from_le_bytes([
                (red * 255.0) as u8,
                (green * 255.0) as u8,
                (blue * 255.0) as u8,
                (alpha * 255.0) as u8,
            ]),
            Color::RgbaLinear {
                red,
                green,
                blue,
                alpha,
            } => u32::from_le_bytes([
                (red.linear_to_nonlinear_srgb() * 255.0) as u8,
                (green.linear_to_nonlinear_srgb() * 255.0) as u8,
                (blue.linear_to_nonlinear_srgb() * 255.0) as u8,
                (alpha * 255.0) as u8,
            ]),
            Color::Hsla {
                hue,
                saturation,
                lightness,
                alpha,
            } => {
                let [red, green, blue] =
                    HslRepresentation::hsl_to_nonlinear_srgb(hue, saturation, lightness);
                u32::from_le_bytes([
                    (red * 255.0) as u8,
                    (green * 255.0) as u8,
                    (blue * 255.0) as u8,
                    (alpha * 255.0) as u8,
                ])
            }
        }
    }

    /// Converts Color to a u32 from linear RGB colorspace.
    ///
    /// Maps the RGBA channels in RGBA order to a little-endian byte array (GPUs are little-endian).
    /// A will be the most significant byte and R the least significant.
    pub fn as_linear_rgba_u32(self: Color) -> u32 {
        match self {
            Color::Rgba {
                red,
                green,
                blue,
                alpha,
            } => u32::from_le_bytes([
                (red.nonlinear_to_linear_srgb() * 255.0) as u8,
                (green.nonlinear_to_linear_srgb() * 255.0) as u8,
                (blue.nonlinear_to_linear_srgb() * 255.0) as u8,
                (alpha * 255.0) as u8,
            ]),
            Color::RgbaLinear {
                red,
                green,
                blue,
                alpha,
            } => u32::from_le_bytes([
                (red * 255.0) as u8,
                (green * 255.0) as u8,
                (blue * 255.0) as u8,
                (alpha * 255.0) as u8,
            ]),
            Color::Hsla {
                hue,
                saturation,
                lightness,
                alpha,
            } => {
                let [red, green, blue] =
                    HslRepresentation::hsl_to_nonlinear_srgb(hue, saturation, lightness);
                u32::from_le_bytes([
                    (red.nonlinear_to_linear_srgb() * 255.0) as u8,
                    (green.nonlinear_to_linear_srgb() * 255.0) as u8,
                    (blue.nonlinear_to_linear_srgb() * 255.0) as u8,
                    (alpha * 255.0) as u8,
                ])
            }
        }
    }
}

impl Default for Color {
    fn default() -> Self {
        Color::WHITE
    }
}

impl AddAssign<Color> for Color {
    fn add_assign(&mut self, rhs: Color) {
        match self {
            Color::Rgba {
                red,
                green,
                blue,
                alpha,
            } => {
                let rhs = rhs.as_rgba_f32();
                *red += rhs[0];
                *green += rhs[1];
                *blue += rhs[2];
                *alpha += rhs[3];
            }
            Color::RgbaLinear {
                red,
                green,
                blue,
                alpha,
            } => {
                let rhs = rhs.as_linear_rgba_f32();
                *red += rhs[0];
                *green += rhs[1];
                *blue += rhs[2];
                *alpha += rhs[3];
            }
            Color::Hsla {
                hue,
                saturation,
                lightness,
                alpha,
            } => {
                let rhs = rhs.as_linear_rgba_f32();
                *hue += rhs[0];
                *saturation += rhs[1];
                *lightness += rhs[2];
                *alpha += rhs[3];
            }
        }
    }
}

impl Add<Color> for Color {
    type Output = Color;

    fn add(self, rhs: Color) -> Self::Output {
        match self {
            Color::Rgba {
                red,
                green,
                blue,
                alpha,
            } => {
                let rhs = rhs.as_rgba_f32();
                Color::Rgba {
                    red: red + rhs[0],
                    green: green + rhs[1],
                    blue: blue + rhs[2],
                    alpha: alpha + rhs[3],
                }
            }
            Color::RgbaLinear {
                red,
                green,
                blue,
                alpha,
            } => {
                let rhs = rhs.as_linear_rgba_f32();
                Color::RgbaLinear {
                    red: red + rhs[0],
                    green: green + rhs[1],
                    blue: blue + rhs[2],
                    alpha: alpha + rhs[3],
                }
            }
            Color::Hsla {
                hue,
                saturation,
                lightness,
                alpha,
            } => {
                let rhs = rhs.as_linear_rgba_f32();
                Color::Hsla {
                    hue: hue + rhs[0],
                    saturation: saturation + rhs[1],
                    lightness: lightness + rhs[2],
                    alpha: alpha + rhs[3],
                }
            }
        }
    }
}

impl AddAssign<Vec4> for Color {
    fn add_assign(&mut self, rhs: Vec4) {
        let rhs: Color = rhs.into();
        *self += rhs;
    }
}

impl Add<Vec4> for Color {
    type Output = Color;

    fn add(self, rhs: Vec4) -> Self::Output {
        let rhs: Color = rhs.into();
        self + rhs
    }
}

impl From<Color> for [f32; 4] {
    fn from(color: Color) -> Self {
        color.as_rgba_f32()
    }
}

impl From<[f32; 4]> for Color {
    fn from([r, g, b, a]: [f32; 4]) -> Self {
        Color::rgba(r, g, b, a)
    }
}

impl From<[f32; 3]> for Color {
    fn from([r, g, b]: [f32; 3]) -> Self {
        Color::rgb(r, g, b)
    }
}

impl From<Color> for Vec4 {
    fn from(color: Color) -> Self {
        let color: [f32; 4] = color.into();
        Vec4::new(color[0], color[1], color[2], color[3])
    }
}

impl From<Vec4> for Color {
    fn from(vec4: Vec4) -> Self {
        Color::rgba(vec4.x, vec4.y, vec4.z, vec4.w)
    }
}

impl From<Color> for wgpu::Color {
    fn from(color: Color) -> Self {
        if let Color::RgbaLinear {
            red,
            green,
            blue,
            alpha,
        } = color.as_rgba_linear()
        {
            wgpu::Color {
                r: red as f64,
                g: green as f64,
                b: blue as f64,
                a: alpha as f64,
            }
        } else {
            unreachable!()
        }
    }
}

impl Mul<f32> for Color {
    type Output = Color;

    fn mul(self, rhs: f32) -> Self::Output {
        match self {
            Color::Rgba {
                red,
                green,
                blue,
                alpha,
            } => Color::Rgba {
                red: red * rhs,
                green: green * rhs,
                blue: blue * rhs,
                alpha,
            },
            Color::RgbaLinear {
                red,
                green,
                blue,
                alpha,
            } => Color::RgbaLinear {
                red: red * rhs,
                green: green * rhs,
                blue: blue * rhs,
                alpha,
            },
            Color::Hsla {
                hue,
                saturation,
                lightness,
                alpha,
            } => Color::Hsla {
                hue: hue * rhs,
                saturation: saturation * rhs,
                lightness: lightness * rhs,
                alpha,
            },
        }
    }
}

impl MulAssign<f32> for Color {
    fn mul_assign(&mut self, rhs: f32) {
        match self {
            Color::Rgba {
                red, green, blue, ..
            }
            | Color::RgbaLinear {
                red, green, blue, ..
            } => {
                *red *= rhs;
                *green *= rhs;
                *blue *= rhs;
            }
            Color::Hsla {
                hue,
                saturation,
                lightness,
                ..
            } => {
                *hue *= rhs;
                *saturation *= rhs;
                *lightness *= rhs;
            }
        }
    }
}

impl Mul<Vec4> for Color {
    type Output = Color;

    fn mul(self, rhs: Vec4) -> Self::Output {
        match self {
            Color::Rgba {
                red,
                green,
                blue,
                alpha,
            } => Color::Rgba {
                red: red * rhs.x,
                green: green * rhs.y,
                blue: blue * rhs.z,
                alpha: alpha * rhs.w,
            },
            Color::RgbaLinear {
                red,
                green,
                blue,
                alpha,
            } => Color::RgbaLinear {
                red: red * rhs.x,
                green: green * rhs.y,
                blue: blue * rhs.z,
                alpha: alpha * rhs.w,
            },
            Color::Hsla {
                hue,
                saturation,
                lightness,
                alpha,
            } => Color::Hsla {
                hue: hue * rhs.x,
                saturation: saturation * rhs.y,
                lightness: lightness * rhs.z,
                alpha: alpha * rhs.w,
            },
        }
    }
}

impl MulAssign<Vec4> for Color {
    fn mul_assign(&mut self, rhs: Vec4) {
        match self {
            Color::Rgba {
                red,
                green,
                blue,
                alpha,
            }
            | Color::RgbaLinear {
                red,
                green,
                blue,
                alpha,
            } => {
                *red *= rhs.x;
                *green *= rhs.y;
                *blue *= rhs.z;
                *alpha *= rhs.w;
            }
            Color::Hsla {
                hue,
                saturation,
                lightness,
                alpha,
            } => {
                *hue *= rhs.x;
                *saturation *= rhs.y;
                *lightness *= rhs.z;
                *alpha *= rhs.w;
            }
        }
    }
}

impl Mul<Vec3> for Color {
    type Output = Color;

    fn mul(self, rhs: Vec3) -> Self::Output {
        match self {
            Color::Rgba {
                red,
                green,
                blue,
                alpha,
            } => Color::Rgba {
                red: red * rhs.x,
                green: green * rhs.y,
                blue: blue * rhs.z,
                alpha,
            },
            Color::RgbaLinear {
                red,
                green,
                blue,
                alpha,
            } => Color::RgbaLinear {
                red: red * rhs.x,
                green: green * rhs.y,
                blue: blue * rhs.z,
                alpha,
            },
            Color::Hsla {
                hue,
                saturation,
                lightness,
                alpha,
            } => Color::Hsla {
                hue: hue * rhs.x,
                saturation: saturation * rhs.y,
                lightness: lightness * rhs.z,
                alpha,
            },
        }
    }
}

impl MulAssign<Vec3> for Color {
    fn mul_assign(&mut self, rhs: Vec3) {
        match self {
            Color::Rgba {
                red, green, blue, ..
            }
            | Color::RgbaLinear {
                red, green, blue, ..
            } => {
                *red *= rhs.x;
                *green *= rhs.y;
                *blue *= rhs.z;
            }
            Color::Hsla {
                hue,
                saturation,
                lightness,
                ..
            } => {
                *hue *= rhs.x;
                *saturation *= rhs.y;
                *lightness *= rhs.z;
            }
        }
    }
}

impl Mul<[f32; 4]> for Color {
    type Output = Color;

    fn mul(self, rhs: [f32; 4]) -> Self::Output {
        match self {
            Color::Rgba {
                red,
                green,
                blue,
                alpha,
            } => Color::Rgba {
                red: red * rhs[0],
                green: green * rhs[1],
                blue: blue * rhs[2],
                alpha: alpha * rhs[3],
            },
            Color::RgbaLinear {
                red,
                green,
                blue,
                alpha,
            } => Color::RgbaLinear {
                red: red * rhs[0],
                green: green * rhs[1],
                blue: blue * rhs[2],
                alpha: alpha * rhs[3],
            },
            Color::Hsla {
                hue,
                saturation,
                lightness,
                alpha,
            } => Color::Hsla {
                hue: hue * rhs[0],
                saturation: saturation * rhs[1],
                lightness: lightness * rhs[2],
                alpha: alpha * rhs[3],
            },
        }
    }
}

impl MulAssign<[f32; 4]> for Color {
    fn mul_assign(&mut self, rhs: [f32; 4]) {
        match self {
            Color::Rgba {
                red,
                green,
                blue,
                alpha,
            }
            | Color::RgbaLinear {
                red,
                green,
                blue,
                alpha,
            } => {
                *red *= rhs[0];
                *green *= rhs[1];
                *blue *= rhs[2];
                *alpha *= rhs[3];
            }
            Color::Hsla {
                hue,
                saturation,
                lightness,
                alpha,
            } => {
                *hue *= rhs[0];
                *saturation *= rhs[1];
                *lightness *= rhs[2];
                *alpha *= rhs[3];
            }
        }
    }
}

impl Mul<[f32; 3]> for Color {
    type Output = Color;

    fn mul(self, rhs: [f32; 3]) -> Self::Output {
        match self {
            Color::Rgba {
                red,
                green,
                blue,
                alpha,
            } => Color::Rgba {
                red: red * rhs[0],
                green: green * rhs[1],
                blue: blue * rhs[2],
                alpha,
            },
            Color::RgbaLinear {
                red,
                green,
                blue,
                alpha,
            } => Color::RgbaLinear {
                red: red * rhs[0],
                green: green * rhs[1],
                blue: blue * rhs[2],
                alpha,
            },
            Color::Hsla {
                hue,
                saturation,
                lightness,
                alpha,
            } => Color::Hsla {
                hue: hue * rhs[0],
                saturation: saturation * rhs[1],
                lightness: lightness * rhs[2],
                alpha,
            },
        }
    }
}

impl MulAssign<[f32; 3]> for Color {
    fn mul_assign(&mut self, rhs: [f32; 3]) {
        match self {
            Color::Rgba {
                red, green, blue, ..
            }
            | Color::RgbaLinear {
                red, green, blue, ..
            } => {
                *red *= rhs[0];
                *green *= rhs[1];
                *blue *= rhs[2];
            }
            Color::Hsla {
                hue,
                saturation,
                lightness,
                ..
            } => {
                *hue *= rhs[0];
                *saturation *= rhs[1];
                *lightness *= rhs[2];
            }
        }
    }
}

#[derive(Debug, Error)]
pub enum HexColorError {
    #[error("Unexpected length of hex string")]
    Length,
    #[error("Error parsing hex value")]
    Hex(#[from] hex::FromHexError),
}

fn decode_rgb(data: &[u8]) -> Result<Color, HexColorError> {
    let mut buf = [0; 3];
    match hex::decode_to_slice(data, &mut buf) {
        Ok(_) => {
            let r = buf[0] as f32 / 255.0;
            let g = buf[1] as f32 / 255.0;
            let b = buf[2] as f32 / 255.0;
            Ok(Color::rgb(r, g, b))
        }
        Err(err) => Err(HexColorError::Hex(err)),
    }
}

fn decode_rgba(data: &[u8]) -> Result<Color, HexColorError> {
    let mut buf = [0; 4];
    match hex::decode_to_slice(data, &mut buf) {
        Ok(_) => {
            let r = buf[0] as f32 / 255.0;
            let g = buf[1] as f32 / 255.0;
            let b = buf[2] as f32 / 255.0;
            let a = buf[3] as f32 / 255.0;
            Ok(Color::rgba(r, g, b, a))
        }
        Err(err) => Err(HexColorError::Hex(err)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_color() {
        assert_eq!(Color::hex("FFF").unwrap(), Color::rgb(1.0, 1.0, 1.0));
        assert_eq!(Color::hex("000").unwrap(), Color::rgb(0.0, 0.0, 0.0));
        assert!(Color::hex("---").is_err());

        assert_eq!(Color::hex("FFFF").unwrap(), Color::rgba(1.0, 1.0, 1.0, 1.0));
        assert_eq!(Color::hex("0000").unwrap(), Color::rgba(0.0, 0.0, 0.0, 0.0));
        assert!(Color::hex("----").is_err());

        assert_eq!(Color::hex("FFFFFF").unwrap(), Color::rgb(1.0, 1.0, 1.0));
        assert_eq!(Color::hex("000000").unwrap(), Color::rgb(0.0, 0.0, 0.0));
        assert!(Color::hex("------").is_err());

        assert_eq!(
            Color::hex("FFFFFFFF").unwrap(),
            Color::rgba(1.0, 1.0, 1.0, 1.0)
        );
        assert_eq!(
            Color::hex("00000000").unwrap(),
            Color::rgba(0.0, 0.0, 0.0, 0.0)
        );
        assert!(Color::hex("--------").is_err());

        assert!(Color::hex("1234567890").is_err());
    }

    #[test]
    fn conversions_vec4() {
        let starting_vec4 = Vec4::new(0.4, 0.5, 0.6, 1.0);
        let starting_color = Color::from(starting_vec4);

        assert_eq!(starting_vec4, Vec4::from(starting_color),);

        let transformation = Vec4::new(0.5, 0.5, 0.5, 1.0);

        assert_eq!(
            starting_color * transformation,
            Color::from(starting_vec4 * transformation),
        );
    }

    #[test]
    fn mul_and_mulassign_f32() {
        let transformation = 0.5;
        let starting_color = Color::rgba(0.4, 0.5, 0.6, 1.0);

        assert_eq!(
            starting_color * transformation,
            Color::rgba(0.4 * 0.5, 0.5 * 0.5, 0.6 * 0.5, 1.0),
        );

        let mut mutated_color = starting_color;
        mutated_color *= transformation;

        assert_eq!(starting_color * transformation, mutated_color,);
    }

    #[test]
    fn mul_and_mulassign_f32by3() {
        let transformation = [0.4, 0.5, 0.6];
        let starting_color = Color::rgba(0.4, 0.5, 0.6, 1.0);

        assert_eq!(
            starting_color * transformation,
            Color::rgba(0.4 * 0.4, 0.5 * 0.5, 0.6 * 0.6, 1.0),
        );

        let mut mutated_color = starting_color;
        mutated_color *= transformation;

        assert_eq!(starting_color * transformation, mutated_color,);
    }

    #[test]
    fn mul_and_mulassign_f32by4() {
        let transformation = [0.4, 0.5, 0.6, 0.9];
        let starting_color = Color::rgba(0.4, 0.5, 0.6, 1.0);

        assert_eq!(
            starting_color * transformation,
            Color::rgba(0.4 * 0.4, 0.5 * 0.5, 0.6 * 0.6, 1.0 * 0.9),
        );

        let mut mutated_color = starting_color;
        mutated_color *= transformation;

        assert_eq!(starting_color * transformation, mutated_color,);
    }

    #[test]
    fn mul_and_mulassign_vec3() {
        let transformation = Vec3::new(0.2, 0.3, 0.4);
        let starting_color = Color::rgba(0.4, 0.5, 0.6, 1.0);

        assert_eq!(
            starting_color * transformation,
            Color::rgba(0.4 * 0.2, 0.5 * 0.3, 0.6 * 0.4, 1.0),
        );

        let mut mutated_color = starting_color;
        mutated_color *= transformation;

        assert_eq!(starting_color * transformation, mutated_color,);
    }

    #[test]
    fn mul_and_mulassign_vec4() {
        let transformation = Vec4::new(0.2, 0.3, 0.4, 0.5);
        let starting_color = Color::rgba(0.4, 0.5, 0.6, 1.0);

        assert_eq!(
            starting_color * transformation,
            Color::rgba(0.4 * 0.2, 0.5 * 0.3, 0.6 * 0.4, 1.0 * 0.5),
        );

        let mut mutated_color = starting_color;
        mutated_color *= transformation;

        assert_eq!(starting_color * transformation, mutated_color,);
    }
}
