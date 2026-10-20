//! UI text benchmark

use argh::FromArgs;
use bevy::{
    diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin},
    ecs::component::Mutable,
    prelude::*,
    text::FontAtlasSet,
    window::{PresentMode, WindowResolution},
    winit::WinitSettings,
};

const LOREM_TEXT_1: &str = "Lorem ipsum dolor sit amet, consectetur adipiscing elit, sed do \
eiusmod tempor incididunt ut labore et dolore magna aliqua. Ut enim ad minim veniam, quis \
nostrud exercitation ullamco laboris nisi ut aliquip ex ea commodo consequat.";
const LOREM_TEXT_2: &str = "Duis aute irure dolor in reprehenderit in voluptate velit esse cillum dolore eu fugiat nulla pariatur. \
Excepteur sint occaecat cupidatat non proident, sunt in culpa qui officia deserunt mollit anim id est laborum.";

#[derive(Component)]
struct Lorem(bool);

#[derive(Component)]
struct NumberSpan;

#[derive(FromArgs, Resource)]
/// `many_text` UI text stress test
struct Args {
    /// whether to set the font changed each frame
    #[argh(switch)]
    set_font_changed: bool,

    /// at the start of each frame despawn any existing UI nodes and spawn a new UI tree
    #[argh(switch)]
    respawn: bool,

    /// at the start of each frame clear all font atlases
    #[argh(switch)]
    clear_font_atlases: bool,

    /// update the text each frame
    #[argh(switch)]
    animate: bool,
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

    if args.set_font_changed {
        app.add_systems(Update, set_changed::<TextFont>);
    }

    if args.respawn {
        app.add_systems(Update, (despawn_layout, setup_text).chain());
    }

    if args.clear_font_atlases {
        app.add_systems(Update, clear_all_font_atlases);
    }

    if args.animate {
        app.add_systems(Update, update_lorem_text);
        app.add_systems(Update, update_number_text);
    }

    app.run();
}

#[derive(Component)]
struct ManyTextRoot;

fn setup_camera(mut commands: Commands) {
    warn!(include_str!("warning_string.txt"));
    commands.spawn(Camera2d);
}

fn setup_text(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands
        .spawn((
            Node {
                display: Display::Grid,
                grid_template_columns: RepeatedGridTrack::flex(4, 1.0),
                width: percent(100),
                height: percent(100),
                column_gap: px(2.),
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
                    .spawn((
                        Node {
                            flex_direction: FlexDirection::Column,
                            border: px(1.).all(),
                            padding: px(1.).all(),
                            row_gap: px(2.),
                            ..default()
                        },
                        BorderColor::all(Color::WHITE),
                    ))
                    .with_children(|parent| {
                        parent.spawn((Text(font_path.to_string()), text_font.clone()));
                        for justify in [
                            Justify::Left,
                            Justify::Center,
                            Justify::Right,
                            Justify::Justified,
                        ] {
                            for linebreak in [LineBreak::AnyCharacter, LineBreak::WordBoundary] {
                                parent.spawn((
                                    Text(format!("Justify::{justify:?}, LineBreak::{linebreak:?}")),
                                    text_font.clone(),
                                    TextColor::from(bevy::color::palettes::css::YELLOW),
                                ));
                                let layout = TextLayout { justify, linebreak };
                                parent.spawn((
                                    Text::new(LOREM_TEXT_1),
                                    Lorem(false),
                                    layout,
                                    text_font.clone().with_font_size(10.),
                                    TextColor::from(bevy::color::palettes::css::NAVY),
                                ));
                                parent.spawn((
                                    Text::new(LOREM_TEXT_2),
                                    Lorem(true),
                                    layout,
                                    text_font.clone().with_font_size(11.),
                                    TextColor::from(bevy::color::palettes::css::PALE_GREEN),
                                ));
                            }
                        }

                        parent
                            .spawn((
                                Text::new(LOREM_TEXT_1),
                                Lorem(false),
                                text_font.clone().with_font_size(12.),
                                TextColor::from(bevy::color::palettes::css::MISTY_ROSE),
                            ))
                            .with_child((TextSpan::new(" "), text_font.clone().with_font_size(13.)))
                            .with_child((
                                TextSpan::new(LOREM_TEXT_2),
                                Lorem(true),
                                text_font.clone().with_font_size(13.),
                                TextColor::from(bevy::color::palettes::css::MAROON),
                            ));

                        parent
                            .spawn((
                                Text::default(),
                                TextLayout::linebreak(LineBreak::AnyCharacter),
                            ))
                            .with_children(|parent| {
                                for i in (0..10).cycle().take(100) {
                                    parent.spawn((
                                        TextSpan(i.to_string()),
                                        text_font.clone().with_font_size((6 + i) as f32),
                                        NumberSpan,
                                    ));
                                }
                            });
                    });
            }
        });
}

fn set_changed<C: Component<Mutability = Mutable>>(mut component_query: Query<&mut C>) {
    for mut component in &mut component_query {
        component.set_changed();
    }
}

fn despawn_layout(mut commands: Commands, root_node: Single<Entity, With<ManyTextRoot>>) {
    commands.entity(*root_node).despawn();
}

fn clear_all_font_atlases(mut font_atlases: ResMut<FontAtlasSet>) {
    font_atlases.clear();
}

fn update_lorem_text(mut lorem_text_query: Query<(&mut Text, &mut Lorem)>) {
    for (mut text, mut lorem) in &mut lorem_text_query {
        if lorem.0 {
            text.0.clear();
            text.0.push_str(LOREM_TEXT_1);
        } else {
            text.0.clear();
            text.0.push_str(LOREM_TEXT_2);
        }

        lorem.0 = !lorem.0;
    }
}

fn update_number_text(mut n: Local<u32>, mut number_spans: Query<&mut TextSpan, With<NumberSpan>>) {
    for mut text in &mut number_spans {
        text.0 = format!("{}", (text.0.parse::<u32>().unwrap() + *n) % 10);
    }

    *n = (*n + 1) % 10;
}
