//! This example showcases how to implement 2D screen shake.
//! It follows the GDC talk ["Math for Game Programmers: Juicing Your Cameras With Math"](https://www.youtube.com/watch?v=tu-Qe66AvtY) by Squirrel Eiserloh
//!
//! The key features are:
//! - Camera shake is dependent on a "trauma" value between 0.0 and 1.0. The more trauma, the stronger the shake.
//! - Trauma automatically decays over time.
//! - The camera shake will always only affect the camera `Transform` up to a maximum displacement.
//! - The camera's `Transform` is only affected by the shake for the rendering. The `Transform` stays "normal" for the rest of the game logic.
//! - All displacements are governed by a noise function, guaranteeing that the shake is smooth and continuous.
//!   This means that the camera won't jump around wildly.
//!
//! ## Controls
//!
//! | Key Binding                      | Action                     |
//! |:---------------------------------|:---------------------------|
//! | Space (pressed repeatedly)       | Increase camera trauma     |

use bevy::{
    input::common_conditions::input_just_pressed, math::ops::powf, prelude::*,
    sprite_render::MeshMaterial2d,
};

// Before we implement the code, let's quickly introduce the underlying constants.
// They are later encoded in a `CameraShakeConfig` component, but introduced here so we can easily tweak them.
// Try playing around with them and see how the shake behaves!

/// The trauma decay rate controls how quickly the trauma decays.
/// 0.5 means that a full trauma of 1.0 will decay to 0.0 in 2 seconds.
const TRAUMA_DECAY_PER_SECOND: f32 = 0.5;

/// The trauma exponent controls how the trauma affects the shake.
/// Camera shakes don't feel punchy when they go up linearly, so we use an exponent of 2.0.
/// The higher the exponent, the more abrupt is the transition between no shake and full shake.
const TRAUMA_EXPONENT: f32 = 2.0;

/// The maximum angle the camera can rotate on full trauma.
/// 10.0 degrees is a somewhat high but still reasonable shake. Try bigger values for something more silly and wiggly.
const MAX_ANGLE: f32 = 10.0_f32.to_radians();

/// The maximum translation the camera will move on full trauma in both the x and y directions.
/// 20.0 px is a low enough displacement to not be distracting. Try higher values for an effect that looks like the camera is wandering around.
const MAX_TRANSLATION: f32 = 20.0;

/// How much we are traversing the noise function in arbitrary units per second.
/// This dictates how fast the camera shakes.
/// 20.0 is a fairly fast shake. Try lower values for a more dreamy effect.
const NOISE_SPEED: f32 = 20.0;

/// How much trauma we add per press of the space key.
/// A value of 1.0 would mean that a single press would result in a maximum trauma, i.e. 1.0.
const TRAUMA_PER_PRESS: f32 = 0.4;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, (setup_scene, setup_instructions, setup_camera))
        // At the start of the frame, restore the camera's transform to its unshaken state.
        .add_systems(PreUpdate, reset_transform)
        .add_systems(
            Update,
            // Increase trauma when the space key is pressed.
            increase_trauma.run_if(input_just_pressed(KeyCode::Space)),
        )
        // Just before the end of the frame, apply the shake.
        // This is ordered so that the transform propagation produces correct values for the global transform, which is used by Bevy's rendering.
        .add_systems(PostUpdate, shake_camera.before(TransformSystems::Propagate))
        .run();
}

