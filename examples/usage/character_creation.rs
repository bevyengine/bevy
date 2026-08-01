//! This example illustrates how to manage data input from select
//! widgets in the `ui_widgets` crate.
//!
//! When a UI widget is interacted with by an application user,
//! various events may be triggered that that the user presumably wants
//! to be recognized by the application. State may be directly changed
//! on the widgets themselves depending on the widget. It is up to the application to
//! handle and interpret these events and changes, which usually means updating its own internal
//! state and refreshing its user interface so that the user sees the most
//! recent data.
//!
//! The example will implement a traditional Model View Controller (MVC) design
//! using the various ui widgets to power a character creation screen.
//!
//! To read more about state management, consult the [`bevy::ui_widgets`]
//! crate level documentation.

use bevy::{
    color::palettes,
    ecs::schedule::IntoScheduleConfigs,
    input_focus::tab_navigation::TabIndex,
    prelude::*,
    text::{EditableText, TextCursorStyle},
    ui::Checked,
    ui_widgets::{Checkbox, RadioButton, RadioGroup, TextInput, ValueChange},
    window::{CursorIcon, PrimaryWindow, SystemCursorIcon},
};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .init_resource::<Character>()
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                // Updates the Model if the user changed the name via text input
                on_changed_editable_text,
                // Updates the View after any Model changes
                refresh_character.run_if(resource_changed::<Character>),
            )
                .chain(),
        )
        .run();
}

fn setup(mut commands: Commands, character: Res<Character>) {
    commands.spawn_scene_list(bsn_list! {
        Camera2d,

        // This scene will serve as one half of our "View"
        // and all of the "Controller" in our MVC design.
        // The user interacts with UI widgets, which are processed by the "Controller"
        // via observers and other systems. The observers and systems update the
        // state and also update the look of the UI widgets
        // (i.e. changing text color of a selected option, inserting an X).
        ui(&character),

        // This scene will serve as the other half of our "View" in our MVC design.
        // The user will see the character they are creating.
        character_view(&character),
    })
}

/// This resource serves as the "Model" in our MVC Design.
/// It serves as our source of truth for the character being created.
#[derive(Resource)]
struct Character {
    name: String,
    age: u32,
    hat_type: HatType,
    tint_yellow: bool,
}

impl Default for Character {
    fn default() -> Self {
        Character {
            name: "Bevy".into(),
            age: 5,
            hat_type: HatType::default(),
            tint_yellow: false,
        }
    }
}

/// Hat options for the character to have.
/// This derives `Component` so each enum value can be placed on individual radio buttons
/// for ease of identifying which radio button was selected.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq)]
enum HatType {
    #[default]
    None,
    TopHat,
    DunceCap,
}

const HAT_TYPES: [HatType; 3] = [HatType::None, HatType::TopHat, HatType::DunceCap];

// --- START MARKER COMPONENTS --- //

#[derive(Component, Clone, Default)]
struct NameInput;

#[derive(Component, Clone, Default)]
struct AgeSlider;

#[derive(Component, Clone, Default)]
struct HatTypeRadioGroup;

#[derive(Component, Clone, Default)]
struct TintYellowCheckbox;

#[derive(Component, Clone, Default)]
struct CharacterView;

// --- END MARKER COMPONENTS --- //

// --- WIDGET VIEW & CONTROLLER --- //

/// Spawns the ui widgets that will serve as half of our "View",
/// with "Controller" logic as important observers in our MVC design.
fn ui(character: &Character) -> impl Scene {
    bsn! {
        // UI will take up the left half of the screen.
        Node {
            flex_direction: FlexDirection::Column,
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            height: percent(100)
            width: percent(50),
        }
        Children [
            // The character creation pane
            Node {
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::SpaceEvenly,
                align_items: AlignItems::Center,
                margin: percent(5),
                width: percent(90),
                padding: UiRect::vertical(px(10)),
                border_radius: BorderRadius::all(px(5)),
                row_gap: px(10),
            }
            BackgroundColor(palettes::basic::GRAY)
            Children [
                // Pane header
                Node
                Children[
                    Text::new("Character Creator")
                ],

                name_text_input_row(character),

                hat_type_radio_group(character),

                tint_yellow_checkbox_row(character),
            ]
        ]
    }
}

