use alloc::sync::Arc;

use bevy_app::{Plugin, PostUpdate};
use bevy_color::{Color, Srgba};
use bevy_ecs::{
    change_detection::DetectChangesMut,
    component::Component,
    entity::Entity,
    error::warn,
    hierarchy::{ChildOf, Children},
    lifecycle::Insert,
    observer::On,
    query::{Changed, With},
    reflect::ReflectComponent,
    relationship::Relationship,
    resource::Resource,
    system::{Commands, Query, Res, ResMut},
    template::FromTemplate,
};
use bevy_input_focus::{tab_navigation::TabIndex, AutoFocus};
use bevy_log::{info, warn};
use bevy_math::{Vec2, Vec3};
use bevy_reflect::{prelude::ReflectDefault, Reflect};
use bevy_scene::prelude::*;
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
        ButtonVariant, ColorChannel, ColorPlaneValue, ColorSwatchValue, FeathersButton,
        FeathersColorPlane, FeathersColorSlider, FeathersColorSwatch, FeathersLazyMenu,
        FeathersMenuPopup, FeathersMenuToolButton, FeathersNumberInput, FeathersTextInput,
        FeathersTextInputContainer, NumberInputPrecision, NumberInputValue,
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
}

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
                            on(handle_swatch_init)
                         }
                    }
                )
            ]
            on(handle_update_input_color)
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
        }
        Node {
            width: px(256 + 8),
        }
        TabIndex
        on(handle_popup_init)
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
                    padding: UiRect::axes(px(4), px(0)),
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
                on(handle_rg_color_plane)
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
                // grid_template_rows: vec![RepeatedGridTrack::px(4, 24.0)],
                grid_auto_rows: vec![GridTrack::px(24.0)],
                align_items: AlignItems::Center,
                margin: UiRect::axes(px(4), px(0)),
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
                    on(|_change: On<ValueChange<f32>>| {
                        // color.rgb_color.blue = change.value;
                    })
                ),
                (
                    #r_input
                    @FeathersNumberInput
                    NumberInputPrecision(2)
                    // DemoVec3Field::X
                    Node {
                        flex_grow: 1.0,
                    }
                    on(
                        |value_change: On<ValueChange<f32>>| {
                        // states.vec3_prop.x = value_change.value;
                    })
                ),
                label("G"),
                (
                    #g_slider
                    @FeathersColorSlider {
                        @value: 0.5,
                        @channel: ColorChannel::Green
                    }
                    AccessibleLabel("Green Channel")
                    on(|_change: On<ValueChange<f32>>| {
                        // color.rgb_color.blue = change.value;
                    })
                ),
                (
                    #g_input
                    @FeathersNumberInput
                    NumberInputPrecision(2)
                    // DemoVec3Field::X
                    Node {
                        flex_grow: 1.0,
                    }
                    on(
                        |value_change: On<ValueChange<f32>>| {
                        // states.vec3_prop.x = value_change.value;
                    })
                ),
                label("B"),
                (
                    #b_slider
                    @FeathersColorSlider {
                        @value: 0.5,
                        @channel: ColorChannel::Blue
                    }
                    AccessibleLabel("Blue Channel")
                    on(|_change: On<ValueChange<f32>>| {
                        // color.rgb_color.blue = change.value;
                    })
                ),
                (
                    #b_input
                    @FeathersNumberInput
                    NumberInputPrecision(2)
                    // DemoVec3Field::X
                    Node {
                        flex_grow: 1.0,
                    }
                    on(
                        |value_change: On<ValueChange<f32>>| {
                        // states.vec3_prop.x = value_change.value;
                    })
                ),
                label("A"),
                (
                    #a_slider
                    @FeathersColorSlider {
                        @value: 0.5,
                        @channel: ColorChannel::Alpha
                    }
                    AccessibleLabel("Alpha Channel")
                    on(|_change: On<ValueChange<f32>>| {
                        // color.rgb_color.alpha = change.value;
                    })
                ),
                (
                    #a_input
                    @FeathersNumberInput
                    NumberInputPrecision(2)
                    // DemoVec3Field::X
                    Node {
                        flex_grow: 1.0,
                    }
                    on(
                        |value_change: On<ValueChange<f32>>| {
                        // states.vec3_prop.x = value_change.value;
                    })
                ),
                (
                    @FeathersTextInputContainer
                    Node {
                        flex_grow: 0.
                        padding: { px(4).left() },
                        grid_column: GridPlacement::span(3),
                    }
                    Children [
                        (
                            #hex_input
                            @FeathersTextInput {
                                // @visible_width: 10f32,
                                @max_characters: 9usize,
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
                        )
                    ]
                )
            ],
        ]
    ))
}

