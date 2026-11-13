//! A panel listing the resources of the local world, and the systems that keep it in sync.

use alloc::{string::String, vec::Vec};

use bevy_ecs::{
    component::{Component, ComponentId},
    entity::Entity,
    hierarchy::{ChildOf, Children},
    observer::On,
    query::With,
    reflect::ReflectComponent,
    resource::Resource,
    system::{Query, ResMut},
    world::World,
};
use bevy_feathers::{
    containers::subpane_body,
    controls::{FeathersTreeItem, FeathersTreeView},
};
use bevy_log::warn;
use bevy_platform::collections::{HashMap, HashSet};
use bevy_reflect::{prelude::ReflectDefault, Reflect};
use bevy_scene::{bsn, bsn_list, on, Scene, WorldSceneExt};
use bevy_text::{LineBreak, TextLayout};
use bevy_time::{Time, Timer, TimerMode};
use bevy_ui::{px, Node, Overflow, OverflowAxis};
use bevy_ui_widgets::{tree_view_self_update, ValueChange};

use crate::{
    entity_tree::row_label, InspectableEntity, InspectorSelection, InspectorSource,
    InspectorSources,
};

/// Marker for the tree view that holds the resource list rows.
#[derive(Component, Debug, Default, Clone, Copy, Reflect)]
#[reflect(Component, Debug, Default, Clone)]
pub struct InspectorResourceList;

/// A row of the resource list, and the resource entity it displays.
#[derive(Component, Debug, Clone, Reflect)]
#[reflect(Component, Debug, Clone)]
pub struct InspectorResourceRow {
    /// The entity holding the resource in the local world.
    pub inspected: Entity,
}

/// Pacing of the resource list synchronization pass.
#[derive(Resource, Debug)]
pub struct ResourceListSync {
    /// Time between synchronization passes.
    pub timer: Timer,
}

impl Default for ResourceListSync {
    fn default() -> Self {
        let mut timer = Timer::from_seconds(0.25, TimerMode::Repeating);
        timer.set_elapsed(timer.duration());
        Self { timer }
    }
}

impl ResourceListSync {
    /// Forces a synchronization pass on the next tick.
    pub fn set_dirty(&mut self) {
        self.timer.almost_finish();
    }
}

/// The scrolling list of the local world's resources, one row per resource.
pub fn resource_list_body() -> impl Scene {
    bsn! {
        @subpane_body()
        Node {
            overflow: Overflow {
                x: OverflowAxis::Clip,
                y: OverflowAxis::Scroll,
            },
        }
        Children [
            InspectorResourceList
            @FeathersTreeView
            on(tree_view_self_update)
            on(inspector_resource_selected)
        ]
    }
}

/// Observer that records the resource entity of the selected row in [`InspectorSelection`].
pub fn inspector_resource_selected(
    change: On<ValueChange<Option<Entity>>>,
    lists: Query<(), With<InspectorResourceList>>,
    rows: Query<&InspectorResourceRow>,
    mut selection: ResMut<InspectorSelection>,
) {
    if !lists.contains(change.source) {
        return;
    }

    selection.0 = change
        .value
        .and_then(|row| rows.get(row).ok())
        .map(|row| InspectableEntity::new(row.inspected, InspectorSource::Local));
}

