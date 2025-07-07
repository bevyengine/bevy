//! The Feathers standard color palette.
use bevy_color::Color;

/// <div style="background-color: #000000; width: 10px; padding: 10px; border: 1px solid;"></div>
pub const BLACK: Color = Color::oklcha(0.0, 0.0, 0.0, 1.0);
/// <div style="background-color: #1F1F24; width: 10px; padding: 10px; border: 1px solid;"></div> - window background
pub const GRAY_0: Color = Color::oklcha(0.2414, 0.0095, 285.67, 1.0);
/// <div style="background-color: #2A2A2E; width: 10px; padding: 10px; border: 1px solid;"></div> - pane background
pub const GRAY_1: Color = Color::oklcha(0.2866, 0.0072, 285.93, 1.0);
/// <div style="background-color: #36373B; width: 10px; padding: 10px; border: 1px solid;"></div> - item background
pub const GRAY_2: Color = Color::oklcha(0.3373, 0.0071, 274.77, 1.0);
/// <div style="background-color: #46474D; width: 10px; padding: 10px; border: 1px solid;"></div> - item background (active)
pub const GRAY_3: Color = Color::oklcha(0.3992, 0.0101, 278.38, 1.0);
/// <div style="background-color: #414142; width: 10px; padding: 10px; border: 1px solid;"></div> - border
pub const WARM_GRAY_1: Color = Color::oklcha(0.3757, 0.0017, 286.32, 1.0);
/// <div style="background-color: #B1B1B2; width: 10px; padding: 10px; border: 1px solid;"></div> - bright label text
pub const LIGHT_GRAY_1: Color = Color::oklcha(0.7607, 0.0014, 286.37, 1.0);
/// <div style="background-color: #838385; width: 10px; padding: 10px; border: 1px solid;"></div> - dim label text
pub const LIGHT_GRAY_2: Color = Color::oklcha(0.6106, 0.003, 286.31, 1.0);
/// <div style="background-color: #FFFFFF; width: 10px; padding: 10px; border: 1px solid;"></div> - button label text
pub const WHITE: Color = Color::oklcha(1.0, 0.000000059604645, 90.0, 1.0);
/// <div style="background-color: #206EC9; width: 10px; padding: 10px; border: 1px solid;"></div> - call-to-action and selection color
pub const ACCENT: Color = Color::oklcha(0.542, 0.1594, 255.4, 1.0);
/// <div style="background-color: #AB4051; width: 10px; padding: 10px; border: 1px solid;"></div> - for X-axis inputs and drag handles
pub const X_AXIS: Color = Color::oklcha(0.5232, 0.1404, 13.84, 1.0);
/// <div style="background-color: #5D8D0A; width: 10px; padding: 10px; border: 1px solid;"></div> - for Y-axis inputs and drag handles
pub const Y_AXIS: Color = Color::oklcha(0.5866, 0.1543, 129.84, 1.0);
/// <div style="background-color: #2160A3; width: 10px; padding: 10px; border: 1px solid;"></div> - for Z-axis inputs and drag handles
pub const Z_AXIS: Color = Color::oklcha(0.4847, 0.1249, 253.08, 1.0);
