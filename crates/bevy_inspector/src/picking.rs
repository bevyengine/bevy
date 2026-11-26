use bevy_camera::{
    visibility::{InheritedVisibility, RenderLayers},
    Camera, RenderTarget,
};
use bevy_ecs::{
    change_detection::DetectChangesMut,
    entity::Entity,
    hierarchy::{ChildOf, Children},
    message::MessageReader,
    query::With,
    resource::Resource,
    system::{Commands, In, Local, Query, Res, ResMut, SystemParam},
};
use bevy_input::{mouse::MouseButton, touch::Touches, ButtonInput};
use bevy_picking::{
    backend::{ray::RayMap, PointerHits},
    events::PointerState,
    hover::{HoverMap, PickingInteraction, PreviousHoverMap},
    mesh_picking::ray_cast::{MeshRayCast, MeshRayCastSettings},
    pointer::{Location, PointerAction, PointerButton, PointerId, PointerInput},
    Pickable, PickingSettings,
};
use bevy_platform::collections::{HashMap, HashSet};
use bevy_ui::{ComputedNode, ComputedUiTargetCamera, Display, Node, UiGlobalTransform};
use bevy_ui_widgets::{DockChange, DockEdit, DockLeaf, DockPanelContent, DockTree, ValueChange};
use bevy_window::{PrimaryWindow, Window};

use crate::{
    entity_tree::{InspectorUi, TreeReveal},
    overlay::{InspectorPanel, OverlayDock},
    world_tabs::{InspectorPage, InspectorPageBody, InspectorPageTab},
    InspectableEntity, InspectorMode, InspectorSelection, InspectorSource, InspectorSources,
};

#[derive(SystemParam)]
pub(crate) struct InspectorPanels<'w, 's> {
    inspector_ui: Query<'w, 's, Entity, With<InspectorUi>>,
    pickables: Query<'w, 's, &'static Pickable>,
    children: Query<'w, 's, &'static Children>,
    parents: Query<'w, 's, &'static ChildOf>,
    nodes: Query<
        'w,
        's,
        (
            &'static ComputedNode,
            &'static UiGlobalTransform,
            &'static InheritedVisibility,
            &'static ComputedUiTargetCamera,
        ),
    >,
    cameras: Query<'w, 's, (&'static Camera, &'static RenderTarget)>,
    primary_window: Query<'w, 's, Entity, With<PrimaryWindow>>,
}

impl InspectorPanels<'_, '_> {
    fn contains(&self, entity: Entity) -> bool {
        core::iter::once(entity)
            .chain(self.parents.iter_ancestors(entity))
            .any(|entity| self.inspector_ui.contains(entity))
    }

    fn under(&self, location: &Location) -> bool {
        self.inspector_ui
            .iter()
            .any(|root| self.blocks(root, location))
    }

    fn blocks(&self, entity: Entity, location: &Location) -> bool {
        if self.pickables.get(entity) != Ok(&Pickable::IGNORE) {
            return self.panel_under(entity, location);
        }
        self.children.get(entity).is_ok_and(|children| {
            children
                .iter()
                .copied()
                .any(|child| self.blocks(child, location))
        })
    }

    fn panel_under(&self, panel: Entity, location: &Location) -> bool {
        {
            let Ok((node, transform, visibility, target)) = self.nodes.get(panel) else {
                return false;
            };
            let Some((camera, render_target)) = target
                .get()
                .and_then(|camera| self.cameras.get(camera).ok())
            else {
                return false;
            };
            if !visibility.get()
                || node.is_empty()
                || render_target.normalize(self.primary_window.single().ok())
                    != Some(location.target.clone())
            {
                return false;
            }
            let mut point = location.position * camera.target_scaling_factor().unwrap_or(1.0);
            if let Some(viewport) = camera.physical_viewport_rect() {
                point -= viewport.min.as_vec2();
            }
            node.contains_point(*transform, point)
        }
    }
}

/// Whether the app added `MeshPickingPlugin`, which picks meshes itself.
#[derive(Resource, Debug, Default, Clone, Copy, PartialEq, Eq)]
pub(crate) struct MeshBackendPresent(pub bool);

#[derive(SystemParam)]
pub(crate) struct MeshFallback<'w, 's> {
    backend: Option<Res<'w, MeshBackendPresent>>,
    ray_map: Option<Res<'w, RayMap>>,
    cameras: Query<'w, 's, (&'static Camera, Option<&'static RenderLayers>)>,
    pickables: Query<'w, 's, &'static Pickable>,
    layers: Query<'w, 's, &'static RenderLayers>,
    ray_cast: Option<MeshRayCast<'w, 's>>,
}

impl MeshFallback<'_, '_> {
    fn nearest(&mut self, pointer: PointerId) -> Option<Entity> {
        if self.backend.as_ref().is_some_and(|backend| backend.0) {
            return None;
        }
        let ray_map = self.ray_map.as_ref()?;
        let ray_cast = self.ray_cast.as_mut()?;
        let mut nearest: Option<(isize, f32, Entity)> = None;
        for (ray_id, ray) in ray_map.iter() {
            if ray_id.pointer != pointer {
                continue;
            }
            let Ok((camera, camera_layers)) = self.cameras.get(ray_id.camera) else {
                continue;
            };
            let camera_layers = camera_layers.cloned().unwrap_or_default();
            let filter = |entity: Entity| {
                let entity_layers = self.layers.get(entity).cloned().unwrap_or_default();
                camera_layers.intersects(&entity_layers)
                    && self
                        .pickables
                        .get(entity)
                        .ok()
                        .is_none_or(|pickable| pickable.is_hoverable)
            };
            let early_exit = |entity: Entity| {
                self.pickables
                    .get(entity)
                    .is_ok_and(|pickable| pickable.should_block_lower)
            };
            let settings = MeshRayCastSettings::default()
                .with_filter(&filter)
                .with_early_exit_test(&early_exit);
            let Some((entity, hit)) = ray_cast.cast_ray(*ray, &settings).first() else {
                continue;
            };
            let closer = nearest.is_none_or(|(order, distance, _)| {
                camera.order > order || (camera.order == order && hit.distance < distance)
            });
            if closer {
                nearest = Some((camera.order, hit.distance, *entity));
            }
        }
        nearest.map(|(.., entity)| entity)
    }
}

