//! Named colors from the CSS1 specification, also known as
//! [basic colors](https://en.wikipedia.org/wiki/Web_colors#Basic_colors).
//! This is the same set of colors used in the
//! [VGA graphcs standard](https://en.wikipedia.org/wiki/Video_Graphics_Array).

use crate::Srgba;

/// <div style="background-color:rgb(0%, 0%, 0%); width: 10px; padding: 10px; border: 1px solid;"></div>
pub const BLACK: Srgba = Srgba::new(0.0, 0.0, 0.0, 1.0);
/// <div style="background-color:rgb(0%, 0%, 100%); width: 10px; padding: 10px; border: 1px solid;"></div>
pub const BLUE: Srgba = Srgba::new(0.0, 0.0, 1.0, 1.0);
/// <div style="background-color:rgb(0%, 100%, 100%); width: 10px; padding: 10px; border: 1px solid;"></div>
pub const CYAN: Srgba = Srgba::new(0.0, 1.0, 1.0, 1.0);
/// <div style="background-color:rgb(0%, 50%, 0%); width: 10px; padding: 10px; border: 1px solid;"></div>
pub const GREEN: Srgba = Srgba::new(0.0, 0.5, 0.0, 1.0);
/// <div style="background-color:rgb(100%, 0%, 100%); width: 10px; padding: 10px; border: 1px solid;"></div>
pub const FUCHSIA: Srgba = Srgba::new(1.0, 0.0, 1.0, 1.0);
/// <div style="background-color:rgb(50%, 50%, 50%); width: 10px; padding: 10px; border: 1px solid;"></div>
pub const GRAY: Srgba = Srgba::new(0.5, 0.5, 0.5, 1.0);
/// <div style="background-color:rgb(0%, 100%, 0%); width: 10px; padding: 10px; border: 1px solid;"></div>
pub const LIME: Srgba = Srgba::new(0.0, 1.0, 0.0, 1.0);
/// <div style="background-color:rgb(50%, 0%, 0%); width: 10px; padding: 10px; border: 1px solid;"></div>
pub const MAROON: Srgba = Srgba::new(0.5, 0.0, 0.0, 1.0);
/// <div style="background-color:rgb(0%, 0%, 50%); width: 10px; padding: 10px; border: 1px solid;"></div>
pub const NAVY: Srgba = Srgba::new(0.0, 0.0, 0.5, 1.0);
/// <div style="background-color:rgb(50%, 50%, 0%); width: 10px; padding: 10px; border: 1px solid;"></div>
pub const OLIVE: Srgba = Srgba::new(0.5, 0.5, 0.0, 1.0);
/// <div style="background-color:rgb(50%, 0%, 50%); width: 10px; padding: 10px; border: 1px solid;"></div>
pub const PURPLE: Srgba = Srgba::new(0.5, 0.0, 0.5, 1.0);
/// <div style="background-color:rgb(100%, 0%, 0%); width: 10px; padding: 10px; border: 1px solid;"></div>
pub const RED: Srgba = Srgba::new(1.0, 0.0, 0.0, 1.0);
/// <div style="background-color:rgb(75%, 75%, 75%); width: 10px; padding: 10px; border: 1px solid;"></div>
pub const SILVER: Srgba = Srgba::new(0.75, 0.75, 0.75, 1.0);
/// <div style="background-color:rgb(0%, 50%, 50%); width: 10px; padding: 10px; border: 1px solid;"></div>
pub const TEAL: Srgba = Srgba::new(0.0, 0.5, 0.5, 1.0);
/// <div style="background-color:rgb(100%, 100%, 100%); width: 10px; padding: 10px; border: 1px solid;"></div>
pub const WHITE: Srgba = Srgba::new(1.0, 1.0, 1.0, 1.0);
/// <div style="background-color:rgb(100%, 100%, 0%); width: 10px; padding: 10px; border: 1px solid;"></div>
pub const YELLOW: Srgba = Srgba::new(1.0, 1.0, 0.0, 1.0);
