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
            stepping.add_schedule(label.dyn_clone());
        }
        assert!(!stepping.is_enabled());
        app.insert_resource(stepping);

        // add our startup & stepping systems
        app.insert_resource(State {
            ui_top: self.top,
            ui_left: self.left,
            schedule_text_map: HashMap::new(),
        })
        .add_systems(
            Debug,
            (
                build_ui.run_if(not(initialized)),
                handle_input,
                update_ui.run_if(initialized),
            )
                .chain(),
        );
    }
}

/// Struct for maintaining stepping state
#[derive(Resource, Debug)]
struct State {
    // map of Schedule/NodeId to TextSection index in UI entity
    // This is used to draw the position indicator as we step
    schedule_text_map: HashMap<BoxedScheduleLabel, Vec<usize>>,

    // ui positioning
    ui_top: Val,
    ui_left: Val,
}

/// condition to check if the stepping UI has been constructed
fn initialized(state: Res<State>) -> bool {
    !state.schedule_text_map.is_empty()
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
    mut stepping: ResMut<Stepping>,
    mut state: ResMut<State>,
) {
    debug_assert!(state.schedule_text_map.is_empty());

    let mut text_sections = Vec::new();
    let mut always_run = Vec::new();

    let schedule_order = match stepping.schedules() {
        Ok(s) => s,
        Err(_) => return,
    };

    // go through the stepping schedules and construct a list of systems for
    // each label
    for label in schedule_order {
        println!("schedule {:?}", label);
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

        for (node_id, system) in systems {
            if system.name().starts_with("bevy") {
                always_run.push((label.dyn_clone(), node_id));
                println!("skipping {:?}", system.name());
                continue;
            }

            system_index.push(text_sections.len());
            text_sections.push(TextSection::new(
                "   ",
                TextStyle {
                    font: asset_server.load(FONT_MEDIUM),
                    font_size: FONT_SIZE,
                    color: FONT_COLOR,
                },
            ));

            println!("found system: {:?}/{}\n", label, system.name());
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

    for (label, node) in always_run.drain(..) {
        stepping.always_run_node(label, node);
    }

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

fn handle_input(keyboard_input: Res<Input<KeyCode>>, mut stepping: ResMut<Stepping>) {
    if keyboard_input.just_pressed(KeyCode::Slash) {
        debug!("stepping: {:#?}", stepping);
    }
    // grave key to toggle stepping mode for the FixedUpdate schedule
    if keyboard_input.just_pressed(KeyCode::Grave) {
        if stepping.is_enabled() {
            stepping.disable();
            debug!("disabled stepping");
        } else {
            stepping.enable();
            debug!("enabled stepping");
        }
    }

    if !stepping.is_enabled() {
        return;
    }

    // space key will step the remainder of this frame
    if keyboard_input.just_pressed(KeyCode::Space) {
        debug!("continue");
        stepping.continue_frame();
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

    // ensure the UI is only visible when stepping is enabled
    let (ui, mut text, vis) = ui.single_mut();
    match (vis, stepping.is_enabled()) {
        (Visibility::Hidden, true) => {
            commands.entity(ui).insert(Visibility::Inherited);
        }
        (Visibility::Hidden, false) => (),
        (_, true) => (),
        (_, false) => {
            commands.entity(ui).insert(Visibility::Hidden);
        }
    }

    // if we're not stepping, there's nothing more to be done here.
    if !stepping.is_enabled() {
        return;
    }

    let cursor = stepping.cursor();
    for (schedule, label) in stepping.schedules().unwrap().iter().enumerate() {
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
