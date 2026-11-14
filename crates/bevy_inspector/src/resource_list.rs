//! A panel listing the resources of the local world, and the systems that keep it in sync.

use alloc::{format, string::String, vec::Vec};

use bevy_app::PropagateOver;
use bevy_ecs::{
    component::{Component, ComponentId, ComponentInfo},
    entity::Entity,
    hierarchy::{ChildOf, Children},
    observer::On,
    query::With,
    reflect::{AppTypeRegistry, ReflectComponent},
    system::{Query, ResMut},
    world::World,
};
use bevy_feathers::{
    constants::{fonts, size},
    containers::subpane_body,
    controls::{FeathersTreeItem, FeathersTreeView},
    theme::ThemeTextColor,
    tokens,
};
use bevy_log::warn;
use bevy_platform::collections::{HashMap, HashSet};
use bevy_reflect::{prelude::ReflectDefault, Reflect};
use bevy_scene::{bsn, bsn_list, on, Scene, WorldSceneExt};
use bevy_text::{FontSize, FontSourceTemplate, FontWeight, LineBreak, TextFont, TextLayout};
use bevy_ui::{px, widget::Text, AlignItems, Node, Overflow, OverflowAxis, UiRect};
use bevy_ui_widgets::{tree_view_self_update, ValueChange};

use crate::{
    entity_tree::row_label, InspectableEntity, InspectorSelection, InspectorSource,
    InspectorSources, PanelSync,
};

/// The default number of rows a resource list shows before summarizing the rest in one row.
pub const MAX_RESOURCE_ROWS: usize = 200;

/// The tree view that holds the resource list rows.
#[derive(Component, Debug, Clone, Copy, Reflect)]
#[reflect(Component, Debug, Default, Clone)]
pub struct InspectorResourceList {
    /// The number of resource rows shown. Further resources are counted in a final row.
    pub max_rows: usize,
}

impl Default for InspectorResourceList {
    fn default() -> Self {
        Self {
            max_rows: MAX_RESOURCE_ROWS,
        }
    }
}

/// A row of the resource list, and the resource entity it displays.
#[derive(Component, Debug, Clone, Reflect)]
#[reflect(Component, Debug, Clone)]
pub struct InspectorResourceRow {
    /// The entity holding the resource in the local world.
    pub inspected: Entity,
}

/// The final row of a resource list, counting the resources beyond its row limit.
#[derive(Component, Debug, Default, Clone, Copy, Reflect)]
#[reflect(Component, Debug, Default, Clone)]
pub struct InspectorResourceOverflow;

/// The resources and row limit a resource list last displayed.
#[derive(Component, Default)]
struct ListedResources {
    resources: HashSet<Entity>,
    max_rows: usize,
}

/// Pacing of the resource list synchronization pass.
pub type ResourceListSync = PanelSync<InspectorResourceList>;

