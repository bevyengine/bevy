use alloc::sync::Arc;

use bevy_app::{Plugin, PostUpdate};
use bevy_camera::visibility::Visibility;
use bevy_color::{Color, Hsla, Srgba};
use bevy_ecs::{
    change_detection::{DetectChanges, DetectChangesMut},
    component::Component,
    entity::Entity,
    event::EntityEvent,
    hierarchy::{ChildOf, Children},
    lifecycle::Despawn,
    observer::On,
    query::{Changed, With},
    reflect::ReflectComponent,
    relationship::Relationship,
    resource::Resource,
    schedule::IntoScheduleConfigs,
    system::{Commands, Query, Res, ResMut},
    template::FromTemplate,
    world::Mut,
};
use bevy_input::keyboard::{KeyCode, KeyboardInput};
use bevy_input_focus::{
    tab_navigation::TabIndex, AutoFocus, FocusCause, FocusLost, FocusedInput, InputFocus,
};
use bevy_log::warn;
use bevy_math::{UVec2, Vec2, Vec3};
use bevy_reflect::{prelude::ReflectDefault, Reflect};
use bevy_scene::{prelude::*, Ready};
use bevy_text::{EditableText, Justify, LineHeight, TextEdit, TextLayout};
use bevy_ui::{
    prelude::AccessibleLabel, px, AlignItems, AlignSelf, Display, FlexDirection, GridPlacement,
    GridTrack, JustifySelf, Node, RepeatedGridTrack,
};
use bevy_ui_widgets::{
    popover::{Popover, PopoverAlign, PopoverPlacement, PopoverSide},
    Activate, ActivateOnPress, SliderValue, ValueChange,
};

use crate::{
    constants::fonts,
    controls::{
        ButtonVariant, ColorChannel, ColorPlaneValue, ColorSlider, ColorSwatchGridUpdate,
        ColorSwatchValue, FeathersButton, FeathersColorPlane, FeathersColorSlider,
        FeathersColorSwatch, FeathersColorSwatchGrid, FeathersLazyMenu, FeathersMenuPopup,
        FeathersMenuToolButton, FeathersNumberInput, FeathersTextInput, FeathersTextInputContainer,
        HardLimit, NumberInputPrecision, NumberInputRange, NumberInputStep, NumberInputValue,
        SliderBaseColor,
    },
    display::{caption, label},
    font_styles::InheritableFont,
    rounded_corners::RoundedCorners,
};

const RECENT_COLORS_COLUMNS: u32 = 12;
const RECENT_COLORS_ROWS: u32 = 2;
const RECENT_COLORS_COUNT: usize = (RECENT_COLORS_COLUMNS * RECENT_COLORS_ROWS) as usize;

/// Component that contains the value of the color input.
#[derive(Component, Default, Clone, Reflect)]
#[reflect(Component, Clone, Default)]
#[component(immutable)]
pub struct ColorInputValue(pub Color);

/// Supported color editing modes
#[derive(Default, Clone, Copy, Reflect, PartialEq)]
pub enum ColorInputMode {
    /// Red/green/blue mode with R/G plane
    #[default]
    RGPlane,
    /// Hue/saturation/lightness mode with H/S plane
    HSPlane,
}

/// Resource that contains user preferences for the color input.
/// This is global (shared between all picker instances), because it's
/// a user preference.
///
/// This means that the choice of color space is "sticky": when the user chooses RGB or HSL modes,
/// the next time the picker opens, even if it's editing a different entity or color attribute,
/// the picker will be in that mode. The assumption is that artists have a preferred mode and will
/// generally stick with it. (Also, we have no way to store the user's preference on a
/// per-attribute basis.)
#[derive(Resource, Default)]
// #[derive(SettingsGroup)] // TODO
pub struct ColorInputSettings {
    /// Which color space we're editing
    pub mode: ColorInputMode,
    /// List of recently edited colors
    pub recent_colors: Vec<Color>,
}

impl ColorInputSettings {
    /// Push the new color onto the head of the list, and remove any duplicate entries.
    /// Also trim the list size to 24 items, which is the size of the grid.
    pub fn add_recent_color(&mut self, color: Color) {
        if let Some(index) = self
            .recent_colors
            .iter()
            .position(|existing| *existing == color)
        {
            self.recent_colors.remove(index);
        }

        self.recent_colors.insert(0, color);
        self.recent_colors.truncate(RECENT_COLORS_COUNT);
    }
}

/// A button which displays a color swatch; when clicked, it displays a popup containing
/// a color picker.
///
/// This is spawnable by inheriting it as a "scene component" with optional
/// [`FeathersColorInputProps`].
///
/// The picker contains a number of user preferences which are sticky, and which
/// are stored on the [`ColorInputSettings`] resource. If the bevy settings plugin in installed,
/// these user preferences will be saved along with the user's settings.
#[derive(SceneComponent, Default, Clone, Reflect)]
#[reflect(Component, Clone, Default)]
#[scene(FeathersColorInputProps)]
#[require(ColorInputValue)]
pub struct FeathersColorInput;

