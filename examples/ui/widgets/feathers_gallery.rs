//! This example shows off the various Bevy Feathers widgets.

use bevy::{
    color::palettes,
    ecs::VariantDefaults,
    feathers::{
        constants::{fonts, icons},
        containers::*,
        controls::*,
        cursor::{EntityCursor, OverrideCursor},
        dark_theme::create_dark_theme,
        display::{caption, icon, label, label_dim, label_small},
        font_styles::InheritableFont,
        palette,
        rounded_corners::RoundedCorners,
        theme::{ThemeBackgroundColor, ThemedText, UiTheme},
        tokens, FeathersPlugins,
    },
    input_focus::{tab_navigation::TabGroup, AutoFocus, InputFocus},
    prelude::*,
    text::{EditableText, TextEdit, TextEditChange},
    ui::{Checked, InteractionDisabled, Selected},
    ui_widgets::{
        checkbox_self_update, listbox_update_selection,
        popover::{Popover, PopoverAlign, PopoverPlacement, PopoverSide},
        radio_self_update, slider_self_update, Activate, ActivateOnPress, RadioGroup, RequestClose,
        SliderPrecision, SliderStep, SliderValue, ValueChange,
    },
    window::SystemCursorIcon,
};
use std::sync::Arc;

/// A struct to hold the state of various widgets shown in the demo.
#[derive(Resource)]
struct DemoWidgetStates {
    rgb_color: Srgba,
    hsl_color: Hsla,
    scalar_prop: f32,
    vec3_prop: Vec3,
}

#[derive(Component, Clone, Copy, PartialEq, FromTemplate)]
enum SwatchType {
    #[default]
    Rgb,
    Hsl,
}

#[derive(Component, Clone, Copy, Default)]
struct HexColorInput;

#[derive(Component, Clone, Copy, Default)]
struct DemoDisabledButton;

#[derive(Component, Clone, Copy, Default)]
struct DemoDialogToggle;

#[derive(Component, Clone, Copy, Default)]
struct DemoScalarField;

#[derive(Component, Clone, Copy, Default, VariantDefaults)]
enum DemoVec3Field {
    #[default]
    X,
    Y,
    Z,
}

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, FeathersPlugins))
        .insert_resource(UiTheme(create_dark_theme()))
        .insert_resource(DemoWidgetStates {
            rgb_color: palettes::tailwind::EMERALD_800.with_alpha(0.7),
            hsl_color: palettes::tailwind::AMBER_800.into(),
            scalar_prop: 7.0,
            vec3_prop: Vec3::new(10.1, 7.124, 100.0),
        })
        .add_systems(Startup, scene.spawn())
        .add_systems(Update, update_colors)
        .run();
}

fn scene() -> impl SceneList {
    bsn_list![Camera2d, demo_root()]
}

fn demo_root() -> impl Scene {
    bsn! {
        Node {
            width: percent(100),
            height: percent(100),
            align_items: AlignItems::Start,
            justify_content: JustifyContent::Start,
            display: Display::Flex,
            flex_direction: FlexDirection::Row,
            column_gap: px(8),
        }
        TabGroup
        ThemeBackgroundColor(tokens::WINDOW_BG)
        Children[
            demo_column_1(),
            demo_column_2(),
        ]
    }
}

#[derive(Component, Debug, Clone, Default, PartialEq)]
enum Months {
    #[default]
    Jan,
    Feb,
    Mar,
    Apr,
    May,
    Jun,
    Jul,
    Aug,
    Sep,
    Oct,
    Nov,
    Dec,
}

impl Months {
    const ALL: [Months; 12] = [
        Months::Jan,
        Months::Feb,
        Months::Mar,
        Months::Apr,
        Months::May,
        Months::Jun,
        Months::Jul,
        Months::Aug,
        Months::Sep,
        Months::Oct,
        Months::Nov,
        Months::Dec,
    ];

