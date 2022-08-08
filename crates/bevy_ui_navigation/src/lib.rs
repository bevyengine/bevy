mod commands;
pub mod events;
mod focusable;
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
    pub use crate::focusable::{FocusAction, FocusState, Focusable, Focused};
    pub use crate::menu::{MenuBuilder, MenuSetting};
    pub use crate::resolve::{MenuNavigationStrategy, NavLock};
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
/// use bevy_ui_navigation::{prelude::*, events, NavRequestSystem, NavigationPlugin};
/// # use bevy_ecs::{prelude::*, system::SystemParam};
/// # use bevy_app::prelude::*;
/// # use std::marker::PhantomData;
/// # #[derive(SystemParam)]
/// # struct MoveCursor3d<'w, 's> {
/// #     #[system_param(ignore)]
/// #     _f: PhantomData<fn() -> (&'w (), &'s ())>,
/// # }
/// # use events::Direction as D;
/// # impl<'w, 's> MenuNavigationStrategy for MoveCursor3d<'w, 's> {
/// #     fn resolve_2d<'a>(&self, _: Entity, _: D, _: bool, _: &'a [Entity]) -> Option<&'a Entity> {
/// #         None
/// #     }
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
        app.register_type::<focusable::Focusable>()
            .register_type::<focusable::FocusState>()
            .register_type::<focusable::FocusAction>()
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
                CoreStage::PreUpdate,
                named::resolve_named_menus.before(menu::insert_tree_menus),
            )
            .add_system_to_stage(CoreStage::PreUpdate, menu::insert_tree_menus)
            .add_startup_system_to_stage(
                StartupStage::PostStartup,
                named::resolve_named_menus.before(menu::insert_tree_menus),
            )
            .add_startup_system_to_stage(StartupStage::PostStartup, menu::insert_tree_menus);
    }
}

#[cfg(test)]
mod test {
    use bevy_core::Name;
    use bevy_ecs::{
        event::Event,
        prelude::{Entity, Events},
        query::With,
        world::{EntityMut, World},
    };
    use bevy_hierarchy::BuildWorldChildren;
    use prelude::{
        FocusState, Focusable, Focused, MenuBuilder, MenuNavigationStrategy, MenuSetting, NavEvent,
        NavRequest,
    };

    use super::*;
    // Why things might fail?
    // -> State becomes inconsistent, assumptions are broken
    // How would assumptions be broken?
    // -> The ECS hierarchy changed under our feet
    // -> state was modified by users and we didn't expect it
    // -> internal state is not updated correctly to reflect the actual state
    // Consistency design:
    // - Strong dependency on bevy hierarchy not being mucked with
    //   (doesn't handle changed parents well)
    // - Need to get rid of TreeMenu::active_child probably
    // - Possible to "check and fix" the state in a system that accepts
    //   Changed<Parent> + RemovedComponent<Focusable | TreeMenu | Parent>
    // - But the check cannot anticipate when the hierarchy is changed,
    //   so we are doomed to expose to users inconsistent states
    //   -> implication: we don't need to maintain it in real time, since
    //      after certain hierarchy manipulations, it will be inconsistent either way.
    //      So we could do with only checking and updating when receiving
    //      NavRequest (sounds like good use case for system chaining)

    /// Define a menu structure to spawn.
    ///
    /// This just describes the menu structure,  use [`SpawnHierarchy::spawn`],
    /// to spawn the entities in the world,.
    enum SpawnHierarchy {
        Rootless(SpawnRootless),
        Menu(SpawnMenu),
    }
    impl SpawnHierarchy {
        fn spawn(self, world: &mut World) {
            match self {
                Self::Rootless(menu) => menu.spawn(world),
                Self::Menu(menu) => menu.spawn(&mut world.spawn()),
            };
        }
    }
    struct SpawnFocusable {
        name: &'static str,
        prioritized: bool,
        child_menu: Option<SpawnMenu>,
    }

