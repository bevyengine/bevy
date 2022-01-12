use bevy::prelude::*;

/// This example illustrates, how a loading screen can be implemented. It has an animated spinner and listens for an `AssetEvent`.

fn main() {
    App::new()
        .insert_resource(Msaa { samples: 4 })
        .add_plugins(DefaultPlugins)
        // Loading screen plugin and its `asset_listening_system`
        .add_plugin(plugin::SimpleLoadingScreenPlugin)
        // system that sets up everything for our game
        .add_startup_system(setup_game)
        // system that will be run after everything is set up
        .add_system_set(SystemSet::on_enter(plugin::LoadingState::Done).with_system(game_system))
        .run();
}

// Here we prepare our gaming scene, meaning we setup our ingame camera and load our asset.
fn setup_game(mut commands: Commands, asset_server: Res<AssetServer>) {
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

// Your, or at least one of your, game systems.
fn game_system(_commands: Commands, _asset_server: Res<AssetServer>) {
    // TODO: do your thing!
}

//########################################## Example of a loading screen plugin ##########################################//
// You can use this as a template for your own project/game. Don't forget, that you also need a system,
// like `asset_listening_system`, to track your loading progress and to transition into your playing state.
pub mod plugin {
    use bevy::{ecs::schedule::SystemSet, gltf::Gltf, prelude::*};

    #[derive(Clone, Eq, PartialEq, Debug, Hash)]
    pub enum LoadingState {
        Loading,
        Done,
    }

    pub struct SimpleLoadingScreenPlugin;

    impl Plugin for SimpleLoadingScreenPlugin {
        fn build(&self, app: &mut App) {
            app.add_state(LoadingState::Loading)
                .add_startup_system(setup_loading_screen)
                .add_system_set(
                    SystemSet::on_update(LoadingState::Loading).with_system(animate_spinner),
                )
                .add_system_set(
                    SystemSet::on_update(LoadingState::Loading).with_system(asset_listening_system),
                )
                .add_system_set(
                    SystemSet::on_exit(LoadingState::Loading).with_system(close_loading_screen),
                );
        }
    }

    #[derive(Component)]
    pub struct LoadingScreen;
    #[derive(Component)]
    pub struct LoadingScreenContent;
    const LOADING_SCREEN_DEFAULT_TEXT: &str = "Loading...";

    // Setup our simple loading screen, it uses an image as spinner and a loading text bellow it.
    // INFO: you need to change this if you want something fancier 🙂
    fn setup_loading_screen(mut commands: Commands, asset_server: Res<AssetServer>) {
        // UI camera
        commands.spawn_bundle(UiCameraBundle::default());
        commands
            // NodeBundle used as background and root for our loading screen
            .spawn_bundle(NodeBundle {
                style: Style {
                    size: Size::new(Val::Percent(100.0), Val::Percent(100.0)),
                    position_type: PositionType::Absolute,
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                    flex_direction: FlexDirection::ColumnReverse,
                    ..Default::default()
                },
                color: Color::rgba(0.14, 0.14, 0.15, 1.0).into(),
                ..Default::default()
            })
            // add the stuff we want to see on the loading screen as children
            .with_children(|parent| {
                parent
                    // The bevy bird used as a spinner
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
                    })
                    .insert(LoadingScreenContent);
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
                                vertical: VerticalAlign::Center,
                            },
                        ),
                        ..Default::default()
                    })
                    .insert(LoadingScreenContent);
            })
            .insert(LoadingScreen);
    }

    // System that listens for our `AssetEvent` and changes our game state after we finished loading our asset
    // INFO: you need to change this if you have different assets 🙂
    fn asset_listening_system(
        // if you have different assets, either use another `EventReader` for your particular type of asset,
        // or use `AssetServer::get_load_state` with a resource in which you track the handles of your assets.
        mut asset_events: EventReader<AssetEvent<Gltf>>,
        mut state: ResMut<State<LoadingState>>,
    ) {
        for event in asset_events.iter() {
            match event {
                AssetEvent::Created { handle: _ } => {
                    info!("Done loading, switch state to `LoadingState::Done`");
                    // in this case, we know that we only had one asset (FlightHelmet.gltf#Scene0) to load,
                    // so we can switch our game state from `LoadingState::Loading` over to `LoadingState::Done`
                    let _ = state.overwrite_set(LoadingState::Done);
                }
                // we don't care about these events in our example
                AssetEvent::Modified { handle: _ } => {}
                AssetEvent::Removed { handle: _ } => {}
            }
        }
    }

    // Animates aka rotates the bevy bird `UiIamge` clockwise.
    fn animate_spinner(
        time: Res<Time>,
        // This queries for the children and style of the `NodeBundle` we marked with `LoadingScreen`
        loading_screen: Query<&Children, With<LoadingScreen>>,
        // Queries for the `Transform`s of all `UiImage`s, so that we can get our child from it.
        mut images_query: Query<&mut Transform, (With<UiImage>, With<LoadingScreenContent>)>,
    ) {
        // We only have one loading screen, so we can use `single()` instead of `iter()`
        let children = loading_screen.single();
        // We only have one `UiImage` in our loading screen, so `first()` does the trick
        if let Some(child) = children.first() {
            // Actually get the `Transform` of our child from all `UiImage`s
            if let Ok(mut transform) = images_query.get_mut(*child) {
                // Rotate the image clockwise
                transform.rotate(Quat::from_rotation_z(-time.delta_seconds() * 0.5));
            }
        }
    }

    // Close our loading screen. This system is called when we exit `LoadingState::Loading`.
    fn close_loading_screen(
        mut loading_screen: Query<(&mut Style, &Children), With<LoadingScreen>>,
        mut text_query: Query<&mut Text, With<LoadingScreenContent>>,
        mut images_query: Query<&mut Transform, (With<UiImage>, With<LoadingScreenContent>)>,
    ) {
        // We only have one loading screen, so we can use `single_mut()` instead of `iter_mut()`
        let (mut style, children) = loading_screen.single_mut();
        style.display = Display::None;
        // We only have one `Text` in our loading screen, so `first()` does the trick
        if let Some(child) = children.first() {
            // Reset the text to our default text
            if let Ok(mut loading_text) = text_query.get_mut(*child) {
                loading_text.sections[0].value = LOADING_SCREEN_DEFAULT_TEXT.to_string();
            }
            // Actually get the `Transform` of our child from all `UiImage`s
            if let Ok(mut transform) = images_query.get_mut(*child) {
                // Reset to the default rotation
                transform.rotation = Quat::default();
            }
        }
    }
}
