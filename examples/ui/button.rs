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
        Mutated<Click>,
        Option<&Hover>,
        &mut Handle<ColorMaterial>,
        &Children,
    )>,
    mut hover_query: Query<(
        &Button,
        Mutated<Hover>,
        Option<&Click>,
        &mut Handle<ColorMaterial>,
        &Children,
    )>,
    text_query: Query<&mut Text>,
) {
    for (_button, hover, click, mut material, children) in &mut hover_query.iter() {
        let mut text = text_query.get_mut::<Text>(children[0]).unwrap();
        match *hover {
            Hover::Hovered => {
                if let Some(Click::Released) = click {
                    text.value = "Hover".to_string();
                    *material = button_materials.hovered;
                }
            }
            Hover::NotHovered => {
                if let Some(Click::Pressed) = click {
                    text.value = "Press".to_string();
                    *material = button_materials.pressed;
                } else {
                    text.value = "Button".to_string();
                    *material = button_materials.normal;
                }
            }
        }
    }

    for (_button, click, hover, mut material, children) in &mut click_query.iter() {
        let mut text = text_query.get_mut::<Text>(children[0]).unwrap();
        match *click {
            Click::Pressed => {
                text.value = "Press".to_string();
                *material = button_materials.pressed;
            }
            Click::Released => {
                if let Some(Hover::Hovered) = hover {
                    text.value = "Hover".to_string();
                    *material = button_materials.hovered;
                } else {
                    text.value = "Button".to_string();
                    *material = button_materials.normal;
                }
            }
        }
    }
}

fn setup(
    mut commands: Commands,
    mut materials: ResMut<Assets<ColorMaterial>>,
    asset_server: Res<AssetServer>,
    button_materials: Res<ButtonMaterials>,
) {
    commands
        // ui camera
        .spawn(UiCameraComponents::default())
        // wrapper component to center with flexbox
        .spawn(NodeComponents {
            style: Style {
                size: Size {
                    width: Val::Percent(1.0),
                    height: Val::Percent(1.0),
                },
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..Default::default()
            },
            material: materials.add(Color::NONE.into()),
            ..Default::default()
        })
        .with_children(|parent| {
            parent
                .spawn(ButtonComponents {
                    style: Style {
                        size: Size {
                            width: Val::Px(150.0),
                            height: Val::Px(70.0),
                        },
                        ..Default::default()
                    },
                    material: button_materials.normal,
                    ..Default::default()
                })
                .with_children(|parent| {
                    parent.spawn(TextComponents {
                        style: Style {
                            size: Size {
                                width: Val::Percent(1.0),
                                height: Val::Percent(1.0),
                            },
                            margin: Rect {
                                top: Val::Px(10.0),
                                ..Default::default()
                            },
                            ..Default::default()
                        },
                        text: Text {
                            value: "Button".to_string(),
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
        });
}