/// Props used to construct a [`FeathersColorInput`] scene.
#[derive(Default)]
pub struct FeathersColorInputProps {
    /// Set a percentage of the swatch to display the opaque version of the
    /// current color.
    pub opaque_color_percentage: f32,
}

/// Component which stores references to the button swatch entity.
#[derive(Component, Clone, FromTemplate, Debug)]
struct ButtonEntityRefs(Entity);

/// Component which stores references to all the various internal widgets so that we don't have
/// to trawl the hierarchy looking for them.
#[derive(Component, Clone, Debug, FromTemplate)]
struct PopupEntityRefs {
    mode_rgb: Entity,
    mode_hsl: Entity,
    rg_plane: Entity,
    hs_plane: Entity,

    rgb_group: Entity,
    hsl_group: Entity,

    r_slider: Entity,
    r_input: Entity,
    g_slider: Entity,
    g_input: Entity,
    b_slider: Entity,
    b_input: Entity,

    h_slider: Entity,
    h_input: Entity,
    s_slider: Entity,
    s_input: Entity,
    l_slider: Entity,
    l_input: Entity,

    a_slider: Entity,
    a_input: Entity,

    hex_input_container: Entity,
    hex_input: Entity,
    swatch: Entity,
    recent: Entity,
}

/// Which color model is the current source of truth
#[derive(Default, Clone, Copy, Debug, PartialEq)]
enum SourceColorSpace {
    /// RGB
    #[default]
    Rgb,
    /// HSL
    Hsl,
}

/// Used to track the current color value and which color space we are editing. We don't use
/// the [`Color`] enum here for reasons explained below.
#[derive(Component, Clone, Debug, FromTemplate)]
struct ColorInputState {
    // Which color space is the source of truth
    source: SourceColorSpace,

    /// The current RGB color
    rgb: Srgba,

    /// The current HSL color
    hsl: Hsla,
}

impl ColorInputState {
    /// Synchronize the colors when switching color spaces.
    ///
    /// The color source is determined by which panel is shown, whether it's an RGB mode panel,
    /// an HSL mode panel, or any additional supported color sources that may be added later.
    /// When controls are interacted with, only the current source is updated; when we switch
    /// color sources, the new source is updated from the previous source.
    ///
    /// This maintains separate color values for each color space, rather than a single union, so
    /// that we can prevent lossy conversions when switching color spaces. For example, an RGB color
    /// that has no saturation (black, white or gray) has an indeterminate hue. We do this by only
    /// overwriting a color model's channels when they're well-defined in the source color,
    /// preserving the previous value otherwise.
    fn change_source(&mut self, next_source: SourceColorSpace) {
        if next_source != self.source {
            match self.source {
                SourceColorSpace::Rgb => {
                    let hsl = Hsla::from(self.rgb);

                    // Copy `hsl` into `self.hsl`, but leave indeterminate channels unchanged from
                    // the previous value. We only adopt a channel when the channels it depends on
                    // are well-defined; otherwise we keep the old values.
                    const EPSILON: f32 = 1e-6;

                    // Lightness and alpha are always well-defined.
                    self.hsl.lightness = hsl.lightness;
                    self.hsl.alpha = hsl.alpha;

                    // Saturation is indeterminate for pure black and pure white.
                    if hsl.lightness > EPSILON && hsl.lightness < 1.0 - EPSILON {
                        self.hsl.saturation = hsl.saturation;

                        // Hue is indeterminate for grays (zero saturation).
                        if hsl.saturation > EPSILON {
                            self.hsl.hue = hsl.hue;
                        }
                    }
                }

                SourceColorSpace::Hsl => {
                    // Convert HSL to RGB
                    self.rgb = Srgba::from(self.hsl);
                }
            }
            self.source = next_source;
        }
    }

    /// Set a single channel, returning the resulting color in the current source space.
    fn set_channel(&mut self, channel: ColorChannel, value: f32) -> Color {
        match channel {
            ColorChannel::Red => {
                self.rgb.red = value;
                self.rgb.into()
            }
            ColorChannel::Green => {
                self.rgb.green = value;
                self.rgb.into()
            }
            ColorChannel::Blue => {
                self.rgb.blue = value;
                self.rgb.into()
            }
            ColorChannel::HslHue => {
                self.hsl.hue = value;
                self.hsl.into()
            }
            ColorChannel::HslSaturation => {
                self.hsl.saturation = value;
                self.hsl.into()
            }
            ColorChannel::HslLightness => {
                self.hsl.lightness = value;
                self.hsl.into()
            }
            ColorChannel::Alpha => match self.source {
                SourceColorSpace::Rgb => {
                    self.rgb.alpha = value;
                    self.rgb.into()
                }
                SourceColorSpace::Hsl => {
                    self.hsl.alpha = value;
                    self.hsl.into()
                }
            },
        }
    }

