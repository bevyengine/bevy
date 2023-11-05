use bevy::prelude::*;
use bevy::render::render_resource::Extent3d;
use bevy::render::render_resource::TextureDimension;
use bevy::render::render_resource::TextureFormat;
use rand::Rng;

const IMAGE_WIDTH: u32 = 256;
const IMAGE_HEIGHT: u32 = 256;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        // Let's make the fixed timestep really fast for this example.
        .insert_resource(Time::<Fixed>::from_hz(256.0))
        .add_systems(Startup, setup)
        .add_systems(FixedUpdate, draw)
        .run();
}

/// Store the image handle that we will draw to, here.
#[derive(Resource)]
struct MyProcGenImage(Handle<Image>);

fn setup(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
    // spawn a camera
    commands.spawn(Camera2dBundle::default());

    // create an image that we are going to draw into
    let mut image = Image::new_fill(
        // 2D image of size 256x256
        Extent3d {
            width: IMAGE_WIDTH,
            height: IMAGE_HEIGHT,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        // Initialize it with a beige color
        &(Color::BEIGE.as_rgba_u8()),
        // Use the most common encoding for the data
        // (8-bit RGBA with sRGB gamma)
        TextureFormat::Rgba8UnormSrgb,
    );

    // to make it extra fancy, we can set the Alpha of each pixel
    // so that it fades out in a circular fashion
    for y in 0..IMAGE_HEIGHT {
        for x in 0..IMAGE_WIDTH {
            let center = Vec2::new(IMAGE_WIDTH as f32 / 2.0, IMAGE_HEIGHT as f32 / 2.0);
            let max_radius = IMAGE_HEIGHT.min(IMAGE_WIDTH) as f32 / 2.0;
            let r = Vec2::new(x as f32, y as f32).distance(center);
            let a = 1.0 - (r / max_radius as f32).clamp(0.0, 1.0);

            // here we will set the A value by accessing the raw data bytes
            // (it is the 4th byte of each pixel, as per our `TextureFormat`)

            // find our pixel by its coordinates
            let pixel_bytes = image.pixel_bytes_mut(UVec3::new(x, y, 0)).unwrap();
            // convert our f32 to u8
            pixel_bytes[3] = (a * u8::MAX as f32) as u8;
        }
    }

    // add it to Bevy's assets, so it can be used for rendering
    // this will give us a handle we can use
    // (to display it in a sprite, or as part of UI, etc.)
    let handle = images.add(image);

    // create a sprite entity using our image
    commands.spawn(SpriteBundle {
        texture: handle.clone(),
        ..Default::default()
    });

    commands.insert_resource(MyProcGenImage(handle));
}

/// Every fixed update tick, draw one more pixel to make a spiral pattern
fn draw(
    my_handle: Res<MyProcGenImage>,
    mut images: ResMut<Assets<Image>>,
    // used to keep track of where we are
    mut i: Local<u32>,
    mut draw_color: Local<Color>,
) {
    let mut rng = rand::thread_rng();

    if *i == 0 {
        // Generate a random color on first run.
        *draw_color = Color::rgb(rng.gen(), rng.gen(), rng.gen());
    }

    // Get the image from Bevy's asset storage.
    let image = images.get_mut(&my_handle.0).expect("Image not found");

    // Compute the position of the pixel to draw.

    let center = Vec2::new(IMAGE_WIDTH as f32 / 2.0, IMAGE_HEIGHT as f32 / 2.0);
    let max_radius = IMAGE_HEIGHT.min(IMAGE_WIDTH) as f32 / 2.0;
    let rot_speed = 0.0123;
    let period = 0.12345;

    let r = (*i as f32 * period).sin() * max_radius;
    let xy = Vec2::from_angle(*i as f32 * rot_speed) * r + center;
    let pos = UVec3::new(xy.x as u32, xy.y as u32, 0);

    // Get the old color of that pixel.
    let old_color = image.get_color_at(pos).unwrap();

    // If the old color is our current color, change our drawing color.
    // (the values are never going to match exactly,
    // because of the f32 -> u8 -> f32 conversion)
    let tolerance = 1.0 / 255.0;
    if (old_color.r() - draw_color.r()).abs() <= tolerance
        && (old_color.g() - draw_color.g()).abs() <= tolerance
        && (old_color.b() - draw_color.b()).abs() <= tolerance
    {
        *draw_color = Color::rgb(rng.gen(), rng.gen(), rng.gen());
    }

    // Set the new color, but keep old alpha value from image.
    image
        .set_color_at(pos, draw_color.with_a(old_color.a()))
        .unwrap();

    *i += 1;
}