#[derive(SystemParam)]
pub(crate) struct EntitiesPage<'w, 's> {
    tabs: Query<'w, 's, (Entity, &'static InspectorPageTab, &'static ChildOf)>,
    bodies: Query<'w, 's, (&'static InspectorPageBody, &'static Node, &'static ChildOf)>,
    parents: Query<'w, 's, &'static ChildOf>,
    contents: Query<'w, 's, &'static DockPanelContent>,
    docks: Query<'w, 's, &'static DockTree, With<OverlayDock>>,
    commands: Commands<'w, 's>,
}

impl EntitiesPage<'_, '_> {
    fn show(&mut self) {
        for content in &self.contents {
            if content.panel.as_str() != InspectorPanel::Entities.key() {
                continue;
            }
            let Ok(tree) = self.docks.get(content.dock) else {
                continue;
            };
            let Some(leaf) = tree.find_leaf_for_tab(content.tab) else {
                continue;
            };
            if tree.leaf(leaf).and_then(DockLeaf::active) != Some(content.tab) {
                self.commands.trigger(DockChange {
                    dock: content.dock,
                    edit: DockEdit::Activate {
                        leaf,
                        tab: content.tab,
                    },
                    is_final: true,
                });
            }
        }
        for (body, node, root) in &self.bodies {
            if body.0 != InspectorPage::Entities || node.display != Display::None {
                continue;
            }
            let tab = self.tabs.iter().find(|(_, tab, strip)| {
                tab.0 == InspectorPage::Entities
                    && self.parents.get(strip.parent()).map(ChildOf::parent) == Ok(root.parent())
            });
            if let Some((tab, _, strip)) = tab {
                self.commands.trigger(ValueChange {
                    source: strip.parent(),
                    value: Some(tab),
                    is_final: true,
                });
            }
        }
    }
}

#[derive(SystemParam)]
pub(crate) struct PickTargets<'w, 's> {
    panels: InspectorPanels<'w, 's>,
    windows: Query<'w, 's, (), With<Window>>,
    pickables: Query<'w, 's, &'static Pickable>,
}

#[derive(SystemParam)]
pub(crate) struct Presses<'w, 's> {
    picks: Local<'s, HashMap<PointerId, Pick>>,
    held: Local<'s, HashSet<PointerId>>,
    ended: Local<'s, HashMap<PointerId, bool>>,
    pointers: Query<'w, 's, &'static PointerId>,
    mouse: Option<ResMut<'w, ButtonInput<MouseButton>>>,
    touches: Option<ResMut<'w, Touches>>,
}

impl Presses<'_, '_> {
    fn press(&mut self, pointer: PointerId, pick: Option<Pick>) {
        if let Some(pick) = pick {
            self.picks.insert(pointer, pick);
        }
        if let Some(pressed_again) = self.ended.get_mut(&pointer) {
            *pressed_again = true;
        }
        if pick.is_some_and(|pick| pick != Pick::Blocked) {
            self.held.insert(pointer);
        } else {
            self.held.remove(&pointer);
        }
    }

    fn release(&mut self, pointer: PointerId) -> Option<Pick> {
        if self.held.remove(&pointer) {
            self.ended.insert(pointer, false);
        } else {
            self.ended.remove(&pointer);
        }
        self.picks.remove(&pointer)
    }

    fn hide_raw_input(&mut self) {
        let Self {
            picks,
            held,
            ended,
            pointers,
            mouse,
            touches,
        } = self;
        let exists = |pointer: &PointerId| {
            !matches!(pointer, PointerId::Custom(_)) || pointers.iter().any(|id| id == pointer)
        };
        picks.retain(|pointer, _| exists(pointer));
        held.retain(exists);
        for pointer in held.iter() {
            match pointer {
                PointerId::Mouse => {
                    if let Some(mouse) = mouse.as_mut() {
                        mouse.reset(MouseButton::Left);
                    }
                }
                PointerId::Touch(id) => {
                    if let Some(touches) = touches.as_mut() {
                        touches.release(*id);
                        touches.clear_just_pressed(*id);
                        touches.clear_just_released(*id);
                        touches.clear_just_canceled(*id);
                    }
                }
                PointerId::Custom(_) => {}
            }
        }
        for (pointer, pressed_again) in ended.drain() {
            match (pointer, mouse.as_mut(), touches.as_mut()) {
                (PointerId::Mouse, Some(mouse), _) if !pressed_again => {
                    mouse.clear_just_pressed(MouseButton::Left);
                    mouse.clear_just_released(MouseButton::Left);
                }
                (PointerId::Touch(id), _, Some(touches)) => {
                    if !pressed_again {
                        touches.clear_just_pressed(id);
                    }
                    touches.clear_just_released(id);
                    touches.clear_just_canceled(id);
                }
                _ => {}
            }
        }
    }
}

/// What a press or release of the primary button landed on.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Pick {
    Entity(Entity),
    Blocked,
    Nothing,
}

impl PickTargets<'_, '_> {
    fn topmost(&self, hits: &[(f32, f32, Entity)], location: &Location) -> Option<Pick> {
        if self.panels.under(location) {
            return Some(Pick::Blocked);
        }
        for (.., entity) in hits {
            if self.windows.contains(*entity) {
                continue;
            }
            let pickable = self.pickables.get(*entity).copied().unwrap_or_default();
            if self.panels.contains(*entity) {
                if pickable.is_hoverable || pickable.should_block_lower {
                    return Some(Pick::Blocked);
                }
                continue;
            }
            if pickable.is_hoverable {
                return Some(Pick::Entity(*entity));
            }
            if pickable.should_block_lower {
                return Some(Pick::Blocked);
            }
        }
        None
    }
}

