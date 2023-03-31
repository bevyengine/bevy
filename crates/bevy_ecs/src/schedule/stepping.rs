#![allow(dead_code, unused_mut)]
#![allow(unused_variables)]
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
    /// until we encounter a system marked with [`SystemBehavior::Break`].
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
                Update::SetBehavior(label, system, behavior) => {
                    let state = self.schedule_states.get_mut(&label).unwrap_or_else(|| {
                        panic!(
                            "stepping is not enabled for schedule {:?}; use
                `.add_stepping({:?})` to enable stepping",
                            label, label
                        )
                    });
                    state.set_behavior(system, behavior);
                }
                Update::ClearBehavior(label, system) => {
                    let state = self.schedule_states.get_mut(&label).unwrap_or_else(|| {
                        panic!(
                            "stepping is not enabled for schedule {:?}; use
                `.add_stepping({:?})` to enable stepping",
                            label, label
                        )
                    });
                    state.clear_behavior(system);
                }
            }
        }

        if clear_schedule {
            self.next_stepping_frame();
        }
    }

    /// Advance schedule states for a new stepping frame
    fn next_stepping_frame(&mut self) {
        println!("next_stepping_frame(): resetting schedule states");
        // clear any progress we've made in the stepping frame
        for (_, state) in self.schedule_states.iter_mut() {
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
    /// NOTE: [`AlwaysRun`] systems ignore this value and always run
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
    fn progress(&mut self, schedule: &Schedule, mut action: StepAction) -> (FixedBitSet, bool) {
        // Now that we have the schedule, apply any pending system behavior
        // updates.  The schedule is required to map from system `TypeId` to
        // `NodeId`.
        if !self.updates.is_empty() {
            self.apply_updates(schedule);
        }

        let systems_schedule = schedule.executable();

        // let skip = FixedBitSet::with_capacity(systems_schedule.systems.len());
        let skip = FixedBitSet::with_capacity(schedule.systems_len());
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
            let systems: Vec<(TypeId, std::borrow::Cow<'static, str>)> = $schedule.systems().unwrap().enumerate()
                .map(|(i, (_, system))| {
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
    fn step() {
        let (schedule, _world) = setup();

        let mut stepping = Stepping::new();
        stepping.enable().add_schedule(TestSchedule);

        // stepping while waiting should result in skipping all systems
        stepping.next_frame();
        let skip = stepping.build_skip_list(&schedule).unwrap();
        assert_eq!(skip.count_ones(..), schedule.executable().systems.len());

        // single stepping should result in only running the first system
        println!("step 1");
        stepping.step_frame();
        stepping.next_frame();
        let skip = stepping.build_skip_list(&schedule).unwrap();
        assert_eq!(skip.len(), schedule.executable().systems.len());
        assert!(!skip.contains(0));
        assert!(skip.contains(1));

        // call it again without issuing another `step()` call, and we should
        // skip all the systems
        println!("wait");
        stepping.next_frame();
        let skip = stepping.build_skip_list(&schedule).unwrap();
        assert_eq!(skip.count_ones(..), schedule.executable().systems.len());

        // doing it again should step the second system
        println!("step 2");
        stepping.step_frame();
        stepping.next_frame();
        let skip = stepping.build_skip_list(&schedule).unwrap();
        assert_eq!(skip.len(), schedule.executable().systems.len());
        assert!(skip.contains(0));
        assert!(!skip.contains(1));

        // Now that we've run all of the systems in our stepping schedules, we
        // want to wrap back around to the first system.  However, stepping only
        // detects that we need to wrap back around when it runs a system step
        // pass, but no schedule is able to run a system.
        //
        // Let's verify that behavior now first by trying to step again, to see
        // that all systems are skipped.
        println!("step 3 -- wrap detect");
        stepping.step_frame();
        stepping.next_frame();
        let skip = stepping.build_skip_list(&schedule).unwrap();
        assert_eq!(skip.count_ones(..), schedule.executable().systems.len());

        // Now, call `next_frame()` again, then build the skip list.  This list
        // should have wrapped, and allow only the first system to run.
        println!("step 3 -- wrapped");
        stepping.next_frame();
        let skip = stepping.build_skip_list(&schedule).unwrap();
        assert_eq!(skip.len(), schedule.executable().systems.len());
        assert!(!skip.contains(0));
        assert!(skip.contains(1));
    }

    #[test]
    fn step_never_run2() {
        let (schedule, _world) = setup();

        let mut stepping = Stepping::new();
        stepping.enable().add_schedule(TestSchedule);

        stepping.never_run(TestSchedule, first_system);

        println!("next frame; waiting");
        stepping.next_frame();
        let skip = stepping.build_skip_list(&schedule).unwrap();
        assert_eq!(skip.len(), schedule.executable().systems.len());
        assert!(skip.contains(0));
        assert!(skip.contains(1));

        println!("next frame; step");
        stepping.step_frame();
        stepping.next_frame();
        let skip = stepping.build_skip_list(&schedule).unwrap();
        assert_eq!(skip.len(), schedule.executable().systems.len());
        assert!(skip.contains(0));
        assert!(!skip.contains(1));

        println!("next frame; step, detect wrapping");
        stepping.step_frame();
        stepping.next_frame();
        let skip = stepping.build_skip_list(&schedule).unwrap();
        assert_eq!(skip.len(), schedule.executable().systems.len());
        assert!(skip.contains(0));
        assert!(skip.contains(1));

        println!("next frame; wrapped step");
        stepping.next_frame();
        let skip = stepping.build_skip_list(&schedule).unwrap();
        assert_eq!(skip.len(), schedule.executable().systems.len());
        assert!(skip.contains(0));
        assert!(!skip.contains(1));
    }

    // This test just verifies that stepping stops on breakpoint systems, then
    // steps past them
    #[test]
    fn step_breakpoint2() {
        let (schedule, _world) = setup();

        let mut stepping = Stepping::new();
        stepping.enable().add_schedule(TestSchedule);

        stepping.set_breakpoint(TestSchedule, first_system);

        // stepping while waiting should result in skipping all systems
        stepping.next_frame();
        let skip = stepping.build_skip_list(&schedule).unwrap();
        assert_eq!(skip.count_ones(..), schedule.executable().systems.len());

        // single stepping should result in only running the first system
        println!("step 1");
        stepping.step_frame();
        stepping.next_frame();
        let skip = stepping.build_skip_list(&schedule).unwrap();
        assert_eq!(skip.len(), schedule.executable().systems.len());
        assert!(!skip.contains(0));
        assert!(skip.contains(1));

        // now the second system should run
        println!("next frame: step");
        stepping.step_frame();
        stepping.next_frame();
        let skip = stepping.build_skip_list(&schedule).unwrap();
        assert_eq!(skip.len(), schedule.executable().systems.len());
        assert!(skip.contains(0));
        assert!(!skip.contains(1));
    }

    // verify continue_frame behavior
    #[test]
    fn continue_frame() {
        let (schedule, _world) = setup();

        let mut stepping = Stepping::new();
        stepping.enable().add_schedule(TestSchedule);

        // starting from a fresh frame, continue should run every system in the
        // frame
        println!("continue frame");
        stepping.continue_frame();
        stepping.next_frame();
        let skip = stepping.build_skip_list(&schedule).unwrap();
        assert_eq!(skip.len(), schedule.executable().systems.len());
        assert!(!skip.contains(0));
        assert!(!skip.contains(1));

        // if we render the next frame, we should be in a waiting state, and
        // nothing should run
        stepping.next_frame();
        assert_eq!(stepping.action, StepAction::Waiting);
        let skip = stepping.build_skip_list(&schedule).unwrap();
        assert_eq!(skip.count_ones(..), schedule.executable().systems.len());

        // step to only run the first frame
        println!("next frame: step");
        stepping.step_frame();
        stepping.next_frame();
        let skip = stepping.build_skip_list(&schedule).unwrap();
        assert_eq!(skip.len(), schedule.executable().systems.len());
        assert!(!skip.contains(0));
        assert!(skip.contains(1));

        // continue the frame, and we should see only the second system running
        println!("continue frame");
        stepping.continue_frame();
        stepping.next_frame();
        let skip = stepping.build_skip_list(&schedule).unwrap();
        assert_eq!(skip.len(), schedule.executable().systems.len());
        assert!(skip.contains(0));
        assert!(!skip.contains(1));

        // if we continue again, we should run all systems
        println!("continue frame");
        stepping.continue_frame();
        stepping.next_frame();
        let skip = stepping.build_skip_list(&schedule).unwrap();
        assert_eq!(skip.len(), schedule.executable().systems.len());
        assert!(!skip.contains(0));
        assert!(!skip.contains(1));

        // if we throw in a breakpoint for the second system only the first
        // sytem should run
        println!("continue frame; with breakpoint on second system");
        stepping.set_breakpoint(TestSchedule, second_system);
        stepping.continue_frame();
        stepping.next_frame();
        let skip = stepping.build_skip_list(&schedule).unwrap();
        assert_eq!(skip.len(), schedule.executable().systems.len());
        assert!(!skip.contains(0));
        assert!(skip.contains(1));

        // now resume the frame we hit the breakpoint on, and only the second
        // system should run
        println!("continue frame; resume at second system");
        stepping.continue_frame();
        stepping.next_frame();
        let skip = stepping.build_skip_list(&schedule).unwrap();
        assert_eq!(skip.len(), schedule.executable().systems.len());
        assert!(skip.contains(0));
        assert!(!skip.contains(1));
    }
}
