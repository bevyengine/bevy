//! Shows how to create a loading screen that waits for assets to load and render.
use bevy::{
    asset::await_loads::{
        AwaitDependencies, AwaitLoads, AwaitLoadsBuilder, AwaitReadyFrames, LoadFinished, ReadyFlag,
    },
    ecs::system::SystemId,
    prelude::*,
};

use pipelines_ready::*;

// We'll use the `AwaitLoadsBuilder` to keep track of all the assets we need for each scene, and
// spawn a corresponding `AwaitLoads` entity. Once the assets have been loaded, we'll spawn the
// scene.
//
// In order to ensure that visual assets are fully rendered before transitioning to the scene, we
// need to get the current status of cached pipelines.
//
// While loading and pipelines compilation is happening, we will show a loading screen.

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        // `PipelinesReadyPlugin` is declared in the `pipelines_ready` module below.
        .add_plugins(PipelinesReadyPlugin)
        .insert_resource(LoadingState::default())
        .add_systems(Startup, (setup, load_loading_screen))
        .add_systems(
            Update,
            (
                track_pipelines_ready,
                level_selection,
                display_loading_screen,
            ),
        )
        .run();
}

// A `Resource` that holds the current loading state.
#[derive(Resource, Default)]
enum LoadingState {
    #[default]
    LevelReady,
    LevelLoading,
}

// This resource will hold the level related systems ID for later use.
#[derive(Resource)]
struct LevelData {
    unload_level_id: SystemId,
    level_1_id: SystemId,
    level_2_id: SystemId,
}

fn setup(mut commands: Commands) {
    let level_data = LevelData {
        unload_level_id: commands.register_system(unload_current_level),
        level_1_id: commands.register_system(load_level_1),
        level_2_id: commands.register_system(load_level_2),
    };
    commands.insert_resource(level_data);

    // Spawns the UI that will show the user prompts.
    let text_style = TextFont {
        font_size: FontSize::Px(42.0),
        ..default()
    };
    commands
        .spawn((
            Node {
                justify_self: JustifySelf::Center,
                align_self: AlignSelf::FlexEnd,
                ..default()
            },
            BackgroundColor(Color::NONE),
        ))
        .with_child((Text::new("Press 1 or 2 to load a new scene."), text_style));
}

// Selects the level you want to load.
fn level_selection(
    mut commands: Commands,
    keyboard: Res<ButtonInput<KeyCode>>,
    level_data: Res<LevelData>,
    loading_state: Res<LoadingState>,
) {
    // Only trigger a load if the current level is fully loaded.
    if let LoadingState::LevelReady = loading_state.as_ref() {
        if keyboard.just_pressed(KeyCode::Digit1) {
            commands.run_system(level_data.unload_level_id);
            commands.run_system(level_data.level_1_id);
            commands.run_system_cached(spawn_pipeline_ready_loader);
        } else if keyboard.just_pressed(KeyCode::Digit2) {
            commands.run_system(level_data.unload_level_id);
            commands.run_system(level_data.level_2_id);
            commands.run_system_cached(spawn_pipeline_ready_loader);
        }
    }
}

// Marker component for easier deletion of entities.
#[derive(Component)]
struct LevelComponents;

// Removes all currently loaded level assets from the game World.
fn unload_current_level(mut commands: Commands, entities: Query<Entity, With<LevelComponents>>) {
    for entity in entities.iter() {
        commands.entity(entity).despawn();
    }
}

fn load_level_1(mut commands: Commands, asset_server: Res<AssetServer>) {
    let mut load_builder = AwaitLoadsBuilder::from(asset_server);
    let fox = load_builder.load(GltfAssetLabel::Scene(0).from_asset("models/animated/Fox.glb"));

    let mut loader = commands.spawn(load_builder.build());

    // Only spawn the level once the loader finishes loading. This avoids cases where the player
    // falls through the floor because the level hasn't spawned in yet!
    loader.observe(move |_: On<Insert<LoadFinished>>, mut commands: Commands| {
        // Spawn the camera.
        commands.spawn((
            Camera3d::default(),
            Transform::from_xyz(155.0, 155.0, 155.0).looking_at(Vec3::new(0.0, 40.0, 0.0), Vec3::Y),
            LevelComponents,
        ));

        // Spawn the fox.
        commands.spawn((
            WorldAssetRoot(fox.clone()),
            Transform::from_xyz(0.0, 0.0, 0.0),
            LevelComponents,
        ));

        // Spawn the light.
        commands.spawn((
            DirectionalLight {
                shadow_maps_enabled: true,
                ..default()
            },
            Transform::from_xyz(3.0, 3.0, 2.0).looking_at(Vec3::ZERO, Vec3::Y),
            LevelComponents,
        ));
    });
}

