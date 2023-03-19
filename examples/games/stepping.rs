use bevy::app::MainScheduleOrder;
use bevy::ecs::schedule::{NodeId, ScheduleLabel};
use bevy::prelude::*;
use std::collections::HashMap;

/// Plugin to add a stepping UI to an example
#[derive(Default)]
pub struct SteppingPlugin {
    schedule_labels: Vec<Box<dyn ScheduleLabel>>,
}

impl SteppingPlugin {
    pub fn for_schedules(labels: Vec<Box<dyn ScheduleLabel>>) -> SteppingPlugin {
        SteppingPlugin {
            schedule_labels: labels,
        }
    }
}

impl Plugin for SteppingPlugin {
    fn build(&self, app: &mut App) {
        // create and insert our stepping schedule into the main schedule order
        app.init_schedule(Stepping);
        let mut order = app.world.resource_mut::<MainScheduleOrder>();
        order.insert_after(Update, Stepping);

        // add our startup & stepping systems
        app.insert_resource(State {
            schedule_labels: self.schedule_labels.clone(),
            status: Status::Init,
            system_text_map: HashMap::new(),
            last_system_ids: Vec::new(),
        })
        .add_systems(
            Stepping,
            (
                build_ui.run_if(not(ui_ready)),
                handle_input.run_if(ui_ready),
                update_ui.run_if(ui_ready),
            )
                .ignore_stepping(),
        );
    }
}

/// Independent [`Schedule`] for stepping systems.
///
/// The stepping systems must run in their own schedule to be able to inspect
/// all the other schedules in the [`App`].  This is because the currently
/// executing schedule is removed from the [`Schedules`] resource while it is
/// being run.
#[derive(Debug, Hash, PartialEq, Eq, Clone, ScheduleLabel)]
struct Stepping;

#[derive(Debug)]
enum Status {
    Init,
    Run,
    Step(usize),
}

/// Struct for maintaining stepping state
#[derive(Resource, Debug)]
struct State {
    // schedules that will be stepped
    schedule_labels: Vec<Box<dyn ScheduleLabel>>,

    // map of Schedule/NodeId to TextSection index in UI entity
    // This is used to draw the position indicator as we step
    system_text_map: HashMap<(Box<dyn ScheduleLabel>, NodeId), usize>,

    // keep track of the last system NodeId in each schedule; we use this to
    // know when to switch to the next schedule when stepping
    last_system_ids: Vec<NodeId>,

    // status of the stepping plugin
    status: Status,
}

/// condition to check if the stepping UI has been constructed
fn ui_ready(state: Res<State>) -> bool {
    match state.status {
        Status::Init => false,
        _ => true,
    }
}

const FONT_SIZE: f32 = 20.0;
const FONT_COLOR: Color = Color::rgb(0.2, 0.2, 0.2);
const FONT_BOLD: &str = "fonts/FiraSans-Bold.ttf";
const FONT_MEDIUM: &str = "fonts/FiraMono-Medium.ttf";

#[derive(Component)]
struct SteppingUi;

fn build_ui(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    schedules: Res<Schedules>,
    mut state: ResMut<State>,
) {
    let mut text_sections = Vec::new();
    let mut last_systems = Vec::new();
    let mut text_map = HashMap::new();

    // go through the supplied labels and construct a list of systems for each
    // label
    for label in &state.schedule_labels {
        info!("getting schedule {:?}", label);
        let schedule = schedules.get(&**label).unwrap();
        let mut last_system: Option<NodeId> = None;
        text_sections.push(TextSection::new(
            format!("{:?}\n", label),
            TextStyle {
                font: asset_server.load(FONT_BOLD),
                font_size: FONT_SIZE,
                color: FONT_COLOR,
            },
        ));
        for (node_id, system) in schedule.ordered_systems() {
            // skip any system that doesn't permit stepping
            if !schedule.system_permits_stepping(node_id) {
                debug!(
                    "stepping disabled for {:?}/{}",
                    label,
                    schedule.system_at(node_id).unwrap().name().to_string()
                );
                continue;
            }

            text_map.insert((label.clone(), node_id), text_sections.len());
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

            last_system = Some(node_id);
        }

        match last_system {
            Some(id) => last_systems.push(id),
            // It's possible that the [`Stepping`] schedule ran before one of
            // the schedules we're going to be stepping.  In this case, the
            // other schedule will not yet have its `SystemSchedule` built
            // (this happens the first time the schedule runs).  So let's
            // return, and try again later.
            //
            // NOTE: This will cause problems with schedules that are very
            // rarely run.
            None => {
                info!("schedule {:?} has no systems; delaying ui creation", label);
                return;
            }
        }
    }
    state.last_system_ids = last_systems;
    state.system_text_map = text_map;
    state.status = Status::Run;

    commands.spawn((
        SteppingUi,
        TextBundle {
            text: Text::from_sections(text_sections),
            style: Style {
                position_type: PositionType::Absolute,
                top: Val::Percent(50.0),
                left: Val::Percent(25.0),
                padding: UiRect::all(Val::Px(10.0)),
                ..default()
            },
            background_color: BackgroundColor(Color::rgba(1.0, 1.0, 1.0, 0.33)),
            ..default()
        },
    ));

    // stepping description box
    commands
        .spawn((TextBundle::from_sections([TextSection::new(
            "Press ` to toggle stepping mode (S: step system, Space: step frame)",
            TextStyle {
                font: asset_server.load("fonts/FiraMono-Medium.ttf"),
                font_size: 15.0,
                color: FONT_COLOR,
            },
        )])
        .with_style(Style {
            position_type: PositionType::Absolute,
            bottom: Val::Px(5.0),
            left: Val::Px(5.0),
            ..default()
        }),))
        .insert(Visibility::Hidden);
}

