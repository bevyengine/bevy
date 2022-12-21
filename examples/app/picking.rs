//! An example that shows how both 3D meshes and UI entities may be "picked" by
//! using the cursor.
//!
//! Combines parts of the 3D shapes example and the UI example.

use std::f32::consts::PI;

use bevy::{
    input::mouse::{MouseScrollUnit, MouseWheel},
    prelude::*,
    render::picking::{PickedEvent, PickedEventVariant, Picking},
};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_startup_system(setup)
        .add_startup_system(setup_ui)
        .add_system(rotate_shapes)
        .add_system(mouse_scroll)
        .add_system(picking_shapes)
        .add_system(picking_logo)
        .add_system(picking_text)
        .run();
}

/// A marker component for our shapes so we can query them separately from the ground plane
#[derive(Component)]
struct Shape;

const X_EXTENT: f32 = 14.5;

const LOGO_NORMAL: f32 = 500.0;
const LOGO_HOVERED: f32 = 600.0;

const COLOR_NORMAL: Color = Color::WHITE;
const COLOR_HOVERED: Color = Color::GOLD;

#[derive(Resource, Deref, DerefMut)]
struct NormalMaterial(Handle<StandardMaterial>);

#[derive(Resource, Deref, DerefMut)]
struct HoveredMaterial(Handle<StandardMaterial>);

#[derive(Resource, Deref, DerefMut)]
struct SelectedMaterial(Handle<StandardMaterial>);

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let normal = materials.add(COLOR_NORMAL.into());

    commands.insert_resource(NormalMaterial(normal.clone()));
    commands.insert_resource(HoveredMaterial(materials.add(COLOR_HOVERED.into())));

    let shapes = [
        meshes.add(shape::Cube::default().into()),
        meshes.add(shape::Box::default().into()),
        meshes.add(shape::Capsule::default().into()),
        meshes.add(shape::Torus::default().into()),
        meshes.add(shape::Cylinder::default().into()),
        meshes.add(shape::Icosphere::default().try_into().unwrap()),
        meshes.add(shape::UVSphere::default().into()),
    ];

    let num_shapes = shapes.len();

    for (i, shape) in shapes.into_iter().enumerate() {
        commands.spawn((
            PbrBundle {
                mesh: shape,
                material: normal.clone(),
                transform: Transform::from_xyz(
                    -X_EXTENT / 2. + i as f32 / (num_shapes - 1) as f32 * X_EXTENT,
                    2.0,
                    0.0,
                )
                .with_rotation(Quat::from_rotation_x(-PI / 4.)),
                ..default()
            },
            Shape,
        ));
    }

    commands.spawn(PointLightBundle {
        point_light: PointLight {
            intensity: 9000.0,
            range: 100.,
            shadows_enabled: true,
            ..default()
        },
        transform: Transform::from_xyz(8.0, 16.0, 8.0),
        ..default()
    });

    // ground plane
    commands.spawn(PbrBundle {
        mesh: meshes.add(shape::Plane { size: 50. }.into()),
        material: materials.add(Color::SILVER.into()),
        ..default()
    });

    commands.spawn((
        Camera3dBundle {
            transform: Transform::from_xyz(0.0, 6., 12.0)
                .looking_at(Vec3::new(0., 1., 0.), Vec3::Y),
            ..default()
        },
        Picking::default(),
    ));
}

fn setup_ui(mut commands: Commands, asset_server: Res<AssetServer>) {
    // root node
    commands
        .spawn(NodeBundle {
            style: Style {
                size: Size::new(Val::Percent(100.0), Val::Percent(100.0)),
                justify_content: JustifyContent::SpaceBetween,
                ..default()
            },
            ..default()
        })
        .with_children(|parent| {
            // right vertical fill
            parent
                .spawn(NodeBundle {
                    style: Style {
                        flex_direction: FlexDirection::Column,
                        justify_content: JustifyContent::Center,
                        size: Size::new(Val::Px(200.0), Val::Percent(100.0)),
                        ..default()
                    },
                    background_color: Color::rgb(0.15, 0.15, 0.15).into(),
                    ..default()
                })
                .with_children(|parent| {
                    // Title
                    parent.spawn(
                        TextBundle::from_section(
                            "Scrolling list",
                            TextStyle {
                                font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                                font_size: 25.,
                                color: Color::WHITE,
                            },
                        )
                        .with_style(Style {
                            size: Size::new(Val::Undefined, Val::Px(25.)),
                            margin: UiRect {
                                left: Val::Auto,
                                right: Val::Auto,
                                ..default()
                            },
                            ..default()
                        }),
                    );
                    // List with hidden overflow
                    parent
                        .spawn(NodeBundle {
                            style: Style {
                                flex_direction: FlexDirection::Column,
                                align_self: AlignSelf::Center,
                                size: Size::new(Val::Percent(100.0), Val::Percent(50.0)),
                                overflow: Overflow::Hidden,
                                ..default()
                            },
                            background_color: Color::rgb(0.10, 0.10, 0.10).into(),
                            ..default()
                        })
                        .with_children(|parent| {
                            // Moving panel
                            parent
                                .spawn((
                                    NodeBundle {
                                        style: Style {
                                            flex_direction: FlexDirection::Column,
                                            flex_grow: 1.0,
                                            max_size: Size::UNDEFINED,
                                            ..default()
                                        },
                                        ..default()
                                    },
                                    ScrollingList::default(),
                                ))
                                .with_children(|parent| {
                                    // List items
                                    for i in 0..50 {
                                        parent.spawn(
                                            TextBundle::from_section(
                                                format!("Item {i}"),
                                                TextStyle {
                                                    font: asset_server
                                                        .load("fonts/FiraSans-Bold.ttf"),
                                                    font_size: 30.,
                                                    color: COLOR_NORMAL,
                                                },
                                            )
                                            .with_style(Style {
                                                flex_shrink: 0.,
                                                size: Size::new(Val::Undefined, Val::Px(20.)),
                                                margin: UiRect {
                                                    left: Val::Auto,
                                                    right: Val::Auto,
                                                    ..default()
                                                },
                                                ..default()
                                            }),
                                        );
                                    }
                                });
                        });
                });
            // bevy logo (flex center)
            parent
                .spawn(NodeBundle {
                    style: Style {
                        size: Size::new(Val::Percent(100.0), Val::Percent(100.0)),
                        position_type: PositionType::Absolute,
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::FlexStart,
                        ..default()
                    },
                    ..default()
                })
                .with_children(|parent| {
                    // bevy logo (image)
                    parent.spawn(ImageBundle {
                        style: Style {
                            size: Size::new(Val::Px(LOGO_NORMAL), Val::Auto),
                            ..default()
                        },
                        image: asset_server.load("branding/bevy_logo_dark_big.png").into(),
                        ..default()
                    });
                });
        });
}

