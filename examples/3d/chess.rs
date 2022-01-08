use bevy::prelude::*;

const TILE_SIZE: f32 = 1.0;
const COORDINATE_OFFSET_X: f32 = 3.5;
const COORDINATE_OFFSET_Z: f32 = 3.5;
const WHITE_TILE_COLOR: Color = Color::rgb(1.0, 0.9, 0.9);
const BLACK_TILE_COLOR: Color = Color::rgb(0.0, 0.1, 0.1);
const WHITE_PIECE_COLOR: Color = Color::rgb(1.0, 0.8, 0.8);
const BLACK_PIECE_COLOR: Color = Color::rgb(0.0, 0.2, 0.2);
const PIECE_SCALE_X: f32 = 0.18;
const PIECE_SCALE_Y: f32 = 0.18;
const PIECE_SCALE_Z: f32 = 0.18;
const WHITE_PIECE_ROTATION_X: f32 = 1.65;
const WHITE_PIECE_ROTATION_Y: f32 = 1.35;
const BLACK_PIECE_ROTATION_X: f32 = 1.55;
const BLACK_PIECE_ROTATION_Y: f32 = 0.65;

fn main() {
    App::new()
        .insert_resource(Msaa { samples: 4 })
        .insert_resource(WindowDescriptor {
            title: "Chess!".to_string(),
            width: 1280.0,
            height: 720.0,
            ..Default::default()
        })
        .add_plugins(DefaultPlugins)
        .add_startup_system(setup)
        .add_startup_system(create_board)
        .add_startup_system(create_white_pieces)
        .add_startup_system(create_black_pieces)
        .run();
}

fn setup(mut commands: Commands) {
    commands.spawn_bundle(PerspectiveCameraBundle {
        transform: Transform::from_xyz(0.0, 5.0, 8.).looking_at(Vec3::ZERO, Vec3::Y),
        ..Default::default()
    });
    commands.spawn_bundle(PointLightBundle {
        transform: Transform::from_translation(Vec3::new(30.0, 30.0, 30.0)),
        point_light: PointLight {
            intensity: 600000.,
            range: 100.,
            ..Default::default()
        },
        ..Default::default()
    });
}

fn create_board(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let mesh = meshes.add(Mesh::from(shape::Plane { size: TILE_SIZE }));
    let white_material = materials.add(WHITE_TILE_COLOR.into());
    let black_material = materials.add(BLACK_TILE_COLOR.into());

    // Add 64 Squares
    for i in 0..8 {
        for j in 0..8 {
            commands.spawn_bundle(PbrBundle {
                mesh: mesh.clone(),
                // Iterate over i and j, check to see if the iteration is 0 or 1, if 0 place a
                //  while tile, if 1 place a black tile
                material: if (i + j + 1) % 2 == 0 {
                    white_material.clone()
                } else {
                    black_material.clone()
                },
                // Place the tile, maintaining alignment of the coordinate system near
                //  the center of the rendered board.
                transform: Transform::from_translation(Vec3::new(
                    i as f32 - COORDINATE_OFFSET_X,
                    0.0,
                    j as f32 - COORDINATE_OFFSET_Z,
                )),
                ..Default::default()
            });
        }
    }
}

