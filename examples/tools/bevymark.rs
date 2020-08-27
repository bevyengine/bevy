use bevy::{
    diagnostic::{Diagnostics, FrameTimeDiagnosticsPlugin},
    prelude::*,
};

const BIRDS_PER_SECOND: u32 = 1000;
const GRAVITY: f32 = -9.8;
const MAX_VELOCITY: f32 = 750.;
const BIRD_SCALE: f32 = 0.15;
const HALF_BIRD_SIZE: f32 = 256. * BIRD_SCALE * 0.5;
struct BevyCounter {
    pub count: u128,
}

struct Bird {
    velocity: Vec3,
}

fn main() {
    App::build()
        .add_resource(WindowDescriptor {
            title: "BevyMark".to_string(),
            width: 1000,
            height: 800,
            vsync: true,
            resizable: false,
            ..Default::default()
        })
        .add_default_plugins()
        .add_plugin(FrameTimeDiagnosticsPlugin::default())
        .add_resource(BevyCounter { count: 0 })
        .add_resource(Option::<Handle<ColorMaterial>>::None)
        .add_startup_system(setup.system())
        .add_system(mouse_handler.system())
        .add_system(movement_system.system())
        .add_system(collision_system.system())
        .add_system(counter_system.system())
        .run();
}

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut out_material_handle: ResMut<Option<Handle<ColorMaterial>>>,
) {
    *out_material_handle = Some(
        materials.add(
            asset_server
                .load("assets/branding/icon.png")
                .unwrap()
                .into(),
        ),
    );

    commands
        .spawn(Camera2dComponents::default())
        .spawn(UiCameraComponents::default())
        .spawn(TextComponents {
            text: Text {
                font: asset_server.load("assets/fonts/FiraSans-Bold.ttf").unwrap(),
                value: "Bird Count:".to_string(),
                style: TextStyle {
                    color: Color::rgb(0.0, 1.0, 0.0),
                    font_size: 40.0,
                },
            },
            style: Style {
                position_type: PositionType::Absolute,
                position: Rect {
                    top: Val::Px(5.0),
                    left: Val::Px(5.0),
                    ..Default::default()
                },
                ..Default::default()
            },
            ..Default::default()
        });
}

fn mouse_handler(
    mut commands: Commands,
    time: Res<Time>,
    mouse_button_input: Res<Input<MouseButton>>,
    window: Res<WindowDescriptor>,
    material_handle: Res<Option<Handle<ColorMaterial>>>,
    mut counter: ResMut<BevyCounter>,
) {
    if mouse_button_input.pressed(MouseButton::Left) {
        let spawn_count = (BIRDS_PER_SECOND as f32 * time.delta_seconds) as u128;
        let bird_x = (window.width as i32 / -2) as f32 + HALF_BIRD_SIZE;
        let bird_y = (window.height / 2) as f32 - HALF_BIRD_SIZE;

        for count in 0..spawn_count {
            let bird_position = Vec3::new(bird_x, bird_y, (counter.count + count) as f32 * 0.00001);

            commands
                .spawn(SpriteComponents {
                    material: material_handle.unwrap(),
                    translation: Translation(bird_position),
                    scale: Scale(BIRD_SCALE),
                    draw: Draw {
                        is_transparent: true,
                        ..Default::default()
                    },
                    ..Default::default()
                })
                .with(Bird {
                    velocity: Vec3::new(
                        rand::random::<f32>() * MAX_VELOCITY - (MAX_VELOCITY * 0.5),
                        0.,
                        0.,
                    ),
                });
        }

        counter.count += spawn_count;
    }
}

fn movement_system(time: Res<Time>, mut bird_query: Query<(&mut Bird, &mut Translation)>) {
    for (mut bird, mut translation) in &mut bird_query.iter() {
        translation.0 += bird.velocity * time.delta_seconds;

        let new_y = bird.velocity.y() + GRAVITY;
        bird.velocity.set_y(new_y);
    }
}

fn collision_system(
    window: Res<WindowDescriptor>,
    mut bird_query: Query<(&mut Bird, &Translation)>,
) {
    let half_width = window.width as f32 * 0.5;
    let half_height = window.height as f32 * 0.5;

    for (mut bird, translation) in &mut bird_query.iter() {
        let x_vel = bird.velocity.x();
        let y_vel = bird.velocity.y();
        let x_pos = translation.x();
        let y_pos = translation.y();

        if (x_vel > 0. && x_pos + HALF_BIRD_SIZE > half_width)
            || (x_vel <= 0. && x_pos - HALF_BIRD_SIZE < -(half_width))
        {
            bird.velocity.set_x(-x_vel);
        }
        if y_vel < 0. && y_pos - HALF_BIRD_SIZE < -half_height {
            bird.velocity.set_y(-y_vel);
        }
    }
}

fn counter_system(
    diagnostics: Res<Diagnostics>,
    counter: Res<BevyCounter>,
    mut query: Query<&mut Text>,
) {
    let mut fps_value = 0.;

    if let Some(fps) = diagnostics.get(FrameTimeDiagnosticsPlugin::FPS) {
        if let Some(average) = fps.average() {
            fps_value = average;
        }
    };

    for mut text in &mut query.iter() {
        text.value = format!(
            "Bird Count: {}\nAverage FPS: {:.2}",
            counter.count, fps_value,
        );
    }
}
