//! Draws a mix of approximately `100_000` textured and untextured rectangles on screen using the UI's renderer.
//!
//! This example doesn't spawn UI node bundles and instead add its own custom extraction functions to the `ExtractSchedule`.
//! This bypasses the layout systems so that only the UI's rendering systems are put under stress.
//!
//! To run the demo with extraction iterating the UI stack use:
//! `cargo run --example many_rects --release iter-stack`
//!
use bevy::render::{texture::DEFAULT_IMAGE_HANDLE, Extract, RenderApp};
use bevy::ui::{ExtractedUiNode, ExtractedUiNodes};

use bevy::{
    diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin},
    prelude::*,
    window::{PresentMode, WindowPlugin},
};
use rand::{seq::SliceRandom, Rng, SeedableRng};

const SEED: u64 = 42;
const MIN_EDGE: f32 = 10.;
const MAX_EDGE: f32 = 150.;
const WIDTH: f32 = 1024.;
const HEIGHT: f32 = 768.;
const STACK_SIZE: usize = 33000;
const TEXTURED_RATIO: f32 = 0.2;

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
struct ExtractRect;

fn main() {
    let mut app = App::new();
    app.add_plugins((
        DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                resolution: (WIDTH, HEIGHT).into(),
                title: "many_rects".into(),
                present_mode: PresentMode::AutoNoVsync,
                ..default()
            }),
            ..default()
        }),
        FrameTimeDiagnosticsPlugin,
        LogDiagnosticsPlugin::default(),
    ))
    .add_systems(Startup, setup);

    let render_app = match app.get_sub_app_mut(RenderApp) {
        Ok(render_app) => render_app,
        Err(_) => return,
    };

    if std::env::args().any(|arg| arg == "iter-stack") {
        render_app.add_systems(
            ExtractSchedule,
            (
                extract_rect_iter_stack::<1>,
                extract_rect_iter_stack::<2>,
                extract_rect_iter_stack::<4>,
                extract_rect_iter_stack::<8>,
                extract_rect_iter_stack::<16>,
                extract_rect_iter_stack::<32>,
            )
                .chain(),
        );
    } else {
        render_app.add_systems(
            ExtractSchedule,
            (
                extract_rect::<1>,
                extract_rect::<2>,
                extract_rect::<4>,
                extract_rect::<8>,
                extract_rect::<16>,
                extract_rect::<32>,
            )
                .chain(),
        );
    }

    app.run();
}

#[derive(Component)]
pub struct ExtractionMarker<const N: usize>;

#[derive(Resource, Deref, DerefMut)]
pub struct RectStack(Vec<Entity>);

fn extract_rect_iter_stack<const N: usize>(
    mut extracted_uinodes: ResMut<ExtractedUiNodes>,
    images: Extract<Res<Assets<Image>>>,
    ui_stack: Extract<Res<RectStack>>,
    uinode_query: Extract<
        Query<
            (
                &Size,
                &GlobalTransform,
                &BackgroundColor,
                Option<&UiImage>,
                &ViewVisibility,
            ),
            With<ExtractionMarker<N>>,
        >,
    >,
) {
    let mut extraction_buffer = extracted_uinodes.get_buffer();
    for (stack_index, entity) in ui_stack.iter().enumerate() {
        if let Ok((size, transform, color, maybe_image, visibility)) = uinode_query.get(*entity) {
            // Skip invisible and completely transparent nodes
            if !visibility.get() || color.0.a() == 0.0 {
                continue;
            }

            let (image, flip_x, flip_y) = if let Some(image) = maybe_image {
                // Skip loading images
                if !images.contains(&image.texture) {
                    continue;
                }
                (image.texture.clone_weak(), image.flip_x, image.flip_y)
            } else {
                (DEFAULT_IMAGE_HANDLE.typed(), false, false)
            };
            extraction_buffer.extend(
                stack_index as u32,
                (0..N).map(|_| ExtractedUiNode {
                    transform: transform.compute_matrix(),
                    color: color.0,
                    rect: Rect {
                        min: Vec2::ZERO,
                        max: size.0,
                    },
                    clip: None,
                    image: image.clone_weak(),
                    atlas_size: None,
                    flip_x,
                    flip_y,
                }),
            );
        }
    }
}