/// Adds and removes resource list rows so that they match the resources of the local world.
///
/// The list stays empty unless [`InspectorSources`] includes [`InspectorSource::Local`].
pub fn sync_resource_list(world: &mut World) {
    let delta = world
        .get_resource::<Time>()
        .map(Time::delta)
        .unwrap_or_default();

    if !world
        .resource_mut::<ResourceListSync>()
        .timer
        .tick(delta)
        .just_finished()
    {
        return;
    }

    let Some(list) = world
        .query_filtered::<Entity, With<InspectorResourceList>>()
        .iter(world)
        .next()
    else {
        return;
    };

    let local = world
        .resource::<InspectorSources>()
        .sources
        .contains(&InspectorSource::Local);
    let expected: HashMap<Entity, ComponentId> = if local {
        world
            .resource_entities()
            .iter()
            .filter(|(id, entity)| {
                world
                    .get_entity(*entity)
                    .is_ok_and(|entity| entity.contains_id(*id))
            })
            .map(|(id, entity)| (entity, id))
            .collect()
    } else {
        HashMap::default()
    };

    let existing: Vec<(Entity, Entity)> = world
        .get::<Children>(list)
        .map(|children| {
            children
                .iter()
                .copied()
                .filter_map(|row| Some((row, world.get::<InspectorResourceRow>(row)?.inspected)))
                .collect()
        })
        .unwrap_or_default();
    let shown: HashSet<Entity> = existing.iter().map(|(_, inspected)| *inspected).collect();

    let stale: Vec<Entity> = existing
        .iter()
        .filter(|(_, inspected)| !expected.contains_key(inspected))
        .map(|(row, _)| *row)
        .collect();
    let missing: Vec<Entity> = expected
        .keys()
        .copied()
        .filter(|inspected| !shown.contains(inspected))
        .collect();
    if stale.is_empty() && missing.is_empty() {
        return;
    }

    for row in stale {
        if let Ok(row) = world.get_entity_mut(row) {
            row.despawn();
        }
    }

    let labels: HashMap<Entity, String> = expected
        .iter()
        .map(|(entity, id)| (*entity, crate::component_short_name(world, *id)))
        .collect();

    for inspected in missing {
        match world.spawn_scene(resource_row(labels[&inspected].clone())) {
            Ok(mut row) => {
                row.insert((InspectorResourceRow { inspected }, ChildOf(list)));
            }
            Err(error) => warn!("failed to spawn an inspector resource row: {error}"),
        }
    }

    let keys: HashMap<Entity, (&str, Entity)> = world
        .get::<Children>(list)
        .map(|children| {
            children
                .iter()
                .copied()
                .filter_map(|row| {
                    let inspected = world.get::<InspectorResourceRow>(row)?.inspected;
                    Some((row, (labels.get(&inspected)?.as_str(), inspected)))
                })
                .collect()
        })
        .unwrap_or_default();
    if let Some(mut children) = world.get_mut::<Children>(list) {
        children.sort_by_key(|row| keys.get(row).copied());
    }
}

