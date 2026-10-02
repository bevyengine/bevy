use bevy_app::{App, Plugin, PreUpdate};
use bevy_ecs::{
    change_detection::{DetectChanges, Ref},
    component::Component,
    entity::Entity,
    hierarchy::ChildOf,
    observer::On,
    query::With,
    reflect::ReflectComponent,
    schedule::IntoScheduleConfigs,
    system::{Commands, Query, Res},
};
use bevy_input::{
    keyboard::{KeyCode, KeyboardInput},
    ButtonState,
};
use bevy_input_focus::FocusedInput;
use bevy_math::Vec2;
use bevy_picking::{
    events::{PointerCancel, PointerDragEnd, PointerState},
    pointer::{PointerButton, PointerId, PointerLocation},
    Pickable, PickingSystems,
};
use bevy_reflect::{prelude::ReflectDefault, Reflect};
use bevy_ui::{ComputedNode, Node, PositionType, UiGlobalTransform, UiScale, Val};

/// Marks a UI ancestor that receives drag-proxy visuals.
///
/// Insert this component with [`Node`] on an ancestor of each drag source. The plugin uses the
/// nearest marked ancestor and its computed position as the coordinate origin for proxy placement.
#[derive(Component, Debug, Default, Clone, Copy, Reflect)]
#[reflect(Component, Default, Clone)]
pub struct DragOverlayRoot;

/// Makes a caller-supplied visual follow a pointer above its nearest [`DragOverlayRoot`].
///
/// `offset` is measured in logical UI pixels and is added after the pointer position is adjusted
/// for [`bevy_ui::UiScale`]. The proxy is made unpickable and is despawned when the pointer drag
/// completes or is cancelled. When this component is inserted, the visual must be a descendant of
/// the pointer's drag source so source-disappearance cleanup can retain that identity after the
/// proxy is reparented. The visual must have [`Node`], and an ancestor must have
/// [`DragOverlayRoot`]. An entity without [`Node`] is ignored. Without an overlay root, the proxy
/// keeps its current parent and its absolute position remains relative to that parent.
#[derive(Component, Debug, Clone, Copy, PartialEq, Reflect)]
#[reflect(Component, Clone, PartialEq)]
pub struct DragProxy {
    /// The pointer that controls this proxy.
    pub pointer_id: PointerId,
    /// The logical UI offset from the pointer position.
    pub offset: Vec2,
}

#[derive(Component)]
struct DragProxySource(Entity);

/// Plugin that manages the hierarchy, position, and lifetime of [`DragProxy`] entities.
///
/// Source-disappearance cleanup requires [`PointerState`], which is installed by Bevy's picking
/// plugins. Add [`DragOverlayRoot`] to a [`Node`] ancestor before inserting a proxy.
pub struct DragProxyPlugin;

impl Plugin for DragProxyPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<UiScale>()
            .add_observer(cleanup_drag_proxies_on_end)
            .add_observer(cleanup_drag_proxies_on_cancel)
            .add_observer(cleanup_drag_proxies_on_escape)
            .add_systems(PreUpdate, update_drag_proxies.after(PickingSystems::Last));
    }
}

