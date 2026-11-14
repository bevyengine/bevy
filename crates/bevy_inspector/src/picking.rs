use bevy_camera::{
    visibility::{InheritedVisibility, RenderLayers},
    Camera, RenderTarget,
};
use bevy_ecs::{
    bundle::Bundle,
    change_detection::DetectChangesMut,
    component::Component,
    entity::Entity,
    hierarchy::{ChildOf, Children},
    message::MessageReader,
    query::With,
    system::{Commands, In, Local, Query, Res, ResMut, SystemParam},
};
use bevy_picking::{
    backend::{ray::RayMap, PointerHits},
    mesh_picking::{
        ray_cast::{MeshRayCast, MeshRayCastSettings},
        MeshPickingSettings,
    },
    pointer::{Location, PointerAction, PointerButton, PointerId, PointerInput},
    Pickable,
};
use bevy_ui::{
    percent, px, ComputedNode, ComputedUiTargetCamera, Display, GlobalZIndex, Node, PositionType,
    UiGlobalTransform,
};
use bevy_ui_widgets::ValueChange;
use bevy_window::{PrimaryWindow, Window};

use crate::{
    entity_tree::{InspectorUi, TreeReveal},
    world_tabs::{InspectorPage, InspectorPageBody, InspectorPageTab},
    InspectableEntity, InspectorMode, InspectorSelection, InspectorSource, InspectorSources,
};

#[derive(SystemParam)]
pub(crate) struct InspectorPanels<'w, 's> {
    inspector_ui: Query<
        'w,
        's,
        (Entity, Option<&'static Pickable>, Option<&'static Children>),
        With<InspectorUi>,
    >,
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
            .flat_map(|(root, pickable, children)| {
                let passes_through = pickable.is_some_and(|pickable| *pickable == Pickable::IGNORE);
                let panels = children
                    .map(|children| children.iter().copied().collect())
                    .unwrap_or_default();
                if passes_through {
                    panels
                } else {
                    alloc::vec![root]
                }
            })
            .any(|panel| {
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
            })
    }
}

#[derive(SystemParam)]
pub(crate) struct MeshFallback<'w, 's> {
    mesh_picking: Option<Res<'w, MeshPickingSettings>>,
    ray_map: Option<Res<'w, RayMap>>,
    cameras: Query<'w, 's, (&'static Camera, Option<&'static RenderLayers>)>,
    pickables: Query<'w, 's, &'static Pickable>,
    layers: Query<'w, 's, &'static RenderLayers>,
    ray_cast: Option<MeshRayCast<'w, 's>>,
}

