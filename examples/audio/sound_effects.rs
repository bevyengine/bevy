//! Sound effects are short audio clips which are played in response to an event: a button click, a character taking damage, footsteps, etc.
//!
//! In this example, we'll showcase a simple abstraction that can be used to load and play randomized sound effects in response to an event.
//! The logic here is highly customizable: we encourage you to adapt it to meet your game's needs!

use bevy::{
    color::palettes::tailwind::{BLUE_600, BLUE_700, BLUE_800},
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
    map: HashMap<String, SfxList>,
}

impl SoundEffects {
    /// Plays a random sound effect matching the given name.
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
    fn play(&mut self, name: impl AsRef<str>, commands: &mut Commands) {
        let name = name.as_ref();
        if let Some(sfx_list) = self.map.get_mut(name) {
            let source = sfx_list.sample();
            commands.spawn(AudioBundle {
                source,
                // We want the sound effect to play once and then despawn.
                settings: PlaybackSettings::DESPAWN,
                ..Default::default()
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
            asset_server.load("sounds/button_press_1.mp3"),
            asset_server.load("sounds/button_press_2.mp3"),
            asset_server.load("sounds/button_press_3.mp3"),
        ];
        map.insert("button_press".to_string(), SfxList::new(button_press_sfxs));

        Self { map }
    }
}

struct SfxList {
    sfxs: Vec<Handle<AudioSource>>,
    last_played: Option<usize>,
}

impl SfxList {
    /// Generates a new sound effect list.
    fn new(sfxs: Vec<Handle<AudioSource>>) -> Self {
        Self {
            sfxs,
            last_played: None,
        }
    }

    /// Plays a random sound effect from the list.
    ///
    /// The last-played sound-effect will not be chosen again,
    /// unless the list contains only one sound effect.
    ///
    /// # Warning
    ///
    /// This will return a default handle if the list is empty.
    fn sample(&mut self) -> Handle<AudioSource> {
        if self.sfxs.is_empty() {
            return Handle::default();
        } else if self.sfxs.len() == 1 {
            return self.sfxs[0].clone_weak();
        }

        // If you need precise control over the randomization order of your sound effects,
        // store the RNG as a resource and modify these functions to take it as an argument.
        let rng = &mut rand::thread_rng();

        let index = rng.sample(Uniform::from(0..self.sfxs.len()));

        // Use a simple rejection sampling strategy
        // to avoid playing the same sound effect twice in a row.
        if Some(index) == self.last_played {
            self.sample()
        } else {
            self.last_played = Some(index);
            self.sfxs[index].clone_weak()
        }
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
                ..Default::default()
            });
        });
}

fn play_sound_effect_when_button_pressed(
    button_query: Query<&Interaction, Changed<Interaction>>,
    mut sound_effects: ResMut<SoundEffects>,
    mut commands: Commands,
) {
    for interaction in button_query.iter() {
        if *interaction == Interaction::Pressed {
            sound_effects.play("button_press", &mut commands);
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
