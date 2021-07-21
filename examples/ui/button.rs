use bevy::{
    prelude::*,
    ui::widget::{ButtonMaterials, CustomButtonMaterialBehavior, DefaultButtonMaterials},
};

/// This example illustrates how to create a button that changes color and text based on its
/// interaction state.
fn main() {
    App::build()
        .add_plugins(DefaultPlugins)
        .add_startup_system(setup.system())
        .add_system(button_system.system())
        .run();
}

fn button_system(
    mut interaction_query: Query<(&Interaction, &Children), (Changed<Interaction>, With<Button>)>,
    mut text_query: Query<&mut Text>,
) {
    for (interaction, children) in interaction_query.iter_mut() {
        let mut text = text_query.get_mut(children[0]).unwrap();
        match *interaction {
            Interaction::Clicked => {
                text.sections[0].value = "Press".to_string();
            }
            Interaction::Hovered => {
                text.sections[0].value = "Hover".to_string();
            }
            Interaction::None => {
                text.sections[0].value = "Button".to_string();
            }
        }
    }
}

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    button_materials: Res<DefaultButtonMaterials>,
) {
    // ui camera
    commands.spawn_bundle(UiCameraBundle::default());
    commands
        // "container" node
        .spawn_bundle(NodeBundle {
            style: Style {
                size: Size::new(Val::Percent(100.0), Val::Percent(100.0)),
                justify_content: JustifyContent::SpaceBetween,
                ..Default::default()
            },
            material: materials.add(Color::NONE.into()),
            ..Default::default()
        })
        .with_children(|parent| {
            // button with default materials
            parent
                .spawn_bundle(ButtonBundle {
                    style: Style {
                        size: Size::new(Val::Px(150.0), Val::Px(65.0)),
                        // center button
                        margin: Rect::all(Val::Auto),
                        // horizontally center child text
                        justify_content: JustifyContent::Center,
                        // vertically center child text
                        align_items: AlignItems::Center,
                        ..Default::default()
                    },
                    material: button_materials.normal.clone(),
                    ..Default::default()
                })
                .with_children(|parent| {
                    parent.spawn_bundle(TextBundle {
                        text: Text::with_section(
                            "Button",
                            TextStyle {
                                font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                                font_size: 40.0,
                                color: Color::rgb(0.9, 0.9, 0.9),
                            },
                            Default::default(),
                        ),
                        ..Default::default()
                    });
                });

            // button with custom materials
            parent
                .spawn_bundle(ButtonBundle {
                    style: Style {
                        size: Size::new(Val::Px(150.0), Val::Px(65.0)),
                        // center button
                        margin: Rect::all(Val::Auto),
                        // horizontally center child text
                        justify_content: JustifyContent::Center,
                        // vertically center child text
                        align_items: AlignItems::Center,
                        ..Default::default()
                    },
                    material: button_materials.normal.clone(),
                    ..Default::default()
                })
                .insert(ButtonMaterials {
                    normal: materials.add(Color::rgb(0.75, 0.35, 0.35).into()),
                    hovered: materials.add(Color::rgb(0.35, 0.35, 0.75).into()),
                    pressed: materials.add(Color::rgb(0.35, 0.75, 0.35).into()),
                })
                .with_children(|parent| {
                    parent.spawn_bundle(TextBundle {
                        text: Text::with_section(
                            "Button",
                            TextStyle {
                                font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                                font_size: 40.0,
                                color: Color::rgb(0.9, 0.9, 0.9),
                            },
                            Default::default(),
                        ),
                        ..Default::default()
                    });
                });

            // button with custom material behavior
            parent
                .spawn_bundle(ButtonBundle {
                    style: Style {
                        size: Size::new(Val::Px(150.0), Val::Px(65.0)),
                        // center button
                        margin: Rect::all(Val::Auto),
                        // horizontally center child text
                        justify_content: JustifyContent::Center,
                        // vertically center child text
                        align_items: AlignItems::Center,
                        ..Default::default()
                    },
                    material: button_materials.normal.clone(),
                    ..Default::default()
                })
                .insert(CustomButtonMaterialBehavior {})
                .with_children(|parent| {
                    parent.spawn_bundle(TextBundle {
                        text: Text::with_section(
                            "Button",
                            TextStyle {
                                font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                                font_size: 40.0,
                                color: Color::rgb(0.9, 0.9, 0.9),
                            },
                            Default::default(),
                        ),
                        ..Default::default()
                    });
                });
        });
}
