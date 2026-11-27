//! Feathers testbed
//!
//! You can switch scenes by pressing the spacebar

mod helpers;

use argh::FromArgs;
use bevy::{
    feathers::{dark_theme::create_dark_theme, theme::UiTheme, FeathersPlugins},
    prelude::*,
};

use helpers::Next;

#[derive(FromArgs)]
/// feathers testbed
pub struct Args {
    #[argh(positional)]
    scene: Option<Scene>,
}

fn main() {
    #[cfg(not(target_arch = "wasm32"))]
    let args: Args = argh::from_env();
    #[cfg(target_arch = "wasm32")]
    let args: Args = Args::from_args(&[], &[]).unwrap();

    let mut app = App::new();
    app.add_plugins((
        DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                // Keep the window size fixed so screenshots are comparable between runs
                resolution: (1280, 720).into(),
                resizable: false,
                ..Default::default()
            }),
            ..Default::default()
        }),
        FeathersPlugins,
    ))
    .insert_resource(UiTheme(create_dark_theme()))
    .add_systems(OnEnter(Scene::Placeholder), placeholder::scene.spawn())
    .add_systems(Update, switch_scene);

    match args.scene {
        None => app.init_state::<Scene>(),
        Some(scene) => app.insert_state(scene),
    };

    #[cfg(feature = "bevy_ci_testing")]
    app.add_systems(Update, helpers::switch_scene_in_ci::<Scene>);

    app.run();
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, States, Default)]
enum Scene {
    /// Stand-in until the first widget scene is added, since a state needs at least one variant.
    #[default]
    Placeholder,
}

impl Scene {
    const ALL_ORDERED: &'static [Scene] = &[Scene::Placeholder];
}

impl std::str::FromStr for Scene {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut isit = Self::default();
        while s.to_lowercase() != format!("{isit:?}").to_lowercase() {
            isit = isit.next();
            if isit == Self::default() {
                return Err(format!("Invalid Scene name: {s}"));
            }
        }
        Ok(isit)
    }
}

impl Next for Scene {
    fn next(&self) -> Self {
        Scene::ALL_ORDERED[(Scene::ALL_ORDERED
            .iter()
            .position(|scene| scene == self)
            .unwrap()
            + 1)
            % Scene::ALL_ORDERED.len()]
    }
}

fn switch_scene(
    keyboard: Res<ButtonInput<KeyCode>>,
    scene: Res<State<Scene>>,
    mut next_scene: ResMut<NextState<Scene>>,
) {
    if keyboard.just_pressed(KeyCode::Space) {
        info!("Switching scene");
        next_scene.set(scene.get().next());
    }
}

mod placeholder {
    use bevy::{
        feathers::{theme::ThemeBackgroundColor, tokens},
        prelude::*,
    };

    pub fn scene() -> impl SceneList {
        bsn_list! {
            DespawnOnExit<super::Scene>({super::Scene::Placeholder})
            Camera2d
            --
            DespawnOnExit<super::Scene>({super::Scene::Placeholder})
            Node {
                width: percent(100),
                height: percent(100),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
            }
            ThemeBackgroundColor(tokens::WINDOW_BG)
            Children [
                ImageNode {
                    image: "branding/icon.svg"
                }
            ]
        }
    }
}
