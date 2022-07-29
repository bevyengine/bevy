mod commands;
pub mod events;
mod menu;
mod named;
mod resolve;

use std::marker::PhantomData;

use bevy_app::prelude::*;
use bevy_ecs::{
    schedule::{ParallelSystemDescriptorCoercion, SystemLabel},
    system::{SystemParam, SystemParamItem},
};

pub use non_empty_vec::NonEmpty;
use resolve::TreeMenu;

pub mod prelude {
    pub use crate::events::{NavEvent, NavEventReaderExt, NavRequest};
    pub use crate::menu::{MenuBuilder, MenuSetting};
    pub use crate::resolve::{
        FocusAction, FocusState, Focusable, Focused, MenuNavigationStrategy, NavLock,
    };
}

/// The label of the system in which the [`NavRequest`] events are handled, the
/// focus state of the [`Focusable`]s is updated and the [`NavEvent`] events
/// are sent.
///
/// Systems updating visuals of UI elements should run _after_ the `NavRequestSystem`,
/// while systems that emit [`NavRequest`] should run _before_ it.
/// For example, an input system should run before the `NavRequestSystem`.
///
/// Failing to do so won't cause logical errors, but will make the UI feel more slugish
/// than necessary. This is especially critical of you are running on low framerate.
///
/// # Example
///
/// ```rust, no_run
/// use bevy_ui_navigation::prelude::*;
/// # #[derive(SystemParam)] struct MoveCursor3d<'w, 's>(PhantomData<&'w &'s ()>);
/// # impl<'w, 's> MenuNavigationStrategy for MoveCursor3d<'w, 's> {
/// #   fn resolve_2d<'a>(
/// #       &self,
/// #       focused: Entity,
/// #       direction: events::Direction,
/// #       cycles: bool,
/// #       siblings: &'a [Entity],
/// #   ) -> Option<&'a Entity> { None }
/// # }
/// # fn button_system() {}
/// fn main() {
///     App::new()
///         .add_plugin(NavigationPlugin::<MoveCursor3d>::new())
///         // ...
///         // Add the button color update system after the focus update system
///         .add_system(button_system.after(NavRequestSystem))
///         // ...
///         .run();
/// }
/// ```
///
/// [`NavRequest`]: prelude::NavRequest
/// [`NavEvent`]: prelude::NavEvent
/// [`Focusable`]: prelude::Focusable
#[derive(Clone, Debug, Hash, PartialEq, Eq, SystemLabel)]
pub struct NavRequestSystem;

/// The navigation plugin.
///
/// Add it to your app with `.add_plugin(NavigationPlugin::new())` and send
/// [`NavRequest`]s to move focus within declared [`Focusable`] entities.
///
/// You should prefer `bevy_ui` provided defaults
/// if you don't want to bother with that.
///
/// # Note on generic parameters
///
/// The `STGY` type parameter might seem complicated, but all you have to do
/// is for your type to implement [`SystemParam`] and [`MenuNavigationStrategy`].
///
/// [`MenuNavigationStrategy`]: resolve::MenuNavigationStrategy
/// [`Focusable`]: prelude::Focusable
/// [`NavRequest`]: prelude::NavRequest
#[derive(Default)]
pub struct NavigationPlugin<STGY>(PhantomData<fn() -> STGY>);

impl<STGY: resolve::MenuNavigationStrategy> NavigationPlugin<STGY> {
    pub fn new() -> Self {
        Self(PhantomData)
    }
}
impl<STGY: SystemParam + 'static> Plugin for NavigationPlugin<STGY>
where
    for<'w, 's> SystemParamItem<'w, 's, STGY>: resolve::MenuNavigationStrategy,
{
    fn build(&self, app: &mut App) {
        // Reflection
        app.register_type::<resolve::Focusable>()
            .register_type::<resolve::FocusState>()
            .register_type::<resolve::FocusAction>()
            .register_type::<menu::MenuSetting>()
            .register_type::<TreeMenu>();

        app.add_event::<events::NavRequest>()
            .add_event::<events::NavEvent>()
            .insert_resource(resolve::NavLock::new())
            .add_system(resolve::set_first_focused.before(NavRequestSystem))
            .add_system(resolve::listen_nav_requests::<STGY>.label(NavRequestSystem))
            // PostUpdate because we want the Menus to be setup correctly before the
            // next call to `set_first_focused`, which depends on the Menu tree layout
            // existing already to chose a "intuitively correct" first focusable.
            // The user is most likely to spawn his UI in the Update stage, so it makes
            // sense to react to changes in the PostUpdate stage.
            .add_system_to_stage(
                CoreStage::PostUpdate,
                named::resolve_named_menus.before(resolve::insert_tree_menus),
            )
            .add_system_to_stage(CoreStage::PostUpdate, resolve::insert_tree_menus);
    }
}