fn handle_rg_color_plane(
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
        info!("Plane: {rgb:?}");
        commands.trigger(ValueChange {
            source: root_id,
            value: rgb,
            is_final: change.is_final,
        });
    }
}

fn handle_update_input_color(
    insert: On<Insert, ColorInputValue>,
    q_color_input: Query<
        (&ColorInputValue, &ButtonEntityRefs, &Children),
        With<FeathersColorInput>,
    >,
    q_popup: Query<&PopupEntityRefs, With<FeathersMenuPopup>>,
    mut commands: Commands,
) {
    let input_ent = insert.entity;
    if let Ok((&ColorInputValue(value), refs, children)) = q_color_input.get(input_ent) {
        commands.entity(refs.0).insert(ColorSwatchValue(value));

        if children.is_empty() {
            warn!("FeathersColorInput missing children");
        } else if let Some(refs) = children
            .iter()
            .find_map(|child_id| q_popup.get(*child_id).ok())
        {
            info!("Got refs: {refs:?}");
        }
    }
}

fn handle_swatch_init(
    insert: On<Insert, FeathersColorSwatch>,
    q_parent: Query<&ChildOf>,
    q_color_input: Query<&ColorInputValue>,
    mut commands: Commands,
) {
    let Some(&ColorInputValue(value)) = q_parent
        .iter_ancestors(insert.entity)
        .find_map(|e| q_color_input.get(e).ok())
    else {
        warn!("Missing ColorInputValue");
        return;
    };

    commands
        .entity(insert.entity)
        .insert(ColorSwatchValue(value));
}

fn update_swatch_color(
    q_swatch: Query<(&ColorInputValue, &Children), Changed<ColorInputValue>>,
    mut commands: Commands,
) {
    for (value, children) in q_swatch.iter() {
        if let Some(second_child) = children.get(1) {
            // commands
            //     .entity(*second_child)
            //     .insert(BackgroundColor(value.0.with_alpha(1.0)));
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

fn handle_popup_init(
    insert: On<Insert, PopupEntityRefs>,
    q_popup: Query<(&PopupEntityRefs, &ChildOf)>,
    q_color_input: Query<&ColorInputValue>,
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

    let rgb: Srgba = value.to_srgba();

    // if let Ok(mut color_plane_value) = q_color_plane.get_mut(refs.rg_plane) {
    //     color_plane_value.set_if_neq(ColorPlaneValue(Vec3::new(rgb.red, rgb.green, rgb.blue)));
    // } else {
    //     warn!("Color plane not found: {:?}", refs.rg_plane);
    // }
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
    commands.entity(refs.r_input).insert(NumberInputValue::F32(
        (rgb.red * 256.0 * 10.0).floor() * 0.1,
    ));
    commands.entity(refs.g_input).insert(NumberInputValue::F32(
        (rgb.green * 256.0 * 10.0).floor() * 0.1,
    ));
    commands.entity(refs.b_input).insert(NumberInputValue::F32(
        (rgb.blue * 256.0 * 10.0).floor() * 0.1,
    ));
    commands.entity(refs.a_input).insert(NumberInputValue::F32(
        (rgb.alpha * 256.0 * 10.0).floor() * 0.1,
    ));
}

/// Plugin which registers the observers for updating the swatch color.
pub struct ColorInputPlugin;

impl Plugin for ColorInputPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        app.init_resource::<ColorInputSettings>();
        app.add_systems(PostUpdate, (update_mode_selector, update_swatch_color));
    }
}