/// An observer that styles the cursor, used primarily for buttons / checkboxes.
/// This is not part of the Controller.
fn on_pointer_over_pointer_cursor(
    _event: On<Pointer<Over>>,
    mut window_q: Query<Entity, With<PrimaryWindow>>,
    mut commands: Commands,
) {
    for window in window_q.iter_mut() {
        commands
            .entity(window)
            .insert(CursorIcon::System(SystemCursorIcon::Pointer));
    }
}

/// An observer that styles the cursor, used primarily for text input.
/// This is not part of the Controller.
fn on_pointer_over_text_cursor(
    _event: On<Pointer<Over>>,
    mut window_q: Query<Entity, With<PrimaryWindow>>,
    mut commands: Commands,
) {
    for window in window_q.iter_mut() {
        commands
            .entity(window)
            .insert(CursorIcon::System(SystemCursorIcon::Text));
    }
}

/// An observer that styles the cursor, used for all widgets.
/// This is not part of the Controller.
fn on_pointer_out_default_cursor(
    _event: On<Pointer<Out>>,
    mut window_q: Query<Entity, With<PrimaryWindow>>,
    mut commands: Commands,
) {
    for window in window_q.iter_mut() {
        commands
            .entity(window)
            .insert(CursorIcon::System(SystemCursorIcon::Default));
    }
}

// --- START TEXT INPUT -- //

/// Creates the text input that allows the user to input a name for the character.
fn name_text_input_row(character: &Character) -> impl Scene {
    bsn! {
        Node {
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            justify_content: JustifyContent::SpaceBetween,
        }
        Children [
            Node
            Children [
                Text::new("Name: ")
            ],

            name_text_input(character)
        ]
    }
}

fn name_text_input(character: &Character) -> impl Scene {
    let name = character.name.clone();
    bsn! {
        Node {
            width: px(200),
            border: px(5),
            border_radius: BorderRadius::all(px(10)),
            padding: UiRect::axes(px(5), px(2)),
        }
        NameInput
        TextInput
        EditableText::new(name)
        // TabIndex Component is necessary for the text input to receive focus for typing.
        TabIndex(0)
        // This is necessary so that the text cursor pops up when the input has focus.
        TextCursorStyle::default()
        BackgroundColor(Color::BLACK)
        on(on_pointer_over_text_cursor)
        on(on_pointer_out_default_cursor)
    }
}

/// A system that implements Controller logic to update the Name Input.
/// This particular system updates the Model upon any change in value to the text input's `EditableText`.
/// The text input widget does not regularly emit any events on value change. Instead, the
/// `EditableText`'s value can be polled under app-specific conditions to update the Model when appropriate.
fn on_changed_editable_text(
    name_input_q: Query<&EditableText, With<NameInput>>,
    mut character: ResMut<Character>,
) {
    let Ok(editable_text) = name_input_q.single_inner() else {
        return;
    };
    let new_name = editable_text.value().to_string();
    if character.name != new_name {
        character.name = new_name;
    }

    // We do not need to update the ui widget view because it updates itself.
    // However, that is not the norm with headless widgets!
}

// --- END TEXT INPUT -- //

// --- START RADIO GROUP -- //

/// Creates the radio group that allows the user to select a hat for the character.
fn hat_type_radio_group(character: &Character) -> impl Scene {
    bsn! {
        Node {
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            justify_content: JustifyContent::SpaceBetween,
        }
        RadioGroup
        HatTypeRadioGroup
        // This observer is part of the Controller -- it reacts to the user's input!
        on(on_value_change_hat_type)
        Children [
            Node
            Children [
                Text::new("Hat: ")
            ],

            {
                HAT_TYPES.iter()
                    .map(|hat_type| hat_type_radio_button(*hat_type, character))
                    .collect::<Vec<_>>()
            }
        ]
    }
}