fn handle_input(
    keyboard_input: Res<Input<KeyCode>>,
    schedules: Res<Schedules>,
    mut state: ResMut<State>,
    mut schedule_events: EventWriter<bevy::ecs::schedule::ScheduleEvent>,
) {
    use bevy::ecs::schedule::ScheduleEvent::*;

    // grave key to toggle stepping mode for the FixedUpdate schedule
    if keyboard_input.just_pressed(KeyCode::Grave) {
        match state.status {
            Status::Init => return,
            Status::Run => {
                state.status = Status::Step(0);
                for label in &state.schedule_labels {
                    schedule_events.send(EnableStepping(label.clone()));
                }
            }
            Status::Step(_) => {
                state.status = Status::Run;
                for label in &state.schedule_labels {
                    schedule_events.send(DisableStepping(label.clone()));
                }
            }
        }
    }

    // check if we're stepping, and if so grab the schedule index
    let index = match state.status {
        Status::Step(i) => i,
        _ => return,
    };

    // space key will step the remainder of this frame
    if keyboard_input.just_pressed(KeyCode::Space) {
        info!("stepping frame");
        for i in index..state.schedule_labels.len() {
            let label = &state.schedule_labels[i];
            schedule_events.send(StepFrame(label.clone()));
        }
        state.status = Status::Step(0);
        return;
    }

    // If they didn't request a system step, we're done here
    if !keyboard_input.just_pressed(KeyCode::S) {
        return;
    }

    // grab the label, schedule, and node id for the system we're stepping
    let label = &state.schedule_labels[index];
    let schedule = schedules.get(&**label).unwrap();
    let node_id = match schedule.next_step_system_node_id() {
        Some(id) => id,
        None => return,
    };

    info!(
        "stepping {:?}: {}",
        label,
        schedule.next_step_system_name().unwrap().to_string()
    );
    schedule_events.send(StepSystem(label.clone()));

    // if we're running the last system in this schedule, update status to
    // point at the next system
    if node_id == state.last_system_ids[index] {
        let index = index + 1;
        if index >= state.schedule_labels.len() {
            state.status = Status::Step(0);
        } else {
            state.status = Status::Step(index);
        }
    }
}

fn update_ui(
    mut commands: Commands,
    state: Res<State>,
    mut ui: Query<(Entity, &mut Text, &Visibility), With<SteppingUi>>,
    schedules: Res<Schedules>,
) {
    // If we're stepping, ensure the UI is visibile, and grab the current
    // schedule label.  Otherwise, hide the UI and just return.
    let (entity, mut text, vis) = ui.single_mut();
    let index = match state.status {
        Status::Step(index) => {
            if vis == Visibility::Hidden {
                commands.entity(entity).insert(Visibility::Inherited);
            };
            index
        }
        _ => {
            // ensure the UI is hidden if we're not stepping
            if vis != Visibility::Hidden {
                commands.entity(entity).insert(Visibility::Hidden);
            }
            return;
        }
    };
    let label = &state.schedule_labels[index];

    let schedule = schedules.get(&**label).unwrap();
    let node_id = match schedule.next_step_system_node_id() {
        Some(id) => id,
        None => return,
    };

    for ((l, id), index) in &state.system_text_map {
        let mark = if l == label && *id == node_id {
            "-> "
        } else {
            "   "
        };
        text.sections[*index].value = mark.to_string();
    }
}
