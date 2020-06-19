use bevy::{
    diagnostic::{Diagnostics, FrameTimeDiagnosticsPlugin},
    prelude::*,
};

fn main() {
    App::build()
        .add_default_plugins()
        .add_plugin(FrameTimeDiagnosticsPlugin::default())
        .add_startup_system(setup.system())
        .add_system(text_update_system.system())
        .run();
}

fn text_update_system(diagnostics: Res<Diagnostics>, mut label: ComMut<Label>) {
    if let Some(fps) = diagnostics.get(FrameTimeDiagnosticsPlugin::FPS) {
        if let Some(average) = fps.average() {
            label.text = format!("FPS: {}", average);
        }
    }
}

fn setup(command_buffer: &mut CommandBuffer, asset_server: Res<AssetServer>) {
    let font_handle = asset_server.load("assets/fonts/FiraSans-Bold.ttf").unwrap();
    command_buffer
        .build()
        // 2d camera
        .add_entity(OrthographicCameraEntity::default())
        .add_entity(OrthographicCameraEntity::ui())
        // texture
        .add_entity(LabelEntity {
            node: Node::new(
                math::vec2(0.0, 0.0),
                Anchors::TOP_LEFT,
                Margins::new(0.0, 250.0, 0.0, 60.0),
            ),
            label: Label {
                text: "FPS:".to_string(),
                font: font_handle,
                style: TextStyle {
                    font_size: 60.0,
                    color: Color::WHITE,
                },
            },
            ..Default::default()
        });
}
