use fixedbitset::FixedBitSet;
use std::any::TypeId;
use std::collections::HashMap;

use crate::{
    schedule::{InternedScheduleLabel, NodeId, Schedule, ScheduleLabel},
    system::{IntoSystem, ResMut, Resource},
};
use bevy_utils::{
    tracing::{error, info, warn},
    TypeIdMap,
};
use thiserror::Error;

#[cfg(test)]
use bevy_utils::tracing::debug;

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
    /// System will always run regardless of stepping action
    AlwaysRun,

    /// System will never run while stepping is enabled
    NeverRun,

    /// When [`Action::Waiting`] this system will not be run
    /// When [`Action::Step`] this system will be stepped
    /// When [`Action::Continue`] system execution will stop before executing
    /// this system unless its the first system run when continuing
    Break,

    /// When [`Action::Waiting`] this system will not be run
    /// When [`Action::Step`] this system will be stepped
    /// When [`Action::Continue`] this system will be run
    Continue,
}

// schedule_order index, and schedule start point
#[derive(Debug, Default, Clone, Copy)]
struct Cursor {
    /// index within `Stepping::schedule_order`
    pub schedule: usize,
    /// index within the schedule's system list
    pub system: usize,
}

// Two methods of referring to Systems, via TypeId, or per-Schedule NodeId
enum SystemIdentifier {
    Type(TypeId),
    Node(NodeId),
}

/// Updates to [`Stepping.schedule_states`] that will be applied at the start
/// of the next render frame
enum Update {
    /// Set the action stepping will perform for this render frame
    SetAction(Action),
    /// Enable stepping for this schedule
    AddSchedule(InternedScheduleLabel),
    /// Disable stepping for this schedule
    RemoveSchedule(InternedScheduleLabel),
    /// Clear any system-specific behaviors for this schedule
    ClearSchedule(InternedScheduleLabel),
    /// Set a system-specific behavior for this schedule & system
    SetBehavior(InternedScheduleLabel, SystemIdentifier, SystemBehavior),
    /// Clear any system-specific behavior for this schedule & system
    ClearBehavior(InternedScheduleLabel, SystemIdentifier),
}

#[derive(Error, Debug)]
#[error("not available until all configured schedules have been run; try again next frame")]
pub struct NotReady;

#[derive(Resource, Default)]
/// Resource for controlling system stepping behavior
pub struct Stepping {
    // [`ScheduleState`] for each [`Schedule`] with stepping enabled
    schedule_states: HashMap<InternedScheduleLabel, ScheduleState>,

    // dynamically generated [`Schedule`] order
    schedule_order: Vec<InternedScheduleLabel>,

    // current position in the stepping frame
    cursor: Cursor,

    // index in [`schedule_order`] of the last schedule to call `skipped_systems()`
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
    /// Create a new instance of the `Stepping` resource.
    pub fn new() -> Self {
        Stepping::default()
    }

    /// System to call denoting that a new render frame has begun
    ///
    /// Note: This system is automatically added to the default `MainSchedule`.
    pub fn begin_frame(stepping: Option<ResMut<Self>>) {
        if let Some(mut stepping) = stepping {
            stepping.next_frame();
        }
    }