/// Picks for the inspector, and keeps the pointer's hover and events away from the app.
///
/// While the overlay is showing, the mode is [`InspectorMode::Edit`] and the inspected world is
/// local, [`HoverMap`] and [`PreviousHoverMap`] keep only the inspector's own UI, which is what
/// `bevy_picking` dispatches from. The app therefore sees no hover and no pointer events, while
/// the overlay's own controls keep working and the inspector still reads every backend's hits to
/// find the entity under the pointer. A press that lands on the app is also removed from
/// [`ButtonInput<MouseButton>`] or [`Touches`] until it ends, so systems from `Update` on do not
/// see it.
pub(crate) fn pick_for_inspector(
    In(visible): In<bool>,
    settings: Option<Res<PickingSettings>>,
    mode: Res<InspectorMode>,
    sources: Res<InspectorSources>,
    mut pointer_hits: MessageReader<PointerHits>,
    mut inputs: MessageReader<PointerInput>,
    mut hover_map: Option<ResMut<HoverMap>>,
    mut previous_hover_map: Option<ResMut<PreviousHoverMap>>,
    mut pointer_state: Option<ResMut<PointerState>>,
    mut presses: Presses,
    targets: PickTargets,
    mut meshes: MeshFallback,
    mut selection: ResMut<InspectorSelection>,
    mut reveal: ResMut<TreeReveal>,
    mut entities_page: EntitiesPage,
    mut commands: Commands,
) {
    let mut hits: HashMap<PointerId, Vec<(f32, f32, Entity)>> = HashMap::default();
    for reported in pointer_hits.read() {
        hits.entry(reported.pointer).or_default().extend(
            reported
                .picks
                .iter()
                .map(|(entity, hit)| (reported.order, hit.depth, *entity)),
        );
    }
    let inputs: Vec<(PointerId, PointerAction, Location)> = inputs
        .read()
        .filter(|input| {
            matches!(
                input.action,
                PointerAction::Press(PointerButton::Primary)
                    | PointerAction::Release(PointerButton::Primary)
                    | PointerAction::Cancel
                    | PointerAction::Move { .. }
                    | PointerAction::Scroll { .. }
            )
        })
        .map(|input| (input.pointer_id, input.action, input.location.clone()))
        .collect();

    let active = visible
        && *mode == InspectorMode::Edit
        && settings.as_ref().is_none_or(|settings| settings.is_enabled)
        && sources.sources.contains(&InspectorSource::Local);
    if !active {
        presses.picks.clear();
        for (pointer, action, _) in inputs {
            match action {
                PointerAction::Press(PointerButton::Primary) => presses.press(pointer, None),
                PointerAction::Release(PointerButton::Primary) | PointerAction::Cancel => {
                    presses.release(pointer);
                }
                _ => {}
            }
        }
        presses.hide_raw_input();
        return;
    }

    // Everything dispatched to the app comes from these two maps and the pointer's press and drag
    // state, so keeping only the inspector's own UI in them is what stops the app seeing the
    // pointer, without stopping our own controls.
    if let Some(map) = hover_map.as_mut() {
        for hits in map.0.values_mut() {
            hits.retain(|entity, _| {
                let keep = targets.panels.contains(*entity);
                if !keep {
                    // `update_interactions` clears interactions from `PreviousHoverMap`, which
                    // this prunes as well, so the app's own hover state is cleared here.
                    commands
                        .entity(*entity)
                        .try_insert(PickingInteraction::None);
                }
                keep
            });
        }
    }
    if let Some(state) = pointer_state.as_mut() {
        for ((_, button), primary) in state.pointer_buttons.iter_mut() {
            if *button != PointerButton::Primary {
                continue;
            }
            primary
                .pressing
                .retain(|entity, _| targets.panels.contains(*entity));
            primary
                .dragging
                .retain(|entity, _| targets.panels.contains(*entity));
            primary
                .dragging_over
                .retain(|entity, _| targets.panels.contains(*entity));
        }
    }
    if let Some(map) = previous_hover_map.as_mut() {
        for hits in map.0.values_mut() {
            hits.retain(|entity, _| targets.panels.contains(*entity));
        }
    }
    for hits in hits.values_mut() {
        hits.sort_by(|a, b| b.0.total_cmp(&a.0).then(a.1.total_cmp(&b.1)));
    }

    for (pointer, action, location) in inputs {
        let under_pointer = hits.get(&pointer).map_or(&[][..], Vec::as_slice);
        // The mesh fallback costs a ray cast, so only the press and release ask for it; a hover
        // hint only needs what the backends already reported.
        let pressed_entity = |meshes: &mut MeshFallback| {
            targets
                .topmost(under_pointer, &location)
                .unwrap_or_else(|| meshes.nearest(pointer).map_or(Pick::Nothing, Pick::Entity))
        };
        match action {
            PointerAction::Cancel => {
                presses.release(pointer);
            }
            PointerAction::Press(PointerButton::Primary) => {
                let pick = pressed_entity(&mut meshes);
                presses.press(pointer, Some(pick));
            }
            PointerAction::Release(PointerButton::Primary) => {
                let Some(pressed) = presses.release(pointer) else {
                    continue;
                };
                if pressed_entity(&mut meshes) != pressed {
                    continue;
                }
                match pressed {
                    Pick::Entity(entity) => {
                        selection.0 = Some(InspectableEntity::new(entity, InspectorSource::Local));
                        reveal.0 = Some(entity);
                        entities_page.show();
                    }
                    Pick::Nothing => {
                        selection.set_if_neq(InspectorSelection(None));
                        reveal.0 = None;
                    }
                    Pick::Blocked => {}
                }
            }
            _ => {}
        }
    }
    presses.hide_raw_input();
}

#[cfg(test)]
mod tests {
    use bevy_app::Update;
    use bevy_app::{App, PreUpdate, TaskPoolPlugin};
    use bevy_asset::{AssetApp, AssetPlugin, Assets};
    use bevy_camera::{
        primitives::Aabb,
        visibility::{InheritedVisibility, ViewVisibility, Visibility},
        NormalizedRenderTarget,
    };
    use bevy_ecs::{
        message::MessageWriter, name::Name, observer::On, resource::Resource,
        schedule::IntoScheduleConfigs, system::RunSystemOnce,
    };
    use bevy_input::{
        mouse::MouseButtonInput,
        touch::{TouchInput, TouchPhase},
        ButtonState, InputPlugin,
    };
    use bevy_math::{Dir3, Vec2, Vec3};
    use bevy_mesh::{Mesh, Mesh3d};
    use bevy_picking::{
        backend::{ray::RayId, HitData},
        events::PointerClick,
        InteractionPlugin, PickingPlugin, PickingSystems,
    };
    use bevy_scene::WorldSceneExt;
    use bevy_shape::{Cuboid, Ray3d};
    use bevy_transform::components::GlobalTransform;
    use bevy_ui::Expanded;
    use bevy_ui_widgets::SelectedTreeItem;

    use super::*;
    use crate::{
        entity_tree::{entity_tree_panel, InspectorTreeView, TreeRowIndex},
        InspectorPlugin,
    };

    fn test_app() -> App {
        let mut app = App::new();
        app.add_plugins((
            TaskPoolPlugin::default(),
            bevy_time::TimePlugin,
            AssetPlugin::default(),
            bevy_scene::ScenePlugin,
            InspectorPlugin,
        ));
        app.init_asset::<bevy_image::Image>();
        app.init_asset::<bevy_text::Font>();
        app
    }

