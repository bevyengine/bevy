//! Sound effects are short audio clips which are played in response to an event: a button click, a character taking damage, footsteps, etc.
//!
//! In this example, we'll showcase a simple abstraction that can be used to load and play randomized sound effects in response to an event.
//! The logic here is highly customizable: we encourage you to adapt it to meet your game's needs!

use bevy::{
    color::palettes::tailwind::{BLUE_600, BLUE_700, BLUE_800},
    ecs::world::Command,
    prelude::*,
    utils::HashMap,
};
use rand::{distributions::Uniform, Rng};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        // This must be below the `DefaultPlugins` plugin, as it depends on the `AssetPlugin`.
        .init_resource::<SoundEffects>()
        .add_systems(Startup, spawn_button)
        .add_systems(
            Update,
            (
                play_sound_effect_when_button_pressed,
                change_button_color_based_on_interaction,
            ),
        )
        .run();
}

#[derive(Resource)]
struct SoundEffects {
    map: HashMap<String, Vec<Handle<AudioSource>>>,
}

impl SoundEffects {
    /// Plays a random sound effect matching the given name.
    ///
    /// When defining the settings for this method, you almost always want to use [`PlaybackMode::Despawn`](bevy::audio::PlaybackMode).
    /// Every time a sound effect is played, a new entity is generated. Once the sound effect is complete,
    /// the entity should be cleaned up, rather than looping or sitting around uselessly.
    ///
    /// This method accepts any type which implements `AsRef<str>`.
    /// This allows you to pass in `&str`, `String`, or a custom type that can be converted to a string.
    ///
    /// These custom types can be useful for defining enums that represent specific sound effects.
    /// Generally speaking, enum values should be used to represent one-off or special-cased sound effects,
    /// while string keys are a better fit for sound effects corresponding to objects loaded from a data file.
    ///
    /// # Example
    ///
    /// ```
    /// enum GameSfx {
    ///     SplashScreenJingle,
    ///     Victory,
    ///     Defeat,
    /// }
    ///
    /// impl AsRef<str> for GameSfx {
    ///     fn as_ref(&self) -> &str {
    ///         match self {
    ///             GameSfx::SplashScreenJingle => "splash_screen_jingle",
    ///             GameSfx::Victory => "victory",
    ///             GameSfx::Defeat => "defeat",
    ///         }
    ///     }
    /// }
    /// ```
    fn play(&mut self, name: impl AsRef<str>, world: &mut World, settings: PlaybackSettings) {
        let name = name.as_ref();
        if let Some(sfx_list) = self.map.get_mut(name) {
            // If you need precise control over the randomization order of your sound effects,
            // store the RNG as a resource and modify these functions to take it as an argument.
            let rng = &mut rand::thread_rng();

            let index = rng.sample(Uniform::from(0..sfx_list.len()));
            // We don't need a (slightly) more expensive strong handle here (which are used to keep assets loaded in memory)
            // because a copy is always stored in the SoundEffects resource.
            let source = sfx_list[index].clone_weak();

            world.spawn(AudioBundle {
                source,
                // We want the sound effect to play once and then despawn.
                settings,
            });
        } else {
            warn!("Sound effect not found: {}", name);
        }
    }
}

impl FromWorld for SoundEffects {
    // `FromWorld` is the simplest way to load assets into a resource,
    // but you likely want to integrate this into your own asset loading strategy.
    fn from_world(world: &mut World) -> Self {
        let asset_server = world.get_resource::<AssetServer>().unwrap();
        let mut map = HashMap::default();

        // Load sound effects here.
        // Using string parsing to strip numbered suffixes + load_folder is a good way to load many sound effects at once.
        let button_press_sfxs = vec![
            asset_server.load("sounds/button_press_1.ogg"),
            asset_server.load("sounds/button_press_2.ogg"),
            asset_server.load("sounds/button_press_3.ogg"),
        ];
        map.insert("button_press".to_string(), button_press_sfxs);

        Self { map }
    }
}

/// A custom command used to play sound effects.
struct PlaySoundEffect {
    name: String,
    settings: PlaybackSettings,
}

impl Command for PlaySoundEffect {
    fn apply(self, world: &mut World) {
        // Access both the world and the resource we need from it using resource_scope
        // which temporarily removes the SoundEffects resource from the world
        world.resource_scope(|world, mut sound_effects: Mut<SoundEffects>| {
            sound_effects.play(self.name, world, self.settings);
        });
    }
}

/// An "extension trait" used to make it convenient to play sound effects via [`Commands`].
///
/// This technique allows us to implement methods for types that we don't own,
/// which can be used as long as the trait is in scope.
trait SfxCommands {
    fn play_sound_effect_with_settings(
        &mut self,
        name: impl AsRef<str>,
        settings: PlaybackSettings,
    );

    fn play_sound_effect(&mut self, name: impl AsRef<str>) {
        // This default method implementation saves work for types implementing this trait;
        // if not overwritten, the trait's default method will be used here, forwarding to the
        // more customizable method
        self.play_sound_effect_with_settings(name, PlaybackSettings::DESPAWN);
    }
}

impl<'w, 's> SfxCommands for Commands<'w, 's> {
    // By accepting an `AsRef<str>` here, we can be flexible about what we want to accept:
    // &str literals are better for prototyping and data-driven sound effects,
    // but enums are nicer for special-cased effects
    fn play_sound_effect_with_settings(
        &mut self,
        name: impl AsRef<str>,
        settings: PlaybackSettings,
    ) {
        let name = name.as_ref().to_string();
        self.add(PlaySoundEffect { name, settings });
    }
}

fn spawn_button(mut commands: Commands) {
    commands.spawn(Camera2dBundle::default());

    commands
        .spawn(ButtonBundle {
            style: Style {
                width: Val::Px(300.0),
                height: Val::Px(100.0),
                align_self: AlignSelf::Center,
                justify_self: JustifySelf::Center,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            background_color: BLUE_600.into(),
            border_radius: BorderRadius::all(Val::Px(10.)),
            ..default()
        })
        .with_children(|child_builder| {
            child_builder.spawn(TextBundle {
                text: Text::from_section("Generate sound effect!", TextStyle::default()),
                ..default()
            });
        });
}

fn play_sound_effect_when_button_pressed(
    button_query: Query<&Interaction, Changed<Interaction>>,
    mut commands: Commands,
) {
    for interaction in button_query.iter() {
        if *interaction == Interaction::Pressed {
            commands.play_sound_effect("button_press");
        }
    }
}

fn change_button_color_based_on_interaction(
    mut button_query: Query<(&Interaction, &mut BackgroundColor), Changed<Interaction>>,
) {
    for (interaction, mut color) in button_query.iter_mut() {
        *color = match interaction {
            Interaction::None => BLUE_600.into(),
            Interaction::Hovered => BLUE_700.into(),
            Interaction::Pressed => BLUE_800.into(),
        }
    }
}
