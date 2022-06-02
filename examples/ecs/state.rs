//! This example illustrates how to use [`States`] to control transitioning from a `Menu` state to
//! an `InGame` state.
//!
//! Use arrow keys to move the bevy icon, then escape key to show the menu again.
//!
//! The `Menu` state systems use a `MenuConfiguration` resource to adapt its behaviour and allow for restart or continue.

use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .insert_resource(MenuConfiguration {
            is_continue_available: false,
        })
        .add_state(AppState::Menu)
        .add_system_set(SystemSet::on_enter(AppState::Menu).with_system(setup_menu))
        .add_system_set(SystemSet::on_update(AppState::Menu).with_system(menu))
        .add_system_set(SystemSet::on_exit(AppState::Menu).with_system(cleanup_menu))
        .add_system_set(SystemSet::on_enter(AppState::InGame).with_system(setup_game))
        .add_system_set(
            SystemSet::on_update(AppState::InGame)
                .with_system(back_to_menu)
                .with_system(movement)
                .with_system(change_color),
        )
        .add_system_set(SystemSet::on_exit(AppState::InGame).with_system(cleanup_game))
        .run();
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
enum AppState {
    Menu,
    InGame,
}

struct MenuConfiguration {
    is_continue_available: bool,
}

struct MenuData {
    button_entity: Entity,
    continue_entity: Option<Entity>,
}

#[derive(Component)]
struct RemoveWhenGameDone;

const NORMAL_BUTTON: Color = Color::rgb(0.15, 0.15, 0.15);
const HOVERED_BUTTON: Color = Color::rgb(0.25, 0.25, 0.25);
const PRESSED_BUTTON: Color = Color::rgb(0.35, 0.75, 0.35);

fn setup_menu(
    menu_configuration: Res<MenuConfiguration>,
    mut commands: Commands,
    asset_server: Res<AssetServer>,
) {
    let button_entity = create_button(
        &mut commands,
        &*asset_server,
        if menu_configuration.is_continue_available {
            "Restart"
        } else {
            "Play"
        },
    );

    let mut continue_entity = None;
    if menu_configuration.is_continue_available {
        continue_entity = Some(create_button(&mut commands, &*asset_server, "Continue"));
    } else {
        // ui camera
        commands.spawn_bundle(UiCameraBundle::default());
    }

    commands.insert_resource(MenuData {
        button_entity,
        continue_entity,
    });
}

/// A utility function to easily create similar looking buttons
fn create_button(commands: &mut Commands, asset_server: &AssetServer, title: &str) -> Entity {
    commands
        .spawn_bundle(ButtonBundle {
            style: Style {
                size: Size::new(Val::Px(150.0), Val::Px(65.0)),
                // center button
                margin: UiRect::all(Val::Auto),
                // horizontally center child text
                justify_content: JustifyContent::Center,
                // vertically center child text
                align_items: AlignItems::Center,
                ..default()
            },
            color: NORMAL_BUTTON.into(),
            ..default()
        })
        .with_children(|parent| {
            parent.spawn_bundle(TextBundle {
                text: Text::with_section(
                    title,
                    TextStyle {
                        font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                        font_size: 40.0,
                        color: Color::rgb(0.9, 0.9, 0.9),
                    },
                    Default::default(),
                ),
                ..default()
            });
        })
        .id()
}

fn menu(
    mut state: ResMut<State<AppState>>,
    menu_data: Res<MenuData>,
    menu_configuration: Res<MenuConfiguration>,
    mut interaction_query: Query<
        (Entity, &Interaction, &mut UiColor),
        (Changed<Interaction>, With<Button>),
    >,
) {
    for (entity, interaction, mut color) in interaction_query.iter_mut() {
        match *interaction {
            Interaction::Clicked => {
                *color = PRESSED_BUTTON.into();
                if menu_configuration.is_continue_available {
                    if menu_data.continue_entity.unwrap() == entity {
                        state.pop().expect("Could not modify state.");
                    } else {
                        state
                            .replace(AppState::InGame)
                            .expect("Could not modify state.");
                    }
                } else {
                    state
                        .set(AppState::InGame)
                        .expect("Could not modify state.");
                }
            }
            Interaction::Hovered => {
                *color = HOVERED_BUTTON.into();
            }
            Interaction::None => {
                *color = NORMAL_BUTTON.into();
            }
        }
    }
}

fn cleanup_menu(mut commands: Commands, menu_data: Res<MenuData>) {
    commands.entity(menu_data.button_entity).despawn_recursive();
    if menu_data.continue_entity.is_some() {
        commands
            .entity(menu_data.continue_entity.unwrap())
            .despawn_recursive();
    }
}

fn cleanup_game(mut commands: Commands, query_to_remove: Query<Entity, With<RemoveWhenGameDone>>) {
    for entity in query_to_remove.iter() {
        commands.entity(entity).despawn();
    }
}

fn setup_game(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands
        .spawn_bundle(OrthographicCameraBundle::new_2d())
        .insert(RemoveWhenGameDone);
    commands
        .spawn_bundle(SpriteBundle {
            texture: asset_server.load("branding/icon.png"),
            ..default()
        })
        .insert(RemoveWhenGameDone);
}

fn back_to_menu(
    mut menu_configuration: ResMut<MenuConfiguration>,
    mut state: ResMut<State<AppState>>,
    input: Res<Input<KeyCode>>,
) {
    if input.pressed(KeyCode::Escape) {
        menu_configuration.is_continue_available = true;
        state.push(AppState::Menu).expect("Could not push state.");
    }
}

const SPEED: f32 = 100.0;

fn movement(
    time: Res<Time>,
    input: Res<Input<KeyCode>>,
    mut query: Query<&mut Transform, With<Sprite>>,
) {
    for mut transform in query.iter_mut() {
        let mut direction = Vec3::ZERO;
        if input.pressed(KeyCode::Left) {
            direction.x -= 1.0;
        }
        if input.pressed(KeyCode::Right) {
            direction.x += 1.0;
        }
        if input.pressed(KeyCode::Up) {
            direction.y += 1.0;
        }
        if input.pressed(KeyCode::Down) {
            direction.y -= 1.0;
        }

        if direction != Vec3::ZERO {
            transform.translation += direction.normalize() * SPEED * time.delta_seconds();
        }
    }
}

fn change_color(time: Res<Time>, mut query: Query<&mut Sprite>) {
    for mut sprite in query.iter_mut() {
        sprite
            .color
            .set_b((time.seconds_since_startup() * 0.5).sin() as f32 + 2.0);
    }
}
