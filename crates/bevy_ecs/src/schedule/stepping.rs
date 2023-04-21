use bevy_utils::tracing::warn;
use fixedbitset::FixedBitSet;
use std::any::TypeId;
use std::collections::HashMap;

use crate::{
    schedule::{BoxedScheduleLabel, NodeId, Schedule, ScheduleLabel},
    system::{IntoSystem, ResMut, Resource, System},
};
use bevy_utils::thiserror::Error;

use crate as bevy_ecs;

#[derive(Debug, Default, PartialEq, Eq, Copy, Clone)]
enum Action {
    /// Stepping is disabled; run all systems
    #[default]
    RunAll,

    /// Stepping is enabled, but we're only running required systems this frame
    Waiting,

    /// Stepping is enabled; run all systems until the end of the frame, or
    /// until we encounter a system marked with [`SystemBehavior::Break`] or all
    /// systems in the frame have run.
    Continue,

    /// stepping is enabled; only run the next system in our step list
    Step,
}

#[derive(Debug, Copy, Clone)]
enum SystemBehavior {
    AlwaysRun,
    NeverRun,
    Continue,
    Break,
}

// schedule_order index, and schedule start point
#[derive(Debug, Default, Clone)]
pub struct Cursor {
    pub schedule: usize,
    pub system: usize,
}

enum SystemIdentifier {
    Type(TypeId),
    #[allow(dead_code)]
    Node(NodeId),
}

enum Update {
    SetAction(Action),
    AddSchedule(BoxedScheduleLabel),
    RemoveSchedule(BoxedScheduleLabel),
    ClearSchedule(BoxedScheduleLabel),
    SetBehavior(BoxedScheduleLabel, SystemIdentifier, SystemBehavior),
    ClearBehavior(BoxedScheduleLabel, SystemIdentifier),
}

#[derive(Error, Debug)]
#[error("not available until all configured schedules have been run; try again next frame")]
pub struct NotReady;

#[derive(Resource, Default)]
pub struct Stepping {
    // [`ScheduleState`] for each [`Schedule`] with stepping enabled
    schedule_states: HashMap<BoxedScheduleLabel, ScheduleState>,

    // dynamically generated [`Schedule`] order
    schedule_order: Vec<BoxedScheduleLabel>,

    // current position in the stepping frame
    cursor: Cursor,

    // index in [`schedule_order`] of the last schedule to call `build_skip_list`
    previous_schedule: Option<usize>,

    // Action to perform during this render frame
    action: Action,

    // Updates apply at the start of the next render frame
    updates: Vec<Update>,
}

impl std::fmt::Debug for Stepping {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Stepping {{ action: {:?}, schedules: {:?}, order: {:?}",
            self.action,
            self.schedule_states.keys(),
            self.schedule_order
        )?;
        if self.action != Action::RunAll {
            let Cursor { schedule, system } = self.cursor;
            match self.schedule_order.get(schedule) {
                Some(label) => write!(f, "cursor: {:?}[{}], ", label, system)?,
                None => write!(f, "cursor: None, ")?,
            };
        }
        write!(f, "}}")
    }
}

impl Stepping {
    pub fn new() -> Self {
        Stepping::default()
    }

    pub fn begin_frame(stepping: Option<ResMut<Self>>) {
        if let Some(mut stepping) = stepping {
            stepping.next_frame();
        }
    }

    /// Return the list of schedules with stepping enabled in the order
    /// they are executed in.
    pub fn schedules(&self) -> Result<&Vec<BoxedScheduleLabel>, NotReady> {
        /*
        println!(
            "schedule_order {:?}, schedule_states {:?}",
            self.schedule_order.len(),
            self.schedule_states.len()
        ); */
        if self.schedule_order.len() == self.schedule_states.len() {
            Ok(&self.schedule_order)
        } else {
            Err(NotReady)
        }
    }

    /// Return our current position within the stepping frame
    ///
    /// NOTE: This function **will** return `None` during normal execution with
    /// stepping enabled.  This can happen at the end of the stepping frame
    /// after the last system has been run, but before the start of the next
    /// render frame.
    pub fn cursor(&self) -> Option<(BoxedScheduleLabel, NodeId)> {
        if self.action == Action::RunAll {
            return None;
        }
        let label = match self.schedule_order.get(self.cursor.schedule) {
            None => return None,
            Some(label) => label,
        };
        let state = match self.schedule_states.get(label) {
            None => return None,
            Some(state) => state,
        };
        state
            .node_ids
            .get(self.cursor.system)
            .map(|node_id| (label.dyn_clone(), *node_id))
    }

    /// Enable stepping for the provided schedule
    pub fn add_schedule(&mut self, schedule: impl ScheduleLabel) -> &mut Self {
        self.updates.push(Update::AddSchedule(schedule.dyn_clone()));
        self
    }

    /// disable stepping for the provided schedule
    pub fn remove_schedule(&mut self, schedule: impl ScheduleLabel) -> &mut Self {
        self.updates
            .push(Update::RemoveSchedule(Box::new(schedule)));
        self
    }