    fn spawn_panel(app: &mut App) -> Entity {
        let panel = app
            .world_mut()
            .spawn_scene(entity_tree_panel())
            .unwrap()
            .id();
        app.update();
        panel
    }

    fn location() -> Location {
        Location {
            target: NormalizedRenderTarget::None {
                width: 1,
                height: 1,
            },
            position: Vec2::ZERO,
        }
    }

    fn write_hits(app: &mut App, hits: &[(Entity, f32)]) {
        write_pointer_hits(app, PointerId::Mouse, hits);
    }

    fn write_pointer_hits(app: &mut App, pointer: PointerId, hits: &[(Entity, f32)]) {
        let camera = app.world_mut().spawn_empty().id();
        let picks = hits
            .iter()
            .map(|(entity, depth)| (*entity, HitData::new(camera, *depth, None, None)))
            .collect();
        app.world_mut()
            .write_message(PointerHits::new(pointer, picks, 0.0));
    }

    fn send(app: &mut App, action: PointerAction) {
        send_pointer(app, PointerId::Mouse, action);
    }

    fn send_pointer(app: &mut App, pointer: PointerId, action: PointerAction) {
        app.world_mut()
            .write_message(PointerInput::new(pointer, location(), action));
        app.update();
    }

    fn drag(app: &mut App, press: &[(Entity, f32)], release: &[(Entity, f32)]) {
        write_hits(app, press);
        send(app, PointerAction::Press(PointerButton::Primary));
        write_hits(app, release);
        send(app, PointerAction::Release(PointerButton::Primary));
    }

    fn click(app: &mut App, hits: &[(Entity, f32)]) {
        drag(app, hits, hits);
    }

    fn selected(app: &App) -> Option<Entity> {
        app.world()
            .resource::<InspectorSelection>()
            .0
            .as_ref()
            .map(|selection| selection.entity)
    }

    fn row(app: &App, entity: Entity) -> Option<Entity> {
        app.world()
            .resource::<TreeRowIndex>()
            .row(entity, InspectorSource::Local)
    }

    fn tree(app: &mut App) -> Entity {
        let world = app.world_mut();
        world
            .query_filtered::<Entity, With<InspectorTreeView>>()
            .single(world)
            .unwrap()
    }

    fn set_mode(app: &mut App, mode: InspectorMode) {
        *app.world_mut().resource_mut::<InspectorMode>() = mode;
        app.update();
    }

    #[test]
    fn clicking_an_entity_selects_the_topmost_hit() {
        let mut app = test_app();
        spawn_panel(&mut app);
        let near = app.world_mut().spawn(Name::new("Near")).id();
        let far = app.world_mut().spawn(Name::new("Far")).id();
        click(&mut app, &[(far, 4.0), (near, 1.0)]);
        assert_eq!(selected(&app), Some(near));

        let tree = tree(&mut app);
        assert_eq!(
            app.world().get::<SelectedTreeItem>(tree).unwrap().0,
            row(&app, near)
        );
    }

    #[test]
    fn clicking_inspector_ui_keeps_the_selection() {
        let mut app = test_app();
        let panel = spawn_panel(&mut app);
        let subject = app.world_mut().spawn(Name::new("Subject")).id();
        click(&mut app, &[(subject, 1.0)]);
        assert_eq!(selected(&app), Some(subject));

        let other = app.world_mut().spawn(Name::new("Other")).id();
        let tree = tree(&mut app);
        click(&mut app, &[(tree, 0.0), (other, 2.0)]);
        assert_eq!(selected(&app), Some(subject));
        click(&mut app, &[(panel, 0.0)]);
        assert_eq!(selected(&app), Some(subject));
    }

    #[test]
    fn a_drag_between_entities_selects_nothing() {
        let mut app = test_app();
        spawn_panel(&mut app);
        let first = app.world_mut().spawn(Name::new("First")).id();
        let second = app.world_mut().spawn(Name::new("Second")).id();
        drag(&mut app, &[(first, 1.0)], &[(second, 1.0)]);
        assert_eq!(selected(&app), None);
        drag(&mut app, &[], &[(second, 1.0)]);
        assert_eq!(selected(&app), None);
    }

    #[test]
    fn a_click_on_empty_space_clears_the_selection() {
        let mut app = test_app();
        spawn_panel(&mut app);
        let subject = app.world_mut().spawn(Name::new("Subject")).id();
        click(&mut app, &[(subject, 1.0)]);
        assert_eq!(selected(&app), Some(subject));

        click(&mut app, &[]);
        assert_eq!(selected(&app), None);
        assert!(app.world().resource::<TreeReveal>().0.is_none());
        let tree = tree(&mut app);
        assert_eq!(app.world().get::<SelectedTreeItem>(tree).unwrap().0, None);

        click(&mut app, &[(subject, 1.0)]);
        let window = app.world_mut().spawn(Window::default()).id();
        click(&mut app, &[(window, 9.0)]);
        assert_eq!(selected(&app), None);
    }

    #[test]
    fn a_drag_between_an_entity_and_empty_space_keeps_the_selection() {
        let mut app = test_app();
        spawn_panel(&mut app);
        let subject = app.world_mut().spawn(Name::new("Subject")).id();
        let other = app.world_mut().spawn(Name::new("Other")).id();
        click(&mut app, &[(subject, 1.0)]);

        drag(&mut app, &[(other, 1.0)], &[]);
        assert_eq!(selected(&app), Some(subject));
        drag(&mut app, &[], &[(other, 1.0)]);
        assert_eq!(selected(&app), Some(subject));
    }

    #[test]
    fn a_click_on_empty_space_keeps_the_selection_in_preview() {
        let mut app = test_app();
        spawn_panel(&mut app);
        let subject = app.world_mut().spawn(Name::new("Subject")).id();
        click(&mut app, &[(subject, 1.0)]);
        set_mode(&mut app, InspectorMode::Preview);
        click(&mut app, &[]);
        assert_eq!(selected(&app), Some(subject));
    }

