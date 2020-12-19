use bevy::prelude::*;

struct AnimatableFloat {
    value: f32,
}

fn main() {
    App::build()
        .add_plugins(DefaultPlugins)
        .add_startup_system(setup)
        .add_system(update_text)
        .add_system(animate_custom)
        .run();
}

fn setup(commands: &mut Commands, asset_server: Res<AssetServer>) {
    let font: Handle<Font> = asset_server.load("fonts/FiraSans-Bold.ttf");
    commands
        .spawn(UiCameraBundle::default())
        .spawn(TextBundle {
            text: Text {
                font,
                value: "Unset".to_string(),
                style: TextStyle {
                    font_size: 30.0,
                    color: Color::WHITE,
                    ..Default::default()
                },
            },
            ..Default::default()
        })
        .with(AnimatableFloat { value: 0.0 })
        .with(AnimationSplineOne {
            spline: Spline::from_vec(vec![
                Key::new(0.0, 0.0, Interpolation::Cosine),
                Key::new(3.0, 1000.0, Interpolation::Cosine),
            ]),
            loop_style: LoopStyle::PingPong,
            ..Default::default()
        });
}

fn update_text(mut q: Query<(&AnimatableFloat, &mut Text)>) {
    for (float, mut text) in q.iter_mut() {
        text.value = format!("{:.0}", float.value);
    }
}

fn animate_custom(mut q: Query<(&AnimationSplineOne, &mut AnimatableFloat)>) {
    for (animator, mut float) in q.iter_mut() {
        if let Some(val) = animator.current() {
            float.value = val;
        }
    }
}
