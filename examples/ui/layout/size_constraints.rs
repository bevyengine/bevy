//! Demonstrates how the to use the size constraints to control the size of a UI node.

use bevy::{
    color::palettes::css::*,
    prelude::*,
    text::FontSourceTemplate,
    ui::Checked,
    ui_widgets::{RadioButton, RadioGroup, ValueChange},
};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_observer(on_value_change_constraints)
        .run();
}

const ACTIVE_BORDER_COLOR: Color = Color::Srgba(ANTIQUE_WHITE);
const INACTIVE_BORDER_COLOR: Color = Color::BLACK;

const ACTIVE_INNER_COLOR: Color = Color::WHITE;
const INACTIVE_INNER_COLOR: Color = Color::Srgba(NAVY);

const ACTIVE_TEXT_COLOR: Color = Color::BLACK;
const HOVERED_TEXT_COLOR: Color = Color::WHITE;
const UNHOVERED_TEXT_COLOR: Color = Color::srgb(0.5, 0.5, 0.5);

/// A marker component for the UI Node which will be resized by user input.
#[derive(Component, Clone, Default)]
struct Bar;

/// The four properties on the `Node` of the `Bar` entity that can be changed.
#[derive(Copy, Clone, Debug, Component, PartialEq)]
enum Constraint {
    FlexBasis,
    Width,
    MinWidth,
    MaxWidth,
}

#[derive(Copy, Clone, Component, Default, PartialEq)]
struct RadioButtonValue(Val);

fn setup(mut commands: Commands) {
    // UI Camera
    commands.spawn(Camera2d);

    commands.spawn_scene(bsn! {
        // Background Node
        Node {
            width: percent(100),
            height: percent(100),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
        }
        BackgroundColor(Color::BLACK)
        Children [
            // Centered column that contains all the content of the example.
            Node {
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
            }
            Children [
                Text::new("Size Constraints Example")
                font_style_scene()
                Node {
                    margin: UiRect::bottom(px(25)),
                },

                bar_scene(),

                // Controls (radio buttons)
                Node {
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Stretch,
                    padding: UiRect::all(px(10)),
                    margin: UiRect::top(px(50)),
                }
                BackgroundColor(YELLOW)
                Children [
                    {
                        [
                            Constraint::MinWidth,
                            Constraint::FlexBasis,
                            Constraint::Width,
                            Constraint::MaxWidth,
                        ].into_iter().map(|constraint| {
                            radio_group_scene(constraint)
                        })
                        .collect::<Vec<_>>()
                    }
                ]
            ]
        ]
    });
}

fn font_style_scene() -> impl Scene {
    bsn! {
        TextFont {
            font: FontSourceTemplate::Handle("fonts/FiraSans-Bold.ttf"),
            font_size: FontSize::Px(33.0),
        }
        TextColor(Color::srgb(0.9, 0.9, 0.9))
    }
}

fn bar_scene() -> impl Scene {
    bsn! {
        Node {
            flex_basis: percent(100),
            align_self: AlignSelf::Stretch,
            padding: UiRect::all(px(10)),
        }
        BackgroundColor(YELLOW)
        Children [
            Node {
                align_items: AlignItems::Stretch,
                width: percent(100),
                height: px(100),
                padding: UiRect::all(px(4)),
            }
            BackgroundColor(Color::BLACK)
            Children [
                // This bar will grow and shrink as the constraints are tinkered with.
                Bar
                Node
                BackgroundColor(Color::WHITE)
            ]
        ]
    }
}

fn radio_group_scene(constraint: Constraint) -> impl Scene {
    let label = match constraint {
        Constraint::FlexBasis => "flex_basis",
        Constraint::Width => "size",
        Constraint::MinWidth => "min_size",
        Constraint::MaxWidth => "max_size",
    };

    bsn! {
        Node {
            flex_direction: FlexDirection::Column,
            padding: UiRect::all(px(2)),
            align_items: AlignItems::Stretch,
        }
        BackgroundColor(Color::BLACK)
        Children [
            Node {
                flex_direction: FlexDirection::Row,
                justify_content: JustifyContent::End,
                padding: UiRect::all(px(2)),
            }
            Children [
                // Row Label
                Node {
                    min_width: px(200),
                    max_width: px(200),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                }
                Children [
                    Text::new(label)
                    font_style_scene()
                ],

                // Row Buttons
                Node
                RadioGroup
                Children [
                    Checked
                    radio_button_scene(
                        constraint,
                        RadioButtonValue(auto()),
                        "Auto".to_string(),
                        true,
                    ),

                    {
                        [0, 25, 50, 75, 100, 125].into_iter().map(|percent_value| {
                            radio_button_scene(
                                constraint,
                                RadioButtonValue(percent(percent_value)),
                                format!("{percent_value}%"),
                                false,
                            )
                        }).collect::<Vec<_>>()
                    },
                ],
            ]
        ]
    }
}