    impl SpawnFocusable {
        fn spawn(self, mut entity: EntityMut) {
            let SpawnFocusable {
                name,
                prioritized,
                child_menu,
            } = self;
            entity.insert(Name::new(name));
            let focusable = if prioritized {
                Focusable::new().prioritized()
            } else {
                Focusable::new()
            };
            entity.insert(focusable);
            if let Some(child_menu) = child_menu {
                // SAFETY: we do not call any methods on `entity` after `world_mut()`
                unsafe {
                    child_menu.spawn(&mut entity.world_mut().spawn());
                };
                std::mem::drop(entity);
            }
        }
    }
    struct SpawnMenu {
        name: &'static str,
        children: Vec<SpawnFocusable>,
    }
    impl SpawnMenu {
        fn spawn(self, entity: &mut EntityMut) {
            let SpawnMenu { name, children } = self;
            let parent_focusable = name.strip_suffix(" Menu");
            let menu_builder = match parent_focusable {
                Some(name) => MenuBuilder::from_named(name),
                None => MenuBuilder::Root,
            };
            entity.insert_bundle((Name::new(name), menu_builder, MenuSetting::new()));
            entity.with_children(|commands| {
                for child in children.into_iter() {
                    child.spawn(commands.spawn());
                }
            });
        }
    }
    struct SpawnRootless {
        focusables: Vec<SpawnFocusable>,
    }
    impl SpawnRootless {
        fn spawn(self, world: &mut World) {
            for focusable in self.focusables.into_iter() {
                focusable.spawn(world.spawn())
            }
        }
    }
    /// Define a `SpawnHierarchy`.
    ///
    /// Syntax:
    /// - `spawn_hierarchy![ <focus_kind>, ... ]`:
    ///   A hierarchy of focusable components with a root menu.
    /// - `spawn_hierarchy!(@rootless [ <focus_kind> , ...] )`:
    ///   A hierarchy of focusable components **without** a root menu.
    /// - `<focus_kind>` is one of the following:
    ///   - `focusable("Custom")`: a focusable with the `Name::new("Custom")` component
    ///   - `focusable_to("Custom" [ <focus_kind> , ...] )`:
    ///     a focusable with the `Name::new("Custom")` component, parent of a menu (`MenuBuilder`)
    ///     marked with the `Name::new("Custom Menu")` component. The menu content is the
    ///     content of the provided list
    ///   - `prioritized("Custom")`: a focusable with the `Name::new("Custom")` component,
    ///     spawned with `Focusable::new().prioritized()`.
    macro_rules! spawn_hierarchy {
        ( @rootless [ $( $elem_kind:ident $elem_args:tt ),* $(,)? ] ) => (
            SpawnHierarchy::Rootless(SpawnRootless {
                focusables: vec![ $(
                    spawn_hierarchy!(@elem $elem_kind $elem_args),
                )* ],
            })
        );
        ( @menu $name:expr, $( $elem_name:ident $elem_args:tt ),* $(,)? ) => (
            SpawnMenu {
                name: $name,
                children: vec![ $(
                    spawn_hierarchy!(@elem $elem_name $elem_args),
                )* ],
            }
        );
        ( @elem prioritized ( $name:literal ) ) => (
            SpawnFocusable {
                name: $name,
                prioritized: true,
                child_menu: None,
            }
        );
        ( @elem focusable ( $name:literal ) ) => (
            SpawnFocusable {
                name: $name,
                prioritized: false,
                child_menu: None,
            }
        );
        ( @elem focusable_to ( $name:literal [ $( $submenu:tt )* ] ) ) => (
            SpawnFocusable {
                name: $name,
                prioritized: false,
                child_menu: Some( spawn_hierarchy!(@menu concat!( $name , " Menu"),  $( $submenu )* ) ),
            }
        );
        ($( $elem_name:ident $elem_args:tt ),* $(,)? ) => (
            SpawnHierarchy::Menu(spawn_hierarchy!(@menu "Root", $( $elem_name $elem_args ),*))
        );
    }

    /// Assert identity of a list of entities by their `Name` component
    /// (makes understanding test failures easier)
    ///
    /// This is a macro, so that when there is an assert failure or panic,
    /// the line of code it points to is the calling site,
    /// rather than the function body.
    ///
    /// There is nothing beside that that would prevent converting this into a function.
    macro_rules! assert_expected_focus_change {
        ($app:expr, $events:expr, $expected_from:expr, $expected_to:expr $(,)?) => {
            if let [NavEvent::FocusChanged { to, from }] = $events {
                let actual_from = $app.name_list(&*from);
                assert_eq!(&*actual_from, $expected_from);

                let actual_to = $app.name_list(&*to);
                assert_eq!(&*actual_to, $expected_to);
            } else {
                panic!(
                    "Expected a signle FocusChanged NavEvent, got: {:#?}",
                    $events
                );
            }
        };
    }