impl MeshFallback<'_, '_> {
    fn nearest(&mut self) -> Option<Entity> {
        if self.mesh_picking.is_some() {
            return None;
        }
        let ray_map = self.ray_map.as_ref()?;
        let ray_cast = self.ray_cast.as_mut()?;
        let mut nearest: Option<(isize, f32, Entity)> = None;
        for (ray_id, ray) in ray_map.iter() {
            if ray_id.pointer != PointerId::Mouse {
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

#[derive(Component)]
pub(crate) struct InspectorPickBlocker;

pub(crate) fn pick_blocker(z: i32) -> impl Bundle {
    (
        InspectorPickBlocker,
        Node {
            position_type: PositionType::Absolute,
            left: px(0),
            top: px(0),
            width: percent(100),
            height: percent(100),
            ..Default::default()
        },
        GlobalZIndex(z),
        Pickable::default(),
    )
}

#[derive(SystemParam)]
pub(crate) struct EntitiesPage<'w, 's> {
    tabs: Query<'w, 's, (Entity, &'static InspectorPageTab, &'static ChildOf)>,
    bodies: Query<'w, 's, (&'static InspectorPageBody, &'static Node, &'static ChildOf)>,
    parents: Query<'w, 's, &'static ChildOf>,
    commands: Commands<'w, 's>,
}

impl EntitiesPage<'_, '_> {
    fn show(&mut self) {
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
    blockers: Query<'w, 's, (), With<InspectorPickBlocker>>,
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
            if self.windows.contains(*entity) || self.blockers.contains(*entity) {
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

pub(crate) fn select_clicked_entity(
    In(visible): In<bool>,
    mode: Res<InspectorMode>,
    sources: Res<InspectorSources>,
    mut pointer_hits: MessageReader<PointerHits>,
    mut inputs: MessageReader<PointerInput>,
    mut pressed: Local<Option<Pick>>,
    targets: PickTargets,
    mut meshes: MeshFallback,
    mut selection: ResMut<InspectorSelection>,
    mut reveal: ResMut<TreeReveal>,
    mut entities_page: EntitiesPage,
) {
    let mut hits: Vec<(f32, f32, Entity)> = pointer_hits
        .read()
        .filter(|hits| hits.pointer == PointerId::Mouse)
        .flat_map(|hits| {
            hits.picks
                .iter()
                .map(|(entity, hit)| (hits.order, hit.depth, *entity))
        })
        .collect();
    let clicks: Vec<(PointerAction, Location)> = inputs
        .read()
        .filter(|input| {
            input.pointer_id == PointerId::Mouse
                && matches!(
                    input.action,
                    PointerAction::Press(PointerButton::Primary)
                        | PointerAction::Release(PointerButton::Primary)
                )
        })
        .map(|input| (input.action, input.location.clone()))
        .collect();
    let active = visible
        && *mode == InspectorMode::Edit
        && sources.sources.contains(&InspectorSource::Local);
    if !active {
        *pressed = None;
        return;
    }
    if clicks.is_empty() {
        return;
    }
    hits.sort_by(|a, b| b.0.total_cmp(&a.0).then(a.1.total_cmp(&b.1)));

    let mut picked = None;
    let mut pick = |location: &Location| {
        *picked.get_or_insert_with(|| {
            targets
                .topmost(&hits, location)
                .unwrap_or_else(|| meshes.nearest().map_or(Pick::Nothing, Pick::Entity))
        })
    };
    for (action, location) in clicks {
        match action {
            PointerAction::Press(_) => *pressed = Some(pick(&location)),
            _ => {
                let Some(pressed) = pressed.take() else {
                    continue;
                };
                if pick(&location) != pressed {
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
        }
    }
}

#[cfg(test)]
mod tests {
    use bevy_app::{App, PreUpdate, TaskPoolPlugin};
    use bevy_asset::{AssetApp, AssetPlugin, Assets};
    use bevy_camera::{
        primitives::Aabb,
        visibility::{InheritedVisibility, ViewVisibility, Visibility},
        NormalizedRenderTarget,
    };
    use bevy_ecs::{
        message::MessageWriter, name::Name, observer::On, resource::Resource,
        schedule::IntoScheduleConfigs,
    };
    use bevy_math::{Dir3, Vec2, Vec3};
    use bevy_mesh::{Mesh, Mesh3d};
    use bevy_picking::{
        backend::{ray::RayId, HitData},
        events::PointerClick,
        mesh_picking::MeshPickingSettings,
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
        let camera = app.world_mut().spawn_empty().id();
        let picks = hits
            .iter()
            .map(|(entity, depth)| (*entity, HitData::new(camera, *depth, None, None)))
            .collect();
        app.world_mut()
            .write_message(PointerHits::new(PointerId::Mouse, picks, 0.0));
    }

    fn send(app: &mut App, action: PointerAction) {
        app.world_mut()
            .write_message(PointerInput::new(PointerId::Mouse, location(), action));
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

    fn blocker(app: &mut App) -> Entity {
        app.world_mut().spawn(pick_blocker(0)).id()
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
    fn a_click_on_the_blocker_picks_what_is_behind_it() {
        let mut app = test_app();
        spawn_panel(&mut app);
        let blocker = blocker(&mut app);
        let subject = app.world_mut().spawn(Name::new("Subject")).id();
        let window = app.world_mut().spawn(Window::default()).id();
        click(&mut app, &[(blocker, 0.0), (subject, 1.0), (window, 9.0)]);
        assert_eq!(selected(&app), Some(subject));
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
        let blocker = blocker(&mut app);
        click(&mut app, &[(blocker, 0.0), (window, 9.0)]);
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
    fn a_click_that_only_hits_the_blocker_or_window_selects_the_mesh_behind() {
        let mut app = ray_cast_app();
        spawn_panel(&mut app);
        let mesh = spawn_mesh(&mut app, 4.0);
        let blocker = blocker(&mut app);
        let window = app.world_mut().spawn(Window::default()).id();
        click(&mut app, &[(blocker, 0.0), (window, 9.0)]);
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
        app.init_resource::<MeshPickingSettings>();
        spawn_panel(&mut app);
        spawn_mesh(&mut app, 4.0);
        click(&mut app, &[]);
        assert_eq!(selected(&app), None);
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
                (keep_ray, report_hits).in_set(PickingSystems::Backend),
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
    fn editing_selects_through_the_blocker_and_the_app_gets_no_click() {
        let mut app = picking_app();
        spawn_panel(&mut app);
        let mesh = observed_mesh(&mut app);
        app_click(&mut app, |app| alloc::vec![blocker(app), mesh]);
        assert_eq!(selected(&app), Some(mesh));
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
        app_click(&mut app, |app| alloc::vec![blocker(app)]);
        assert_eq!(selected(&app), Some(mesh));
    }
}