    fn to_str(&self) -> &'static str {
        match self {
            Months::Jan => "January",
            Months::Feb => "February",
            Months::Mar => "March",
            Months::Apr => "April",
            Months::May => "May",
            Months::Jun => "June",
            Months::Jul => "July",
            Months::Aug => "August",
            Months::Sep => "September",
            Months::Oct => "October",
            Months::Nov => "November",
            Months::Dec => "December",
        }
    }
}

fn demo_column_1() -> impl Scene {
    // Lazily-constructed menu popup
    let popup: Arc<dyn Fn() -> Box<dyn Scene> + Sync + Send> = Arc::new(|| {
        Box::new(bsn!(
            @FeathersMenuPopup
            // Override popover placement to right-align the popup
            Popover {
                positions: vec![
                    PopoverPlacement {
                        side: PopoverSide::Bottom,
                        align: PopoverAlign::End,
                        gap: 2.0,
                    },
                    PopoverPlacement {
                        side: PopoverSide::Top,
                        align: PopoverAlign::End,
                        gap: 2.0,
                    },
                ],
                window_margin: 10.0,
            }
            Children [
                (
                    @FeathersMenuItem {
                        @caption: bsn! { Text("MenuItem 4") ThemedText }
                    }
                    on(|_: On<Activate>| {
                        info!("Menu item 4 clicked!");
                    })
                ),
                (
                    @FeathersMenuItem {
                        @caption: bsn! { Text("MenuItem 5") ThemedText }
                    }
                    on(|_: On<Activate>| {
                        info!("Menu item 5 clicked!");
                    })
                ),
                (
                    @FeathersMenuItem {
                        @caption: bsn! { Text("MenuItem 6") ThemedText }
                    }
                    on(|_: On<Activate>| {
                        info!("Menu item 6 clicked!");
                    })
                )
            ]
        ))
    });

    bsn! {
        Node {
            display: Display::Flex,
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Stretch,
            justify_content: JustifyContent::Start,
            padding: px(8),
            row_gap: px(8),
            width: percent(30),
            min_width: px(200),
        }
        Children [
            (
                Node {
                    display: Display::Flex,
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Start,
                    column_gap: px(8),
                }
                Children [
                    (
                        @FeathersButton {
                            @caption: bsn! { caption("Normal") }
                        }
                        Node {
                            flex_grow: 1.0,
                        }
                        AccessibleLabel("Normal")
                        on(|_activate: On<Activate>| {
                            info!("Normal button clicked!");
                        })
                        AutoFocus
                    ),
                    (
                        @FeathersButton {
                            @caption: bsn! { caption("Disabled") },
                        }
                        Node {
                            flex_grow: 1.0,
                        }
                        AccessibleLabel("Disabled")
                        InteractionDisabled
                        DemoDisabledButton
                        on(|_activate: On<Activate>| {
                            info!("Disabled button clicked!");
                        })
                    ),
                    (
                        @FeathersButton {
                            @caption: bsn! { caption("Primary") },
                            @variant: ButtonVariant::Primary,
                        }
                        AccessibleLabel("Primary")
                        Node {
                            flex_grow: 1.0,
                        }
                        on(|_activate: On<Activate>| {
                            info!("Primary button clicked!");
                        })
                    ),
                    (
                        @FeathersMenu
                        Children [
                            (
                                @FeathersMenuButton {
                                    @caption: bsn! { caption("Menu") }
                                }
                                AccessibleLabel("Menu Example")
                                Node {
                                    flex_grow: 1.0,
                                }
                            ),
                            (
                                @FeathersMenuPopup
                                Children [
                                    (
                                        @FeathersMenuItem {
                                            @caption: bsn! { caption("MenuItem 1") }
                                        }
                                        on(|_: On<Activate>| {
                                            info!("Menu item 1 clicked!");
                                        })
                                    ),
                                    (
                                        @FeathersMenuItem {
                                            @caption: bsn! { caption("MenuItem 2") }
                                        }
                                        on(|_: On<Activate>| {
                                            info!("Menu item 2 clicked!");
                                        })
                                    ),
                                    @FeathersMenuDivider,
                                    (
                                        @FeathersMenuItem {
                                            @caption: bsn! { caption("MenuItem 3") }
                                        }
                                        on(|_: On<Activate>| {
                                            info!("Menu item 3 clicked!");
                                        })
                                    )
                                ]
                            )
                        ]
                    ),
                    (
                        @FeathersLazyMenu { popup }
                        Children [
                            (
                                @FeathersMenuToolButton {
                                    @caption: bsn! { Text("\u{0398}") ThemedText }
                                }
                                AccessibleLabel("Menu Example")
                                Node {
                                    flex_grow: 1.0,
                                }
                            )
                        ]
                    )
                ]
            ),
            (
                @FeathersSelect {
                    @options: {list_rows_from_strings([
                        "One",
                        "Two",
                        "Three",
                    ], Some(0))},
                }
                Node {
                    flex_grow: 1.0,
                }
                on(|change: On<ValueChange<Entity>>, q_options: Query<&OptionIndex>| {
                    let Ok(option) = q_options.get(change.value) else {
                        info!("Select changed, not sure");
                        return;
                    };
                    info!("Select changed to index {}", option.0);
                })
            ),
            (
                @FeathersSelect {
                    @options: {
                        Box::new(
                            Months::ALL
                                .into_iter()
                                .map(|m| -> Box<dyn SceneList> {
                                    let label = m.to_str();
                                    if m == Months::default() {
                                        bsn! { @FeathersListRow Selected template_value(m) Children [ caption(label) ] }.into()
                                    } else {
                                        bsn! { @FeathersListRow template_value(m) Children [ caption(label) ] }.into()
                                    }
                                })
                                .collect::<Vec<_>>(),
                        ) as Box<dyn SceneList>
                    },
                    @max_visible: 6,
                }
                Node {
                    flex_grow: 1.0,
                }
                on(|change: On<ValueChange<Entity>>, q_months: Query<&Months>| {
                    let Ok(month) = q_months.get(change.value) else {
                        return;
                    };
                    info!("Select changed to {:?}", month);
                })
            ),
            (
                Node {
                    display: Display::Flex,
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Start,
                    column_gap: px(1),
                }
                Children [
                    (
                        @FeathersButton {
                            @caption: bsn! { caption("Left") },
                            @corners: RoundedCorners::Left,
                        }
                        Node {
                            flex_grow: 1.0,
                        }
                        AccessibleLabel("Left")
                        on(|_activate: On<Activate>| {
                            info!("Left button clicked!");
                        })
                    ),
                    (
                        @FeathersButton {
                            @caption: bsn! { caption("Center") },
                            @corners: RoundedCorners::None,
                        }
                        Node {
                            flex_grow: 1.0,
                        }
                        AccessibleLabel("Center")
                        on(|_activate: On<Activate>| {
                            info!("Center button clicked!");
                        })
                    ),
                    (
                        @FeathersButton {
                            @caption: bsn! { caption("Right") },
                            @variant: ButtonVariant::Primary,
                            @corners: RoundedCorners::Right,
                        }
                        Node {
                            flex_grow: 1.0,
                        }
                        AccessibleLabel("Right")
                        on(|_activate: On<Activate>| {
                            info!("Right button clicked!");
                        })
                    ),
                ]
            ),
            (
                Node {
                    display: Display::Flex,
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Start,
                    column_gap: px(8),
                }
                Children [
                    (
                        @FeathersButton {
                            @caption: bsn! { caption("Toggle override") },
                        }
                        Node {
                            flex_grow: 1.0,
                        }
                        on(|_activate: On<Activate>, mut ovr: ResMut<OverrideCursor>| {
                            ovr.0 = if ovr.0.is_some() {
                                None
                            } else {
                                Some(EntityCursor::System(SystemCursorIcon::Wait))
                            };
                            info!("Override cursor button clicked!");
                        })
                    ),
                    (
                        @FeathersButton {
                            @caption: bsn! { caption("Quit\u{2026}") },
                        }
                        Node {
                            flex_grow: 1.0,
                        }
                        on(spawn_quit_dialog)
                    ),
                ]
            ),
            (
                Node {
                    display: Display::Flex,
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Start,
                    column_gap: px(8),
                }
                Children [
                    label("Dialog:"),
                    (
                        @FeathersToggleSwitch
                        DemoDialogToggle
                        on(toggle_demo_dialog)
                    ),
                ]
            ),
            (
                @FeathersCheckbox {
                    @caption: bsn! { caption("Checkbox") }
                }
                Checked
                AccessibleLabel("Checkbox Example")
                on(
                    |change: On<ValueChange<bool>>,
                        query: Query<Entity, With<DemoDisabledButton>>,
                        mut commands: Commands| {
                        info!("Checkbox clicked!");
                        let mut button = commands.entity(query.single().unwrap());
                        if change.value {
                            button.insert(InteractionDisabled);
                        } else {
                            button.remove::<InteractionDisabled>();
                        }
                        let mut checkbox = commands.entity(change.source);
                        if change.value {
                            checkbox.insert(Checked);
                        } else {
                            checkbox.remove::<Checked>();
                        }
                    }
                )
            ),
            (
                @FeathersCheckbox {
                    @caption: bsn! { caption("Fast Click Checkbox") }
                }
                ActivateOnPress
                AccessibleLabel("Fast Click Checkbox Example")
                on(
                    |change: On<ValueChange<bool>>,
                     mut commands: Commands| {
                        info!("Checkbox clicked!");
                        let mut checkbox = commands.entity(change.source);
                        if change.value {
                            checkbox.insert(Checked);
                        } else {
                            checkbox.remove::<Checked>();
                        }
                    }
                )
            ),
            (
                @FeathersCheckbox {
                    @caption: bsn! { caption("Disabled") },
                }
                InteractionDisabled
                AccessibleLabel("Disabled Checkbox Example")
                on(|_change: On<ValueChange<bool>>| {
                    warn!("Disabled checkbox clicked!");
                })
            ),
            (
                @FeathersCheckbox {
                    @caption: bsn! { caption("Checked+Disabled") }
                }
                InteractionDisabled
                Checked
                AccessibleLabel("Disabled and Checked Checkbox Example")
                on(|_change: On<ValueChange<bool>>| {
                    warn!("Disabled checkbox clicked!");
                })
            ),
            (
                Node {
                    display: Display::Flex,
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Start,
                    column_gap: px(8),
                }
                Children [
                    (
                        Node {
                            display: Display::Flex,
                            flex_direction: FlexDirection::Column,
                            row_gap: px(4),
                        }
                        RadioGroup
                        on(radio_self_update)
                        Children [
                            (
                                @FeathersRadio {
                                    @caption: bsn! { caption("One") }
                                }
                                Checked
                            ),
                            @FeathersRadio {
                                @caption: bsn! { caption("Two") }
                            },
                            (
                                @FeathersRadio {
                                    @caption: bsn! { caption("Fast Click") }
                                }
                                ActivateOnPress
                            ),
                            (
                                @FeathersRadio {
                                    @caption: bsn! { caption("Disabled") }
                                }
                                InteractionDisabled
                            ),
                        ]
                    )
                ]
            ),
            (
                Node {
                    display: Display::Flex,
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Start,
                    column_gap: px(8),
                }
                Children [
                    (@FeathersToggleSwitch on(checkbox_self_update)),
                    (@FeathersToggleSwitch ActivateOnPress on(checkbox_self_update)),
                    (@FeathersToggleSwitch InteractionDisabled on(checkbox_self_update)),
                    (@FeathersToggleSwitch InteractionDisabled Checked on(checkbox_self_update)),
                    (@FeathersDisclosureToggle on(checkbox_self_update)),
                ]
            ),
            (
                @FeathersSlider {
                    @max: 100.0,
                }
                SliderValue(20.0)
                SliderStep(10.)
                SliderPrecision(2)
                on(slider_self_update)
            ),
            (
                Node {
                    display: Display::Flex,
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::SpaceBetween,
                    column_gap: px(4),
                }
                Children [
                    label("Srgba"),
                    // Spacer
                    flex_spacer(),
                    // Text input
                    (
                        @FeathersTextInputContainer
                        Node {
                            flex_grow: 0.
                            padding: { px(4).left() },
                        }
                        Children [
                            (
                                @FeathersTextInput {
                                    @visible_width: 10f32,
                                    @max_characters: 9usize,
                                }
                                InheritableFont {
                                    font: fonts::MONO
                                }
                                HexColorInput
                                on(handle_hex_color_change)
                            )
                        ]
                    )
                    (@FeathersColorSwatch {
                        @opaque_color_percentage: 30.0,
                    } SwatchType::Rgb),
                ]
            ),
            (
                @FeathersColorPlane::RedBlue
                on(|change: On<ValueChange<Vec2>>, mut color: ResMut<DemoWidgetStates>| {
                    color.rgb_color.red = change.value.x;
                    color.rgb_color.blue = change.value.y;
                })
            ),
            (
                @FeathersColorSlider {
                    @value: 0.5,
                    @channel: ColorChannel::Red
                }
                AccessibleLabel("Red Channel")
                on(|change: On<ValueChange<f32>>, mut color: ResMut<DemoWidgetStates>| {
                    color.rgb_color.red = change.value;
                })
            ),
            (
                @FeathersColorSlider {
                    @value: 0.5,
                    @channel: ColorChannel::Green
                }
                AccessibleLabel("Green Channel")
                on(|change: On<ValueChange<f32>>, mut color: ResMut<DemoWidgetStates>| {
                    color.rgb_color.green = change.value;
                })
            ),
            (
                @FeathersColorSlider {
                    @value: 0.5,
                    @channel: ColorChannel::Blue
                }
                AccessibleLabel("Blue Channel")
                on(|change: On<ValueChange<f32>>, mut color: ResMut<DemoWidgetStates>| {
                    color.rgb_color.blue = change.value;
                })
            ),
            (
                @FeathersColorSlider {
                    @value: 0.5,
                    @channel: ColorChannel::Alpha
                }
                AccessibleLabel("Alpha Channel")
                on(|change: On<ValueChange<f32>>, mut color: ResMut<DemoWidgetStates>| {
                    color.rgb_color.alpha = change.value;
                })
            ),
            (
                Node {
                    display: Display::Flex,
                    align_items: AlignItems::Center,
                    flex_direction: FlexDirection::Row,
                    justify_content: JustifyContent::SpaceBetween,
                }
                Children [
                    label("Hsl"),
                    (@FeathersColorSwatch SwatchType::Hsl)
                ]
            ),
            (
                @FeathersColorSlider {
                    @value: 0.5,
                    @channel: ColorChannel::HslHue
                }
                AccessibleLabel("Hue Channel")
                on(|change: On<ValueChange<f32>>, mut color: ResMut<DemoWidgetStates>| {
                    color.hsl_color.hue = change.value;
                })
            ),
            (
                @FeathersColorSlider {
                    @value: 0.5,
                    @channel: ColorChannel::HslSaturation
                }
                AccessibleLabel("Saturation Channel")
                on(|change: On<ValueChange<f32>>, mut color: ResMut<DemoWidgetStates>| {
                    color.hsl_color.saturation = change.value;
                })
            ),
            (
                @FeathersColorSlider {
                    @value: 0.5,
                    @channel: ColorChannel::HslLightness
                }
                AccessibleLabel("Lightness Channel")
                on(|change: On<ValueChange<f32>>, mut color: ResMut<DemoWidgetStates>| {
                    color.hsl_color.lightness = change.value;
                })
            )
        ]
    }
}

