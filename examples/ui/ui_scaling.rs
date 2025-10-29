//! This example illustrates the [`UiScale`] resource from `bevy_ui`.

use bevy::{color::palettes::css::*, prelude::*};
use core::time::Duration;

const SCALE_TIME: u64 = 400;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .insert_resource(TargetScale {
            start_scale: 1.0,
            target_scale: 1.0,
            target_time: Timer::new(Duration::from_millis(SCALE_TIME), TimerMode::Once),
        })
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (change_scaling, apply_scaling.after(change_scaling)),
        )
        .run();
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn(Camera2d);

    let text_font = TextFont {
        font_size: 13.,
        ..default()
    };

    commands
        .spawn((
            Node {
                width: percent(50),
                height: percent(50),
                position_type: PositionType::Absolute,
                left: percent(25),
                top: percent(25),
                justify_content: JustifyContent::SpaceAround,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(ANTIQUE_WHITE.into()),
        ))
        .with_children(|parent| {
            parent
                .spawn((
                    Node {
                        width: px(40),
                        height: px(40),
                        ..default()
                    },
                    BackgroundColor(RED.into()),
                ))
                .with_children(|parent| {
                    parent.spawn((Text::new("Size!"), text_font, TextColor::BLACK));
                });
            parent.spawn((
                Node {
                    width: percent(15),
                    height: percent(15),
                    ..default()
                },
                BackgroundColor(BLUE.into()),
            ));
            parent.spawn((
                ImageNode::new(asset_server.load("branding/icon.png")),
                Node {
                    width: px(30),
                    height: px(30),
                    ..default()
                },
            ));
        });
}

/// System that changes the scale of the ui when pressing up or down on the keyboard.
fn change_scaling(input: Res<ButtonInput<KeyCode>>, mut ui_scale: ResMut<TargetScale>) {
    if input.just_pressed(KeyCode::ArrowUp) {
        let scale = (ui_scale.target_scale * 2.0).min(8.);
        ui_scale.set_scale(scale);
        info!("Scaling up! Scale: {}", ui_scale.target_scale);
    }
    if input.just_pressed(KeyCode::ArrowDown) {
        let scale = (ui_scale.target_scale / 2.0).max(1. / 8.);
        ui_scale.set_scale(scale);
        info!("Scaling down! Scale: {}", ui_scale.target_scale);
    }
}

#[derive(Resource)]
struct TargetScale {
    start_scale: f32,
    target_scale: f32,
    target_time: Timer,
}

impl TargetScale {
    fn set_scale(&mut self, scale: f32) {
        self.start_scale = self.current_scale();
        self.target_scale = scale;
        self.target_time.reset();
    }

    fn current_scale(&self) -> f32 {
        let completion = self.target_time.fraction();
        let t = ease_in_expo(completion);
        self.start_scale.lerp(self.target_scale, t)
    }

    fn tick(&mut self, delta: Duration) -> &Self {
        self.target_time.tick(delta);
        self
    }

    fn already_completed(&self) -> bool {
        self.target_time.is_finished() && !self.target_time.just_finished()
    }
}

fn apply_scaling(
    time: Res<Time>,
    mut target_scale: ResMut<TargetScale>,
    mut ui_scale: ResMut<UiScale>,
) {
    if target_scale.tick(time.delta()).already_completed() {
        return;
    }

    ui_scale.0 = target_scale.current_scale();
}

fn ease_in_expo(x: f32) -> f32 {
    if x == 0. {
        0.
    } else {
        ops::powf(2.0f32, 5. * x - 5.)
    }
}
