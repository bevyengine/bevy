//! A simple UI containing several buttons that modify a counter, to demonstrate keyboard navigation

use bevy::ui::Focusable;
use bevy::{prelude::*, winit::WinitSettings};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        // Only run the app when there is user input. This will significantly reduce CPU/GPU use.
        .insert_resource(WinitSettings::desktop_app())
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (update_button_style, update_focus_style, button_trigger),
        )
        .run();
}

const NORMAL_BUTTON: Color = Color::rgb(0.15, 0.15, 0.15);
const HOVERED_BUTTON: Color = Color::rgb(0.25, 0.25, 0.25);
const PRESSED_BUTTON: Color = Color::rgb(0.35, 0.75, 0.35);

const NORMAL_BORDER: Color = Color::BLACK;
const FOCUS_BORDER: Color = Color::BLUE;

#[derive(Component)]
enum CounterChange {
    Add,
    Subtract,
    Reset,
}

#[derive(Resource)]
struct Counter(i32);

#[derive(Component)]
struct CounterText;

fn update_button_style(
    mut interaction_query: Query<(&Interaction, &mut BackgroundColor), Changed<Interaction>>,
) {
    for (interaction, mut color) in &mut interaction_query {
        *color = match *interaction {
            Interaction::Pressed => PRESSED_BUTTON,
            Interaction::Hovered => HOVERED_BUTTON,
            Interaction::None => NORMAL_BUTTON,
        }
        .into();
    }
}

fn update_focus_style(
    mut focusable_query: Query<(&Focusable, &mut BorderColor), Changed<Focusable>>,
) {
    for (focusable, mut color) in &mut focusable_query {
        info!("Update focus {focusable:?}");
        *color = if focusable.is_focus_visible() {
            FOCUS_BORDER.into()
        } else {
            NORMAL_BORDER.into()
        };
    }
}

fn button_trigger(
    mut interaction_query: Query<(&Interaction, &CounterChange), Changed<Interaction>>,
    mut text_query: Query<&mut Text, With<CounterText>>,
    mut counter: ResMut<Counter>,
) {
    for (interaction, counter_change) in &mut interaction_query {
        if matches!(interaction, Interaction::Pressed) {
            match counter_change {
                CounterChange::Add => counter.0 += 1,
                CounterChange::Subtract => counter.0 -= 1,
                CounterChange::Reset => counter.0 = 0,
            }
            text_query.single_mut().sections[0].value = format!("Counter: {}", counter.0);
        }
    }
}

fn text(text: &str, parent: &mut ChildBuilder, counter: bool) {
    let text_bundle = TextBundle::from_section(
        text,
        TextStyle {
            font_size: 40.0,
            color: Color::rgb(0.9, 0.9, 0.9),
            ..default()
        },
    );
    if counter {
        parent.spawn((text_bundle, CounterText))
    } else {
        parent.spawn(text_bundle)
    };
}

fn spawn_button(name: &str, parent: &mut ChildBuilder, counter_change: CounterChange) {
    parent
        .spawn((
            counter_change,
            ButtonBundle {
                style: Style {
                    width: Val::Px(175.0),
                    height: Val::Px(65.0),
                    border: UiRect::all(Val::Px(5.0)),
                    margin: UiRect::all(Val::Px(5.0)),
                    // horizontally center child text
                    justify_content: JustifyContent::Center,
                    // vertically center child text
                    align_items: AlignItems::Center,
                    ..default()
                },
                border_color: BorderColor(NORMAL_BORDER),
                background_color: NORMAL_BUTTON.into(),
                ..default()
            },
        ))
        .with_children(|parent| {
            text(name, parent, false);
        });
}

fn setup(mut commands: Commands) {
    commands.insert_resource(Counter(0));
    // ui camera
    commands.spawn(Camera2dBundle::default());
    commands
        .spawn(NodeBundle {
            style: Style {
                width: Val::Percent(100.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                flex_direction: FlexDirection::Column,
                ..default()
            },
            ..default()
        })
        .with_children(|parent| {
            text("Counter: 0", parent, true);
            parent.spawn(NodeBundle::default()).with_children(|parent| {
                spawn_button("Add", parent, CounterChange::Add);
                spawn_button("Subtract", parent, CounterChange::Subtract);
                spawn_button("Reset", parent, CounterChange::Reset);
            });
        });
}
