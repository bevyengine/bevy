use bevy_app::{App, AppBuilder, Events, ManualEventReader};
use bevy_asset::AssetPlugin;
use bevy_core::CorePlugin;
use bevy_ecs::{component::Component, prelude::*};
use bevy_input::InputPlugin;
use bevy_openxr::{get_render_plugin, OpenXRPlugin, XRCameraBundle};
use bevy_openxr_core::{
    event::{XRState, XRViewSurfaceCreated, XRViewsCreated},
    OpenXRCorePlugin,
};
use bevy_render::{
    prelude::Msaa,
    render_graph::{base::node, RenderGraph},
    renderer::RenderResourceId,
};
use bevy_scene::ScenePlugin;
use bevy_sprite::SpritePlugin;
use bevy_text::TextPlugin;
use bevy_transform::TransformPlugin;
use bevy_ui::UiPlugin;
use bevy_wgpu::WgpuPlugin;
use bevy_window::WindowPlugin;

#[test]
fn test() {
    let mut builder = App::build();
    builder.insert_resource(Msaa { samples: 2 });
    builder.add_plugin(OpenXRPlugin);
    builder.add_plugin(CorePlugin);
    builder.add_plugin(TransformPlugin::default());
    builder.add_plugin(InputPlugin::default());
    builder.add_plugin(WindowPlugin::default());
    builder.add_plugin(AssetPlugin::default());
    builder.add_plugin(ScenePlugin::default());
    builder.add_plugin(get_render_plugin());
    builder.add_plugin(SpritePlugin::default());
    builder.add_plugin(UiPlugin::default());
    builder.add_plugin(TextPlugin::default());
    builder.add_plugin(WgpuPlugin::default());
    builder.add_plugin(OpenXRCorePlugin);

    builder.add_startup_system(setup.system());

    println!("========================= FRAME 1");
    builder.app.update();
    assert_eq!(read_events::<XRState>(&mut builder), &[&XRState::Running]);
    println!("========================= FRAME 2");
    builder.app.update();
    let surface_events = read_events::<XRViewSurfaceCreated>(&mut builder);
    assert_eq!(surface_events.len(), 1);
    assert!(surface_events[0].width > 0);
    assert!(surface_events[0].height > 0);

    assert_eq!(
        read_events::<XRState>(&mut builder),
        &[&XRState::Running, &XRState::RunningFocused]
    );

    let views_events = read_events::<XRViewsCreated>(&mut builder);
    assert_eq!(views_events.len(), 1);
    assert_eq!(views_events[0].views.len(), 2);

    let graph = builder.world().get_resource::<RenderGraph>().unwrap();
    let xr_window_texture_node = graph.get_node_state(node::MAIN_DEPTH_TEXTURE).unwrap();
    assert_eq!(xr_window_texture_node.output_slots.len(), 1);
    if let RenderResourceId::Texture(texture) = xr_window_texture_node.output_slots.get(0).unwrap()
    {
        // FIXME assert that texture is from swapchain?
    };
    println!("========================= FRAME 3");
}

fn read_events<T: Component>(builder: &mut AppBuilder) -> Vec<&T> {
    let events = builder.world().get_resource::<Events<T>>().unwrap();
    let mut reader = ManualEventReader::<T>::default();
    let events = reader.iter(events).collect::<Vec<_>>();
    events
}

fn setup(mut commands: Commands) {
    commands.spawn_bundle(XRCameraBundle::default());
}

/*
#[test]
#[should_panic(expected = "Must call set_xr_instance")]
fn test_should_panic_if_no_instance_set() {
    let mut builder = App::build();
    builder.add_plugin(OpenXRCorePlugin);
}

 */