    /// Return a [`Color`] representing the current source color.
    fn to_color(&self) -> Color {
        match self.source {
            SourceColorSpace::Rgb => Color::from(self.rgb),
            SourceColorSpace::Hsl => Color::from(self.hsl),
        }
    }

    /// Return the alpha component of the current source color
    fn to_alpha(&self) -> f32 {
        match self.source {
            SourceColorSpace::Rgb => self.rgb.alpha,
            SourceColorSpace::Hsl => self.hsl.alpha,
        }
    }
}

/// Marks the number input so that we know what channel it's editing.
#[derive(Component, Default, Clone)]
struct NumberInputChannel(ColorChannel);

impl FeathersColorInput {
    fn scene(props: FeathersColorInputProps) -> impl Scene {
        let popup: Arc<dyn Fn() -> Box<dyn Scene> + Sync + Send> = Arc::new(color_input_popup);
        bsn! {
            @FeathersLazyMenu { popup }
            ButtonEntityRefs(#swatch)
            ColorInputState
            Children [
                (
                    @FeathersMenuToolButton {
                        @caption: bsn! {
                            #swatch
                            @FeathersColorSwatch {
                                @opaque_color_percentage: {props.opaque_color_percentage},
                                @corners: RoundedCorners::None,
                            }
                            Node {
                                width: px(18),
                                min_width: px(18),
                                height: px(18),
                                flex_grow: 0.0,
                                align_self: AlignSelf::Center,
                                justify_self: JustifySelf::Start,
                            }
                         }
                    }
                )
            ]
        }
    }
}

// Lazily-constructed menu popup
fn color_input_popup() -> Box<dyn Scene> {
    Box::new(bsn!(
        @FeathersMenuPopup
        PopupEntityRefs {
            mode_rgb: #mode_rgb,
            mode_hsl: #mode_hsl,

            rg_plane: #rg_plane,
            hs_plane: #hs_plane,

            rgb_group: #rgb_group,
            hsl_group: #hsl_group,

            r_slider: #r_slider,
            r_input: #r_input,
            g_slider: #g_slider,
            g_input: #g_input,
            b_slider: #b_slider,
            b_input: #b_input,

            h_slider: #h_slider,
            h_input: #h_input,
            s_slider: #s_slider,
            s_input: #s_input,
            l_slider: #l_slider,
            l_input: #l_input,

            a_slider: #a_slider,
            a_input: #a_input,

            hex_input_container: #hex_input_container,
            hex_input: #hex_input,
            swatch: #swatch,
            recent: #recent,
        }
        Node {
            width: px(256 + 18), // room for 256 px wide plane widgets + padding/border
            row_gap: px(4),
            padding: px(4)
        }
        TabIndex
        on(popup_ready)
        on(popup_despawn)
        Popover {
            positions: vec![
                PopoverPlacement {
                    side: PopoverSide::Bottom,
                    align: PopoverAlign::End,
                    gap: 2.0,
                },
                PopoverPlacement {
                    side: PopoverSide::Bottom,
                    align: PopoverAlign::Start,
                    gap: 2.0,
                },
                PopoverPlacement {
                    side: PopoverSide::Right,
                    align: PopoverAlign::Center,
                    gap: 2.0,
                },
                PopoverPlacement {
                    side: PopoverSide::Left,
                    align: PopoverAlign::Center,
                    gap: 2.0,
                },
                PopoverPlacement {
                    side: PopoverSide::Top,
                    align: PopoverAlign::End,
                    gap: 2.0,
                },
                PopoverPlacement {
                    side: PopoverSide::Top,
                    align: PopoverAlign::Start,
                    gap: 2.0,
                },
            ],
            window_margin: 10.0,
        }
        Children [
            (
                Node {
                    display: Display::Flex,
                    flex_direction: FlexDirection::Row,
                    column_gap: px(1),
                }
                Children [
                    (
                        #mode_rgb
                        @FeathersButton {
                            @caption: bsn! { caption("RGB") },
                            @corners: RoundedCorners::Left,
                        }
                        Node {
                            flex_grow: 1.0,
                        }
                        ActivateOnPress
                        AccessibleLabel("RGB")
                        on(|_activate: On<Activate>, mut settings: ResMut<ColorInputSettings>| {
                            settings.mode = ColorInputMode::RGPlane;
                        })
                        AutoFocus
                    ),
                    (
                        #mode_hsl
                        @FeathersButton {
                            @caption: bsn! { caption("HSL") },
                            @corners: RoundedCorners::Right,
                        }
                        Node {
                            flex_grow: 1.0,
                        }
                        ActivateOnPress
                        AccessibleLabel("HSL")
                        on(|_activate: On<Activate>, mut settings: ResMut<ColorInputSettings>| {
                            settings.mode = ColorInputMode::HSPlane;
                        })
                    ),
                ]
            ),

            (
                #rg_plane
                @FeathersColorPlane::RedGreen
                Node {
                    width: px(256 + 8),
                    height: px(256 + 8),
                }
                on(rg_color_plane_value_change)
            ),

            (
                #hs_plane
                @FeathersColorPlane::HueSaturation
                Node {
                    width: px(256 + 8),
                    height: px(256 + 8),
                }
                on(hs_color_plane_value_change)
            ),

            #rgb_group
            Node {
                display: Display::Grid,
                column_gap: px(4),
                row_gap: px(4),
                grid_template_columns: vec![
                    RepeatedGridTrack::auto(1),
                    RepeatedGridTrack::flex(1, 1.),
                    RepeatedGridTrack::px(1, 50.),
                ],
                grid_auto_rows: vec![GridTrack::auto()],
                align_items: AlignItems::Center,
            }
            Children [
                label("R"),
                (
                    #r_slider
                    @FeathersColorSlider {
                        @value: 0.5,
                        @channel: ColorChannel::Red
                    }
                    AccessibleLabel("Red Channel")
                    on(color_slider_value_change)
                ),
                (
                    #r_input
                    @FeathersNumberInput
                    template_value(NumberInputValue::F32(0.0))
                    template_value(HardLimit(NumberInputRange::F32(0.0..=255.0)))
                    NumberInputPrecision(1)
                    NumberInputStep(20.0)
                    NumberInputChannel(ColorChannel::Red)
                    on(number_input_value_change)
                ),
                label("G"),
                (
                    #g_slider
                    @FeathersColorSlider {
                        @value: 0.5,
                        @channel: ColorChannel::Green
                    }
                    AccessibleLabel("Green Channel")
                    on(color_slider_value_change)
                ),
                (
                    #g_input
                    @FeathersNumberInput
                    template_value(NumberInputValue::F32(0.0))
                    template_value(HardLimit(NumberInputRange::F32(0.0..=255.0)))
                    NumberInputPrecision(1)
                    NumberInputStep(20.0)
                    NumberInputChannel(ColorChannel::Green)
                    on(number_input_value_change)
                ),
                label("B"),
                (
                    #b_slider
                    @FeathersColorSlider {
                        @value: 0.5,
                        @channel: ColorChannel::Blue
                    }
                    AccessibleLabel("Blue Channel")
                    on(color_slider_value_change)
                ),
                (
                    #b_input
                    @FeathersNumberInput
                    template_value(NumberInputValue::F32(0.0))
                    template_value(HardLimit(NumberInputRange::F32(0.0..=255.0)))
                    NumberInputPrecision(1)
                    NumberInputStep(20.0)
                    NumberInputChannel(ColorChannel::Blue)
                    on(number_input_value_change)
                ),
            ],

            #hsl_group
            Node {
                display: Display::Grid,
                column_gap: px(4),
                row_gap: px(4),
                grid_template_columns: vec![
                    RepeatedGridTrack::auto(1),
                    RepeatedGridTrack::flex(1, 1.),
                    RepeatedGridTrack::px(1, 50.),
                ],
                grid_auto_rows: vec![GridTrack::auto()],
                align_items: AlignItems::Center,
            }
            Children [
                label("H"),
                (
                    #h_slider
                    @FeathersColorSlider {
                        @value: 0.5,
                        @channel: ColorChannel::HslHue
                    }
                    AccessibleLabel("Hue Channel")
                    on(color_slider_value_change)
                ),
                (
                    #h_input
                    @FeathersNumberInput
                    template_value(NumberInputValue::F32(0.0))
                    template_value(HardLimit(NumberInputRange::F32(0.0..=360.0)))
                    NumberInputPrecision(1)
                    NumberInputStep(30.0)
                    NumberInputChannel(ColorChannel::HslHue)
                    Node {
                        flex_grow: 1.0,
                    }
                    on(number_input_value_change)
                ),
                label("S"),
                (
                    #s_slider
                    @FeathersColorSlider {
                        @value: 0.5,
                        @channel: ColorChannel::HslSaturation
                    }
                    AccessibleLabel("Saturation Channel")
                    on(color_slider_value_change)
                ),
                (
                    #s_input
                    @FeathersNumberInput
                    template_value(NumberInputValue::F32(0.0))
                    template_value(HardLimit(NumberInputRange::F32(0.0..=100.0)))
                    NumberInputPrecision(1)
                    NumberInputStep(10.0)
                    NumberInputChannel(ColorChannel::HslSaturation)
                    Node {
                        flex_grow: 1.0,
                    }
                    on(number_input_value_change)
                ),
                label("L"),
                (
                    #l_slider
                    @FeathersColorSlider {
                        @value: 0.5,
                        @channel: ColorChannel::HslLightness
                    }
                    AccessibleLabel("Lightness Channel")
                    on(color_slider_value_change)
                ),
                (
                    #l_input
                    @FeathersNumberInput
                    template_value(NumberInputValue::F32(0.0))
                    template_value(HardLimit(NumberInputRange::F32(0.0..=100.0)))
                    NumberInputPrecision(1)
                    NumberInputStep(10.0)
                    NumberInputChannel(ColorChannel::HslLightness)
                    Node {
                        flex_grow: 1.0,
                    }
                    on(number_input_value_change)
                ),
            ],

            Node {
                display: Display::Grid,
                column_gap: px(4),
                row_gap: px(4),
                grid_template_columns: vec![
                    RepeatedGridTrack::auto(1),
                    RepeatedGridTrack::flex(1, 1.),
                    RepeatedGridTrack::px(1, 50.),
                ],
                grid_auto_rows: vec![GridTrack::auto()],
                align_items: AlignItems::Center,
            }
            Children [
                label("A"),
                (
                    #a_slider
                    @FeathersColorSlider {
                        @value: 0.5,
                        @channel: ColorChannel::Alpha
                    }
                    AccessibleLabel("Alpha Channel")
                    on(color_slider_value_change)
                ),
                (
                    #a_input
                    @FeathersNumberInput
                    template_value(NumberInputValue::F32(0.0))
                    template_value(HardLimit(NumberInputRange::F32(0.0..=255.0)))
                    NumberInputPrecision(1)
                    NumberInputStep(10.0)
                    NumberInputChannel(ColorChannel::Alpha)
                    on(number_input_value_change)
                ),
                (
                    #hex_input_container
                    @FeathersTextInputContainer
                    Node {
                        flex_grow: 0.
                        padding: { px(4).left() },
                        grid_column: GridPlacement::span(2),
                    }
                    Children [
                        (
                            #hex_input
                            @FeathersTextInput {
                                @max_characters: 9usize,
                            }
                            TextLayout {
                                justify: Justify::Center,
                            }
                            InheritableFont {
                                font: fonts::MONO
                            }
                            template_value(LineHeight::Px(24.0)) // TODO: Make const for this
                            on(hex_input_on_enter_key)
                            on(hex_input_on_focus_loss)
                        )
                    ]
                ),
                (
                    #swatch
                    @FeathersColorSwatch {
                        @opaque_color_percentage: 50.0,
                    }
                    Node {
                        flex_grow: 0.0,
                        align_self: AlignSelf::Stretch,
                    }
                )
            ],

            // Recent colors
            #recent
            @FeathersColorSwatchGrid {
                size: UVec2::new(RECENT_COLORS_COLUMNS, RECENT_COLORS_ROWS),
                opaque_color_percentage: 50.0,
            }
            on(recent_color_selected)
        ]
    ))
}

fn rg_color_plane_value_change(
    change: On<ValueChange<Vec2>>,
    q_parent: Query<&ChildOf>,
    mut q_state: Query<&mut ColorInputState>,
    mut commands: Commands,
) {
    if let Some((root_id, mut state)) = color_input_state(&q_parent, &mut q_state, change.source) {
        state.rgb.red = change.value.x;
        state.rgb.green = change.value.y;
        let value: Color = state.rgb.into();
        commands.trigger(ValueChange {
            source: root_id,
            value,
            is_final: change.is_final,
        });
    }
}

fn hs_color_plane_value_change(
    change: On<ValueChange<Vec2>>,
    q_parent: Query<&ChildOf>,
    mut q_state: Query<&mut ColorInputState>,
    mut commands: Commands,
) {
    if let Some((root_id, mut state)) = color_input_state(&q_parent, &mut q_state, change.source) {
        state.hsl.hue = change.value.x * 360.0;
        state.hsl.saturation = change.value.y;
        let value: Color = state.hsl.into();
        commands.trigger(ValueChange {
            source: root_id,
            value,
            is_final: change.is_final,
        });
    }
}

fn color_slider_value_change(
    change: On<ValueChange<f32>>,
    q_slider: Query<&ColorSlider>,
    q_parent: Query<&ChildOf>,
    mut q_state: Query<&mut ColorInputState>,
    mut commands: Commands,
) {
    let Ok(slider) = q_slider.get(change.source) else {
        return;
    };

    if let Some((root_id, mut state)) = color_input_state(&q_parent, &mut q_state, change.source) {
        let value = state.set_channel(slider.channel, change.value);
        commands.trigger(ValueChange {
            source: root_id,
            value,
            is_final: change.is_final,
        });
    }
}

fn number_input_value_change(
    change: On<ValueChange<f32>>,
    q_input: Query<&NumberInputChannel>,
    q_parent: Query<&ChildOf>,
    mut q_state: Query<&mut ColorInputState>,
    mut commands: Commands,
) {
    let Ok(channel) = q_input.get(change.source) else {
        return;
    };

    if let Some((root_id, mut state)) = color_input_state(&q_parent, &mut q_state, change.source) {
        let value = state.set_channel(channel.0, change.value / display_scale(channel.0));
        commands.trigger(ValueChange {
            source: root_id,
            value,
            is_final: change.is_final,
        });
    }
}

fn hex_input_on_enter_key(
    key_input: On<FocusedInput<KeyboardInput>>,
    q_text_input: Query<&EditableText>,
    q_parent: Query<&ChildOf>,
    mut q_state: Query<&mut ColorInputState>,
    mut commands: Commands,
) {
    if key_input.input.key_code != KeyCode::Enter {
        return;
    }

    let Ok(editable_text) = q_text_input.get(key_input.event_target()) else {
        return;
    };

    if let Some((root_id, mut state)) =
        color_input_state(&q_parent, &mut q_state, key_input.event_target())
    {
        hex_input_value_change(root_id, &mut state, editable_text, &mut commands);
    }
}

fn hex_input_on_focus_loss(
    focus_lost: On<FocusLost>,
    q_text_input: Query<&EditableText>,
    q_parent: Query<&ChildOf>,
    mut q_state: Query<&mut ColorInputState>,
    mut commands: Commands,
) {
    let Ok(editable_text) = q_text_input.get(focus_lost.event_target()) else {
        return;
    };

    if let Some((root_id, mut state)) =
        color_input_state(&q_parent, &mut q_state, focus_lost.event_target())
    {
        hex_input_value_change(root_id, &mut state, editable_text, &mut commands);
    }
}

fn hex_input_value_change(
    root_id: Entity,
    state: &mut ColorInputState,
    editable_text: &EditableText,
    commands: &mut Commands,
) {
    if let Ok(new_rgb_color) = Srgba::hex(editable_text.value().to_string())
        && state.rgb != new_rgb_color
    {
        state.rgb = new_rgb_color;
        commands.trigger(ValueChange {
            source: root_id,
            value: Color::from(new_rgb_color),
            is_final: true,
        });
    }
    // TODO: We currently have no way to report errors, so silently
    // fail for now.
}

fn color_input_value_change(
    mut q_input: Query<
        (
            &ColorInputValue,
            &mut ColorInputState,
            &ButtonEntityRefs,
            &Children,
        ),
        Changed<ColorInputValue>,
    >,
    q_popup: Query<&PopupEntityRefs, With<FeathersMenuPopup>>,
    mut q_color_plane: Query<&mut ColorPlaneValue>,
    mut q_editable_text: Query<&mut EditableText>,
    mut grid_update: ColorSwatchGridUpdate,
    settings: Res<ColorInputSettings>,
    mut commands: Commands,
) {
    for (&ColorInputValue(value), mut state, refs, children) in q_input.iter_mut() {
        commands.entity(refs.0).insert(ColorSwatchValue(value));

        // Update either the RGB or HSL value depending on what mode we are in.
        match state.source {
            SourceColorSpace::Rgb => state.rgb = value.into(),
            SourceColorSpace::Hsl => state.hsl = value.into(),
        }

        if let Some(refs) = children
            .iter()
            .find_map(|child_id| q_popup.get(*child_id).ok())
        {
            update_controls(
                &mut q_color_plane,
                &mut q_editable_text,
                &mut commands,
                refs,
                &state,
            );

            grid_update.update(refs.recent, &settings.recent_colors, Some(value));
        }
    }
}

/// Re-emit a recent color selection from the color input itself, so that it's indistinguishable
/// from any other edit.
fn recent_color_selected(
    change: On<ValueChange<Color>>,
    q_parent: Query<&ChildOf>,
    q_state: Query<&ColorInputState>,
    mut commands: Commands,
) {
    if let Some(root_id) = q_parent
        .iter_ancestors(change.source)
        .find(|e| q_state.contains(*e))
    {
        commands.trigger(ValueChange {
            source: root_id,
            value: change.value,
            is_final: true,
        });
    }
}

fn update_mode_selector(
    q_refs: Query<(Entity, &PopupEntityRefs)>,
    q_parent: Query<&ChildOf>,
    mut q_state: Query<&mut ColorInputState>,
    mut q_button: Query<&mut ButtonVariant>,
    mut q_node: Query<&mut Node>,
    mut q_color_plane: Query<&mut ColorPlaneValue>,
    mut q_editable_text: Query<&mut EditableText>,
    settings: Res<ColorInputSettings>,
    mut focus: ResMut<InputFocus>,
    mut commands: Commands,
) {
    for (popup_id, refs) in q_refs.iter() {
        set_mode_selector(refs, &mut q_button, settings.mode);
        set_pane_visible(refs, &mut q_node, settings.mode, &mut commands);

        if settings.is_changed()
            && let Some((_, mut state)) = color_input_state(&q_parent, &mut q_state, popup_id)
        {
            // Make sure that the right color model is designated as the source.
            // Also, ensure that focus moves to a widget that is not about to be hidden,
            // as this will auto-close the popup.
            match settings.mode {
                ColorInputMode::RGPlane => {
                    state.change_source(SourceColorSpace::Rgb);
                    focus.set(refs.mode_rgb, FocusCause::Auto);
                }
                ColorInputMode::HSPlane => {
                    state.change_source(SourceColorSpace::Hsl);
                    focus.set(refs.mode_hsl, FocusCause::Auto);
                }
            }

            update_controls(
                &mut q_color_plane,
                &mut q_editable_text,
                &mut commands,
                refs,
                &state,
            );
        }
    }
}

fn set_mode_selector(
    refs: &PopupEntityRefs,
    q_button: &mut Query<&mut ButtonVariant>,
    mode: ColorInputMode,
) {
    if let Ok(mut rgb_variant) = q_button.get_mut(refs.mode_rgb) {
        rgb_variant.set_if_neq(if mode == ColorInputMode::RGPlane {
            ButtonVariant::Primary
        } else {
            ButtonVariant::Normal
        });
    }

    if let Ok(mut hsl_variant) = q_button.get_mut(refs.mode_hsl) {
        hsl_variant.set_if_neq(if mode == ColorInputMode::HSPlane {
            ButtonVariant::Primary
        } else {
            ButtonVariant::Normal
        });
    }
}

fn set_pane_visible(
    refs: &PopupEntityRefs,
    q_node: &mut Query<&mut Node>,
    mode: ColorInputMode,
    commands: &mut Commands,
) {
    set_node_visible(
        q_node,
        refs.rg_plane,
        if mode == ColorInputMode::RGPlane {
            Display::Flex
        } else {
            Display::None
        },
        commands,
    );
    set_node_visible(
        q_node,
        refs.rgb_group,
        if mode == ColorInputMode::RGPlane {
            Display::Grid
        } else {
            Display::None
        },
        commands,
    );
    set_node_visible(
        q_node,
        refs.hex_input_container,
        if mode == ColorInputMode::RGPlane {
            Display::Flex
        } else {
            Display::None
        },
        commands,
    );
    // The swatch fills the row the hex input vacates.
    if let Ok(mut swatch_node) = q_node.get_mut(refs.swatch) {
        let span = if mode == ColorInputMode::RGPlane {
            GridPlacement::auto()
        } else {
            GridPlacement::span(3)
        };
        if swatch_node.grid_column != span {
            swatch_node.grid_column = span;
        }
    }
    set_node_visible(
        q_node,
        refs.hs_plane,
        if mode == ColorInputMode::HSPlane {
            Display::Flex
        } else {
            Display::None
        },
        commands,
    );
    set_node_visible(
        q_node,
        refs.hsl_group,
        if mode == ColorInputMode::HSPlane {
            Display::Grid
        } else {
            Display::None
        },
        commands,
    );
}

fn set_node_visible(
    q_node: &mut Query<&mut Node>,
    id: Entity,
    display: Display,
    commands: &mut Commands,
) {
    if let Ok(mut node) = q_node.get_mut(id)
        && node.display != display
    {
        node.display = display;

        // Also set visibility to prevent tab navigation
        commands.entity(id).insert(if display == Display::None {
            Visibility::Hidden
        } else {
            Visibility::Inherited
        });
    }
}

/// Record the edited color in the recent colors palette when the popup closes.
///
/// The popup is despawned by every path which closes it, including ESC, so this is also the
/// commit point for a cancelled edit. That's deliberate: the picker has no commit button, and
/// the recent colors list is a convenience rather than part of the edited document.
fn popup_despawn(
    despawn: On<Despawn<PopupEntityRefs>>,
    q_popup: Query<&ChildOf, With<PopupEntityRefs>>,
    q_color_input: Query<&ColorInputValue>,
    mut settings: ResMut<ColorInputSettings>,
) {
    let Ok(parent) = q_popup.get(despawn.entity) else {
        return;
    };

    // The color input is gone too if the whole widget is being torn down.
    let Ok(ColorInputValue(value)) = q_color_input.get(parent.get()) else {
        return;
    };

    settings.add_recent_color(*value);
}

fn popup_ready(
    ready: On<Ready>,
    q_popup: Query<(&PopupEntityRefs, &ChildOf)>,
    mut q_color_input: Query<(&ColorInputValue, &mut ColorInputState)>,
    mut q_editable_text: Query<&mut EditableText>,
    mut q_color_plane: Query<&mut ColorPlaneValue>,
    mut grid_update: ColorSwatchGridUpdate,
    settings: Res<ColorInputSettings>,
    mut commands: Commands,
) {
    let Ok((refs, parent)) = q_popup.get(ready.entity) else {
        return;
    };

    let Ok((ColorInputValue(value), mut state)) = q_color_input.get_mut(parent.get()) else {
        warn!("Could not locate popup parent");
        return;
    };

    match settings.mode {
        ColorInputMode::RGPlane => {
            state.source = SourceColorSpace::Rgb;
            state.rgb = (*value).into();
        }
        ColorInputMode::HSPlane => {
            state.source = SourceColorSpace::Hsl;
            state.hsl = (*value).into();
        }
    }

    update_controls(
        &mut q_color_plane,
        &mut q_editable_text,
        &mut commands,
        refs,
        &state,
    );

    grid_update.update(refs.recent, &settings.recent_colors, Some(*value));
}

fn update_controls(
    q_color_plane: &mut Query<'_, '_, &mut ColorPlaneValue>,
    q_editable_text: &mut Query<'_, '_, &mut EditableText>,
    commands: &mut Commands<'_, '_>,
    refs: &PopupEntityRefs,
    state: &ColorInputState,
) {
    let color = state.to_color();

    if let Ok(mut color_plane_value) = q_color_plane.get_mut(refs.rg_plane) {
        color_plane_value.set_if_neq(ColorPlaneValue(Vec3::new(
            state.rgb.red,
            state.rgb.green,
            state.rgb.blue,
        )));
    }

    if let Ok(mut color_plane_value) = q_color_plane.get_mut(refs.hs_plane) {
        color_plane_value.set_if_neq(ColorPlaneValue(Vec3::new(
            state.hsl.hue / 360.0,
            state.hsl.saturation,
            0.5,
        )));
    }

    commands
        .entity(refs.r_slider)
        .insert(SliderValue(state.rgb.red))
        .insert(SliderBaseColor(color));
    commands
        .entity(refs.g_slider)
        .insert(SliderValue(state.rgb.green))
        .insert(SliderBaseColor(color));
    commands
        .entity(refs.b_slider)
        .insert(SliderValue(state.rgb.blue))
        .insert(SliderBaseColor(color));
    commands
        .entity(refs.a_slider)
        .insert(SliderValue(state.to_alpha()))
        .insert(SliderBaseColor(color));
    commands
        .entity(refs.h_slider)
        .insert(SliderValue(state.hsl.hue))
        .insert(SliderBaseColor(color));
    commands
        .entity(refs.s_slider)
        .insert(SliderValue(state.hsl.saturation))
        .insert(SliderBaseColor(color));
    commands
        .entity(refs.l_slider)
        .insert(SliderValue(state.hsl.lightness))
        .insert(SliderBaseColor(color));

    // Round to nearest tenth, so that the string of digits
    // won't be too long to display in the limited space.
    fn scaled_number_input_value(channel: ColorChannel, native: f32) -> NumberInputValue {
        NumberInputValue::F32((native * display_scale(channel) * 10.0).round() / 10.0)
    }

    commands
        .entity(refs.r_input)
        .insert(scaled_number_input_value(ColorChannel::Red, state.rgb.red));
    commands
        .entity(refs.g_input)
        .insert(scaled_number_input_value(
            ColorChannel::Green,
            state.rgb.green,
        ));
    commands
        .entity(refs.b_input)
        .insert(scaled_number_input_value(
            ColorChannel::Blue,
            state.rgb.blue,
        ));

    commands
        .entity(refs.h_input)
        .insert(scaled_number_input_value(
            ColorChannel::HslHue,
            state.hsl.hue,
        ));
    commands
        .entity(refs.s_input)
        .insert(scaled_number_input_value(
            ColorChannel::HslSaturation,
            state.hsl.saturation,
        ));
    commands
        .entity(refs.l_input)
        .insert(scaled_number_input_value(
            ColorChannel::HslLightness,
            state.hsl.lightness,
        ));

    commands
        .entity(refs.a_input)
        .insert(scaled_number_input_value(
            ColorChannel::Alpha,
            state.to_alpha(),
        ));

    // Update the color swatch
    commands.entity(refs.swatch).insert(ColorSwatchValue(color));

    // Update the hex input
    if let Ok(mut editable_text) = q_editable_text.get_mut(refs.hex_input) {
        let hex_value = state.rgb.to_hex();
        if editable_text.value() != hex_value.as_str() {
            editable_text.queue_edit(TextEdit::SelectAll);
            editable_text.queue_edit(TextEdit::Insert(hex_value.into()));
        }
    }
}

/// Helper function used to find the [`ColorInputState`] component in the ancestor chain.
fn color_input_state<'a>(
    q_parent: &Query<&ChildOf>,
    q_state: &'a mut Query<&mut ColorInputState>,
    from: Entity,
) -> Option<(Entity, Mut<'a, ColorInputState>)> {
    let root_id = q_parent
        .iter_ancestors(from)
        .find(|e| q_state.contains(*e))?;
    let state = q_state.get_mut(root_id).ok()?;
    Some((root_id, state))
}

/// Scale between a number input's display units and the channel's native range.
///
/// Note that RGBA sliders go from 0..=255 (inclusive). The multiplier
/// is arbitrary since the representation is f32, but CSS users are familiar
/// with 8-bit colors.
fn display_scale(channel: ColorChannel) -> f32 {
    match channel {
        ColorChannel::Red | ColorChannel::Green | ColorChannel::Blue | ColorChannel::Alpha => 255.0,
        ColorChannel::HslSaturation | ColorChannel::HslLightness => 100.0,
        ColorChannel::HslHue => 1.0,
    }
}

/// Plugin which registers the observers for updating the swatch color.
pub struct ColorInputPlugin;

impl Plugin for ColorInputPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        app.init_resource::<ColorInputSettings>();
        app.add_systems(
            PostUpdate,
            (update_mode_selector, color_input_value_change).chain(),
        );
    }
}

/// Observer function which updates the color input value in response to a [`ValueChange`] event.
pub fn color_input_self_update(value_change: On<ValueChange<Color>>, mut commands: Commands) {
    commands
        .entity(value_change.source)
        .insert(ColorInputValue(value_change.value));
}
