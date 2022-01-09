use bevy::{
    ecs::schedule::SystemSet, prelude::*, gltf::Gltf,
};

/// This example illustrates, how a loading screen can be implemented. It has an animated spinner and listens for an AssetEvent.


#[derive(Clone, Eq, PartialEq, Debug, Hash)]
enum GameState {
    Loading,
    Playing,
}

fn main() {
    App::new()
        .insert_resource(Msaa { samples: 4 })
        .add_plugins(DefaultPlugins)
        .add_state(GameState::Loading)
        .add_startup_system(setup_loading_screen)
        .add_startup_system(setup_game)
        .add_system_set(SystemSet::on_update(GameState::Loading).with_system(animate_spinner))
        .add_system_set(SystemSet::on_update(GameState::Loading).with_system(asset_listening_system))
        .add_system_set(SystemSet::on_exit(GameState::Loading).with_system(close_loading_screen))
        .add_system_set(SystemSet::on_enter(GameState::Playing).with_system(game_system))
        .run();
}

#[derive(Component)]
pub struct LoadingScreen;
const LOADING_SCREEN_DEFAULT_TEXT: &str = "Loading...";

// Setup our simple loading screen, it uses an image as spinner and a loading text bellow it.
fn setup_loading_screen(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
) {
    // UI camera
    commands.spawn_bundle(UiCameraBundle::default());
    commands
        // NodeBundle used as background
        .spawn_bundle(NodeBundle {
            style: Style {
                size: Size::new(Val::Percent(100.0), Val::Percent(100.0)),
                position_type: PositionType::Absolute,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                flex_direction: FlexDirection::ColumnReverse,
                ..Default::default()
            },
            color: Color::rgba(0.1, 0.1, 0.1, 1.0).into(),
            ..Default::default()
        })
        .with_children(|parent| {
            parent
                // The bevy bird used as s spinner
                .spawn_bundle(ImageBundle {
                    style: Style {
                        position_type: PositionType::Relative,
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::Center,
                        size: Size::new(Val::Auto, Val::Percent(25.0)),
                        margin: UiRect {
                            bottom: Val::Percent(3.0),
                            ..Default::default()
                        },
                        ..Default::default()
                    },
                    image: asset_server.load("branding/icon.png").into(),
                    ..Default::default()
                });
            parent
                // Loading screen text bellow the bevy spinner
                .spawn_bundle(TextBundle {
                    style: Style {
                        position_type: PositionType::Relative,
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::Center,
                        ..Default::default()
                    },
                    text: Text::with_section(
                        LOADING_SCREEN_DEFAULT_TEXT,
                        TextStyle {
                            font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                            font_size: 50.0,
                            color: Color::WHITE,
                        },
                        TextAlignment {
                            horizontal: HorizontalAlign::Center,
                            vertical: VerticalAlign::Center
                        },
                    ),
                    ..Default::default()
                });
        })
        .insert(LoadingScreen);
}

// Animates aka rotates the bevy bird UiIamge clockwise.
fn animate_spinner(
    time: Res<Time>,
    mut loading_screen: Query<(&mut Style, &Children), With<LoadingScreen>>,
    mut text_query: Query<&mut Transform, With<UiImage>>
) {
    for (mut _style, children) in loading_screen.iter_mut() {
        for child in children.iter() {
            if let Ok(mut transform) = text_query.get_mut(*child) {
                transform.rotate(Quat::from_rotation_z(-time.delta_seconds() * 0.5));
            }
        }
    }
}

// Close our loading screen. This system is called when we exit GameState::Loading.
fn close_loading_screen(
    mut loading_screen: Query<(&mut Style, &Children), With<LoadingScreen>>,
    mut text_query: Query<&mut Text>
) {
    for (mut style, children) in loading_screen.iter_mut() {
        style.display = Display::None;
        // Reset the loading screen text to our default text
        for child in children.iter() {
            if let Ok(mut loading_text) = text_query.get_mut(*child) {
                loading_text.sections[0].value = LOADING_SCREEN_DEFAULT_TEXT.to_string();
            }
        }
    }
}

// Here we prepare our gaming scene, meaning we setup our ingame camera and load our asset.
fn setup_game(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut loading_screen: Query<(&mut Style, &Children), With<LoadingScreen>>,
    mut text_query: Query<&mut Text>
) {
    // Change our default loading screen text to something more descriptive
    for (mut _style, children) in loading_screen.iter_mut() {
        for child in children.iter() {
            if let Ok(mut loading_text) = text_query.get_mut(*child) {
                loading_text.sections[0].value = "Loading \"FlightHelmet\"".to_string();
            }
        }
    }

    // actually load our asset
    commands.spawn_scene(asset_server.load("models/FlightHelmet/FlightHelmet.gltf#Scene0"));
    // spawn our camera
    commands.spawn_bundle(PerspectiveCameraBundle {
        transform: Transform::from_xyz(0.7, 0.7, 1.0).looking_at(Vec3::new(0.0, 0.3, 0.0), Vec3::Y),
        ..Default::default()
    });
    // bring light into the darkness
    const HALF_SIZE: f32 = 1.0;
    commands.spawn_bundle(DirectionalLightBundle {
        directional_light: DirectionalLight {
            shadow_projection: OrthographicProjection {
                left: -HALF_SIZE,
                right: HALF_SIZE,
                bottom: -HALF_SIZE,
                top: HALF_SIZE,
                near: -10.0 * HALF_SIZE,
                far: 10.0 * HALF_SIZE,
                ..Default::default()
            },
            shadows_enabled: true,
            ..Default::default()
        },
        ..Default::default()
    });
}

// System that listens for our AssetEvent and changes our game state after we finished loading our asset
fn asset_listening_system(
    mut asset_events: EventReader<AssetEvent<Gltf>>,
    mut state: ResMut<State<GameState>>,
) {
    for event in asset_events.iter() {
        match event {
            AssetEvent::Created { handle: _ } => {
                println!("Done loading, switch state to `GameState::Playing`");
                // in this case, we know that we only had one asset (FlightHelmet.gltf#Scene0) to load,
                // so we can switch our game state from GameState::Loading over to GameState::Playing
                let _ = state.overwrite_set(GameState::Playing);
            },
            // we don't care about these events in our example
            AssetEvent::Modified { handle: _ } => {},
            AssetEvent::Removed { handle: _ } => {},
        }

    }
}

// Your, or at least one of your, game systems.
fn game_system(
    _commands: Commands,
    _asset_server: Res<AssetServer>,
) {
    // TODO: do your thing!
}
