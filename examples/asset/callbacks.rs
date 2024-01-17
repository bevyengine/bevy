//! Demonstrates the use of [`Callback`]s, which run once when triggered.
//!
//! These can be useful to help structure your logic in a push-based fashion,
//! reducing the overhead of running extremely rarely run systems
//! and improving schedule flexibility. Their advantage over [`SystemId`]s is
//! that they are [`Asset`]s.
//!
//! See the [`RunCallbackWorld::run_callback`](bevy::prelude::RunCallbackWorld::run_callback)
//! docs for more details.

use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        // `init_asset` must be called for new types of callbacks.
        // By default, only `Callback` (no input or output) is inited.
        .init_asset::<Callback<PressedDown>>()
        .add_systems(Startup, setup)
        .add_systems(Update, evaluate_callbacks)
        .run();
}

// Need to store the `Handle` to the callback
#[derive(Component)]
struct OnPressedDown(Handle<Callback<PressedDown>>);

// Input and output of `Callback`s must implement `TypePath`
#[derive(TypePath)]
struct PressedDown(Entity);

// Need mutible access to `Assets` to add a callback
fn setup(mut commands: Commands, mut callbacks: ResMut<Assets<Callback<PressedDown>>>) {
    // Camera
    commands.spawn(Camera2dBundle::default());

    // root node
    commands
        .spawn(NodeBundle {
            style: Style {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            ..default()
        })
        .with_children(|parent| {
            // button
            parent
                .spawn((
                    ButtonBundle {
                        style: Style {
                            width: Val::Px(250.0),
                            height: Val::Px(65.0),
                            margin: UiRect::all(Val::Px(20.0)),
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            ..default()
                        },
                        background_color: Color::TEAL.into(),
                        ..default()
                    },
                    // Add callback
                    OnPressedDown(callbacks.add(Callback::from_system(toggle))),
                ))
                .with_children(|parent| {
                    // text
                    parent.spawn(TextBundle::from_section(
                        "false",
                        TextStyle {
                            font_size: 40.0,
                            color: Color::BLACK,
                            ..default()
                        },
                    ));
                });
        });
}

// `Callback`s can be created from any system.
fn toggle(
    In(pressed_down): In<PressedDown>,
    mut text: Query<&mut Text>,
    children: Query<&Children>,
    mut value: Local<bool>,
) {
    *value = !*value;
    let children = children.get(pressed_down.0).unwrap();
    let mut text = text.iter_many_mut(children);
    let mut text = text.fetch_next().unwrap();
    text.sections[0].value = value.to_string();
}

/// Runs the systems associated with each `OnPressedDown` component if button is pressed.
///
/// This could be done in an exclusive system rather than using `Commands` if preferred.
fn evaluate_callbacks(
    on_pressed_down: Query<(Entity, &OnPressedDown, &Interaction), Changed<Interaction>>,
    mut commands: Commands,
) {
    for (entity, on_button_pressed, interaction) in &on_pressed_down {
        if *interaction == Interaction::Pressed {
            commands.run_callback_with_input(on_button_pressed.0.clone(), PressedDown(entity));
        }
    }
}
