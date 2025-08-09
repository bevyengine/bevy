//! Renders a lot of `Text2d`s

use std::ops::RangeInclusive;

use bevy::{
    camera::visibility::NoFrustumCulling,
    diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin},
    prelude::*,
    text::FontAtlasSets,
    window::{PresentMode, WindowResolution},
};

use argh::FromArgs;
use rand::{
    seq::{IndexedRandom, IteratorRandom},
    Rng, SeedableRng,
};
use rand_chacha::ChaCha8Rng;

const CAMERA_SPEED: f32 = 1000.0;

// Some code points for valid glyphs in `FiraSans-Bold.ttf`
const CODE_POINT_RANGES: [RangeInclusive<u32>; 5] = [
    0x20..=0x7e,
    0xa0..=0x17e,
    0x180..=0x2b2,
    0x3f0..=0x479,
    0x48a..=0x52f,
];

#[derive(FromArgs, Resource)]
/// `many_text2d` stress test
struct Args {
    /// whether to use many different glyphs to increase the amount of font atlas textures used.
    #[argh(switch)]
    many_glyphs: bool,

    /// whether to use many different font sizes to increase the amount of font atlas textures used.
    #[argh(switch)]
    many_font_sizes: bool,

    /// whether to force the text to recompute every frame by triggering change detection.
    #[argh(switch)]
    recompute: bool,

    /// whether to disable all frustum culling.
    #[argh(switch)]
    no_frustum_culling: bool,

    /// whether the text should use `Justify::Center`.
    #[argh(switch)]
    center: bool,
}

#[derive(Resource)]
struct FontHandle(Handle<Font>);
impl FromWorld for FontHandle {
    fn from_world(world: &mut World) -> Self {
        Self(world.load_asset("fonts/FiraSans-Bold.ttf"))
    }
}

fn main() {
    // `from_env` panics on the web
    #[cfg(not(target_arch = "wasm32"))]
    let args: Args = argh::from_env();
    #[cfg(target_arch = "wasm32")]
    let args = Args::from_args(&[], &[]).unwrap();

    let mut app = App::new();

    app.add_plugins((
        FrameTimeDiagnosticsPlugin::default(),
        LogDiagnosticsPlugin::default(),
        DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                present_mode: PresentMode::AutoNoVsync,
                resolution: WindowResolution::new(1920.0, 1080.0).with_scale_factor_override(1.0),
                ..default()
            }),
            ..default()
        }),
    ))
    .init_resource::<FontHandle>()
    .add_systems(Startup, setup)
    .add_systems(Update, (move_camera, print_counts));

    if args.recompute {
        app.add_systems(Update, recompute);
    }

    app.insert_resource(args).run();
}

#[derive(Deref, DerefMut)]
struct PrintingTimer(Timer);

impl Default for PrintingTimer {
    fn default() -> Self {
        Self(Timer::from_seconds(1.0, TimerMode::Repeating))
    }
}

fn setup(mut commands: Commands, font: Res<FontHandle>, args: Res<Args>) {
    warn!(include_str!("warning_string.txt"));

    let mut rng = ChaCha8Rng::seed_from_u64(42);

    let tile_size = Vec2::splat(64.0);
    let map_size = Vec2::splat(640.0);

    let half_x = (map_size.x / 4.0) as i32;
    let half_y = (map_size.y / 4.0) as i32;

    // Spawns the camera

    commands.spawn(Camera2d);

    // Builds and spawns the `Text2d`s, distributing them in a way that ensures a
    // good distribution of on-screen and off-screen entities.
    let mut text2ds = vec![];
    for y in -half_y..half_y {
        for x in -half_x..half_x {
            let position = Vec2::new(x as f32, y as f32);
            let translation = (position * tile_size).extend(rng.random::<f32>());
            let rotation = Quat::from_rotation_z(rng.random::<f32>());
            let scale = Vec3::splat(rng.random::<f32>() * 2.0);
            let color = Hsla::hsl(rng.random_range(0.0..360.0), 0.8, 0.8);

            text2ds.push((
                Text2d(random_text(&mut rng, &args)),
                random_text_font(&mut rng, &args, font.0.clone()),
                TextColor(color.into()),
                TextLayout::new_with_justify(if args.center {
                    Justify::Center
                } else {
                    Justify::Left
                }),
                Transform {
                    translation,
                    rotation,
                    scale,
                },
            ));
        }
    }

    if args.no_frustum_culling {
        let bundles = text2ds.into_iter().map(|bundle| (bundle, NoFrustumCulling));
        commands.spawn_batch(bundles);
    } else {
        commands.spawn_batch(text2ds);
    }
}

// System for rotating and translating the camera
fn move_camera(time: Res<Time>, mut camera_query: Query<&mut Transform, With<Camera>>) {
    let Ok(mut camera_transform) = camera_query.single_mut() else {
        return;
    };
    camera_transform.rotate_z(time.delta_secs() * 0.5);
    *camera_transform =
        *camera_transform * Transform::from_translation(Vec3::X * CAMERA_SPEED * time.delta_secs());
}

// System for printing the number of texts on every tick of the timer
fn print_counts(
    time: Res<Time>,
    mut timer: Local<PrintingTimer>,
    texts: Query<&ViewVisibility, With<Text2d>>,
    atlases: Res<FontAtlasSets>,
    font: Res<FontHandle>,
) {
    timer.tick(time.delta());
    if !timer.just_finished() {
        return;
    }

    let num_atlases = atlases
        .get(font.0.id())
        .map(|set| set.iter().map(|atlas| atlas.1.len()).sum())
        .unwrap_or(0);

    let visible_texts = texts.iter().filter(|visibility| visibility.get()).count();

    info!(
        "Texts: {} Visible: {} Atlases: {}",
        texts.iter().count(),
        visible_texts,
        num_atlases
    );
}

fn random_text_font(rng: &mut ChaCha8Rng, args: &Args, font: Handle<Font>) -> TextFont {
    let font_size = if args.many_font_sizes {
        *[10.0, 20.0, 30.0, 40.0, 50.0, 60.0].choose(rng).unwrap()
    } else {
        60.0
    };

    TextFont {
        font_size,
        font,
        ..default()
    }
}

fn random_text(rng: &mut ChaCha8Rng, args: &Args) -> String {
    if !args.many_glyphs {
        return "Bevy".to_string();
    }

    CODE_POINT_RANGES
        .choose(rng)
        .unwrap()
        .clone()
        .choose_multiple(rng, 4)
        .into_iter()
        .map(|cp| char::from_u32(cp).unwrap())
        .collect::<String>()
}

fn recompute(mut query: Query<&mut Text2d>) {
    for mut text2d in &mut query {
        text2d.set_changed();
    }
}
