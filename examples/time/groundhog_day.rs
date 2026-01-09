//! Shows how `Time<TimeTravel>` can be used to go back in time.

use std::time::Duration;

use bevy::{
    color::palettes::css::*,
    prelude::*,
    time::{common_conditions::on_real_timer, TimeTravel},
};
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;

#[derive(Resource, Deref, DerefMut)]
struct SeededRng(ChaCha8Rng);
impl Default for SeededRng {
    fn default() -> Self {
        Self(ChaCha8Rng::seed_from_u64(10223163112))
    }
}

#[derive(Resource)]
struct TimeOfReset(Duration);

const START: Duration = Duration::from_hours(6);
const END: Duration = Duration::from_mins(START.as_secs() / 60 + 5);

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .init_resource::<SeededRng>()
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                progress,
                (
                    update_virtual_time_info_text,
                    update_real_time_info_text,
                    reset,
                )
                    .run_if(on_real_timer(Duration::from_millis(250))),
            ),
        )
        .run();
}

/// `Real` time related marker
#[derive(Component)]
struct RealTime;

/// `Virtual` time related marker
#[derive(Component)]
struct VirtualTime;

fn setup(
    mut commands: Commands,
    mut time: ResMut<Time<Virtual>>,
    mut time_travel: ResMut<Time<TimeTravel>>,
    mut rng: ResMut<SeededRng>,
) {
    time.set_relative_speed(25.);

    time_travel.advance_to(START);

    commands.insert_resource(TimeOfReset(rng.random_range(START..END)));

    commands.spawn(Camera2d);

    // info UI
    let font_size = 33.;

    commands.spawn((
        Node {
            display: Display::Flex,
            justify_content: JustifyContent::SpaceBetween,
            width: percent(100),
            position_type: PositionType::Absolute,
            top: px(0),
            padding: UiRect::all(px(20)),
            ..default()
        },
        children![
            (
                Text::default(),
                TextFont {
                    font_size,
                    ..default()
                },
                RealTime,
            ),
            (
                Text::default(),
                TextFont {
                    font_size,
                    ..default()
                },
                TextColor(GOLD.into()),
                TextLayout::new_with_justify(Justify::Right),
                VirtualTime,
            ),
        ],
    ));
}

/// Update the `Real` time info text
fn update_real_time_info_text(time: Res<Time<Real>>, mut query: Query<&mut Text, With<RealTime>>) {
    for mut text in &mut query {
        **text = format!("REAL TIME\nElapsed: {:.1?}", time.elapsed());
    }
}

/// Update the `Virtual` time info text
fn update_virtual_time_info_text(
    time: Res<Time<TimeTravel>>,
    end_of_day: Res<TimeOfReset>,
    mut query: Query<&mut Text, With<VirtualTime>>,
) {
    for mut text in &mut query {
        **text = format!(
            "LOOPING TIME\nTime: {}\nLoop At: {}",
            pretty_duration(time.elapsed()),
            pretty_duration(end_of_day.0),
        );
    }
}

fn pretty_duration(duration: Duration) -> String {
    let secs = duration.as_secs();
    format!(
        "{:2}h{:2}m{:2}s",
        (secs / 3600) % 24,
        (secs / 60) % 60,
        secs % 60
    )
}

fn reset(
    mut time: ResMut<Time<TimeTravel>>,
    mut end_of_day: ResMut<TimeOfReset>,
    mut rng: ResMut<SeededRng>,
) {
    if time.elapsed() > end_of_day.0 {
        time.set_to(START);
        end_of_day.0 = rng.random_range(START..END);
    }
}

fn progress(time: Res<Time<TimeTravel>>, end_of_day: Res<TimeOfReset>, mut gizmos: Gizmos) {
    let total = (END - START).as_secs();
    let target = (end_of_day.0 - START).as_secs();
    let current = (time.elapsed() - START).as_secs();
    // println!("{} -> {} / {}", current, target, total);

    let width = 500.0;
    let position = |value: u64| (value as f32 / total as f32) * width - width / 2.0;

    gizmos.rect_2d(Isometry2d::default(), Vec2::new(500.0, 5.0), BLUE);
    gizmos.line_2d(
        Vec2::new(position(0), 0.0),
        Vec2::new(position(current), 0.0),
        GREEN,
    );
    gizmos.line_2d(
        Vec2::new(position(target), -10.0),
        Vec2::new(position(target), 10.0),
        RED,
    );
}
