use bevy::prelude::*;

fn main() {
    App::build()
        .add_default_plugins()
        .init_resource::<ButtonMaterials>()
        .add_startup_system(setup.system())
        .add_system(button_system.system())
        .run();
}

struct ButtonMaterials {
    normal: Handle<ColorMaterial>,
    hovered: Handle<ColorMaterial>,
    pressed: Handle<ColorMaterial>,
}

impl FromResources for ButtonMaterials {
    fn from_resources(resources: &Resources) -> Self {
        let mut materials = resources.get_mut::<Assets<ColorMaterial>>().unwrap();
        ButtonMaterials {
            normal: materials.add(Color::rgb(0.02, 0.02, 0.02).into()),
            hovered: materials.add(Color::rgb(0.05, 0.05, 0.05).into()),
            pressed: materials.add(Color::rgb(0.1, 0.5, 0.1).into()),
        }
    }
}

fn button_system(
    button_materials: Res<ButtonMaterials>,
    mut click_query: Query<(
        &Button,
        Changed<Click>,
        Option<&Hover>,
        &mut Handle<ColorMaterial>,
        &Children,
    )>,
    mut hover_query: Query<(
        &Button,
        Changed<Hover>,
        Option<&Click>,
        &mut Handle<ColorMaterial>,
        &Children,
    )>,
    label_query: Query<&mut Label>,
) {
    for (_button, hover, click, mut material, children) in &mut hover_query.iter() {
        let mut label = label_query.get_mut::<Label>(children[0]).unwrap();
        match *hover {
            Hover::Hovered => {
                if let Some(Click::Released) = click {
                    label.text = "Hover".to_string();
                    *material = button_materials.hovered;
                }
            }
            Hover::NotHovered => {
                if let Some(Click::Pressed) = click {
                    label.text = "Press".to_string();
                    *material = button_materials.pressed;
                } else {
                    label.text = "Button".to_string();
                    *material = button_materials.normal;
                }
            }
        }
    }

    for (_button, click, hover, mut material, children) in &mut click_query.iter() {
        let mut label = label_query.get_mut::<Label>(children[0]).unwrap();
        match *click {
            Click::Pressed => {
                label.text = "Press".to_string();
                *material = button_materials.pressed;
            }
            Click::Released => {
                if let Some(Hover::Hovered) = hover {
                    label.text = "Hover".to_string();
                    *material = button_materials.hovered;
                } else {
                    label.text = "Button".to_string();
                    *material = button_materials.normal;
                }
            }
        }
    }
}

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    button_materials: Res<ButtonMaterials>,
) {
    commands
        // ui camera
        .spawn(OrthographicCameraComponents::default())
        .spawn(ButtonComponents {
            node: Node::new(Anchors::CENTER, Margins::new(-75.0, 75.0, -35.0, 35.0)),
            material: button_materials.normal,
            ..Default::default()
        })
        .with_children(|parent| {
            parent.spawn(LabelComponents {
                node: Node::new(Anchors::FULL, Margins::new(0.0, 0.0, 12.0, 0.0)),
                label: Label {
                    text: "Button".to_string(),
                    font: asset_server.load("assets/fonts/FiraSans-Bold.ttf").unwrap(),
                    style: TextStyle {
                        font_size: 40.0,
                        color: Color::rgb(0.8, 0.8, 0.8),
                        align: TextAlign::Center,
                    },
                },
                ..Default::default()
            });
        });
}