    #[test]
    fn clicking_a_deep_child_expands_its_ancestors_and_highlights_its_row() {
        let mut app = test_app();
        let root = app.world_mut().spawn(Name::new("Root")).id();
        let middle = app
            .world_mut()
            .spawn((Name::new("Middle"), ChildOf(root)))
            .id();
        let leaf = app
            .world_mut()
            .spawn((Name::new("Leaf"), ChildOf(middle)))
            .id();
        spawn_panel(&mut app);
        assert!(row(&app, root).is_some());
        assert!(row(&app, leaf).is_none());

        click(&mut app, &[(leaf, 1.0)]);
        assert_eq!(selected(&app), Some(leaf));
        let leaf_row = row(&app, leaf).unwrap();
        for ancestor in [root, middle] {
            let ancestor_row = row(&app, ancestor).unwrap();
            assert!(app.world().get::<Expanded>(ancestor_row).is_some());
        }
        assert!(app.world().get::<Expanded>(leaf_row).is_none());
        let tree = tree(&mut app);
        assert_eq!(
            app.world().get::<SelectedTreeItem>(tree).unwrap().0,
            Some(leaf_row)
        );
    }

    #[test]
    fn clicks_do_nothing_while_the_tree_is_hidden() {
        let mut app = test_app();
        let panel = spawn_panel(&mut app);
        app.world_mut().entity_mut(panel).insert(Visibility::Hidden);
        let subject = app.world_mut().spawn(Name::new("Subject")).id();
        click(&mut app, &[(subject, 1.0)]);
        assert_eq!(selected(&app), None);

        app.world_mut()
            .entity_mut(panel)
            .insert(Visibility::Inherited);
        app.update();
        assert_eq!(selected(&app), None);
    }

    #[test]
    fn clicks_do_nothing_without_a_tree() {
        let mut app = test_app();
        let subject = app.world_mut().spawn(Name::new("Subject")).id();
        click(&mut app, &[(subject, 1.0)]);
        assert_eq!(selected(&app), None);
        assert!(app.world().resource::<TreeReveal>().0.is_none());
    }

    #[test]
    fn preview_mode_picks_nothing() {
        let mut app = test_app();
        spawn_panel(&mut app);
        set_mode(&mut app, InspectorMode::Preview);
        let subject = app.world_mut().spawn(Name::new("Subject")).id();
        click(&mut app, &[(subject, 1.0)]);
        assert_eq!(selected(&app), None);
    }

    #[test]
    fn switching_modes_between_press_and_release_selects_nothing() {
        let mut app = test_app();
        spawn_panel(&mut app);
        let subject = app.world_mut().spawn(Name::new("Subject")).id();
        write_hits(&mut app, &[(subject, 1.0)]);
        send(&mut app, PointerAction::Press(PointerButton::Primary));
        set_mode(&mut app, InspectorMode::Preview);
        set_mode(&mut app, InspectorMode::Edit);
        write_hits(&mut app, &[(subject, 1.0)]);
        send(&mut app, PointerAction::Release(PointerButton::Primary));
        assert_eq!(selected(&app), None);
    }

    #[cfg(feature = "remote")]
    #[test]
    fn clicks_do_nothing_while_inspecting_a_remote_app() {
        let mut app = test_app();
        app.insert_resource(InspectorSources::remote_main("localhost", 1));
        spawn_panel(&mut app);
        let subject = app.world_mut().spawn(Name::new("Subject")).id();
        click(&mut app, &[(subject, 1.0)]);
        assert_eq!(selected(&app), None);

        let source = app
            .world()
            .resource::<InspectorSources>()
            .sources
            .iter()
            .next()
            .cloned()
            .unwrap();
        app.world_mut().resource_mut::<InspectorSelection>().0 =
            Some(InspectableEntity::new(subject, source));
        click(&mut app, &[]);
        assert_eq!(selected(&app), Some(subject));
    }

    fn ray_cast_app() -> App {
        let mut app = test_app();
        app.init_asset::<Mesh>().init_resource::<RayMap>();
        let camera = app.world_mut().spawn(Camera::default()).id();
        app.world_mut().resource_mut::<RayMap>().map.insert(
            RayId::new(camera, PointerId::Mouse),
            Ray3d::new(Vec3::ZERO, Dir3::NEG_Z),
        );
        app
    }

    fn spawn_mesh(app: &mut App, depth: f32) -> Entity {
        let mesh = app
            .world_mut()
            .resource_mut::<Assets<Mesh>>()
            .add(Mesh::from(Cuboid::new(1.0, 1.0, 1.0)));
        app.world_mut()
            .spawn((
                Mesh3d(mesh),
                GlobalTransform::from_translation(Vec3::new(0.0, 0.0, -depth)),
                Aabb::from_min_max(Vec3::splat(-0.5), Vec3::splat(0.5)),
                InheritedVisibility::VISIBLE,
                ViewVisibility::VISIBLE,
            ))
            .id()
    }

    fn aim(app: &mut App, direction: Dir3) {
        for ray in app.world_mut().resource_mut::<RayMap>().map.values_mut() {
            *ray = Ray3d::new(Vec3::ZERO, direction);
        }
    }

    #[test]
    fn a_click_with_no_backend_hit_selects_the_nearest_mesh() {
        let mut app = ray_cast_app();
        spawn_panel(&mut app);
        spawn_mesh(&mut app, 8.0);
        let near = spawn_mesh(&mut app, 4.0);
        click(&mut app, &[]);
        assert_eq!(selected(&app), Some(near));
    }

    #[test]
    fn a_click_that_only_hits_the_window_selects_the_mesh_behind() {
        let mut app = ray_cast_app();
        spawn_panel(&mut app);
        let mesh = spawn_mesh(&mut app, 4.0);
        let window = app.world_mut().spawn(Window::default()).id();
        click(&mut app, &[(window, 9.0)]);
        assert_eq!(selected(&app), Some(mesh));
    }

    #[test]
    fn the_mesh_fallback_skips_ignored_meshes() {
        let mut app = ray_cast_app();
        spawn_panel(&mut app);
        let ignored = spawn_mesh(&mut app, 4.0);
        app.world_mut().entity_mut(ignored).insert(Pickable::IGNORE);
        let behind = spawn_mesh(&mut app, 8.0);
        click(&mut app, &[]);
        assert_eq!(selected(&app), Some(behind));

        app.world_mut().entity_mut(behind).insert(Pickable::IGNORE);
        app.world_mut().resource_mut::<InspectorSelection>().0 = None;
        click(&mut app, &[]);
        assert_eq!(selected(&app), None);
    }

    #[test]
    fn a_click_that_misses_every_mesh_clears_the_selection() {
        let mut app = ray_cast_app();
        spawn_panel(&mut app);
        let mesh = spawn_mesh(&mut app, 4.0);
        click(&mut app, &[]);
        assert_eq!(selected(&app), Some(mesh));

        aim(&mut app, Dir3::Z);
        click(&mut app, &[]);
        assert_eq!(selected(&app), None);
    }

