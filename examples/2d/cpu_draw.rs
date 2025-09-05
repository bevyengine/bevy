//! Example of how to draw to a texture from the CPU.
//!
//! You can set the values of individual pixels to whatever you want.
//! Bevy provides user-friendly APIs that work with [`Color`]
//! values and automatically perform any necessary conversions and encoding
//! into the texture's native pixel format.

use bevy::asset::RenderAssetUsages;
use bevy::color::{color_difference::EuclideanDistance, palettes::css};
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;

const IMAGE_WIDTH: u32 = 256;
const IMAGE_HEIGHT: u32 = 256;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        // In this example, we will use a fixed timestep to draw a pattern on the screen
        // one pixel at a time, so the pattern will gradually emerge over time, and
        // the speed at which it appears is not tied to the framerate.
        // Let's make the fixed update very fast, so it doesn't take too long. :)
        .insert_resource(Time::<Fixed>::from_hz(1024.0))
        .add_systems(Startup, setup)
        .add_systems(FixedUpdate, draw)
        .run();
}

/// Store the image handle that we will draw to, here.
#[derive(Resource)]
struct MyProcGenImage(Handle<Image>);

#[derive(Resource)]
struct SeededRng(ChaCha8Rng);

fn setup(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
    commands.spawn(Camera2d);

    // Create an image that we are going to draw into
    let mut image = Image::new_fill(
        // 2D image of size 256x256
        Extent3d {
            width: IMAGE_WIDTH,
            height: IMAGE_HEIGHT,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        // Initialize it with a beige color
        &(css::BEIGE.to_u8_array()),
        // Use the same encoding as the color we set
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    );

    // To make it extra fancy, we can set the Alpha of each pixel,
    // so that it fades out in a circular fashion.
    for y in 0..IMAGE_HEIGHT {
        for x in 0..IMAGE_WIDTH {
            let center = Vec2::new(IMAGE_WIDTH as f32 / 2.0, IMAGE_HEIGHT as f32 / 2.0);
            let max_radius = IMAGE_HEIGHT.min(IMAGE_WIDTH) as f32 / 2.0;
            let r = Vec2::new(x as f32, y as f32).distance(center);
            let a = 1.0 - (r / max_radius).clamp(0.0, 1.0);

            // Here we will set the A value by accessing the raw data bytes.
            // (it is the 4th byte of each pixel, as per our `TextureFormat`)

            // Find our pixel by its coordinates
            let pixel_bytes = image.pixel_bytes_mut(UVec3::new(x, y, 0)).unwrap();
            // Convert our f32 to u8
            pixel_bytes[3] = (a * u8::MAX as f32) as u8;
        }
    }

    // Add it to Bevy's assets, so it can be used for rendering
    // this will give us a handle we can use
    // (to display it in a sprite, or as part of UI, etc.)
    let handle = images.add(image);

    // Create a sprite entity using our image
    commands.spawn(Sprite::from_image(handle.clone()));
    commands.insert_resource(MyProcGenImage(handle));

    // We're seeding the PRNG here to make this example deterministic for testing purposes.
    // This isn't strictly required in practical use unless you need your app to be deterministic.
    let seeded_rng = ChaCha8Rng::seed_from_u64(19878367467712);
    commands.insert_resource(SeededRng(seeded_rng));
}

/// Every fixed update tick, draw one more pixel to make a spiral pattern
fn draw(
    my_handle: Res<MyProcGenImage>,
    mut images: ResMut<Assets<Image>>,
    // Used to keep track of where we are
    mut i: Local<u32>,
    mut draw_color: Local<Color>,
    mut seeded_rng: ResMut<SeededRng>,
) {
    if *i == 0 {
        // Generate a random color on first run.
        *draw_color = Color::linear_rgb(
            seeded_rng.0.random(),
            seeded_rng.0.random(),
            seeded_rng.0.random(),
        );
    }

    // Get the image from Bevy's asset storage.
    let image = images.get_mut(&my_handle.0).expect("Image not found");

    // Compute the position of the pixel to draw.

    let center = Vec2::new(IMAGE_WIDTH as f32 / 2.0, IMAGE_HEIGHT as f32 / 2.0);
    let max_radius = IMAGE_HEIGHT.min(IMAGE_WIDTH) as f32 / 2.0;
    let rot_speed = 0.0123;
    let period = 0.12345;

    let r = ops::sin(*i as f32 * period) * max_radius;
    let xy = Vec2::from_angle(*i as f32 * rot_speed) * r + center;
    let (x, y) = (xy.x as u32, xy.y as u32);

    // Get the old color of that pixel.
    let old_color = image.get_color_at(x, y).unwrap();

    // If the old color is our current color, change our drawing color.
    let tolerance = 1.0 / 255.0;
    if old_color.distance(&draw_color) <= tolerance {
        *draw_color = Color::linear_rgb(
            seeded_rng.0.random(),
            seeded_rng.0.random(),
            seeded_rng.0.random(),
        );
    }

    // Set the new color, but keep old alpha value from image.
    image
        .set_color_at(x, y, draw_color.with_alpha(old_color.alpha()))
        .unwrap();

    *i += 1;
}
