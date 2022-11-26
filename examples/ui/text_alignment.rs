//! This example demonstrates how text is displayed with each `TextAlignment`

use bevy::prelude::*;

const ALIGNMENTS: [TextAlignment; 9] = [
    TextAlignment::TOP_LEFT,
    TextAlignment::TOP_CENTER,
    TextAlignment::TOP_RIGHT,
    TextAlignment::CENTER_LEFT,
    TextAlignment::CENTER,
    TextAlignment::CENTER_RIGHT,
    TextAlignment::BOTTOM_LEFT,
    TextAlignment::BOTTOM_CENTER,
    TextAlignment::BOTTOM_RIGHT,
];

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_startup_system(setup)
        .run();
}
fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn(Camera2dBundle::default());
    for x in 0..3 {
        for y in 0..3 {
            let alignment = ALIGNMENTS[x + 3 * y];
            commands.spawn((
                TextBundle::from_section(
                    format!("{:?}\n{:?}", alignment.vertical, alignment.horizontal), 
                     TextStyle {
                        font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                        font_size: 24.0,
                        color: Color::WHITE,
                     }
                )
                .with_text_alignment(alignment)
                .with_style(Style {
                    position_type: PositionType::Absolute,
                    position: UiRect { left: Val::Px(x as f32 * 210. + 5.), top: Val::Px(y as f32 * 210. + 5.), ..Default::default() },
                    size: Size::new(Val::Px(200.), Val::Px(200.)),
                    ..Default::default()
                }),
                BackgroundColor(Color::NAVY)
            ));
        }
    }
}