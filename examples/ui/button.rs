use bevy::prelude::*;

fn main() {
    App::build()
        .add_default_plugins()
        .add_startup_system(setup.system())
        .add_system(button_system.system())
        .run();
}

fn button_system(
    mut click_query: Query<(&Button, Changed<Click>)>,
    mut hover_query: Query<(&Button, Changed<Hover>)>,
) {
    for (_button, click) in &mut click_query.iter() {
        match *click {
            Click::Pressed => {
                println!("pressed");
            }
            Click::Released => {
                println!("released");
            }
        }
    }

    for (_button, hover) in &mut hover_query.iter() {
        match *hover {
            Hover::Hovered => {
                println!("hovered");
            }
            Hover::NotHovered => {
                println!("unhovered");
            }
        }
    }
}

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    commands
        // ui camera
        .spawn(OrthographicCameraComponents::default())
        .spawn(ButtonComponents {
            node: Node::new(Anchors::BOTTOM_LEFT, Margins::new(10.0, 160.0, 10.0, 80.0)),
            material: materials.add(Color::rgb(0.2, 0.8, 0.2).into()),
            ..Default::default()
        })
        .with_children(|parent| {
            parent.spawn(LabelComponents {
                node: Node::new(Anchors::CENTER, Margins::new(52.0, 10.0, 20.0, 20.0)),
                label: Label {
                    text: "Button".to_string(),
                    font: asset_server.load("assets/fonts/FiraSans-Bold.ttf").unwrap(),
                    style: TextStyle {
                        font_size: 40.0,
                        color: Color::rgb(0.1, 0.1, 0.1),
                    },
                },
                ..Default::default()
            });
        });
}