/// Let's start with the core mechanic: how do we shake the camera?
/// This system runs right at the end of the frame, so that we can sneak in the shake effect before rendering kicks in.
fn shake_camera(
    camera_shake: Single<(&mut CameraShakeState, &CameraShakeConfig, &mut Transform)>,
    time: Res<Time>,
) {
    let (mut camera_shake, config, mut transform) = camera_shake.into_inner();

    // Before we even start thinking about the shake, we save the original transform so it's not lost.
    // At the start of the next frame, we will restore the camera's transform to this original transform.
    camera_shake.original_transform = *transform;

    // To generate the transform offset, we use a noise function. Noise is like a random number generator, but cooler.
    // Let's start with a visual intuition: <https://assets-global.website-files.com/64b6d182aee713bd0401f4b9/64b95974ec292aabac45fc8e_image.png>
    // The image on the left is made from pure randomness, the image on the right is made from a kind of noise called Perlin noise.
    // Notice how the noise has much more "structure" than the randomness? How it looks like it has peaks and valleys?
    // This property makes noise very desirable for a variety of visual effects. In our case, what we want is that the
    // camera does not wildly teleport around the world, but instead *moves* through the world frame by frame.
    // We can use 1D Perlin noise for this, which takes one input and outputs a value between -1.0 and 1.0. If we increase the input by a little bit,
    // like by the time since the last frame, we get a different output that is still "close" to the previous one.

    // This is the input to the noise function. Just using the elapsed time is pretty good input,
    // since it means that noise generations that are close in time will be close in output.
    // We simply multiply it by a constant to be able to "speed up" or "slow down" the noise.
    let t = time.elapsed_secs() * config.noise_speed;

    // Now we generate three noise values. One for the rotation, one for the x-offset, and one for the y-offset.
    // But if we generated those three noise values with the same input, we would get the same output three times!
    // To avoid this, we simply add a random offset to each input.
    // You can think of this as the seed value you would give a random number generator.
    let rotation_noise = perlin_noise::generate(t + 0.0);
    let x_noise = perlin_noise::generate(t + 100.0);
    let y_noise = perlin_noise::generate(t + 200.0);

    // Games often deal with linear increments. For example, if an enemy deals 10 damage and attacks you 2 times, you will take 20 damage.
    // But that's not how impact feels! Human senses are much more attuned to exponential changes.
    // So, we make sure that the `shake` value we use is an exponential function of the trauma.
    // But doesn't this make the value explode? Fortunately not: since `trauma` is between 0.0 and 1.0, exponentiating it will actually make it smaller!
    // See <https://www.wolframalpha.com/input?i=plot+x+and+x%5E2+and+x%5E3+for+x+in+%5B0%2C+1%5D> for a graph.
    let shake = powf(camera_shake.trauma, config.exponent);

    // Now, to get the final offset, we multiply this noise value by the shake value and the maximum value.
    // The noise value is in [-1, 1], so by multiplying it with a maximum value, we get a value in [-max_value, +max_value].
    // Multiply this by the shake value to get the exponential effect, and we're done!
    let roll_offset = rotation_noise * shake * config.max_angle;
    let x_offset = x_noise * shake * config.max_translation;
    let y_offset = y_noise * shake * config.max_translation;

    // Finally, we apply the offset to the camera's transform. Since we already stored the original transform,
    // and this system runs right at the end of the frame, we can't accidentally break any game logic by changing the transform.
    transform.translation.x += x_offset;
    transform.translation.y += y_offset;
    transform.rotate_z(roll_offset);

    // Some bookkeeping at the end: trauma should decay over time.
    camera_shake.trauma -= config.trauma_decay_per_second * time.delta_secs();
    camera_shake.trauma = camera_shake.trauma.clamp(0.0, 1.0);
}

/// Increase the trauma when the space key is pressed.
fn increase_trauma(mut camera_shake: Single<&mut CameraShakeState>) {
    camera_shake.trauma += TRAUMA_PER_PRESS;
    camera_shake.trauma = camera_shake.trauma.clamp(0.0, 1.0);
}

/// Restore the camera's transform to its unshaken state.
/// Runs at the start of the frame, so that gameplay logic doesn't need to care about camera shake.
fn reset_transform(camera_shake: Single<(&CameraShakeState, &mut Transform)>) {
    let (camera_shake, mut transform) = camera_shake.into_inner();
    *transform = camera_shake.original_transform;
}

