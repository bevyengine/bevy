//! Bevy color palette. from [rfcs/69-editor-design-system](https://github.com/coreh/bevy-rfcs/blob/editor-design-system/rfcs/69-editor-design-system.md)
use crate::Srgba;

/// The Bevy theme palette
pub struct BevyTheme {
    /// <div style="background-color:#1e1e22; width: 10px; padding: 10px; border: 1px solid;"></div>
    pub background: Srgba,
    /// <div style="background-color:#ececec; width: 10px; padding: 10px; border: 1px solid;"></div>
    pub foreground: Srgba,
    /// <div style="background-color:#1b1b1c; width: 10px; padding: 10px; border: 1px solid;"></div>
    pub surface_m2: Srgba,
    /// <div style="background-color:#1e1e1f; width: 10px; padding: 10px; border: 1px solid;"></div>
    pub surface_m1: Srgba,
    /// <div style="background-color:#232326; width: 10px; padding: 10px; border: 1px solid;"></div>
    pub surface: Srgba,
    /// <div style="background-color:#2b2c2f; width: 10px; padding: 10px; border: 1px solid;"></div>
    pub surface_p1: Srgba,
    /// <div style="background-color:#383838; width: 10px; padding: 10px; border: 1px solid;"></div>
    pub surface_p2: Srgba,
    /// <div style="background-color:#0ea5e9; width: 10px; padding: 10px; border: 1px solid;"></div>
    pub accent: Srgba,
    /// <div style="background-color:#831843; width: 10px; padding: 10px; border: 1px solid;"></div>
    pub animation: Srgba,
    /// <div style="background-color:#9333ea; width: 10px; padding: 10px; border: 1px solid;"></div>
    pub asset: Srgba,
    /// <div style="background-color:#f74c00; width: 10px; padding: 10px; border: 1px solid;"></div>
    pub code: Srgba,
    /// <div style="background-color:#fcd34d; width: 10px; padding: 10px; border: 1px solid;"></div>
    pub light: Srgba,
    /// <div style="background-color:#10b981; width: 10px; padding: 10px; border: 1px solid;"></div>
    pub resource: Srgba,
    /// <div style="background-color:#799bbb; width: 10px; padding: 10px; border: 1px solid;"></div>
    pub primary: Srgba,
    /// <div style="background-color:#576f86; width: 10px; padding: 10px; border: 1px solid;"></div>
    pub primary_dark: Srgba,
    /// <div style="background-color:#bb799c; width: 10px; padding: 10px; border: 1px solid;"></div>
    pub destructive: Srgba,
    /// <div style="background-color:#865767; width: 10px; padding: 10px; border: 1px solid;"></div>
    pub destructive_dark: Srgba,
    /// <div style="background-color:#ff3653; width: 10px; padding: 10px; border: 1px solid;"></div>
    pub x: Srgba,
    /// <div style="background-color:#8adb00; width: 10px; padding: 10px; border: 1px solid;"></div>
    pub y: Srgba,
    /// <div style="background-color:#2c8fff; width: 10px; padding: 10px; border: 1px solid;"></div>
    pub z: Srgba,
}

impl Default for BevyTheme {
    fn default() -> Self {
        BevyTheme {
            background: Srgba::new(0.11764706, 0.11764706, 0.13333334, 1.0),
            foreground: Srgba::new(0.9254902, 0.9254902, 0.9254902, 1.0),
            surface_m2: Srgba::new(0.105882354, 0.105882354, 0.10980392, 1.0),
            surface_m1: Srgba::new(0.11764706, 0.11764706, 0.12156863, 1.0),
            surface: Srgba::new(0.13725491, 0.13725491, 0.14901961, 1.0),
            surface_p1: Srgba::new(0.16862746, 0.17254902, 0.18431373, 1.0),
            surface_p2: Srgba::new(0.21960784, 0.21960784, 0.21960784, 1.0),
            accent: Srgba::new(0.05490196, 0.64705884, 0.9137255, 1.0),
            animation: Srgba::new(0.5137255, 0.09411765, 0.2627451, 1.0),
            asset: Srgba::new(0.5764706, 0.2, 0.91764706, 1.0),
            code: Srgba::new(0.96862745, 0.29803923, 0.0, 1.0),
            light: Srgba::new(0.9882353, 0.827451, 0.3019608, 1.0),
            resource: Srgba::new(0.0627451, 0.7254902, 0.5058824, 1.0),
            primary: Srgba::new(0.4745098, 0.60784316, 0.73333335, 1.0),
            primary_dark: Srgba::new(0.34117648, 0.43529412, 0.5254902, 1.0),
            destructive: Srgba::new(0.73333335, 0.4745098, 0.6117647, 1.0),
            destructive_dark: Srgba::new(0.5254902, 0.34117648, 0.40392157, 1.0),
            x: Srgba::new(1.0, 0.21176471, 0.3254902, 1.0),
            y: Srgba::new(0.5411765, 0.85882354, 0.0, 1.0),
            z: Srgba::new(0.17254902, 0.56078434, 1.0, 1.0),
        }
    }
}
