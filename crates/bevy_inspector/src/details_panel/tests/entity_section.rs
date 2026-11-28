use super::*;
use crate::details_panel::entity_section::SECTION_KEY;
use bevy_ecs::{change_detection::MaybeLocation, entity_disabling::Disabled};

fn expand_section(app: &mut App) {
    app.world_mut()
        .resource_mut::<DetailsCollapsed>()
        .toggled
        .insert(SECTION_KEY.into());
}

fn section_value(app: &App, path: &str) -> String {
    let index = app.world().resource::<DetailsIndex>();
    let widget = &index.fields[&(SECTION_KEY.into(), path.to_string())];
    let shown = displayed(&widget.value);
    assert!(
        has_text(app.world(), widget.entity, &shown),
        "the `{path}` row does not show {shown:?}"
    );
    shown
}

fn section_group(app: &App) -> Option<Entity> {
    app.world().resource::<DetailsIndex>().group(SECTION_KEY)
}

#[test]
fn selecting_an_entity_shows_its_archetype_and_spawn_tick() {
    let mut app = test_app();
    expand_section(&mut app);
    let subject = inspect(&mut app, Subject::default());

    let entity = app.world().entity(subject);
    let location = entity.location();
    let spawn_tick = entity.spawn_tick().get();
    let components = entity.archetype().component_count();
    assert_eq!(section_value(&app, "index"), subject.index().to_string());
    assert_eq!(
        section_value(&app, "generation"),
        subject.generation().to_string()
    );
    assert_eq!(
        section_value(&app, "archetype"),
        location.archetype_id.index().to_string()
    );
    assert_eq!(
        section_value(&app, "archetype_row"),
        location.archetype_row.index().to_string()
    );
    assert_eq!(
        section_value(&app, "table"),
        location.table_id.as_usize().to_string()
    );
    assert_eq!(
        section_value(&app, "table_row"),
        location.table_row.index().to_string()
    );
    assert_eq!(
        section_value(&app, "archetype_size"),
        components.to_string()
    );
    assert_eq!(section_value(&app, "spawn_tick"), spawn_tick.to_string());
    assert_eq!(section_value(&app, "disabled"), "false");
}

#[test]
fn the_entity_section_starts_collapsed_and_expands_when_toggled() {
    let mut app = test_app();
    inspect(&mut app, Subject::default());
    let group = section_group(&app).unwrap();
    assert_eq!(body_children(&mut app)[0], group);
    assert!(app
        .world()
        .resource::<DetailsIndex>()
        .widget(SECTION_KEY, "archetype")
        .is_none());
    let fields = fields_container(&app, SECTION_KEY);
    assert_eq!(
        app.world().get::<Node>(fields).unwrap().display,
        Display::None
    );

    let toggle = app
        .world_mut()
        .spawn(InspectorDetailsComponent(SECTION_KEY.into()))
        .observe(inspector_details_toggled)
        .id();
    app.world_mut().trigger(ValueChange {
        source: toggle,
        value: true,
        is_final: true,
    });
    app.world_mut().flush();
    refresh(&mut app);
    assert!(app
        .world()
        .resource::<DetailsIndex>()
        .widget(SECTION_KEY, "archetype")
        .is_some());
    assert_eq!(body_children(&mut app)[0], section_group(&app).unwrap());

    app.world_mut().trigger(ValueChange {
        source: toggle,
        value: false,
        is_final: true,
    });
    app.world_mut().flush();
    refresh(&mut app);
    assert!(app
        .world()
        .resource::<DetailsIndex>()
        .widget(SECTION_KEY, "archetype")
        .is_none());
}

#[test]
fn adding_a_component_updates_the_storage_rows_in_place() {
    let mut app = test_app();
    app.register_type::<Tagged>();
    expand_section(&mut app);
    let subject = inspect(&mut app, Subject::default());
    let group = section_group(&app).unwrap();
    let archetype_row = widget_at(&app, "archetype").unwrap();
    let before = app.world().entity(subject).location();

    app.world_mut()
        .entity_mut(subject)
        .insert(Tagged::default());
    refresh(&mut app);

    let after = app.world().entity(subject).location();
    assert_ne!(after.archetype_id, before.archetype_id);
    assert_ne!(after.table_id, before.table_id);
    assert_eq!(section_group(&app), Some(group));
    assert_eq!(widget_at(&app, "archetype"), Some(archetype_row));
    assert_eq!(
        section_value(&app, "archetype"),
        after.archetype_id.index().to_string()
    );
    assert_eq!(
        section_value(&app, "table"),
        after.table_id.as_usize().to_string()
    );
    assert_eq!(
        section_value(&app, "table_row"),
        after.table_row.index().to_string()
    );
}

#[test]
fn a_disabled_entity_shows_as_disabled() {
    let mut app = test_app();
    expand_section(&mut app);
    inspect(&mut app, (Subject::default(), Disabled));

    assert_eq!(section_value(&app, "disabled"), "true");
    assert!(app
        .world()
        .resource::<DetailsIndex>()
        .group(Subject::type_path())
        .is_some());
}

#[test]
fn a_custom_disabling_component_shows_as_disabled() {
    #[derive(Component)]
    struct Hidden;

    let mut app = test_app();
    app.world_mut().register_disabling_component::<Hidden>();
    expand_section(&mut app);
    inspect(&mut app, (Subject::default(), Hidden));

    assert_eq!(section_value(&app, "disabled"), "true");
}

