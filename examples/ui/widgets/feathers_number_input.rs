//! This example shows off the various Bevy Feathers widgets.

use bevy::{
    feathers::{
        controls::*,
        dark_theme::create_dark_theme,
        display::label,
        theme::{ThemeBackgroundColor, UiTheme},
        tokens, FeathersPlugins,
    },
    input_focus::tab_navigation::TabGroup,
    prelude::*,
    ui::InteractionDisabled,
    ui_widgets::ValueChange,
};

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, FeathersPlugins))
        .insert_resource(UiTheme(create_dark_theme()))
        .add_systems(Startup, scene.spawn())
        // .add_systems(Update, update_colors)
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
            flex_direction: FlexDirection::Column,
            flex_wrap: FlexWrap::Wrap,
            padding: px(8),
            row_gap: px(8),
        }
        TabGroup
        ThemeBackgroundColor(tokens::WINDOW_BG)
        Children[
            demo_field_f32("none (bare)", 1.0, bsn!()),
            demo_field_f32("soft limit", 2.0, bsn!(
                template_value(SoftLimit(NumberInputRange::F32(0.0..10.0)))
            )),
            demo_field_f32("hard limit", 3.0, bsn!(
                template_value(HardLimit(NumberInputRange::F32(-100.0..100.0)))
            )),
            demo_field_f32("soft + hard", 4.0, bsn!(
                template_value(SoftLimit(NumberInputRange::F32(0.0..10.0)))
                template_value(HardLimit(NumberInputRange::F32(-100.0..100.0)))
            )),
            demo_field_f32("precision(0)", 5.0, bsn!(
                NumberInputPrecision(0)
            )),
            demo_field_f32("precision(2)", 6.0, bsn!(
                NumberInputPrecision(2)
            )),
            demo_field_f32("precision(4)", 7.0, bsn!(
                NumberInputPrecision(4)
            )),
            demo_field_f32("step(1.0)", 8.0, bsn!(
                NumberInputStep(1.0f64)
            )),
            demo_field_f64("f64: soft limit", 1.0f64, bsn!(
                template_value(SoftLimit(NumberInputRange::F64(0.0f64..10.0f64)))
            )),
            demo_field_f64("f64: soft limit + precision(2)", 1.0f64, bsn!(
                template_value(SoftLimit(NumberInputRange::F64(0.0f64..10.0f64)))
                NumberInputPrecision(2)
            )),
            demo_field_i32("i32: bare", 1, bsn!()),
            demo_field_i32("i32: soft limit", 1, bsn!(
                template_value(SoftLimit(NumberInputRange::I32(0..10)))
            )),
            demo_field_f32_with_sigil("precision(2) + sigil", 6.0, bsn!(
                NumberInputPrecision(2)
            )),
            demo_field_f32("soft limit + disabled", 2.0, bsn!(
                InteractionDisabled
                template_value(SoftLimit(NumberInputRange::F32(0.0..10.0)))
            )),
        ]
    }
}

fn demo_field_f32(label_text: &str, value: f32, options: impl Scene) -> impl Scene {
    bsn! {
        Node {
            display: Display::Flex,
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Start,
            width: px(200),
            row_gap: px(4),
        }
        Children [
            label(label_text),
            Node {
                display: Display::Flex,
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                align_self: AlignSelf::Stretch,
                justify_content: JustifyContent::SpaceBetween,
            }
            Children [
                (
                    @FeathersNumberInput
                    template_value(NumberInputValue::F32(value))
                    {options}
                    Node {
                        flex_grow: 1.0,
                        max_width: px(120),
                    }
                    on(
                        |value_change: On<ValueChange<f32>>, mut commands: Commands| {
                        commands.entity(value_change.event_target())
                            .insert(NumberInputValue::F32(value_change.value));
                    })
                ),
                (
                    #Output
                    label("-")
                )
            ]
        ]
    }
}

fn demo_field_f32_with_sigil(label_text: &str, value: f32, options: impl Scene) -> impl Scene {
    bsn! {
        Node {
            display: Display::Flex,
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Start,
            width: px(200),
        }
        Children [
            label(label_text),
            Node {
                display: Display::Flex,
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                align_self: AlignSelf::Stretch,
                justify_content: JustifyContent::SpaceBetween,
            }
            Children [
                (
                    @FeathersNumberInput {
                        @sigil_color: tokens::TEXT_INPUT_X_AXIS,
                        @label_text: "X",
                    }
                    template_value(NumberInputValue::F32(value))
                    {options}
                    Node {
                        flex_grow: 1.0,
                        max_width: px(120),
                    }
                    on(
                        |value_change: On<ValueChange<f32>>, mut commands: Commands| {
                        commands.entity(value_change.event_target())
                            .insert(NumberInputValue::F32(value_change.value));
                    })
                ),
                (
                    #Output
                    label("-")
                )
            ]
        ]
    }
}

fn demo_field_f64(label_text: &str, value: f64, options: impl Scene) -> impl Scene {
    bsn! {
        Node {
            display: Display::Flex,
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Start,
            width: px(200),
        }
        Children [
            label(label_text),
            Node {
                display: Display::Flex,
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                align_self: AlignSelf::Stretch,
                justify_content: JustifyContent::SpaceBetween,
            }
            Children [
                (
                    @FeathersNumberInput
                    template_value(NumberInputValue::F64(value))
                    {options}
                    Node {
                        flex_grow: 1.0,
                        max_width: px(120),
                    }
                    on(
                        |value_change: On<ValueChange<f64>>, mut commands: Commands| {
                        commands.entity(value_change.event_target())
                            .insert(NumberInputValue::F64(value_change.value));
                    })
                ),
                (
                    #Output
                    label("-")
                )
            ]
        ]
    }
}

fn demo_field_i32(label_text: &str, value: i32, options: impl Scene) -> impl Scene {
    bsn! {
        Node {
            display: Display::Flex,
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Start,
            width: px(200),
        }
        Children [
            label(label_text),
            Node {
                display: Display::Flex,
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                align_self: AlignSelf::Stretch,
                justify_content: JustifyContent::SpaceBetween,
            }
            Children [
                (
                    @FeathersNumberInput
                    template_value(NumberInputValue::I32(value))
                    {options}
                    Node {
                        flex_grow: 1.0,
                        max_width: px(120),
                    }
                    on(
                        |value_change: On<ValueChange<i32>>, mut commands: Commands| {
                        commands.entity(value_change.event_target())
                            .insert(NumberInputValue::I32(value_change.value));
                    })
                ),
                (
                    #Output
                    label("-")
                )
            ]
        ]
    }
}
