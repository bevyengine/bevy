use bevy::prelude::*;

struct AnimatableFloat {
    value: f32,
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
        .with(AnimationSpline {
            spline: Spline::from_vec(vec![
                Key::new(0.0, 0.0, Interpolation::Cosine),
                Key::new(3.0, 1000.0, Interpolation::Cosine),
            ]),
            loop_style: LoopStyle::PingPong,
            ..Default::default()
        });
}

fn update_text(float: &AnimatableFloat, mut text: Mut<Text>) {
    text.value = format!("{:.0}", float.value);
}

fn animate_custom(animator: &AnimationSpline, mut float: Mut<AnimatableFloat>) {
    if let Some(val) = animator.current() {
        float.value = val;
    }
}
