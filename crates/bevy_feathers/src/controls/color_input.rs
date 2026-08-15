use alloc::sync::Arc;

use bevy_app::{Plugin, PostUpdate};
use bevy_color::{Alpha, Color, Hsla, Srgba};
use bevy_ecs::{
    change_detection::DetectChangesMut,
    component::Component,
    entity::Entity,
    event::EntityEvent,
    hierarchy::{ChildOf, Children},
    observer::On,
    query::{Changed, With},
    reflect::ReflectComponent,
    relationship::Relationship,
    resource::Resource,
    system::{Commands, Query, Res, ResMut},
    template::FromTemplate,
};
use bevy_input::keyboard::{KeyCode, KeyboardInput};
use bevy_input_focus::{tab_navigation::TabIndex, AutoFocus, FocusLost, FocusedInput};
use bevy_log::warn;
use bevy_math::{Vec2, Vec3};
use bevy_reflect::{prelude::ReflectDefault, Reflect};
use bevy_scene::{prelude::*, Ready};
use bevy_text::{EditableText, Justify, TextEdit, TextLayout};
use bevy_ui::{
    prelude::AccessibleLabel, px, AlignItems, AlignSelf, Display, FlexDirection, GridPlacement,
    GridTrack, JustifySelf, Node, RepeatedGridTrack, UiRect,
};
use bevy_ui_widgets::{
    popover::{Popover, PopoverAlign, PopoverPlacement, PopoverSide},
    Activate, ActivateOnPress, SliderValue, ValueChange,
};

use crate::{
    constants::fonts,
    controls::{
        ButtonVariant, ColorChannel, ColorPlaneValue, ColorSlider, ColorSwatchValue,
        FeathersButton, FeathersColorPlane, FeathersColorSlider, FeathersColorSwatch,
        FeathersLazyMenu, FeathersMenuPopup, FeathersMenuToolButton, FeathersNumberInput,
        FeathersTextInput, FeathersTextInputContainer, HardLimit, NumberInputPrecision,
        NumberInputRange, NumberInputStep, NumberInputValue,
    },
    display::{caption, label},
    font_styles::InheritableFont,
    rounded_corners::RoundedCorners,
};

/// Component that contains the value of the color input.
#[derive(Component, Default, Clone, Reflect)]
#[reflect(Component, Clone, Default)]
#[component(immutable)]
pub struct ColorInputValue(pub Color);

/// Supported color editing modes
#[derive(Default, Clone, Copy, Reflect, PartialEq)]
pub enum ColorInputMode {
    /// Red/green/blue mode
    #[default]
    RGB,
    /// Hue/saturation/lightness mode
    HSL,
    /// Shows an array of recent colors
    Recent,
}

/// Resource that contains user preferences for the color input
#[derive(Resource, Default)]
pub struct ColorInputSettings {
    /// Which color space we're editing
    pub mode: ColorInputMode,
    /// List of recently edited colors
    pub recent_colors: Vec<Color>,
}

/// A color swatch widget.
///
/// This is spawnable by inheriting it as a "scene component" with optional
/// [`FeathersColorInputProps`].
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

/// Component which stores references to all the various internal widgets so that we don't have
/// to trawl the hierarchy looking for them.
#[derive(Component, Clone, FromTemplate, Debug)]
struct ButtonEntityRefs(Entity);

