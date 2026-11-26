use super::*;
use crate::details_panel::rows::descendant_with;

#[test]
fn keeps_opaque_captions_stable() {
    let mut app = test_app();
    app.register_type::<Tagged>();
    app.world_mut().spawn((InspectorUi, InspectorDetailsBody));
    let subject = app
        .world_mut()
        .spawn(Tagged {
            label: Name::new("hello"),
        })
        .id();

    app.world_mut().resource_mut::<InspectorSelection>().0 =
        Some(InspectableEntity::new(subject, InspectorSource::Local));
    app.update();

    fn captions(app: &App) -> Vec<(String, String)> {
        let index = app.world().resource::<DetailsIndex>();
        let mut rows: Vec<(String, String)> = index
            .fields
            .iter()
            .filter(|(_, widget)| matches!(widget.value, FieldValue::Label(_)))
            .map(|(key, widget)| {
                let text = app
                    .world()
                    .get::<Text>(widget.entity)
                    .map(|text| text.0.clone())
                    .unwrap_or_default();
                (key.1.clone(), text)
            })
            .collect();
        rows.sort();
        rows
    }

    let first = captions(&app);
    assert!(!first.is_empty());
    assert!(first.iter().all(|(_, text)| !text.is_empty()));

    for _ in 0..3 {
        app.world_mut()
            .resource_mut::<DetailsPanelSync>()
            .set_dirty();
        app.update();
    }

    assert_eq!(captions(&app), first);
}

#[test]
fn wraps_long_value_captions() {
    let mut app = test_app();
    app.register_type::<Holder>();
    app.world_mut().spawn((InspectorUi, InspectorDetailsBody));
    let subject = app.world_mut().spawn(Holder(Arc::new(StrongHandle))).id();

    app.world_mut().resource_mut::<InspectorSelection>().0 =
        Some(InspectableEntity::new(subject, InspectorSource::Local));
    app.update();

    let index = app.world().resource::<DetailsIndex>();
    let captions: Vec<Entity> = index
        .fields
        .values()
        .filter(|widget| matches!(widget.value, FieldValue::Label(_)))
        .map(|widget| widget.entity)
        .collect();
    assert!(!captions.is_empty());
    for caption in captions {
        let layout = app.world().get::<TextLayout>(caption).unwrap();
        assert_eq!(layout.linebreak, LineBreak::AnyCharacter);
        let node = app.world().get::<Node>(caption).unwrap();
        assert_eq!(node.min_width, px(0));
        assert_eq!(node.flex_shrink, 1.0);
    }
}

#[test]
fn clips_labels_to_a_single_line() {
    let mut app = test_app();
    inspect(&mut app, Subject::default());

    let rows = row_entities(&app);
    assert!(!rows.is_empty());
    let world = app.world();
    for row in rows {
        let label = descendant_with::<Text>(world, row).unwrap();
        let leading = world.get::<ChildOf>(label).unwrap().parent();
        assert_eq!(
            world.get::<Node>(leading).unwrap().overflow,
            Overflow::clip_x()
        );
        let layout = world.get::<TextLayout>(label).unwrap();
        assert_eq!(layout.linebreak, LineBreak::NoWrap);
    }
    let scale = widget_at(&app, "scale").unwrap();
    let row = world.get::<ChildOf>(scale).unwrap().parent();
    let leading = world.get::<Children>(row).unwrap()[0];
    assert!(world.entity(leading).contains::<ColumnSplitLeading>());
}

#[test]
fn finds_the_first_descendant_in_child_order() {
    let mut world = World::new();
    let root = world.spawn(Marker).id();
    let first = world.spawn(ChildOf(root)).id();
    let nested = world.spawn((Marker, ChildOf(first))).id();
    world.spawn((Marker, ChildOf(root)));

    assert_eq!(descendant_with::<Marker>(&world, root), Some(nested));
}

#[test]
fn never_returns_the_root_itself() {
    let mut world = World::new();
    let root = world.spawn(Marker).id();
    world.spawn(ChildOf(root));

    assert_eq!(descendant_with::<Marker>(&world, root), None);
}

#[test]
fn finds_nothing_under_a_missing_root() {
    let mut world = World::new();
    let root = world.spawn_empty().id();
    world.despawn(root);

    assert_eq!(descendant_with::<Marker>(&world, root), None);
}

