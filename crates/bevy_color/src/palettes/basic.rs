//! Named colors from the CSS1 specification, also known as
//! [basic colors](https://en.wikipedia.org/wiki/Web_colors#Basic_colors).
//! This is the same set of colors used in the
//! [VGA graphics standard](https://en.wikipedia.org/wiki/Video_Graphics_Array).

use crate::Srgba;

/// <div style="background-color: #00FFFF; width: 10px; padding: 10px; border: 1px solid;"></div>
pub const AQUA: Srgba = Srgba::rgb(0.0, 1.0, 1.0);
/// <div style="background-color: #000000; width: 10px; padding: 10px; border: 1px solid;"></div>
pub const BLACK: Srgba = Srgba::rgb(0.0, 0.0, 0.0);
/// <div style="background-color: #0000FF; width: 10px; padding: 10px; border: 1px solid;"></div>
pub const BLUE: Srgba = Srgba::rgb(0.0, 0.0, 1.0);
/// <div style="background-color: #FF00FF; width: 10px; padding: 10px; border: 1px solid;"></div>
pub const FUCHSIA: Srgba = Srgba::rgb(1.0, 0.0, 1.0);
/// <div style="background-color: #808080; width: 10px; padding: 10px; border: 1px solid;"></div>
pub const GRAY: Srgba = Srgba::rgb(0.5019608, 0.5019608, 0.5019608);
/// <div style="background-color: #008000; width: 10px; padding: 10px; border: 1px solid;"></div>
pub const GREEN: Srgba = Srgba::rgb(0.0, 0.5019608, 0.0);
/// <div style="background-color: #00FF00; width: 10px; padding: 10px; border: 1px solid;"></div>
pub const LIME: Srgba = Srgba::rgb(0.0, 1.0, 0.0);
/// <div style="background-color: #800000; width: 10px; padding: 10px; border: 1px solid;"></div>
pub const MAROON: Srgba = Srgba::rgb(0.5019608, 0.0, 0.0);
/// <div style="background-color: #000080; width: 10px; padding: 10px; border: 1px solid;"></div>
pub const NAVY: Srgba = Srgba::rgb(0.0, 0.0, 0.5019608);
/// <div style="background-color: #808000; width: 10px; padding: 10px; border: 1px solid;"></div>
pub const OLIVE: Srgba = Srgba::rgb(0.5019608, 0.5019608, 0.0);
/// <div style="background-color: #800080; width: 10px; padding: 10px; border: 1px solid;"></div>
pub const PURPLE: Srgba = Srgba::rgb(0.5019608, 0.0, 0.5019608);
/// <div style="background-color: #FF0000; width: 10px; padding: 10px; border: 1px solid;"></div>
pub const RED: Srgba = Srgba::rgb(1.0, 0.0, 0.0);
/// <div style="background-color: #C0C0C0; width: 10px; padding: 10px; border: 1px solid;"></div>
pub const SILVER: Srgba = Srgba::rgb(0.7529412, 0.7529412, 0.7529412);
/// <div style="background-color: #008080; width: 10px; padding: 10px; border: 1px solid;"></div>
pub const TEAL: Srgba = Srgba::rgb(0.0, 0.5019608, 0.5019608);
/// <div style="background-color: #FFFFFF; width: 10px; padding: 10px; border: 1px solid;"></div>
pub const WHITE: Srgba = Srgba::rgb(1.0, 1.0, 1.0);
/// <div style="background-color: #FFFF00; width: 10px; padding: 10px; border: 1px solid;"></div>
pub const YELLOW: Srgba = Srgba::rgb(1.0, 1.0, 0.0);