/// Component which stores references to all the various internal widgets so that we don't have
/// to trawl the hierarchy looking for them.
#[derive(Component, Clone, Debug, FromTemplate)]
struct PopupEntityRefs {
    mode_rgb: Entity,
    mode_hsl: Entity,
    mode_recent: Entity,
    rg_plane: Entity,
    // hs_plane: Entity,
    r_slider: Entity,
    r_input: Entity,
    g_slider: Entity,
    g_input: Entity,
    b_slider: Entity,
    b_input: Entity,
    a_slider: Entity,
    a_input: Entity,
    hex_input: Entity,
    swatch: Entity,
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
    // Note: because the color plane has a built-in margin, we don't want to put a padding or
    // column gap on the popup, but instead put margins on individual children.
    Box::new(bsn!(
        @FeathersMenuPopup
        PopupEntityRefs {
            mode_rgb: #mode_rgb,
            mode_hsl: #mode_hsl,
            mode_recent: #mode_recent,
            rg_plane: #rg_plane,
            r_slider: #r_slider,
            r_input: #r_input,
            g_slider: #g_slider,
            g_input: #g_input,
            b_slider: #b_slider,
            b_input: #b_input,
            a_slider: #a_slider,
            a_input: #a_input,
            hex_input: #hex_input,
            swatch: #swatch,
        }
        Node {
            width: px(256 + 8 + 8 + 2),
            row_gap: px(4),
            padding: px(4)
        }
        TabIndex
        on(handle_popup_ready)
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
                            settings.mode = ColorInputMode::RGB;
                        })
                        AutoFocus
                    ),
                    (
                        #mode_hsl
                        @FeathersButton {
                            @caption: bsn! { caption("HSL") },
                            @corners: RoundedCorners::None,
                        }
                        Node {
                            flex_grow: 1.0,
                        }
                        ActivateOnPress
                        AccessibleLabel("Center")
                        on(|_activate: On<Activate>, mut settings: ResMut<ColorInputSettings>| {
                            settings.mode = ColorInputMode::HSL;
                        })
                    ),
                    (
                        #mode_recent
                        @FeathersButton {
                            @caption: bsn! { caption("Recent") },
                            @variant: ButtonVariant::Primary,
                            @corners: RoundedCorners::Right,
                        }
                        Node {
                            flex_grow: 1.0,
                        }
                        ActivateOnPress
                        AccessibleLabel("Recent Colors")
                        on(|_activate: On<Activate>, mut settings: ResMut<ColorInputSettings>| {
                            settings.mode = ColorInputMode::Recent;
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
            Node {
                display: Display::Grid,
                column_gap: px(4),
                row_gap: px(4),
                grid_template_columns: vec![
                    RepeatedGridTrack::auto(1),
                    RepeatedGridTrack::flex(1, 1.),
                    RepeatedGridTrack::px(1, 50.),
                ],
                grid_auto_rows: vec![GridTrack::px(24.0)],
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
                    NumberInputStep(10.0)
                    NumberInputChannel(ColorChannel::Red)
                    Node {
                        flex_grow: 1.0,
                    }
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
                    NumberInputStep(10.0)
                    NumberInputChannel(ColorChannel::Green)
                    Node {
                        flex_grow: 1.0,
                    }
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
                    NumberInputStep(10.0)
                    NumberInputChannel(ColorChannel::Blue)
                    Node {
                        flex_grow: 1.0,
                    }
                    on(number_input_value_change)
                ),
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
                    Node {
                        flex_grow: 1.0,
                    }
                    on(number_input_value_change)
                ),
                (
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
                            Node {
                                margin: UiRect {
                                    top: px(4),
                                    left: px(4),
                                    bottom: px(0),
                                    right: px(4),
                                }
                            }
                            InheritableFont {
                                font: fonts::MONO
                            }
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
        ]
    ))
}

