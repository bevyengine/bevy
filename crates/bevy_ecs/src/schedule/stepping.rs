use bevy_utils::tracing::warn;
use fixedbitset::FixedBitSet;
use std::any::TypeId;
use std::collections::HashMap;

use crate::{
    schedule::{BoxedScheduleLabel, NodeId, Schedule, ScheduleLabel},
    system::{IntoSystem, ResMut, Resource, System},
};

use crate as bevy_ecs;

#[derive(Debug, Default, PartialEq, Eq, Copy, Clone)]
enum StepAction {
    /// Stepping is disabled; run all systems
    #[default]
    RunAll,

    /// Stepping is enabled, but we're only running required systems this frame
    Waiting,

    /// Stepping is enabled; run all systems until the end of the frame, or
    /// until we encounter a system marked with [`StepBehavior::Break`] or all
    /// systems in the frame have run.
    Continue,

    /// stepping is enabled; only run the next system in our step list
    Step,
}

#[derive(Debug, Copy, Clone)]
enum StepBehavior {
    AlwaysRun,
    NeverRun,
    Continue,
    Break,
}

enum SystemIdentifier {
    Type(TypeId),
    Node(NodeId),
}

enum Update {
    Action(StepAction),
    AddSchedule(BoxedScheduleLabel),
    RemoveSchedule(BoxedScheduleLabel),
    ClearSchedule(BoxedScheduleLabel),
    SetBehavior(BoxedScheduleLabel, SystemIdentifier, StepBehavior),
    ClearBehavior(BoxedScheduleLabel, SystemIdentifier),
}

#[derive(Resource, Default)]
pub struct Stepping {
    /// [`ScheduleState`] for each [`Schedule`] with stepping enabled
    schedule_states: HashMap<BoxedScheduleLabel, ScheduleState>,

    /// Action to perform during this frame
    action: StepAction,

    /// Updates to stepping state to be applied at the start of the next frame
    updates: Vec<Update>,

    /// dynamically generated [`Schedule`] order
    schedule_order: Vec<BoxedScheduleLabel>,

    /// tracks the index of the last schedule stepped in schedule_order
    last_schedule: Option<usize>,
}

impl Stepping {
    pub fn new() -> Self {
        Stepping::default()
    }

    /// System that notifies the [`Stepping`] resource that a new frame has
    /// begun.
    pub fn begin_frame(stepping: Option<ResMut<Self>>) {
        if let Some(mut stepping) = stepping {
            stepping.next_frame();
        }
    }

    /// Return the list of schedules with stepping enabled in the order
    /// they are executed in.
    pub fn schedules(&mut self) -> &Vec<BoxedScheduleLabel> {
        &self.schedule_order
    }

    /// Enable stepping for the provided schedule
    pub fn add_schedule<Label>(&mut self, schedule: Label) -> &mut Self
    where
        Label: ScheduleLabel + Clone,
    {
        self.updates.push(Update::AddSchedule(Box::new(schedule)));
        self
    }

    /// disable stepping for the provided schedule
    pub fn remove_schedule(&mut self, schedule: impl ScheduleLabel) -> &mut Self {
        self.updates
            .push(Update::RemoveSchedule(Box::new(schedule)));
        self
    }

    /// clear [`StepBehavior`] set for all systems in the provided [`Schedule`]
    pub fn clear_schedule(&mut self, schedule: impl ScheduleLabel) -> &mut Self {
        self.updates.push(Update::ClearSchedule(Box::new(schedule)));
        self
    }

    /// Begin stepping at the start of the next frame
    pub fn enable(&mut self) -> &mut Self {
        self.updates.push(Update::Action(StepAction::Waiting));
        self
    }

    /// Disable stepping, resume normal systems execution
    pub fn disable(&mut self) -> &mut Self {
        self.updates.push(Update::Action(StepAction::RunAll));
        self
    }

    /// run the next system
    pub fn step_frame(&mut self) -> &mut Self {
        self.updates.push(Update::Action(StepAction::Step));
        self
    }

    /// continue stepping until the end of the frame or a breakpoint is hit
    pub fn continue_frame(&mut self) -> &mut Self {
        self.updates.push(Update::Action(StepAction::Continue));
        self
    }

