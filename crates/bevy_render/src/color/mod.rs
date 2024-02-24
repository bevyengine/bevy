mod colorspace;

pub use colorspace::*;

use bevy_math::{Vec3, Vec4};
use bevy_reflect::{Reflect, ReflectDeserialize, ReflectSerialize};
use serde::{Deserialize, Serialize};
use std::ops::{Add, Mul, MulAssign};
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Reflect)]
#[reflect(PartialEq, Serialize, Deserialize)]
pub enum Color {
    /// sRGBA color
    Rgba {
        /// Red channel. [0.0, 1.0]
        red: f32,
        /// Green channel. [0.0, 1.0]
        green: f32,
        /// Blue channel. [0.0, 1.0]
        blue: f32,
        /// Alpha channel. [0.0, 1.0]
        alpha: f32,
    },
    /// RGBA color in the Linear sRGB colorspace (often colloquially referred to as "linear", "RGB", or "linear RGB").
    RgbaLinear {
        /// Red channel. [0.0, 1.0]
        red: f32,
        /// Green channel. [0.0, 1.0]
        green: f32,
        /// Blue channel. [0.0, 1.0]
        blue: f32,
        /// Alpha channel. [0.0, 1.0]
        alpha: f32,
    },
    /// HSL (hue, saturation, lightness) color with an alpha channel
    Hsla {
        /// Hue channel. [0.0, 360.0]
        hue: f32,
        /// Saturation channel. [0.0, 1.0]
        saturation: f32,
        /// Lightness channel. [0.0, 1.0]
        lightness: f32,
        /// Alpha channel. [0.0, 1.0]
        alpha: f32,
    },
    /// LCH(ab) (lightness, chroma, hue) color with an alpha channel
    Lcha {
        /// Lightness channel. [0.0, 1.5]
        lightness: f32,
        /// Chroma channel. [0.0, 1.5]
        chroma: f32,
        /// Hue channel. [0.0, 360.0]
        hue: f32,
        /// Alpha channel. [0.0, 1.0]
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
    #[doc(alias = "transparent")]
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
    ///
    /// # Arguments
    ///
    /// * `r` - Red channel. [0.0, 1.0]
    /// * `g` - Green channel. [0.0, 1.0]
    /// * `b` - Blue channel. [0.0, 1.0]
    ///
    /// See also [`Color::rgba`], [`Color::rgb_u8`], [`Color::hex`].
    ///
    pub const fn rgb(r: f32, g: f32, b: f32) -> Color {
        Color::Rgba {
            red: r,
            green: g,
            blue: b,
            alpha: 1.0,
        }
    }

    /// New `Color` from sRGB colorspace.
    ///
    /// # Arguments
    ///
    /// * `r` - Red channel. [0.0, 1.0]
    /// * `g` - Green channel. [0.0, 1.0]
    /// * `b` - Blue channel. [0.0, 1.0]
    /// * `a` - Alpha channel. [0.0, 1.0]
    ///
    /// See also [`Color::rgb`], [`Color::rgba_u8`], [`Color::hex`].
    ///
    pub const fn rgba(r: f32, g: f32, b: f32, a: f32) -> Color {
        Color::Rgba {
            red: r,
            green: g,
            blue: b,
            alpha: a,
        }
    }

    /// New `Color` from linear RGB colorspace.
    ///
    /// # Arguments
    ///
    /// * `r` - Red channel. [0.0, 1.0]
    /// * `g` - Green channel. [0.0, 1.0]
    /// * `b` - Blue channel. [0.0, 1.0]
    ///
    /// See also [`Color::rgb`], [`Color::rgba_linear`].
    ///
    pub const fn rgb_linear(r: f32, g: f32, b: f32) -> Color {
        Color::RgbaLinear {
            red: r,
            green: g,
            blue: b,
            alpha: 1.0,
        }
    }

    /// New `Color` from linear RGB colorspace.
    ///
    /// # Arguments
    ///
    /// * `r` - Red channel. [0.0, 1.0]
    /// * `g` - Green channel. [0.0, 1.0]
    /// * `b` - Blue channel. [0.0, 1.0]
    /// * `a` - Alpha channel. [0.0, 1.0]
    ///
    /// See also [`Color::rgba`], [`Color::rgb_linear`].
    ///
    pub const fn rgba_linear(r: f32, g: f32, b: f32, a: f32) -> Color {
        Color::RgbaLinear {
            red: r,
            green: g,
            blue: b,
            alpha: a,
        }
    }

    /// New `Color` with HSL representation in sRGB colorspace.
    ///
    /// # Arguments
    ///
    /// * `hue` - Hue channel. [0.0, 360.0]
    /// * `saturation` - Saturation channel. [0.0, 1.0]
    /// * `lightness` - Lightness channel. [0.0, 1.0]
    ///
    /// See also [`Color::hsla`].
    ///
    pub const fn hsl(hue: f32, saturation: f32, lightness: f32) -> Color {
        Color::Hsla {
            hue,
            saturation,
            lightness,
            alpha: 1.0,
        }
    }

    /// New `Color` with HSL representation in sRGB colorspace.
    ///
    /// # Arguments
    ///
    /// * `hue` - Hue channel. [0.0, 360.0]
    /// * `saturation` - Saturation channel. [0.0, 1.0]
    /// * `lightness` - Lightness channel. [0.0, 1.0]
    /// * `alpha` - Alpha channel. [0.0, 1.0]
    ///
    /// See also [`Color::hsl`].
    ///
    pub const fn hsla(hue: f32, saturation: f32, lightness: f32, alpha: f32) -> Color {
        Color::Hsla {
            hue,
            saturation,
            lightness,
            alpha,
        }
    }

    /// New `Color` with LCH representation in sRGB colorspace.
    ///
    /// # Arguments
    ///
    /// * `lightness` - Lightness channel. [0.0, 1.5]
    /// * `chroma` - Chroma channel. [0.0, 1.5]
    /// * `hue` - Hue channel. [0.0, 360.0]
    ///
    /// See also [`Color::lcha`].
    pub const fn lch(lightness: f32, chroma: f32, hue: f32) -> Color {
        Color::Lcha {
            lightness,
            chroma,
            hue,
            alpha: 1.0,
        }
    }

    /// New `Color` with LCH representation in sRGB colorspace.
    ///
    /// # Arguments
    ///
    /// * `lightness` - Lightness channel. [0.0, 1.5]
    /// * `chroma` - Chroma channel. [0.0, 1.5]
    /// * `hue` - Hue channel. [0.0, 360.0]
    /// * `alpha` - Alpha channel. [0.0, 1.0]
    ///
    /// See also [`Color::lch`].
    pub const fn lcha(lightness: f32, chroma: f32, hue: f32, alpha: f32) -> Color {
        Color::Lcha {
            lightness,
            chroma,
            hue,
            alpha,
        }
    }

    /// New `Color` from sRGB colorspace.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bevy_render::color::Color;
    /// let color = Color::hex("FF00FF").unwrap(); // fuchsia
    /// let color = Color::hex("FF00FF7F").unwrap(); // partially transparent fuchsia
    ///
    /// // A standard hex color notation is also available
    /// assert_eq!(Color::hex("#FFFFFF").unwrap(), Color::rgb(1.0, 1.0, 1.0));
    /// ```
    ///
    pub fn hex<T: AsRef<str>>(hex: T) -> Result<Color, HexColorError> {
        let hex = hex.as_ref();
        let hex = hex.strip_prefix('#').unwrap_or(hex);

        match *hex.as_bytes() {
            // RGB
            [r, g, b] => {
                let [r, g, b, ..] = decode_hex([r, r, g, g, b, b])?;
                Ok(Color::rgb_u8(r, g, b))
            }
            // RGBA
            [r, g, b, a] => {
                let [r, g, b, a, ..] = decode_hex([r, r, g, g, b, b, a, a])?;
                Ok(Color::rgba_u8(r, g, b, a))
            }
            // RRGGBB
            [r1, r2, g1, g2, b1, b2] => {
                let [r, g, b, ..] = decode_hex([r1, r2, g1, g2, b1, b2])?;
                Ok(Color::rgb_u8(r, g, b))
            }
            // RRGGBBAA
            [r1, r2, g1, g2, b1, b2, a1, a2] => {
                let [r, g, b, a, ..] = decode_hex([r1, r2, g1, g2, b1, b2, a1, a2])?;
                Ok(Color::rgba_u8(r, g, b, a))
            }
            _ => Err(HexColorError::Length),
        }
    }

    /// New `Color` from sRGB colorspace.
    ///
    /// # Arguments
    ///
    /// * `r` - Red channel. [0, 255]
    /// * `g` - Green channel. [0, 255]
    /// * `b` - Blue channel. [0, 255]
    ///
    /// See also [`Color::rgb`], [`Color::rgba_u8`], [`Color::hex`].
    ///
    pub fn rgb_u8(r: u8, g: u8, b: u8) -> Color {
        Color::rgba_u8(r, g, b, u8::MAX)
    }

    // Float operations in const fn are not stable yet
    // see https://github.com/rust-lang/rust/issues/57241
    /// New `Color` from sRGB colorspace.
    ///
    /// # Arguments
    ///
    /// * `r` - Red channel. [0, 255]
    /// * `g` - Green channel. [0, 255]
    /// * `b` - Blue channel. [0, 255]
    /// * `a` - Alpha channel. [0, 255]
    ///
    /// See also [`Color::rgba`], [`Color::rgb_u8`], [`Color::hex`].
    ///
    pub fn rgba_u8(r: u8, g: u8, b: u8, a: u8) -> Color {
        Color::rgba(
            r as f32 / u8::MAX as f32,
            g as f32 / u8::MAX as f32,
            b as f32 / u8::MAX as f32,
            a as f32 / u8::MAX as f32,
        )
    }

    /// Converts a Color to variant [`Color::Rgba`] and return red in sRGB colorspace
    pub fn r(&self) -> f32 {
        match self.as_rgba() {
            Color::Rgba { red, .. } => red,
            _ => unreachable!(),
        }
    }

    /// Converts a Color to variant [`Color::Rgba`] and return green in sRGB colorspace
    pub fn g(&self) -> f32 {
        match self.as_rgba() {
            Color::Rgba { green, .. } => green,
            _ => unreachable!(),
        }
    }

    /// Converts a Color to variant [`Color::Rgba`] and return blue in sRGB colorspace
    pub fn b(&self) -> f32 {
        match self.as_rgba() {
            Color::Rgba { blue, .. } => blue,
            _ => unreachable!(),
        }
    }

    /// Converts a Color to variant [`Color::Rgba`] and set red
    pub fn set_r(&mut self, r: f32) -> &mut Self {
        *self = self.as_rgba();
        match self {
            Color::Rgba { red, .. } => *red = r,
            _ => unreachable!(),
        }
        self
    }

    /// Converts a Color to variant [`Color::Rgba`] and return this color with red set to a new value
    #[must_use]
    pub fn with_r(mut self, r: f32) -> Self {
        self.set_r(r);
        self
    }

    /// Converts a Color to variant [`Color::Rgba`] and set green
    pub fn set_g(&mut self, g: f32) -> &mut Self {
        *self = self.as_rgba();
        match self {
            Color::Rgba { green, .. } => *green = g,
            _ => unreachable!(),
        }
        self
    }

    /// Converts a Color to variant [`Color::Rgba`] and return this color with green set to a new value
    #[must_use]
    pub fn with_g(mut self, g: f32) -> Self {
        self.set_g(g);
        self
    }

    /// Converts a Color to variant [`Color::Rgba`] and set blue
    pub fn set_b(&mut self, b: f32) -> &mut Self {
        *self = self.as_rgba();
        match self {
            Color::Rgba { blue, .. } => *blue = b,
            _ => unreachable!(),
        }
        self
    }

    /// Converts a Color to variant [`Color::Rgba`] and return this color with blue set to a new value
    #[must_use]
    pub fn with_b(mut self, b: f32) -> Self {
        self.set_b(b);
        self
    }

    /// Converts a Color to variant [`Color::Hsla`] and return hue
    pub fn h(&self) -> f32 {
        match self.as_hsla() {
            Color::Hsla { hue, .. } => hue,
            _ => unreachable!(),
        }
    }

    /// Converts a Color to variant [`Color::Hsla`] and return saturation
    pub fn s(&self) -> f32 {
        match self.as_hsla() {
            Color::Hsla { saturation, .. } => saturation,
            _ => unreachable!(),
        }
    }

    /// Converts a Color to variant [`Color::Hsla`] and return lightness
    pub fn l(&self) -> f32 {
        match self.as_hsla() {
            Color::Hsla { lightness, .. } => lightness,
            _ => unreachable!(),
        }
    }

    /// Converts a Color to variant [`Color::Hsla`] and set hue
    pub fn set_h(&mut self, h: f32) -> &mut Self {
        *self = self.as_hsla();
        match self {
            Color::Hsla { hue, .. } => *hue = h,
            _ => unreachable!(),
        }
        self
    }

    /// Converts a Color to variant [`Color::Hsla`] and return this color with hue set to a new value
    #[must_use]
    pub fn with_h(mut self, h: f32) -> Self {
        self.set_h(h);
        self
    }

    /// Converts a Color to variant [`Color::Hsla`] and set saturation
    pub fn set_s(&mut self, s: f32) -> &mut Self {
        *self = self.as_hsla();
        match self {
            Color::Hsla { saturation, .. } => *saturation = s,
            _ => unreachable!(),
        }
        self
    }

    /// Converts a Color to variant [`Color::Hsla`] and return this color with saturation set to a new value
    #[must_use]
    pub fn with_s(mut self, s: f32) -> Self {
        self.set_s(s);
        self
    }

    /// Converts a Color to variant [`Color::Hsla`] and set lightness
    pub fn set_l(&mut self, l: f32) -> &mut Self {
        *self = self.as_hsla();
        match self {
            Color::Hsla { lightness, .. } => *lightness = l,
            _ => unreachable!(),
        }
        self
    }

    /// Converts a Color to variant [`Color::Hsla`] and return this color with lightness set to a new value
    #[must_use]
    pub fn with_l(mut self, l: f32) -> Self {
        self.set_l(l);
        self
    }

    /// Get alpha.
    #[inline(always)]
    pub fn a(&self) -> f32 {
        match self {
            Color::Rgba { alpha, .. }
            | Color::RgbaLinear { alpha, .. }
            | Color::Hsla { alpha, .. }
            | Color::Lcha { alpha, .. } => *alpha,
        }
    }

    /// Set alpha.
    pub fn set_a(&mut self, a: f32) -> &mut Self {
        match self {
            Color::Rgba { alpha, .. }
            | Color::RgbaLinear { alpha, .. }
            | Color::Hsla { alpha, .. }
            | Color::Lcha { alpha, .. } => {
                *alpha = a;
            }
        }
        self
    }

    /// Returns this color with a new alpha value.
    #[must_use]
    pub fn with_a(mut self, a: f32) -> Self {
        self.set_a(a);
        self
    }

    /// Determine if the color is fully transparent, i.e. if the alpha is 0.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bevy_render::color::Color;
    /// // Fully transparent colors
    /// assert!(Color::NONE.is_fully_transparent());
    /// assert!(Color::rgba(1.0, 0.5, 0.5, 0.0).is_fully_transparent());
    ///
    /// // (Partially) opaque colors
    /// assert!(!Color::BLACK.is_fully_transparent());
    /// assert!(!Color::rgba(1.0, 0.5, 0.5, 0.2).is_fully_transparent());
    /// ```
    #[inline(always)]
    pub fn is_fully_transparent(&self) -> bool {
        self.a() == 0.0
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
            Color::Lcha {
                lightness,
                chroma,
                hue,
                alpha,
            } => {
                let [red, green, blue] =
                    LchRepresentation::lch_to_nonlinear_srgb(*lightness, *chroma, *hue);

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
            Color::Lcha {
                lightness,
                chroma,
                hue,
                alpha,
            } => {
                let [red, green, blue] =
                    LchRepresentation::lch_to_nonlinear_srgb(*lightness, *chroma, *hue);

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
            Color::Lcha {
                lightness,
                chroma,
                hue,
                alpha,
            } => {
                let rgb = LchRepresentation::lch_to_nonlinear_srgb(*lightness, *chroma, *hue);
                let (hue, saturation, lightness) = HslRepresentation::nonlinear_srgb_to_hsl(rgb);

                Color::Hsla {
                    hue,
                    saturation,
                    lightness,
                    alpha: *alpha,
                }
            }
        }
    }

    /// Converts a `Color` to variant `Color::Lcha`
    pub fn as_lcha(self: &Color) -> Color {
        match self {
            Color::Rgba {
                red,
                green,
                blue,
                alpha,
            } => {
                let (lightness, chroma, hue) =
                    LchRepresentation::nonlinear_srgb_to_lch([*red, *green, *blue]);
                Color::Lcha {
                    lightness,
                    chroma,
                    hue,
                    alpha: *alpha,
                }
            }
            Color::RgbaLinear {
                red,
                green,
                blue,
                alpha,
            } => {
                let (lightness, chroma, hue) = LchRepresentation::nonlinear_srgb_to_lch([
                    red.linear_to_nonlinear_srgb(),
                    green.linear_to_nonlinear_srgb(),
                    blue.linear_to_nonlinear_srgb(),
                ]);
                Color::Lcha {
                    lightness,
                    chroma,
                    hue,
                    alpha: *alpha,
                }
            }
            Color::Hsla {
                hue,
                saturation,
                lightness,
                alpha,
            } => {
                let rgb = HslRepresentation::hsl_to_nonlinear_srgb(*hue, *saturation, *lightness);
                let (lightness, chroma, hue) = LchRepresentation::nonlinear_srgb_to_lch(rgb);
                Color::Lcha {
                    lightness,
                    chroma,
                    hue,
                    alpha: *alpha,
                }
            }
            Color::Lcha { .. } => *self,
        }
    }

    /// Converts a `Color` to a `[u8; 4]` from sRGB colorspace
    pub fn as_rgba_u8(&self) -> [u8; 4] {
        let [r, g, b, a] = self.as_rgba_f32();
        [
            (r * u8::MAX as f32) as u8,
            (g * u8::MAX as f32) as u8,
            (b * u8::MAX as f32) as u8,
            (a * u8::MAX as f32) as u8,
        ]
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
            Color::Lcha {
                lightness,
                chroma,
                hue,
                alpha,
            } => {
                let [red, green, blue] =
                    LchRepresentation::lch_to_nonlinear_srgb(lightness, chroma, hue);

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
            Color::Lcha {
                lightness,
                chroma,
                hue,
                alpha,
            } => {
                let [red, green, blue] =
                    LchRepresentation::lch_to_nonlinear_srgb(lightness, chroma, hue);

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
            Color::Lcha {
                lightness,
                chroma,
                hue,
                alpha,
            } => {
                let rgb = LchRepresentation::lch_to_nonlinear_srgb(lightness, chroma, hue);
                let (hue, saturation, lightness) = HslRepresentation::nonlinear_srgb_to_hsl(rgb);

                [hue, saturation, lightness, alpha]
            }
        }
    }

    /// Converts a `Color` to a `[f32; 4]` from LCH colorspace
    pub fn as_lcha_f32(self: Color) -> [f32; 4] {
        match self {
            Color::Rgba {
                red,
                green,
                blue,
                alpha,
            } => {
                let (lightness, chroma, hue) =
                    LchRepresentation::nonlinear_srgb_to_lch([red, green, blue]);
                [lightness, chroma, hue, alpha]
            }
            Color::RgbaLinear {
                red,
                green,
                blue,
                alpha,
            } => {
                let (lightness, chroma, hue) = LchRepresentation::nonlinear_srgb_to_lch([
                    red.linear_to_nonlinear_srgb(),
                    green.linear_to_nonlinear_srgb(),
                    blue.linear_to_nonlinear_srgb(),
                ]);
                [lightness, chroma, hue, alpha]
            }
            Color::Hsla {
                hue,
                saturation,
                lightness,
                alpha,
            } => {
                let rgb = HslRepresentation::hsl_to_nonlinear_srgb(hue, saturation, lightness);
                let (lightness, chroma, hue) = LchRepresentation::nonlinear_srgb_to_lch(rgb);

                [lightness, chroma, hue, alpha]
            }
            Color::Lcha {
                lightness,
                chroma,
                hue,
                alpha,
            } => [lightness, chroma, hue, alpha],
        }
    }

    /// Converts `Color` to a `u32` from sRGB colorspace.
    ///
    /// Maps the RGBA channels in RGBA order to a little-endian byte array (GPUs are little-endian).
    /// `A` will be the most significant byte and `R` the least significant.
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
            Color::Lcha {
                lightness,
                chroma,
                hue,
                alpha,
            } => {
                let [red, green, blue] =
                    LchRepresentation::lch_to_nonlinear_srgb(lightness, chroma, hue);

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
    /// `A` will be the most significant byte and `R` the least significant.
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
            Color::Lcha {
                lightness,
                chroma,
                hue,
                alpha,
            } => {
                let [red, green, blue] =
                    LchRepresentation::lch_to_nonlinear_srgb(lightness, chroma, hue);

                u32::from_le_bytes([
                    (red.nonlinear_to_linear_srgb() * 255.0) as u8,
                    (green.nonlinear_to_linear_srgb() * 255.0) as u8,
                    (blue.nonlinear_to_linear_srgb() * 255.0) as u8,
                    (alpha * 255.0) as u8,
                ])
            }
        }
    }

    /// New `Color` from `[f32; 4]` (or a type that can be converted into them) with RGB representation in sRGB colorspace.
    #[inline]
    pub fn rgba_from_array(arr: impl Into<[f32; 4]>) -> Self {
        let [r, g, b, a]: [f32; 4] = arr.into();
        Color::rgba(r, g, b, a)
    }

    /// New `Color` from `[f32; 3]` (or a type that can be converted into them) with RGB representation in sRGB colorspace.
    #[inline]
    pub fn rgb_from_array(arr: impl Into<[f32; 3]>) -> Self {
        let [r, g, b]: [f32; 3] = arr.into();
        Color::rgb(r, g, b)
    }

    /// New `Color` from `[f32; 4]` (or a type that can be converted into them) with RGB representation in linear RGB colorspace.
    #[inline]
    pub fn rgba_linear_from_array(arr: impl Into<[f32; 4]>) -> Self {
        let [r, g, b, a]: [f32; 4] = arr.into();
        Color::rgba_linear(r, g, b, a)
    }

    /// New `Color` from `[f32; 3]` (or a type that can be converted into them) with RGB representation in linear RGB colorspace.
    #[inline]
    pub fn rgb_linear_from_array(arr: impl Into<[f32; 3]>) -> Self {
        let [r, g, b]: [f32; 3] = arr.into();
        Color::rgb_linear(r, g, b)
    }

    /// New `Color` from `[f32; 4]` (or a type that can be converted into them) with HSL representation in sRGB colorspace.
    #[inline]
    pub fn hsla_from_array(arr: impl Into<[f32; 4]>) -> Self {
        let [h, s, l, a]: [f32; 4] = arr.into();
        Color::hsla(h, s, l, a)
    }

    /// New `Color` from `[f32; 3]` (or a type that can be converted into them) with HSL representation in sRGB colorspace.
    #[inline]
    pub fn hsl_from_array(arr: impl Into<[f32; 3]>) -> Self {
        let [h, s, l]: [f32; 3] = arr.into();
        Color::hsl(h, s, l)
    }

    /// New `Color` from `[f32; 4]` (or a type that can be converted into them) with LCH representation in sRGB colorspace.
    #[inline]
    pub fn lcha_from_array(arr: impl Into<[f32; 4]>) -> Self {
        let [l, c, h, a]: [f32; 4] = arr.into();
        Color::lcha(l, c, h, a)
    }

    /// New `Color` from `[f32; 3]` (or a type that can be converted into them) with LCH representation in sRGB colorspace.
    #[inline]
    pub fn lch_from_array(arr: impl Into<[f32; 3]>) -> Self {
        let [l, c, h]: [f32; 3] = arr.into();
        Color::lch(l, c, h)
    }

    /// Convert `Color` to RGBA and return as `Vec4`.
    #[inline]
    pub fn rgba_to_vec4(&self) -> Vec4 {
        let color = self.as_rgba();
        match color {
            Color::Rgba {
                red,
                green,
                blue,
                alpha,
            } => Vec4::new(red, green, blue, alpha),
            _ => unreachable!(),
        }
    }

    /// Convert `Color` to RGBA and return as `Vec3`.
    #[inline]
    pub fn rgb_to_vec3(&self) -> Vec3 {
        let color = self.as_rgba();
        match color {
            Color::Rgba {
                red, green, blue, ..
            } => Vec3::new(red, green, blue),
            _ => unreachable!(),
        }
    }

    /// Convert `Color` to linear RGBA and return as `Vec4`.
    #[inline]
    pub fn rgba_linear_to_vec4(&self) -> Vec4 {
        let color = self.as_rgba_linear();
        match color {
            Color::RgbaLinear {
                red,
                green,
                blue,
                alpha,
            } => Vec4::new(red, green, blue, alpha),
            _ => unreachable!(),
        }
    }

    /// Convert `Color` to linear RGBA and return as `Vec3`.
    #[inline]
    pub fn rgb_linear_to_vec3(&self) -> Vec3 {
        let color = self.as_rgba_linear();
        match color {
            Color::RgbaLinear {
                red, green, blue, ..
            } => Vec3::new(red, green, blue),
            _ => unreachable!(),
        }
    }

    /// Convert `Color` to HSLA and return as `Vec4`.
    #[inline]
    pub fn hsla_to_vec4(&self) -> Vec4 {
        let color = self.as_hsla();
        match color {
            Color::Hsla {
                hue,
                saturation,
                lightness,
                alpha,
            } => Vec4::new(hue, saturation, lightness, alpha),
            _ => unreachable!(),
        }
    }

    /// Convert `Color` to HSLA and return as `Vec3`.
    #[inline]
    pub fn hsl_to_vec3(&self) -> Vec3 {
        let color = self.as_hsla();
        match color {
            Color::Hsla {
                hue,
                saturation,
                lightness,
                ..
            } => Vec3::new(hue, saturation, lightness),
            _ => unreachable!(),
        }
    }

    /// Convert `Color` to LCHA and return as `Vec4`.
    #[inline]
    pub fn lcha_to_vec4(&self) -> Vec4 {
        let color = self.as_lcha();
        match color {
            Color::Lcha {
                lightness,
                chroma,
                hue,
                alpha,
            } => Vec4::new(lightness, chroma, hue, alpha),
            _ => unreachable!(),
        }
    }

    /// Convert `Color` to LCHA and return as `Vec3`.
    #[inline]
    pub fn lch_to_vec3(&self) -> Vec3 {
        let color = self.as_lcha();
        match color {
            Color::Lcha {
                lightness,
                chroma,
                hue,
                ..
            } => Vec3::new(lightness, chroma, hue),
            _ => unreachable!(),
        }
    }
}

impl Default for Color {
    fn default() -> Self {
        Color::WHITE
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
                let rhs = rhs.as_hsla_f32();
                Color::Hsla {
                    hue: hue + rhs[0],
                    saturation: saturation + rhs[1],
                    lightness: lightness + rhs[2],
                    alpha: alpha + rhs[3],
                }
            }
            Color::Lcha {
                lightness,
                chroma,
                hue,
                alpha,
            } => {
                let rhs = rhs.as_lcha_f32();
                Color::Lcha {
                    lightness: lightness + rhs[0],
                    chroma: chroma + rhs[1],
                    hue: hue + rhs[2],
                    alpha: alpha + rhs[3],
                }
            }
        }
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
            Color::Lcha {
                lightness,
                chroma,
                hue,
                alpha,
            } => Color::Lcha {
                lightness: lightness * rhs,
                chroma: chroma * rhs,
                hue: hue * rhs,
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
            Color::Lcha {
                lightness,
                chroma,
                hue,
                ..
            } => {
                *lightness *= rhs;
                *chroma *= rhs;
                *hue *= rhs;
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
            Color::Lcha {
                lightness,
                chroma,
                hue,
                alpha,
            } => Color::Lcha {
                lightness: lightness * rhs.x,
                chroma: chroma * rhs.y,
                hue: hue * rhs.z,
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
            Color::Lcha {
                lightness,
                chroma,
                hue,
                alpha,
            } => {
                *lightness *= rhs.x;
                *chroma *= rhs.y;
                *hue *= rhs.z;
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
            Color::Lcha {
                lightness,
                chroma,
                hue,
                alpha,
            } => Color::Lcha {
                lightness: lightness * rhs.x,
                chroma: chroma * rhs.y,
                hue: hue * rhs.z,
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
            Color::Lcha {
                lightness,
                chroma,
                hue,
                ..
            } => {
                *lightness *= rhs.x;
                *chroma *= rhs.y;
                *hue *= rhs.z;
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
            Color::Lcha {
                lightness,
                chroma,
                hue,
                alpha,
            } => Color::Lcha {
                lightness: lightness * rhs[0],
                chroma: chroma * rhs[1],
                hue: hue * rhs[2],
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
            Color::Lcha {
                lightness,
                chroma,
                hue,
                alpha,
            } => {
                *lightness *= rhs[0];
                *chroma *= rhs[1];
                *hue *= rhs[2];
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
            Color::Lcha {
                lightness,
                chroma,
                hue,
                alpha,
            } => Color::Lcha {
                lightness: lightness * rhs[0],
                chroma: chroma * rhs[1],
                hue: hue * rhs[2],
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
            Color::Lcha {
                lightness,
                chroma,
                hue,
                ..
            } => {
                *lightness *= rhs[0];
                *chroma *= rhs[1];
                *hue *= rhs[2];
            }
        }
    }
}

impl encase::ShaderType for Color {
    type ExtraMetadata = ();

    const METADATA: encase::private::Metadata<Self::ExtraMetadata> = {
        let size =
            encase::private::SizeValue::from(<f32 as encase::private::ShaderSize>::SHADER_SIZE)
                .mul(4);
        let alignment = encase::private::AlignmentValue::from_next_power_of_two_size(size);

        encase::private::Metadata {
            alignment,
            has_uniform_min_alignment: false,
            min_size: size,
            extra: (),
        }
    };

    const UNIFORM_COMPAT_ASSERT: fn() = || {};
}

impl encase::private::WriteInto for Color {
    fn write_into<B: encase::private::BufferMut>(&self, writer: &mut encase::private::Writer<B>) {
        let linear = self.as_linear_rgba_f32();
        for el in &linear {
            encase::private::WriteInto::write_into(el, writer);
        }
    }
}

impl encase::private::ReadFrom for Color {
    fn read_from<B: encase::private::BufferRef>(
        &mut self,
        reader: &mut encase::private::Reader<B>,
    ) {
        let mut buffer = [0.0f32; 4];
        for el in &mut buffer {
            encase::private::ReadFrom::read_from(el, reader);
        }

        *self = Color::RgbaLinear {
            red: buffer[0],
            green: buffer[1],
            blue: buffer[2],
            alpha: buffer[3],
        }
    }
}

impl encase::private::CreateFrom for Color {
    fn create_from<B>(reader: &mut encase::private::Reader<B>) -> Self
    where
        B: encase::private::BufferRef,
    {
        // These are intentionally not inlined in the constructor to make this
        // resilient to internal Color refactors / implicit type changes.
        let red: f32 = encase::private::CreateFrom::create_from(reader);
        let green: f32 = encase::private::CreateFrom::create_from(reader);
        let blue: f32 = encase::private::CreateFrom::create_from(reader);
        let alpha: f32 = encase::private::CreateFrom::create_from(reader);
        Color::RgbaLinear {
            red,
            green,
            blue,
            alpha,
        }
    }
}

impl encase::ShaderSize for Color {}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum HexColorError {
    #[error("Unexpected length of hex string")]
    Length,
    #[error("Invalid hex char")]
    Char(char),
}

/// Converts hex bytes to an array of RGB\[A\] components
///
/// # Example
/// For RGB: *b"ffffff" -> [255, 255, 255, ..]
/// For RGBA: *b"E2E2E2FF" -> [226, 226, 226, 255, ..]
const fn decode_hex<const N: usize>(mut bytes: [u8; N]) -> Result<[u8; N], HexColorError> {
    let mut i = 0;
    while i < bytes.len() {
        // Convert single hex digit to u8
        let val = match hex_value(bytes[i]) {
            Ok(val) => val,
            Err(byte) => return Err(HexColorError::Char(byte as char)),
        };
        bytes[i] = val;
        i += 1;
    }
    // Modify the original bytes to give an `N / 2` length result
    i = 0;
    while i < bytes.len() / 2 {
        // Convert pairs of u8 to R/G/B/A
        // e.g `ff` -> [102, 102] -> [15, 15] = 255
        bytes[i] = bytes[i * 2] * 16 + bytes[i * 2 + 1];
        i += 1;
    }
    Ok(bytes)
}

/// Parse a single hex digit (a-f/A-F/0-9) as a `u8`
const fn hex_value(b: u8) -> Result<u8, u8> {
    match b {
        b'0'..=b'9' => Ok(b - b'0'),
        b'A'..=b'F' => Ok(b - b'A' + 10),
        b'a'..=b'f' => Ok(b - b'a' + 10),
        // Wrong hex digit
        _ => Err(b),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_color() {
        assert_eq!(Color::hex("FFF"), Ok(Color::WHITE));
        assert_eq!(Color::hex("FFFF"), Ok(Color::WHITE));
        assert_eq!(Color::hex("FFFFFF"), Ok(Color::WHITE));
        assert_eq!(Color::hex("FFFFFFFF"), Ok(Color::WHITE));
        assert_eq!(Color::hex("000"), Ok(Color::BLACK));
        assert_eq!(Color::hex("000F"), Ok(Color::BLACK));
        assert_eq!(Color::hex("000000"), Ok(Color::BLACK));
        assert_eq!(Color::hex("000000FF"), Ok(Color::BLACK));
        assert_eq!(Color::hex("03a9f4"), Ok(Color::rgb_u8(3, 169, 244)));
        assert_eq!(Color::hex("yy"), Err(HexColorError::Length));
        assert_eq!(Color::hex("yyy"), Err(HexColorError::Char('y')));
        assert_eq!(Color::hex("#f2a"), Ok(Color::rgb_u8(255, 34, 170)));
        assert_eq!(Color::hex("#e23030"), Ok(Color::rgb_u8(226, 48, 48)));
        assert_eq!(Color::hex("#ff"), Err(HexColorError::Length));
        assert_eq!(Color::hex("##fff"), Err(HexColorError::Char('#')));
    }

    #[test]
    fn conversions_vec4() {
        let starting_vec4 = Vec4::new(0.4, 0.5, 0.6, 1.0);
        let starting_color = Color::rgba_from_array(starting_vec4);

        assert_eq!(starting_vec4, starting_color.rgba_to_vec4());

        let transformation = Vec4::new(0.5, 0.5, 0.5, 1.0);

        assert_eq!(
            starting_color * transformation,
            Color::rgba_from_array(starting_vec4 * transformation)
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

        assert_eq!(starting_color * transformation, mutated_color);
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

        assert_eq!(starting_color * transformation, mutated_color);
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

        assert_eq!(starting_color * transformation, mutated_color);
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

        assert_eq!(starting_color * transformation, mutated_color);
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

        assert_eq!(starting_color * transformation, mutated_color);
    }

    // regression test for https://github.com/bevyengine/bevy/pull/8040
    #[test]
    fn convert_to_rgba_linear() {
        let rgba = Color::rgba(0., 0., 0., 0.);
        let rgba_l = Color::rgba_linear(0., 0., 0., 0.);
        let hsla = Color::hsla(0., 0., 0., 0.);
        let lcha = Color::lcha(0., 0., 0., 0.);
        assert_eq!(rgba_l, rgba_l.as_rgba_linear());
        let Color::RgbaLinear { .. } = rgba.as_rgba_linear() else {
            panic!("from Rgba")
        };
        let Color::RgbaLinear { .. } = hsla.as_rgba_linear() else {
            panic!("from Hsla")
        };
        let Color::RgbaLinear { .. } = lcha.as_rgba_linear() else {
            panic!("from Lcha")
        };
    }
}
