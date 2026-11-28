use super::*;
use crate::details_panel::{
    entity_section::SECTION_KEY,
    ticks::{header_marked, marked_paths, name_color},
};
use bevy_app::Update;
use bevy_ecs::change_detection::DetectChangesMut;
use bevy_feathers::tokens;

fn ticks_shown(app: &App, component: &str) -> Option<String> {
    let world = app.world();
    let group = world.resource::<DetailsIndex>().groups.get(component)?;
    Some(world.get::<Text>(group.ticks?)?.0.clone())
}

fn expected_ticks(app: &App, entity: Entity, component: ComponentId) -> String {
    let ticks = app
        .world()
        .entity(entity)
        .get_change_ticks_by_id(component)
        .unwrap();
    format!(
        "added {}, changed {}",
        ticks.added.get(),
        ticks.changed.get()
    )
}

fn marked(app: &App, component: &str) -> bool {
    header_marked(app.world(), component)
}

fn marked_rows(app: &App, component: &str) -> Vec<String> {
    marked_paths(app.world(), component)
}

fn speed_up(mut motions: Query<&mut Motion>) {
    for mut motion in &mut motions {
        motion.speed += 1.0;
    }
}

fn motion_app() -> (App, Entity) {
    let mut app = test_app();
    app.register_type::<Motion>();
    app.add_systems(Update, speed_up);
    let entity = inspect(&mut app, (Subject::default(), Motion::default()));
    (app, entity)
}

#[test]
fn a_component_card_shows_its_added_and_changed_ticks() {
    let mut app = test_app();
    let entity = inspect(&mut app, Subject::default());
    refresh(&mut app);

    let id = app.world().component_id::<Subject>().unwrap();
    assert_eq!(
        ticks_shown(&app, Subject::type_path()),
        Some(expected_ticks(&app, entity, id))
    );
}

#[test]
fn the_shown_ticks_follow_a_write() {
    let mut app = test_app();
    let entity = inspect(&mut app, Subject::default());
    let before = ticks_shown(&app, Subject::type_path());

    app.world_mut().get_mut::<Subject>(entity).unwrap().scale = 3.0;
    refresh(&mut app);

    let id = app.world().component_id::<Subject>().unwrap();
    let after = ticks_shown(&app, Subject::type_path());
    assert_ne!(after, before);
    assert_eq!(after, Some(expected_ticks(&app, entity, id)));
}

#[test]
fn a_component_written_every_frame_stays_marked() {
    let (mut app, _) = motion_app();
    assert!(!marked(&app, Motion::type_path()));

    for _ in 0..3 {
        refresh(&mut app);
        assert!(marked(&app, Motion::type_path()));
        assert_eq!(marked_rows(&app, Motion::type_path()), ["speed"]);
    }
}

#[test]
fn an_idle_component_is_not_marked() {
    let (mut app, _) = motion_app();
    settle(&mut app);

    assert!(marked(&app, Motion::type_path()));
    assert!(!marked(&app, Subject::type_path()));
}

#[test]
fn the_marks_clear_after_a_refresh_without_a_write() {
    let mut app = test_app();
    let entity = inspect(&mut app, Subject::default());

    app.world_mut().get_mut::<Subject>(entity).unwrap().scale = 2.0;
    refresh(&mut app);
    assert!(marked(&app, Subject::type_path()));
    assert_eq!(marked_rows(&app, Subject::type_path()), ["scale"]);

    refresh(&mut app);
    assert!(!marked(&app, Subject::type_path()));
    assert!(marked_rows(&app, Subject::type_path()).is_empty());
}

#[test]
fn only_the_rows_of_the_changed_fields_are_marked() {
    let mut app = test_app();
    let entity = inspect(&mut app, Subject::default());

    let mut subject = app.world_mut().get_mut::<Subject>(entity).unwrap();
    subject.enabled = true;
    subject.nested.depth = 3;
    refresh(&mut app);

    assert_eq!(
        marked_rows(&app, Subject::type_path()),
        ["enabled", "nested.depth"]
    );
}

