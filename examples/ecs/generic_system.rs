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
use system_param_in_associated_type::*;

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
                system::<ItemA>,
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

    commands.insert_resource(ResourceC { data: 3 });
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

// For a more advanced usage you may want have a group of system params to implement a trait.
// Note that this example is a little contrived in the interest of keeping things simple. The
// purpose here is to demontrate how to get the traits and lifetimes to work properly.
mod system_with_generic_system_param {
    use super::*;
    use bevy::ecs::system::SystemParam;

    struct DamagePlugin;
    impl Plugin for DamagePlugin {
        fn build(&self, app: &mut App) {
            app.add_systems(Startup, setup_damage).add_systems(
                Update,
                (
                    apply_damage::<PlayerDamageParams>,
                    apply_damage::<EnemyDamageParams>,
                ),
            );
        }
    }

    #[derive(Component)]
    struct Player;

    #[derive(Component)]
    struct Enemy;

    #[derive(Component)]
    struct Health(f32);

    #[derive(Resource)]
    struct EnemySettings {
        /// damage done by player to enemy
        take_damage: f32,
        /// damage done by enemy to player
        do_damage: f32,
    }

    pub trait GetDamage {
        fn apply_damage(&mut self) {}
    }

    #[derive(SystemParam)]
    struct PlayerDamageParams<'w, 's> {
        player: Query<'w, 's, &'static mut Health, With<Player>>,
        enemy_settings: Res<'w, EnemySettings>,
    }
    impl<'w, 's> GetDamage for PlayerDamageParams<'w, 's> {
        fn apply_damage(&mut self) {
            let mut player_health = self.player.single_mut();
            player_health.0 += self.enemy_settings.do_damage;
        }
    }

    #[derive(SystemParam)]
    struct EnemyDamageParams<'w, 's> {
        enemies: Query<'w, 's, &'static mut Health, With<Enemy>>,
        enemy_settings: Res<'w, EnemySettings>,
    }
    impl<'w, 's> GetDamage for EnemyDamageParams<'w, 's> {
        fn apply_damage(&mut self) {
            for mut enemy_health in self.enemies.iter_mut() {
                enemy_health.0 -= self.enemy_settings.take_damage;
            }
        }
    }

    // Note that the param passed into a system is `SystemParam::Item` and not just `SystemParam`.
    fn apply_damage<S: SystemParam>(mut param: S::Item<'_, '_>)
    where
        for<'w, 's> S::Item<'w, 's>: GetDamage,
    {
        param.apply_damage();
    }

    fn setup_damage(mut commands: Commands) {
        commands.insert_resource(EnemySettings {
            do_damage: 1.0,
            take_damage: 2.0,
        });
    }
}

// TODO: change this to use assets?
// You may want to be have the SystemParam be specified in an associated type.
mod system_param_in_associated_type {
    use super::*;
    use bevy::ecs::system::{lifetimeless::SRes, StaticSystemParam, SystemParam, SystemParamItem};

    #[derive(Resource)]
    pub struct ResourceC {
        pub data: u32,
    }

    pub trait MyTrait {
        type Param: SystemParam + 'static;

        fn do_something(&self, param: &mut SystemParamItem<Self::Param>) -> u32;
    }

    #[derive(Resource)]
    pub struct ItemA;
    impl MyTrait for ItemA {
        // specifies that data needed by do_something
        type Param = SRes<ResourceC>;

        fn do_something(&self, param: &mut SystemParamItem<Self::Param>) -> u32 {
            param.data
        }
    }

    pub fn system<S: MyTrait + Resource>(
        mut param: StaticSystemParam<<S as MyTrait>::Param>,
        asset: ResMut<S>,
    ) {
        asset.do_something(&mut param);
    }
}
