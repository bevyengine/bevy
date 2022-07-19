use bevy::prelude::*;

use bevy_ui_navigation::{DefaultNavigationPlugins, FocusState, Focusable, NavRequestSystem};

/// This example shows wrapping at screen edge, even when there are off-screen focusables.
fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(DefaultNavigationPlugins)
        .add_startup_system(setup)
        .add_system(button_system.after(NavRequestSystem))
        .run();
}

#[derive(Component)]
struct IdleColor(UiColor);

fn button_system(
    mut interaction_query: Query<(&Focusable, &mut UiColor, &IdleColor), Changed<Focusable>>,
) {
    for (focusable, mut material, IdleColor(idle_color)) in interaction_query.iter_mut() {
        if let FocusState::Focused = focusable.state() {
            *material = Color::WHITE.into();
        } else {
            *material = *idle_color;
        }
    }
}

fn setup(mut commands: Commands) {
    let top = 30;
    let as_rainbow = |i: u32| Color::hsl((i as f32 / top as f32) * 360.0, 0.9, 0.5);
    // ui camera
    commands.spawn_bundle(Camera2dBundle::default());
    for i in 0..top {
        for j in 0..top {
            let full = (i + j).max(1);
            spawn_button(&mut commands, as_rainbow((i * j) % full).into(), top, i, j);
        }
    }
}
fn spawn_button(commands: &mut Commands, color: UiColor, max: u32, i: u32, j: u32) {
    let size = 340.0 / max as f32;
    commands
        .spawn_bundle(ButtonBundle {
            color,
            style: Style {
                size: Size::new(Val::Percent(size), Val::Percent(size)),
                position_type: PositionType::Absolute,
                position: UiRect {
                    bottom: Val::Percent((400.0 / max as f32) * i as f32),
                    left: Val::Percent((400.0 / max as f32) * j as f32),
                    ..default()
                },
                ..default()
            },
            ..default()
        })
        .insert(Focusable::default())
        .insert(IdleColor(color));
}