    /// clear behavior set for all systems in the provided [`Schedule`]
    pub fn clear_schedule(&mut self, schedule: impl ScheduleLabel) -> &mut Self {
        self.updates.push(Update::ClearSchedule(Box::new(schedule)));
        self
    }

    /// Begin stepping at the start of the next frame
    pub fn enable(&mut self) -> &mut Self {
        self.updates.push(Update::SetAction(Action::Waiting));
        self
    }

    /// Disable stepping, resume normal systems execution
    pub fn disable(&mut self) -> &mut Self {
        self.updates.push(Update::SetAction(Action::RunAll));
        self
    }

    /// Check if stepping is enabled
    pub fn is_enabled(&self) -> bool {
        self.action != Action::RunAll
    }

    /// run the next system
    pub fn step_frame(&mut self) -> &mut Self {
        self.updates.push(Update::SetAction(Action::Step));
        self
    }

    /// continue stepping until the end of the frame or a breakpoint is hit
    pub fn continue_frame(&mut self) -> &mut Self {
        self.updates.push(Update::SetAction(Action::Continue));
        self
    }

    /// Ensure this system always runs when stepping is enabled
    ///
    /// Note: if the system is run multiple times in the [`Schedule`], this
    /// will apply for all instances of the system.
    pub fn always_run<Marker>(
        &mut self,
        schedule: impl ScheduleLabel,
        system: impl IntoSystem<(), (), Marker>,
    ) -> &mut Self {
        let type_id = IntoSystem::into_system(system).type_id();
        self.updates.push(Update::SetBehavior(
            schedule.dyn_clone(),
            SystemIdentifier::Type(type_id),
            SystemBehavior::AlwaysRun,
        ));

        self
    }

    /// Ensure this system instance always runs when stepping is enabled
    pub fn always_run_node(&mut self, schedule: impl ScheduleLabel, node: NodeId) -> &mut Self {
        self.updates.push(Update::SetBehavior(
            schedule.dyn_clone(),
            SystemIdentifier::Node(node),
            SystemBehavior::AlwaysRun,
        ));
        self
    }

    /// Ensure this system never runs when stepping is enabled
    pub fn never_run<Marker>(
        &mut self,
        schedule: impl ScheduleLabel,
        system: impl IntoSystem<(), (), Marker>,
    ) -> &mut Self {
        let type_id = IntoSystem::into_system(system).type_id();
        self.updates.push(Update::SetBehavior(
            Box::new(schedule),
            SystemIdentifier::Type(type_id),
            SystemBehavior::NeverRun,
        ));

        self
    }

    /// Ensure this system instance never runs when stepping is enabled
    pub fn never_run_node(&mut self, schedule: impl ScheduleLabel, node: NodeId) -> &mut Self {
        self.updates.push(Update::SetBehavior(
            schedule.dyn_clone(),
            SystemIdentifier::Node(node),
            SystemBehavior::NeverRun,
        ));
        self
    }

    /// Add a breakpoint for system
    pub fn set_breakpoint<Marker>(
        &mut self,
        schedule: impl ScheduleLabel,
        system: impl IntoSystem<(), (), Marker>,
    ) -> &mut Self {
        let type_id = IntoSystem::into_system(system).type_id();
        self.updates.push(Update::SetBehavior(
            Box::new(schedule),
            SystemIdentifier::Type(type_id),
            SystemBehavior::Break,
        ));

        self
    }

    /// Add a breakpoint for system instance
    pub fn set_breakpoint_node(&mut self, schedule: impl ScheduleLabel, node: NodeId) -> &mut Self {
        self.updates.push(Update::SetBehavior(
            schedule.dyn_clone(),
            SystemIdentifier::Node(node),
            SystemBehavior::Break,
        ));
        self
    }

    /// Clear a breakpoint for the system
    pub fn clear_breakpoint<Marker>(
        &mut self,
        schedule: impl ScheduleLabel,
        system: impl IntoSystem<(), (), Marker>,
    ) -> &mut Self {
        self.clear_system(schedule, system);

        self
    }

    /// clear a breakpoint for system instance
    pub fn clear_breakpoint_node(
        &mut self,
        schedule: impl ScheduleLabel,
        node: NodeId,
    ) -> &mut Self {
        self.clear_node(schedule, node);
        self
    }

    /// Clear any behavior set for the system
    pub fn clear_system<Marker>(
        &mut self,
        schedule: impl ScheduleLabel,
        system: impl IntoSystem<(), (), Marker>,
    ) -> &mut Self {
        let type_id = IntoSystem::into_system(system).type_id();
        self.updates.push(Update::ClearBehavior(
            schedule.dyn_clone(),
            SystemIdentifier::Type(type_id),
        ));

        self
    }

    /// clear a breakpoint for system instance
    pub fn clear_node(&mut self, schedule: impl ScheduleLabel, node: NodeId) -> &mut Self {
        self.updates.push(Update::ClearBehavior(
            schedule.dyn_clone(),
            SystemIdentifier::Node(node),
        ));
        self
    }