    /// Ensure these systems always run when stepping is enabled
    pub fn always_run<Marker>(
        &mut self,
        schedule: impl ScheduleLabel,
        system: impl IntoSystem<(), (), Marker>,
    ) -> &mut Self {
        let type_id = IntoSystem::into_system(system).type_id();
        self.updates.push(Update::SetBehavior(
            Box::new(schedule),
            SystemIdentifier::Type(type_id),
            StepBehavior::AlwaysRun,
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
            StepBehavior::NeverRun,
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
            StepBehavior::Break,
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

    /// Clear any behavior set for the system
    pub fn clear_system<Marker>(
        &mut self,
        schedule: impl ScheduleLabel,
        system: impl IntoSystem<(), (), Marker>,
    ) -> &mut Self {
        let type_id = IntoSystem::into_system(system).type_id();
        self.updates.push(Update::ClearBehavior(
            Box::new(schedule),
            SystemIdentifier::Type(type_id),
        ));

        self
    }

    /// Advance schedule states for the next render frame
    fn next_frame(&mut self) {
        // If we were stepping, and nothing ran, the action will remain Step.
        // This happens when we've already run everything in the "stepping"
        // frame.  Now, the user requested a step, let's reset our schedule
        // state to the beginning and start a new "stepping" frame.
        match self.action {
            StepAction::Step => {
                println!("wrapping state");
                self.next_stepping_frame();

                // This return here is to ensure that the step is executed.  We
                // don't want to process updates from the previous frame
                // because they may overwrite the action, resulting in Step
                // doing nothing.
                return;
            }
            StepAction::Continue => {
                self.next_stepping_frame();
                self.action = StepAction::Waiting;
            }
            _ => (),
        }

        if self.updates.is_empty() {
            return;
        }

        let mut clear_schedule = false;
        for update in self.updates.drain(..) {
            match update {
                Update::Action(StepAction::RunAll) => {
                    self.action = StepAction::RunAll;
                    clear_schedule = true;
                }
                Update::Action(a) => self.action = a,
                Update::AddSchedule(l) => {
                    self.schedule_states.insert(l, ScheduleState::default());
                }
                Update::RemoveSchedule(l) => {
                    self.schedule_states.remove(&l);
                }
                Update::ClearSchedule(label) => match self.schedule_states.get_mut(&label) {
                    Some(state) => state.clear_behaviors(),
                    None => {
                        warn!(
                            "stepping is not enabled for schedule {:?}; use
                    `.add_stepping({:?})` to enable stepping",
                            label, label
                        )
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
                            )
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
                            )
                        }
                    }
                }
            }
        }

        if clear_schedule {
            self.next_stepping_frame();
        }
    }

    /// Advance schedule states for a new stepping frame
    fn next_stepping_frame(&mut self) {
        println!("next_stepping_frame(): being new stepping frame");

        // clear any progress we've made in the stepping frame
        self.last_schedule = None;

        for state in self.schedule_states.values_mut() {
            state.next = 0;
        }
    }

    /// Build the list of systems the supplied schedule should skip this render
    /// frame
    pub fn build_skip_list(&mut self, schedule: &Schedule) -> Option<FixedBitSet> {
        if self.action == StepAction::RunAll {
            return None;
        }
        let state = match schedule.label() {
            Some(label) => self.schedule_states.get_mut(label)?,
            None => return None,
        };

        // We are stepping through a  schedule that has stepping enabled.
        //
        // Let's take a moment to make sure this schedule is in our ordered
        // schedule list.  We maintain this to give any stepping UI a way to
        // know what order schedules are called in.
        let label = schedule.label().as_ref().unwrap();
        let index = self.schedule_order.iter().position(|l| l == label);
        match (self.last_schedule, index) {
            (_, Some(i)) => self.last_schedule = Some(i),
            (Some(last), None) => {
                self.schedule_order.insert(last + 1, label.clone());
                self.last_schedule = Some(last + 1);
            }
            (None, None) => {
                self.schedule_order.insert(0, label.clone());
                self.last_schedule = Some(0);
            }
        }

        let (list, wait) = state.progress(schedule, self.action);
        if wait {
            println!(
                "build_skip_list({:?}): state {:?} -> {:?}",
                schedule.label(),
                self.action,
                StepAction::Waiting
            );
            self.action = StepAction::Waiting;
        }
        Some(list)
    }
}

