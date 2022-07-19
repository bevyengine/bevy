mod commands;
pub mod event_helpers;
pub mod events;
mod named;
mod resolve;
mod seeds;

use std::marker::PhantomData;

use bevy_app::prelude::*;
use bevy_ecs::{
    schedule::{ParallelSystemDescriptorCoercion, SystemLabel},
    system::{SystemParam, SystemParamItem},
};

pub use events::{NavEvent, NavRequest};
pub use non_empty_vec::NonEmpty;
pub use resolve::{FocusAction, FocusState, Focusable, Focused, MoveParam, NavLock};
pub use seeds::NavMenu;

/// The [`Bundle`](bevy::prelude::Bundle)s
/// returned by the [`NavMenu`] methods.
pub mod bundles {
    pub use crate::seeds::{MenuSeed, NamedMenuSeed};
}

/// The label of the system in which the [`NavRequest`] events are handled, the
/// focus state of the [`Focusable`]s is updated and the [`NavEvent`] events
/// are sent.
///
/// Systems updating visuals of UI elements should run _after_ the `NavRequestSystem`,
/// while systems that emit [`NavRequest`] should run _before_ it. For example, the
/// [`systems::default_mouse_input`] systems should run before the `NavRequestSystem`.
///
/// Failing to do so won't cause logical errors, but will make the UI feel more slugish
/// than necessary. This is especially critical of you are running on low framerate.
///
/// # Example
///
/// ```rust, no_run
/// use bevy::prelude::*;
/// use bevy_ui_navigation::{NavRequestSystem, DefaultNavigationPlugins};
/// # fn button_system() {}
/// fn main() {
///     App::new()
///         .add_plugins(DefaultPlugins)
///         .add_plugins(DefaultNavigationPlugins)
///         // ...
///         // Add the button color update system after the focus update system
///         .add_system(button_system.after(NavRequestSystem))
///         // ...
///         .run();
/// }
/// ```
#[derive(Clone, Debug, Hash, PartialEq, Eq, SystemLabel)]
pub struct NavRequestSystem;

/// The navigation plugin.
///
/// Add it to your app with `.add_plugin(NavigationPlugin)` and send
/// [`NavRequest`]s to move focus within declared [`Focusable`] entities.
///
/// This means you'll also have to add manaully the systems from [`systems`]
/// and [`systems::InputMapping`]. You should prefer [`DefaultNavigationPlugins`]
/// if you don't want to bother with that.
///
/// # Note on generic parameters
///
/// The `MP` type parameter might seem complicated, but all you have to do
/// is for your type to implement [`SystemParam`] and [`MoveParam`].
/// See the [`resolve::UiProjectionQuery`] source code for implementation hints.
pub struct GenericNavigationPlugin<MP>(PhantomData<MP>);
unsafe impl<T> Send for GenericNavigationPlugin<T> {}
unsafe impl<T> Sync for GenericNavigationPlugin<T> {}

impl<MP: MoveParam> GenericNavigationPlugin<MP> {
    pub fn new() -> Self {
        Self(PhantomData)
    }
}
impl<MP: SystemParam + 'static> Plugin for GenericNavigationPlugin<MP>
where
    for<'w, 's> SystemParamItem<'w, 's, MP>: MoveParam,
{
    fn build(&self, app: &mut App) {
        app.add_event::<NavRequest>()
            .add_event::<NavEvent>()
            .insert_resource(NavLock::new())
            .add_system(resolve::listen_nav_requests::<MP>.label(NavRequestSystem))
            .add_system(resolve::set_first_focused)
            .add_system(resolve::insert_tree_menus)
            .add_system(named::resolve_named_menus.before(resolve::insert_tree_menus));
    }
}
