//! Demonstrates the use of color pickers.

use std::f32::consts::PI;

use bevy::prelude::*;
use bevy::ui::widget;
use bevy::ui::widget::{HueWheelMaterial, SaturationValueBoxEvent};
use bevy_internal::ui::widget::HueWheelSibling;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, (setup_ui, setup_3d))
        .add_systems(Update, saturation_value_box_system)
        .run();
}

#[derive(Debug, Component)]
struct SaturationValueBox1;

#[derive(Debug, Component)]
struct SaturationValueBox2;

#[derive(Debug, Component)]
struct Cube;

#[derive(Debug, Component)]
struct Base;

// Looks at events from saturation-value boxes.
// If found, checks which one the event stemmed from,
// then applies the color to either the cube or base.
fn saturation_value_box_system(
    mut events: EventReader<SaturationValueBoxEvent>,
    mut color_materials: ResMut<Assets<StandardMaterial>>,
    svb1: Query<Entity, With<SaturationValueBox1>>,
    svb2: Query<Entity, With<SaturationValueBox2>>,
    base: Query<&Handle<StandardMaterial>, With<Base>>,
    cube: Query<&Handle<StandardMaterial>, With<Cube>>,
) {
    for SaturationValueBoxEvent { entity, color } in events.read() {
        let handle = if svb1.single() == *entity {
            base.single()
        } else if svb2.single() == *entity {
            cube.single()
        } else {
            continue;
        };

        let Some(material) = color_materials.get_mut(handle) else {
            continue;
        };

        material.base_color = *color;
    }
}

fn setup_3d(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // circular base
    commands.spawn((
        PbrBundle {
            mesh: meshes.add(shape::Circle::new(4.0).into()),
            material: materials.add(Color::WHITE.into()),
            transform: Transform::from_rotation(Quat::from_rotation_x(
                -std::f32::consts::FRAC_PI_2,
            )),
            ..default()
        },
        Base,
    ));
    // cube
    commands.spawn((
        PbrBundle {
            mesh: meshes.add(Mesh::from(shape::Cube { size: 1.0 })),
            material: materials.add(Color::rgb_u8(124, 144, 255).into()),
            transform: Transform::from_xyz(0.0, 0.5, 0.0),
            ..default()
        },
        Cube,
    ));
    // light
    commands.spawn(PointLightBundle {
        point_light: PointLight {
            intensity: 1500.0,
            shadows_enabled: true,
            ..default()
        },
        transform: Transform::from_xyz(4.0, 8.0, 4.0),
        ..default()
    });
    // camera
    commands.spawn(Camera3dBundle {
        transform: Transform::from_xyz(-2.5, 4.5, 9.0).looking_at(Vec3::ZERO, Vec3::Y),
        ..default()
    });
}

fn setup_ui(
    mut commands: Commands,
    mut hue_materials: ResMut<Assets<widget::HueWheelMaterial>>,
    mut satval_materials: ResMut<Assets<widget::SaturationValueBoxMaterial>>,
) {
    let hue_wheel_material = HueWheelMaterial::default();

    // Given a desired px diameter of the wheel..
    let wheel_diameter = 200.0;

    // ..some math is required in order to have the inner box UI element
    // exactly touch the wheel
    let wheel_radius = wheel_diameter / 2.;
    let inner_wheel_radius = wheel_radius * hue_wheel_material.inner_radius;
    let box_size = (inner_wheel_radius * 2.) * (PI / 4.).cos();

    commands
        .spawn(NodeBundle {
            style: Style {
                display: Display::Flex,
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                ..default()
            },
            ..default()
        })
        .with_children(|flex: &mut ChildBuilder<'_, '_, '_>| {
            // picker 1
            flex.spawn(NodeBundle {
                style: Style {
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                    width: Val::Px(wheel_diameter),
                    height: Val::Px(wheel_diameter),
                    ..default()
                },
                ..default()
            })
            .with_children(|picker1| {
                let hue_wheel = picker1
                    .spawn(HueWheelBundle {
                        style: Style {
                            position_type: PositionType::Absolute,
                            width: Val::Percent(100.),
                            height: Val::Percent(100.),
                            ..default()
                        },
                        material: hue_materials.add(hue_wheel_material.clone()),
                        ..default()
                    })
                    .id();
                picker1.spawn((
                    SaturationValueBoxBundle {
                        style: Style {
                            position_type: PositionType::Absolute,
                            width: Val::Px(box_size),
                            height: Val::Px(box_size),
                            ..default()
                        },
                        material: satval_materials.add(default()),
                        ..default()
                    },
                    SaturationValueBox1,
                    // added to make the saturation-value box update automatically
                    HueWheelSibling(hue_wheel),
                ));
            });

            // picker 2
            flex.spawn(NodeBundle {
                style: Style {
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                    width: Val::Px(wheel_diameter),
                    height: Val::Px(wheel_diameter),
                    ..default()
                },
                ..default()
            })
            .with_children(|picker2| {
                let hue_wheel = picker2
                    .spawn(HueWheelBundle {
                        style: Style {
                            position_type: PositionType::Absolute,
                            width: Val::Percent(100.),
                            height: Val::Percent(100.),
                            ..default()
                        },
                        material: hue_materials.add(hue_wheel_material.clone()),
                        ..default()
                    })
                    .id();
                picker2.spawn((
                    SaturationValueBoxBundle {
                        style: Style {
                            position_type: PositionType::Absolute,
                            width: Val::Px(box_size),
                            height: Val::Px(box_size),
                            ..default()
                        },
                        material: satval_materials.add(default()),
                        ..default()
                    },
                    SaturationValueBox2,
                    // added to make the saturation-value box update automatically
                    HueWheelSibling(hue_wheel),
                ));
            });
        });
}