fn hat_type_radio_button(hat_type: HatType, character: &Character) -> Box<dyn Scene> {
    let base_radio_button = || {
        bsn! {
            Node {
                border: px(5),
                border_radius: BorderRadius::all(px(10)),
                padding: UiRect::axes(px(5), px(2)),
            }
            RadioButton
            template_value(hat_type)
            BackgroundColor(Color::BLACK)
            on(on_pointer_over_pointer_cursor)
            on(on_pointer_out_default_cursor)
        }
    };
    if character.hat_type == hat_type {
        Box::new(bsn! {
            base_radio_button()
            // The selected hat_type must have the `Checked` component.
            Checked
            Children [
                Text::new(format!("{hat_type:?}"))
                // The selected hat_type's text is green as opposed to white.
                TextColor(palettes::basic::GREEN)
            ]
        })
    } else {
        Box::new(bsn! {
            base_radio_button()
            Children [
                Text::new(format!("{hat_type:?}"))
                TextColor(palettes::basic::WHITE)
            ]
        })
    }
}

/// This observer is part of the Controller logic.
/// This observer will update the `Character` Resource based on a change to the Hat Type Radio Group.
/// Radio groups emit a ValueChange<Entity> event when the user clicks on a radio button.
/// The value of the event is of the clicked radio button.
/// The source of the event is the parent radio group.
fn on_value_change_hat_type(
    event: On<ValueChange<Entity>>,
    hat_type_value_q: Query<(Entity, &HatType, Has<Checked>, &Children), With<RadioButton>>,
    hat_type_radio_group_q: Single<Entity, With<HatTypeRadioGroup>>,
    mut character: ResMut<Character>,
    mut commands: Commands,
) {
    // Ensure this value change event is for the Hat Type Radio Group
    // Although unnecessary in this example, apps with multiple radio groups need to distinguish
    // what the value change is for.
    if event.source != hat_type_radio_group_q.entity() {
        return;
    }

    let Ok((_, new_hat_type, has_checked, _)) = hat_type_value_q.get(event.value) else {
        return;
    };

    if has_checked {
        // The hat type has actually not changed, so we do not need to do anything.
        return;
    }

    // Update the Model
    character.hat_type = *new_hat_type;

    // Update the Widget portion of the View
    for (button_entity, hat_type, has_checked, children) in hat_type_value_q.iter() {
        if character.hat_type == *hat_type {
            commands.entity(button_entity).insert(Checked);
            // The radio button only has one child for the text and color of the button
            commands
                .entity(children[0])
                .insert(TextColor(palettes::basic::GREEN.into()));
        } else if has_checked {
            commands.entity(button_entity).remove::<Checked>();
            commands.entity(children[0]).insert(TextColor(Color::WHITE));
        }
    }
    // Because character has been modified, refresh_character will run and update the other half of the "View".
}

// --- END RADIO GROUP -- //

// --- START CHECK BOX -- //

/// Creates the checkbox that allows the user to toggle a yellow tint of the character.
fn tint_yellow_checkbox_row(character: &Character) -> impl Scene {
    bsn! {
        Node {
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            justify_content: JustifyContent::SpaceBetween,
        }
        Children [
            Node
            Children [
                Text::new("Tint Yellow: ")
            ],

            tint_yellow_checkbox(character)
        ]
    }
}

fn tint_yellow_checkbox(character: &Character) -> Box<dyn Scene> {
    let base_checkbox = || {
        bsn! {
            Node {
                padding: UiRect::horizontal(px(5)),
            }
            Checkbox
            TintYellowCheckbox
            BackgroundColor(Color::WHITE)
            on(on_pointer_over_pointer_cursor)
            on(on_pointer_out_default_cursor)
            // This observer is part of the controller -- it reacts to the user's input!
            on(on_value_change_tint_yellow)
        }
    };

    if character.tint_yellow {
        Box::new(bsn! {
            base_checkbox()
            Checked
            Children [
                Text::new("X")
                TextColor(palettes::basic::GREEN)
            ]
        })
    } else {
        Box::new(bsn! {
            base_checkbox()
            Children [
                Text::new(" ")
                TextColor(palettes::basic::GREEN)
            ]
        })
    }
}

