//! Shows how to create a loading screen that waits for assets to load and render.
use bevy::{ecs::system::SystemId, prelude::*};
use pipelines_info::*;

// The way we'll go about doing this in this example is to
// keep track of all assets that we want to have loaded before
// we transition to the desired scene.
//
// In order to ensure that visual assets are fully rendered
// before transitioning to the scene, we need to get the
// current status of cached pipelines.
//
// While loading and pipelines compilation is happening, we
// will show a loading screen. Once loading is complete, we
// will transition to the scene we just loaded.

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        // `PipelinesInfoPlugin` is declared in the `pipelines_info` module below.
        .add_plugins(PipelinesInfoPlugin)
        .insert_resource(LoadingState::default())
        .insert_resource(LoadingData::new(5))
        .add_systems(Startup, (setup, load_loading_screen))
        .add_systems(
            Update,
            (update_loading_data, level_selection, display_loading_screen),
        )
        .run()
}

// A `Resource` that holds the current loading state.
#[derive(Resource, Default)]
enum LoadingState {
    #[default]
    LevelReady,
    LevelLoading,
}

// A resource that holds the current loading data.
#[derive(Resource, Debug, Default)]
struct LoadingData {
    // This will hold the currently unloaded/loading assets.
    loading_assets: Vec<UntypedHandle>,
    // The number of frames that the loading_assets vector needs to be empty
    // for to confirm that everything is loaded.
    confirmation_frames_target: usize,
    // Current number of confirmation frames.
    confirmation_frames_count: usize,
}

impl LoadingData {
    fn new(confirmation_frames_target: usize) -> Self {
        Self {
            loading_assets: Vec::new(),
            confirmation_frames_target,
            confirmation_frames_count: 0,
        }
    }
}

// This resource will hold the systems ID for later use.
#[derive(Resource)]
struct LevelData {
    unload_level_id: SystemId,
    level_1_id: SystemId,
    level_2_id: SystemId,
}

fn setup(mut commands: Commands) {
    let level_data = LevelData {
        unload_level_id: commands.register_one_shot_system(unload_current_level),
        level_1_id: commands.register_one_shot_system(load_level_1),
        level_2_id: commands.register_one_shot_system(load_level_2),
    };
    commands.insert_resource(level_data);
}

// Selects the level you want to load.
fn level_selection(
    mut commands: Commands,
    keyboard: Res<ButtonInput<KeyCode>>,
    level_data: Res<LevelData>,
    loading_state: Res<LoadingState>,
) {
    match loading_state.as_ref() {
        LoadingState::LevelReady => {
            if keyboard.just_pressed(KeyCode::Digit1) {
                info!("Loading LEVEL 1");
                commands.run_system(level_data.unload_level_id);
                commands.run_system(level_data.level_1_id);
            } else if keyboard.just_pressed(KeyCode::Digit2) {
                info!("Loading LEVEL 2");
                commands.run_system(level_data.unload_level_id);
                commands.run_system(level_data.level_2_id);
            }
        }
        _ => (),
    }
}

// Removes all currently loaded level assets from the game World.
fn unload_current_level(
    mut commands: Commands,
    mut loading_state: ResMut<LoadingState>,
    entities: Query<Entity, With<LevelComponents>>,
) {
    *loading_state = LoadingState::LevelLoading;
    for entity in entities.iter() {
        commands.entity(entity).despawn_recursive();
    }
}

// Marker component for easier deletion of entities.
#[derive(Component)]
struct LevelComponents;

fn load_level_1(
    mut commands: Commands,
    mut loading_data: ResMut<LoadingData>,
    mut pipelines_info: ResMut<PipelinesInfo>,
    asset_server: Res<AssetServer>,
) {
    // Declare your pipeline count here.
    // TODO: Find a way to get pipeline count target dynamically
    pipelines_info.pipeline_count = 11;

    // Spawn the camera.
    commands.spawn((
        Camera3dBundle {
            transform: Transform::from_xyz(5.0, 5.0, -5.0)
                .looking_at(Vec3::new(0.0, 0.0, 0.0), Vec3::Y),
            ..default()
        },
        LevelComponents,
    ));

    // Save the asset into the `loading_assets` vector.
    let house_scene = asset_server.load("models/HouseInTheWoods/HouseInTheWoods.glb#Scene0");
    loading_data.loading_assets.push(house_scene.clone().into());
    // Spawn the house.
    commands.spawn((
        SceneBundle {
            scene: house_scene.clone(),
            transform: Transform::from_xyz(0.0, 0.0, 0.0),
            ..default()
        },
        LevelComponents,
    ));

    // Spawn the light.
    commands.spawn((
        DirectionalLightBundle {
            transform: Transform::from_xyz(3.0, 3.0, 2.0).looking_at(Vec3::ZERO, Vec3::Y),
            directional_light: DirectionalLight {
                shadows_enabled: true,
                ..default()
            },
            ..default()
        },
        LevelComponents,
    ));
}

