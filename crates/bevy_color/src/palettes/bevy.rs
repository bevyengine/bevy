//! Bevy color palette. from [rfcs/69-editor-design-system](https://github.com/coreh/bevy-rfcs/blob/editor-design-system/rfcs/69-editor-design-system.md)

use crate::Srgba;

/// <div style="background-color:#1e1e22; width: 10px; padding: 10px; border: 1px solid;"></div>
pub const BACKGROUND: Srgba = Srgba::new(0.11764706, 0.11764706, 0.13333334, 1.0);
/// <div style="background-color:#ececec; width: 10px; padding: 10px; border: 1px solid;"></div>
pub const FOREGROUND: Srgba = Srgba::new(0.9254902, 0.9254902, 0.9254902, 1.0);
/// <div style="background-color:#1b1b1c; width: 10px; padding: 10px; border: 1px solid;"></div>
pub const SURFACE_M2: Srgba = Srgba::new(0.105882354, 0.105882354, 0.10980392, 1.0);
/// <div style="background-color:#1e1e1f; width: 10px; padding: 10px; border: 1px solid;"></div>
pub const SURFACE_M1: Srgba = Srgba::new(0.11764706, 0.11764706, 0.12156863, 1.0);
/// <div style="background-color:#232326; width: 10px; padding: 10px; border: 1px solid;"></div>
pub const SURFACE: Srgba = Srgba::new(0.13725491, 0.13725491, 0.14901961, 1.0);
/// <div style="background-color:#2b2c2f; width: 10px; padding: 10px; border: 1px solid;"></div>
pub const SURFACE_P1: Srgba = Srgba::new(0.16862746, 0.17254902, 0.18431373, 1.0);
/// <div style="background-color:#383838; width: 10px; padding: 10px; border: 1px solid;"></div>
pub const SURFACE_P2: Srgba = Srgba::new(0.21960784, 0.21960784, 0.21960784, 1.0);
/// <div style="background-color:#0ea5e9; width: 10px; padding: 10px; border: 1px solid;"></div>
pub const ACCENT: Srgba = Srgba::new(0.05490196, 0.64705884, 0.9137255, 1.0);
/// <div style="background-color:#831843; width: 10px; padding: 10px; border: 1px solid;"></div>
pub const ANIMATION: Srgba = Srgba::new(0.5137255, 0.09411765, 0.2627451, 1.0);
/// <div style="background-color:#9333ea; width: 10px; padding: 10px; border: 1px solid;"></div>
pub const ASSET: Srgba = Srgba::new(0.5764706, 0.2, 0.91764706, 1.0);
/// <div style="background-color:#f74c00; width: 10px; padding: 10px; border: 1px solid;"></div>
pub const CODE: Srgba = Srgba::new(0.96862745, 0.29803923, 0.0, 1.0);
/// <div style="background-color:#fcd34d; width: 10px; padding: 10px; border: 1px solid;"></div>
pub const LIGHT: Srgba = Srgba::new(0.9882353, 0.827451, 0.3019608, 1.0);
/// <div style="background-color:#10b981; width: 10px; padding: 10px; border: 1px solid;"></div>
pub const RESOURCE: Srgba = Srgba::new(0.0627451, 0.7254902, 0.5058824, 1.0);
/// <div style="background-color:#799bbb; width: 10px; padding: 10px; border: 1px solid;"></div>
pub const PRIMARY: Srgba = Srgba::new(0.4745098, 0.60784316, 0.73333335, 1.0);
/// <div style="background-color:#576f86; width: 10px; padding: 10px; border: 1px solid;"></div>
pub const PRIMARY_DARK: Srgba = Srgba::new(0.34117648, 0.43529412, 0.5254902, 1.0);
/// <div style="background-color:#bb799c; width: 10px; padding: 10px; border: 1px solid;"></div>
pub const DESTRUCTIVE: Srgba = Srgba::new(0.73333335, 0.4745098, 0.6117647, 1.0);
/// <div style="background-color:#865767; width: 10px; padding: 10px; border: 1px solid;"></div>
pub const DESTRUCTIVE_DARK: Srgba = Srgba::new(0.5254902, 0.34117648, 0.40392157, 1.0);
/// <div style="background-color:#990000; width: 10px; padding: 10px; border: 1px solid;"></div>
pub const X: Srgba = Srgba::new(0.6, 0.0, 0.0, 1.0);
/// <div style="background-color:#007700; width: 10px; padding: 10px; border: 1px solid;"></div>
pub const Y: Srgba = Srgba::new(0.0, 0.46666667, 0.0, 1.0);
/// <div style="background-color:#0000cc; width: 10px; padding: 10px; border: 1px solid;"></div>
pub const Z: Srgba = Srgba::new(0.0, 0.0, 0.8, 1.0);