/// The current state of the camera shake that is updated every frame.
#[derive(Component, Debug, Default)]
struct CameraShakeState {
    /// The current trauma level in [0.0, 1.0].
    trauma: f32,
    /// The original transform of the camera before applying the shake.
    /// We store this so that we can restore the camera's transform to its original state at the start of the next frame.
    original_transform: Transform,
}

/// Configuration for the camera shake.
/// See the constants at the top of the file for some good default values and detailed explanations.
#[derive(Component, Debug)]
#[require(CameraShakeState)]
struct CameraShakeConfig {
    trauma_decay_per_second: f32,
    exponent: f32,
    max_angle: f32,
    max_translation: f32,
    noise_speed: f32,
}

fn setup_camera(mut commands: Commands) {
    commands.spawn((
        Camera2d,
        // Enable camera shake for this camera.
        CameraShakeConfig {
            trauma_decay_per_second: TRAUMA_DECAY_PER_SECOND,
            exponent: TRAUMA_EXPONENT,
            max_angle: MAX_ANGLE,
            max_translation: MAX_TRANSLATION,
            noise_speed: NOISE_SPEED,
        },
    ));
}

/// Spawn a scene so we have something to look at.
fn setup_scene(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    // Background tile
    commands.spawn((
        Mesh2d(meshes.add(Rectangle::new(1000., 700.))),
        MeshMaterial2d(materials.add(Color::srgb(0.2, 0.2, 0.3))),
    ));

    // The shape in the middle could be our player character.
    commands.spawn((
        Mesh2d(meshes.add(Rectangle::new(50.0, 100.0))),
        MeshMaterial2d(materials.add(Color::srgb(0.25, 0.94, 0.91))),
        Transform::from_xyz(0., 0., 2.),
    ));

    // These two shapes could be obstacles.
    commands.spawn((
        Mesh2d(meshes.add(Rectangle::new(50.0, 50.0))),
        MeshMaterial2d(materials.add(Color::srgb(0.85, 0.0, 0.2))),
        Transform::from_xyz(-450.0, 200.0, 2.),
    ));

    commands.spawn((
        Mesh2d(meshes.add(Rectangle::new(70.0, 50.0))),
        MeshMaterial2d(materials.add(Color::srgb(0.5, 0.8, 0.2))),
        Transform::from_xyz(450.0, -150.0, 2.),
    ));
}

fn setup_instructions(mut commands: Commands) {
    commands.spawn((
        Text::new("Press space repeatedly to trigger a progressively stronger screen shake"),
        Node {
            position_type: PositionType::Absolute,
            bottom: Val::Px(12.0),
            left: Val::Px(12.0),
            ..default()
        },
    ));
}

/// Tiny 1D Perlin noise implementation. The mathematical details are not important here.
mod perlin_noise {
    use super::*;

    pub fn generate(x: f32) -> f32 {
        // Left coordinate of the unit-line that contains the input.
        let x_floor = x.floor() as usize;

        // Input location in the unit-line.
        let xf0 = x - x_floor as f32;
        let xf1 = xf0 - 1.0;

        // Wrap to range 0-255.
        let xi0 = x_floor & 0xFF;
        let xi1 = (x_floor + 1) & 0xFF;

        // Apply the fade function to the location.
        let t = fade(xf0).clamp(0.0, 1.0);

        // Generate hash values for each point of the unit-line.
        let h0 = PERMUTATION_TABLE[xi0];
        let h1 = PERMUTATION_TABLE[xi1];

        // Linearly interpolate between dot products of each gradient with its distance to the input location.
        let a = dot_grad(h0, xf0);
        let b = dot_grad(h1, xf1);
        a.interpolate_stable(&b, t)
    }

    // A cubic curve that smoothly transitions from 0 to 1 as t goes from 0 to 1
    fn fade(t: f32) -> f32 {
        t * t * t * (t * (t * 6.0 - 15.0) + 10.0)
    }

    fn dot_grad(hash: u8, xf: f32) -> f32 {
        // In 1D case, the gradient may be either 1 or -1.
        // The distance vector is the input offset (relative to the smallest bound).
        if hash & 0x1 != 0 {
            xf
        } else {
            -xf
        }
    }