fn extract_rect<const N: usize>(
    mut extracted_uinodes: ResMut<ExtractedUiNodes>,
    images: Extract<Res<Assets<Image>>>,
    uinode_query: Extract<
        Query<
            (
                &StackIndex,
                &Size,
                &GlobalTransform,
                &BackgroundColor,
                Option<&UiImage>,
                &ViewVisibility,
            ),
            With<ExtractionMarker<N>>,
        >,
    >,
) {
    let mut extraction_buffer = extracted_uinodes.get_buffer();
    for (stack_index, size, transform, color, maybe_image, visibility) in uinode_query.iter() {
        // Skip invisible and completely transparent nodes
        if !visibility.get() || color.0.a() == 0.0 {
            continue;
        }

        let (image, flip_x, flip_y) = if let Some(image) = maybe_image {
            // Skip loading images
            if !images.contains(&image.texture) {
                continue;
            }
            (image.texture.clone_weak(), image.flip_x, image.flip_y)
        } else {
            (DEFAULT_IMAGE_HANDLE.typed(), false, false)
        };
        extraction_buffer.extend(
            stack_index.0 as u32,
            (0..N).map(|_| ExtractedUiNode {
                transform: transform.compute_matrix(),
                color: color.0,
                rect: Rect {
                    min: Vec2::ZERO,
                    max: size.0,
                },
                clip: None,
                image: image.clone_weak(),
                atlas_size: None,
                flip_x,
                flip_y,
            }),
        );
    }
}

#[derive(Component)]
pub struct Size(Vec2);

#[derive(Component)]
pub struct StackIndex(usize);

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn(Camera2dBundle::default());

    let image_handles = [
        "branding/bevy_logo_light.png",
        "branding/bevy_logo_dark.png",
        "branding/icon.png",
    ];
    let colors = [
        Color::WHITE,
        Color::RED,
        Color::GREEN,
        Color::BLUE,
        Color::YELLOW,
    ];
    let mut rng = rand::rngs::StdRng::seed_from_u64(SEED);
    let mut rect_stack = RectStack(Vec::with_capacity(STACK_SIZE));
    for _ in 0..STACK_SIZE {
        let n = rng.gen_range(0..63);
        let mut builder = match n {
            0..=31 => commands.spawn(ExtractionMarker::<1>),
            32..=47 => commands.spawn(ExtractionMarker::<2>),
            48..=55 => commands.spawn(ExtractionMarker::<4>),
            56..=59 => commands.spawn(ExtractionMarker::<8>),
            60..=61 => commands.spawn(ExtractionMarker::<16>),
            _ => commands.spawn(ExtractionMarker::<32>),
        };
        if rng.gen::<f32>() <= TEXTURED_RATIO {
            let image = image_handles.choose(&mut rng).unwrap();
            builder.insert(UiImage::new(asset_server.load(*image)));
        }
        rect_stack.push(builder.id());
    }
    rect_stack.shuffle(&mut rng);

    let bundles: Vec<_> = rect_stack
        .iter()
        .enumerate()
        .map(|(stack_index, entity)| {
            (*entity, {
                let w = rng.gen_range(MIN_EDGE..MAX_EDGE);
                let h = rng.gen_range(MIN_EDGE..MAX_EDGE);
                let x = rng.gen_range(0.0..WIDTH);
                let y = rng.gen_range(0.0..HEIGHT);
                let color = *colors.choose(&mut rng).unwrap();
                (
                    Size(Vec2::new(w, h)),
                    Transform::from_translation(Vec3::new(x, y, 1.0)),
                    GlobalTransform::default(),
                    StackIndex(stack_index),
                    BackgroundColor(color),
                    VisibilityBundle::default(),
                )
            })
        })
        .collect();

    commands.insert_or_spawn_batch(bundles);
    commands.insert_resource(rect_stack);
}
