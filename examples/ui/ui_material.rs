//! Demonstrates the use of [`UiMaterials`](UiMaterial) and how to change material values

use bevy::prelude::*;
use bevy::reflect::TypePath;
use bevy::render::render_resource::*;

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
        // rainbow color effect
        let new_color = Color::hsl((time.elapsed_seconds() * 60.0) % 360.0, 1., 0.5);
        material.color = LinearRgba::from(new_color).to_f32_array().into();
    }
}

fn setup(mut commands: Commands, mut ui_materials: ResMut<Assets<CustomUiMaterial>>) {
    // Camera so we can see UI
    commands.spawn(Camera2dBundle::default());

    commands
        .spawn(NodeBundle {
            style: Style {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            ..default()
        })
        .with_children(|parent| {
            parent.spawn(MaterialNodeBundle {
                style: Style {
                    position_type: PositionType::Absolute,
                    width: Val::Px(250.0),
                    height: Val::Px(250.0),
                    ..default()
                },
                material: ui_materials.add(CustomUiMaterial {
                    color: LinearRgba::WHITE.to_f32_array().into(),
                }),
                ..default()
            });
        });
}

#[derive(AsBindGroup, Asset, TypePath, Debug, Clone)]
struct CustomUiMaterial {
    #[uniform(0)]
    color: Vec4,
}

impl UiMaterial for CustomUiMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/circle_shader.wgsl".into()
    }
}
