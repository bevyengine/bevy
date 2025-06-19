//! The Feathers standard color palette.
use bevy_color::Color;

/// Black
pub const BLACK: Color = Color::oklcha(0.0, 0.0, 0.0, 1.0);
/// Gray 0 - window background
pub const GRAY_0: Color = Color::oklcha(0.2414, 0.0095, 285.67, 1.0);
/// Gray 1 - pane background
pub const GRAY_1: Color = Color::oklcha(0.2866, 0.0072, 285.93, 1.0);
/// Gray 2 - item background
pub const GRAY_2: Color = Color::oklcha(0.3373, 0.0071, 274.77, 1.0);
/// Gray 3 - item background (active)
pub const GRAY_3: Color = Color::oklcha(0.3992, 0.0101, 278.38, 1.0);
/// Warm Gray 3 - border
pub const WARM_GRAY_1: Color = Color::oklcha(0.3757, 0.0017, 286.32, 1.0);
/// Light Gray 1 - bright label text
pub const LIGHT_GRAY_1: Color = Color::oklcha(0.7607, 0.0014, 286.37, 1.0);
/// Light Gray 2 - dim label text
pub const LIGHT_GRAY_2: Color = Color::oklcha(0.6106, 0.003, 286.31, 1.0);
/// White - button label text
pub const WHITE: Color = Color::oklcha(1.0, 0.000000059604645, 90.0, 1.0);
/// Accent - call-to-action and selection color
pub const ACCENT: Color = Color::oklcha(0.542, 0.1594, 255.4, 1.0);
/// Dark Coral - for X-axis inputs and drag handles
pub const X_AXIS: Color = Color::oklcha(0.5232, 0.1404, 13.84, 1.0);
/// Olive - for Y-axis inputs and drag handles
pub const Y_AXIS: Color = Color::oklcha(0.5866, 0.1543, 129.84, 1.0);
/// Steel Blue - for Z-axis inputs and drag handles
pub const Z_AXIS: Color = Color::oklcha(0.4847, 0.1249, 253.08, 1.0);
