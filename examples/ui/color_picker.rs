//! Demonstrates the use of color pickers.

use std::f32::consts::PI;

use bevy::prelude::*;
use bevy::ui::widget::{
    HueWheelMaterial, HueWheelSibling, SaturationValueBoxEvent, SaturationValueBoxMaterial,
};
use bevy_internal::ui::widget::{hsv_to_rgb, HueWheelEvent};

fn main() {
    App::new()
        .init_resource::<BaseAndCube>()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, (setup_ui, setup_3d))
        .add_systems(
            Update,
            // First check for hue wheel or sat-val events to update colors,
            // then apply updates if any updates were had
            (
                (saturation_value_box_system, hue_wheel_system),
                update_colors.run_if(resource_changed::<BaseAndCube>()),
            )
                .chain(),
        )
        .run();
}

#[derive(Debug, Default)]
struct Hsv {
    hue: f32,
    saturation: f32,
    value: f32,
}

#[derive(Debug, Resource, Default)]
struct BaseAndCube {
    base: Hsv,
    cube: Hsv,
}

#[derive(Debug, Component)]
struct BaseColorBox;

#[derive(Debug, Component)]
struct CubeColorBox;

#[derive(Debug, Component)]
struct BaseColorWheel;

#[derive(Debug, Component)]
struct CubeColorWheel;

#[derive(Debug, Component)]
struct Cube;

#[derive(Debug, Component)]
struct Base;

// Looks at events from saturation-value boxes.
// If found, checks which one the event stemmed from,
// then applies the color to either the cube or base.
fn saturation_value_box_system(
    mut events: EventReader<SaturationValueBoxEvent>,
    mut colors: ResMut<BaseAndCube>,
    base_box: Query<Entity, With<BaseColorBox>>,
    cube_box: Query<Entity, With<CubeColorBox>>,
) {
    for SaturationValueBoxEvent {
        entity,
        saturation,
        value,
        ..
    } in events.read()
    {
        if base_box.single() == *entity {
            colors.base.saturation = *saturation;
            colors.base.value = *value;
        } else if cube_box.single() == *entity {
            colors.cube.saturation = *saturation;
            colors.cube.value = *value;
        }
    }
}

// Looks at events from hue-wheels.
// If found, checks which one the event stemmed from,
// then applies the color to either the cube or base.
fn hue_wheel_system(
    mut events: EventReader<HueWheelEvent>,
    mut colors: ResMut<BaseAndCube>,
    base_wheel: Query<Entity, With<BaseColorWheel>>,
    cube_wheel: Query<Entity, With<CubeColorWheel>>,
) {
    for HueWheelEvent { entity, hue } in events.read() {
        if base_wheel.single() == *entity {
            colors.base.hue = *hue;
        } else if cube_wheel.single() == *entity {
            colors.cube.hue = *hue;
        }
    }
}

fn update_colors(
    colors: Res<BaseAndCube>,
    base: Query<&Handle<StandardMaterial>, With<Base>>,
    cube: Query<&Handle<StandardMaterial>, With<Cube>>,
    mut color_materials: ResMut<Assets<StandardMaterial>>,
) {
    let Some(material) = color_materials.get_mut(base.single()) else {
        return;
    };

    material.base_color = hsv_to_rgb(colors.base.hue, colors.base.saturation, colors.base.value);

    let Some(material) = color_materials.get_mut(cube.single()) else {
        return;
    };

    material.base_color = hsv_to_rgb(colors.cube.hue, colors.cube.saturation, colors.cube.value);
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
            material: materials.add(Color::default().into()),
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
            material: materials.add(Color::default().into()),
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
    mut hue_materials: ResMut<Assets<HueWheelMaterial>>,
    mut satval_materials: ResMut<Assets<SaturationValueBoxMaterial>>,
) {
    let hue_wheel_material = HueWheelMaterial::default();

    // Given a desired px diameter of the wheel..
    let wheel_diameter = 200.0;

    // ..some math is required in order to have the inner box UI element
    // exactly touch the wheel
    let wheel_radius = wheel_diameter / 2.;
    let inner_wheel_radius = wheel_radius * hue_wheel_material.uniform.inner_radius;
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
                    .spawn((
                        HueWheelBundle {
                            style: Style {
                                position_type: PositionType::Absolute,
                                width: Val::Percent(100.),
                                height: Val::Percent(100.),
                                ..default()
                            },
                            material: hue_materials.add(hue_wheel_material.clone()),
                            ..default()
                        },
                        BaseColorWheel,
                    ))
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
                    BaseColorBox,
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
                    .spawn((
                        HueWheelBundle {
                            style: Style {
                                position_type: PositionType::Absolute,
                                width: Val::Percent(100.),
                                height: Val::Percent(100.),
                                ..default()
                            },
                            material: hue_materials.add(hue_wheel_material.clone()),
                            ..default()
                        },
                        CubeColorWheel,
                    ))
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
                    CubeColorBox,
                    // added to make the saturation-value box update automatically
                    HueWheelSibling(hue_wheel),
                ));
            });
        });
}