    // A navigation strategy that does nothing, useful for testing.
    #[derive(SystemParam)]
    struct MockNavigationStrategy<'w, 's> {
        #[system_param(ignore)]
        _f: PhantomData<fn() -> (&'w (), &'s ())>,
    }
    // Just to make the next `impl` block shorter, unused otherwise.
    use events::Direction as D;
    impl<'w, 's> MenuNavigationStrategy for MockNavigationStrategy<'w, 's> {
        fn resolve_2d<'a>(&self, _: Entity, _: D, _: bool, _: &'a [Entity]) -> Option<&'a Entity> {
            None
        }
    }
    fn receive_events<E: Event + Clone>(world: &World) -> Vec<E> {
        let events = world.resource::<Events<E>>();
        events.iter_current_update_events().cloned().collect()
    }

    /// Wrapper around `App` to make it easier to test the navigation systems.
    struct NavEcsMock {
        app: App,
    }
    impl NavEcsMock {
        fn currently_focused(&mut self) -> &str {
            let mut query = self.app.world.query_filtered::<&Name, With<Focused>>();
            &**query.iter(&self.app.world).next().unwrap()
        }
        fn kill_named(&mut self, to_kill: &str) -> Vec<NavEvent> {
            let mut query = self.app.world.query::<(Entity, &Name)>();
            let requested = query
                .iter(&self.app.world)
                .find_map(|(e, name)| (&**name == to_kill).then(|| e));
            if let Some(to_kill) = requested {
                self.app.world.despawn(to_kill);
            }
            self.app.update();
            receive_events(&mut self.app.world)
        }
        fn name_list(&mut self, entity_list: &[Entity]) -> Vec<&str> {
            let mut query = self.app.world.query::<&Name>();
            entity_list
                .iter()
                .filter_map(|e| query.get(&self.app.world, *e).ok())
                .map(|name| &**name)
                .collect()
        }
        fn new(hierarchy: SpawnHierarchy) -> Self {
            let mut app = App::new();
            app.add_plugin(NavigationPlugin::<MockNavigationStrategy>::new());
            hierarchy.spawn(&mut app.world);
            // Run once to convert the `MenuSetting` and `MenuBuilder` into `TreeMenu`.
            app.update();

            Self { app }
        }
        fn run_focus_on(&mut self, entity_name: &str) -> Vec<NavEvent> {
            let mut query = self.app.world.query::<(Entity, &Name)>();
            let requested = query
                .iter(&self.app.world)
                .find_map(|(e, name)| (&**name == entity_name).then(|| e))
                .unwrap();
            self.app.world.send_event(NavRequest::FocusOn(requested));
            self.app.update();
            receive_events(&mut self.app.world)
        }
        fn run_request(&mut self, request: NavRequest) -> Vec<NavEvent> {
            self.app.world.send_event(request);
            self.app.update();
            receive_events(&mut self.app.world)
        }
        fn state_of(&mut self, requested: &str) -> FocusState {
            let mut query = self.app.world.query::<(&Focusable, &Name)>();
            let requested = query
                .iter(&self.app.world)
                .find_map(|(focus, name)| (&**name == requested).then(|| focus));
            requested.unwrap().state()
        }
    }

    // ====
    // Expected basic functionalities
    // ====

    #[test]
    fn move_in_menuless() {
        let mut app = NavEcsMock::new(spawn_hierarchy!(@rootless [
            prioritized("Initial"),
            focusable("Left"),
            focusable("Right"),
        ]));
        assert_eq!(app.currently_focused(), "Initial");
        app.run_focus_on("Left");
        assert_eq!(app.currently_focused(), "Left");
    }

    #[test]
    fn deep_initial_focusable() {
        let mut app = NavEcsMock::new(spawn_hierarchy![
            focusable("Middle"),
            focusable_to("Left" [
                focusable("LCenter1"),
                focusable("LCenter2"),
                focusable_to("LTop" [
                    prioritized("LTopForward"),
                    focusable("LTopBackward"),
                ]),
                focusable("LCenter3"),
                focusable("LBottom"),
            ]),
            focusable("Right"),
        ]);
        use FocusState::{Active, Inert};
        assert_eq!(app.currently_focused(), "LTopForward");
        assert_eq!(app.state_of("Left"), Active);
        assert_eq!(app.state_of("Right"), Inert);
        assert_eq!(app.state_of("Middle"), Inert);
        assert_eq!(app.state_of("LTop"), Active);
        assert_eq!(app.state_of("LCenter1"), Inert);
        assert_eq!(app.state_of("LTopBackward"), Inert);
    }

    #[test]
    fn move_in_complex_menu_hierarchy() {
        let mut app = NavEcsMock::new(spawn_hierarchy![
            prioritized("Initial"),
            focusable_to("Left" [
                focusable_to("LTop" [
                    focusable("LTopForward"),
                    focusable("LTopBackward"),
                ]),
                focusable_to("LBottom" [
                    focusable("LBottomForward"),
                    focusable("LBottomForward1"),
                    focusable("LBottomForward2"),
                    prioritized("LBottomBackward"),
                    focusable("LBottomForward3"),
                    focusable("LBottomForward4"),
                    focusable("LBottomForward5"),
                ]),
            ]),
            focusable_to("Right" [
                focusable_to("RTop" [
                    focusable("RTopForward"),
                    focusable("RTopBackward"),
                ]),
                focusable("RBottom"),
            ]),
        ]);
        assert_eq!(app.currently_focused(), "Initial");

        // Move deep into a menu
        let events = app.run_focus_on("RBottom");
        assert_expected_focus_change!(app, &events[..], ["Initial"], ["RBottom", "Right"]);

        // Go up and back down several layers of menus
        let events = app.run_focus_on("LTopForward");
        assert_expected_focus_change!(
            app,
            &events[..],
            ["RBottom", "Right"],
            ["LTopForward", "LTop", "Left"],
        );
        // See if cancel event works
        let events = app.run_request(NavRequest::Cancel);
        assert_expected_focus_change!(app, &events[..], ["LTopForward", "LTop"], ["LTop"]);

        // Move to sibling within menu
        let events = app.run_focus_on("LBottom");
        assert_expected_focus_change!(app, &events[..], ["LTop"], ["LBottom"]);

        // Move down into menu by activating a focusable
        // (also make sure `prioritized` works)
        let events = app.run_request(NavRequest::Action);
        assert_expected_focus_change!(
            app,
            &events[..],
            ["LBottom"],
            ["LBottomBackward", "LBottom"]
        );
    }

    // ====
    // What happens when Focused element is killed
    // ====

    // Select a new focusable in the same menu (or anything if no menus exist)
    #[test]
    fn focus_rootless_kill_robust() {
        let mut app = NavEcsMock::new(spawn_hierarchy!(@rootless [
            prioritized("Initial"),
            focusable("Right"),
        ]));
        assert_eq!(app.currently_focused(), "Initial");
        app.kill_named("Initial");
        assert_eq!(app.currently_focused(), "Right");

        app.kill_named("Right");
        let events = app.run_request(NavRequest::Action);
        assert_eq!(events.len(), 0, "{:#?}", events);
    }

    // Go up the menu tree if it was the last focusable in the menu
    // And swap to something in the same menu if focusable killed in it.
    #[test]
    fn menu_elem_kill_robust() {
        let mut app = NavEcsMock::new(spawn_hierarchy![
            focusable_to("Left" [
                focusable("LTop"),
                focusable("LBottom"),
            ]),
            focusable_to("Antony" [
                prioritized("Caesar"),
                focusable("Brutus"),
            ]),
            focusable_to("Octavian" [
                focusable("RTop"),
                focusable("RBottom"),
            ]),
        ]);
        // NOTE: was broken because didn't properly set
        // active_child and Active when initial focus was given to
        // a deep element.
        assert_eq!(app.currently_focused(), "Caesar");
        assert_eq!(app.state_of("Antony"), FocusState::Active);
        app.kill_named("Caesar");
        assert_eq!(app.currently_focused(), "Brutus");
        app.kill_named("Brutus");
        assert_eq!(app.currently_focused(), "Antony");
    }

    // ====
    // removal of parent menu and focusables
    // ====

    /*
    // Relink the child menu to the removed parent's parents
    #[test]
    fn parent_menu_kill_robust() {
        todo!()
    }

    // Make sure this works with root as well
    #[test]
    fn root_menu_kill_robust() {
        todo!()
    }

    // Relink when the focusable parent of a menu is killed
    #[test]
    fn parent_focusable_kill_robust() {
        todo!()
    }

    */

    // ====
    // some reparenting potential problems
    // ====

    // Focused element is reparented to a new menu
    // Active element is reparented to a new menu
    // NOTE: those are not expected to work. Currently considered a user error.
}