fn rg_color_plane_value_change(
    change: On<ValueChange<Vec2>>,
    q_parent: Query<&ChildOf>,
    q_value: Query<(Entity, &ColorInputValue)>,
    mut commands: Commands,
) {
    if let Some((root_id, ColorInputValue(color))) = q_parent
        .iter_ancestors(change.source)
        .find_map(|e| q_value.get(e).ok())
    {
        let mut rgb = color.to_srgba();
        rgb.red = change.value.x;
        rgb.green = change.value.y;
        let value: Color = rgb.into();
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
    q_value: Query<(Entity, &ColorInputValue)>,
    mut commands: Commands,
) {
    let Some(slider) = q_slider.get(change.source).ok() else {
        return;
    };

    if let Some((root_id, ColorInputValue(color))) = q_parent
        .iter_ancestors(change.source)
        .find_map(|e| q_value.get(e).ok())
    {
        let new_value: Color = match slider.channel {
            ColorChannel::Red => {
                let mut rgb = color.to_srgba();
                rgb.red = change.value;
                rgb.into()
            }
            ColorChannel::Green => {
                let mut rgb = color.to_srgba();
                rgb.green = change.value;
                rgb.into()
            }
            ColorChannel::Blue => {
                let mut rgb = color.to_srgba();
                rgb.blue = change.value;
                rgb.into()
            }
            ColorChannel::HslHue => {
                let mut hsl: Hsla = (*color).into();
                hsl.hue = change.value;
                hsl.into()
            }
            ColorChannel::HslSaturation => {
                let mut hsl: Hsla = (*color).into();
                hsl.saturation = change.value;
                hsl.into()
            }
            ColorChannel::HslLightness => {
                let mut hsl: Hsla = (*color).into();
                hsl.lightness = change.value;
                hsl.into()
            }
            ColorChannel::Alpha => color.with_alpha(change.value),
        };
        commands.trigger(ValueChange {
            source: root_id,
            value: new_value,
            is_final: change.is_final,
        });
    }
}

fn number_input_value_change(
    change: On<ValueChange<f32>>,
    q_input: Query<&NumberInputChannel>,
    q_parent: Query<&ChildOf>,
    q_value: Query<(Entity, &ColorInputValue)>,
    mut commands: Commands,
) {
    let Some(channel) = q_input.get(change.source).ok() else {
        return;
    };

    if let Some((root_id, ColorInputValue(color))) = q_parent
        .iter_ancestors(change.source)
        .find_map(|e| q_value.get(e).ok())
    {
        let new_value: Color = match channel.0 {
            ColorChannel::Red => {
                let mut rgb = color.to_srgba();
                rgb.red = change.value / 255.0;
                rgb.into()
            }
            ColorChannel::Green => {
                let mut rgb = color.to_srgba();
                rgb.green = change.value / 255.0;
                rgb.into()
            }
            ColorChannel::Blue => {
                let mut rgb = color.to_srgba();
                rgb.blue = change.value / 255.0;
                rgb.into()
            }
            ColorChannel::HslHue => {
                let mut hsl: Hsla = (*color).into();
                hsl.hue = change.value;
                hsl.into()
            }
            ColorChannel::HslSaturation => {
                let mut hsl: Hsla = (*color).into();
                hsl.saturation = change.value;
                hsl.into()
            }
            ColorChannel::HslLightness => {
                let mut hsl: Hsla = (*color).into();
                hsl.lightness = change.value;
                hsl.into()
            }
            ColorChannel::Alpha => color.with_alpha(change.value / 255.0),
        };
        commands.trigger(ValueChange {
            source: root_id,
            value: new_value,
            is_final: change.is_final,
        });
    }
}

fn hex_input_on_enter_key(
    key_input: On<FocusedInput<KeyboardInput>>,
    q_text_input: Query<&EditableText>,
    q_parent: Query<&ChildOf>,
    q_value: Query<(Entity, &ColorInputValue)>,
    mut commands: Commands,
) {
    if key_input.input.key_code != KeyCode::Enter {
        return;
    }

    let Some(editable_text) = q_text_input.get(key_input.event_target()).ok() else {
        return;
    };

    let Some((root_id, ColorInputValue(color))) = q_parent
        .iter_ancestors(key_input.event_target())
        .find_map(|e| q_value.get(e).ok())
    else {
        return;
    };

    hex_input_value_change(root_id, *color, editable_text, &mut commands);
}

fn hex_input_on_focus_loss(
    focus_lost: On<FocusLost>,
    q_text_input: Query<&EditableText>,
    q_parent: Query<&ChildOf>,
    q_value: Query<(Entity, &ColorInputValue)>,
    mut commands: Commands,
) {
    let Some(editable_text) = q_text_input.get(focus_lost.event_target()).ok() else {
        return;
    };

    let Some((root_id, ColorInputValue(color))) = q_parent
        .iter_ancestors(focus_lost.event_target())
        .find_map(|e| q_value.get(e).ok())
    else {
        return;
    };

    hex_input_value_change(root_id, *color, editable_text, &mut commands);
}

fn hex_input_value_change(
    root_id: Entity,
    current_color: Color,
    editable_text: &EditableText,
    commands: &mut Commands,
) {
    if let Ok(new_rgb_color) = Srgba::hex(editable_text.value().to_string()) {
        let new_color: Color = new_rgb_color.into();
        if new_color != current_color {
            commands.trigger(ValueChange {
                source: root_id,
                value: new_color,
                is_final: true,
            });
        }
    }
}

fn color_input_value_change(
    q_input: Query<(&ColorInputValue, &ButtonEntityRefs, &Children), Changed<ColorInputValue>>,
    q_popup: Query<&PopupEntityRefs, With<FeathersMenuPopup>>,
    mut q_color_plane: Query<&mut ColorPlaneValue>,
    mut q_editable_text: Query<&mut EditableText>,
    mut commands: Commands,
) {
    for (&ColorInputValue(value), refs, children) in q_input.iter() {
        commands.entity(refs.0).insert(ColorSwatchValue(value));

        if let Some(refs) = children
            .iter()
            .find_map(|child_id| q_popup.get(*child_id).ok())
        {
            update_controls(
                &mut q_color_plane,
                &mut q_editable_text,
                &mut commands,
                refs,
                &value,
            );
        }
    }
}

fn update_mode_selector(
    q_refs: Query<&PopupEntityRefs>,
    mut q_button: Query<&mut ButtonVariant>,
    settings: Res<ColorInputSettings>,
) {
    for refs in q_refs.iter() {
        set_mode_selector(refs, &mut q_button, &settings);
    }
}

fn set_mode_selector(
    refs: &PopupEntityRefs,
    q_button: &mut Query<&mut ButtonVariant>,
    settings: &ColorInputSettings,
) {
    if let Ok(mut rgb_variant) = q_button.get_mut(refs.mode_rgb) {
        rgb_variant.set_if_neq(if settings.mode == ColorInputMode::RGB {
            ButtonVariant::Primary
        } else {
            ButtonVariant::Normal
        });
    }

    if let Ok(mut hsl_variant) = q_button.get_mut(refs.mode_hsl) {
        hsl_variant.set_if_neq(if settings.mode == ColorInputMode::HSL {
            ButtonVariant::Primary
        } else {
            ButtonVariant::Normal
        });
    }

    if let Ok(mut recent_variant) = q_button.get_mut(refs.mode_recent) {
        recent_variant.set_if_neq(if settings.mode == ColorInputMode::Recent {
            ButtonVariant::Primary
        } else {
            ButtonVariant::Normal
        });
    }
}

fn handle_popup_ready(
    insert: On<Ready>,
    q_popup: Query<(&PopupEntityRefs, &ChildOf)>,
    q_color_input: Query<&ColorInputValue>,
    mut q_editable_text: Query<&mut EditableText>,
    mut q_color_plane: Query<&mut ColorPlaneValue>,
    mut commands: Commands,
) {
    let Ok((refs, parent)) = q_popup.get(insert.entity) else {
        return;
    };

    let Ok(ColorInputValue(value)) = q_color_input.get(parent.get()) else {
        warn!("Could not locate popup parent");
        return;
    };

    update_controls(
        &mut q_color_plane,
        &mut q_editable_text,
        &mut commands,
        refs,
        value,
    );
}

fn update_controls(
    q_color_plane: &mut Query<'_, '_, &mut ColorPlaneValue>,
    q_editble_text: &mut Query<'_, '_, &mut EditableText>,
    commands: &mut Commands<'_, '_>,
    refs: &PopupEntityRefs,
    value: &Color,
) {
    let rgb: Srgba = value.to_srgba();

    if let Ok(mut color_plane_value) = q_color_plane.get_mut(refs.rg_plane) {
        color_plane_value.set_if_neq(ColorPlaneValue(Vec3::new(rgb.red, rgb.green, rgb.blue)));
    }

    commands
        .entity(refs.rg_plane)
        .insert(ColorPlaneValue(Vec3::new(rgb.red, rgb.green, rgb.blue)));
    commands.entity(refs.r_slider).insert(SliderValue(rgb.red));
    commands
        .entity(refs.g_slider)
        .insert(SliderValue(rgb.green));
    commands.entity(refs.b_slider).insert(SliderValue(rgb.blue));
    commands
        .entity(refs.a_slider)
        .insert(SliderValue(rgb.alpha));
    // Convert to nearest lower tenth, so that the string of digits
    // won't be too long to display in the limited space.
    commands.entity(refs.r_input).insert(NumberInputValue::F32(
        (rgb.red * 256.0 * 10.0).floor() / 10.0,
    ));
    commands.entity(refs.g_input).insert(NumberInputValue::F32(
        (rgb.green * 256.0 * 10.0).floor() / 10.0,
    ));
    commands.entity(refs.b_input).insert(NumberInputValue::F32(
        (rgb.blue * 256.0 * 10.0).floor() / 10.0,
    ));
    commands.entity(refs.a_input).insert(NumberInputValue::F32(
        (rgb.alpha * 256.0 * 10.0).floor() / 10.0,
    ));
    commands
        .entity(refs.swatch)
        .insert(ColorSwatchValue(*value));

    if let Ok(mut editable_text) = q_editble_text.get_mut(refs.hex_input) {
        editable_text.queue_edit(TextEdit::SelectAll);
        editable_text.queue_edit(TextEdit::Insert(rgb.to_hex().into()));
    }
}

/// Plugin which registers the observers for updating the swatch color.
pub struct ColorInputPlugin;

impl Plugin for ColorInputPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        app.init_resource::<ColorInputSettings>();
        app.add_systems(PostUpdate, (update_mode_selector, color_input_value_change));
    }
}

/// Observer function which updates the color input value in response to a [`ValueChange`] event.
pub fn color_input_self_update(value_change: On<ValueChange<Color>>, mut commands: Commands) {
    commands
        .entity(value_change.source)
        .insert(ColorInputValue(value_change.value));
}