#[derive(Component, Default)]
struct ScrollingList {
    position: f32,
}

fn mouse_scroll(
    mut mouse_wheel_events: EventReader<MouseWheel>,
    mut query_list: Query<(&mut ScrollingList, &mut Style, &Children, &Node)>,
    query_item: Query<&Node>,
) {
    for mouse_wheel_event in mouse_wheel_events.iter() {
        for (mut scrolling_list, mut style, children, uinode) in &mut query_list {
            let items_height: f32 = children
                .iter()
                .map(|entity| query_item.get(*entity).unwrap().size().y)
                .sum();
            let panel_height = uinode.size().y;
            let max_scroll = (items_height - panel_height).max(0.);
            let dy = match mouse_wheel_event.unit {
                MouseScrollUnit::Line => mouse_wheel_event.y * 20.,
                MouseScrollUnit::Pixel => mouse_wheel_event.y,
            };
            scrolling_list.position += dy;
            scrolling_list.position = scrolling_list.position.clamp(-max_scroll, 0.);
            style.position.top = Val::Px(scrolling_list.position);
        }
    }
}

fn rotate_shapes(mut query: Query<&mut Transform, With<Shape>>, time: Res<Time>) {
    for mut transform in &mut query {
        transform.rotate_y(time.delta_seconds() / 2.);
    }
}

fn picking_shapes(
    mut pick_events: EventReader<PickedEvent>,
    mut shapes: Query<&mut Handle<StandardMaterial>, With<Shape>>,
    normal: Res<NormalMaterial>,
    hovered: Res<HoveredMaterial>,
) {
    for pick_event in pick_events.iter() {
        let PickedEvent { entity, event } = pick_event;

        match event {
            PickedEventVariant::Picked => {
                if let Ok(mut material_handle) = shapes.get_mut(*entity) {
                    *material_handle = hovered.clone();
                }
            }
            PickedEventVariant::Unpicked => {
                if let Ok(mut material_handle) = shapes.get_mut(*entity) {
                    *material_handle = normal.clone();
                }
            }
        }
    }
}

fn picking_logo(
    mut pick_events: EventReader<PickedEvent>,
    mut logo: Query<&mut Style, With<UiImage>>,
) {
    for pick_event in pick_events.iter() {
        let PickedEvent { entity, event } = pick_event;

        match event {
            PickedEventVariant::Picked => {
                if let Ok(mut style) = logo.get_mut(*entity) {
                    style.size = Size::new(Val::Px(LOGO_HOVERED), Val::Auto);
                }
            }
            PickedEventVariant::Unpicked => {
                if let Ok(mut style) = logo.get_mut(*entity) {
                    style.size = Size::new(Val::Px(LOGO_NORMAL), Val::Auto);
                }
            }
        }
    }
}

fn picking_text(mut pick_events: EventReader<PickedEvent>, mut texts: Query<&mut Text>) {
    for pick_event in pick_events.iter() {
        let PickedEvent { entity, event } = pick_event;

        match event {
            PickedEventVariant::Picked => {
                if let Ok(mut text) = texts.get_mut(*entity) {
                    for section in &mut text.sections {
                        section.style.color = COLOR_HOVERED;
                    }
                }
            }
            PickedEventVariant::Unpicked => {
                if let Ok(mut text) = texts.get_mut(*entity) {
                    for section in &mut text.sections {
                        section.style.color = COLOR_NORMAL;
                    }
                }
            }
        }
    }
}
