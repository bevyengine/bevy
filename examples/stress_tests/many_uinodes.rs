//! This example shows what happens when there is a lot of UI nodes on screen.

use bevy_internal::{
    render::{texture::DEFAULT_IMAGE_HANDLE, Extract, RenderApp},
    ui::{ExtractedUiNode, ExtractedUiNodes},
};

use bevy::{
    diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin},
    prelude::*,
    window::{PresentMode, WindowPlugin},
};
use rand::{seq::SliceRandom, Rng, SeedableRng};

fn main() {
    let mut app = App::new();
    app.add_plugins((
        DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                resolution: (1024.0, 768.0).into(),
                title: "many_uinodes".into(),
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

    render_app.add_systems(
        ExtractSchedule,
        (
            extract::<A>,
            extract::<B>,
            extract::<C>,
            extract::<D>,
            extract::<E>,
        ),
    );

    app.run();
}

#[derive(Component)]
pub struct A;

#[derive(Component)]
pub struct B;

#[derive(Component)]
pub struct C;

#[derive(Component)]
pub struct D;

#[derive(Component)]
pub struct E;

fn extract<M: Component>(
    mut extracted_uinodes: ResMut<ExtractedUiNodes>,
    images: Extract<Res<Assets<Image>>>,
    uinode_query: Extract<
        Query<
            (
                Entity,
                &StackIndex,
                &Size,
                &GlobalTransform,
                &BackgroundColor,
                Option<&UiImage>,
                &ComputedVisibility,
            ),
            With<M>,
        >,
    >,
) {
    for (entity, stack_index, size, transform, color, maybe_image, visibility) in
        uinode_query.iter()
    {
        // Skip invisible and completely transparent nodes
        if !visibility.is_visible() || color.0.a() == 0.0 {
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

        extracted_uinodes.push_node(
            stack_index.0 as u32,
            ExtractedUiNode {
                transform: transform.compute_matrix(),
                color: color.0,
                rect: Rect {s
                    min: Vec2::ZERO,
                    max: size.0,
                },
                clip: None,
                image,
                atlas_size: None,
                flip_x,
                flip_y,
            },
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
    let mut rng = rand::rngs::StdRng::seed_from_u64(42);

    for stack_index in 0..100_000 {
        let w = rng.gen_range(10.0..150.0);
        let h = rng.gen_range(10.0..150.0);
        let x = rng.gen_range(0.0..1024.0);
        let y = rng.gen_range(0.0..768.0);
        let color = *colors.choose(&mut rng).unwrap();

        let mut builder = commands.spawn((
            Size(Vec2::new(w, h)),
            Transform::from_translation(Vec3::new(x, y, 1.0)),
            GlobalTransform::default(),
            StackIndex(stack_index),
            BackgroundColor(color),
            VisibilityBundle::default(),
        ));

        if rng.gen_range(0..5) == 0 {
            let image = image_handles.choose(&mut rng).unwrap();
            builder.insert(UiImage::new(asset_server.load(*image)));
        }

        match rng.gen_range(0..5) {
            0 => builder.insert(A),
            1 => builder.insert(B),
            2 => builder.insert(C),
            3 => builder.insert(D),
            _ => builder.insert(E),
        };
    }
}