#[test]
fn an_entity_without_components_shows_its_section_above_the_message() {
    let mut app = test_app();
    expand_section(&mut app);
    app.world_mut().spawn((InspectorUi, InspectorDetailsBody));
    let subject = app.world_mut().spawn_empty().id();
    select(
        &mut app,
        Some(InspectableEntity::new(subject, InspectorSource::Local)),
    );
    refresh(&mut app);

    let group = section_group(&app).unwrap();
    assert_eq!(body_children(&mut app).first(), Some(&group));
    assert_eq!(
        body_message(&mut app).as_deref(),
        Some("The selected entity has no components")
    );
    assert_eq!(section_value(&app, "index"), subject.index().to_string());
    assert_eq!(section_value(&app, "archetype_size"), "0");

    app.world_mut().entity_mut(subject).despawn();
    refresh(&mut app);
    assert!(section_group(&app).is_none());
    assert_eq!(
        body_message(&mut app).as_deref(),
        Some("The selected entity no longer exists")
    );
}

#[test]
fn spawned_by_shows_the_spawn_location_when_it_is_tracked() {
    let mut app = test_app();
    expand_section(&mut app);
    app.world_mut().spawn((InspectorUi, InspectorDetailsBody));
    let line = line!() + 1;
    let subject = app.world_mut().spawn(Subject::default()).id();
    select(
        &mut app,
        Some(InspectableEntity::new(subject, InspectorSource::Local)),
    );

    let shown = section_value(&app, "spawned_by");
    if MaybeLocation::caller().into_option().is_some() {
        assert!(
            shown.starts_with(&format!("{}:{line}:", file!())),
            "{shown}"
        );
    } else {
        assert_eq!(
            shown,
            crate::details_panel::entity_section::LOCATION_UNTRACKED
        );
    }
}

#[test]
fn an_unchanged_entity_keeps_its_row_texts_across_refreshes() {
    let mut app = test_app();
    expand_section(&mut app);
    inspect(&mut app, Subject::default());
    settle(&mut app);
    let group = section_group(&app).unwrap();
    let ticks = text_ticks(&mut app, group);
    assert!(!ticks.is_empty());

    settle(&mut app);

    assert_eq!(section_group(&app), Some(group));
    assert_eq!(text_ticks(&mut app, group), ticks);
}

fn text_ticks(app: &mut App, root: Entity) -> Vec<(Entity, Tick)> {
    let mut ticks = Vec::new();
    let mut stack = alloc::vec![root];
    while let Some(entity) = stack.pop() {
        let entity_ref = app.world().entity(entity);
        if let Some(text) = entity_ref.get_ref::<Text>() {
            ticks.push((entity, text.last_changed()));
        }
        if let Some(children) = entity_ref.get::<Children>() {
            stack.extend(children.iter().copied());
        }
    }
    ticks.sort_by_key(|(entity, _)| *entity);
    ticks
}

#[test]
fn a_selected_resource_shows_its_entity_section() {
    #[derive(Resource, Reflect, Default)]
    #[reflect(Resource)]
    struct Gauge {
        level: f32,
    }

    let mut app = test_app();
    app.register_type::<Gauge>();
    expand_section(&mut app);
    app.world_mut().spawn((InspectorUi, InspectorDetailsBody));
    app.init_resource::<Gauge>();
    let id = app.world().component_id::<Gauge>().unwrap();
    let gauge = app.world().resource_entities().get(id).unwrap();
    select(
        &mut app,
        Some(InspectableEntity::new(gauge, InspectorSource::Local)),
    );

    assert!(section_group(&app).is_some());
    assert_eq!(section_value(&app, "index"), gauge.index().to_string());
}

#[cfg(feature = "remote")]
#[test]
fn a_remote_selection_has_no_entity_section() {
    use crate::{
        remote::{world::RemoteWorlds, RemoteSource},
        WorldKind,
    };

    let mut app = test_app();
    expand_section(&mut app);
    app.world_mut().spawn((InspectorUi, InspectorDetailsBody));
    app.insert_resource(InspectorSources::remote_main("localhost", 1));
    app.update();
    let remote = Entity::from_raw_u32(7).unwrap();
    {
        let mut worlds = app.world_mut().resource_mut::<RemoteWorlds>();
        let mirror = worlds.get_mut(WorldKind::Main);
        mirror.spawn(remote).unwrap();
        mirror.write(
            remote,
            crate::remote::world::TreeComponents {
                name: Some("remote".to_string()),
                ..Default::default()
            },
        );
    }
    let source = InspectorSource::Remote(RemoteSource::new("localhost", 1, WorldKind::Main));
    select(&mut app, Some(InspectableEntity::new(remote, source)));

    let index = app.world().resource::<DetailsIndex>();
    assert!(index.group(Name::type_path()).is_some());
    assert!(section_group(&app).is_none());
}

#[test]
fn component_groups_follow_the_section_in_their_usual_order() {
    let mut app = test_app();
    app.register_type::<Tagged>().register_type::<Wrapper>();
    inspect(
        &mut app,
        (Subject::default(), Tagged::default(), Wrapper::default()),
    );

    let selection = app.world().resource::<InspectorSelection>().0.clone();
    let components = inspect_components(app.world(), selection, true);
    assert_eq!(components.len(), 3);
    assert!(components
        .iter()
        .all(|component| &*component.key.0 != SECTION_KEY));

    let index = app.world().resource::<DetailsIndex>();
    let expected: Vec<Entity> = core::iter::once(SECTION_KEY)
        .chain(components.iter().map(|component| &*component.key.0))
        .map(|key| index.group(key).unwrap())
        .collect();
    assert_eq!(body_children(&mut app), expected);
}