    #[test]
    fn no_mesh_fallback_when_the_app_has_mesh_picking() {
        let mut app = ray_cast_app();
        app.insert_resource(MeshBackendPresent(true));
        spawn_panel(&mut app);
        spawn_mesh(&mut app, 4.0);
        click(&mut app, &[]);
        assert_eq!(selected(&app), None);
    }

    #[test]
    fn see_through_inspector_ui_is_not_under_the_pointer() {
        use bevy_camera::CameraPlugin;
        use bevy_math::UVec2;
        use bevy_ui::{px, PositionType, UiPlugin, UiTargetCamera};

        let mut app = App::new();
        app.add_plugins((
            TaskPoolPlugin::default(),
            bevy_time::TimePlugin,
            AssetPlugin::default(),
            bevy_transform::TransformPlugin,
            CameraPlugin,
            bevy_text::TextPlugin,
            UiPlugin,
        ));
        app.init_asset::<bevy_image::Image>()
            .init_asset::<bevy_image::TextureAtlasLayout>()
            .init_asset::<Mesh>()
            .init_asset::<bevy_mesh::skinning::SkinnedMeshInverseBindposes>()
            .init_resource::<HoverMap>()
            .init_resource::<PointerState>()
            .init_resource::<bevy_input_focus::InputFocus>()
            .add_message::<PointerInput>()
            .add_message::<PointerHits>();
        let size = UVec2::new(400, 200);
        let camera = app
            .world_mut()
            .spawn((Camera::default(), RenderTarget::None { size }))
            .id();
        let side = || Node {
            width: px(100),
            ..Default::default()
        };
        let world = app.world_mut();
        let root = world
            .spawn((
                InspectorUi,
                Pickable::IGNORE,
                UiTargetCamera(camera),
                Node {
                    width: px(size.x),
                    height: px(size.y),
                    ..Default::default()
                },
            ))
            .id();
        let dock = world
            .spawn((
                Pickable::IGNORE,
                Node {
                    flex_grow: 1.0,
                    ..Default::default()
                },
                ChildOf(root),
            ))
            .id();
        world.spawn((side(), ChildOf(dock)));
        world.spawn((
            Node {
                width: px(4),
                ..Default::default()
            },
            ChildOf(dock),
        ));
        let center = world
            .spawn((
                Pickable::IGNORE,
                Node {
                    flex_grow: 1.0,
                    ..Default::default()
                },
                ChildOf(dock),
            ))
            .id();
        world.spawn((
            Pickable::IGNORE,
            Node {
                flex_grow: 1.0,
                ..Default::default()
            },
            ChildOf(center),
        ));
        world.spawn((side(), ChildOf(dock)));
        world.spawn((
            Node {
                position_type: PositionType::Absolute,
                left: px(150),
                top: px(10),
                width: px(40),
                height: px(20),
                ..Default::default()
            },
            ChildOf(root),
        ));
        app.update();

        let mut under = |x: f32, y: f32| {
            let location = Location {
                target: NormalizedRenderTarget::None {
                    width: size.x,
                    height: size.y,
                },
                position: Vec2::new(x, y),
            };
            app.world_mut()
                .run_system_once_with(
                    |In(location): In<Location>, panels: InspectorPanels| panels.under(&location),
                    location,
                )
                .unwrap()
        };
        assert!(!under(250.0, 150.0));
        assert!(under(50.0, 100.0));
        assert!(under(102.0, 100.0));
        assert!(under(350.0, 100.0));
        assert!(under(160.0, 15.0));
    }

    #[test]
    fn a_click_over_inspector_ui_does_not_ray_cast() {
        let mut app = ray_cast_app();
        spawn_panel(&mut app);
        spawn_mesh(&mut app, 4.0);
        let tree = tree(&mut app);
        click(&mut app, &[(tree, 0.0)]);
        assert_eq!(selected(&app), None);
    }

    #[test]
    fn a_drag_onto_or_off_a_mesh_selects_nothing() {
        let mut app = ray_cast_app();
        spawn_panel(&mut app);
        spawn_mesh(&mut app, 4.0);
        aim(&mut app, Dir3::Z);
        send(&mut app, PointerAction::Press(PointerButton::Primary));
        aim(&mut app, Dir3::NEG_Z);
        send(&mut app, PointerAction::Release(PointerButton::Primary));
        assert_eq!(selected(&app), None);

        send(&mut app, PointerAction::Press(PointerButton::Primary));
        aim(&mut app, Dir3::Z);
        send(&mut app, PointerAction::Release(PointerButton::Primary));
        assert_eq!(selected(&app), None);
    }

    #[derive(Resource)]
    struct FixedRay(RayId, Ray3d);

    #[derive(Resource, Default)]
    struct BackendHits(Vec<Entity>);

    #[derive(Resource, Default)]
    struct AppClicks(usize);

    fn keep_ray(ray: Res<FixedRay>, mut ray_map: ResMut<RayMap>) {
        ray_map.map.insert(ray.0, ray.1);
    }

    fn report_hits(
        ray: Res<FixedRay>,
        hits: Res<BackendHits>,
        mut writer: MessageWriter<PointerHits>,
    ) {
        let picks = hits
            .0
            .iter()
            .enumerate()
            .map(|(depth, entity)| {
                (
                    *entity,
                    HitData::new(ray.0.camera, depth as f32, None, None),
                )
            })
            .collect();
        writer.write(PointerHits::new(PointerId::Mouse, picks, 0.0));
    }

    fn picking_app() -> App {
        let mut app = ray_cast_app();
        let (id, ray) = app
            .world()
            .resource::<RayMap>()
            .iter()
            .map(|(id, ray)| (*id, *ray))
            .next()
            .unwrap();
        app.add_plugins((PickingPlugin, InteractionPlugin))
            .insert_resource(FixedRay(id, ray))
            .init_resource::<BackendHits>()
            .init_resource::<AppClicks>()
            .add_systems(
                PreUpdate,
                (keep_ray, report_hits)
                    .chain()
                    .ambiguous_with_all()
                    .in_set(PickingSystems::Backend),
            );
        app.world_mut().spawn(PointerId::Mouse);
        app
    }

    fn app_click(app: &mut App, hits: impl Fn(&mut App) -> Vec<Entity>) {
        let hits = hits(app);
        app.world_mut().resource_mut::<BackendHits>().0 = hits;
        send(app, PointerAction::Press(PointerButton::Primary));
        send(app, PointerAction::Release(PointerButton::Primary));
    }