    /// lookup the first system for the supplied schedule index
    fn first_system_index_for_schedule(&self, index: usize) -> usize {
        let label = match self.schedule_order.get(index) {
            None => return 0,
            Some(label) => label,
        };
        let state = match self.schedule_states.get(label) {
            None => return 0,
            Some(state) => state,
        };
        state.first.unwrap_or(0)
    }

    /// Move the cursor to the start of the first schedule
    fn reset_cursor(&mut self) {
        self.cursor = Cursor {
            schedule: 0,
            system: self.first_system_index_for_schedule(0),
        };
    }

    /// Advance schedule states for the next render frame
    fn next_frame(&mut self) {
        // if stepping is enabled; reset our internal state for the start of
        // the next frame
        if self.action != Action::RunAll {
            self.action = Action::Waiting;
            self.previous_schedule = None;

            // if the cursor passed the last schedule, reset it
            let Cursor {
                schedule: schedule_index,
                ..
            } = self.cursor;
            if schedule_index >= self.schedule_order.len() {
                self.reset_cursor();
            }
        }

        if self.updates.is_empty() {
            return;
        }

        let mut reset_cursor = false;
        for update in self.updates.drain(..) {
            match update {
                Update::SetAction(Action::RunAll) => {
                    self.action = Action::RunAll;
                    reset_cursor = true;
                }
                Update::SetAction(a) => self.action = a,
                Update::AddSchedule(l) => {
                    self.schedule_states.insert(l, ScheduleState::default());
                }
                Update::RemoveSchedule(label) => {
                    self.schedule_states.remove(&label);
                    if let Some(index) = self.schedule_order.iter().position(|l| l == &label) {
                        self.schedule_order.remove(index);
                    }
                    reset_cursor = true;
                }
                Update::ClearSchedule(label) => match self.schedule_states.get_mut(&label) {
                    Some(state) => state.clear_behaviors(),
                    None => {
                        warn!(
                            "stepping is not enabled for schedule {:?}; use
                    `.add_stepping({:?})` to enable stepping",
                            label, label
                        );
                    }
                },
                Update::SetBehavior(label, system, behavior) => {
                    match self.schedule_states.get_mut(&label) {
                        Some(state) => state.set_behavior(system, behavior),
                        None => {
                            warn!(
                                "stepping is not enabled for schedule {:?}; use
                    `.add_stepping({:?})` to enable stepping",
                                label, label
                            );
                        }
                    }
                }
                Update::ClearBehavior(label, system) => {
                    match self.schedule_states.get_mut(&label) {
                        Some(state) => state.clear_behavior(system),
                        None => {
                            warn!(
                                "stepping is not enabled for schedule {:?}; use
                    `.add_stepping({:?})` to enable stepping",
                                label, label
                            );
                        }
                    }
                }
            }
        }

        if reset_cursor {
            self.reset_cursor();
        }
    }

    /// get the list of systems this schedule should skip for this render
    /// frame
    pub fn skipped_systems(&mut self, schedule: &Schedule) -> Option<FixedBitSet> {
        if self.action == Action::RunAll {
            return None;
        }

        // grab the label and state for the schedule if they've been added to
        // stepping
        // XXX Take a second look at this
        let mut found = false;
        for label in self.schedule_states.keys() {
            match schedule.label() {
                Some(l) => {
                    if l == label {
                        found = true;
                    }
                }
                None => (),
            }
        }
        debug_assert!(
            !found
                || self
                    .schedule_states
                    .get_mut(schedule.label().as_ref().unwrap())
                    .is_some()
        );
        let (label, state) = match schedule.label() {
            None => return None,
            Some(label) => (label, self.schedule_states.get_mut(label)?),
        };

        // Stepping is enabled, and this schedule is supposed to be stepped.
        //
        // We need to get this schedule's index in the ordered schedule list.
        // If it's not present, it will be inserted in place after the index
        // of the previous schedule that called this function.
        let index = self.schedule_order.iter().position(|l| l == label);
        let index = match (index, self.previous_schedule) {
            (Some(index), _) => index,
            (None, None) => {
                self.schedule_order.insert(0, label.clone());
                0
            }
            (None, Some(last)) => {
                self.schedule_order.insert(last + 1, label.clone());
                last + 1
            }
        };
        self.previous_schedule = Some(index);
        println!(
            "Stepping::skipped_systems(): cursor {:?}, label {:?}, index {}",
            self.cursor, label, index
        );

        // if the stepping frame cursor is pointing at this schedule, we'll run
        // the schedule with the current stepping action.  If this is not the
        // cursor schedule, we'll run the schedule with the waiting action.
        let Cursor {
            schedule: schedule_index,
            system: start,
        } = self.cursor;
        let (list, next_system) = if index == schedule_index {
            let o = state.skipped_systems(schedule, start, self.action);

            // if we just stepped this schedule, then we'll switch the action
            // to be waiting
            if self.action == Action::Step {
                self.action = Action::Waiting;
            }
            o
        } else {
            // we're not supposed to run any systems in this schedule, so pull
            // the skip list, but ignore any changes it makes to the cursor.
            let (skip, _) = state.skipped_systems(schedule, 0, Action::Waiting);
            (skip, Some(start))
        };

        // update the stepping frame cursor based on if there are any systems
        // remaining to be run in the schedule
        // Note: Don't try to detect the end of the render frame here using the
        // schedule index.  We don't know all schedules have been added to the
        // schedule_order, so only next_frame() knows its safe to reset the
        // cursor.
        match next_system {
            Some(i) => self.cursor.system = i,
            None => {
                let index = schedule_index + 1;
                println!(
                    "Stepping::skipped_systems(): next schedule {} -> {}",
                    schedule_index, index,
                );
                self.cursor = Cursor {
                    schedule: index,
                    system: self.first_system_index_for_schedule(index),
                }
            }
        }

        Some(list)
    }
}

