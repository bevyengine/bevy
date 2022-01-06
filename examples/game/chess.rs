use bevy::prelude::*;

fn main() {
    App::new()
        .insert_resource(Msaa { samples: 4})
        .insert_resource(WindowDescriptor {
            title: "Chess!".to_string(),
            width: 1600.0,
            height: 1600.0,
            ..Default::default()
        })
        .add_plugins(DefaultPlugins)
        .add_startup_system(setup)
        .add_startup_system(create_board)
        .add_startup_system(create_game)
        .add_startup_system(create_white_pieces)
        .add_startup_system(create_black_pieces)
        .run();
}

fn setup(
    mut commands: Commands,
) {
    commands.spawn_bundle(PerspectiveCameraBundle {
        transform: Transform::from_xyz(0.0, 5.0, 8.)
            .looking_at(Vec3::ZERO, Vec3::Y),
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
    let mesh = meshes.add(Mesh::from(shape::Plane {size: 1.0 }));
    let white_material = materials.add(Color::rgb(1.0, 0.9, 0.9).into());
    let black_material = materials.add(Color::rgb(0.0, 0.1, 0.1).into());

    // Add 64 Squares
    for i in 0..8 {
        for j in 0..8 {
            commands.spawn_bundle(PbrBundle {
                mesh: mesh.clone(),
                material: if (i + j + 1) % 2 == 0 {
                    white_material.clone()
                } else {
                    black_material.clone()
                },
                transform: Transform::from_translation(Vec3::new(i as f32 - 3.5, 0., j as f32 - 3.5 )),
                ..Default::default()
            });
        }
    }
}

fn create_game (
    mut commands: Commands,
) {

}

fn create_white_pieces (
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let king_handle: Handle<Mesh> = asset_server.load("models/all.glb#Mesh0/Primitive0");
    let queen_handle: Handle<Mesh> = asset_server.load("models/all.glb#Mesh1/Primitive0");
    let bishop_handle: Handle<Mesh> = asset_server.load("models/all.glb#Mesh2/Primitive0");
    let knight_handle: Handle<Mesh> = asset_server.load("models/all.glb#Mesh3/Primitive0");
    let rook_handle: Handle<Mesh> = asset_server.load("models/all.glb#Mesh4/Primitive0");
    let pawn_handle: Handle<Mesh> = asset_server.load("models/all.glb#Mesh5/Primitive0");
    let white_material = materials.add(Color::rgb(1.0, 0.8, 0.8).into());
    

    let king_identity = commands.spawn_bundle(PbrBundle {
        transform: Transform::from_translation(Vec3::new(0.0, 0., 0.0)),
        ..Default::default()
    })
    .with_children(|parent| {
        parent.spawn_bundle(PbrBundle {
            mesh: king_handle.clone(),
            material: white_material.clone(),
            transform: {
                let mut transform = Transform::from_translation(Vec3::new(-0.5, 0., 3.8));
                transform.apply_non_uniform_scale(Vec3::new(0.18, 0.18, 0.18));
                transform.rotate(Quat::from_rotation_x(1.65));
                transform.rotate(Quat::from_rotation_y(1.35));
                transform
            },
            ..Default::default()
        });
    }).id();
    
    let queen_identity = commands.spawn_bundle(PbrBundle {
        transform: Transform::from_translation(Vec3::new(0. ,0., 0.)),
        ..Default::default()
    })
    .with_children(|parent| {
        parent.spawn_bundle(PbrBundle {
            mesh: queen_handle.clone(),
            material: white_material.clone(),
            transform: {
                let mut transform = Transform::from_translation(Vec3::new(0.5, 0., 3.8));
                transform.apply_non_uniform_scale(Vec3::new(0.18, 0.18, 0.18));
                transform.rotate(Quat::from_rotation_x(1.65));
                transform.rotate(Quat::from_rotation_y(1.35));
                transform
            },
            ..Default::default()
        });
    }).id();
    let king_bishop_identity = commands.spawn_bundle(PbrBundle {
        transform: Transform::from_translation(Vec3::new(0., 0., 0.)),
        ..Default::default()
    })
    .with_children(|parent| {
        parent.spawn_bundle(PbrBundle {
            mesh: bishop_handle.clone(),
            material: white_material.clone(),
            transform: {
                let mut transform = Transform::from_translation(Vec3::new(-1.5, 0., 3.8));
                transform.apply_non_uniform_scale(Vec3::new(0.18, 0.18, 0.18));
                transform.rotate(Quat::from_rotation_x(1.65));
                transform.rotate(Quat::from_rotation_y(1.35));
                transform
            },
            ..Default::default()
        });
    }).id();
    let queen_bishop_identity = commands.spawn_bundle(PbrBundle {
        transform: Transform::from_translation(Vec3::new(0., 0., 0.)),
        ..Default::default()
    })
    .with_children(|parent| {
        parent.spawn_bundle(PbrBundle {
            mesh: bishop_handle.clone(),
            material: white_material.clone(),
            transform: {
                let mut transform = Transform::from_translation(Vec3::new(1.5, 0., 3.8));
                transform.apply_non_uniform_scale(Vec3::new(0.18, 0.18, 0.18));
                transform.rotate(Quat::from_rotation_x(1.65));
                transform.rotate(Quat::from_rotation_y(1.35));
                transform
            },
            ..Default::default()
        });
    }).id();
    let king_knight_identity = commands.spawn_bundle(PbrBundle {
        transform: Transform::from_translation(Vec3::new(0., 0., 0.)),
        ..Default::default()
    })
    .with_children(|parent| {
        parent.spawn_bundle(PbrBundle {
            mesh: knight_handle.clone(),
            material: white_material.clone(),
            transform: {
                let mut transform = Transform::from_translation(Vec3::new(-2.5, 0., 3.8));
                transform.apply_non_uniform_scale(Vec3::new(0.18, 0.18, 0.18));
                transform.rotate(Quat::from_rotation_x(1.65));
                transform.rotate(Quat::from_rotation_y(1.35));
                transform
            },
            ..Default::default()
        });
    }).id();
    let queen_knight_identity = commands.spawn_bundle(PbrBundle {
        transform: Transform::from_translation(Vec3::new(0., 0., 0.)),
        ..Default::default()
    })
    .with_children(|parent| {
        parent.spawn_bundle(PbrBundle {
            mesh: knight_handle.clone(),
            material: white_material.clone(),
            transform: {
                let mut transform = Transform::from_translation(Vec3::new(2.5, 0., 3.8));
                transform.apply_non_uniform_scale(Vec3::new(0.18, 0.18, 0.18));
                transform.rotate(Quat::from_rotation_x(1.65));
                transform.rotate(Quat::from_rotation_y(1.35));
                transform
            },
            ..Default::default()
        });
    }).id();
    let king_rook_identity = commands.spawn_bundle(PbrBundle {
        transform: Transform::from_translation(Vec3::new(0., 0., 0.)),
        ..Default::default()
    })
    .with_children(|parent| {
        parent.spawn_bundle(PbrBundle {
            mesh: rook_handle.clone(),
            material: white_material.clone(),
            transform: {
                let mut transform = Transform::from_translation(Vec3::new(-3.5, 0., 3.8));
                transform.apply_non_uniform_scale(Vec3::new(0.18, 0.18, 0.18));
                transform.rotate(Quat::from_rotation_x(1.65));
                transform.rotate(Quat::from_rotation_y(1.35));
                transform
            },
            ..Default::default()
        });
    }).id();
    let queen_rook_identity  = commands.spawn_bundle(PbrBundle {
        transform: Transform::from_translation(Vec3::new(0., 0., 0.)),
        ..Default::default()
    })
    .with_children(|parent| {
        parent.spawn_bundle(PbrBundle {
            mesh: rook_handle.clone(),
            material: white_material.clone(),
            transform: {
                let mut transform = Transform::from_translation(Vec3::new(3.5, 0., 3.8));
                transform.apply_non_uniform_scale(Vec3::new(0.18, 0.18, 0.18));
                transform.rotate(Quat::from_rotation_x(1.65));
                transform.rotate(Quat::from_rotation_y(1.35));
                transform
            },
            ..Default::default()
        });
    }).id();
    let x_position_start = -3.5;
    for idx in 0..8 {
        let pawn_identity = commands.spawn_bundle(PbrBundle {
            transform: Transform::from_translation(Vec3::new(0., 0., 0.)),
            ..Default::default()
        }).with_children(|parent| {
            parent.spawn_bundle(PbrBundle {
                mesh: pawn_handle.clone(),
                material: white_material.clone(),
                transform: {
                    let mut transform = Transform::from_translation(Vec3::new(x_position_start + (idx as f32), 0., 2.8));
                    transform.apply_non_uniform_scale(Vec3::new(0.18, 0.18, 0.18));
                    transform.rotate(Quat::from_rotation_x(1.65));
                    transform.rotate(Quat::from_rotation_y(1.35));
                    transform
                },
                ..Default::default()
            });
        }).id();
    }
}

fn create_black_pieces (
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let king_handle: Handle<Mesh> = asset_server.load("models/all.glb#Mesh0/Primitive0");
    let queen_handle: Handle<Mesh> = asset_server.load("models/all.glb#Mesh1/Primitive0");
    let bishop_handle: Handle<Mesh> = asset_server.load("models/all.glb#Mesh2/Primitive0");
    let knight_handle: Handle<Mesh> = asset_server.load("models/all.glb#Mesh3/Primitive0");
    let rook_handle: Handle<Mesh> = asset_server.load("models/all.glb#Mesh4/Primitive0");
    let pawn_handle: Handle<Mesh> = asset_server.load("models/all.glb#Mesh5/Primitive0");
    let black_material = materials.add(Color::rgb(0.0, 0.2, 0.2).into());

    let king_identity = commands.spawn_bundle(PbrBundle {
        transform: Transform::from_translation(Vec3::new(0.0, 0., 0.0)),
        ..Default::default()
    })
    .with_children(|parent| {
        parent.spawn_bundle(PbrBundle {
            mesh: king_handle.clone(),
            material: black_material.clone(),
            transform: {
                let mut transform = Transform::from_translation(Vec3::new(-0.5, 0., -3.0));
                transform.apply_non_uniform_scale(Vec3::new(0.18, 0.18, 0.18));
                transform.rotate(Quat::from_rotation_x(1.55));
                transform.rotate(Quat::from_rotation_y(1.35));
                transform
            },
            ..Default::default()
        });
    }).id();
    
    let queen_identity = commands.spawn_bundle(PbrBundle {
        transform: Transform::from_translation(Vec3::new(0. ,0., 0.)),
        ..Default::default()
    })
    .with_children(|parent| {
        parent.spawn_bundle(PbrBundle {
            mesh: queen_handle.clone(),
            material: black_material.clone(),
            transform: {
                let mut transform = Transform::from_translation(Vec3::new(0.5, 0., -3.0));
                transform.apply_non_uniform_scale(Vec3::new(0.18, 0.18, 0.18));
                transform.rotate(Quat::from_rotation_x(1.55));
                transform.rotate(Quat::from_rotation_y(1.35));
                transform
            },
            ..Default::default()
        });
    }).id();
    let king_bishop_identity = commands.spawn_bundle(PbrBundle {
        transform: Transform::from_translation(Vec3::new(0., 0., 0.)),
        ..Default::default()
    })
    .with_children(|parent| {
        parent.spawn_bundle(PbrBundle {
            mesh: bishop_handle.clone(),
            material: black_material.clone(),
            transform: {
                let mut transform = Transform::from_translation(Vec3::new(-1.5, 0., -3.0));
                transform.apply_non_uniform_scale(Vec3::new(0.18, 0.18, 0.18));
                transform.rotate(Quat::from_rotation_x(1.55));
                transform.rotate(Quat::from_rotation_y(1.35));
                transform
            },
            ..Default::default()
        });
    }).id();
    let queen_bishop_identity = commands.spawn_bundle(PbrBundle {
        transform: Transform::from_translation(Vec3::new(0., 0., 0.)),
        ..Default::default()
    })
    .with_children(|parent| {
        parent.spawn_bundle(PbrBundle {
            mesh: bishop_handle.clone(),
            material: black_material.clone(),
            transform: {
                let mut transform = Transform::from_translation(Vec3::new(1.5, 0.,  -3.0));
                transform.apply_non_uniform_scale(Vec3::new(0.18, 0.18, 0.18));
                transform.rotate(Quat::from_rotation_x(1.55));
                transform.rotate(Quat::from_rotation_y(1.35));
                transform
            },
            ..Default::default()
        });
    }).id();
    let king_knight_identity = commands.spawn_bundle(PbrBundle {
        transform: Transform::from_translation(Vec3::new(0., 0., 0.)),
        ..Default::default()
    })
    .with_children(|parent| {
        parent.spawn_bundle(PbrBundle {
            mesh: knight_handle.clone(),
            material: black_material.clone(),
            transform: {
                let mut transform = Transform::from_translation(Vec3::new(-2.5, 0.,  -3.0));
                transform.apply_non_uniform_scale(Vec3::new(0.18, 0.18, 0.18));
                transform.rotate(Quat::from_rotation_x(1.55));
                transform.rotate(Quat::from_rotation_y(0.));
                transform
            },
            ..Default::default()
        });
    }).id();
    let queen_knight_identity = commands.spawn_bundle(PbrBundle {
        transform: Transform::from_translation(Vec3::new(0., 0., 0.)),
        ..Default::default()
    })
    .with_children(|parent| {
        parent.spawn_bundle(PbrBundle {
            mesh: knight_handle.clone(),
            material: black_material.clone(),
            transform: {
                let mut transform = Transform::from_translation(Vec3::new(2.5, 0.,  -3.0));
                transform.apply_non_uniform_scale(Vec3::new(0.18, 0.18, 0.18));
                transform.rotate(Quat::from_rotation_x(1.55));
                transform.rotate(Quat::from_rotation_y(0.));
                transform
            },
            ..Default::default()
        });
    }).id();
    let king_rook_identity = commands.spawn_bundle(PbrBundle {
        transform: Transform::from_translation(Vec3::new(0., 0., 0.)),
        ..Default::default()
    })
    .with_children(|parent| {
        parent.spawn_bundle(PbrBundle {
            mesh: rook_handle.clone(),
            material: black_material.clone(),
            transform: {
                let mut transform = Transform::from_translation(Vec3::new(-3.5, 0.,  -3.0));
                transform.apply_non_uniform_scale(Vec3::new(0.18, 0.18, 0.18));
                transform.rotate(Quat::from_rotation_x(1.55));
                transform.rotate(Quat::from_rotation_y(1.35));
                transform
            },
            ..Default::default()
        });
    }).id();
    let queen_rook_identity  = commands.spawn_bundle(PbrBundle {
        transform: Transform::from_translation(Vec3::new(0., 0., 0.)),
        ..Default::default()
    })
    .with_children(|parent| {
        parent.spawn_bundle(PbrBundle {
            mesh: rook_handle.clone(),
            material: black_material.clone(),
            transform: {
                let mut transform = Transform::from_translation(Vec3::new(3.5, 0.,  -3.0));
                transform.apply_non_uniform_scale(Vec3::new(0.18, 0.18, 0.18));
                transform.rotate(Quat::from_rotation_x(1.55));
                transform.rotate(Quat::from_rotation_y(1.35));
                transform
            },
            ..Default::default()
        });
    }).id();
    let x_position_start = -3.5;
    for idx in 0..8 {
        let pawn_identity = commands.spawn_bundle(PbrBundle {
            transform: Transform::from_translation(Vec3::new(0., 0., 0.)),
            ..Default::default()
        }).with_children(|parent| {
            parent.spawn_bundle(PbrBundle {
                mesh: pawn_handle.clone(),
                material: black_material.clone(),
                transform: {
                    let mut transform = Transform::from_translation(Vec3::new(x_position_start + (idx as f32), 0.,  -2.0));
                    transform.apply_non_uniform_scale(Vec3::new(0.18, 0.18, 0.18));
                    transform.rotate(Quat::from_rotation_x(1.55));
                    transform.rotate(Quat::from_rotation_y(1.35));
                    transform
                },
                ..Default::default()
            });
        }).id();
    }
}