fn resource_row(label: String) -> impl Scene {
    bsn! {
        @FeathersTreeItem {
            @label: bsn_list! {
                @row_label(label)
                Node {
                    min_width: px(0),
                    flex_basis: px(0),
                    flex_grow: 1.0,
                }
                TextLayout {
                    linebreak: LineBreak::NoWrap,
                }
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        details_panel::{DetailsIndex, InspectorDetailsBody, InspectorField},
        entity_tree::InspectorUi,
        InspectorPlugin,
    };
    use bevy_app::{App, TaskPoolPlugin};
    use bevy_asset::{AssetApp, AssetPlugin};
    use bevy_ecs::{reflect::ReflectResource, resource::IsResource};
    use bevy_reflect::TypePath;

    #[derive(Resource, Reflect, Default)]
    #[reflect(Resource, Default)]
    struct Alpha {
        value: f32,
    }

    #[derive(Resource, Reflect, Default)]
    #[reflect(Resource, Default)]
    struct Zeta;

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
        app.register_type::<Alpha>().register_type::<Zeta>();
        app
    }

    fn spawn_list(app: &mut App) -> Entity {
        let panel = app.world_mut().spawn(InspectorUi).id();
        app.world_mut()
            .spawn((InspectorResourceList, ChildOf(panel)))
            .observe(inspector_resource_selected)
            .id()
    }

    fn sync(app: &mut App) {
        app.world_mut()
            .resource_mut::<ResourceListSync>()
            .set_dirty();
        app.update();
    }

    fn resource_entity<R: Resource>(app: &App) -> Entity {
        let id = app.world().component_id::<R>().unwrap();
        app.world().resource_entities().get(id).unwrap()
    }

    fn rows(app: &App, list: Entity) -> Vec<(Entity, Entity)> {
        app.world()
            .get::<Children>(list)
            .map(|children| {
                children
                    .iter()
                    .copied()
                    .filter_map(|row| {
                        Some((row, app.world().get::<InspectorResourceRow>(row)?.inspected))
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    fn row_of(app: &App, list: Entity, inspected: Entity) -> Option<Entity> {
        rows(app, list)
            .into_iter()
            .find_map(|(row, shown)| (shown == inspected).then_some(row))
    }

    #[test]
    fn rows_follow_inserted_and_removed_resources_in_name_order() {
        let mut app = test_app();
        let list = spawn_list(&mut app);
        app.insert_resource(Zeta).insert_resource(Alpha::default());
        app.update();
        sync(&mut app);

        let alpha = resource_entity::<Alpha>(&app);
        let zeta = resource_entity::<Zeta>(&app);
        let shown: Vec<Entity> = rows(&app, list).into_iter().map(|(_, e)| e).collect();
        let alpha_at = shown.iter().position(|e| *e == alpha).unwrap();
        let zeta_at = shown.iter().position(|e| *e == zeta).unwrap();
        assert!(alpha_at < zeta_at);

        let before = rows(&app, list);
        sync(&mut app);
        assert_eq!(rows(&app, list), before);

        app.world_mut().remove_resource::<Zeta>();
        sync(&mut app);
        assert!(row_of(&app, list, zeta).is_none());
        assert!(row_of(&app, list, alpha).is_some());
    }

    #[test]
    fn clicking_a_row_selects_the_resource_entity() {
        let mut app = test_app();
        let list = spawn_list(&mut app);
        app.insert_resource(Alpha::default());
        app.update();
        sync(&mut app);

        let alpha = resource_entity::<Alpha>(&app);
        let row = row_of(&app, list, alpha).unwrap();
        app.world_mut().trigger(ValueChange {
            source: list,
            value: Some(row),
            is_final: true,
        });
        app.world_mut().flush();
        assert_eq!(
            app.world().resource::<InspectorSelection>().0,
            Some(InspectableEntity::new(alpha, InspectorSource::Local))
        );
    }

    #[test]
    fn editing_a_selected_resource_in_the_details_panel_changes_it() {
        let mut app = test_app();
        app.add_plugins(bevy_text::TextPlugin);
        let mut units = bevy_feathers::controls::UnitsRegistry::default();
        units.insert(&bevy_feathers::controls::Dimensionless);
        app.insert_resource(units);
        app.insert_resource(bevy_feathers::theme::UiTheme(
            bevy_feathers::dark_theme::create_dark_theme(),
        ));
        app.init_resource::<bevy_input_focus::InputFocus>();
        app.world_mut().spawn((InspectorUi, InspectorDetailsBody));
        let list = spawn_list(&mut app);
        app.insert_resource(Alpha { value: 1.0 });
        app.update();
        sync(&mut app);

        let alpha = resource_entity::<Alpha>(&app);
        let row = row_of(&app, list, alpha).unwrap();
        app.world_mut().trigger(ValueChange {
            source: list,
            value: Some(row),
            is_final: true,
        });
        app.update();

        let index = app.world().resource::<DetailsIndex>();
        assert!(index.group(Alpha::type_path()).is_some());
        assert!(index.group(IsResource::type_path()).is_none());
        let input = app
            .world_mut()
            .query::<(Entity, &InspectorField)>()
            .iter(app.world())
            .find(|(_, field)| field.path == "value")
            .map(|(entity, _)| entity)
            .unwrap();
        app.world_mut().trigger(ValueChange {
            source: input,
            value: 2.5_f32,
            is_final: true,
        });
        app.update();
        assert_eq!(app.world().resource::<Alpha>().value, 2.5);
    }

    #[cfg(feature = "remote")]
    #[test]
    fn shows_no_rows_without_the_local_source() {
        let mut app = test_app();
        app.insert_resource(InspectorSources::remote_main("localhost", 1));
        let list = spawn_list(&mut app);
        app.insert_resource(Alpha::default());
        app.update();
        sync(&mut app);
        assert!(rows(&app, list).is_empty());
    }
}
