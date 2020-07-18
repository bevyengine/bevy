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

fn text_update_system(diagnostics: Res<Diagnostics>, mut query: Query<&mut Label>) {
    for mut label in &mut query.iter() {
        if let Some(fps) = diagnostics.get(FrameTimeDiagnosticsPlugin::FPS) {
            if let Some(average) = fps.average() {
                label.text = format!("FPS: {:.2}", average);
            }
        }
    }
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    let font_handle = asset_server.load("assets/fonts/FiraSans-Bold.ttf").unwrap();
    commands
        // 2d camera
        .spawn(OrthographicCameraComponents::default())
        // texture
        .spawn(LabelComponents {
            node: Node::new(Anchors::TOP_LEFT, Margins::new(0.0, 250.0, 0.0, 60.0)),
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
