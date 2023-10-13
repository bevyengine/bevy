//! This example illustrates the [`UIScale`] resource from `bevy_ui`.

use bevy::{prelude::*, text::TextSettings, utils::Duration};

const SCALE_TIME: u64 = 400;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .insert_resource(TextSettings {
            allow_dynamic_font_size: true,
            ..default()
        })
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

fn setup(mut commands: Commands, asset_server: ResMut<AssetServer>) {
    commands.spawn(Camera2dBundle::default());

    let text_style = TextStyle {
        font: asset_server.load("fonts/FiraMono-Medium.ttf"),
        font_size: 16.,
        color: Color::BLACK,
    };

    commands
        .spawn(NodeBundle {
            style: Style {
                width: Val::Percent(50.0),
                height: Val::Percent(50.0),
                position_type: PositionType::Absolute,
                left: Val::Percent(25.),
                top: Val::Percent(25.),
                justify_content: JustifyContent::SpaceAround,
                align_items: AlignItems::Center,
                ..default()
            },
            background_color: Color::ANTIQUE_WHITE.into(),
            ..default()
        })
        .with_children(|parent| {
            parent
                .spawn(NodeBundle {
                    style: Style {
                        width: Val::Px(40.0),
                        height: Val::Px(40.0),
                        ..default()
                    },
                    background_color: Color::RED.into(),
                    ..default()
                })
                .with_children(|parent| {
                    parent.spawn(TextBundle::from_section("Size!", text_style));
                });
            parent.spawn(NodeBundle {
                style: Style {
                    width: Val::Percent(15.0),
                    height: Val::Percent(15.0),
                    ..default()
                },
                background_color: Color::BLUE.into(),
                ..default()
            });
            parent.spawn(ImageBundle {
                style: Style {
                    width: Val::Px(30.0),
                    height: Val::Px(30.0),
                    ..default()
                },
                image: asset_server.load("branding/icon.png").into(),
                ..default()
            });
        });
}

/// System that changes the scale of the ui when pressing up or down on the keyboard.
fn change_scaling(input: Res<Input<KeyCode>>, mut ui_scale: ResMut<TargetScale>) {
    if input.just_pressed(KeyCode::Up) {
        let scale = (ui_scale.target_scale * 2.0).min(8.);
        ui_scale.set_scale(scale);
        info!("Scaling up! Scale: {}", ui_scale.target_scale);
    }
    if input.just_pressed(KeyCode::Down) {
        let scale = (ui_scale.target_scale / 2.0).max(1. / 8.);
        ui_scale.set_scale(scale);
        info!("Scaling down! Scale: {}", ui_scale.target_scale);
    }
}

#[derive(Resource)]
struct TargetScale {
    start_scale: f64,
    target_scale: f64,
    target_time: Timer,
}

impl TargetScale {
    fn set_scale(&mut self, scale: f64) {
        self.start_scale = self.current_scale();
        self.target_scale = scale;
        self.target_time.reset();
    }

    fn current_scale(&self) -> f64 {
        let completion = self.target_time.percent();
        let multiplier = ease_in_expo(completion as f64);
        self.start_scale + (self.target_scale - self.start_scale) * multiplier
    }

    fn tick(&mut self, delta: Duration) -> &Self {
        self.target_time.tick(delta);
        self
    }

    fn already_completed(&self) -> bool {
        self.target_time.finished() && !self.target_time.just_finished()
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

fn ease_in_expo(x: f64) -> f64 {
    if x == 0. {
        0.
    } else {
        (2.0f64).powf(5. * x - 5.)
    }
}
