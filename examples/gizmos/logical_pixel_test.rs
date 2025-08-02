// // TODO : test the new changes for handling f32 and Val::Px enum based variants for consistent rendering
// // TODO : there's a lot of outdated APIs being used in this scenario, that requires being re-written/re-factored.

// use bevy::prelude::*;
// use bevy_gizmos::prelude::*;
// // use bevy_ui::Val;

// fn main() {
//     App::new()
//         .add_plugins(DefaultPlugins)
//         .add_systems(Startup, setup)
//         .add_systems(Update, (draw_test_gizmos, handle_input))
//         .run();
// }

// fn setup(mut commands: Commands) {
//     // Spawn camera
//     // TODO : fix this, Camera3dBundle no longer exists
//     // commands.spawn(Camera3dBundle {
//     //     transform: Transform::from_xyz(0.0, 0.0, 5.0),
//     //     ..default()
//     // });
//     //
//     // not entirely sure if this would work
//     commands.spawn((Camera3d::default(), Transform::from_xyz(0.0, 0.0, 5.0)));

//     // Add UI for displaying current scale factor
//     //
//     // refactored text spawn logic with upto date API
//     // commands
//     //     .spawn(NodeBundle {
//     //         style: Style {
//     //             position_type: PositionType::Absolute,
//     //             top: Val::Px(10.0),
//     //             left: Val::Px(10.0),
//     //             ..default()
//     //         },
//     //         ..default()
//     //     })
//     //     .with_children(|parent| {
//     //         parent.spawn(TextBundle::from_section(
//     //             "Scale Factor Test - Press 1-5 to change line widths",
//     //             TextStyle {
//     //                 font_size: 20.0,
//     //                 color: Color::WHITE,
//     //                 ..default()
//     //             },
//     //         ));
//     //     });
// }

// fn draw_test_gizmos(
//     mut gizmos: Gizmos,
//     keyboard: Res<ButtonInput<KeyCode>>,
//     windows: Query<&Window>,
// ) {
//     let window = windows.single();
//     let scale_factor = window.unwrap().scale_factor();

//     // Test different Val units
//     // TODO : find potential fix for this
//     // TODO : figure out a suitable alternative
//     let mut config = gizmos.config_mut();

//     if keyboard.pressed(KeyCode::Digit1) {
//         config.line.width = Val::Px(2.0); // 2 logical pixels
//     } else if keyboard.pressed(KeyCode::Digit2) {
//         config.line.width = Val::Px(4.0); // 4 logical pixels
//     } else if keyboard.pressed(KeyCode::Digit3) {
//         config.line.width = Val::Vw(0.5); // 0.5% of viewport width
//     } else if keyboard.pressed(KeyCode::Digit4) {
//         config.line.width = Val::Vh(0.5); // 0.5% of viewport height
//     } else if keyboard.pressed(KeyCode::Digit5) {
//         config.line.width = Val::VMin(1.0); // 1% of smaller viewport dimension
//     }

//     // Draw test patterns
//     gizmos.line(
//         Vec3::new(-2.0, 0.0, 0.0),
//         Vec3::new(2.0, 0.0, 0.0),
//         Color::RED,
//     );
//     gizmos.line(
//         Vec3::new(0.0, -2.0, 0.0),
//         Vec3::new(0.0, 2.0, 0.0),
//         Color::GREEN,
//     );
//     gizmos.circle(Vec3::ZERO, Vec3::Z, 1.0);

//     // Draw scale factor info
//     gizmos.line_2d(
//         Vec2::new(-100.0, -100.0),
//         Vec2::new(100.0, -100.0),
//         Color::YELLOW,
//     );
// }

// fn handle_input(keyboard: Res<ButtonInput<KeyCode>>, windows: Query<&Window>) {
//     if keyboard.just_pressed(KeyCode::Space) {
//         let window = windows.single();
//         println!("Current scale factor: {}", window.unwrap().scale_factor());
//     }
// }