fn demo_column_2() -> impl Scene {
    bsn! {
        Node {
            display: Display::Flex,
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Stretch,
            justify_content: JustifyContent::Start,
            padding: px(8),
            row_gap: px(8),
            width: percent(30),
            min_width: px(200),
        }
        Children [
            (
                pane() Children [
                    pane_header() Children [
                        @FeathersToolButton {
                            @variant: ButtonVariant::Primary,
                            @caption: bsn! { caption("\u{0398}") }
                        },
                        pane_header_divider(),
                        @FeathersToolButton {
                            @variant: ButtonVariant::Plain,
                            @caption: bsn! { caption("\u{00BC}") }
                        },
                        @FeathersToolButton {
                            @variant: ButtonVariant::Plain,
                            @caption: bsn! { caption("\u{00BD}") }
                        },
                        @FeathersToolButton {
                            @variant: ButtonVariant::Plain,
                            @caption: bsn! { caption("\u{00BE}") }
                        },
                        pane_header_divider(),
                        @FeathersToolButton {
                            @variant: ButtonVariant::Plain,
                            @caption: bsn! { icon(icons::CHEVRON_DOWN) }
                        },
                        flex_spacer(),
                        @FeathersToolButton {
                            @variant: ButtonVariant::Plain,
                            @caption: bsn! { icon(icons::X) }
                        },
                    ],
                    (
                        pane_body() Children [
                            label_dim("A standard editor pane"),
                            subpane() Children [
                                subpane_header() Children [
                                    caption("Left"),
                                    caption("Center"),
                                    caption("Right")
                                ],
                                subpane_body() Children [
                                    label_dim("A standard sub-pane"),
                                    group()
                                    Children [
                                        group_header() Children [
                                            caption("Group"),
                                        ],
                                        group_body()
                                        Children [
                                            label("A standard group"),
                                            label_small("Scalar property"),
                                            (
                                                @FeathersNumberInput
                                                DemoScalarField
                                                NumberInputPrecision(2)
                                                HardLimit::f32(0.0..100.0)
                                                Node {
                                                    flex_grow: 1.0,
                                                    max_width: px(100),
                                                }
                                                on(
                                                    |value_change: On<ValueChange<f32>>,
                                                    mut states: ResMut<DemoWidgetStates>| {
                                                    states.scalar_prop = value_change.value;
                                                })
                                            ),
                                            label_small("Scalar property (copy)"),
                                            (
                                                @FeathersNumberInput
                                                DemoScalarField
                                                NumberInputPrecision(4)
                                                Node {
                                                    flex_grow: 1.0,
                                                    max_width: px(100),
                                                }
                                                on(
                                                    |value_change: On<ValueChange<f32>>,
                                                    mut states: ResMut<DemoWidgetStates>| {
                                                    states.scalar_prop = value_change.value;
                                                })
                                            ),
                                            label_small("Vec3 property"),
                                            Node {
                                                display: Display::Flex,
                                                flex_direction: FlexDirection::Row,
                                                column_gap: px(6),
                                                align_items: AlignItems::Center,
                                                justify_content: JustifyContent::SpaceBetween,
                                            }
                                            Children [
                                                (
                                                    @FeathersNumberInput {
                                                        @sigil_color: tokens::TEXT_INPUT_X_AXIS,
                                                        @label_text: "X",
                                                    }
                                                    NumberInputPrecision(2)
                                                    DemoVec3Field::X
                                                    Node {
                                                        flex_grow: 1.0,
                                                    }
                                                    BorderColor::all(palette::X_AXIS)
                                                    on(
                                                        |value_change: On<ValueChange<f32>>,
                                                        mut states: ResMut<DemoWidgetStates>| {
                                                        states.vec3_prop.x = value_change.value;
                                                    })
                                                ),
                                                (
                                                    @FeathersNumberInput {
                                                        @sigil_color: tokens::TEXT_INPUT_Y_AXIS,
                                                        @label_text: "Y",
                                                    }
                                                    NumberInputPrecision(2)
                                                    DemoVec3Field::Y
                                                    Node {
                                                        flex_grow: 1.0,
                                                    }
                                                    on(
                                                        |value_change: On<ValueChange<f32>>,
                                                        mut states: ResMut<DemoWidgetStates>| {
                                                        states.vec3_prop.y = value_change.value;
                                                    })
                                                ),
                                                (
                                                    @FeathersNumberInput {
                                                        @sigil_color: tokens::TEXT_INPUT_Z_AXIS,
                                                        @label_text: "Z",
                                                    }
                                                    NumberInputPrecision(2)
                                                    DemoVec3Field::Z
                                                    Node {
                                                        flex_grow: 1.0,
                                                    }
                                                    on(
                                                        |value_change: On<ValueChange<f32>>,
                                                        mut states: ResMut<DemoWidgetStates>| {
                                                        states.vec3_prop.z = value_change.value;
                                                    })
                                                ),
                                            ],
                                        ],
                                    ]
                                ],
                            ]
                        ]
                    ),
                ]
            ),
            subpane() Children [
                subpane_header() Children [
                    caption("List"),
                ],
                subpane_body() Children [
                    @FeathersListView {
                        @rows: {bsn_list![
                            @FeathersListRow Children [caption("First World")],
                            @FeathersListRow Selected Children [caption("Second Nature")],
                            @FeathersListRow Children [caption("Third Degree")],
                            @FeathersListRow InteractionDisabled Children [caption("Fourth Wall")],
                            @FeathersListRow Children [caption("Fifth Column")],
                            @FeathersListRow Children [caption("Sixth Sense")],
                            @FeathersListRow Children [caption("Seventh Heaven")],
                            @FeathersListRow Children [caption("Eighth Wonder")],
                            @FeathersListRow Children [caption("Ninth Inning")],
                            @FeathersListRow Children [caption("Tenth Amendment")],
                            @FeathersListRow Children [caption("Eleventh Hour")],
                            @FeathersListRow Children [caption("Twelfth Night")],
                        ]}
                    }
                    Node {
                        max_height: px(130)
                    }
                    on(listbox_update_selection)
                ],
            ]
        ]
    }
}

