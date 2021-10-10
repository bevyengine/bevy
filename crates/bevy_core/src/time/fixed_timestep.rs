use crate::Time;
use bevy_ecs::{
    archetype::{Archetype, ArchetypeComponentId},
    component::ComponentId,
    query::Access,
    schedule::ShouldRun,
    system::{ConfigurableSystem, IntoSystem, Local, Res, ResMut, System},
    world::World,
};
use bevy_utils::HashMap;
use std::borrow::Cow;

pub struct FixedTimestepState {
    pub step: f64,
    pub accumulator: f64,
}

impl FixedTimestepState {
    /// The amount of time each step takes
    pub fn step(&self) -> f64 {
        self.step
    }

    /// The number of steps made in a second
    pub fn steps_per_second(&self) -> f64 {
        1.0 / self.step
    }

    /// The amount of time (in seconds) left over from the last step
    pub fn accumulator(&self) -> f64 {
        self.accumulator
    }

    /// The percentage of "step" stored inside the accumulator. Calculated as accumulator / step
    pub fn overstep_percentage(&self) -> f64 {
        self.accumulator / self.step
    }
}

#[derive(Default)]
pub struct FixedTimesteps {
    fixed_timesteps: HashMap<String, FixedTimestepState>,
}

impl FixedTimesteps {
    pub fn get(&self, name: &str) -> Option<&FixedTimestepState> {
        self.fixed_timesteps.get(name)
    }
}

pub struct FixedTimestep {
    state: State,
    internal_system: Box<dyn System<In = (), Out = ShouldRun>>,
}

impl Default for FixedTimestep {
    fn default() -> Self {
        Self {
            state: State::default(),
            internal_system: Box::new(Self::prepare_system.system()),
        }
    }
}

impl FixedTimestep {
    pub fn step(step: f64) -> Self {
        Self {
            state: State {
                step,
                ..Default::default()
            },
            ..Default::default()
        }
    }

    pub fn steps_per_second(rate: f64) -> Self {
        Self {
            state: State {
                step: 1.0 / rate,
                ..Default::default()
            },
            ..Default::default()
        }
    }

    pub fn with_label(mut self, label: &str) -> Self {
        self.state.label = Some(label.to_string());
        self
    }

    fn prepare_system(
        mut state: Local<State>,
        time: Res<Time>,
        mut fixed_timesteps: ResMut<FixedTimesteps>,
    ) -> ShouldRun {
        let should_run = state.update(&time);
        if let Some(ref label) = state.label {
            let res_state = fixed_timesteps.fixed_timesteps.get_mut(label).unwrap();
            res_state.step = state.step;
            res_state.accumulator = state.accumulator;
        }

        should_run
    }
}

#[derive(Clone)]
pub struct State {
    label: Option<String>, // TODO: consider making this a TypedLabel
    step: f64,
    accumulator: f64,
    looping: bool,
}

impl Default for State {
    fn default() -> Self {
        Self {
            step: 1.0 / 60.0,
            accumulator: 0.0,
            label: None,
            looping: false,
        }
    }
}

impl State {
    fn update(&mut self, time: &Time) -> ShouldRun {
        if !self.looping {
            self.accumulator += time.delta_seconds_f64();
        }

        if self.accumulator >= self.step {
            self.accumulator -= self.step;
            self.looping = true;
            ShouldRun::YesAndCheckAgain
        } else {
            self.looping = false;
            ShouldRun::No
        }
    }
}

impl System for FixedTimestep {
    type In = ();
    type Out = ShouldRun;

    fn name(&self) -> Cow<'static, str> {
        Cow::Borrowed(std::any::type_name::<FixedTimestep>())
    }

    fn new_archetype(&mut self, archetype: &Archetype) {
        self.internal_system.new_archetype(archetype);
    }

    fn archetype_component_access(&self) -> &Access<ArchetypeComponentId> {
        self.internal_system.archetype_component_access()
    }

    fn component_access(&self) -> &Access<ComponentId> {
        self.internal_system.component_access()
    }

    fn is_send(&self) -> bool {
        self.internal_system.is_send()
    }

    unsafe fn run_unsafe(&mut self, _input: (), world: &World) -> ShouldRun {
        // SAFE: this system inherits the internal system's component access and archetype component
        // access, which means the caller has ensured running the internal system is safe
        self.internal_system.run_unsafe((), world)
    }

    fn apply_buffers(&mut self, world: &mut World) {
        self.internal_system.apply_buffers(world)
    }

    fn initialize(&mut self, world: &mut World) {
        self.internal_system =
            Box::new(Self::prepare_system.config(|c| c.0 = Some(self.state.clone())));
        self.internal_system.initialize(world);
        if let Some(ref label) = self.state.label {
            let mut fixed_timesteps = world.get_resource_mut::<FixedTimesteps>().unwrap();
            fixed_timesteps.fixed_timesteps.insert(
                label.clone(),
                FixedTimestepState {
                    accumulator: 0.0,
                    step: self.state.step,
                },
            );
        }
    }

    fn check_change_tick(&mut self, change_tick: u32) {
        self.internal_system.check_change_tick(change_tick);
    }
}