    fn observed_mesh(app: &mut App) -> Entity {
        let mesh = spawn_mesh(app, 4.0);
        app.world_mut()
            .entity_mut(mesh)
            .observe(|_: On<PointerClick>, mut clicks: ResMut<AppClicks>| clicks.0 += 1);
        mesh
    }

    #[test]
    fn editing_selects_and_the_app_gets_no_click() {
        let mut app = picking_app();
        spawn_panel(&mut app);
        let mesh = observed_mesh(&mut app);
        app_click(&mut app, |_| alloc::vec![mesh]);
        assert_eq!(selected(&app), Some(mesh));
        assert_eq!(app.world().resource::<AppClicks>().0, 0);
    }

    fn interaction(app: &App, entity: Entity) -> PickingInteraction {
        app.world()
            .get::<PickingInteraction>(entity)
            .copied()
            .unwrap_or_default()
    }

    #[test]
    fn entering_edit_mode_clears_the_apps_hover_and_press_state() {
        let mut app = picking_app();
        spawn_panel(&mut app);
        set_mode(&mut app, InspectorMode::Preview);
        let mesh = observed_mesh(&mut app);
        app.world_mut().resource_mut::<BackendHits>().0 = alloc::vec![mesh];
        send(&mut app, PointerAction::Move { delta: Vec2::ONE });
        assert_eq!(interaction(&app, mesh), PickingInteraction::Hovered);

        send(&mut app, PointerAction::Press(PointerButton::Primary));
        let pressed = |app: &App| {
            app.world()
                .resource::<PointerState>()
                .get(PointerId::Mouse, PointerButton::Primary)
                .is_some_and(|state| state.pressing.contains_key(&mesh))
        };
        assert!(pressed(&app));

        set_mode(&mut app, InspectorMode::Edit);
        assert_eq!(interaction(&app, mesh), PickingInteraction::None);
        assert!(!pressed(&app));
        assert_eq!(app.world().resource::<AppClicks>().0, 0);
    }

    #[test]
    fn previewing_leaves_the_click_to_the_app() {
        let mut app = picking_app();
        spawn_panel(&mut app);
        set_mode(&mut app, InspectorMode::Preview);
        let mesh = observed_mesh(&mut app);
        app_click(&mut app, |_| alloc::vec![mesh]);
        assert_eq!(selected(&app), None);
        assert_eq!(app.world().resource::<AppClicks>().0, 1);
    }

    #[test]
    fn the_mesh_fallback_works_with_real_picking_while_editing() {
        let mut app = picking_app();
        spawn_panel(&mut app);
        let mesh = spawn_mesh(&mut app, 4.0);
        app_click(&mut app, |_| alloc::vec![]);
        assert_eq!(selected(&app), Some(mesh));
    }

    #[test]
    fn a_touch_on_an_entity_selects_it() {
        let mut app = test_app();
        spawn_panel(&mut app);
        let touched = app.world_mut().spawn(Name::new("Touched")).id();
        let under_mouse = app.world_mut().spawn(Name::new("Under mouse")).id();
        let touch = PointerId::Touch(0);
        for action in [
            PointerAction::Press(PointerButton::Primary),
            PointerAction::Release(PointerButton::Primary),
        ] {
            write_hits(&mut app, &[(under_mouse, 0.0)]);
            write_pointer_hits(&mut app, touch, &[(touched, 1.0)]);
            send_pointer(&mut app, touch, action);
        }
        assert_eq!(selected(&app), Some(touched));
    }

    #[test]
    fn the_mesh_fallback_casts_the_ray_of_the_pointer_that_clicked() {
        let mut app = ray_cast_app();
        spawn_panel(&mut app);
        let mesh = spawn_mesh(&mut app, 4.0);
        let camera = app.world_mut().spawn(Camera::default()).id();
        let touch = PointerId::Touch(0);
        app.world_mut()
            .resource_mut::<RayMap>()
            .map
            .insert(RayId::new(camera, touch), Ray3d::new(Vec3::ZERO, Dir3::Z));
        click(&mut app, &[]);
        assert_eq!(selected(&app), Some(mesh));

        send_pointer(
            &mut app,
            touch,
            PointerAction::Press(PointerButton::Primary),
        );
        send_pointer(
            &mut app,
            touch,
            PointerAction::Release(PointerButton::Primary),
        );
        assert_eq!(selected(&app), None);
    }

    #[derive(Resource, Default, Debug, PartialEq, Eq)]
    struct AppSawLeft {
        pressed: usize,
        just_pressed: usize,
        just_released: usize,
    }

    fn watch_left(mouse: Res<ButtonInput<MouseButton>>, mut saw: ResMut<AppSawLeft>) {
        saw.pressed += usize::from(mouse.pressed(MouseButton::Left));
        saw.just_pressed += usize::from(mouse.just_pressed(MouseButton::Left));
        saw.just_released += usize::from(mouse.just_released(MouseButton::Left));
    }

    #[derive(Resource, Default)]
    struct AppSawTouch(usize);

    fn watch_touches(touches: Res<Touches>, mut saw: ResMut<AppSawTouch>) {
        saw.0 += touches.iter().count()
            + touches.iter_just_pressed().count()
            + touches.iter_just_released().count()
            + touches.iter_just_canceled().count();
    }

    fn raw_input_app() -> App {
        let mut app = test_app();
        app.add_plugins(InputPlugin)
            .init_resource::<AppSawLeft>()
            .init_resource::<AppSawTouch>()
            .add_systems(Update, (watch_left, watch_touches));
        app
    }

    fn mouse_button(app: &mut App, state: ButtonState) {
        app.world_mut().write_message(MouseButtonInput {
            button: MouseButton::Left,
            state,
            window: Entity::PLACEHOLDER,
        });
    }

    fn raw_click(app: &mut App, hits: &[(Entity, f32)]) {
        write_hits(app, hits);
        mouse_button(app, ButtonState::Pressed);
        send(app, PointerAction::Press(PointerButton::Primary));
        app.update();
        write_hits(app, hits);
        mouse_button(app, ButtonState::Released);
        send(app, PointerAction::Release(PointerButton::Primary));
    }

    fn saw_left(app: &App) -> &AppSawLeft {
        app.world().resource::<AppSawLeft>()
    }

