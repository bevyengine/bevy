use bevy_platform::collections::HashSet;

use crate::component::{CheckChangeTicks, FragmentingValue};

#[derive(Default)]
pub struct FragmentingValueComponentsStorage {
    pub(crate) existing_values: HashSet<FragmentingValue>,
}

impl FragmentingValueComponentsStorage {
    pub fn check_change_ticks(&mut self, _check: CheckChangeTicks) {}
}