fn update_colors(
    states: Res<DemoWidgetStates>,
    mut sliders: Query<(Entity, &ColorSlider, &mut SliderBaseColor)>,
    mut swatches: Query<(&mut ColorSwatchValue, &SwatchType), With<FeathersColorSwatch>>,
    mut color_planes: Query<&mut ColorPlaneValue, With<FeathersColorPlane>>,
    q_text_input: Single<(Entity, &mut EditableText), With<HexColorInput>>,
    q_scalar_input: Query<Entity, With<DemoScalarField>>,
    q_vec3_input: Query<(Entity, &DemoVec3Field)>,
    mut commands: Commands,
    focus: Res<InputFocus>,
) {
    if states.is_changed() {
        for (slider_ent, slider, mut base) in sliders.iter_mut() {
            match slider.channel {
                ColorChannel::Red => {
                    base.0 = states.rgb_color.into();
                    commands
                        .entity(slider_ent)
                        .insert(SliderValue(states.rgb_color.red));
                }
                ColorChannel::Green => {
                    base.0 = states.rgb_color.into();
                    commands
                        .entity(slider_ent)
                        .insert(SliderValue(states.rgb_color.green));
                }
                ColorChannel::Blue => {
                    base.0 = states.rgb_color.into();
                    commands
                        .entity(slider_ent)
                        .insert(SliderValue(states.rgb_color.blue));
                }
                ColorChannel::HslHue => {
                    base.0 = states.hsl_color.into();
                    commands
                        .entity(slider_ent)
                        .insert(SliderValue(states.hsl_color.hue));
                }
                ColorChannel::HslSaturation => {
                    base.0 = states.hsl_color.into();
                    commands
                        .entity(slider_ent)
                        .insert(SliderValue(states.hsl_color.saturation));
                }
                ColorChannel::HslLightness => {
                    base.0 = states.hsl_color.into();
                    commands
                        .entity(slider_ent)
                        .insert(SliderValue(states.hsl_color.lightness));
                }
                ColorChannel::Alpha => {
                    base.0 = states.rgb_color.into();
                    commands
                        .entity(slider_ent)
                        .insert(SliderValue(states.rgb_color.alpha));
                }
            }
        }

        for (mut swatch_value, swatch_type) in swatches.iter_mut() {
            swatch_value.0 = match swatch_type {
                SwatchType::Rgb => states.rgb_color.into(),
                SwatchType::Hsl => states.hsl_color.into(),
            };
        }

        for mut plane_value in color_planes.iter_mut() {
            plane_value.0.x = states.rgb_color.red;
            plane_value.0.y = states.rgb_color.blue;
            plane_value.0.z = states.rgb_color.green;
        }

        // Only update the hex input field when it's not focused, otherwise it interferes
        // with typing.
        let (input_ent, mut editable_text) = q_text_input.into_inner();
        if Some(input_ent) != focus.get() {
            editable_text.queue_edit(TextEdit::SelectAll);
            editable_text.queue_edit(TextEdit::Insert(states.rgb_color.to_hex().into()));
        }

        for scalar_input_ent in q_scalar_input.iter() {
            commands
                .entity(scalar_input_ent)
                .insert(NumberInputValue::F32(states.scalar_prop));
        }

        for (vec3_input_ent, axis) in q_vec3_input.iter() {
            let new_value = match axis {
                DemoVec3Field::X => states.vec3_prop.x,
                DemoVec3Field::Y => states.vec3_prop.y,
                DemoVec3Field::Z => states.vec3_prop.z,
            };

            commands
                .entity(vec3_input_ent)
                .insert(NumberInputValue::F32(new_value));
        }
    }
}