fn create_white_pieces(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let king_handle: Handle<Mesh> =
        asset_server.load("models/chess/pieces-all.glb#Mesh0/Primitive0");
    let queen_handle: Handle<Mesh> =
        asset_server.load("models/chess/pieces-all.glb#Mesh1/Primitive0");
    let bishop_handle: Handle<Mesh> =
        asset_server.load("models/chess/pieces-all.glb#Mesh2/Primitive0");
    let knight_handle: Handle<Mesh> =
        asset_server.load("models/chess/pieces-all.glb#Mesh3/Primitive0");
    let rook_handle: Handle<Mesh> =
        asset_server.load("models/chess/pieces-all.glb#Mesh4/Primitive0");
    let pawn_handle: Handle<Mesh> =
        asset_server.load("models/chess/pieces-all.glb#Mesh5/Primitive0");

    let white_material = materials.add(WHITE_PIECE_COLOR.into());

    commands
        .spawn_bundle(PbrBundle {
            transform: Transform::from_translation(Vec3::new(0.0, 0.0, 0.0)),
            ..Default::default()
        })
        .with_children(|parent| {
            parent.spawn_bundle(PbrBundle {
                mesh: king_handle.clone(),
                material: white_material.clone(),
                transform: {
                    // Set King's Piece to the correct starting tile.
                    let mut transform = Transform::from_translation(Vec3::new(-0.5, 0., 3.8));
                    transform.apply_non_uniform_scale(Vec3::new(
                        PIECE_SCALE_X,
                        PIECE_SCALE_Y,
                        PIECE_SCALE_Z,
                    ));
                    transform.rotate(Quat::from_rotation_x(WHITE_PIECE_ROTATION_X));
                    transform.rotate(Quat::from_rotation_y(WHITE_PIECE_ROTATION_Y));
                    transform
                },
                ..Default::default()
            });
        })
        .id();

    commands
        .spawn_bundle(PbrBundle {
            transform: Transform::from_translation(Vec3::new(0., 0., 0.)),
            ..Default::default()
        })
        .with_children(|parent| {
            parent.spawn_bundle(PbrBundle {
                mesh: queen_handle.clone(),
                material: white_material.clone(),
                transform: {
                    // Set Queen's piece to the correct starting tile.
                    let mut transform = Transform::from_translation(Vec3::new(0.5, 0., 3.8));
                    transform.apply_non_uniform_scale(Vec3::new(
                        PIECE_SCALE_X,
                        PIECE_SCALE_Y,
                        PIECE_SCALE_Z,
                    ));
                    transform.rotate(Quat::from_rotation_x(WHITE_PIECE_ROTATION_X));
                    transform.rotate(Quat::from_rotation_y(WHITE_PIECE_ROTATION_Y));
                    transform
                },
                ..Default::default()
            });
        })
        .id();
    commands
        .spawn_bundle(PbrBundle {
            transform: Transform::from_translation(Vec3::new(0., 0., 0.)),
            ..Default::default()
        })
        .with_children(|parent| {
            parent.spawn_bundle(PbrBundle {
                mesh: bishop_handle.clone(),
                material: white_material.clone(),
                transform: {
                    // Set King's Bishop piece to the correct starting tile.
                    let mut transform = Transform::from_translation(Vec3::new(-1.5, 0., 3.8));
                    transform.apply_non_uniform_scale(Vec3::new(
                        PIECE_SCALE_X,
                        PIECE_SCALE_Y,
                        PIECE_SCALE_Z,
                    ));
                    transform.rotate(Quat::from_rotation_x(WHITE_PIECE_ROTATION_X));
                    transform.rotate(Quat::from_rotation_y(WHITE_PIECE_ROTATION_Y));
                    transform
                },
                ..Default::default()
            });
        })
        .id();
    commands
        .spawn_bundle(PbrBundle {
            transform: Transform::from_translation(Vec3::new(0., 0., 0.)),
            ..Default::default()
        })
        .with_children(|parent| {
            parent.spawn_bundle(PbrBundle {
                mesh: bishop_handle.clone(),
                material: white_material.clone(),
                transform: {
                    // Set Queen's Bishop piece to the correct starting tile.
                    let mut transform = Transform::from_translation(Vec3::new(1.5, 0., 3.8));
                    transform.apply_non_uniform_scale(Vec3::new(
                        PIECE_SCALE_X,
                        PIECE_SCALE_Y,
                        PIECE_SCALE_Z,
                    ));
                    transform.rotate(Quat::from_rotation_x(WHITE_PIECE_ROTATION_X));
                    transform.rotate(Quat::from_rotation_y(WHITE_PIECE_ROTATION_Y));
                    transform
                },
                ..Default::default()
            });
        })
        .id();
    commands
        .spawn_bundle(PbrBundle {
            transform: Transform::from_translation(Vec3::new(0., 0., 0.)),
            ..Default::default()
        })
        .with_children(|parent| {
            parent.spawn_bundle(PbrBundle {
                mesh: knight_handle.clone(),
                material: white_material.clone(),
                transform: {
                    // Set King's Knight piece to the correct starting tile.
                    let mut transform = Transform::from_translation(Vec3::new(-2.5, 0., 3.8));
                    transform.apply_non_uniform_scale(Vec3::new(
                        PIECE_SCALE_X,
                        PIECE_SCALE_Y,
                        PIECE_SCALE_Z,
                    ));
                    transform.rotate(Quat::from_rotation_x(WHITE_PIECE_ROTATION_X));
                    transform.rotate(Quat::from_rotation_y(WHITE_PIECE_ROTATION_Y));
                    transform
                },
                ..Default::default()
            });
        })
        .id();
    commands
        .spawn_bundle(PbrBundle {
            transform: Transform::from_translation(Vec3::new(0., 0., 0.)),
            ..Default::default()
        })
        .with_children(|parent| {
            parent.spawn_bundle(PbrBundle {
                mesh: knight_handle.clone(),
                material: white_material.clone(),
                transform: {
                    // Set Queen's Knight piece to the correct starting tile.
                    let mut transform = Transform::from_translation(Vec3::new(2.5, 0., 3.8));
                    transform.apply_non_uniform_scale(Vec3::new(
                        PIECE_SCALE_X,
                        PIECE_SCALE_Y,
                        PIECE_SCALE_Z,
                    ));
                    transform.rotate(Quat::from_rotation_x(WHITE_PIECE_ROTATION_X));
                    transform.rotate(Quat::from_rotation_y(WHITE_PIECE_ROTATION_Y));
                    transform
                },
                ..Default::default()
            });
        })
        .id();
    commands
        .spawn_bundle(PbrBundle {
            transform: Transform::from_translation(Vec3::new(0., 0., 0.)),
            ..Default::default()
        })
        .with_children(|parent| {
            parent.spawn_bundle(PbrBundle {
                mesh: rook_handle.clone(),
                material: white_material.clone(),
                transform: {
                    // Set King's Rook piece to the correct starting tile.
                    let mut transform = Transform::from_translation(Vec3::new(-3.5, 0., 3.8));
                    transform.apply_non_uniform_scale(Vec3::new(
                        PIECE_SCALE_X,
                        PIECE_SCALE_Y,
                        PIECE_SCALE_Z,
                    ));
                    transform.rotate(Quat::from_rotation_x(WHITE_PIECE_ROTATION_X));
                    transform.rotate(Quat::from_rotation_y(WHITE_PIECE_ROTATION_Y));
                    transform
                },
                ..Default::default()
            });
        })
        .id();
    commands
        .spawn_bundle(PbrBundle {
            transform: Transform::from_translation(Vec3::new(0., 0., 0.)),
            ..Default::default()
        })
        .with_children(|parent| {
            parent.spawn_bundle(PbrBundle {
                mesh: rook_handle.clone(),
                material: white_material.clone(),
                transform: {
                    // Set Queen's Rook piece to the correct starting tile.
                    let mut transform = Transform::from_translation(Vec3::new(3.5, 0., 3.8));
                    transform.apply_non_uniform_scale(Vec3::new(
                        PIECE_SCALE_X,
                        PIECE_SCALE_Y,
                        PIECE_SCALE_Z,
                    ));
                    transform.rotate(Quat::from_rotation_x(WHITE_PIECE_ROTATION_X));
                    transform.rotate(Quat::from_rotation_y(WHITE_PIECE_ROTATION_Y));
                    transform
                },
                ..Default::default()
            });
        })
        .id();

    // Iterate over tiles and place Pawn pieces from left -> right at the correct starting location.
    let x_position_start = -3.5;
    for idx in 0..8 {
        commands
            .spawn_bundle(PbrBundle {
                transform: Transform::from_translation(Vec3::new(0., 0., 0.)),
                ..Default::default()
            })
            .with_children(|parent| {
                parent.spawn_bundle(PbrBundle {
                    mesh: pawn_handle.clone(),
                    material: white_material.clone(),
                    transform: {
                        // Set Pawn piece at correct location
                        let mut transform = Transform::from_translation(Vec3::new(
                            x_position_start + (idx as f32),
                            0.0,
                            2.8,
                        ));
                        transform.apply_non_uniform_scale(Vec3::new(
                            PIECE_SCALE_X,
                            PIECE_SCALE_Y,
                            PIECE_SCALE_Z,
                        ));
                        transform.rotate(Quat::from_rotation_x(WHITE_PIECE_ROTATION_X));
                        transform.rotate(Quat::from_rotation_y(WHITE_PIECE_ROTATION_Y));
                        transform
                    },
                    ..Default::default()
                });
            })
            .id();
    }
}

