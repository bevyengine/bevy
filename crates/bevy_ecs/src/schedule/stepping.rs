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

    /// continue stepping until the end of the frame or a breakpoint is hit
    pub fn r#continue(&mut self) -> &mut Self {
        self.updates.push(Update::Action(StepAction::Continue));
        self
    }

    /// run the next system
    pub fn step(&mut self) -> &mut Self {
        self.updates.push(Update::Action(StepAction::Step));
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

    pub(crate) fn next_frame(&mut self) {
        // If we were stepping, and nothing ran, the action will remain Step.
        // This happens when we've already run everything in the "stepping"
        // frame.  Now, the user requested a step, let's reset our schedule
        // state to the beginning and start a new "stepping" frame.
        if self.action == StepAction::Step {
            self.clear_schedule_progress();

            // This return here is to ensure that the step is executed.  We
            // don't want to process updates from the previous frame because
            // they may overwrite the action, resulting in Step doing nothing.
            return;
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
            self.clear_schedule_progress();
        }
    }

    pub fn begin_frame(stepping: Option<ResMut<Self>>) {
        if let Some(mut stepping) = stepping {
            stepping.next_frame();
        }
    }

    /// clears all schedule progress, starting a new stepping frame.
    fn clear_schedule_progress(&mut self) {
        println!("clearing schedule progress");
        // clear any progress we've made in the stepping frame
        for (_, state) in self.schedule_states.iter_mut() {
            state.next = 0;
        }
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
        for (pos, node_id) in schedule.executable().system_ids.iter().enumerate() {
            use StepBehavior::*;
            match self.systems.get(node_id).unwrap_or(&Continue) {
                AlwaysRun => (),
                NeverRun => (),
                Continue => (),
                Break => (),
            }
        }
        (skip, false)
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

    #[test]
    fn stepping_disabled() {
        let (schedule, _world) = setup();

        let mut stepping = Stepping::new();
        stepping.disable().add_schedule(TestSchedule);

        // returns None when stepping isn't enabled
        stepping.disable();
        stepping.next_frame();
        let skip = stepping.build_skip_list(&schedule);
        assert!(skip.is_none());
    }

    // if build_skip_list is caled with an unknown schedule, it will return none
    #[test]
    fn unknown_schedule() {
        let (schedule, _world) = setup();

        let mut stepping = Stepping::new();
        stepping.enable();

        stepping.next_frame();
        let skip = stepping.build_skip_list(&schedule);
        assert!(skip.is_none());
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
        stepping.step();
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
        stepping.step();
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
        stepping.step();
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
    fn step_always_run() {
        let (schedule, _world) = setup();

        let mut stepping = Stepping::new();
        stepping.enable().add_schedule(TestSchedule);

        // Let's make it so the first system always runs
        stepping.always_run(TestSchedule, first_system);

        // when we're stopped/waiting, only the second system should be in the
        // skip list, as the first system should always run.
        println!("next frame; waiting");
        stepping.next_frame();
        let skip = stepping.build_skip_list(&schedule).unwrap();
        assert_eq!(skip.len(), schedule.executable().systems.len());
        assert!(!skip.contains(0));
        assert!(skip.contains(1));

        // let's step and verify neither system is skipped
        println!("next frame; step");
        stepping.step();
        stepping.next_frame();
        let skip = stepping.build_skip_list(&schedule).unwrap();
        assert_eq!(skip.len(), schedule.executable().systems.len());
        assert!(!skip.contains(0));
        assert!(!skip.contains(1));

        // verify wrapping occurrs properly
        println!("next frame; step, detect wrapping");
        stepping.step();
        stepping.next_frame();
        let skip = stepping.build_skip_list(&schedule).unwrap();
        assert_eq!(skip.len(), schedule.executable().systems.len());
        assert!(!skip.contains(0));
        assert!(skip.contains(1));

        println!("next frame; wrapped step");
        stepping.next_frame();
        let skip = stepping.build_skip_list(&schedule).unwrap();
        assert_eq!(skip.len(), schedule.executable().systems.len());
        assert!(!skip.contains(0));
        assert!(!skip.contains(1));
    }

    #[test]
    fn step_never_run() {
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
        stepping.step();
        stepping.next_frame();
        let skip = stepping.build_skip_list(&schedule).unwrap();
        assert_eq!(skip.len(), schedule.executable().systems.len());
        assert!(skip.contains(0));
        assert!(!skip.contains(1));

        println!("next frame; step, detect wrapping");
        stepping.step();
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
    fn step_breakpoint() {
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
        stepping.step();
        stepping.next_frame();
        let skip = stepping.build_skip_list(&schedule).unwrap();
        assert_eq!(skip.len(), schedule.executable().systems.len());
        assert!(!skip.contains(0));
        assert!(skip.contains(1));

        // now the second system should run
        println!("next frame: step");
        stepping.step();
        stepping.next_frame();
        let skip = stepping.build_skip_list(&schedule).unwrap();
        assert_eq!(skip.len(), schedule.executable().systems.len());
        assert!(skip.contains(0));
        assert!(!skip.contains(1));
    }
}
