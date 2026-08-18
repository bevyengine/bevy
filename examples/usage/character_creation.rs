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
    ui_widgets::{
        Checkbox, RadioButton, RadioGroup, Slider, SliderOrientation, SliderPrecision, SliderRange,
        SliderThumb, SliderValue, TextInput, TrackClick, ValueChange,
    },
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
                // Updates the Model if the user changed the name via text input.
                // This is a Controller system, and is not an Observer because of
                // the way the text input widget is designed.
                on_changed_editable_text,
                // Updates the View after any Model changes
                refresh_character.run_if(resource_exists_and::<Character>(|character| {
                    !character.changed_fields.is_empty()
                })),
            )
                .chain(),
        )
        .run();
}

fn setup(mut commands: Commands, character: Res<Character>) {
    commands.spawn_scene_list(bsn_list! {
        Camera2d,

        // This scene will serve as one half of our "View"
        // and most of the "Controller" in our MVC design.
        // The user interacts with UI widgets, which are processed by the "Controller"
        // via observers and systems. The observers and systems update the
        // state and also update the look of the UI widgets
        // (i.e. changing text color of a selected option, inserting an X).
        ui(&character),

        // This scene will serve as the other half of our "View" in our MVC design.
        // The user will see the character they are creating.
        character_view(&character),
    });
}

/// This resource serves as the "Model" in our MVC Design.
/// It serves as our source of truth for the character being created.
#[derive(Resource)]
struct Character {
    name: String,
    age: u32,
    hat_type: HatType,
    tint_yellow: bool,
    // This is used by the app to more efficiently queue refreshes
    // of the character view by only refreshing the necessary entities.
    changed_fields: Vec<ChangedField>,
}

impl Default for Character {
    fn default() -> Self {
        Character {
            name: "Bevy".into(),
            age: 5,
            hat_type: HatType::default(),
            tint_yellow: false,
            changed_fields: vec![],
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

#[derive(Clone, Copy, Debug, PartialEq)]
enum ChangedField {
    Name,
    Age,
    HatType,
    TintYellow,
}

const HAT_TYPES: [HatType; 3] = [HatType::None, HatType::TopHat, HatType::DunceCap];

// --- START MARKER COMPONENTS --- //

#[derive(Component, Clone, Default)]
struct NameInput;

#[derive(Component, Clone, Default)]
struct AgeSlider;

#[derive(Component, Clone, Default)]
struct AgeSliderThumb;

#[derive(Component, Clone, Default)]
struct AgeSliderText;

#[derive(Component, Clone, Default)]
struct HatTypeRadioGroup;

#[derive(Component, Clone, Default)]
struct TintYellowCheckbox;

#[derive(Component, Clone, Default)]
struct CharacterView;

#[derive(Component, Clone, Default)]
struct CharacterSprite;

#[derive(Component, Clone, Default)]
struct CharacterHat;

#[derive(Component, Clone, Default)]
struct CharacterNameAndAge;

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

                age_slider_row(character),

                hat_type_radio_group_row(character),

                tint_yellow_checkbox_row(character),
            ]
        ]
    }
}

// --- START TEXT INPUT -- //

/// Creates the text input row that allows the user to input a name for the character.
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
        character.changed_fields.push(ChangedField::Name);
    }

    // We do not need to update the ui widget view in our app because `EditableText`
    // manages its own state internally; it updates the text the user sees automatically.

    // Because character has been modified, refresh_character will run and update the other half of the "View".
}

// --- END TEXT INPUT -- //

// --- START SLIDER -- //

/// Creates the age slider that allows the user to input the age of the character.
fn age_slider_row(character: &Character) -> impl Scene {
    let age = character.age;
    bsn! {
        Node {
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            justify_content: JustifyContent::SpaceBetween,
            column_gap: px(10),
        }
        Children [
            Node
            Children [
                Text::new("Age:")
            ],

            age_slider(character),

            Node {
                width: px(30),
            }
            Children [
                AgeSliderText
                Text::new(format!("{}", age))
            ],
        ]
    }
}

