use bevy::{app::MainScheduleOrder, ecs::schedule::*, prelude::*};
use std::collections::HashMap;

/// Independent [`Schedule`] for stepping systems.
///
/// The stepping systems must run in their own schedule to be able to inspect
/// all the other schedules in the [`App`].  This is because the currently
/// executing schedule is removed from the [`Schedules`] resource while it is
/// being run.
#[derive(Debug, Hash, PartialEq, Eq, Clone, ScheduleLabel)]
struct Debug;

/// Plugin to add a stepping UI to an example
#[derive(Default)]
pub struct SteppingPlugin {
    schedule_labels: Vec<Box<dyn ScheduleLabel>>,
    top: Val,
    left: Val,
}

impl SteppingPlugin {
    /// Initialize the plugin to step the schedules specified in `labels`
    pub fn for_schedules(labels: Vec<Box<dyn ScheduleLabel>>) -> SteppingPlugin {
        SteppingPlugin {
            schedule_labels: labels,
            ..default()
        }
    }

    /// Set the location of the stepping UI when activated
    pub fn at(self, left: Val, top: Val) -> SteppingPlugin {
        SteppingPlugin { top, left, ..self }
    }
}

impl Plugin for SteppingPlugin {
    fn build(&self, app: &mut App) {
        // create and insert our debug schedule into the main schedule order.
        // We need an independent schedule so we have access to all other
        // schedules through the `Stepping` resource
        app.init_schedule(Debug);
        let mut order = app.world.resource_mut::<MainScheduleOrder>();
        order.insert_after(Update, Debug);

        // create our stepping resource
        let mut stepping = Stepping::new();
        for label in self.schedule_labels.iter() {
            stepping.add_schedule(label.clone());
        }
        app.insert_resource(stepping);

        // add our startup & stepping systems
        app.insert_resource(State {
            ui_top: self.top,
            ui_left: self.left,
            status: Status::Init,
            schedule_text_map: HashMap::new(),
        })
        .add_systems(
            Debug,
            (
                build_ui.run_if(not(initialized)),
                handle_input.run_if(initialized),
                update_ui.run_if(initialized),
            ),
        );
    }
}

#[derive(Debug, PartialEq)]
enum Status {
    // initial state, waiting for build_ui() to complete successfully
    Init,
    // game running normally
    Run,
    // stepping enabled; value is index into State.schedule_labels
    Step,
}

/// Struct for maintaining stepping state
#[derive(Resource, Debug)]
struct State {
    // map of Schedule/NodeId to TextSection index in UI entity
    // This is used to draw the position indicator as we step
    schedule_text_map: HashMap<BoxedScheduleLabel, Vec<usize>>,

    // status of the stepping plugin
    status: Status,

    // ui positioning
    ui_top: Val,
    ui_left: Val,
}

/// condition to check if the stepping UI has been constructed
fn initialized(state: Res<State>) -> bool {
    !matches!(state.status, Status::Init)
}

const FONT_SIZE: f32 = 20.0;
const FONT_COLOR: Color = Color::rgb(0.2, 0.2, 0.2);
const FONT_BOLD: &str = "fonts/FiraSans-Bold.ttf";
const FONT_MEDIUM: &str = "fonts/FiraMono-Medium.ttf";

#[derive(Component)]
struct SteppingUi;

