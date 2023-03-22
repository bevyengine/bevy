use crate::Input;
use bevy_ecs::system::Res;
use std::hash::Hash;

/// Stateful run condition that can be toggled via a input press using [`Input::just_pressed`].
///
/// ```rust,no_run
/// use bevy::prelude::*;
/// use bevy::input::common_conditions::input_toggle_active;
///
/// fn main() {
///     App::new()
///         .add_plugins(DefaultPlugins)
///         .add_systems(Update, pause_menu.run_if(input_toggle_active(false, KeyCode::Escape)))
///         .run();
/// }
///
/// fn pause_menu() {
///     println!("in pause menu");
/// }
/// ```
///
/// If you want other systems to be able to access whether the toggled state is active,
/// you should use a custom resource or a state for that:
/// ```rust,no_run
/// use bevy::prelude::*;
/// use bevy::input::common_conditions::input_toggle_active;
///
/// #[derive(Resource, Default)]
/// struct Paused(bool);
///
/// fn main() {
///     App::new()
///         .add_plugins(DefaultPlugins)
///         .init_resource::<Paused>()
///         .add_systems(Update, pause_menu.run_if(|paused: Res<Paused>| paused.0))
///         .run();
/// }
///
/// fn update_pause_state(mut paused: ResMut<Paused>, input: Input<KeyCode>) {
///   if input.just_pressed(KeyCode::Escape) {
///     paused.0 = !paused.0;
///   }
/// }
///
/// fn pause_menu() {
///     println!("in pause menu");
/// }
///
/// ```
pub fn input_toggle_active<T>(default: bool, input: T) -> impl FnMut(Res<Input<T>>) -> bool + Clone
where
    T: Copy + Eq + Hash + Send + Sync + 'static,
{
    let mut active = default;
    move |inputs: Res<Input<T>>| {
        active ^= inputs.just_pressed(input);
        active
    }
}

/// Run condition that is active if [`Input::pressed`] is true for the given input.
pub fn input_pressed<T>(input: T) -> impl FnMut(Res<Input<T>>) -> bool + Clone
where
    T: Copy + Eq + Hash + Send + Sync + 'static,
{
    move |inputs: Res<Input<T>>| inputs.pressed(input)
}

/// Run condition that is active if [`Input::just_pressed`] is true for the given input.
///
/// ```rust,no_run
/// use bevy::prelude::*;
/// use bevy::input::common_conditions::input_just_pressed;
/// fn main() {
///     App::new()
///         .add_plugins(DefaultPlugins)
///         .add_systems(Update, jump.run_if(input_just_pressed(KeyCode::Space)))
///         .run();
/// }
///
/// # fn jump() {}
/// ```
pub fn input_just_pressed<T>(input: T) -> impl FnMut(Res<Input<T>>) -> bool + Clone
where
    T: Copy + Eq + Hash + Send + Sync + 'static,
{
    move |inputs: Res<Input<T>>| inputs.just_pressed(input)
}

/// Run condition that is active if [`Input::just_released`] is true for the given input.
pub fn input_just_released<T>(input: T) -> impl FnMut(Res<Input<T>>) -> bool + Clone
where
    T: Copy + Eq + Hash + Send + Sync + 'static,
{
    move |inputs: Res<Input<T>>| inputs.just_released(input)
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::prelude::{IntoSystemConfigs, KeyCode, Schedule};

    fn test_system() {}

    // Ensure distributive_run_if compiles with the common conditions.
    #[test]
    fn distributive_run_if_compiles() {
        Schedule::default().add_systems(
            (test_system, test_system)
                .distributive_run_if(input_toggle_active(false, KeyCode::Escape))
                .distributive_run_if(input_pressed(KeyCode::Escape))
                .distributive_run_if(input_just_pressed(KeyCode::Escape))
                .distributive_run_if(input_just_released(KeyCode::Escape)),
        );
    }
}
