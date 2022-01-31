use bevy::prelude::*;
use bevy_internal::asset::AssetPlugin;
use bevy_internal::core_pipeline::CorePipelinePlugin;
use bevy_internal::input::InputPlugin;
use bevy_internal::render::options::WgpuOptions;
use bevy_internal::render::RenderPlugin;
use bevy_internal::sprite::SpritePlugin;
use bevy_internal::text::TextPlugin;
use bevy_internal::ui::UiPlugin;
use bevy_internal::window::{WindowId, WindowPlugin};

const WINDOW_WIDTH: u32 = 200;
const WINDOW_HEIGHT: u32 = 100;

struct HeadlessUiPlugin;

impl Plugin for HeadlessUiPlugin {
    fn build(&self, app: &mut App) {
        // These tests are meant to be ran on systems without gpu, or display.
        // To make this work, we tell bevy not to look for any rendering backends.
        app.insert_resource(WgpuOptions {
            backends: None,
            ..Default::default()
        })
        // To test the positioning of UI elements,
        // we first need a window to position these elements in.
        .insert_resource({
            let mut windows = Windows::default();
            windows.add(Window::new(
                // At the moment, all ui elements are placed in the primary window.
                WindowId::primary(),
                &WindowDescriptor::default(),
                WINDOW_WIDTH,
                WINDOW_HEIGHT,
                1.0,
                None,
                // Because this test is running without a real window, we pass `None` here.
                None,
            ));
            windows
        })
        .add_plugins(MinimalPlugins)
        .add_plugin(TransformPlugin)
        .add_plugin(WindowPlugin::default())
        .add_plugin(InputPlugin)
        .add_plugin(AssetPlugin)
        .add_plugin(RenderPlugin)
        .add_plugin(CorePipelinePlugin)
        .add_plugin(SpritePlugin)
        .add_plugin(TextPlugin)
        .add_plugin(UiPlugin);
    }
}

#[test]
fn test_button_translation() {
    let mut app = App::new();
    app.add_plugin(HeadlessUiPlugin)
        .add_startup_system(setup_button_test);

    // First call to `update` also runs the startup systems.
    app.update();

    let mut query = app.world.query_filtered::<Entity, With<Button>>();
    let button = *query.iter(&app.world).collect::<Vec<_>>().first().unwrap();

    // The button's translation got updated because the UI system had a window to place it in.
    // If we hadn't added a window, the button's translation would at this point be all zero's.
    let button_transform = app.world.entity(button).get::<Transform>().unwrap();
    assert_eq!(
        button_transform.translation.x.floor() as u32,
        WINDOW_WIDTH / 2
    );
    assert_eq!(
        button_transform.translation.y.floor() as u32,
        WINDOW_HEIGHT / 2
    );
}

fn setup_button_test(mut commands: Commands) {
    commands.spawn_bundle(UiCameraBundle::default());
    commands.spawn_bundle(ButtonBundle {
        style: Style {
            size: Size::new(Val::Px(150.0), Val::Px(65.0)),
            // Center this button in the middle of the window.
            margin: Rect::all(Val::Auto),
            ..Default::default()
        },
        ..Default::default()
    });
}