#[test]
fn a_changed_variant_marks_its_row() {
    let mut app = test_app();
    let entity = inspect(&mut app, Subject::default());

    app.world_mut().get_mut::<Subject>(entity).unwrap().mode = Mode::Running;
    refresh(&mut app);

    assert!(marked(&app, Subject::type_path()));
    assert_eq!(marked_rows(&app, Subject::type_path()), ["mode"]);
}

#[test]
fn a_changed_vector_axis_marks_the_vector_row() {
    let mut app = test_app();
    app.register_type::<Motion>();
    let entity = inspect(&mut app, Motion::default());

    app.world_mut()
        .get_mut::<Motion>(entity)
        .unwrap()
        .velocity
        .y = 1.0;
    refresh(&mut app);

    let marked = marked_rows(&app, Motion::type_path());
    assert!(marked.contains(&"velocity.y".to_string()));
    assert!(!marked.contains(&"speed".to_string()));
}

#[test]
fn a_write_that_keeps_every_value_marks_only_the_header() {
    let mut app = test_app();
    let entity = inspect(&mut app, Subject::default());

    app.world_mut()
        .get_mut::<Subject>(entity)
        .unwrap()
        .set_changed();
    refresh(&mut app);

    assert!(marked(&app, Subject::type_path()));
    assert!(marked_rows(&app, Subject::type_path()).is_empty());
}

#[test]
fn a_written_component_keeps_the_color_of_its_name() {
    let mut app = test_app();
    let entity = inspect(&mut app, Subject::default());
    assert_eq!(
        name_color(app.world(), Subject::type_path()),
        Some(tokens::LISTROW_TEXT)
    );

    app.world_mut().get_mut::<Subject>(entity).unwrap().scale = 2.0;
    refresh(&mut app);

    assert!(marked(&app, Subject::type_path()));
    assert_eq!(
        name_color(app.world(), Subject::type_path()),
        Some(tokens::LISTROW_TEXT)
    );
}

#[test]
fn a_collapsed_component_is_marked_while_written_and_shows_no_ticks() {
    let mut app = test_app();
    app.world_mut()
        .resource_mut::<DetailsCollapsed>()
        .toggled
        .insert(Subject::type_path().into());
    let entity = inspect(&mut app, Subject::default());

    app.world_mut()
        .get_mut::<Subject>(entity)
        .unwrap()
        .set_changed();
    refresh(&mut app);

    assert!(marked(&app, Subject::type_path()));
    assert!(marked_rows(&app, Subject::type_path()).is_empty());
    assert_eq!(ticks_shown(&app, Subject::type_path()), None);
}

#[test]
fn expanding_a_collapsed_component_shows_its_ticks() {
    let mut app = test_app();
    app.world_mut()
        .resource_mut::<DetailsCollapsed>()
        .toggled
        .insert(Subject::type_path().into());
    let entity = inspect(&mut app, Subject::default());
    assert_eq!(ticks_shown(&app, Subject::type_path()), None);

    app.world_mut()
        .resource_mut::<DetailsCollapsed>()
        .toggled
        .clear();
    refresh(&mut app);

    let id = app.world().component_id::<Subject>().unwrap();
    assert_eq!(
        ticks_shown(&app, Subject::type_path()),
        Some(expected_ticks(&app, entity, id))
    );
}

#[test]
fn an_edit_made_through_the_panel_marks_nothing() {
    let mut app = test_app();
    app.insert_resource(InspectorSources::local());
    let entity = inspect(&mut app, Subject::default());
    let before = ticks_shown(&app, Subject::type_path());

    edit(&mut app, entity, "scale", float(4.0));
    refresh(&mut app);

    assert_eq!(app.world().get::<Subject>(entity).unwrap().scale, 4.0);
    assert_ne!(ticks_shown(&app, Subject::type_path()), before);
    assert!(!marked(&app, Subject::type_path()));
    assert!(marked_rows(&app, Subject::type_path()).is_empty());
}

