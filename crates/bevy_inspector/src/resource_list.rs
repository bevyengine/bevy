//! A panel listing the resources of the local world, and the systems that keep it in sync.

use alloc::{
    format,
    string::{String, ToString},
    vec::Vec,
};

use bevy_app::{Propagate, PropagateOver};
use bevy_ecs::error::ErrorReport;
use bevy_ecs::{
    change_detection::{DetectChanges, Ref},
    component::{Component, ComponentId, ComponentInfo},
    entity::Entity,
    event::EntityEvent,
    hierarchy::{ChildOf, Children},
    observer::On,
    query::With,
    reflect::{AppTypeRegistry, ReflectComponent, ReflectResource},
    resource::Resource,
    system::{Query, Res, ResMut},
    template::template,
    world::World,
};
use bevy_feathers::{
    constants::{fonts, size},
    containers::subpane_body,
    controls::{FeathersTextInput, FeathersTextInputContainer, FeathersTreeItem, FeathersTreeView},
    display::icon,
    theme::{SurfaceLevel, ThemeContext, ThemeTextColor},
    tokens,
};
use bevy_fuzzy::FuzzyPattern;
use bevy_log::warn;
use bevy_platform::collections::{HashMap, HashSet};
use bevy_reflect::{prelude::ReflectDefault, Reflect};
use bevy_scene::{bsn, bsn_list, on, Scene, WorldSceneExt};
use bevy_text::{
    EditableText, FontSize, FontSourceTemplate, FontWeight, LineBreak, Placeholder, TextEdit,
    TextEditChange, TextFont, TextLayout,
};
use bevy_ui::{px, widget::Text, AlignItems, Node, UiRect};
use bevy_ui_widgets::{tree_view_self_update, ValueChange};