/// The scrolling list of the local world's resources, one row per resource.
///
/// Resources whose type is registered with [`ReflectComponent`] come first. The others follow
/// with dimmed labels, since the details panel cannot show their fields.
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
/// The list stays empty unless [`InspectorSources`] includes [`InspectorSource::Local`]. Rows are
/// only rebuilt when the set of resources or the row limit changes.
pub fn sync_resource_list(world: &mut World) {
    if !ResourceListSync::tick(world) {
        return;
    }

    let Some((list, max_rows)) = world
        .query::<(Entity, &InspectorResourceList)>()
        .iter(world)
        .next()
        .map(|(list, settings)| (list, settings.max_rows))
    else {
        return;
    };

    let local = world
        .resource::<InspectorSources>()
        .sources
        .contains(&InspectorSource::Local);
    let resources: HashMap<Entity, ComponentId> = if local {
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

    if world.get::<ListedResources>(list).is_some_and(|listed| {
        listed.max_rows == max_rows
            && listed.resources.len() == resources.len()
            && resources
                .keys()
                .all(|entity| listed.resources.contains(entity))
    }) {
        return;
    }

    let mut entries: Vec<(bool, String, Entity)> = {
        let registry = world.get_resource::<AppTypeRegistry>().map(|r| r.read());
        resources
            .iter()
            .map(|(entity, id)| {
                let reflected = world
                    .components()
                    .get_info(*id)
                    .and_then(ComponentInfo::type_id)
                    .and_then(|type_id| registry.as_ref()?.get(type_id))
                    .is_some_and(|registration| registration.data::<ReflectComponent>().is_some());
                (!reflected, crate::component_short_name(world, *id), *entity)
            })
            .collect()
    };
    entries.sort();
    let hidden = entries.len().saturating_sub(max_rows);
    entries.truncate(max_rows);

    let shown: HashMap<Entity, usize> = entries
        .iter()
        .enumerate()
        .map(|(position, (.., entity))| (*entity, position))
        .collect();
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
    let existing_inspected: HashSet<Entity> =
        existing.iter().map(|(_, inspected)| *inspected).collect();

    for (row, inspected) in &existing {
        if !shown.contains_key(inspected)
            && let Ok(row) = world.get_entity_mut(*row)
        {
            row.despawn();
        }
    }

    for (unreflected, label, inspected) in entries {
        if existing_inspected.contains(&inspected) {
            continue;
        }
        match world.spawn_scene(resource_row(label, !unreflected)) {
            Ok(mut row) => {
                row.insert((InspectorResourceRow { inspected }, ChildOf(list)));
            }
            Err(error) => warn!("failed to spawn an inspector resource row: {error}"),
        }
    }

    sync_overflow_row(world, list, hidden);

    let positions: HashMap<Entity, usize> = world
        .get::<Children>(list)
        .map(|children| {
            children
                .iter()
                .copied()
                .filter_map(|row| {
                    let inspected = world.get::<InspectorResourceRow>(row)?.inspected;
                    Some((row, *shown.get(&inspected)?))
                })
                .collect()
        })
        .unwrap_or_default();
    if let Some(mut children) = world.get_mut::<Children>(list) {
        children.sort_by_key(|row| positions.get(row).copied().unwrap_or(usize::MAX));
    }

    world.entity_mut(list).insert(ListedResources {
        resources: resources.into_keys().collect(),
        max_rows,
    });
}

/// Spawns, updates or despawns the row counting the `hidden` resources beyond the row limit.
fn sync_overflow_row(world: &mut World, list: Entity, hidden: usize) {
    let existing = world.get::<Children>(list).and_then(|children| {
        children
            .iter()
            .copied()
            .find(|child| world.get::<InspectorResourceOverflow>(*child).is_some())
    });
    let text = format!("{hidden} more");
    match (existing, hidden) {
        (Some(row), 0) => {
            if let Ok(row) = world.get_entity_mut(row) {
                row.despawn();
            }
        }
        (Some(row), _) => {
            if let Some(mut label) = world.get_mut::<Text>(row)
                && label.0 != text
            {
                label.0 = text;
            }
        }
        (None, 0) => {}
        (None, _) => match world.spawn_scene(overflow_row(text)) {
            Ok(mut row) => {
                row.insert(ChildOf(list));
            }
            Err(error) => warn!("failed to spawn an inspector resource row: {error}"),
        },
    }
}

fn resource_row(label: String, reflected: bool) -> impl Scene {
    let color = if reflected {
        tokens::LISTROW_TEXT
    } else {
        tokens::LISTROW_TEXT_DISABLED
    };
    bsn! {
        @FeathersTreeItem {
            @label: bsn_list! {
                @row_label(label)
                ThemeTextColor({color.clone()})
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

fn overflow_row(text: String) -> impl Scene {
    bsn! {
        InspectorResourceOverflow
        Text(text)
        TextFont {
            font: FontSourceTemplate::Families(fonts::SANS),
            font_size: FontSize::Px(14.0),
            weight: FontWeight::NORMAL,
        }
        PropagateOver<TextFont>
        ThemeTextColor(tokens::LISTROW_TEXT_DISABLED)
        Node {
            min_height: size::ROW_HEIGHT,
            align_items: AlignItems::Center,
            padding: UiRect::left(px(24)),
        }
        TextLayout {
            linebreak: LineBreak::NoWrap,
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
    use bevy_ecs::{
        reflect::ReflectResource,
        resource::{IsResource, Resource},
    };
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

    #[derive(Resource)]
    struct AaaUnreflected;

    fn spawn_list(app: &mut App) -> Entity {
        spawn_capped_list(app, MAX_RESOURCE_ROWS)
    }

    fn spawn_capped_list(app: &mut App, max_rows: usize) -> Entity {
        let panel = app.world_mut().spawn(InspectorUi).id();
        app.world_mut()
            .spawn((InspectorResourceList { max_rows }, ChildOf(panel)))
            .observe(inspector_resource_selected)
            .id()
    }

    fn overflow_text(app: &App, list: Entity) -> Option<String> {
        let children = app.world().get::<Children>(list)?;
        let last = *children.last()?;
        app.world().get::<InspectorResourceOverflow>(last)?;
        Some(app.world().get::<Text>(last)?.0.clone())
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
    fn resources_without_reflection_sort_after_reflected_ones() {
        let mut app = test_app();
        let list = spawn_list(&mut app);
        app.insert_resource(AaaUnreflected).insert_resource(Zeta);
        app.update();
        sync(&mut app);

        let shown: Vec<Entity> = rows(&app, list).into_iter().map(|(_, e)| e).collect();
        let zeta_at = shown
            .iter()
            .position(|e| *e == resource_entity::<Zeta>(&app))
            .unwrap();
        let unreflected_at = shown
            .iter()
            .position(|e| *e == resource_entity::<AaaUnreflected>(&app))
            .unwrap();
        assert!(zeta_at < unreflected_at);
    }

    #[test]
    fn rows_beyond_the_limit_are_counted_in_a_final_row() {
        let mut app = test_app();
        let list = spawn_capped_list(&mut app, 3);
        app.update();
        sync(&mut app);

        assert_eq!(rows(&app, list).len(), 3);
        let hidden: usize = overflow_text(&app, list)
            .unwrap()
            .strip_suffix(" more")
            .unwrap()
            .parse()
            .unwrap();
        assert!(hidden > 0);

        app.insert_resource(Zeta);
        sync(&mut app);
        assert_eq!(rows(&app, list).len(), 3);
        assert_eq!(
            overflow_text(&app, list),
            Some(alloc::format!("{} more", hidden + 1))
        );

        app.world_mut()
            .get_mut::<InspectorResourceList>(list)
            .unwrap()
            .max_rows = usize::MAX;
        sync(&mut app);
        assert_eq!(overflow_text(&app, list), None);
        assert_eq!(rows(&app, list).len(), hidden + 4);
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
