//! This example illustrates how to manage data input from select
//! widgets in the `ui_widgets` crate.
//!
//! When a UI widget is interacted with by an application user,
//! various events are triggered that that the user presumably wants
//! to be recognized by the application. It is up to the application to
//! handle and interpret these events, which usually means updating its own internal
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
    prelude::*,
    ui::Checked,
    ui_widgets::{Checkbox, RadioButton, RadioGroup, TextInput, ValueChange},
};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .init_resource::<Character>()
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            refresh_character.run_if(resource_changed::<Character>),
        )
        .run();
}

fn setup(mut commands: Commands, character: Res<Character>) {
    commands.spawn_scene_list(bsn_list! {
        Camera2d,

        // This scene will serve as our "Controller" in our MVC design.
        // The user interacts with the controller, which will affect the
        // "View" (the rendered character).
        ui(&character),

        // This scene will serve as our "View" in our MVC design.
        // The user will see the character they are creating.
        character_view(&character),
    })
}

/// This resource serves as the "Model" in our MVC Design.
/// It serves as our source of truth for the character being created.
#[derive(Resource)]
pub struct Character {
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

/// --- START MARKER COMPONENTS --- ///

#[derive(Component, Clone, Default)]
pub struct NameInput;

#[derive(Component, Clone, Default)]
pub struct AgeSlider;

#[derive(Component, Clone, Default)]
pub struct HatTypeRadioGroup;

#[derive(Component, Clone, Default)]
pub struct TintYellowCheckbox;

#[derive(Component, Clone, Default)]
pub struct CharacterView;

/// --- END MARKER COMPONENTS --- ///

/// --- CONTROLLER --- ///

/// Spawns the ui widgets that will serve as the "Controller"
/// in our MVC design.
fn ui(character: &Character) -> impl Scene {
    bsn! {
        // UI will take up the left third of the screen.
        Node {
            flex_direction: FlexDirection::Column,
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            height: percent(100)
            width: percent(33),
        }
        Children [
            // The character creation pane
            Node {
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::SpaceEvenly,
                align_items: AlignItems::Center,
                margin: percent(5),
                width: percent(90),
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

                // Hat type radio group
                hat_type_radio_group(character)
                // This observer updates the Model based on user input.
                on(on_value_change_hat_type),

            ]
        ]
    }
}

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

    // Update the Controller
    for (button_entity, hat_type, has_checked, children) in hat_type_value_q.iter() {
        if character.hat_type == *hat_type {
            commands.entity(button_entity).insert(Checked);
            commands
                .entity(children[0])
                .insert(TextColor(palettes::basic::GREEN.into()));
        } else if has_checked {
            commands.entity(button_entity).remove::<Checked>();
            commands.entity(children[0]).insert(TextColor(Color::WHITE));
        }
    }
    // Because character has been modified, refresh_character will run and update the "View".
}

/// --- VIEW --- ///

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
        Transform::default()
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
