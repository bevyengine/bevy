//! Custom curve animation sampling
use bevy::{animation::ActiveAnimation, input::common_conditions::input_just_pressed, prelude::*};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .init_resource::<AnimationControl>()
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                keyboard_control,
                pickup_sprite.run_if(input_just_pressed(MouseButton::Left)),
                dorp_sprite.run_if(input_just_pressed(MouseButton::Right)),
                update_text.run_if(resource_changed::<AnimationControl>),
            ),
        )
        .add_systems(PostUpdate, myanimation_system)
        .run();
}

///update text
#[derive(Component)]
pub struct AnimationControlTextMark;

/// Provide numerical values for animation.(computer translation)
#[derive(Resource)]
pub struct AnimationControl {
    speed: f32,
    distance: f32,
}

impl Default for AnimationControl {
    fn default() -> Self {
        Self {
            speed: 100.0,
            distance: 1.0,
        }
    }
}

#[derive(Component)]
struct MyAnimationCurve {
    active: ActiveAnimation,
    // actually,I don't want use dyn
    curve: FunctionCurve<Vec<f32>, Box<(dyn Fn(f32) -> Vec<f32> + 'static + Send + Sync)>>,
}

#[derive(Component, Default, PartialEq, Eq)]
enum PickupState {
    #[default]
    Waiting,
    Picking,
    Move,
}

fn setup(mut commands: Commands, control: Res<AnimationControl>, asset_server: Res<AssetServer>) {
    commands.spawn(Camera2d::default());

    commands
        .spawn(Node {
            ..Default::default()
        })
        .with_children(|parent| {
            let text = format!(
                "speed:{}\ndistance_proportion:{}",
                control.speed, control.distance
            );

            parent.spawn((
                Text::new(text),
                TextFont {
                    font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                    font_size: 33.0,
                    ..default()
                },
                TextColor(Color::srgb(0.9, 0.9, 0.9)),
                AnimationControlTextMark,
            ));
        });
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            bottom: Val::Px(0.0),
            ..Default::default()
        },
        Text::new(
            "
ArrowUp and ArrowDown control distance proportion.
ArrowLeft and ArrowRight control speed.",
        ),
        TextFont {
            font: asset_server.load("fonts/FiraSans-Bold.ttf"),
            font_size: 33.0,
            ..default()
        },
        TextColor(Color::srgb(0.9, 0.9, 0.9)),
    ));

    commands.spawn((
        Sprite {
            color: Color::srgb(0.0, 0.0, 1.0),
            custom_size: Some(vec2(64.0, 64.0)),
            ..Default::default()
        },
        PickupState::default(),
    ));
}

fn pickup_sprite(
    mut sprites: Query<(&Transform, &mut Sprite, &mut PickupState)>,
    windows: Query<&Window>,
    cameras: Query<(&Camera, &GlobalTransform)>,
) {
    let window = windows.single();
    let Ok((camera, position)) = cameras.single() else {
        error!("camera isn't single");
        return;
    };

    let Some(world_position) = window
        .unwrap()
        .cursor_position()
        .map(|cursor| camera.viewport_to_world(position, cursor))
        .map(|ray| ray.unwrap().origin.truncate())
    else {
        return;
    };

    for (transform, mut sprite, mut pickstate) in &mut sprites {
        if *pickstate != PickupState::Waiting {
            continue;
        }

        let scaled = sprite.custom_size.unwrap_or(vec2(0.0, 0.0)) * transform.scale.truncate();
        let bounding_box = Rect::from_center_size(transform.translation.truncate(), scaled);

        if bounding_box.contains(world_position) {
            *pickstate = PickupState::Picking;
            sprite.color = Color::srgb(1.0, 0.0, 0.0);
        }
    }
}

fn drop_sprite(
    mut commands: Commands,
    windows: Query<&Window>,
    cameras: Query<(&Camera, &GlobalTransform)>,
    query: Query<(&mut PickupState, &Transform, &mut Sprite, Entity)>,
    control: Res<AnimationControl>,
) {
    let window = windows.single();
    let Ok((camera, position)) = cameras.single() else {
        return;
    };
    let Some(world_position) = window
        .unwrap()
        .cursor_position()
        .map(|cursor| camera.viewport_to_world(position, cursor))
        .map(|ray| ray.unwrap().origin.truncate())
    else {
        return;
    };

    for (mut pickupstate, trans, mut sprite, entity) in query {
        if *pickupstate == PickupState::Picking {
            let xy = trans.translation.xy();
            let distance = xy.distance(world_position).abs();

            let transx = trans.translation.x;
            let transy = trans.translation.y;

            let proportion = control.distance;
            let speed = control.speed;

            let animatiom = MyAnimationCurve {
                active: *ActiveAnimation::default()
                    .set_clip_duration(distance)
                    .set_speed(speed),
                curve: FunctionCurve::new(
                    Interval::new(0.0, distance).unwrap(),
                    // You can customize mathematical calculations here,
                    // Actually, it can also support iter.(computer translation)
                    Box::new(move |i| {
                        vec![
                            (i / distance) * proportion * (world_position.x - transx) + transx,
                            (i / distance) * proportion * (world_position.y - transy) + transy,
                            (1.0 - (i / distance)).max(0.0).min(1.0),
                            (i / distance).max(0.0).min(1.0),
                        ]
                    }),
                ),
            };

            sprite.color = Color::srgb(0.0, 1.0, 0.0);
            *pickupstate = PickupState::Move;

            commands.entity(entity).insert(animatiom);
        }
    }
}

fn myanimation_system(
    mut commands: Commands,
    query: Query<(
        &mut PickupState,
        &mut Transform,
        &mut MyAnimationCurve,
        &mut Sprite,
        Entity,
    )>,
    time: Res<Time>,
) {
    for (mut pickupstate, mut trans, mut myanimatiom, mut sprite, entity) in query {
        myanimatiom.active.update(time.delta_secs());
        let seek_time = myanimatiom.active.seek_time();

        let Some(samples) = myanimatiom.curve.sample(seek_time) else {
            *pickupstate = PickupState::Waiting;
            commands.entity(entity).remove::<MyAnimationCurve>();
            continue;
        };

        // Use your own mathematical results to customize the processing procedure.(computer translation)
        trans.translation.x = samples[0];
        trans.translation.y = samples[1];

        sprite.color = Color::srgb(samples[2], 0.0, samples[3]);

        if myanimatiom.active.is_finished() {
            *pickupstate = PickupState::Waiting;
            commands.entity(entity).remove::<MyAnimationCurve>();
        }
    }
}

fn keyboard_control(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut control: ResMut<AnimationControl>,
) {
    if keyboard_input.just_pressed(KeyCode::ArrowUp) {
        control.distance += 0.1;
    }
    if keyboard_input.just_pressed(KeyCode::ArrowDown) {
        control.distance -= 0.1;
    }
    if keyboard_input.just_pressed(KeyCode::ArrowLeft) {
        control.speed -= 5.0;
    }
    if keyboard_input.just_pressed(KeyCode::ArrowRight) {
        control.speed += 5.0;
    }
}

fn update_text(
    control: Res<AnimationControl>,
    mut query: Query<&mut Text, With<AnimationControlTextMark>>,
) {
    let mut text = query.single_mut().unwrap();

    text.0 = format!(
        "speed:{}\ndistance_proportion:{}",
        control.speed, control.distance
    );
}