use crate::{
    entity_tree::{
        row_label, row_label_entity, rows_container, write_label, InspectorRowLabel, FILTER_ICON,
    },
    InspectableEntity, InspectorSelection, InspectorSource, InspectorSources, PanelSync,
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

/// Marker for the text input holding the resource list filter query.
#[derive(Component, Debug, Default, Clone, Copy, Reflect)]
#[reflect(Component, Debug, Default, Clone)]
pub struct ResourceListFilterInput;

/// The query filtering the resource list.
///
/// While it is blank, the list shows every resource. Otherwise it shows the resources whose short
/// type name fuzzy matches the query (see [`FuzzyPattern`]), best match first, with the matched
/// characters highlighted.
#[derive(Resource, Debug, Default, Clone, PartialEq, Eq, Reflect)]
#[reflect(Resource, Debug, Default, Clone, PartialEq)]
pub struct ResourceListFilter(pub String);

/// The resources, row limit and filter query a resource list last displayed.
#[derive(Component, Default)]
struct ListedResources {
    resources: HashSet<Entity>,
    max_rows: usize,
    query: String,
}

/// Pacing of the resource list synchronization pass.
pub type ResourceListSync = PanelSync<InspectorResourceList>;

/// The filter box and the scrolling list of the local world's resources, one row per resource.
///
/// Resources whose type is registered with [`ReflectComponent`] come first. The others follow
/// with dimmed labels, since the details panel cannot show their fields.
pub fn resource_list_body() -> impl Scene {
    bsn! {
        @subpane_body()
        Node {
            min_height: px(0),
        }
        Children [
            @FeathersTextInputContainer
            Node {
                flex_grow: 0.0,
                flex_shrink: 0.0,
                padding: UiRect {
                    left: px(8.0),
                },
            }
            Propagate::<ThemeContext>(ThemeContext(SurfaceLevel::Base))
            Children [
                @icon(FILTER_ICON)
                --
                ResourceListFilterInput
                @FeathersTextInput
                TextFont {
                    font_size: size::SMALL_FONT,
                }
                template(|_| Ok(Placeholder::new("Filter...")))
                on(inspector_resource_filter_changed)
            ]
            --
            InspectorResourceList
            @FeathersTreeView
            Node {
                flex_grow: 1.0,
                min_height: px(0),
            }
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

/// Observer that copies the filter input's text into [`ResourceListFilter`].
pub fn inspector_resource_filter_changed(
    change: On<TextEditChange>,
    inputs: Query<&EditableText, With<ResourceListFilterInput>>,
    mut filter: ResMut<ResourceListFilter>,
) {
    let Ok(input) = inputs.get(change.event_target()) else {
        return;
    };
    let query = input.value();
    if filter.0 != query {
        filter.0 = query.to_string();
    }
}

/// Shows the [`ResourceListFilter`] query in the filter inputs when it is set by other means than
/// typing.
pub(crate) fn sync_resource_filter_input(
    filter: Res<ResourceListFilter>,
    mut inputs: Query<(Ref<ResourceListFilterInput>, &mut EditableText)>,
) {
    for (marker, mut input) in &mut inputs {
        if !(filter.is_changed() || marker.is_added()) || input.value() == filter.0 {
            continue;
        }
        if filter.0.is_empty() {
            input.clear();
        } else {
            input.queue_edit(TextEdit::SelectAll);
            input.queue_edit(TextEdit::Insert(filter.0.as_str().into()));
        }
    }
}

/// Adds, removes and relabels resource list rows so that they match the resources of the local
/// world and the [`ResourceListFilter`].
///
/// The list stays empty unless [`InspectorSources`] includes [`InspectorSource::Local`]. Rows are
/// only rebuilt when the set of resources, the row limit or the filter changes, and a filter
/// change forces a pass.
pub fn sync_resource_list(world: &mut World) {
    let due = ResourceListSync::tick(world);
    let Some((list, max_rows)) = world
        .query::<(Entity, &InspectorResourceList)>()
        .iter(world)
        .next()
        .map(|(list, settings)| (list, settings.max_rows))
    else {
        return;
    };
    let query = world
        .get_resource::<ResourceListFilter>()
        .map(|filter| filter.0.trim().to_string())
        .unwrap_or_default();
    let query_changed = world
        .get::<ListedResources>(list)
        .is_none_or(|listed| listed.query != query);
    if !due && !query_changed {
        return;
    }
    let rows = rows_container(world, list);

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
            && listed.query == query
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
    let ranked = FuzzyPattern::new(&query).rank(&entries, |(_, label, _)| label.as_str());
    let hidden = ranked.len().saturating_sub(max_rows);
    let entries: Vec<(bool, InspectorRowLabel, Entity)> = ranked
        .into_iter()
        .take(max_rows)
        .map(|ranked| {
            let (unreflected, text, entity) = &entries[ranked.index];
            let label = InspectorRowLabel {
                text: text.clone(),
                matched: ranked.matched.indices,
            };
            (*unreflected, label, *entity)
        })
        .collect();

    let shown: HashMap<Entity, usize> = entries
        .iter()
        .enumerate()
        .map(|(position, (.., entity))| (*entity, position))
        .collect();
    let existing: Vec<(Entity, Entity)> = world
        .get::<Children>(rows)
        .map(|children| {
            children
                .iter()
                .copied()
                .filter_map(|row| Some((row, world.get::<InspectorResourceRow>(row)?.inspected)))
                .collect()
        })
        .unwrap_or_default();

    for (row, inspected) in &existing {
        if !shown.contains_key(inspected)
            && let Ok(row) = world.get_entity_mut(*row)
        {
            row.despawn();
        }
    }

    let existing_rows: HashMap<Entity, Entity> = existing
        .iter()
        .map(|(row, inspected)| (*inspected, *row))
        .collect();
    for (unreflected, label, inspected) in entries {
        let row = match existing_rows.get(&inspected) {
            Some(row) => *row,
            None => match world.spawn_scene(resource_row(label.text.clone(), !unreflected)) {
                Ok(mut row) => {
                    row.insert((InspectorResourceRow { inspected }, ChildOf(rows)));
                    row.id()
                }
                Err(error) => {
                    warn!(
                        "failed to spawn an inspector resource row: {}",
                        ErrorReport::new(&error)
                    );
                    continue;
                }
            },
        };
        if let Some(label_entity) = row_label_entity(world, row)
            && world.get::<InspectorRowLabel>(label_entity) != Some(&label)
        {
            write_label(world, label_entity, label);
        }
    }

    sync_overflow_row(world, rows, hidden);

    let positions: HashMap<Entity, usize> = world
        .get::<Children>(rows)
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
    if let Some(mut children) = world.get_mut::<Children>(rows) {
        children.sort_by_key(|row| positions.get(row).copied().unwrap_or(usize::MAX));
    }

    world.entity_mut(list).insert(ListedResources {
        resources: resources.into_keys().collect(),
        max_rows,
        query,
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
            Err(error) => warn!(
                "failed to spawn an inspector resource row: {}",
                ErrorReport::new(&error)
            ),
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
    use bevy_feathers::theme::ThemeToken;
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

    #[derive(Resource, Reflect, Default)]
    #[reflect(Resource, Default)]
    struct AzetaX;

    #[derive(Resource, Reflect)]
    struct Unregistered;

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

    fn set_filter(app: &mut App, query: &str) {
        app.world_mut().resource_mut::<ResourceListFilter>().0 = query.into();
        app.update();
    }

    fn label_of(app: &App, list: Entity, inspected: Entity) -> InspectorRowLabel {
        let row = row_of(app, list, inspected).unwrap();
        let label = row_label_entity(app.world(), row).unwrap();
        app.world().get::<InspectorRowLabel>(label).unwrap().clone()
    }

    #[test]
    fn filter_shows_matching_resources_with_highlighted_labels() {
        let mut app = test_app();
        let list = spawn_list(&mut app);
        app.insert_resource(Alpha::default()).insert_resource(Zeta);
        app.update();
        sync(&mut app);
        let alpha = resource_entity::<Alpha>(&app);
        let zeta = resource_entity::<Zeta>(&app);

        set_filter(&mut app, "zeta");
        assert!(row_of(&app, list, alpha).is_none());
        assert_eq!(label_of(&app, list, zeta).matched, [0, 1, 2, 3]);

        set_filter(&mut app, " ");
        assert!(row_of(&app, list, alpha).is_some());
        assert!(label_of(&app, list, zeta).matched.is_empty());
    }

    #[test]
    fn filtered_rows_are_ranked_and_capped() {
        let mut app = test_app();
        app.register_type::<AzetaX>();
        let list = spawn_capped_list(&mut app, 1);
        app.insert_resource(AzetaX).insert_resource(Zeta);
        app.update();
        sync(&mut app);

        set_filter(&mut app, "zeta");
        let shown: Vec<Entity> = rows(&app, list).into_iter().map(|(_, e)| e).collect();
        assert_eq!(shown, [resource_entity::<Zeta>(&app)]);
        assert_eq!(overflow_text(&app, list), Some("1 more".into()));
    }

    #[test]
    fn a_selected_resource_is_highlighted_again_when_it_matches() {
        let mut app = test_app();
        let list = spawn_list(&mut app);
        app.world_mut()
            .entity_mut(list)
            .insert(bevy_ui_widgets::SelectedTreeItem::default());
        app.insert_resource(Alpha::default()).insert_resource(Zeta);
        app.update();
        sync(&mut app);
        let alpha = resource_entity::<Alpha>(&app);
        let selected = |app: &App| {
            app.world()
                .get::<bevy_ui_widgets::SelectedTreeItem>(list)
                .unwrap()
                .0
        };

        app.world_mut().resource_mut::<InspectorSelection>().0 =
            Some(InspectableEntity::new(alpha, InspectorSource::Local));
        app.update();
        assert_eq!(selected(&app), row_of(&app, list, alpha));

        set_filter(&mut app, "zeta");
        assert!(row_of(&app, list, alpha).is_none());
        set_filter(&mut app, "");
        let row = row_of(&app, list, alpha);
        assert!(row.is_some());
        assert_eq!(selected(&app), row);
        assert_eq!(
            app.world().resource::<InspectorSelection>().0,
            Some(InspectableEntity::new(alpha, InspectorSource::Local))
        );
    }

    #[test]
    fn highlighted_labels_of_dimmed_rows_stay_dimmed() {
        let mut app = test_app();
        let list = spawn_list(&mut app);
        app.register_type::<Unregistered>();
        app.insert_resource(Unregistered);
        app.update();
        set_filter(&mut app, "unreg");

        let row = rows(&app, list)[0].0;
        let label = row_label_entity(app.world(), row).unwrap();
        let colors: Vec<ThemeToken> = app
            .world()
            .get::<Children>(label)
            .unwrap()
            .iter()
            .filter_map(|span| app.world().get::<ThemeTextColor>(*span))
            .map(|color| color.0.clone())
            .collect();
        assert!(colors.contains(&tokens::TEXT_ACCENT));
        assert!(colors.contains(&tokens::LISTROW_TEXT_DISABLED));
    }

    #[test]
    fn filter_input_and_resource_stay_in_step() {
        let mut app = test_app();
        app.add_plugins(bevy_text::TextPlugin);
        let input = app
            .world_mut()
            .spawn((ResourceListFilterInput, EditableText::default()))
            .observe(inspector_resource_filter_changed)
            .id();
        app.update();

        set_filter(&mut app, "time");
        assert_eq!(
            app.world().get::<EditableText>(input).unwrap().value(),
            "time"
        );

        app.world_mut()
            .get_mut::<EditableText>(input)
            .unwrap()
            .queue_edit(TextEdit::Insert("r".into()));
        app.update();
        assert_eq!(app.world().resource::<ResourceListFilter>().0, "timer");
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