fn age_slider(character: &Character) -> impl Scene {
    bsn! {
        Node {
            width: px(220),
            border: px(5),
            padding: UiRect::axes(px(5), px(2)),
        }
        AgeSlider
        Slider {
            track_click: TrackClick::Snap,
            orientation: SliderOrientation::Horizontal,
        }
        SliderValue({character.age as f32})
        // Move every whole number.
        SliderPrecision(0)
        SliderRange::new(1., 100.)
        BackgroundColor(Color::BLACK)
        // This observer is part of the Controller -- it reacts to the user's input!
        on(on_value_change_age_slider)
        on(on_pointer_over_pointer_cursor)
        on(on_pointer_drag_start_grabbing_cursor)
        on(on_pointer_drag_end_grab_cursor)
        on(on_pointer_out_default_cursor)
        Children [
            // Visible Slider Track
            // It is 220px in width via its parent.
            Node {
                height: px(5),
                border_radius: BorderRadius::all(px(3)),
            }
            BackgroundColor(Color::BLACK),

            // Invisible shorter track (does not have background color) that the
            // SliderThumb glides on. This is so that the thumb
            // does not go past the left and right sides of the visible slider track.
            Node {
                display: Display::Flex,
                position_type: PositionType::Absolute,
                left: px(0)
                // Shortened by the slider thumb's width on the right side.
                // This means that it is 200px in width.
                right: px(20),
                top: px(0),
                bottom: px(0),
            }
            Children [
                AgeSliderThumb
                SliderThumb
                Node {
                    display: Display::Flex,
                    width: px(20),
                    height: px(10),
                    position_type: PositionType::Absolute,
                    // Where the thumb is along the track will be updated by `on_value_change_age_slider`
                    left: percent((character.age as f32 - 1.) / (100. - 1.) * 100.),
                }
                BackgroundColor(Color::WHITE)
                on(on_pointer_over_grab_cursor)
                on(on_pointer_out_default_cursor)
                on(on_pointer_drag_start_grabbing_cursor)
                on(on_pointer_drag_end_grab_cursor)
            ]
        ]
    }
}

/// A system that implements Controller logic to update the Age.
/// This particular system updates the Model upon any change in value to the Age Slider.
/// Sliders emit a `ValueChange<f32>` event when the user drags the slider.
/// The value of the event is the new value of the slider.
/// The source of the event is the `Slider` parent entity.
fn on_value_change_age_slider(
    event: On<ValueChange<f32>>,
    age_slider_q: Query<(Entity, &SliderRange), With<AgeSlider>>,
    mut age_slider_text_q: Query<&mut Text, With<AgeSliderText>>,
    mut age_slider_thumb_q: Query<&mut Node, With<AgeSliderThumb>>,
    mut character: ResMut<Character>,
    mut commands: Commands,
) {
    let Ok((entity, slider_range)) = age_slider_q.single_inner() else {
        return;
    };
    if event.source != entity {
        return;
    }

    // Update the Model
    // `SliderPrecision` ensures that this value is a whole number.
    character.age = event.value as u32;
    character.changed_fields.push(ChangedField::Age);

    // Update the Widget portion of the View
    commands
        .entity(event.source)
        .insert(SliderValue(character.age as f32));
    for mut node in age_slider_thumb_q.iter_mut() {
        node.left = percent(slider_range.thumb_position(character.age as f32) * 100.0);
    }
    for mut text in age_slider_text_q.iter_mut() {
        *text = Text::new(format!("{}", character.age));
    }

    // Because character has been modified, refresh_character will run and update the other half of the "View".
}

// --- END SLIDER -- //

// --- START RADIO GROUP -- //

