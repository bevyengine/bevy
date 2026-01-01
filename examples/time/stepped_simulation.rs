//! Shows how to use `Time<SteppedTimeTravel, u32>` to track a simulation,
//! and be able to move forward or backward.

use bevy::{
    color::palettes::css::*,
    input::common_conditions::input_just_pressed,
    prelude::*,
    time::{SteppedTimeTravel, SteppedVirtual},
};
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                apply_cost,
                update_virtual_time_info_text,
                manual_progress::<1>.run_if(input_just_pressed(KeyCode::ArrowRight)),
                manual_progress::<-1>.run_if(input_just_pressed(KeyCode::ArrowLeft)),
            ),
        )
        .run();
}

#[derive(Component)]
struct Steps;

#[derive(Component)]
struct Task;

#[derive(Resource)]
struct TasksCost(Vec<u32>);

#[derive(Resource)]
struct CurrentTask(usize);

const TASK_COLOR: Color = Color::srgb(0.21, 0.21, 0.21);
const TASK_COLOR_DONE: Color = Color::srgb(0.21, 0.8, 0.21);

fn setup(mut commands: Commands, mut time: ResMut<Time<SteppedVirtual, u32>>) {
    time.pause();

    commands.spawn(Camera2d);

    let mut rng = ChaCha8Rng::seed_from_u64(10223163112);

    let mut tasks_cost = vec![0];
    tasks_cost.extend((0..20).map(|_| rng.random_range(3..10)));

    commands.insert_resource(TasksCost(tasks_cost.clone()));
    commands.insert_resource(CurrentTask(0));

    commands
        .spawn((Node {
            display: Display::Flex,
            justify_content: JustifyContent::SpaceBetween,
            align_items: AlignItems::End,
            width: percent(100),
            position_type: PositionType::Absolute,
            top: percent(50),
            padding: UiRect::all(px(20)),
            ..default()
        },))
        .with_children(|spawner| {
            tasks_cost.iter().for_each(|cost| {
                spawner.spawn((
                    Node {
                        width: px(30),
                        height: px(25 * cost),
                        ..default()
                    },
                    BackgroundColor(TASK_COLOR),
                    Text::new(format!("{}", cost)),
                    Task,
                    TextLayout::new_with_justify(Justify::Center),
                ));
            })
        });

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
                Text::new("CONTROLS\nNext Task: Right\nPrevious Task: Left"),
                TextFont {
                    font_size,
                    ..default()
                },
                TextColor(Color::srgb(0.85, 0.85, 0.85)),
                TextLayout::new_with_justify(Justify::Center),
            ),
            (
                Text::default(),
                TextFont {
                    font_size,
                    ..default()
                },
                TextColor(GOLD.into()),
                TextLayout::new_with_justify(Justify::Right),
                Steps,
            ),
        ],
    ));
}

/// Update the `Virtual` time info text
fn update_virtual_time_info_text(
    time: Res<Time<SteppedTimeTravel, u32>>,
    mut query: Query<&mut Text, With<Steps>>,
) {
    if time.is_changed() {
        for mut text in &mut query {
            **text = format!("Steps: {:3}", time.elapsed());
        }
    }
}

fn apply_cost(
    mut time: ResMut<Time<SteppedTimeTravel, u32>>,
    mut current_task: ResMut<CurrentTask>,
    tasks_cost: Res<TasksCost>,
    mut task_boxes: Query<&mut BackgroundColor, With<Task>>,
    mut last_task: Local<usize>,
) {
    if !current_task.is_changed() {
        return;
    }

    if current_task.0 == tasks_cost.0.len() {
        for mut background in &mut task_boxes {
            background.0 = TASK_COLOR;
        }
        current_task.0 = 0;
        *last_task = 0;
        time.set_to(0);
        return;
    }

    for (i, mut background) in task_boxes.iter_mut().enumerate() {
        if i < current_task.0 + 1 {
            background.0 = TASK_COLOR_DONE;
        } else {
            background.0 = TASK_COLOR;
        }
    }

    if current_task.0 > *last_task {
        time.advance_by(tasks_cost.0[current_task.0]);
    } else {
        time.recede_by(tasks_cost.0[*last_task]);
    }
    *last_task = current_task.0
}

fn manual_progress<const DELTA: i8>(mut current_task: ResMut<CurrentTask>) {
    current_task.0 = (current_task.0 as i8 + DELTA).clamp(0, 21) as usize;
}