    #[test]
    fn a_click_on_the_app_while_editing_never_reaches_the_apps_mouse_buttons() {
        let mut app = raw_input_app();
        spawn_panel(&mut app);
        let subject = app.world_mut().spawn(Name::new("Subject")).id();
        raw_click(&mut app, &[(subject, 1.0)]);
        assert_eq!(selected(&app), Some(subject));
        raw_click(&mut app, &[]);
        assert_eq!(selected(&app), None);
        assert_eq!(*saw_left(&app), AppSawLeft::default());

        write_hits(&mut app, &[(subject, 1.0)]);
        mouse_button(&mut app, ButtonState::Pressed);
        mouse_button(&mut app, ButtonState::Released);
        app.world_mut().write_message(PointerInput::new(
            PointerId::Mouse,
            location(),
            PointerAction::Press(PointerButton::Primary),
        ));
        send(&mut app, PointerAction::Release(PointerButton::Primary));
        assert_eq!(selected(&app), Some(subject));
        assert_eq!(*saw_left(&app), AppSawLeft::default());
    }

    #[test]
    fn a_press_on_inspector_ui_in_the_frame_a_hidden_click_ends_reaches_the_app() {
        let mut app = raw_input_app();
        spawn_panel(&mut app);
        let subject = app.world_mut().spawn(Name::new("Subject")).id();
        write_hits(&mut app, &[(subject, 1.0)]);
        mouse_button(&mut app, ButtonState::Pressed);
        send(&mut app, PointerAction::Press(PointerButton::Primary));

        let tree = tree(&mut app);
        write_hits(&mut app, &[(tree, 0.0)]);
        mouse_button(&mut app, ButtonState::Released);
        mouse_button(&mut app, ButtonState::Pressed);
        app.world_mut().write_message(PointerInput::new(
            PointerId::Mouse,
            location(),
            PointerAction::Release(PointerButton::Primary),
        ));
        send(&mut app, PointerAction::Press(PointerButton::Primary));
        assert_eq!(
            *saw_left(&app),
            AppSawLeft {
                pressed: 1,
                just_pressed: 1,
                just_released: 0,
            }
        );

        write_hits(&mut app, &[(tree, 0.0)]);
        mouse_button(&mut app, ButtonState::Released);
        send(&mut app, PointerAction::Release(PointerButton::Primary));
        assert_eq!(saw_left(&app).just_released, 1);
    }

    #[test]
    fn a_click_on_inspector_ui_reaches_the_apps_mouse_buttons() {
        let mut app = raw_input_app();
        spawn_panel(&mut app);
        let tree = tree(&mut app);
        raw_click(&mut app, &[(tree, 0.0)]);
        let saw = saw_left(&app);
        assert_eq!(saw.just_pressed, 1);
        assert_eq!(saw.just_released, 1);
        assert_eq!(saw.pressed, 2);
    }

    #[test]
    fn previewing_leaves_the_apps_mouse_buttons_alone() {
        let mut app = raw_input_app();
        spawn_panel(&mut app);
        set_mode(&mut app, InspectorMode::Preview);
        let subject = app.world_mut().spawn(Name::new("Subject")).id();
        raw_click(&mut app, &[(subject, 1.0)]);
        assert_eq!(selected(&app), None);
        let saw = saw_left(&app);
        assert_eq!(saw.just_pressed, 1);
        assert_eq!(saw.just_released, 1);
        assert_eq!(saw.pressed, 2);
    }

    fn touch(app: &mut App, phase: TouchPhase, action: PointerAction, hits: &[(Entity, f32)]) {
        app.world_mut().write_message(TouchInput {
            phase,
            position: Vec2::ZERO,
            window: Entity::PLACEHOLDER,
            force: None,
            id: 0,
        });
        write_pointer_hits(app, PointerId::Touch(0), hits);
        send_pointer(app, PointerId::Touch(0), action);
    }

    fn raw_tap(app: &mut App, hits: &[(Entity, f32)]) {
        touch(
            app,
            TouchPhase::Started,
            PointerAction::Press(PointerButton::Primary),
            hits,
        );
        touch(
            app,
            TouchPhase::Moved,
            PointerAction::Move { delta: Vec2::ZERO },
            hits,
        );
        touch(
            app,
            TouchPhase::Ended,
            PointerAction::Release(PointerButton::Primary),
            hits,
        );
    }

    #[test]
    fn a_touch_on_the_app_while_editing_never_reaches_the_apps_touches() {
        let mut app = raw_input_app();
        spawn_panel(&mut app);
        let subject = app.world_mut().spawn(Name::new("Subject")).id();
        raw_tap(&mut app, &[(subject, 1.0)]);
        assert_eq!(selected(&app), Some(subject));
        assert_eq!(app.world().resource::<AppSawTouch>().0, 0);

        let tree = tree(&mut app);
        raw_tap(&mut app, &[(tree, 0.0)]);
        assert_eq!(selected(&app), Some(subject));
        assert!(app.world().resource::<AppSawTouch>().0 > 0);
    }

    fn saw_touch(app: &App) -> usize {
        app.world().resource::<AppSawTouch>().0
    }

    #[test]
    fn a_hidden_touch_stays_hidden_when_preview_starts_before_it_lifts() {
        let mut app = raw_input_app();
        spawn_panel(&mut app);
        let subject = app.world_mut().spawn(Name::new("Subject")).id();
        let hits = [(subject, 1.0)];
        touch(
            &mut app,
            TouchPhase::Started,
            PointerAction::Press(PointerButton::Primary),
            &hits,
        );
        set_mode(&mut app, InspectorMode::Preview);
        touch(
            &mut app,
            TouchPhase::Moved,
            PointerAction::Move { delta: Vec2::ZERO },
            &hits,
        );
        touch(
            &mut app,
            TouchPhase::Ended,
            PointerAction::Release(PointerButton::Primary),
            &hits,
        );
        assert_eq!(saw_touch(&app), 0);

        raw_tap(&mut app, &hits);
        assert!(saw_touch(&app) > 0);
    }

    #[test]
    fn a_canceled_touch_on_the_app_never_reaches_the_apps_touches() {
        let mut app = raw_input_app();
        spawn_panel(&mut app);
        let subject = app.world_mut().spawn(Name::new("Subject")).id();
        let hits = [(subject, 1.0)];
        touch(
            &mut app,
            TouchPhase::Started,
            PointerAction::Press(PointerButton::Primary),
            &hits,
        );
        touch(&mut app, TouchPhase::Canceled, PointerAction::Cancel, &hits);
        app.update();
        assert_eq!(selected(&app), None);
        assert_eq!(saw_touch(&app), 0);
    }
}
