use bevy::prelude::*;

struct Person;
struct Name(String);
struct GreetTimer(Timer);
struct Frame(u32);
struct Paused(bool);

impl FlagResource for Paused {
    fn flag(&self) -> bool {
        self.0
    }
}

fn fixed_timestep(time: Res<Time>, mut timer: ResMut<GreetTimer>, mut frame: ResMut<Frame>) {
    timer.0.tick(time.delta_seconds);
    if timer.0.finished {
        frame.0 += 1;
    }
}

fn greet_people(_paused: FlagRes<Paused>, frame: ChangedRes<Frame>, _person: &Person, name: &Name) {
    println!("hello {} on frame {}!", name.0, frame.0);
}

fn pause(
    mut paused: ResMut<Paused>,
    _frame: ChangedRes<Frame>,
    keyboard_input: Res<Input<KeyCode>>,
) {
    if keyboard_input.pressed(KeyCode::Left) {
        paused.0 = !paused.0;
        println!("Pause: {}", paused.0)
    }
}

fn add_people(mut commands: Commands) {
    commands
        .spawn((Person, Name("Elaina Proctor".to_string())))
        .spawn((Person, Name("Renzo Hume".to_string())))
        .spawn((Person, Name("Zayna Nieves".to_string())));
}

fn main() {
    App::build()
        .add_resource(GreetTimer(Timer::from_seconds(2.0, true)))
        .add_resource(Frame(0))
        .add_resource(Paused(false))
        .add_startup_system(add_people.system())
        .add_system(fixed_timestep.system())
        .add_system(pause.system())
        .add_system(greet_people.system())
        .add_default_plugins()
        .run();
}
