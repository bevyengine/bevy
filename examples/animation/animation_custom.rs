use bevy::prelude::*;

struct AnimatableFloat {
    value: f32,
}

impl Animatable for AnimatableFloat {
    type Splines = SplinesOne;
    fn anim_tracks() -> AnimTracks {
        AnimTracks::Struct(vec![("value", Color::WHITE)])
    }

    fn set_values(&mut self, values: Vec<f32>) {
        self.value = *values.get(0).unwrap();
    }

    fn values(&self) -> Vec<f32> {
        vec![self.value]
    }
}

fn main() {
    App::build()
        .add_default_plugins()
        .add_startup_system(setup.system())
        .add_system(update_text.system())
        .add_system(animate_custom.system())
        .run();
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    let font: Handle<Font> = asset_server.load("assets/fonts/FiraSans-Bold.ttf").unwrap();
    commands
        .spawn(UiCameraComponents::default())
        .spawn(TextComponents {
            text: Text {
                font,
                value: "Unset".to_string(),
                style: TextStyle {
                    font_size: 30.0,
                    color: Color::WHITE,
                },
                ..Default::default()
            },
            ..Default::default()
        })
        .with(AnimatableFloat { value: 0.0 })
        .with(Animator::<AnimatableFloat> {
            direction: AnimationLoop::PingPong,
            splines: SplinesOne(Spline::from_vec(vec![
                Key::new(0.0, 0.0, Interpolation::Cosine),
                Key::new(20.0, 1000.0, Interpolation::Cosine),
            ])),
            ..Default::default()
        });
}

fn update_text(float: &AnimatableFloat, mut text: Mut<Text>) {
    text.value = format!("{:.0}", float.value);
}

fn animate_custom(
    time: Res<Time>,
    mut animator: Mut<Animator<AnimatableFloat>>,
    mut float: Mut<AnimatableFloat>,
) {
    animator.progress(&mut float, time.delta);
}
