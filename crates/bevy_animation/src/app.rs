use bevy_app::AppBuilder;

use crate::custom::{AnimatedAsset, AnimatedComponent};
use crate::stage;

pub trait AddAnimated {
    fn register_animated_component<T: AnimatedComponent>(&mut self) -> &mut Self;

    fn register_animated_asset<T: AnimatedAsset>(&mut self) -> &mut Self;
}

impl AddAnimated for AppBuilder {
    #[inline(always)]
    fn register_animated_component<T: AnimatedComponent>(&mut self) -> &mut Self {
        self.add_system_to_stage(stage::ANIMATE, T::animator_update_system)
    }

    #[inline(always)]
    fn register_animated_asset<T: AnimatedAsset>(&mut self) -> &mut Self {
        self.add_system_to_stage(stage::ANIMATE, T::animator_update_system)
    }
}
