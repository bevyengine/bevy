use bevy_platform::collections::HashSet;

use crate::component::{CheckChangeTicks, FragmentingValue};

/// Stores each unique combination of component type + value for [`FragmentingValueComponent`]s as
/// untyped [`FragmentingValue`]s.
///
/// [`FragmentingValueComponent`]: crate::component::FragmentingValueComponent
// TODO: make this more useful and add a public api.
#[derive(Default)]
pub struct FragmentingValueComponentsStorage {
    pub(crate) existing_values: HashSet<FragmentingValue>,
}

impl FragmentingValueComponentsStorage {
    pub(crate) fn check_change_ticks(&mut self, _check: CheckChangeTicks) {}
}