/// Creates the radio group row that allows the user to select a hat for the character.
fn hat_type_radio_group_row(character: &Character) -> impl Scene {
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
/// Radio groups emit a `ValueChange<Entity>` event when the user clicks on a radio button.
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
    character.changed_fields.push(ChangedField::HatType);

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

/// Creates the checkbox row that allows the user to toggle a yellow tint of the character.
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
/// Checkboxes emit a `ValueChange<bool>` event when the user clicks on a checkbox.
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
    character.changed_fields.push(ChangedField::TintYellow);

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

/// Returns the "View", powered by data from the `Character` Model.
fn character_view(character: &Character) -> impl Scene {
    bsn! {
        CharacterView
        Transform::from_xyz(320., 0., 0.)
        template_value(Visibility::Inherited)
        Children [
            character_sprite(&character),

            character_hat(&character),

            character_name_and_age(&character),
        ]
    }
}

/// A system that updates the "View" whenever the underlying `Character` "Model" has changed.
fn refresh_character(
    mut commands: Commands,
    character_view_q: Single<(Entity, &Children), With<CharacterView>>,
    view_type_q: Query<(
        Entity,
        Has<CharacterSprite>,
        Has<CharacterHat>,
        Has<CharacterNameAndAge>,
    )>,
    mut character: ResMut<Character>,
) {
    let (character_view, children) = character_view_q.into_inner();
    let (mut already_updated_name_age, mut already_updated_sprite, mut already_updated_hat) =
        (false, false, false);
    for changed_field in character.changed_fields.iter().copied() {
        // First, find the correct child to despawn
        // Then, add an updated child.
        match changed_field {
            ChangedField::Name | ChangedField::Age if !already_updated_name_age => {
                for (child, _, _, is_name_and_age) in children
                    .iter()
                    .filter_map(|child| view_type_q.get(child).ok())
                {
                    if is_name_and_age {
                        commands.entity(child).try_despawn();
                    }
                }
                let new_child = commands
                    .spawn_scene(character_name_and_age(&character))
                    .id();
                commands.entity(character_view).add_child(new_child);

                already_updated_name_age = true;
            }
            ChangedField::TintYellow if !already_updated_sprite => {
                for (child, is_sprite, _, _) in children
                    .iter()
                    .filter_map(|child| view_type_q.get(child).ok())
                {
                    if is_sprite {
                        commands.entity(child).try_despawn();
                    }
                }
                let new_child = commands.spawn_scene(character_sprite(&character)).id();
                commands.entity(character_view).add_child(new_child);

                already_updated_sprite = true;
            }
            ChangedField::HatType if !already_updated_hat => {
                for (child, _, is_hat, _) in children
                    .iter()
                    .filter_map(|child| view_type_q.get(child).ok())
                {
                    if is_hat {
                        commands.entity(child).try_despawn();
                    }
                }
                let new_child = commands.spawn_scene(character_hat(&character)).id();
                commands.entity(character_view).add_child(new_child);

                already_updated_hat = true;
            }
            _ => {}
        }
    }
    character.changed_fields.clear();
}

fn character_sprite(character: &Character) -> Box<dyn Scene> {
    if character.tint_yellow {
        Box::new(bsn! {
            CharacterSprite
            Sprite {
                image: "branding/icon.png",
                color: palettes::basic::YELLOW
            }
            Transform::default()
        })
    } else {
        Box::new(bsn! {
            CharacterSprite
            Sprite {
                image: "branding/icon.png",
            }
            Transform::default()
        })
    }
}

fn character_hat(character: &Character) -> Box<dyn Scene> {
    match character.hat_type {
        HatType::None => Box::new(bsn! {
            CharacterHat
        }),
        HatType::TopHat => Box::new(bsn! {
            CharacterHat
            // 0.78 radians ~ PI / 4
            Transform::from_rotation(Quat::from_rotation_z(0.78))
            template_value(Visibility::Inherited)
            Children [
                // bottom wider portion of the top hat.
                Mesh2d(asset_value(Rectangle::new(
                    40., 10.
                )))
                MeshMaterial2d<ColorMaterial>(asset_value(ColorMaterial::from_color(Color::BLACK)))
                template_value(Transform::from_xyz(55., 60., 1.)),

                // top longer portion of the top hat
                Mesh2d(asset_value(Rectangle::new(
                    20., 50.
                )))
                MeshMaterial2d<ColorMaterial>(asset_value(ColorMaterial::from_color(Color::BLACK)))
                template_value(Transform::from_xyz(55., 85., 1.)),
            ]
        }),
        HatType::DunceCap => Box::new(bsn! {
            CharacterHat
            Mesh2d(asset_value(Triangle2d::new(
                Vec2::new(0., 100.),
                Vec2::new(-20., 0.),
                Vec2::new(20., 0.)
            )))
            MeshMaterial2d<ColorMaterial>(asset_value(ColorMaterial::from_color(palettes::basic::TEAL)))
            template_value(Transform::from_xyz(0., 80., 1.).with_rotation(Quat::from_rotation_z(0.78)))
        }),
    }
}

fn character_name_and_age(character: &Character) -> impl Scene {
    let name = character.name.clone();
    let age = character.age;
    let years = if age == 1 { "year" } else { "years" };
    bsn! {
        CharacterNameAndAge
        Text2d::new(format!("Hi! My name is {name}.\nI am {age} {years} old."))
        template_value(Transform::from_xyz(0., -200., 0.))
    }
}

// --- END CHARACTER VIEW --- //

// --- START MISC OBSERVERS (STYLING) --- //

/// An observer that styles the cursor, used primarily for text input.
/// This is not part of the Controller.
fn on_pointer_over_text_cursor(
    mut event: On<PointerOver>,
    mut window_q: Query<Entity, With<PrimaryWindow>>,
    mut commands: Commands,
) {
    for window in window_q.iter_mut() {
        commands
            .entity(window)
            .insert(CursorIcon::System(SystemCursorIcon::Text));
    }

    event.propagate(false);
}

/// An observer that styles the cursor, used primarily for sliders.
/// This is not part of the Controller.
fn on_pointer_over_grab_cursor(
    mut event: On<PointerOver>,
    mut window_q: Query<Entity, With<PrimaryWindow>>,
    mut commands: Commands,
) {
    for window in window_q.iter_mut() {
        commands
            .entity(window)
            .insert(CursorIcon::System(SystemCursorIcon::Grab));
    }
    event.propagate(false);
}

/// An observer that styles the cursor, used primarily for sliders.
/// This is not part of the Controller.
fn on_pointer_drag_start_grabbing_cursor(
    _event: On<PointerDragStart>,
    mut window_q: Query<Entity, With<PrimaryWindow>>,
    mut commands: Commands,
) {
    for window in window_q.iter_mut() {
        commands
            .entity(window)
            .insert(CursorIcon::System(SystemCursorIcon::Grabbing));
    }
    // Note that this event does not stop propagation!
    // This is because the slider widget processes drag events in order to
    // emit value change events.
}

/// An observer that styles the cursor, used primarily for sliders.
/// This is not part of the Controller.
fn on_pointer_drag_end_grab_cursor(
    _event: On<PointerDragEnd>,
    mut window_q: Query<Entity, With<PrimaryWindow>>,
    mut commands: Commands,
) {
    for window in window_q.iter_mut() {
        commands
            .entity(window)
            .insert(CursorIcon::System(SystemCursorIcon::Grab));
    }
    // Note that this event does not stop propagation!
    // This is because the slider widget processes drag events in order to
    // emit value change events.
}

/// An observer that styles the cursor, used primarily for buttons / checkboxes.
/// This is not part of the Controller.
fn on_pointer_over_pointer_cursor(
    mut event: On<PointerOver>,
    mut window_q: Query<Entity, With<PrimaryWindow>>,
    mut commands: Commands,
) {
    for window in window_q.iter_mut() {
        commands
            .entity(window)
            .insert(CursorIcon::System(SystemCursorIcon::Pointer));
    }
    event.propagate(false);
}

/// An observer that styles the cursor, used for all widgets.
/// This is not part of the Controller.
fn on_pointer_out_default_cursor(
    mut event: On<PointerOut>,
    mut window_q: Query<Entity, With<PrimaryWindow>>,
    mut commands: Commands,
) {
    for window in window_q.iter_mut() {
        commands
            .entity(window)
            .insert(CursorIcon::System(SystemCursorIcon::Default));
    }
    event.propagate(false);
}

// --- END MISC OBSERVERS (STYLING) --- //
