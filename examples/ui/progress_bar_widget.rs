//! This example illustrates how to setup and use the ProgressBar widget.
//! Any Node that has a `ProgressBar` component *and* an immediate child node
//! with the `ProgressBarInner` component will be considered a ProgressBar-widget.

use bevy::{
    prelude::*,
    ui::widget::{LoadingBarInner, ProgressBarWidget}, math::map_range,
};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .insert_resource(Progress::Completed(2.0))
        .add_startup_system(setup)
        .add_system(update_progress_state)
        .add_system(set_widget_progress.after(update_progress_state))
        .add_system(update_widget_text.after(set_widget_progress))
        .run();
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    let canvas_background: Color = Color::rgba_u8(19, 21, 22, 255);
    let progress_bar_background: Color = Color::rgba_u8(29, 31, 33, 255);
    let progress_bar_foreground: Color = Color::rgba_u8(50, 104, 159, 255);
    let text_color: Color = Color::rgba_u8(197, 198, 190, 255);
    let font = asset_server.load("fonts/FiraMono-Medium.ttf");

    // ui camera
    commands.spawn(Camera2dBundle::default());

    // background that fills the entire viewport
    commands
        .spawn(NodeBundle {
            style: Style {
                size: Size::new(Val::Percent(100.0), Val::Percent(100.0)),
                // horizontally center children nodes
                justify_content: JustifyContent::Center,
                // vertically center children nodes
                align_items: AlignItems::Center,
                ..default()
            },
            background_color: canvas_background.into(),
            ..default()
        })
        .with_children(|root| {
            // Progress bar
            root.spawn(NodeBundle {
                style: Style {
                    size: Size::new(Val::Percent(50.0), Val::Px(50.0)),
                    justify_content: JustifyContent::FlexStart,
                    align_items: AlignItems::Center,
                    ..default()
                },
                background_color: progress_bar_background.into(),
                ..default()
            })
            .insert(ProgressBarWidget::new(0.0, 0., 1.))
            .with_children(|outer| {
                // Inner node of the progress bar that will change size as progress changes.
                outer
                    .spawn(NodeBundle {
                        style: Style {
                            size: Size::new(Val::Percent(50.0), Val::Percent(100.0)),
                            position_type: PositionType::Absolute,
                            ..default()
                        },
                        background_color: progress_bar_foreground.into(),
                        ..default()
                    })
                    .insert(LoadingBarInner);
                outer.spawn(TextBundle {
                    text: Text::from_section(
                        "Loading Bar",
                        TextStyle {
                            font: font.clone(),
                            font_size: 20.0,
                            color: text_color,
                        },
                    ),
                    style: Style {
                        margin: UiRect::all(Val::Px(10.0)),
                        ..default()
                    },
                    ..default()
                });
            });
        });
}

const LOAD_DURATION: f32 = 3.0;
const COMPLETE_DURATION: f32 = 1.5;

#[derive(Resource, Clone, Copy, Debug)]
enum Progress {
    Loading(f32),
    Completed(f32),
}

/// This is a helper system to mimic some changing state.
/// It will "Load" for a time until it reaches completed,
/// then it will stay on the "Completed"-state for a time before resetting.
fn update_progress_state(mut progress: ResMut<Progress>, time: Res<Time>) {
    let elapsed_time = match *progress {
        Progress::Loading(value) => value,
        Progress::Completed(value) => value,
    } + time.delta_seconds();

    *progress = match *progress {
        Progress::Loading(_) => {
            if elapsed_time >= LOAD_DURATION {
                Progress::Completed(0.0)
            } else {
                Progress::Loading(elapsed_time)
            }
        }
        Progress::Completed(_) => {
            if elapsed_time >= COMPLETE_DURATION {
                Progress::Loading(0.0)
            } else {
                Progress::Completed(elapsed_time)
            }
        }
    };
}

/// This is responsible for updating the value of the ProgressBarWidget component.
/// This could be in response to changes in player health values, loading of assets ++.
fn set_widget_progress(mut q: Query<&mut ProgressBarWidget>, progress: Res<Progress>) {
    for mut widget in q.iter_mut() {
        let current_progress = match *progress {
            Progress::Loading(value) => map_range(value, (0., LOAD_DURATION), (0., 1.)),
            Progress::Completed(_) => 1.,
        };
        widget.set_progress(current_progress);
    }
}

/// Updates the text of the progress-bar.
fn update_widget_text(
    widgets: Query<(&ProgressBarWidget, &Children), Changed<ProgressBarWidget>>,
    mut q: Query<&mut Text, With<Parent>>,
) {
    for (widget, children) in widgets.iter() {
        for child in children.iter() {
            if let Ok(mut text) = q.get_mut(*child) {
                let progress = widget.get_progress();
                if progress >= 1.0 {
                    text.sections[0].value = format!("Loading complete!");
                } else {
                    text.sections[0].value = format!("Loading: {:.2}%", progress * 100.0);
                }
            }
        }
    }
}
