//! This example illustrates how to create a Node that shows a Material and drives parameters from
//! it

use bevy::prelude::*;
use bevy::ui::UiMaterialPlugin;
use bevy_internal::{reflect::TypePath, render::render_resource::*};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(UiMaterialPlugin::<CustomUiMaterial>::default())
        .add_systems(Startup, setup)
        .add_systems(Update, update)
        .run();
}

fn update(time: Res<Time>, mut ui_materials: ResMut<Assets<CustomUiMaterial>>) {
    for (_, material) in ui_materials.iter_mut() {
        material.fill_amount = time.elapsed_seconds() % 1.0;
        let new_color = Color::hsla(time.elapsed_seconds() * 100.0 % 360.0, 1.0, 0.5, 1.0);
        material.color = new_color.into();
    }
}

fn setup(mut commands: Commands, mut ui_materials: ResMut<Assets<CustomUiMaterial>>) {
    // Camera so we can see UI
    commands.spawn(Camera2dBundle::default());

    commands
        .spawn(NodeBundle {
            style: Style {
                width: Val::Percent(100.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            ..default()
        })
        .with_children(|parent| {
            parent.spawn(MaterialNodeBundle {
                style: Style {
                    width: Val::Px(250.0),
                    height: Val::Px(250.0),
                    ..default()
                },
                material: ui_materials.add(CustomUiMaterial {
                    fill_amount: 0.0,
                    color: Color::WHITE.into(),
                }),
                ..default()
            });
        });
}

#[derive(AsBindGroup, TypePath, Asset, Debug, Clone)]
struct CustomUiMaterial {
    #[uniform(0)]
    fill_amount: f32,
    #[uniform(0)]
    color: Vec4,
}

impl UiMaterial for CustomUiMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/circle_shader.wgsl".into()
    }
}