fn handle_hex_color_change(
    _change: On<TextEditChange>,
    q_text_input: Single<&EditableText, With<HexColorInput>>,
    mut colors: ResMut<DemoWidgetStates>,
) {
    let editable_text = *q_text_input;
    if let Ok(color) = Srgba::hex(editable_text.value().to_string())
        && color != colors.rgb_color
    {
        colors.rgb_color = color;
    }
}

fn spawn_quit_dialog(activate: On<Activate>, mut commands: Commands) {
    commands
        .entity(activate.event_target())
        .queue_spawn_related_scenes::<Children>(bsn_list! (
            @FeathersDialog {
                @width: px(320),
                @contents: bsn_list! {
                    @FeathersDialogHeader Children [
                        caption("Quit Feathers Gallery"),
                        @FeathersDialogClose
                    ],
                    @FeathersDialogBody Children [
                        Text("Are you really sure you want to quit? I mean, really, really sure?")
                        ThemedText
                    ],
                    @FeathersDialogFooter Children [
                        (
                            @FeathersButton {
                                @caption: bsn! { caption("Cancel") },
                            }
                            AccessibleLabel("Cancel")
                            on(|activate: On<Activate>, mut commands: Commands| {
                                commands.trigger(RequestClose { source: activate.event_target() });
                            })
                        ),
                        (
                            @FeathersButton {
                                @caption: bsn! { caption("Exit Application") },
                                @variant: ButtonVariant::Primary,
                            }
                            AccessibleLabel("Exit Application")
                            on(|_activate: On<Activate>, mut exit: MessageWriter<AppExit>| {
                                exit.write(AppExit::Success);
                            })
                        ),

                    ],
                }
            }
            on(|close: On<RequestClose>, mut commands: Commands| {
                commands.entity(close.event_target()).despawn();
            })
        ));
}

fn toggle_demo_dialog(
    change: On<ValueChange<bool>>,
    mut commands: Commands,
    dialogs: Query<Entity, With<FeathersFloatingDialog>>,
) {
    let toggle = change.source;
    if change.value {
        commands.entity(toggle).insert(Checked);
        // Spawn at the root rather than as a child of the toggle, so clicks on the
        // dialog don't bubble up to the toggle switch.
        commands.spawn_scene(bsn! {
            @FeathersFloatingDialog {
                @title: {"Hello".to_string()},
                @width: px(280),
                @contents: bsn_list! {
                    Text("Close this dialog to unset the toggle.") ThemedText
                }
            }
            // The dialog despawns itself on close; this just clears the toggle.
            on(|_close: On<RequestClose>,
                mut commands: Commands,
                toggles: Query<Entity, With<DemoDialogToggle>>| {
                for toggle in toggles.iter() {
                    commands.entity(toggle).remove::<Checked>();
                }
            })
        });
    } else {
        commands.entity(toggle).remove::<Checked>();
        for dialog in dialogs.iter() {
            commands.entity(dialog).despawn();
        }
    }
}