/// This observer is part of the Controller logic.
/// This observer will update the `Character` Resource based on a change to the Tint Yellow Checkbox.
/// Checkboxes emit a ValueChange<bool> event when the user clicks on a checkbox.
/// The value of the event is the value of the toggle.
/// The source of the event is the checkbox.
fn on_value_change_tint_yellow(
    event: On<ValueChange<bool>>,
    tint_yellow_checkbox_q: Query<(Entity, &Children), With<TintYellowCheckbox>>,
    mut character: ResMut<Character>,
    mut commands: Commands,
) {
    let Ok((checkbox_entity, children)) = tint_yellow_checkbox_q.single_inner() else {
        return;
    };

    // Ensure this value change event is for the checkbox.
    // Although unnecessary in this example, apps with multiple checkboxes need to distinguish
    // what the value change is for.
    if event.source != checkbox_entity {
        return;
    }

    // Update the Model
    character.tint_yellow = event.value;

    // Update the Widget portion of the View
    if character.tint_yellow {
        commands.entity(event.source).insert(Checked);
        // The checkbox only has one child for the X and its color
        commands.entity(children[0]).insert(Text::new("X"));
    } else {
        commands.entity(event.source).remove::<Checked>();
        commands.entity(children[0]).insert(Text::new(" "));
    }
    // Because character has been modified, refresh_character will run and update the other half of the "View".
}

// --- END CHECK BOX -- //

// --- END WIDGET VIEW & CONTROLLER --- //

// --- START CHARACTER VIEW --- //

/// A system that updates the "View" whenever the "Model" has changed.
fn refresh_character(
    mut commands: Commands,
    query: Single<Entity, With<CharacterView>>,
    character: Res<Character>,
) {
    commands.entity(query.entity()).despawn();

    commands.spawn_scene(character_view(&character));
}

/// Returns the "View", powered by data from the `Character` Model.
fn character_view(character: &Character) -> impl Scene {
    bsn! {
        CharacterView
        Transform::from_xyz(320., 0., 0.)
        template_value(Visibility::Inherited)
        Children [
            character_sprite_tint(&character),
            character_hat(&character),
            character_name_and_age(&character),
        ]
    }
}

fn character_sprite_tint(character: &Character) -> Box<dyn Scene> {
    if character.tint_yellow {
        Box::new(bsn! {
            Sprite {
                image: "branding/icon.png",
                color: palettes::basic::YELLOW
            }
            Transform::default()
        })
    } else {
        Box::new(bsn! {
            Sprite {
                image: "branding/icon.png",
            }
            Transform::default()
        })
    }
}

fn character_hat(character: &Character) -> Box<dyn Scene> {
    match character.hat_type {
        HatType::None => Box::new(bsn! {}),
        HatType::TopHat => Box::new(bsn! {
            Transform::from_rotation(Quat::from_rotation_z(0.78))
            template_value(Visibility::Inherited)
            Children [
                Mesh2d(asset_value(Rectangle::new(
                    40., 10.
                )))
                MeshMaterial2d<ColorMaterial>(asset_value(ColorMaterial::from_color(Color::BLACK)))
                template_value(Transform::from_xyz(55., 60., 1.)),

                Mesh2d(asset_value(Rectangle::new(
                    20., 50.
                )))
                MeshMaterial2d<ColorMaterial>(asset_value(ColorMaterial::from_color(Color::BLACK)))
                template_value(Transform::from_xyz(55., 85., 1.)),
            ]
        }),
        HatType::DunceCap => Box::new(bsn! {
            Mesh2d(asset_value(Triangle2d::new(
                Vec2::new(0., 100.),
                Vec2::new(-20., 0.),
                Vec2::new(20., 0.)
            )))
            MeshMaterial2d<ColorMaterial>(asset_value(ColorMaterial::from_color(palettes::basic::TEAL)))
            // 0.78 radians ~ PI / 4
            template_value(Transform::from_xyz(0., 80., 1.).with_rotation(Quat::from_rotation_z(0.78)))
        }),
    }
}

fn character_name_and_age(character: &Character) -> impl Scene {
    let name = character.name.clone();
    let age = character.age;
    bsn! {
        Text2d::new(format!("Hi! My name is {name}.\nI am {age} years old."))
        template_value(Transform::from_xyz(0., -200., 0.))
    }
}

// --- END CHARACTER VIEW --- //
