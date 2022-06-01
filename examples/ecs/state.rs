//! This example illustrates how to use [`States`] to control transitioning from a `Menu` state to
//! an `InGame` state.

use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
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
    mut state: ResMut<State<AppState>>,
    mut commands: Commands,
    asset_server: Res<AssetServer>,
) {
    let is_first_load = !state.inactives().contains(&AppState::InGame);
    let button_entity = create_button(
        &mut commands,
        &*asset_server,
        if is_first_load { "Play" } else { "Restart" },
    );

    let mut continue_entity = None;
    if is_first_load {
        // ui camera
        commands.spawn_bundle(UiCameraBundle::default());
    } else {
        continue_entity = Some(create_button(&mut commands, &*asset_server, "Continue"));
    }

    commands.insert_resource(MenuData {
        button_entity,
        continue_entity,
    });
}

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
    mut interaction_query: Query<
        (Entity, &Interaction, &mut UiColor),
        (Changed<Interaction>, With<Button>),
    >,
) {
    for (entity, interaction, mut color) in interaction_query.iter_mut() {
        match *interaction {
            Interaction::Clicked => {
                *color = PRESSED_BUTTON.into();
                if state.inactives().contains(&AppState::InGame) {
                    if menu_data.continue_entity.is_some()
                        && menu_data.continue_entity.unwrap() == entity
                    {
                        state.pop();
                    } else {
                        state.replace(AppState::InGame);
                    }
                } else {
                    state.set(AppState::InGame).unwrap();
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
fn back_to_menu(time: Res<Time>, mut state: ResMut<State<AppState>>, input: Res<Input<KeyCode>>) {
    if input.pressed(KeyCode::Escape) {
        state.push(AppState::Menu);
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