fn load_level_2(
    mut commands: Commands,
    mut loading_data: ResMut<LoadingData>,
    mut pipelines_info: ResMut<PipelinesInfo>,
    asset_server: Res<AssetServer>,
) {
    // Declare your pipeline count here.
    // TODO: Find a way to get pipeline count target dynamically.
    pipelines_info.pipeline_count = 15;

    // Spawn the camera.
    commands.spawn((
        Camera3dBundle {
            transform: Transform::from_xyz(1.0, 1.0, 1.0)
                .looking_at(Vec3::new(0.0, 0.2, 0.0), Vec3::Y),
            ..default()
        },
        LevelComponents,
    ));

    // Spawn the helmet.
    let helmet_scene = asset_server.load("models/FlightHelmet/FlightHelmet.gltf#Scene0");
    loading_data
        .loading_assets
        .push(helmet_scene.clone().into());
    commands.spawn((
        SceneBundle {
            scene: helmet_scene.clone(),
            ..default()
        },
        LevelComponents,
    ));

    // Spawn the light.
    commands.spawn((
        DirectionalLightBundle {
            transform: Transform::from_xyz(3.0, 3.0, 2.0).looking_at(Vec3::ZERO, Vec3::Y),
            directional_light: DirectionalLight {
                shadows_enabled: true,
                ..default()
            },
            ..default()
        },
        LevelComponents,
    ));
}

// Monitors current loading status of assets.
fn update_loading_data(
    mut loading_data: ResMut<LoadingData>,
    mut loading_state: ResMut<LoadingState>,
    asset_server: Res<AssetServer>,
    pipelines_info: Res<PipelinesInfo>,
) {
    if !loading_data.loading_assets.is_empty() {
        // If our loading_assets vector isn't empty, we reset the confirmation frames count.
        loading_data.confirmation_frames_count = 0;

        // Go through each asset and verify their load states.
        // Any assets that are loaded are then added to the pop list for later removal.
        let mut pop_list: Vec<usize> = Vec::new();
        for (index, asset) in loading_data.loading_assets.iter().enumerate() {
            if let Some(state) = asset_server.get_load_states(asset) {
                match state.2 {
                    bevy::asset::RecursiveDependencyLoadState::Loaded => pop_list.push(index),
                    _ => (),
                }
            }
        }

        // Remove all loaded assets from the loading_assets list.
        for i in pop_list.iter() {
            loading_data.loading_assets.remove(*i);
        }

        // If there are no more assets being monitored, and pipelines
        // are loaded, then start counting confirmation frames.
        // Once enough confirmations have passed, everything should be ready.
    } else if pipelines_info.ready_count >= pipelines_info.pipeline_count {
        loading_data.confirmation_frames_count += 1;
        if loading_data.confirmation_frames_count == loading_data.confirmation_frames_target {
            info!("LOADED");
            *loading_state = LoadingState::LevelReady;
        }
    }
}

// Marker tag for loading screen components.
#[derive(Component)]
struct LoadingScreen;

// Spawns the necessary components for the loading screen.
fn load_loading_screen(mut commands: Commands) {
    let text_style = TextStyle {
        font_size: 50.0,
        ..default()
    };

    // Set the camera render order.
    let camera = Camera {
        order: 1,
        is_active: true,
        ..default()
    };

    // Spawn the UI and Loading screen camera.
    commands.spawn((
        Camera2dBundle {
            camera,
            ..default()
        },
        LoadingScreen,
    ));

    // Spawn the UI that will make up the loading screen.
    commands
        .spawn((
            NodeBundle {
                background_color: BackgroundColor(Color::BLACK),
                style: Style {
                    height: Val::Percent(100.0),
                    width: Val::Percent(100.0),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..default()
                },
                ..default()
            },
            LoadingScreen,
        ))
        .with_children(|parent| {
            parent.spawn(TextBundle::from_sections([TextSection::new(
                "Loading...",
                text_style.clone(),
            )]));
        });

    // Spawns the UI that will show the user prompts.
    commands
        .spawn(NodeBundle {
            background_color: BackgroundColor(Color::NONE),
            style: Style {
                justify_self: JustifySelf::Center,
                align_self: AlignSelf::FlexEnd,
                ..default()
            },
            ..default()
        })
        .with_children(|parent| {
            parent.spawn(TextBundle::from_section(
                "Press 1 or 2 to load a new scene.",
                text_style,
            ));
        });
}

fn display_loading_screen(
    mut loading_screen: Query<&mut Visibility, With<LoadingScreen>>,
    loading_state: Res<LoadingState>,
) {
    match loading_state.as_ref() {
        LoadingState::LevelLoading => {
            *loading_screen.get_single_mut().unwrap() = Visibility::Visible
        }
        LoadingState::LevelReady => *loading_screen.get_single_mut().unwrap() = Visibility::Hidden,
    };
}

mod pipelines_info {
    use bevy::{prelude::*, render::render_resource::*, render::*};

    pub struct PipelinesInfoPlugin;
    impl Plugin for PipelinesInfoPlugin {
        fn build(&self, app: &mut App) {
            // TODO: Need to find a way to find the target pipeline count dynamically
            app.insert_resource(PipelinesInfo::default());

            // In order to gain access to the pipelines status, we have to
            // go into the `RenderApp`, grab the resource from the main App
            // and then update the pipelines status from there.
            // Writing between these Apps can only be done through the
            // `ExtractSchedule`.
            app.sub_app_mut(bevy::render::RenderApp)
                .add_systems(ExtractSchedule, update_pipelines_info);
        }
    }

    #[derive(Resource, Debug, Default)]
    pub struct PipelinesInfo {
        pub pipeline_count: usize,
        pub ready_count: usize,
    }

    fn update_pipelines_info(mut main_world: ResMut<MainWorld>, pipelines: Res<PipelineCache>) {
        if let Some(mut pipelines_info) = main_world.get_resource_mut::<PipelinesInfo>() {
            let mut pipelines_ready = 0;
            for pipeline in pipelines.pipelines() {
                match pipeline.state {
                    CachedPipelineState::Ok(_) => {
                        pipelines_ready += 1;
                    }
                    _ => (),
                }
            }
            pipelines_info.ready_count = pipelines_ready;
        }
    }
}
