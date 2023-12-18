//! Generic types allow us to reuse logic across many related systems,
//! allowing us to specialize our function's behavior based on which type (or types) are passed in.
//!
//! This is commonly useful for working on related components or resources,
//! where we want to have unique types for querying purposes but want them all to work the same way.
//! This is particularly powerful when combined with user-defined traits to add more functionality to these related types.
//! Remember to insert a specialized copy of the system into the schedule for each type that you want to operate on!
//!
//! For more advice on working with generic types in Rust, check out <https://doc.rust-lang.org/book/ch10-01-syntax.html>
//! or <https://doc.rust-lang.org/rust-by-example/generics.html>

use bevy::prelude::*;
use system_with_generic_system_param::*;

#[derive(Debug, Default, Clone, Copy, Eq, PartialEq, Hash, States)]
enum AppState {
    #[default]
    MainMenu,
    InGame,
}

#[derive(Component)]
struct TextToPrint(String);

#[derive(Component, Deref, DerefMut)]
struct PrinterTick(Timer);

#[derive(Component)]
struct MenuClose;

#[derive(Component)]
struct LevelUnload;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .init_state::<AppState>()
        .add_systems(Startup, setup_system)
        .add_systems(
            Update,
            (
                print_text_system,
                transition_to_in_game_system.run_if(in_state(AppState::MainMenu)),
                system_with_generic::<ResMut<ResourceA>>,
                system_with_generic::<ResMut<ResourceB>>,
            ),
        )
        // Cleanup systems.
        // Pass in the types your system should operate on using the ::<T> (turbofish) syntax
        .add_systems(OnExit(AppState::MainMenu), cleanup_system::<MenuClose>)
        .add_systems(OnExit(AppState::InGame), cleanup_system::<LevelUnload>)
        .run();
}

fn setup_system(mut commands: Commands) {
    commands.spawn((
        PrinterTick(Timer::from_seconds(1.0, TimerMode::Repeating)),
        TextToPrint("I will print until you press space.".to_string()),
        MenuClose,
    ));

    commands.spawn((
        PrinterTick(Timer::from_seconds(1.0, TimerMode::Repeating)),
        TextToPrint("I will always print".to_string()),
        LevelUnload,
    ));
}

fn print_text_system(time: Res<Time>, mut query: Query<(&mut PrinterTick, &TextToPrint)>) {
    for (mut timer, text) in &mut query {
        if timer.tick(time.delta()).just_finished() {
            info!("{}", text.0);
        }
    }
}

fn transition_to_in_game_system(
    mut next_state: ResMut<NextState<AppState>>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
) {
    if keyboard_input.pressed(KeyCode::Space) {
        next_state.set(AppState::InGame);
    }
}

// Type arguments on functions come after the function name, but before ordinary arguments.
// Here, the `Component` trait is a trait bound on T, our generic type
fn cleanup_system<T: Component>(mut commands: Commands, query: Query<Entity, With<T>>) {
    for e in &query {
        commands.entity(e).despawn_recursive();
    }
}

mod system_with_generic_system_param {
    use super::*;
    use bevy::ecs::system::{SystemParam, SystemParamItem};

    pub trait MyTrait {
        fn calculate_something(&mut self) {}
    }

    #[derive(Resource)]
    pub struct ResourceA(pub usize);

    impl MyTrait for ResMut<'_, ResourceA> {
        fn calculate_something(&mut self) {
            // dbg!(self.0);
            self.0 = 5;
        }
    }

    #[derive(Resource)]
    pub struct ResourceB(pub usize);
    impl MyTrait for ResMut<'_, ResourceB> {
        fn calculate_something(&mut self) {
            // dbg!(self.0);
            self.0 = 10;
        }
    }

    pub fn system_with_generic<S: SystemParam>(mut param: SystemParamItem<S>)
    where
        for<'w, 's> S::Item<'w, 's>: MyTrait,
    {
        param.calculate_something();
    }
}

mod system_param_in_associated_type {
    use super::*;
    use bevy::ecs::system::{lifetimeless::SRes, StaticSystemParam, SystemParam, SystemParamItem};

    #[derive(Resource)]
    struct ResourceA;

    pub trait MyTrait {
        type Param: SystemParam + 'static;

        fn do_something(&self, param: &mut SystemParamItem<Self::Param>) -> u32;
    }

    struct ItemA;
    impl MyTrait for ItemA {
        type Param = SRes<ResourceA>;

        fn do_something(&self, param: &mut SystemParamItem<Self::Param>) -> u32 {
            todo!()
        }
    }

    fn system<S: MyTrait>(param: StaticSystemParam<<S as MyTrait>::Param>) {}
}
