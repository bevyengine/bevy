use crate::system::{SystemParam, SystemParamFetch, SystemParamState, SystemState};
use crate::world::World;

pub struct ParamState<Param: SystemParam> {
    state: SystemState,
    param_state: <Param as SystemParam>::Fetch,
    change_tick: u32,
}

impl<Param: SystemParam> ParamState<Param> {
    pub fn new(world: &mut World) -> Self {
        let mut state = SystemState::new::<Param>();
        let config = <Param::Fetch as SystemParamState>::default_config();
        let param_state = <Param::Fetch as SystemParamState>::init(world, &mut state, config);
        Self {
            state,
            param_state,
            change_tick: 0,
        }
    }

    // TODO: THIS IS SUPER UNSAFE PLEASE DON'T MERGE UNTIL IT IS CONSTRAINED TO READ-ONLY PARAMS
    pub fn get<'a>(&'a mut self, world: &'a World) -> <Param::Fetch as SystemParamFetch<'a>>::Item {
        let change_tick = world.increment_change_tick();
        self.change_tick = change_tick;
        // TODO: add/implement ReadOnlySystemParam and constrain param here
        unsafe {
            <Param::Fetch as SystemParamFetch>::get_param(
                &mut self.param_state,
                &mut self.state,
                world,
                change_tick,
            )
        }
    }
}
