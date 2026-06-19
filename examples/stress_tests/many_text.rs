//! UI text benchmark

use argh::FromArgs;
use bevy::{
    diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin},
    ecs::component::Mutable,
    prelude::*,
    window::{PresentMode, WindowResolution},
    winit::WinitSettings,
};

const LOREM_TEXT_1: &str = "Lorem ipsum dolor sit amet, consectetur adipiscing elit, sed do \
eiusmod tempor incididunt ut labore et dolore magna aliqua. Ut enim ad minim veniam, quis \
nostrud exercitation ullamco laboris nisi ut aliquip ex ea commodo consequat.";
const _LOREM_TEXT_2: &str = "Duis aute irure dolor in reprehenderit in voluptate velit esse cillum dolore eu fugiat nulla pariatur.
Excepteur sint occaecat cupidatat non proident, sunt in culpa qui officia deserunt mollit anim id est laborum.";

#[derive(FromArgs, Resource)]
/// `many_text` UI text stress test
struct Args {
    /// whether to set the text changed each frame
    #[argh(switch)]
    set_text_changed: bool,

    /// whether to set the font changed each frame
    #[argh(switch)]
    set_font_changed: bool,

    /// at the start of each frame despawn any existing UI nodes and spawn a new UI tree
    #[argh(switch)]
    respawn: bool,
}

fn main() {
    let mut app = App::new();

    app.add_plugins((
        DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                present_mode: PresentMode::AutoNoVsync,
                resolution: WindowResolution::new(1920, 1080).with_scale_factor_override(1.0),
                ..default()
            }),
            ..default()
        }),
        FrameTimeDiagnosticsPlugin::default(),
        LogDiagnosticsPlugin::default(),
    ))
    .insert_resource(WinitSettings::continuous())
    .add_systems(Startup, (setup_camera, setup_text));

    // `from_env` panics on the web
    #[cfg(not(target_arch = "wasm32"))]
    let args: Args = argh::from_env();
    #[cfg(target_arch = "wasm32")]
    let args = Args::from_args(&[], &[]).unwrap();

    if args.set_text_changed {
        app.add_systems(Update, set_changed::<Text>);
    }

    if args.set_font_changed {
        app.add_systems(Update, set_changed::<TextFont>);
    }

    if args.respawn {
        app.add_systems(Update, (despawn_text, setup_text).chain());
    }

    app.run();
}

#[derive(Component)]
struct ManyTextRoot;

fn setup_camera(mut commands: Commands) {
    warn!(include_str!("warning_string.txt"));
    commands.spawn(Camera2d);
}

fn setup_text(mut commands: Commands, asset_server: Res<AssetServer>, _args: Res<Args>) {
    commands
        .spawn((
            Node {
                display: Display::Flex,
                flex_direction: FlexDirection::Row,
                width: percent(100),
                height: percent(100),
                overflow: Overflow::clip(),
                ..default()
            },
            ManyTextRoot,
        ))
        .with_children(|parent| {
            for font_path in [
                "fonts/EBGaramond12-Regular.otf",
                "fonts/FiraMono-Medium.ttf",
                "fonts/FiraSans-Bold.ttf",
                "fonts/MonaSans-VariableFont.ttf",
            ] {
                let font = asset_server.load(font_path);
                let text_font = TextFont {
                    font: font.into(),
                    font_size: px(10).into(),
                    ..default()
                };
                parent
                    .spawn(Node {
                        display: Display::Flex,
                        flex_direction: FlexDirection::Column,
                        flex_grow: 1.0,
                        overflow: Overflow::clip(),
                        ..default()
                    })
                    .with_children(|parent| {
                        parent.spawn((Text(format!("{font_path}")), text_font.clone()));
                        for justify in [
                            Justify::Left,
                            Justify::Center,
                            Justify::Right,
                            Justify::Justified,
                        ] {
                            for linebreak in [LineBreak::AnyCharacter, LineBreak::WordBoundary] {
                                parent.spawn((
                                    Text(format!(
                                        "Justify::{justify:?}\n LineBreak::{linebreak:?}"
                                    )),
                                    text_font.clone(),
                                    TextColor::from(bevy::color::palettes::css::YELLOW),
                                ));
                                let layout = TextLayout { justify, linebreak };
                                parent.spawn((
                                    Text::new(LOREM_TEXT_1),
                                    layout,
                                    text_font.clone().with_font_size(11.),
                                    TextColor::from(bevy::color::palettes::css::NAVY),
                                ));
                                parent.spawn((
                                    Text::new(LOREM_TEXT_1),
                                    layout,
                                    text_font.clone().with_font_size(12.),
                                    TextColor::from(bevy::color::palettes::css::PALE_GREEN),
                                ));
                            }
                        }
                    });
            }
        });
}

fn set_changed<C: Component<Mutability = Mutable>>(mut component_query: Query<&mut C>) {
    for mut component in &mut component_query {
        component.set_changed();
    }
}

fn despawn_text(mut commands: Commands, root_node: Single<Entity, With<ManyTextRoot>>) {
    commands.entity(*root_node).despawn();
}
