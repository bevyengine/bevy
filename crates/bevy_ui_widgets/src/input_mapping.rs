use bevy_app::{App, Plugin};
use bevy_ecs::{prelude::*, system::SystemParam};
use bevy_picking::pointer::PointerButton;
use bevy_reflect::Reflect;

/// The global input mapping for triggering `Activate`.
#[derive(Resource, Debug, Clone, PartialEq, Eq, Reflect)]
#[reflect(Resource, Debug, Clone)]
pub struct ActivateInputMap {
    /// The pointer buttons that can trigger `Activate`.
    pub pointer_buttons: Vec<PointerButton>,
}

impl Default for ActivateInputMap {
    fn default() -> Self {
        Self {
            pointer_buttons: vec![PointerButton::Primary],
        }
    }
}

/// The interface for widgets to query whether input is allowed.
#[derive(SystemParam)]
pub struct ActivateInput<'w> {
    map: Res<'w, ActivateInputMap>,
}

impl ActivateInput<'_> {
    /// Returns whether this pointer button can trigger `Activate`.
    pub fn allows_pointer_button(&self, button: PointerButton) -> bool {
        self.map.pointer_buttons.contains(&button)
    }
}

/// Registers the [`ActivateInputMap`] resource and makes it available as a [`SystemParam`].
pub struct ActivateInputMapPlugin;

impl Plugin for ActivateInputMapPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ActivateInputMap>()
            .register_type::<ActivateInputMap>();
    }
}