/// Construct the stepping UI elements from the [`Schedules`] resource.
///
/// This system may run multiple times before constructing the UI as all of the
/// data may not be available on the first run of the system.  This happens if
/// one of the stepping schedules has not yet been run.
fn build_ui(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    schedules: Res<Schedules>,
    stepping: Res<Stepping>,
    mut state: ResMut<State>,
) {
    let mut text_sections = Vec::new();

    // go through the stepping schedules and construct a list of systems for
    // each label
    for label in stepping.schedules() {
        let schedule = schedules.get(&**label).unwrap();
        text_sections.push(TextSection::new(
            format!("{:?}\n", label),
            TextStyle {
                font: asset_server.load(FONT_BOLD),
                font_size: FONT_SIZE,
                color: FONT_COLOR,
            },
        ));

        // grab the list of systems in the schedule, in the order the
        // single-threaded executor would run them.
        let systems = match schedule.systems() {
            Ok(iter) => iter,
            Err(_) => return,
        };

        let mut system_index = Vec::new();

        for (_node_id, system) in systems {
            system_index.push(text_sections.len());
            text_sections.push(TextSection::new(
                "   ",
                TextStyle {
                    font: asset_server.load(FONT_MEDIUM),
                    font_size: FONT_SIZE,
                    color: FONT_COLOR,
                },
            ));

            text_sections.push(TextSection::new(
                format!("{}\n", system.name()),
                TextStyle {
                    font: asset_server.load(FONT_MEDIUM),
                    font_size: FONT_SIZE,
                    color: FONT_COLOR,
                },
            ));
        }

        state.schedule_text_map.insert(label.clone(), system_index);
    }

    state.status = Status::Run;

    commands.spawn((
        SteppingUi,
        TextBundle {
            text: Text::from_sections(text_sections),
            style: Style {
                position_type: PositionType::Absolute,
                top: state.ui_top,
                left: state.ui_left,
                padding: UiRect::all(Val::Px(10.0)),
                ..default()
            },
            background_color: BackgroundColor(Color::rgba(1.0, 1.0, 1.0, 0.33)),
            visibility: Visibility::Hidden,
            ..default()
        },
    ));

    // stepping description box
    commands.spawn((TextBundle::from_sections([TextSection::new(
        "Press ` to toggle stepping mode (S: step system, Space: step frame)",
        TextStyle {
            font: asset_server.load(FONT_MEDIUM),
            font_size: 15.0,
            color: FONT_COLOR,
        },
    )])
    .with_style(Style {
        position_type: PositionType::Absolute,
        bottom: Val::Px(5.0),
        left: Val::Px(5.0),
        ..default()
    }),));
}

fn handle_input(
    keyboard_input: Res<Input<KeyCode>>,
    mut stepping: ResMut<Stepping>,
    mut state: ResMut<State>,
) {
    // grave key to toggle stepping mode for the FixedUpdate schedule
    if keyboard_input.just_pressed(KeyCode::Grave) {
        match state.status {
            Status::Init => return,
            Status::Run => {
                state.status = Status::Step;
                stepping.enable();
            }
            Status::Step => {
                state.status = Status::Run;
                stepping.disable();
            }
        }
    }

    if state.status == Status::Run {
        return;
    }

    // space key will step the remainder of this frame
    if keyboard_input.just_pressed(KeyCode::Space) {
        debug!("continue");
        stepping.continue_frame();
        return;
    } else if keyboard_input.just_pressed(KeyCode::S) {
        debug!("stepping frame");
        stepping.step_frame();
    }
}

fn update_ui(
    mut commands: Commands,
    state: Res<State>,
    stepping: Res<Stepping>,
    mut ui: Query<(Entity, &mut Text, &Visibility), With<SteppingUi>>,
) {
    if ui.is_empty() {
        return;
    }

    // If we're stepping, ensure the UI is visibile, and grab the current
    // schedule label.  Otherwise, hide the UI and just return.
    let (entity, mut text, vis) = ui.single_mut();
    match state.status {
        Status::Step => {
            if vis == Visibility::Hidden {
                commands.entity(entity).insert(Visibility::Inherited);
            };
        }
        _ => {
            // ensure the UI is hidden if we're not stepping
            if vis != Visibility::Hidden {
                commands.entity(entity).insert(Visibility::Hidden);
            }
            return;
        }
    };

    let cursor = stepping.cursor();
    for (schedule, label) in stepping.schedules().iter().enumerate() {
        for (system, text_index) in state
            .schedule_text_map
            .get(label)
            .unwrap()
            .iter()
            .enumerate()
        {
            let here = schedule == cursor.schedule && system == cursor.system;
            debug!(
                "schedule {:?} ({}), system {}, cursor {:?}",
                label, schedule, system, here
            );
            let mark = if schedule == cursor.schedule && system == cursor.system {
                "-> "
            } else {
                "   "
            };
            text.sections[*text_index].value = mark.to_string();
        }
    }
}
