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
    prelude::*,
    ui::Checked,
    ui_widgets::{RadioGroup, RadioButton, Checkbox, TextInput, ValueChange},
};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .init_resource::<Character>()
        .add_systems(Startup, setup)
        .run();
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
/// This derives `Component` so that they can be placed on the individual radio buttons
/// for ease of identifying which radio button was selected.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq)]
enum HatType {
    #[default]
    None,
    TopHat,
    DunceCap,
}

const HAT_TYPES: [HatType; 3] = [HatType::None, HatType::TopHat, HatType::DunceCap];

/// Marker Component for the Name Input Field
#[derive(Component, Clone, Default)]
pub struct NameInput;

/// Marker Component for the Age Slider
#[derive(Component, Clone, Default)]
pub struct AgeSlider;

/// Marker Component for the HatType Radio Group
#[derive(Component, Clone, Default)]
pub struct HatTypeRadioGroup;

/// Marker Component for the Tint Yellow Checkbox
#[derive(Component, Clone, Default)]
pub struct TintYellowCheckbox;

/// Marker Component for the Character View
#[derive(Component, Clone, Default)]
pub struct CharacterView;

fn setup(mut commands: Commands, character: Res<Character>) {
    commands.spawn_scene_list(bsn_list! {
        Camera2d,

        // This scene will serve as our "Controller" in our MVC design.
        // The user interacts with the controller, which will affect the
        // "View" (the rendered character).
        ui(&character),

        // This scene will serve as our "View" in our MVC design.
        // The user can see the character they are creating.
        base_character_view(&character),
    })
}

/// Spawns the ui widgets that will serve as the "Controller" for the
/// character "Model".
fn ui(character: &Character) -> impl Scene {
    bsn! {
        // ui will take up the left third of the screen.
        Node {
            flex_direction: FlexDirection::Column,
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            height: percent(100)
            width: percent(33),
        }
        Children [
            // Holds the character creation pane
            Node {
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                width: percent(100),
                border_radius: BorderRadius::all(px(5)),
            }
            BackgroundColor(palettes::basic::GRAY)
            Children [
                hat_type_radio_group(character),
            ]
        ]
    }
}

fn hat_type_radio_group(character: &Character) -> impl Scene {
    bsn! {
        Node {
            flex_direction: FlexDirection::Row,
        }
        RadioGroup
        HatTypeRadioGroup
        Children [
            Node
            Children [
                Text::new("Hat: ")
            ],

            {
                HAT_TYPES.iter().map(|hat_type| hat_type_radio_button(*hat_type, character)).collect::<Vec<_>>()
            }
        ]
    }
}

fn hat_type_radio_button(hat_type: HatType, character: &Character) -> Box<dyn Scene> {
    if character.hat_type == hat_type {
        Box::new(bsn! {
            Node {
                border: px(5),
            }
            RadioButton
            Checked
            template_value(hat_type)
            Children [
                Text::new(format!("{hat_type:?}"))
                TextColor(palettes::basic::GREEN)
            ]
            BackgroundColor(Color::BLACK)
        })
    } else {
        Box::new(bsn! {
            Node {
                border: px(5),
            }
            RadioButton
            template_value(hat_type)
            Children [
                Text::new(format!("{hat_type:?}"))
                TextColor(palettes::basic::WHITE)
            ]
            BackgroundColor(Color::BLACK)
        })
    }
}

fn base_character_view(character: &Character) -> impl Scene {
    bsn! {
        CharacterView
        Children [
            Sprite {
                image: "branding/icon.png"
            }
            // Transform::from_xyz(426., 0., 0.)
        ]
    }
}