#[derive(Default)]
struct ScheduleState {
    /// Index of the next system to run in the system schedule; if set to
    /// greater than the number of systems in the schedule, then don't run
    /// any systems.
    /// NOTE: [`StepBehavior::AlwaysRun`] systems ignore this value and always
    /// run
    next: usize,

    /// per-system [`StepBehavior`]
    systems: HashMap<NodeId, StepBehavior>,

    /// changes to system behavior that should be applied the next time
    /// [`ScheduleState::progress()`] is called
    updates: HashMap<TypeId, Option<StepBehavior>>,
}

impl ScheduleState {
    // set the stepping behavior for a system in this schedule
    fn set_behavior(&mut self, system: SystemIdentifier, behavior: StepBehavior) {
        match system {
            SystemIdentifier::Node(node_id) => {
                self.systems.insert(node_id, behavior);
            }
            SystemIdentifier::Type(type_id) => {
                self.updates.insert(type_id, Some(behavior));
            }
        }
    }

    // clear the stepping behavior for a system in this schedule
    fn clear_behavior(&mut self, system: SystemIdentifier) {
        match system {
            SystemIdentifier::Node(node_id) => {
                self.systems.remove(&node_id);
            }
            SystemIdentifier::Type(type_id) => {
                self.updates.insert(type_id, None);
            }
        }
    }

    // clear all system behaviors
    fn clear_behaviors(&mut self) {
        self.systems.clear();
    }

    // apply system behavior updates by looking up the node id of the system in
    // the schedule, and updating `systems`
    fn apply_updates(&mut self, schedule: &Schedule) {
        for (node_id, system) in schedule.systems().unwrap() {
            let behavior = self.updates.get(&system.type_id());
            match behavior {
                None => continue,
                Some(None) => {
                    self.systems.remove(&node_id);
                }
                Some(Some(behavior)) => {
                    self.systems.insert(node_id, *behavior);
                }
            }
        }
        self.updates.clear();

        println!("apply_updates(): {:?}", self.systems);
    }

    // progress the schedule for the given action type, returning a skip list,
    // and if the caller should update state to waiting
    fn progress(&mut self, schedule: &Schedule, action: StepAction) -> (FixedBitSet, bool) {
        // Now that we have the schedule, apply any pending system behavior
        // updates.  The schedule is required to map from system `TypeId` to
        // `NodeId`.
        if !self.updates.is_empty() {
            self.apply_updates(schedule);
        }

        match action {
            StepAction::RunAll => unreachable!(),
            StepAction::Waiting => self.skip_list_waiting(schedule),
            StepAction::Continue => self.skip_list_continue(schedule),
            StepAction::Step => self.skip_list_step(schedule),
        }
    }

    fn skip_list_waiting(&mut self, schedule: &Schedule) -> (FixedBitSet, bool) {
        let mut skip = FixedBitSet::with_capacity(schedule.systems_len());

        for (i, (node_id, system)) in schedule.systems().unwrap().enumerate() {
            println!(
                "skip_list_waiting(): {} ({:?}) -- {:?}",
                system.name(),
                node_id,
                self.systems.get(&node_id),
            );

            let mut advance = false;
            match self
                .systems
                .get(&node_id)
                .unwrap_or(&StepBehavior::Continue)
            {
                StepBehavior::AlwaysRun => advance = true,
                StepBehavior::NeverRun => {
                    advance = true;
                    skip.insert(i);
                }
                StepBehavior::Continue | StepBehavior::Break => skip.insert(i),
            }

            if advance && self.next == i {
                self.next += 1;
            }
        }

        (skip, false)
    }

    fn skip_list_step(&mut self, schedule: &Schedule) -> (FixedBitSet, bool) {
        let mut skip = FixedBitSet::with_capacity(schedule.systems_len());
        let mut stepped = false;
        for (i, (node_id, system)) in schedule.systems().unwrap().enumerate() {
            println!(
                "skip_list_step(): {} ({:?}) -- {:?}",
                system.name(),
                node_id,
                self.systems.get(&node_id),
            );

            let mut advance = false;
            match self
                .systems
                .get(&node_id)
                .unwrap_or(&StepBehavior::Continue)
            {
                StepBehavior::AlwaysRun => advance = true,
                StepBehavior::NeverRun => {
                    advance = true;
                    skip.insert(i);
                }
                StepBehavior::Continue | StepBehavior::Break => {
                    if i < self.next || stepped {
                        skip.insert(i);
                    } else {
                        advance = true;
                        stepped = true;
                    }
                }
            }

            if advance && self.next == i {
                self.next += 1;
            }
        }

        (skip, stepped)
    }