    // Perlin noise permutation table. This is a random sequence of the numbers 0-255.
    const PERMUTATION_TABLE: [u8; 256] = [
        0x97, 0xA0, 0x89, 0x5B, 0x5A, 0x0F, 0x83, 0x0D, 0xC9, 0x5F, 0x60, 0x35, 0xC2, 0xE9, 0x07,
        0xE1, 0x8C, 0x24, 0x67, 0x1E, 0x45, 0x8E, 0x08, 0x63, 0x25, 0xF0, 0x15, 0x0A, 0x17, 0xBE,
        0x06, 0x94, 0xF7, 0x78, 0xEA, 0x4B, 0x00, 0x1A, 0xC5, 0x3E, 0x5E, 0xFC, 0xDB, 0xCB, 0x75,
        0x23, 0x0B, 0x20, 0x39, 0xB1, 0x21, 0x58, 0xED, 0x95, 0x38, 0x57, 0xAE, 0x14, 0x7D, 0x88,
        0xAB, 0xA8, 0x44, 0xAF, 0x4A, 0xA5, 0x47, 0x86, 0x8B, 0x30, 0x1B, 0xA6, 0x4D, 0x92, 0x9E,
        0xE7, 0x53, 0x6F, 0xE5, 0x7A, 0x3C, 0xD3, 0x85, 0xE6, 0xDC, 0x69, 0x5C, 0x29, 0x37, 0x2E,
        0xF5, 0x28, 0xF4, 0x66, 0x8F, 0x36, 0x41, 0x19, 0x3F, 0xA1, 0x01, 0xD8, 0x50, 0x49, 0xD1,
        0x4C, 0x84, 0xBB, 0xD0, 0x59, 0x12, 0xA9, 0xC8, 0xC4, 0x87, 0x82, 0x74, 0xBC, 0x9F, 0x56,
        0xA4, 0x64, 0x6D, 0xC6, 0xAD, 0xBA, 0x03, 0x40, 0x34, 0xD9, 0xE2, 0xFA, 0x7C, 0x7B, 0x05,
        0xCA, 0x26, 0x93, 0x76, 0x7E, 0xFF, 0x52, 0x55, 0xD4, 0xCF, 0xCE, 0x3B, 0xE3, 0x2F, 0x10,
        0x3A, 0x11, 0xB6, 0xBD, 0x1C, 0x2A, 0xDF, 0xB7, 0xAA, 0xD5, 0x77, 0xF8, 0x98, 0x02, 0x2C,
        0x9A, 0xA3, 0x46, 0xDD, 0x99, 0x65, 0x9B, 0xA7, 0x2B, 0xAC, 0x09, 0x81, 0x16, 0x27, 0xFD,
        0x13, 0x62, 0x6C, 0x6E, 0x4F, 0x71, 0xE0, 0xE8, 0xB2, 0xB9, 0x70, 0x68, 0xDA, 0xF6, 0x61,
        0xE4, 0xFB, 0x22, 0xF2, 0xC1, 0xEE, 0xD2, 0x90, 0x0C, 0xBF, 0xB3, 0xA2, 0xF1, 0x51, 0x33,
        0x91, 0xEB, 0xF9, 0x0E, 0xEF, 0x6B, 0x31, 0xC0, 0xD6, 0x1F, 0xB5, 0xC7, 0x6A, 0x9D, 0xB8,
        0x54, 0xCC, 0xB0, 0x73, 0x79, 0x32, 0x2D, 0x7F, 0x04, 0x96, 0xFE, 0x8A, 0xEC, 0xCD, 0x5D,
        0xDE, 0x72, 0x43, 0x1D, 0x18, 0x48, 0xF3, 0x8D, 0x80, 0xC3, 0x4E, 0x42, 0xD7, 0x3D, 0x9C,
        0xB4,
    ];
}
