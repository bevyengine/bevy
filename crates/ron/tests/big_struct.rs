use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct ImVec2 {
    pub x: f32,
    pub y: f32,
}

#[derive(Serialize, Deserialize)]
pub struct ImColorsSave {
    pub text: f32,
}

#[derive(Serialize, Deserialize)]
pub struct ImGuiStyleSave {
    pub alpha: f32,
    pub window_padding: ImVec2,
    pub window_min_size: ImVec2,
    pub window_rounding: f32,
    pub window_title_align: ImVec2,
    pub child_window_rounding: f32,
    pub frame_padding: ImVec2,
    pub frame_rounding: f32,
    pub item_spacing: ImVec2,
    pub item_inner_spacing: ImVec2,
    pub touch_extra_padding: ImVec2,
    pub indent_spacing: f32,
    pub columns_min_spacing: f32,
    pub scrollbar_size: f32,
    pub scrollbar_rounding: f32,
    pub grab_min_size: f32,
    pub grab_rounding: f32,
    pub button_text_align: ImVec2,
    pub display_window_padding: ImVec2,
    pub display_safe_area_padding: ImVec2,
    pub anti_aliased_lines: bool,
    pub anti_aliased_shapes: bool,
    pub curve_tessellation_tol: f32,
    pub colors: ImColorsSave,
    pub new_type: NewType,
}

#[derive(Serialize, Deserialize)]
pub struct NewType(i32);

const CONFIG: &str = "(
    alpha: 1.0,
    window_padding: (x: 8, y: 8),
    window_min_size: (x: 32, y: 32),
    window_rounding: 9.0,
    window_title_align: (x: 0.0, y: 0.5),
    child_window_rounding: 0.0,
    frame_padding: (x: 4, y: 3),
    frame_rounding: 0.0,
    item_spacing: (x: 8, y: 4),
    item_inner_spacing: (x: 4, y: 4),
    touch_extra_padding: (x: 0, y: 0),
    indent_spacing: 21.0,
    columns_min_spacing: 6.0,
    scrollbar_size: 16,
    scrollbar_rounding: 9,
    grab_min_size: 10,
    grab_rounding: 0,
    button_text_align: (x: 0.5, y: 0.5),
    display_window_padding: (x: 22, y: 22),
    display_safe_area_padding: (x: 4, y: 4),
    anti_aliased_lines: true,
    anti_aliased_shapes: true,
    curve_tessellation_tol: 1.25,
    colors: (text: 4),
    new_type: NewType(     1  ),

    ignored_field: \"Totally ignored, not causing a panic. Hopefully.\",
)";

#[test]
fn deserialize_big_struct() {
    ron::de::from_str::<ImGuiStyleSave>(CONFIG).unwrap();
}