fn update_drag_proxies(
    mut proxies: Query<(
        Entity,
        Ref<DragProxy>,
        &mut Node,
        Option<&ChildOf>,
        Option<&DragProxySource>,
    )>,
    overlay_roots: Query<
        (Option<&ComputedNode>, Option<&UiGlobalTransform>),
        With<DragOverlayRoot>,
    >,
    parents: Query<&ChildOf>,
    pointers: Query<(&PointerId, &PointerLocation)>,
    entities: Query<()>,
    pointer_state: Option<Res<PointerState>>,
    ui_scale: Res<UiScale>,
    mut commands: Commands,
) {
    for (entity, proxy, mut node, parent, tracked_source) in &mut proxies {
        let pointer_drag = pointer_state
            .as_ref()
            .and_then(|pointer_state| pointer_state.get(proxy.pointer_id, PointerButton::Primary));
        let captured_source = proxy
            .is_added()
            .then(|| {
                pointer_drag.and_then(|state| {
                    parents
                        .iter_ancestors(entity)
                        .find(|ancestor| state.dragging.contains_key(ancestor))
                })
            })
            .flatten();
        let source = tracked_source.map(|source| source.0).or(captured_source);
        let overlay_root = parents
            .iter_ancestors(entity)
            .find(|ancestor| overlay_roots.contains(*ancestor));

        if proxy.is_added()
            && let Some(root) = overlay_root
            && parent.is_none_or(|parent| parent.parent() != root)
        {
            commands.entity(entity).insert(ChildOf(root));
        }
        if proxy.is_added() {
            commands.entity(entity).insert(Pickable::IGNORE);
            if let Some(source) = source {
                commands.entity(entity).insert(DragProxySource(source));
            }
        }

        if pointer_state.is_some() {
            let has_live_source = source.is_some_and(|source| {
                entities.contains(source)
                    && pointer_drag.is_some_and(|state| state.dragging.contains_key(&source))
            });
            if !has_live_source {
                commands.entity(entity).try_despawn();
                continue;
            }
        }

        let Some(location) = pointers.iter().find_map(|(pointer_id, location)| {
            (*pointer_id == proxy.pointer_id)
                .then(|| location.location())
                .flatten()
        }) else {
            continue;
        };
        let scale = ui_scale.0.max(f32::EPSILON);
        let overlay_origin = overlay_root
            .and_then(|root| overlay_roots.get(root).ok())
            .and_then(|(node, transform)| Some((node?, transform?)))
            .map(|(node, transform)| {
                (transform.translation - node.size() * 0.5) * node.inverse_scale_factor()
            })
            .unwrap_or(Vec2::ZERO);
        let position = location.position / scale + proxy.offset - overlay_origin;
        node.position_type = PositionType::Absolute;
        node.left = Val::Px(position.x);
        node.top = Val::Px(position.y);
    }
}

fn despawn_pointer_proxies(
    pointer_id: PointerId,
    proxies: &Query<(Entity, &DragProxy)>,
    commands: &mut Commands,
) {
    for (entity, proxy) in proxies.iter() {
        if proxy.pointer_id == pointer_id {
            commands.entity(entity).try_despawn();
        }
    }
}

fn cleanup_drag_proxies_on_end(
    event: On<PointerDragEnd>,
    proxies: Query<(Entity, &DragProxy)>,
    mut commands: Commands,
) {
    despawn_pointer_proxies(event.pointer.id, &proxies, &mut commands);
}

fn cleanup_drag_proxies_on_cancel(
    event: On<PointerCancel>,
    proxies: Query<(Entity, &DragProxy)>,
    mut commands: Commands,
) {
    despawn_pointer_proxies(event.pointer.id, &proxies, &mut commands);
}