fn create_black_pieces(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let king_handle: Handle<Mesh> =
        asset_server.load("models/chess/pieces-all.glb#Mesh0/Primitive0");
    let queen_handle: Handle<Mesh> =
        asset_server.load("models/chess/pieces-all.glb#Mesh1/Primitive0");
    let bishop_handle: Handle<Mesh> =
        asset_server.load("models/chess/pieces-all.glb#Mesh2/Primitive0");
    let knight_handle: Handle<Mesh> =
        asset_server.load("models/chess/pieces-all.glb#Mesh3/Primitive0");
    let rook_handle: Handle<Mesh> =
        asset_server.load("models/chess/pieces-all.glb#Mesh4/Primitive0");
    let pawn_handle: Handle<Mesh> =
        asset_server.load("models/chess/pieces-all.glb#Mesh5/Primitive0");
    let black_material: Handle<StandardMaterial> = materials.add(BLACK_PIECE_COLOR.into());

    commands
        .spawn_bundle(PbrBundle {
            transform: Transform::from_translation(Vec3::new(0.0, 0., 0.0)),
            ..Default::default()
        })
        .with_children(|parent| {
            parent.spawn_bundle(PbrBundle {
                mesh: king_handle.clone(),
                material: black_material.clone(),
                transform: {
                    // Set King piece to the correct starting tile
                    let mut transform = Transform::from_translation(Vec3::new(-0.5, 0., -3.0));
                    transform.apply_non_uniform_scale(Vec3::new(
                        PIECE_SCALE_X,
                        PIECE_SCALE_Y,
                        PIECE_SCALE_Z,
                    ));
                    transform.rotate(Quat::from_rotation_x(BLACK_PIECE_ROTATION_X));
                    transform.rotate(Quat::from_rotation_y(BLACK_PIECE_ROTATION_Y));
                    transform
                },
                ..Default::default()
            });
        })
        .id();

    commands
        .spawn_bundle(PbrBundle {
            transform: Transform::from_translation(Vec3::new(0., 0., 0.)),
            ..Default::default()
        })
        .with_children(|parent| {
            parent.spawn_bundle(PbrBundle {
                mesh: queen_handle.clone(),
                material: black_material.clone(),
                transform: {
                    // Set Queen piece to the correct starting tile
                    let mut transform = Transform::from_translation(Vec3::new(0.5, 0., -3.0));
                    transform.apply_non_uniform_scale(Vec3::new(
                        PIECE_SCALE_X,
                        PIECE_SCALE_Y,
                        PIECE_SCALE_Z,
                    ));
                    transform.rotate(Quat::from_rotation_x(BLACK_PIECE_ROTATION_X));
                    transform.rotate(Quat::from_rotation_y(BLACK_PIECE_ROTATION_Y));
                    transform
                },
                ..Default::default()
            });
        })
        .id();
    commands
        .spawn_bundle(PbrBundle {
            transform: Transform::from_translation(Vec3::new(0., 0., 0.)),
            ..Default::default()
        })
        .with_children(|parent| {
            parent.spawn_bundle(PbrBundle {
                mesh: bishop_handle.clone(),
                material: black_material.clone(),
                transform: {
                    // Set King's Bishop to the correct starting tile
                    let mut transform = Transform::from_translation(Vec3::new(-1.5, 0., -3.0));
                    transform.apply_non_uniform_scale(Vec3::new(
                        PIECE_SCALE_X,
                        PIECE_SCALE_Y,
                        PIECE_SCALE_Z,
                    ));
                    transform.rotate(Quat::from_rotation_x(BLACK_PIECE_ROTATION_X));
                    transform.rotate(Quat::from_rotation_y(BLACK_PIECE_ROTATION_Y));
                    transform
                },
                ..Default::default()
            });
        })
        .id();
    commands
        .spawn_bundle(PbrBundle {
            transform: Transform::from_translation(Vec3::new(0., 0., 0.)),
            ..Default::default()
        })
        .with_children(|parent| {
            parent.spawn_bundle(PbrBundle {
                mesh: bishop_handle.clone(),
                material: black_material.clone(),
                transform: {
                    // Set Queen's Bishop to the correct starting tile.
                    let mut transform = Transform::from_translation(Vec3::new(1.5, 0., -3.0));
                    transform.apply_non_uniform_scale(Vec3::new(
                        PIECE_SCALE_X,
                        PIECE_SCALE_Y,
                        PIECE_SCALE_Z,
                    ));
                    transform.rotate(Quat::from_rotation_x(BLACK_PIECE_ROTATION_X));
                    transform.rotate(Quat::from_rotation_y(BLACK_PIECE_ROTATION_Y));
                    transform
                },
                ..Default::default()
            });
        })
        .id();
    commands
        .spawn_bundle(PbrBundle {
            transform: Transform::from_translation(Vec3::new(0., 0., 0.)),
            ..Default::default()
        })
        .with_children(|parent| {
            parent.spawn_bundle(PbrBundle {
                mesh: knight_handle.clone(),
                material: black_material.clone(),
                transform: {
                    // Set King's Knight to the correct starting tile.
                    let mut transform = Transform::from_translation(Vec3::new(-2.5, 0., -3.0));
                    transform.apply_non_uniform_scale(Vec3::new(
                        PIECE_SCALE_X,
                        PIECE_SCALE_Y,
                        PIECE_SCALE_Z,
                    ));
                    transform.rotate(Quat::from_rotation_x(BLACK_PIECE_ROTATION_X));
                    transform.rotate(Quat::from_rotation_y(BLACK_PIECE_ROTATION_Y));
                    transform
                },
                ..Default::default()
            });
        })
        .id();
    commands
        .spawn_bundle(PbrBundle {
            transform: Transform::from_translation(Vec3::new(0., 0., 0.)),
            ..Default::default()
        })
        .with_children(|parent| {
            parent.spawn_bundle(PbrBundle {
                mesh: knight_handle.clone(),
                material: black_material.clone(),
                transform: {
                    // Set Queen's Knight to the correct starting tile
                    let mut transform = Transform::from_translation(Vec3::new(2.5, 0., -3.0));
                    transform.apply_non_uniform_scale(Vec3::new(
                        PIECE_SCALE_X,
                        PIECE_SCALE_Y,
                        PIECE_SCALE_Z,
                    ));
                    transform.rotate(Quat::from_rotation_x(BLACK_PIECE_ROTATION_X));
                    transform.rotate(Quat::from_rotation_y(BLACK_PIECE_ROTATION_Y));
                    transform
                },
                ..Default::default()
            });
        })
        .id();
    commands
        .spawn_bundle(PbrBundle {
            transform: Transform::from_translation(Vec3::new(0., 0., 0.)),
            ..Default::default()
        })
        .with_children(|parent| {
            parent.spawn_bundle(PbrBundle {
                mesh: rook_handle.clone(),
                material: black_material.clone(),
                transform: {
                    // Set the King's Rook to the correct starting tile
                    let mut transform = Transform::from_translation(Vec3::new(-3.5, 0., -3.0));
                    transform.apply_non_uniform_scale(Vec3::new(
                        PIECE_SCALE_X,
                        PIECE_SCALE_Y,
                        PIECE_SCALE_Z,
                    ));
                    transform.rotate(Quat::from_rotation_x(BLACK_PIECE_ROTATION_X));
                    transform.rotate(Quat::from_rotation_y(BLACK_PIECE_ROTATION_Y));
                    transform
                },
                ..Default::default()
            });
        })
        .id();
    commands
        .spawn_bundle(PbrBundle {
            transform: Transform::from_translation(Vec3::new(0., 0., 0.)),
            ..Default::default()
        })
        .with_children(|parent| {
            parent.spawn_bundle(PbrBundle {
                mesh: rook_handle.clone(),
                material: black_material.clone(),
                transform: {
                    // Set the Queen's Rook to the correct starting tile
                    let mut transform = Transform::from_translation(Vec3::new(3.5, 0., -3.0));
                    transform.apply_non_uniform_scale(Vec3::new(
                        PIECE_SCALE_X,
                        PIECE_SCALE_Y,
                        PIECE_SCALE_Z,
                    ));
                    transform.rotate(Quat::from_rotation_x(BLACK_PIECE_ROTATION_X));
                    transform.rotate(Quat::from_rotation_y(BLACK_PIECE_ROTATION_Y));
                    transform
                },
                ..Default::default()
            });
        })
        .id();

    // Iterate over tiles and place Pawn pieces from left -> right at the correct starting location.
    let x_position_start = -3.5;
    for idx in 0..8 {
        commands
            .spawn_bundle(PbrBundle {
                transform: Transform::from_translation(Vec3::new(0., 0., 0.)),
                ..Default::default()
            })
            .with_children(|parent| {
                parent.spawn_bundle(PbrBundle {
                    mesh: pawn_handle.clone(),
                    material: black_material.clone(),
                    transform: {
                        // Set Pawn piece at correct location
                        let mut transform = Transform::from_translation(Vec3::new(
                            x_position_start + (idx as f32),
                            0.,
                            -2.0,
                        ));
                        transform.apply_non_uniform_scale(Vec3::new(
                            PIECE_SCALE_X,
                            PIECE_SCALE_Y,
                            PIECE_SCALE_Z,
                        ));
                        transform.rotate(Quat::from_rotation_x(BLACK_PIECE_ROTATION_X));
                        transform.rotate(Quat::from_rotation_y(BLACK_PIECE_ROTATION_Y));
                        transform
                    },
                    ..Default::default()
                });
            })
            .id();
    }
}