#[derive(Default)]
struct ScheduleState {
    /// per-system [`SystemBehavior`]
    behaviors: HashMap<NodeId, SystemBehavior>,

    /// order of NodeIds in the schedule
    node_ids: Vec<NodeId>,

    /// changes to system behavior that should be applied the next time
    /// [`ScheduleState::progress()`] is called
    updates: HashMap<TypeId, Option<SystemBehavior>>,

    /// This field contains the first steppable system in the schedule.
    first: Option<usize>,
}

impl ScheduleState {
    // set the stepping behavior for a system in this schedule
    fn set_behavior(&mut self, system: SystemIdentifier, behavior: SystemBehavior) {
        self.first = None;
        match system {
            SystemIdentifier::Node(node_id) => {
                self.behaviors.insert(node_id, behavior);
            }
            SystemIdentifier::Type(type_id) => {
                self.updates.insert(type_id, Some(behavior));
            }
        }
    }

    // clear the stepping behavior for a system in this schedule
    fn clear_behavior(&mut self, system: SystemIdentifier) {
        self.first = None;
        match system {
            SystemIdentifier::Node(node_id) => {
                self.behaviors.remove(&node_id);
            }
            SystemIdentifier::Type(type_id) => {
                self.updates.insert(type_id, None);
            }
        }
    }

    // clear all system behaviors
    fn clear_behaviors(&mut self) {
        self.behaviors.clear();
        self.first = None;
    }

    // apply system behavior updates by looking up the node id of the system in
    // the schedule, and updating `systems`
    fn apply_updates(&mut self, schedule: &Schedule) {
        for (node_id, system) in schedule.systems().unwrap() {
            let behavior = self.updates.get(&system.type_id());
            match behavior {
                None => continue,
                Some(None) => {
                    self.behaviors.remove(&node_id);
                }
                Some(Some(behavior)) => {
                    self.behaviors.insert(node_id, *behavior);
                }
            }
        }
        self.updates.clear();

        println!("apply_updates(): {:?}", self.behaviors);
    }