    fn skip_list_continue(&mut self, schedule: &Schedule) -> (FixedBitSet, bool) {
        let mut skip = FixedBitSet::with_capacity(schedule.systems_len());
        let mut hit_breakpoint = false;
        let mut pos = self.next;

        for (i, (node_id, system)) in schedule.systems().unwrap().enumerate() {
            println!(
                "skip_list_continue(): {} ({:?}) -- {:?}; i {}, pos {}, break {}",
                system.name(),
                node_id,
                self.systems.get(&node_id),
                i,
                pos,
                hit_breakpoint,
            );

            match self
                .systems
                .get(&node_id)
                .unwrap_or(&StepBehavior::Continue)
            {
                StepBehavior::AlwaysRun => (),
                StepBehavior::NeverRun => skip.insert(i),
                StepBehavior::Continue => {
                    if i != pos {
                        skip.insert(i);
                    }
                }
                StepBehavior::Break => {
                    if i == self.next {
                        println!("resuming from breakpoint");
                    } else if i < pos {
                        println!("skipping already run system");
                        skip.insert(i);
                    } else if i == pos {
                        println!("hit breakpoint");
                        hit_breakpoint = true;
                        skip.insert(i);
                    }
                }
            }

            if !hit_breakpoint && i == pos {
                pos = i + 1;
            }
        }
        self.next = pos;
        (skip, hit_breakpoint)
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

    // Helper for verifying the expected systems will be run by the schedule
    //
    // This macro will construct an expected FixedBitSet for the systems that
    // should be skipped, and compare it with the results from stepping the
    // provided schedule.  If they don't match, it generates a human-readable
    // error message and asserts.
    macro_rules! assert_schedule_runs {
        ($schedule:expr, $stepping:expr, $($system:expr),*) => {
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

            // advance stepping to the next frame, and build the skip list for
            // this schedule
            $stepping.next_frame();
            let actual = match $stepping.build_skip_list($schedule) {
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

    #[test]
    fn stepping_disabled() {
        let (schedule, _world) = setup();

        let mut stepping = Stepping::new();
        stepping.add_schedule(TestSchedule).disable().next_frame();

        assert!(stepping.build_skip_list(&schedule).is_none());
    }

    #[test]
    fn unknown_schedule() {
        let (schedule, _world) = setup();

        let mut stepping = Stepping::new();
        stepping.enable().next_frame();

        assert!(stepping.build_skip_list(&schedule).is_none());
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

        // when stepping, we take one render frame to detect that the stepping
        // frame has completed.  This looks like nothing runs for the first
        // render frame, then the second render-frame resets the stepping frame
        // and we run the first system again.
        //
        // XXX this is weird from the UI standpoint; how does UI display what
        // is the next system to be run, esp when transitioning between
        // schedules.
        stepping.step_frame();
        assert_schedule_runs!(&schedule, &mut stepping,);
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

        stepping.build_skip_list(&schedule_b);
        stepping.build_skip_list(&schedule_a);
        stepping.build_skip_list(&schedule_c);
        assert_eq!(
            &stepping.schedules()[0],
            schedule_b.label().as_ref().unwrap()
        );
        assert_eq!(
            &stepping.schedules()[1],
            schedule_a.label().as_ref().unwrap()
        );
        assert_eq!(
            &stepping.schedules()[2],
            schedule_c.label().as_ref().unwrap()
        );

        // make it complicated, run schedule_d after schedule_b and confirm it
        // was inserted correctly
        stepping.next_frame();
        stepping.build_skip_list(&schedule_b);
        stepping.build_skip_list(&schedule_d);
        stepping.build_skip_list(&schedule_a);
        stepping.build_skip_list(&schedule_c);
        assert_eq!(
            &stepping.schedules()[0],
            schedule_b.label().as_ref().unwrap()
        );
        assert_eq!(
            &stepping.schedules()[1],
            schedule_d.label().as_ref().unwrap()
        );
        assert_eq!(
            &stepping.schedules()[2],
            schedule_a.label().as_ref().unwrap()
        );
        assert_eq!(
            &stepping.schedules()[3],
            schedule_c.label().as_ref().unwrap()
        );
    }
}