fn radio_button_scene(
    constraint: Constraint,
    action: RadioButtonValue,
    label: String,
    active: bool,
) -> impl Scene {
    bsn! {
        RadioButton
        Node {
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            border: UiRect::all(px(2)),
            margin: UiRect::horizontal(px(2)),
        }
        BorderColor::all(if active {
            ACTIVE_BORDER_COLOR
        } else {
            INACTIVE_BORDER_COLOR
        })
        template_value(constraint)
        template_value(action)
        Children [
            Node {
                width: px(100),
                justify_content: JustifyContent::Center,
            }
            BackgroundColor({if active {
                ACTIVE_INNER_COLOR
            } else {
                INACTIVE_INNER_COLOR
            }})
            Children [
                Text::new(label)
                font_style_scene()
                TextColor({if active {
                    ACTIVE_TEXT_COLOR
                } else {
                    UNHOVERED_TEXT_COLOR
                }})
                TextLayout::justify(Justify::Center)
            ]
        ]
        // Observers for updating text on hover/leave
        on(|event: On<Pointer<Over>>,
            has_checked_query: Query<&Checked>,
            child_q: Query<&Children>,
            mut commands: Commands| {
            if has_checked_query.contains(event.entity) {
                return;
            }

            for text_entity in child_q.iter_leaves(event.entity) {
                commands.entity(text_entity).insert(TextColor(HOVERED_TEXT_COLOR));
            }
        })
        on(|event: On<Pointer<Out>>,
            has_checked_query: Query<&Checked>,
            child_q: Query<&Children>,
            mut commands: Commands| {
            if has_checked_query.contains(event.entity) {
                return;
            }

            for text_entity in child_q.iter_leaves(event.entity) {
                commands.entity(text_entity).insert(TextColor(UNHOVERED_TEXT_COLOR));
            }
        })
    }
}

/// This system updates the Bar when a new value for a constraint is selected, and marks
/// the radio button as the one currently selected
fn on_value_change_constraints(
    event: On<ValueChange<Entity>>,
    new_setting_query: Query<
        (&Constraint, &RadioButtonValue, Entity, &Children),
        (With<RadioButton>, Without<Checked>),
    >,
    previous_query: Query<
        (&Constraint, &RadioButtonValue, Entity, &Children),
        (With<RadioButton>, With<Checked>),
    >,
    child_q: Query<&Children>,
    mut commands: Commands,
    mut bar_node: Single<&mut Node, With<Bar>>,
) {
    if let Ok((constraint, value, entity, children)) = new_setting_query.get(event.value) {
        for (previous_constraint, previous_value, previous_entity, previous_children) in
            previous_query.iter()
        {
            if constraint == previous_constraint && value == previous_value {
                // There is no change in constraint. We can exit out early.
                return;
            } else if constraint == previous_constraint {
                commands.entity(previous_entity).remove::<Checked>();
                commands
                    .entity(previous_entity)
                    .insert(BorderColor::all(INACTIVE_BORDER_COLOR));
                // radio button entities only have one child which contains the inner background color.
                commands
                    .entity(*previous_children.first().unwrap())
                    .insert(BackgroundColor(INACTIVE_INNER_COLOR));
                for text_entity in child_q.iter_leaves(previous_entity) {
                    commands
                        .entity(text_entity)
                        .insert(TextColor(UNHOVERED_TEXT_COLOR));
                }

                commands.entity(entity).insert(Checked);
                commands
                    .entity(entity)
                    .insert(BorderColor::all(ACTIVE_BORDER_COLOR));
                commands
                    .entity(*children.first().unwrap())
                    .insert(BackgroundColor(ACTIVE_INNER_COLOR));
                for text_entity in child_q.iter_leaves(entity) {
                    commands
                        .entity(text_entity)
                        .insert(TextColor(ACTIVE_TEXT_COLOR));
                }
            }
        }

        match constraint {
            Constraint::FlexBasis => {
                bar_node.flex_basis = value.0;
            }
            Constraint::Width => {
                bar_node.width = value.0;
            }
            Constraint::MinWidth => {
                bar_node.min_width = value.0;
            }
            Constraint::MaxWidth => {
                bar_node.max_width = value.0;
            }
        }
    }
}
