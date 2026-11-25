//! Text testbed
//!
//! You can switch scene by pressing the spacebar

mod helpers;

use argh::FromArgs;
use bevy::prelude::*;

use helpers::Next;

#[derive(FromArgs)]
/// text testbed
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
    app.add_plugins(DefaultPlugins)
        .add_systems(OnEnter(Scene::FontSize), font_size::scene.spawn())
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
    #[default]
    FontSize,
}

impl Scene {
    const ALL_ORDERED: &'static [Scene] = &[Scene::FontSize];
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

mod font_size {
    use super::Scene;
    use bevy::{color::palettes::css::*, prelude::*};

    pub fn scene() -> impl SceneList {
        let small_sizes = [
            0.5_f32, 1., 2., 3., 4., 5., 6., 7., 8., 9., 10., 11., 12., 14., 16.,
        ]
        .map(|size| {
            bsn! {
                Text({format!("{size}px The quick brown fox jumps over the lazy dog")})
                TextFont { font_size: FontSize::Px(size) }
            }
        });

        let relative_sizes = [
            FontSize::Rem(1.),
            FontSize::Rem(1.5),
            FontSize::Rem(2.),
            FontSize::Vw(1.),
            FontSize::Vw(1.5),
            FontSize::Vw(2.),
            FontSize::Vh(1.),
            FontSize::Vh(1.5),
            FontSize::Vh(2.),
            FontSize::VMin(1.),
            FontSize::VMin(1.5),
            FontSize::VMin(2.),
            FontSize::VMax(1.),
            FontSize::VMax(1.5),
            FontSize::VMax(2.),
        ]
        .map(|font_size| {
            bsn! {
                Text({format!("{font_size:?}")})
                TextFont { font_size: font_size }
            }
        });

        bsn_list! {
            Camera2d
            DespawnOnExit<Scene>(Scene::FontSize)
            --
            Node {
                width: percent(100),
                height: percent(100),
                padding: UiRect::all(px(10)),
                column_gap: px(20),
            }
            DespawnOnExit<Scene>(Scene::FontSize)
            Children [
                Node {
                    flex_direction: FlexDirection::Column,
                    row_gap: px(4),
                }
                Children [
                    {small_sizes.into_iter().collect::<Vec<_>>()}
                    --
                    Node {
                        flex_direction: FlexDirection::Column,
                        margin: UiRect::top(px(20)),
                    }
                    Children [
                        {relative_sizes.into_iter().collect::<Vec<_>>()}
                        --
                        Text("calc [ Vw(2) + 5 ]")
                        TextFont { font_size: FontSize::calc(|ctx| 0.02 * ctx.view.x + 5.) }
                        --
                        Text("calc [ Vw(2) - 5 ]")
                        TextFont { font_size: FontSize::calc(|ctx| 0.02 * ctx.view.x - 5.) }
                    ]
                ]
                --
                Node {
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Start,
                }
                Children [
                    Node {
                        align_items: AlignItems::Baseline,
                        column_gap: px(10),
                    }
                    Children [
                        Text("Ag")
                        TextFont { font_size: FontSize::Px(64.) }
                        TextColor(GOLD)
                        --
                        Text("Ag")
                        TextFont { font_size: FontSize::Px(128.) }
                        TextColor(TOMATO)
                    ]
                    --
                    Text("Ag")
                    TextFont { font_size: FontSize::Px(400.) }
                    TextColor(DEEP_SKY_BLUE)
                ]
            ]
        }
    }
}