fn cleanup_drag_proxies_on_escape(
    event: On<FocusedInput<KeyboardInput>>,
    proxies: Query<Entity, With<DragProxy>>,
    mut commands: Commands,
) {
    if event.input.state == ButtonState::Pressed
        && !event.input.repeat
        && event.input.key_code == KeyCode::Escape
    {
        for proxy in proxies.iter() {
            commands.entity(proxy).try_despawn();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy_app::PreUpdate;
    use bevy_camera::NormalizedRenderTarget;
    use bevy_ecs::hierarchy::{ChildOf, Children};
    use bevy_input::{keyboard::Key, InputPlugin};
    use bevy_input_focus::{InputDispatchPlugin, InputFocusPlugin};
    use bevy_picking::{
        backend::HitData,
        events::{DragEntry, Pointer, PointerCancel, PointerDragEnd, PointerState},
        pointer::{Location, PointerButton, PointerLocation, PointerMap},
        Pickable,
    };
    use bevy_ui::{Node, PositionType, UiScale, Val};
    use bevy_window::{PrimaryWindow, Window, WindowRef};

    #[test]
    fn proxy_reparents_to_nearest_overlay_and_tracks_scaled_pointer() {
        let mut app = App::new();
        app.add_plugins(DragProxyPlugin);
        app.insert_resource(UiScale(2.0));
        app.init_resource::<PointerState>();
        app.init_resource::<PointerMap>();
        let window = app
            .world_mut()
            .spawn((Window::default(), PrimaryWindow))
            .id();
        let overlay = app
            .world_mut()
            .spawn((
                DragOverlayRoot,
                ComputedNode {
                    size: Vec2::new(40.0, 20.0),
                    inverse_scale_factor: 0.5,
                    ..Default::default()
                },
                UiGlobalTransform::from_xy(40.0, 60.0),
            ))
            .id();
        let source = app.world_mut().spawn(ChildOf(overlay)).id();
        let other_source = app.world_mut().spawn(ChildOf(overlay)).id();
        let location = Location {
            target: NormalizedRenderTarget::Window(
                WindowRef::Entity(window).normalize(Some(window)).unwrap(),
            ),
            position: Vec2::new(100.0, 200.0),
        };
        app.world_mut()
            .spawn((PointerId::Mouse, PointerLocation::new(location.clone())));
        app.world_mut()
            .resource_mut::<PointerState>()
            .get_mut(PointerId::Mouse, PointerButton::Primary)
            .dragging
            .insert(
                source,
                DragEntry {
                    start_pos: location.position,
                    latest_pos: location.position,
                },
            );
        app.world_mut()
            .resource_mut::<PointerState>()
            .get_mut(PointerId::Mouse, PointerButton::Primary)
            .dragging
            .insert(
                other_source,
                DragEntry {
                    start_pos: Vec2::ZERO,
                    latest_pos: Vec2::ZERO,
                },
            );
        let proxy = app
            .world_mut()
            .spawn((
                Node::default(),
                ChildOf(source),
                DragProxy {
                    pointer_id: PointerId::Mouse,
                    offset: Vec2::new(10.0, 5.0),
                },
            ))
            .id();

        app.world_mut().run_schedule(PreUpdate);

        assert_eq!(
            app.world()
                .entity(proxy)
                .get::<ChildOf>()
                .map(ChildOf::parent),
            Some(overlay)
        );
        assert!(app
            .world()
            .entity(overlay)
            .get::<Children>()
            .is_some_and(|children| children.contains(&proxy)));
        assert_eq!(
            app.world().entity(proxy).get::<Pickable>(),
            Some(&Pickable::IGNORE)
        );
        let node = app.world().entity(proxy).get::<Node>().unwrap();
        assert_eq!(node.position_type, PositionType::Absolute);
        assert_eq!(node.left, Val::Px(50.0));
        assert_eq!(node.top, Val::Px(80.0));
    }

    #[test]
    fn completing_one_pointer_drag_only_despawns_its_proxy() {
        let mut app = App::new();
        app.set_error_handler(bevy_ecs::error::panic);
        app.add_plugins(DragProxyPlugin);
        app.init_resource::<PointerState>();
        app.init_resource::<PointerMap>();
        let window = app
            .world_mut()
            .spawn((Window::default(), PrimaryWindow))
            .id();
        let overlay = app.world_mut().spawn(DragOverlayRoot).id();
        let mouse_source = app.world_mut().spawn(ChildOf(overlay)).id();
        let touch_source = app.world_mut().spawn(ChildOf(overlay)).id();
        let location = Location {
            target: NormalizedRenderTarget::Window(
                WindowRef::Entity(window).normalize(Some(window)).unwrap(),
            ),
            position: Vec2::ZERO,
        };
        for (pointer_id, source) in [
            (PointerId::Mouse, mouse_source),
            (PointerId::Touch(1), touch_source),
        ] {
            app.world_mut()
                .spawn((pointer_id, PointerLocation::new(location.clone())));
            app.world_mut()
                .resource_mut::<PointerState>()
                .get_mut(pointer_id, PointerButton::Primary)
                .dragging
                .insert(
                    source,
                    DragEntry {
                        start_pos: Vec2::ZERO,
                        latest_pos: Vec2::ZERO,
                    },
                );
        }
        let mouse_proxy = app
            .world_mut()
            .spawn((
                Node::default(),
                ChildOf(mouse_source),
                DragProxy {
                    pointer_id: PointerId::Mouse,
                    offset: Vec2::ZERO,
                },
            ))
            .id();
        let touch_proxy = app
            .world_mut()
            .spawn((
                Node::default(),
                ChildOf(touch_source),
                DragProxy {
                    pointer_id: PointerId::Touch(1),
                    offset: Vec2::ZERO,
                },
            ))
            .id();
        app.update();

        app.world_mut().trigger(PointerDragEnd {
            entity: mouse_source,
            pointer: Pointer::new(PointerId::Mouse, location),
            button: PointerButton::Primary,
            distance: Vec2::ONE,
        });
        app.world_mut()
            .resource_mut::<PointerState>()
            .get_mut(PointerId::Mouse, PointerButton::Primary)
            .dragging
            .clear();
        app.update();

        assert!(app.world().get_entity(mouse_proxy).is_err());
        assert!(app.world().get_entity(touch_proxy).is_ok());
    }

    #[test]
    fn proxy_is_cleaned_up_when_drag_source_disappears() {
        let mut app = App::new();
        app.add_plugins(DragProxyPlugin);
        app.init_resource::<PointerState>();
        app.init_resource::<PointerMap>();
        let window = app
            .world_mut()
            .spawn((Window::default(), PrimaryWindow))
            .id();
        let overlay = app.world_mut().spawn(DragOverlayRoot).id();
        let source = app.world_mut().spawn(ChildOf(overlay)).id();
        let other_source = app.world_mut().spawn(ChildOf(overlay)).id();
        let location = Location {
            target: NormalizedRenderTarget::Window(
                WindowRef::Entity(window).normalize(Some(window)).unwrap(),
            ),
            position: Vec2::ZERO,
        };
        app.world_mut()
            .spawn((PointerId::Mouse, PointerLocation::new(location)));
        app.world_mut()
            .resource_mut::<PointerState>()
            .get_mut(PointerId::Mouse, PointerButton::Primary)
            .dragging
            .insert(
                source,
                DragEntry {
                    start_pos: Vec2::ZERO,
                    latest_pos: Vec2::ZERO,
                },
            );
        app.world_mut()
            .resource_mut::<PointerState>()
            .get_mut(PointerId::Mouse, PointerButton::Primary)
            .dragging
            .insert(
                other_source,
                DragEntry {
                    start_pos: Vec2::ZERO,
                    latest_pos: Vec2::ZERO,
                },
            );
        let proxy = app
            .world_mut()
            .spawn((
                Node::default(),
                ChildOf(source),
                DragProxy {
                    pointer_id: PointerId::Mouse,
                    offset: Vec2::ZERO,
                },
            ))
            .id();
        app.update();
        assert_eq!(
            app.world()
                .entity(proxy)
                .get::<ChildOf>()
                .map(ChildOf::parent),
            Some(overlay)
        );
        assert_eq!(
            app.world()
                .entity(proxy)
                .get::<DragProxySource>()
                .map(|source| source.0),
            Some(source)
        );

        app.world_mut().despawn(source);
        app.update();

        assert!(app.world().get_entity(proxy).is_err());
    }

    #[test]
    fn pointer_cancellation_cleans_up_matching_proxy() {
        let mut app = App::new();
        app.add_plugins(DragProxyPlugin);
        let window = app
            .world_mut()
            .spawn((Window::default(), PrimaryWindow))
            .id();
        let source = app.world_mut().spawn_empty().id();
        let proxy = app
            .world_mut()
            .spawn((
                Node::default(),
                DragProxy {
                    pointer_id: PointerId::Mouse,
                    offset: Vec2::ZERO,
                },
            ))
            .id();
        let location = Location {
            target: NormalizedRenderTarget::Window(
                WindowRef::Entity(window).normalize(Some(window)).unwrap(),
            ),
            position: Vec2::ZERO,
        };

        app.world_mut().trigger(PointerCancel {
            entity: source,
            pointer: Pointer::new(PointerId::Mouse, location),
            hit: HitData::new(window, 0.0, None, None),
        });
        app.update();

        assert!(app.world().get_entity(proxy).is_err());
    }

    #[test]
    fn escape_cleans_up_all_proxies() {
        let mut app = App::new();
        app.add_plugins((
            InputPlugin,
            InputFocusPlugin,
            InputDispatchPlugin,
            DragProxyPlugin,
        ));
        let window = app
            .world_mut()
            .spawn((Window::default(), PrimaryWindow))
            .id();
        let proxies = [PointerId::Mouse, PointerId::Touch(1)].map(|pointer_id| {
            app.world_mut()
                .spawn((
                    Node::default(),
                    DragProxy {
                        pointer_id,
                        offset: Vec2::ZERO,
                    },
                ))
                .id()
        });
        app.update();

        app.world_mut().write_message(KeyboardInput {
            key_code: KeyCode::Escape,
            logical_key: Key::Escape,
            state: ButtonState::Pressed,
            text: None,
            repeat: false,
            window,
        });
        app.update();

        assert!(proxies
            .into_iter()
            .all(|proxy| app.world().get_entity(proxy).is_err()));
    }
}