fn load_level_2(mut commands: Commands, asset_server: Res<AssetServer>) {
    let mut load_builder = AwaitLoadsBuilder::from(asset_server);
    let helmet_scene = load_builder
        .load(GltfAssetLabel::Scene(0).from_asset("models/FlightHelmet/FlightHelmet.gltf"));

    let mut loader = commands.spawn(load_builder.build());

    loader.observe(move |_: On<Insert<LoadFinished>>, mut commands: Commands| {
        // Spawn the camera.
        commands.spawn((
            Camera3d::default(),
            Transform::from_xyz(1.0, 1.0, 1.0).looking_at(Vec3::new(0.0, 0.2, 0.0), Vec3::Y),
            LevelComponents,
        ));

        // Spawn the helmet.
        commands.spawn((WorldAssetRoot(helmet_scene.clone()), LevelComponents));

        // Spawn the light.
        commands.spawn((
            DirectionalLight {
                shadow_maps_enabled: true,
                ..default()
            },
            Transform::from_xyz(3.0, 3.0, 2.0).looking_at(Vec3::ZERO, Vec3::Y),
            LevelComponents,
        ));
    });
}

// This system controls the loading screen. While each level spawns its scene when the assets load,
// their render pipelines might not be ready. So before we drop the loading screen, make sure the
// render pipelines are ready (and a few extra frames after).
fn spawn_pipeline_ready_loader(mut commands: Commands, loaders: Query<Entity, With<AwaitLoads>>) {
    // Set the state to loading. This is what makes the loading screen pop up.
    commands.insert_resource(LoadingState::LevelLoading);
    // Create the loader that will drop the loading screen. It waits for all loaders in the scene,
    // and an additional few frames, so long as the render pipelines are ready.
    commands
        .spawn((
            AwaitDependencies(loaders.iter().collect()),
            AwaitReadyFrames(5),
            ReadyFlag::default(),
            TrackPipelinesReady,
        ))
        // Add the loaders we depend on as children, so that when this loader despawns, so do these
        // loaders.
        .add_children(&loaders.iter().collect::<Vec<_>>())
        .observe(|event: On<Insert<LoadFinished>>, mut commands: Commands| {
            commands.entity(event.entity).despawn();

            commands.insert_resource(LoadingState::LevelReady);
        });
}

#[derive(Component)]
struct TrackPipelinesReady;

fn track_pipelines_ready(
    mut trackers: Populated<&mut ReadyFlag, With<TrackPipelinesReady>>,
    pipelines_ready: Res<PipelinesReady>,
) {
    for mut ready_flag in trackers.iter_mut() {
        ready_flag.0 = pipelines_ready.0;
    }
}

// Marker tag for loading screen components.
#[derive(Component)]
struct LoadingScreen;

// Spawns the necessary components for the loading screen.
fn load_loading_screen(mut commands: Commands) {
    let text_style = TextFont {
        font_size: FontSize::Px(67.0),
        ..default()
    };

    // Spawn the UI and Loading screen camera.
    commands.spawn((
        Camera2d,
        Camera {
            order: 1,
            clear_color: ClearColorConfig::None,
            ..default()
        },
        LoadingScreen,
    ));

    // Spawn the UI that will make up the loading screen.
    commands
        .spawn((
            Node {
                height: percent(100),
                width: percent(100),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(Color::BLACK),
            LoadingScreen,
        ))
        .with_child((Text::new("Loading..."), text_style.clone()));
}

// Determines when to show the loading screen
fn display_loading_screen(
    mut loading_screen: Single<&mut Visibility, (With<LoadingScreen>, With<Node>)>,
    loading_state: Res<LoadingState>,
) {
    let visibility = match loading_state.as_ref() {
        LoadingState::LevelLoading => Visibility::Visible,
        LoadingState::LevelReady => Visibility::Hidden,
    };

    **loading_screen = visibility;
}

mod pipelines_ready {
    use bevy::{
        prelude::*,
        render::{render_resource::*, *},
    };

    pub struct PipelinesReadyPlugin;
    impl Plugin for PipelinesReadyPlugin {
        fn build(&self, app: &mut App) {
            app.insert_resource(PipelinesReady::default());

            // In order to gain access to the pipelines status, we have to
            // go into the `RenderApp`, grab the resource from the main App
            // and then update the pipelines status from there.
            // Writing between these Apps can only be done through the
            // `ExtractSchedule`.
            app.sub_app_mut(RenderApp)
                .add_systems(ExtractSchedule, update_pipelines_ready);
        }
    }

    #[derive(Resource, Debug, Default)]
    pub struct PipelinesReady(pub bool);

    fn update_pipelines_ready(mut main_world: ResMut<MainWorld>, pipelines: Res<PipelineCache>) {
        if let Some(mut pipelines_ready) = main_world.get_resource_mut::<PipelinesReady>() {
            pipelines_ready.0 = pipelines.waiting_pipelines().count() == 0;
        }
    }
}