    /// Return the list of schedules with stepping enabled in the order
    /// they are executed in.
    pub fn schedules(&self) -> Result<&Vec<InternedScheduleLabel>, NotReady> {
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
    pub fn cursor(&self) -> Option<(InternedScheduleLabel, NodeId)> {
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
            .map(|node_id| (*label, *node_id))
    }

    /// Enable stepping for the provided schedule
    pub fn add_schedule(&mut self, schedule: impl ScheduleLabel) -> &mut Self {
        self.updates.push(Update::AddSchedule(schedule.intern()));
        self
    }

    /// Disable stepping for the provided schedule
    ///
    /// NOTE: This function will also clear any system-specific behaviors that
    /// may have been configured.
    pub fn remove_schedule(&mut self, schedule: impl ScheduleLabel) -> &mut Self {
        self.updates.push(Update::RemoveSchedule(schedule.intern()));
        self
    }

    /// Clear behavior set for all systems in the provided [`Schedule`]
    pub fn clear_schedule(&mut self, schedule: impl ScheduleLabel) -> &mut Self {
        self.updates.push(Update::ClearSchedule(schedule.intern()));
        self
    }

    /// Begin stepping at the start of the next frame
    pub fn enable(&mut self) -> &mut Self {
        #[cfg(feature = "bevy_debug_stepping")]
        self.updates.push(Update::SetAction(Action::Waiting));
        #[cfg(not(feature = "bevy_debug_stepping"))]
        error!(
            "Stepping cannot be enabled; \
            bevy was compiled without the bevy_debug_stepping feature"
        );
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

    /// Run the next system during the next render frame
    ///
    /// NOTE: This will have no impact unless stepping has been enabled
    pub fn step_frame(&mut self) -> &mut Self {
        self.updates.push(Update::SetAction(Action::Step));
        self
    }

    /// Run all remaining systems in the stepping frame during the next render
    /// frame
    ///
    /// NOTE: This will have no impact unless stepping has been enabled
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
        let type_id = system.system_type_id();
        self.updates.push(Update::SetBehavior(
            schedule.intern(),
            SystemIdentifier::Type(type_id),
            SystemBehavior::AlwaysRun,
        ));

        self
    }

    /// Ensure this system instance always runs when stepping is enabled
    pub fn always_run_node(&mut self, schedule: impl ScheduleLabel, node: NodeId) -> &mut Self {
        self.updates.push(Update::SetBehavior(
            schedule.intern(),
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
        let type_id = system.system_type_id();
        self.updates.push(Update::SetBehavior(
            schedule.intern(),
            SystemIdentifier::Type(type_id),
            SystemBehavior::NeverRun,
        ));

        self
    }

    /// Ensure this system instance never runs when stepping is enabled
    pub fn never_run_node(&mut self, schedule: impl ScheduleLabel, node: NodeId) -> &mut Self {
        self.updates.push(Update::SetBehavior(
            schedule.intern(),
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
        let type_id = system.system_type_id();
        self.updates.push(Update::SetBehavior(
            schedule.intern(),
            SystemIdentifier::Type(type_id),
            SystemBehavior::Break,
        ));

        self
    }

    /// Add a breakpoint for system instance
    pub fn set_breakpoint_node(&mut self, schedule: impl ScheduleLabel, node: NodeId) -> &mut Self {
        self.updates.push(Update::SetBehavior(
            schedule.intern(),
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
        let type_id = system.system_type_id();
        self.updates.push(Update::ClearBehavior(
            schedule.intern(),
            SystemIdentifier::Type(type_id),
        ));

        self
    }

    /// clear a breakpoint for system instance
    pub fn clear_node(&mut self, schedule: impl ScheduleLabel, node: NodeId) -> &mut Self {
        self.updates.push(Update::ClearBehavior(
            schedule.intern(),
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
            if self.cursor.schedule >= self.schedule_order.len() {
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
                Update::SetAction(action) => {
                    // This match block is really just to filter out invalid
                    // transitions, and add debugging messages for permitted
                    // transitions.  Any action transition that falls through
                    // this match block will be performed.
                    match (self.action, action) {
                        // ignore non-transition updates, and prevent a call to
                        // enable() from overwriting a step or continue call
                        (Action::RunAll, Action::RunAll)
                        | (Action::Waiting, Action::Waiting)
                        | (Action::Continue, Action::Continue)
                        | (Action::Step, Action::Step)
                        | (Action::Continue, Action::Waiting)
                        | (Action::Step, Action::Waiting) => continue,

                        // when stepping is disabled
                        (Action::RunAll, Action::Waiting) => info!("enabled stepping"),
                        (Action::RunAll, _) => {
                            warn!(
                                "stepping not enabled; call Stepping::enable() \
                                before step_frame() or continue_frame()"
                            );
                            continue;
                        }

                        // stepping enabled; waiting
                        (Action::Waiting, Action::RunAll) => info!("disabled stepping"),
                        (Action::Waiting, Action::Continue) => info!("continue frame"),
                        (Action::Waiting, Action::Step) => info!("step frame"),

                        // stepping enabled; continue frame
                        (Action::Continue, Action::RunAll) => info!("disabled stepping"),
                        (Action::Continue, Action::Step) => {
                            warn!("ignoring step_frame(); already continuing next frame");
                            continue;
                        }

                        // stepping enabled; step frame
                        (Action::Step, Action::RunAll) => info!("disabled stepping"),
                        (Action::Step, Action::Continue) => {
                            warn!("ignoring continue_frame(); already stepping next frame");
                            continue;
                        }
                    }

                    // permitted action transition; make the change
                    self.action = action;
                }
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
                            "stepping is not enabled for schedule {:?}; \
                            use `.add_stepping({:?})` to enable stepping",
                            label, label
                        );
                    }
                },
                Update::SetBehavior(label, system, behavior) => {
                    match self.schedule_states.get_mut(&label) {
                        Some(state) => state.set_behavior(system, behavior),
                        None => {
                            warn!(
                                "stepping is not enabled for schedule {:?}; \
                                use `.add_stepping({:?})` to enable stepping",
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
                                "stepping is not enabled for schedule {:?}; \
                                use `.add_stepping({:?})` to enable stepping",
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

        // grab the label and state for this schedule
        let label = schedule.label();
        let state = self.schedule_states.get_mut(&label)?;

        // Stepping is enabled, and this schedule is supposed to be stepped.
        //
        // We need to maintain a list of schedules in the order that they call
        // this function. We'll check the ordered list now to see if this
        // schedule is present. If not, we'll add it after the last schedule
        // that called this function. Finally we want to save off the index of
        // this schedule in the ordered schedule list. This is used to
        // determine if this is the schedule the cursor is pointed at.
        let index = self.schedule_order.iter().position(|l| *l == label);
        let index = match (index, self.previous_schedule) {
            (Some(index), _) => index,
            (None, None) => {
                self.schedule_order.insert(0, label);
                0
            }
            (None, Some(last)) => {
                self.schedule_order.insert(last + 1, label);
                last + 1
            }
        };
        // Update the index of the previous schedule to be the index of this
        // schedule for the next call
        self.previous_schedule = Some(index);

        #[cfg(test)]
        debug!(
            "cursor {:?}, index {}, label {:?}",
            self.cursor, index, label
        );

        // if the stepping frame cursor is pointing at this schedule, we'll run
        // the schedule with the current stepping action.  If this is not the
        // cursor schedule, we'll run the schedule with the waiting action.
        let cursor = self.cursor;
        let (skip_list, next_system) = if index == cursor.schedule {
            let (skip_list, next_system) =
                state.skipped_systems(schedule, cursor.system, self.action);

            // if we just stepped this schedule, then we'll switch the action
            // to be waiting
            if self.action == Action::Step {
                self.action = Action::Waiting;
            }
            (skip_list, next_system)
        } else {
            // we're not supposed to run any systems in this schedule, so pull
            // the skip list, but ignore any changes it makes to the cursor.
            let (skip_list, _) = state.skipped_systems(schedule, 0, Action::Waiting);
            (skip_list, Some(cursor.system))
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
                let index = cursor.schedule + 1;
                self.cursor = Cursor {
                    schedule: index,
                    system: self.first_system_index_for_schedule(index),
                };

                #[cfg(test)]
                debug!("advanced schedule index: {} -> {}", cursor.schedule, index);
            }
        }

        Some(skip_list)
    }
}

#[derive(Default)]
struct ScheduleState {
    /// per-system [`SystemBehavior`]
    behaviors: HashMap<NodeId, SystemBehavior>,

    /// order of [`NodeId`]s in the schedule
    ///
    /// This is a cached copy of `SystemExecutable::system_ids`. We need it
    /// available here to be accessed by [`Stepping::cursor()`] so we can return
    /// [`NodeId`]s to the caller.
    node_ids: Vec<NodeId>,

    /// changes to system behavior that should be applied the next time
    /// [`ScheduleState::skipped_systems()`] is called
    behavior_updates: TypeIdMap<Option<SystemBehavior>>,

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
            // Behaviors are indexed by NodeId, but we cannot map a system
            // TypeId to a NodeId without the `Schedule`.  So queue this update
            // to be processed the next time `skipped_systems()` is called.
            SystemIdentifier::Type(type_id) => {
                self.behavior_updates.insert(type_id, Some(behavior));
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
            // queue TypeId updates to be processed later when we have Schedule
            SystemIdentifier::Type(type_id) => {
                self.behavior_updates.insert(type_id, None);
            }
        }
    }

    // clear all system behaviors
    fn clear_behaviors(&mut self) {
        self.behaviors.clear();
        self.behavior_updates.clear();
        self.first = None;
    }

    // apply system behavior updates by looking up the node id of the system in
    // the schedule, and updating `systems`
    fn apply_behavior_updates(&mut self, schedule: &Schedule) {
        // Systems may be present multiple times within a schedule, so we
        // iterate through all systems in the schedule, and check our behavior
        // updates for the system TypeId.
        // PERF: If we add a way to efficiently query schedule systems by their TypeId, we could remove the full
        // system scan here
        for (node_id, system) in schedule.systems().unwrap() {
            let behavior = self.behavior_updates.get(&system.type_id());
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
        self.behavior_updates.clear();

        #[cfg(test)]
        debug!("apply_updates(): {:?}", self.behaviors);
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
            self.node_ids.clone_from(&schedule.executable().system_ids);
        }

        // Now that we have the schedule, apply any pending system behavior
        // updates.  The schedule is required to map from system `TypeId` to
        // `NodeId`.
        if !self.behavior_updates.is_empty() {
            self.apply_behavior_updates(schedule);
        }

        // if we don't have a first system set, set it now
        if self.first.is_none() {
            for (i, (node_id, _)) in schedule.systems().unwrap().enumerate() {
                match self.behaviors.get(&node_id) {
                    Some(SystemBehavior::AlwaysRun | SystemBehavior::NeverRun) => continue,
                    Some(_) | None => {
                        self.first = Some(i);
                        break;
                    }
                }
            }
        }

        let mut skip = FixedBitSet::with_capacity(schedule.systems_len());
        let mut pos = start;

        for (i, (node_id, _system)) in schedule.systems().unwrap().enumerate() {
            let behavior = self
                .behaviors
                .get(&node_id)
                .unwrap_or(&SystemBehavior::Continue);

            #[cfg(test)]
            debug!(
                "skipped_systems(): systems[{}], pos {}, Action::{:?}, Behavior::{:?}, {}",
                i,
                pos,
                action,
                behavior,
                _system.name()
            );

            match (action, behavior) {
                // regardless of which action we're performing, if the system
                // is marked as NeverRun, add it to the skip list.
                // Also, advance the cursor past this system if it is our
                // current position
                (_, SystemBehavior::NeverRun) => {
                    skip.insert(i);
                    if i == pos {
                        pos += 1;
                    }
                }
                // similarly, ignore any system marked as AlwaysRun; they should
                // never be added to the skip list
                // Also, advance the cursor past this system if it is our
                // current position
                (_, SystemBehavior::AlwaysRun) => {
                    if i == pos {
                        pos += 1;
                    }
                }
                // if we're waiting, no other systems besides AlwaysRun should
                // be run, so add systems to the skip list
                (Action::Waiting, _) => skip.insert(i),

                // If we're stepping, the remaining behaviors don't matter,
                // we're only going to run the system at our cursor.  Any system
                // prior to the cursor is skipped.  Once we encounter the system
                // at the cursor, we'll advance the cursor, and set behavior to
                // Waiting to skip remaining systems.
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
                // If we're continuing, and we encounter a breakpoint we may
                // want to stop before executing the system.  To do this we
                // skip this system and set the action to Waiting.
                //
                // Note: if the cursor is pointing at this system, we will run
                // it anyway.  This allows the user to continue, hit a
                // breakpoint, then continue again to run the breakpoint system
                // and any following systems.
                (Action::Continue, SystemBehavior::Break) => {
                    if i != start {
                        skip.insert(i);

                        // stop running systems if the breakpoint isn't the
                        // system under the cursor.
                        if i > start {
                            action = Action::Waiting;
                        }
                    }
                }
                // should have never gotten into this method if stepping is
                // disabled
                (Action::RunAll, _) => unreachable!(),
            }

            // If we're at the cursor position, and not waiting, advance the
            // cursor.
            if i == pos && action != Action::Waiting {
                pos += 1;
            }
        }

        // output is the skip list, and the index of the next system to run in
        // this schedule.
        if pos >= schedule.systems_len() {
            (skip, None)
        } else {
            (skip, Some(pos))
        }
    }
}

#[cfg(all(test, feature = "bevy_debug_stepping"))]
mod tests {
    use super::*;
    use crate::prelude::*;
    use crate::schedule::ScheduleLabel;

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
    fn third_system() {}

    fn setup() -> (Schedule, World) {
        let mut world = World::new();
        let mut schedule = Schedule::new(TestSchedule);
        schedule.add_systems((first_system, second_system).chain());
        schedule.initialize(&mut world).unwrap();
        (schedule, world)
    }

    // Helper for verifying skip_lists are equal, and if not, printing a human
    // readable message.
    macro_rules! assert_skip_list_eq {
        ($actual:expr, $expected:expr, $system_names:expr) => {
            let actual = $actual;
            let expected = $expected;
            let systems: &Vec<&str> = $system_names;

            if (actual != expected) {
                use std::fmt::Write as _;

                // mismatch, let's construct a human-readable message of what
                // was returned
                let mut msg = format!(
                    "Schedule:\n    {:9} {:16}{:6} {:6} {:6}\n",
                    "index", "name", "expect", "actual", "result"
                );
                for (i, name) in systems.iter().enumerate() {
                    let _ = write!(msg, "    system[{:1}] {:16}", i, name);
                    match (expected.contains(i), actual.contains(i)) {
                        (true, true) => msg.push_str("skip   skip   pass\n"),
                        (true, false) => {
                            msg.push_str("skip   run    FAILED; system should not have run\n")
                        }
                        (false, true) => {
                            msg.push_str("run    skip   FAILED; system should have run\n")
                        }
                        (false, false) => msg.push_str("run    run    pass\n"),
                    }
                }
                assert_eq!(actual, expected, "{}", msg);
            }
        };
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
            let system_names: Vec<&str> = systems
                .iter()
                .map(|(_,n)| n.rsplit_once("::").unwrap().1)
                .collect();

            assert_skip_list_eq!(actual, expected, &system_names);
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

    /// regression test for issue encountered while writing `system_stepping`
    /// example
    #[test]
    fn continue_step_continue_with_breakpoint() {
        let mut world = World::new();
        let mut schedule = Schedule::new(TestSchedule);
        schedule.add_systems((first_system, second_system, third_system).chain());
        schedule.initialize(&mut world).unwrap();

        let mut stepping = Stepping::new();
        stepping
            .add_schedule(TestSchedule)
            .enable()
            .set_breakpoint(TestSchedule, second_system);

        stepping.continue_frame();
        assert_schedule_runs!(&schedule, &mut stepping, first_system);

        stepping.step_frame();
        assert_schedule_runs!(&schedule, &mut stepping, second_system);

        stepping.continue_frame();
        assert_schedule_runs!(&schedule, &mut stepping, third_system);
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

    /// This was discovered in code-review, ensure that `clear_schedule` also
    /// clears any pending changes too.
    #[test]
    fn set_behavior_then_clear_schedule() {
        let (schedule, _world) = setup();

        let mut stepping = Stepping::new();
        stepping
            .add_schedule(TestSchedule)
            .enable()
            .continue_frame();
        assert_schedule_runs!(&schedule, &mut stepping, first_system, second_system);

        stepping.never_run(TestSchedule, first_system);
        stepping.clear_schedule(TestSchedule);
        stepping.continue_frame();
        assert_schedule_runs!(&schedule, &mut stepping, first_system, second_system);
    }

    /// Ensure that if they `clear_schedule` then make further changes to the
    /// schedule, those changes after the clear are applied.
    #[test]
    fn clear_schedule_then_set_behavior() {
        let (schedule, _world) = setup();

        let mut stepping = Stepping::new();
        stepping
            .add_schedule(TestSchedule)
            .enable()
            .continue_frame();
        assert_schedule_runs!(&schedule, &mut stepping, first_system, second_system);

        stepping.clear_schedule(TestSchedule);
        stepping.never_run(TestSchedule, first_system);
        stepping.continue_frame();
        assert_schedule_runs!(&schedule, &mut stepping, second_system);
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

        // start a new frame, then run the schedule two times; first system
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

        // start a new frame, then run the schedule two times; first system
        // should only run on the first one
        stepping.next_frame();
        assert_systems_run!(&schedule, stepping.skipped_systems(&schedule), first_system);
        assert_systems_run!(&schedule, stepping.skipped_systems(&schedule),);
    }

    #[test]
    fn step_duplicate_systems() {
        let mut world = World::new();
        let mut schedule = Schedule::new(TestSchedule);
        schedule.add_systems((first_system, first_system, second_system).chain());
        schedule.initialize(&mut world).unwrap();

        let mut stepping = Stepping::new();
        stepping.add_schedule(TestSchedule).enable();

        // needed for assert_skip_list_eq!
        let system_names = vec!["first_system", "first_system", "second_system"];
        // we're going to step three times, and each system in order should run
        // only once
        for system_index in 0..3 {
            // build the skip list by setting all bits, then clearing our the
            // one system that should run this step
            let mut expected = FixedBitSet::with_capacity(3);
            expected.set_range(.., true);
            expected.set(system_index, false);

            // step the frame and get the skip list
            stepping.step_frame();
            stepping.next_frame();
            let skip_list = stepping
                .skipped_systems(&schedule)
                .expect("TestSchedule has been added to Stepping");

            assert_skip_list_eq!(skip_list, expected, &system_names);
        }
    }

    #[test]
    fn step_run_if_false() {
        let mut world = World::new();
        let mut schedule = Schedule::new(TestSchedule);

        // This needs to be a system test to confirm the interaction between
        // the skip list and system conditions in Schedule::run().  That means
        // all of our systems need real bodies that do things.
        //
        // first system will be configured as `run_if(|| false)`, so it can
        // just panic if called
        let first_system = move || panic!("first_system should not be run");

        // The second system, we need to know when it has been called, so we'll
        // add a resource for tracking if it has been run.  The system will
        // increment the run count.
        #[derive(Resource)]
        struct RunCount(usize);
        world.insert_resource(RunCount(0));
        let second_system = |mut run_count: ResMut<RunCount>| {
            println!("I have run!");
            run_count.0 += 1;
        };

        // build our schedule; first_system should never run, followed by
        // second_system.
        schedule.add_systems((first_system.run_if(|| false), second_system).chain());
        schedule.initialize(&mut world).unwrap();

        // set up stepping
        let mut stepping = Stepping::new();
        stepping.add_schedule(TestSchedule).enable();
        world.insert_resource(stepping);

        // if we step, and the run condition is false, we should not run
        // second_system.  The stepping cursor is at first_system, and if
        // first_system wasn't able to run, that's ok.
        let mut stepping = world.resource_mut::<Stepping>();
        stepping.step_frame();
        stepping.next_frame();
        schedule.run(&mut world);
        assert_eq!(
            world.resource::<RunCount>().0,
            0,
            "second_system should not have run"
        );

        // now on the next step, second_system should run
        let mut stepping = world.resource_mut::<Stepping>();
        stepping.step_frame();
        stepping.next_frame();
        schedule.run(&mut world);
        assert_eq!(
            world.resource::<RunCount>().0,
            1,
            "second_system should have run"
        );
    }

    #[test]
    fn remove_schedule() {
        let (schedule, _world) = setup();
        let mut stepping = Stepping::new();
        stepping.add_schedule(TestSchedule).enable();

        // run the schedule once and verify all systems are skipped
        assert_schedule_runs!(&schedule, &mut stepping,);
        assert!(!stepping.schedules().unwrap().is_empty());

        // remove the test schedule
        stepping.remove_schedule(TestSchedule);
        assert_schedule_runs!(&schedule, &mut stepping, first_system, second_system);
        assert!(stepping.schedules().unwrap().is_empty());
    }

    // verify that Stepping can construct an ordered list of schedules
    #[test]
    fn schedules() {
        let mut world = World::new();

        // build & initialize a few schedules
        let mut schedule_a = Schedule::new(TestScheduleA);
        schedule_a.initialize(&mut world).unwrap();
        let mut schedule_b = Schedule::new(TestScheduleB);
        schedule_b.initialize(&mut world).unwrap();
        let mut schedule_c = Schedule::new(TestScheduleC);
        schedule_c.initialize(&mut world).unwrap();
        let mut schedule_d = Schedule::new(TestScheduleD);
        schedule_d.initialize(&mut world).unwrap();

        // setup stepping and add all the schedules
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
                TestScheduleB.intern(),
                TestScheduleA.intern(),
                TestScheduleC.intern(),
                TestScheduleD.intern(),
            ]
        );
    }

    #[test]
    fn verify_cursor() {
        // helper to build a cursor tuple for the supplied schedule
        fn cursor(schedule: &Schedule, index: usize) -> (InternedScheduleLabel, NodeId) {
            let node_id = schedule.executable().system_ids[index];
            (schedule.label(), node_id)
        }

        let mut world = World::new();

        // create two schedules with a number of systems in them
        let mut schedule_a = Schedule::new(TestScheduleA);
        schedule_a.add_systems((|| {}, || {}, || {}, || {}).chain());
        schedule_a.initialize(&mut world).unwrap();
        let mut schedule_b = Schedule::new(TestScheduleB);
        schedule_b.add_systems((|| {}, || {}, || {}, || {}).chain());
        schedule_b.initialize(&mut world).unwrap();

        // setup stepping and add all schedules
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