#[test]
fn an_app_write_after_a_panel_edit_is_marked() {
    let mut app = test_app();
    app.insert_resource(InspectorSources::local());
    let entity = inspect(&mut app, Subject::default());

    edit(&mut app, entity, "scale", float(4.0));
    app.update();
    app.world_mut().get_mut::<Subject>(entity).unwrap().scale = 5.0;
    refresh(&mut app);

    assert!(marked(&app, Subject::type_path()));
    assert_eq!(marked_rows(&app, Subject::type_path()), ["scale"]);
}

#[test]
fn selecting_another_entity_does_not_mark_its_components() {
    let mut app = test_app();
    inspect(&mut app, Subject::default());
    let other = app.world_mut().spawn(Subject::default()).id();
    app.world_mut()
        .get_mut::<Subject>(other)
        .unwrap()
        .set_changed();

    select(
        &mut app,
        Some(InspectableEntity::new(other, InspectorSource::Local)),
    );

    assert!(!marked(&app, Subject::type_path()));
    let id = app.world().component_id::<Subject>().unwrap();
    assert_eq!(
        ticks_shown(&app, Subject::type_path()),
        Some(expected_ticks(&app, other, id))
    );
}

#[test]
fn the_entity_section_has_no_ticks_or_marks() {
    let (mut app, _) = motion_app();
    settle(&mut app);

    assert!(app
        .world()
        .resource::<DetailsIndex>()
        .group(SECTION_KEY)
        .is_some());
    assert_eq!(ticks_shown(&app, SECTION_KEY), None);
    assert!(!marked(&app, SECTION_KEY));
}

#[test]
fn a_resource_written_between_refreshes_is_marked() {
    #[derive(Resource, Reflect, Default)]
    #[reflect(Resource)]
    struct Gauge {
        level: f32,
    }

    let mut app = test_app();
    app.register_type::<Gauge>();
    app.world_mut().spawn((InspectorUi, InspectorDetailsBody));
    app.init_resource::<Gauge>();
    let id = app.world().component_id::<Gauge>().unwrap();
    let gauge = app.world().resource_entities().get(id).unwrap();
    select(
        &mut app,
        Some(InspectableEntity::new(gauge, InspectorSource::Local)),
    );
    assert_eq!(
        ticks_shown(&app, Gauge::type_path()),
        Some(expected_ticks(&app, gauge, id))
    );
    assert!(!marked(&app, Gauge::type_path()));

    app.world_mut().resource_mut::<Gauge>().level = 1.0;
    refresh(&mut app);
    assert!(marked(&app, Gauge::type_path()));

    refresh(&mut app);
    assert!(!marked(&app, Gauge::type_path()));
}

#[cfg(feature = "remote")]
#[test]
fn a_remote_selection_shows_no_ticks() {
    use crate::{
        remote::{world::RemoteWorlds, RemoteSource},
        WorldKind,
    };

    let mut app = test_app();
    app.world_mut().spawn((InspectorUi, InspectorDetailsBody));
    app.insert_resource(InspectorSources::remote_main("localhost", 1));
    app.update();
    let remote = Entity::from_raw_u32(7).unwrap();
    let write = |app: &mut App, name: &str| {
        let mut worlds = app.world_mut().resource_mut::<RemoteWorlds>();
        let mirror = worlds.get_mut(WorldKind::Main);
        mirror.write(
            remote,
            crate::remote::world::TreeComponents {
                name: Some(name.to_string()),
                ..Default::default()
            },
        );
    };
    app.world_mut()
        .resource_mut::<RemoteWorlds>()
        .get_mut(WorldKind::Main)
        .spawn(remote)
        .unwrap();
    write(&mut app, "remote");
    let source = InspectorSource::Remote(RemoteSource::new("localhost", 1, WorldKind::Main));
    select(&mut app, Some(InspectableEntity::new(remote, source)));

    write(&mut app, "renamed");
    refresh(&mut app);

    let index = app.world().resource::<DetailsIndex>();
    assert!(index.group(Name::type_path()).is_some());
    assert_eq!(ticks_shown(&app, Name::type_path()), None);
    assert!(!marked(&app, Name::type_path()));
}