    fn skipped_systems(
        &mut self,
        schedule: &Schedule,
        start: usize,
        mut action: Action,
    ) -> (FixedBitSet, Option<usize>) {
        use std::cmp::Ordering;

        // if our NodeId list hasn't been populated, copy it over from the
        // schedule
        if self.node_ids.len() != schedule.systems_len() {
            self.node_ids = schedule.executable().system_ids.clone();
        }

        // Now that we have the schedule, apply any pending system behavior
        // updates.  The schedule is required to map from system `TypeId` to
        // `NodeId`.
        if !self.updates.is_empty() {
            self.apply_updates(schedule);
        }

        // if we don't have a first system set, set it now
        if self.first.is_none() {
            for (i, (node_id, _)) in schedule.systems().unwrap().enumerate() {
                match self.behaviors.get(&node_id) {
                    Some(SystemBehavior::AlwaysRun) | Some(SystemBehavior::NeverRun) => continue,
                    Some(_) | None => {
                        self.first = Some(i);
                        break;
                    }
                }
            }
        }

        let mut skip = FixedBitSet::with_capacity(schedule.systems_len());
        let mut pos = start;

        for (i, (node_id, system)) in schedule.systems().unwrap().enumerate() {
            let behavior = self
                .behaviors
                .get(&node_id)
                .unwrap_or(&SystemBehavior::Continue);
            println!(
                "skipped_systems(): systems[{}], pos {}, Action::{:?}, Behavior::{:?}, {}",
                i,
                pos,
                action,
                behavior,
                system.name()
            );
            // suppress this clippy warning; we want to keep the arms split to
            // clearly document whats going on.
            #[allow(clippy::match_same_arms)]
            match (action, behavior) {
                // regardless of which action we're performing, if the system
                // is marked as NeverRun, add it to the skip list
                (_, SystemBehavior::NeverRun) => {
                    skip.insert(i);
                    // always advance the position past this sytem
                    if i == pos {
                        pos += 1;
                    }
                }
                // similarly, ignore any system marked as AlwaysRun; they should
                // never be added to the skip list
                (_, SystemBehavior::AlwaysRun) => {
                    // always advance the position past this sytem
                    if i == pos {
                        pos += 1;
                    }
                }
                // if we're waiting, no other systems besides AlwaysRun should
                // be run, so add systems to the skip list
                (Action::Waiting, _) => skip.insert(i),
                // If we're stepping, the remaining behaviors don't matter,
                // we're only going to run the system at our cursor.  Any system
                // not at our cursor is skipped.  Once we encounter the system
                // at the cursor, we'll move the cursor up, and switch to
                // Waiting, to skip remaining systems.
                (Action::Step, _) => match i.cmp(&pos) {
                    Ordering::Less => skip.insert(i),
                    Ordering::Equal => {
                        pos += 1;
                        action = Action::Waiting;
                    }
                    Ordering::Greater => unreachable!(),
                },
                // If we're continuing, and the step behavior is continue, we
                // want to skip any systems prior to our start position.  That's
                // where the stepping frame left off last time we ran anything.
                (Action::Continue, SystemBehavior::Continue) => {
                    if i < start {
                        skip.insert(i);
                    }
                }
                // If we're continuing, and we encounter a breakpoint we want
                // to stop before executing the system.  So we skip this system,
                // and set the action to waiting.  Note that we do not move the
                // cursor.  As a result, the next time we continue, if the
                // first system is a Break we run it anyway.  This allows the
                // user to continue, hit a breakpoint to stop before running
                // the system, then continue again to run the system with the
                // breakpoint.
                (Action::Continue, SystemBehavior::Break) => {
                    if i != start {
                        debug_assert!(pos == i);
                        skip.insert(i);
                        action = Action::Waiting;
                    }
                }
                // should have never gotten into this method if stepping is
                // disabled
                (Action::RunAll, _) => unreachable!(),
            }

            if i == pos && action != Action::Waiting {
                pos += 1;
            }
        }

        if pos >= schedule.systems_len() {
            (skip, None)
        } else {
            (skip, Some(pos))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::prelude::*;
    use crate::{schedule::ScheduleLabel, world::World};

    pub use crate as bevy_ecs;

    #[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash)]
    struct TestSchedule;

    #[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash)]
    struct TestScheduleA;

    #[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash)]
    struct TestScheduleB;

    #[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash)]
    struct TestScheduleC;

    #[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash)]
    struct TestScheduleD;

    fn first_system() {}
    fn second_system() {}

    fn setup() -> (Schedule, World) {
        let mut world = World::new();
        let mut schedule = Schedule::default();
        schedule
            .set_label(TestSchedule)
            .add_systems((first_system, second_system).chain());
        schedule.initialize(&mut world).unwrap();
        (schedule, world)
    }

    // Helper for verifying that a set of systems will be run for a given skip
    // list
    macro_rules! assert_systems_run {
        ($schedule:expr, $skipped_systems:expr, $($system:expr),*) => {
            // pull an ordered list of systems in the schedule, and save the
            // system TypeId, and name.
            let systems: Vec<(TypeId, std::borrow::Cow<'static, str>)> = $schedule.systems().unwrap()
                .map(|(_, system)| {
                    (system.type_id(), system.name())
                })
            .collect();

            // construct a list of systems that are expected to run
            let mut expected = FixedBitSet::with_capacity(systems.len());
            $(
                let sys = IntoSystem::into_system($system);
                for (i, (type_id, _)) in systems.iter().enumerate() {
                    if sys.type_id() == *type_id {
                        expected.insert(i);
                    }
                }
            )*

            // flip the run list to get our skip list
            expected.toggle_range(..);

            // grab the list of skipped systems
            let actual = match $skipped_systems {
                None => FixedBitSet::with_capacity(systems.len()),
                Some(b) => b,
            };

            if (actual != expected) {
                use std::fmt::Write as _;

                // mismatch, let's construct a human-readable message of what
                // was returned
                let mut msg = format!("Schedule:\n    {:9} {:16}{:6} {:6} {:6}\n",
                    "index", "name", "expect", "actual", "result");
                for (i, (_, full_name)) in systems.iter().enumerate() {
                    let (_, name) = full_name.rsplit_once("::").unwrap();
                    let _ = write!(msg, "    system[{:1}] {:16}", i, name);
                    match (expected.contains(i), actual.contains(i)) {
                        (true, true) => msg.push_str("skip   skip   pass\n"),
                        (true, false) =>
                            msg.push_str("skip   run    FAILED; system should not have run\n"),
                        (false, true) =>
                            msg.push_str("run    skip   FAILED; system should have run\n"),
                        (false, false) => msg.push_str("run    run    pass\n"),
                    }
                }
                assert_eq!(actual, expected, "{}", msg);
            }
        };
    }

    // Helper for verifying the expected systems will be run by the schedule
    //
    // This macro will construct an expected FixedBitSet for the systems that
    // should be skipped, and compare it with the results from stepping the
    // provided schedule.  If they don't match, it generates a human-readable
    // error message and asserts.
    macro_rules! assert_schedule_runs {
        ($schedule:expr, $stepping:expr, $($system:expr),*) => {
            // advance stepping to the next frame, and build the skip list for
            // this schedule
            $stepping.next_frame();
            assert_systems_run!($schedule, $stepping.skipped_systems($schedule), $($system),*);
        };
    }

    #[test]
    fn stepping_disabled() {
        let (schedule, _world) = setup();

        let mut stepping = Stepping::new();
        stepping.add_schedule(TestSchedule).disable().next_frame();

        assert!(stepping.skipped_systems(&schedule).is_none());
        assert!(stepping.cursor().is_none());
    }

    #[test]
    fn unknown_schedule() {
        let (schedule, _world) = setup();

        let mut stepping = Stepping::new();
        stepping.enable().next_frame();

        assert!(stepping.skipped_systems(&schedule).is_none());
    }

    #[test]
    fn disabled_always_run() {
        let (schedule, _world) = setup();

        let mut stepping = Stepping::new();
        stepping
            .add_schedule(TestSchedule)
            .disable()
            .always_run(TestSchedule, first_system);

        assert_schedule_runs!(&schedule, &mut stepping, first_system, second_system);
    }

    #[test]
    fn waiting_always_run() {
        let (schedule, _world) = setup();

        let mut stepping = Stepping::new();
        stepping
            .add_schedule(TestSchedule)
            .enable()
            .always_run(TestSchedule, first_system);

        assert_schedule_runs!(&schedule, &mut stepping, first_system);
    }

    #[test]
    fn step_always_run() {
        let (schedule, _world) = setup();

        let mut stepping = Stepping::new();
        stepping
            .add_schedule(TestSchedule)
            .enable()
            .always_run(TestSchedule, first_system)
            .step_frame();

        assert_schedule_runs!(&schedule, &mut stepping, first_system, second_system);
    }

    #[test]
    fn continue_always_run() {
        let (schedule, _world) = setup();

        let mut stepping = Stepping::new();
        stepping
            .add_schedule(TestSchedule)
            .enable()
            .always_run(TestSchedule, first_system)
            .continue_frame();

        assert_schedule_runs!(&schedule, &mut stepping, first_system, second_system);
    }

    #[test]
    fn disabled_never_run() {
        let (schedule, _world) = setup();

        let mut stepping = Stepping::new();
        stepping
            .add_schedule(TestSchedule)
            .never_run(TestSchedule, first_system)
            .disable();
        assert_schedule_runs!(&schedule, &mut stepping, first_system, second_system);
    }

    #[test]
    fn waiting_never_run() {
        let (schedule, _world) = setup();

        let mut stepping = Stepping::new();
        stepping
            .add_schedule(TestSchedule)
            .enable()
            .never_run(TestSchedule, first_system);

        assert_schedule_runs!(&schedule, &mut stepping,);
    }

    #[test]
    fn step_never_run() {
        let (schedule, _world) = setup();

        let mut stepping = Stepping::new();
        stepping
            .add_schedule(TestSchedule)
            .enable()
            .never_run(TestSchedule, first_system)
            .step_frame();

        assert_schedule_runs!(&schedule, &mut stepping, second_system);
    }

    #[test]
    fn continue_never_run() {
        let (schedule, _world) = setup();

        let mut stepping = Stepping::new();
        stepping
            .add_schedule(TestSchedule)
            .enable()
            .never_run(TestSchedule, first_system)
            .continue_frame();

        assert_schedule_runs!(&schedule, &mut stepping, second_system);
    }

    #[test]
    fn disabled_breakpoint() {
        let (schedule, _world) = setup();

        let mut stepping = Stepping::new();
        stepping
            .add_schedule(TestSchedule)
            .disable()
            .set_breakpoint(TestSchedule, second_system);

        assert_schedule_runs!(&schedule, &mut stepping, first_system, second_system);
    }

    #[test]
    fn waiting_breakpoint() {
        let (schedule, _world) = setup();

        let mut stepping = Stepping::new();
        stepping
            .add_schedule(TestSchedule)
            .enable()
            .set_breakpoint(TestSchedule, second_system);

        assert_schedule_runs!(&schedule, &mut stepping,);
    }

    #[test]
    fn step_breakpoint() {
        let (schedule, _world) = setup();

        let mut stepping = Stepping::new();
        stepping
            .add_schedule(TestSchedule)
            .enable()
            .set_breakpoint(TestSchedule, second_system)
            .step_frame();

        // since stepping stops at every system, breakpoints are ignored during
        // stepping
        assert_schedule_runs!(&schedule, &mut stepping, first_system);
        stepping.step_frame();
        assert_schedule_runs!(&schedule, &mut stepping, second_system);

        // let's go again to verify that we wrap back around to the start of
        // the frame
        stepping.step_frame();
        assert_schedule_runs!(&schedule, &mut stepping, first_system);

        // should be back in a waiting state now that it ran first_system
        assert_schedule_runs!(&schedule, &mut stepping,);
    }

    #[test]
    fn continue_breakpoint() {
        let (schedule, _world) = setup();

        let mut stepping = Stepping::new();
        stepping
            .add_schedule(TestSchedule)
            .enable()
            .set_breakpoint(TestSchedule, second_system)
            .continue_frame();

        assert_schedule_runs!(&schedule, &mut stepping, first_system);
        stepping.continue_frame();
        assert_schedule_runs!(&schedule, &mut stepping, second_system);
        stepping.continue_frame();
        assert_schedule_runs!(&schedule, &mut stepping, first_system);
    }

    #[test]
    fn clear_breakpoint() {
        let (schedule, _world) = setup();

        let mut stepping = Stepping::new();
        stepping
            .add_schedule(TestSchedule)
            .enable()
            .set_breakpoint(TestSchedule, second_system)
            .continue_frame();

        assert_schedule_runs!(&schedule, &mut stepping, first_system);
        stepping.continue_frame();
        assert_schedule_runs!(&schedule, &mut stepping, second_system);

        stepping.clear_breakpoint(TestSchedule, second_system);
        stepping.continue_frame();
        assert_schedule_runs!(&schedule, &mut stepping, first_system, second_system);
    }

    #[test]
    fn clear_system() {
        let (schedule, _world) = setup();

        let mut stepping = Stepping::new();
        stepping
            .add_schedule(TestSchedule)
            .enable()
            .never_run(TestSchedule, second_system)
            .continue_frame();
        assert_schedule_runs!(&schedule, &mut stepping, first_system);

        stepping.clear_system(TestSchedule, second_system);
        stepping.continue_frame();
        assert_schedule_runs!(&schedule, &mut stepping, first_system, second_system);
    }

    #[test]
    fn clear_schedule() {
        let (schedule, _world) = setup();

        let mut stepping = Stepping::new();
        stepping
            .add_schedule(TestSchedule)
            .enable()
            .never_run(TestSchedule, first_system)
            .never_run(TestSchedule, second_system)
            .continue_frame();
        assert_schedule_runs!(&schedule, &mut stepping,);

        stepping.clear_schedule(TestSchedule);
        stepping.continue_frame();
        assert_schedule_runs!(&schedule, &mut stepping, first_system, second_system);
    }

    // Schedules such as FixedUpdate can be called multiple times in a single
    // render frame.  Ensure we only run steppable systems the first time the
    // schedule is run
    #[test]
    fn multiple_calls_per_frame_continue() {
        let (schedule, _world) = setup();

        let mut stepping = Stepping::new();
        stepping
            .add_schedule(TestSchedule)
            .enable()
            .always_run(TestSchedule, second_system)
            .continue_frame();

        // start a new frame, then run the schedule three times; first system
        // should only run on the first one
        stepping.next_frame();
        assert_systems_run!(
            &schedule,
            stepping.skipped_systems(&schedule),
            first_system,
            second_system
        );
        assert_systems_run!(
            &schedule,
            stepping.skipped_systems(&schedule),
            second_system
        );
    }
    #[test]
    fn multiple_calls_per_frame_step() {
        let (schedule, _world) = setup();

        let mut stepping = Stepping::new();
        stepping.add_schedule(TestSchedule).enable().step_frame();

        // start a new frame, then run the schedule three times; first system
        // should only run on the first one
        stepping.next_frame();
        assert_systems_run!(&schedule, stepping.skipped_systems(&schedule), first_system);
        assert_systems_run!(&schedule, stepping.skipped_systems(&schedule),);
    }

    // verify that Stepping can construct an ordered list of schedules
    #[test]
    fn schedules() {
        let mut world = World::new();

        // build & initialize three schedules
        let mut schedule_a = Schedule::default();
        schedule_a.set_label(TestScheduleA);
        schedule_a.initialize(&mut world).unwrap();
        let mut schedule_b = Schedule::default();
        schedule_b.set_label(TestScheduleB);
        schedule_b.initialize(&mut world).unwrap();
        let mut schedule_c = Schedule::default();
        schedule_c.set_label(TestScheduleC);
        schedule_c.initialize(&mut world).unwrap();
        let mut schedule_d = Schedule::default();
        schedule_d.set_label(TestScheduleD);
        schedule_d.initialize(&mut world).unwrap();

        // setup stepping and add all three schedules
        let mut stepping = Stepping::new();
        stepping
            .add_schedule(TestScheduleA)
            .add_schedule(TestScheduleB)
            .add_schedule(TestScheduleC)
            .add_schedule(TestScheduleD)
            .enable()
            .next_frame();

        assert!(stepping.schedules().is_err());

        stepping.skipped_systems(&schedule_b);
        assert!(stepping.schedules().is_err());
        stepping.skipped_systems(&schedule_a);
        assert!(stepping.schedules().is_err());
        stepping.skipped_systems(&schedule_c);
        assert!(stepping.schedules().is_err());

        // when we call the last schedule, Stepping should have enough data to
        // return an ordered list of schedules
        stepping.skipped_systems(&schedule_d);
        assert!(stepping.schedules().is_ok());

        assert_eq!(
            *stepping.schedules().unwrap(),
            vec![
                TestScheduleB.dyn_clone(),
                TestScheduleA.dyn_clone(),
                TestScheduleC.dyn_clone(),
                TestScheduleD.dyn_clone(),
            ]
        );
    }

    // Helper to verify cursor position
    /*
    macro_rules! assert_cursor_at {
        ($stepping:expr, $schedule:expr, $system:expr) => {
            let type_id = IntoSystem::into_system($system).type_id();
            let node_id = $schedule
                .systems()
                .unwrap()
                .find(|t| t.1.type_id() == type_id)
                .unwrap()
                .0;
            let label = $schedule.label().as_ref().unwrap();
            assert_eq!($stepping.cursor(), Some((label.dyn_clone(), node_id)));
        };
    } */

    #[test]
    fn verify_cursor() {
        // helper to build a cursor tuple for the supplied schedule
        fn cursor(schedule: &Schedule, index: usize) -> (BoxedScheduleLabel, NodeId) {
            let node_id = schedule.executable().system_ids[index];
            (schedule.label().as_ref().unwrap().clone(), node_id)
        }

        let mut world = World::new();

        // create two schedules with a number of systems in them
        let mut schedule_a = Schedule::default();
        schedule_a
            .set_label(TestScheduleA)
            .add_systems((|| {}, || {}, || {}, || {}).chain());
        schedule_a.initialize(&mut world).unwrap();
        let mut schedule_b = Schedule::default();
        schedule_b
            .set_label(TestScheduleB)
            .add_systems((|| {}, || {}, || {}, || {}).chain());
        schedule_b.initialize(&mut world).unwrap();

        // setup stepping and add all three schedules
        let mut stepping = Stepping::new();
        stepping
            .add_schedule(TestScheduleA)
            .add_schedule(TestScheduleB)
            .enable();

        assert!(stepping.cursor().is_none());

        // step the system nine times, and verify the cursor before & after
        // each step
        let mut cursors = Vec::new();
        for _ in 0..9 {
            stepping.step_frame().next_frame();
            cursors.push(stepping.cursor());
            stepping.skipped_systems(&schedule_a);
            stepping.skipped_systems(&schedule_b);
            cursors.push(stepping.cursor());
        }

        #[rustfmt::skip]
        assert_eq!(
            cursors,
            vec![
                // before render frame        // after render frame
                None,                         Some(cursor(&schedule_a, 1)),
                Some(cursor(&schedule_a, 1)), Some(cursor(&schedule_a, 2)),
                Some(cursor(&schedule_a, 2)), Some(cursor(&schedule_a, 3)),
                Some(cursor(&schedule_a, 3)), Some(cursor(&schedule_b, 0)),
                Some(cursor(&schedule_b, 0)), Some(cursor(&schedule_b, 1)),
                Some(cursor(&schedule_b, 1)), Some(cursor(&schedule_b, 2)),
                Some(cursor(&schedule_b, 2)), Some(cursor(&schedule_b, 3)),
                Some(cursor(&schedule_b, 3)), None,
                Some(cursor(&schedule_a, 0)), Some(cursor(&schedule_a, 1)),
            ]
        );

        // reset our cursor (disable/enable), and update stepping to test if the
        // cursor properly skips over AlwaysRun & NeverRun systems.  Also set
        // a Break system to ensure that shows properly in the cursor
        stepping
            // disable/enable to reset cursor
            .disable()
            .enable()
            .set_breakpoint_node(TestScheduleA, NodeId::System(1))
            .always_run_node(TestScheduleA, NodeId::System(3))
            .never_run_node(TestScheduleB, NodeId::System(0));

        let mut cursors = Vec::new();
        for _ in 0..9 {
            stepping.step_frame().next_frame();
            cursors.push(stepping.cursor());
            stepping.skipped_systems(&schedule_a);
            stepping.skipped_systems(&schedule_b);
            cursors.push(stepping.cursor());
        }

        #[rustfmt::skip]
        assert_eq!(
            cursors,
            vec![
                // before render frame        // after render frame
                Some(cursor(&schedule_a, 0)), Some(cursor(&schedule_a, 1)),
                Some(cursor(&schedule_a, 1)), Some(cursor(&schedule_a, 2)),
                Some(cursor(&schedule_a, 2)), Some(cursor(&schedule_b, 1)),
                Some(cursor(&schedule_b, 1)), Some(cursor(&schedule_b, 2)),
                Some(cursor(&schedule_b, 2)), Some(cursor(&schedule_b, 3)),
                Some(cursor(&schedule_b, 3)), None,
                Some(cursor(&schedule_a, 0)), Some(cursor(&schedule_a, 1)),
                Some(cursor(&schedule_a, 1)), Some(cursor(&schedule_a, 2)),
                Some(cursor(&schedule_a, 2)), Some(cursor(&schedule_b, 1)),
            ]
        );
    }
}