#[test]
fn vectors_show_an_input_per_axis_under_one_label() {
    let mut app = test_app();
    app.register_type::<Motion>();
    let subject = inspect(&mut app, Motion::default());

    let row = widget_at(&app, "velocity").unwrap();
    let world = app.world();
    let label = descendant_with::<Text>(world, row).unwrap();
    assert_eq!(world.get::<Text>(label).unwrap().0, "velocity");
    let inputs: Vec<Entity> = ["velocity.x", "velocity.y", "velocity.z"]
        .into_iter()
        .map(|path| widget_at(&app, path).unwrap())
        .collect();
    let line = world.get::<ChildOf>(inputs[0]).unwrap().parent();
    assert_eq!(world.get::<ChildOf>(line).unwrap().parent(), row);
    for (input, axis) in inputs.iter().zip(["X", "Y", "Z"]) {
        assert_eq!(world.get::<ChildOf>(*input).unwrap().parent(), line);
        assert!(world.entity(*input).contains::<FeathersNumberInput>());
        assert!(has_text(world, *input, axis));
    }
    let speed = widget_at(&app, "speed").unwrap();
    let speed_row = world.get::<ChildOf>(speed).unwrap().parent();
    let leading = world.get::<Children>(speed_row).unwrap()[0];
    assert!(world.entity(leading).contains::<ColumnSplitLeading>());

    trigger_change(&mut app, inputs[1], 2.5_f32);
    assert_eq!(app.world().get::<Motion>(subject).unwrap().velocity.y, 2.5);
}

#[test]
fn vector_inputs_show_a_changed_value_in_place() {
    let mut app = test_app();
    app.register_type::<Motion>();
    let subject = inspect(&mut app, Motion::default());
    let input = widget_at(&app, "velocity.z").unwrap();

    app.world_mut()
        .get_mut::<Motion>(subject)
        .unwrap()
        .velocity
        .z = 3.0;
    refresh(&mut app);

    assert_eq!(widget_at(&app, "velocity.z"), Some(input));
    assert_eq!(
        app.world().get::<NumericValue>(input),
        Some(&NumericValue::F32(3.0))
    );
}

#[test]
fn struct_fields_are_grouped_in_a_collapsible_block() {
    let mut app = test_app();
    inspect(&mut app, Subject::default());

    let block = widget_at(&app, "nested").unwrap();
    let depth = widget_at(&app, "nested.depth").unwrap();
    let world = app.world();
    let body = descendant_with::<InspectorFieldGroupBody>(world, block).unwrap();
    assert!(world
        .get::<ChildOf>(depth)
        .and_then(|row| world.get::<ChildOf>(row.parent()))
        .is_some_and(|parent| parent.parent() == body));

    let toggle = descendant_with::<InspectorFieldGroupToggle>(world, block).unwrap();
    trigger_change(&mut app, toggle, false);
    assert_eq!(
        app.world().get::<Node>(body).unwrap().display,
        Display::None
    );
    trigger_change(&mut app, toggle, true);
    assert_eq!(
        app.world().get::<Node>(body).unwrap().display,
        Display::Flex
    );
}

#[test]
fn clicking_a_segment_of_a_small_unit_enum_writes_the_variant() {
    let (mut app, subject) = widget_app();
    let segments = field_widget(&mut app, "mode");
    assert!(app.world().get::<FeathersSelect>(segments).is_none());
    assert_eq!(
        segment_variants(&app, segments),
        [ButtonVariant::Primary, ButtonVariant::Normal]
    );

    let running = app.world().get::<Children>(segments).unwrap()[1];
    app.world_mut().trigger(Activate { entity: running });
    app.update();
    assert_eq!(
        app.world().get::<Subject>(subject).unwrap().mode,
        Mode::Running
    );
    assert_eq!(
        segment_variants(&app, segments),
        [ButtonVariant::Normal, ButtonVariant::Primary]
    );

    settle(&mut app);
    assert_eq!(
        app.world().get::<Subject>(subject).unwrap().mode,
        Mode::Running
    );
    let segments = field_widget(&mut app, "mode");
    assert_eq!(
        segment_variants(&app, segments),
        [ButtonVariant::Normal, ButtonVariant::Primary]
    );
}

#[test]
fn limits_integer_inputs_to_their_type() {
    fn limit(app: &mut App, path: &str) -> Option<NumericRange> {
        let input = field_widget(app, path);
        app.world()
            .get::<HardLimit>(input)
            .map(|limit| limit.0.clone())
    }

    let (mut app, entity) = kinds_app();
    app.world_mut().spawn((InspectorUi, InspectorDetailsBody));
    select(
        &mut app,
        Some(InspectableEntity::new(entity, InspectorSource::Local)),
    );

    assert_eq!(limit(&mut app, "byte"), Some(NumericRange::I32(0..=255)));
    assert_eq!(limit(&mut app, "tiny"), Some(NumericRange::I32(-128..=127)));
    assert_eq!(
        limit(&mut app, "word"),
        Some(NumericRange::I64(0..=i64::from(u32::MAX)))
    );
    assert_eq!(
        limit(&mut app, "size"),
        Some(NumericRange::I64(0..=i64::MAX))
    );
    assert_eq!(limit(&mut app, "int"), None);
    assert_eq!(limit(&mut app, "big"), None);
    assert_eq!(limit(&mut app, "single"), None);
}
