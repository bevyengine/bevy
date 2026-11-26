use super::*;
use crate::details_panel::{
    edit::{is_out_of_range, parse_hex_color},
    rows::{color_to_hex, descendant_with},
};

#[test]
fn field_edits_write_into_the_component() {
    let mut app = test_app();
    let subject = app.world_mut().spawn(Subject::default()).id();

    edit(
        &mut app,
        subject,
        "scale",
        FieldValue::Number(NumericValue::F32(2.5)),
    );
    edit(&mut app, subject, "enabled", FieldValue::Bool(true));
    edit(
        &mut app,
        subject,
        "name",
        FieldValue::Text("edited".to_string()),
    );
    edit(
        &mut app,
        subject,
        "nested.depth",
        FieldValue::Number(NumericValue::I32(7)),
    );
    edit(
        &mut app,
        subject,
        "mode",
        FieldValue::Variant {
            variants: alloc::vec!["Idle".to_string(), "Running".to_string()],
            selected: 1,
        },
    );

    let subject = app.world().get::<Subject>(subject).unwrap();
    assert_eq!(subject.scale, 2.5);
    assert!(subject.enabled);
    assert_eq!(subject.name, "edited");
    assert_eq!(subject.nested.depth, 7);
    assert_eq!(subject.mode, Mode::Running);
}

#[test]
fn a_bad_field_path_leaves_the_component_unchanged() {
    let mut app = test_app();
    let subject = app
        .world_mut()
        .spawn(Subject {
            scale: 1.0,
            ..Default::default()
        })
        .id();

    edit(
        &mut app,
        subject,
        "missing.field",
        FieldValue::Number(NumericValue::F32(9.0)),
    );
    edit(
        &mut app,
        subject,
        "scale",
        FieldValue::Text("not a number".to_string()),
    );

    assert_eq!(app.world().get::<Subject>(subject).unwrap().scale, 1.0);
}

#[test]
fn an_edit_is_not_echoed_back_by_the_sync_pass() {
    let mut app = test_app();
    app.init_resource::<EditCount>();
    app.add_observer(|_: On<FieldEdit>, mut count: ResMut<EditCount>| count.0 += 1);
    app.world_mut().spawn((InspectorUi, InspectorDetailsBody));
    let subject = app
        .world_mut()
        .spawn(Subject {
            name: "start".to_string(),
            ..Default::default()
        })
        .id();

    app.world_mut().resource_mut::<InspectorSelection>().0 =
        Some(InspectableEntity::new(subject, InspectorSource::Local));
    app.update();
    assert_eq!(app.world().resource::<EditCount>().0, 0);

    edit(
        &mut app,
        subject,
        "name",
        FieldValue::Text("edited".to_string()),
    );
    assert_eq!(app.world().resource::<EditCount>().0, 1);

    for _ in 0..3 {
        app.world_mut()
            .resource_mut::<DetailsPanelSync>()
            .set_dirty();
        app.update();
    }

    assert_eq!(app.world().resource::<EditCount>().0, 1);
    assert_eq!(app.world().get::<Subject>(subject).unwrap().name, "edited");
}

#[test]
fn edits_every_integer_width() {
    let (mut app, entity) = kinds_app();

    for (path, value) in [
        ("byte", 200),
        ("short", 60_000),
        ("word", 4_000_000_000),
        ("long", i64::MAX),
        ("huge", 7),
        ("size", 9),
        ("tiny", -100),
        ("small", -30_000),
        ("int", -2_000_000_000),
        ("big", i64::MIN),
        ("giant", -7),
        ("signed_size", -9),
    ] {
        edit_kinds(&mut app, entity, path, int(value));
    }

    let kinds = app.world().get::<Kinds>(entity).unwrap();
    assert_eq!(kinds.byte, 200);
    assert_eq!(kinds.short, 60_000);
    assert_eq!(kinds.word, 4_000_000_000);
    assert_eq!(kinds.long, i64::MAX as u64);
    assert_eq!(kinds.huge, 7);
    assert_eq!(kinds.size, 9);
    assert_eq!(kinds.tiny, -100);
    assert_eq!(kinds.small, -30_000);
    assert_eq!(kinds.int, -2_000_000_000);
    assert_eq!(kinds.big, i64::MIN);
    assert_eq!(kinds.giant, -7);
    assert_eq!(kinds.signed_size, -9);
}

#[test]
fn rejects_out_of_range_integers() {
    let (mut app, entity) = kinds_app();
    edit_kinds(&mut app, entity, "byte", int(12));

    edit_kinds(&mut app, entity, "byte", int(300));
    edit_kinds(&mut app, entity, "word", int(-1));
    edit_kinds(
        &mut app,
        entity,
        "tiny",
        FieldValue::Number(NumericValue::F32(1.5)),
    );

    let kinds = app.world().get::<Kinds>(entity).unwrap();
    assert_eq!(kinds.byte, 12);
    assert_eq!(kinds.word, 0);
    assert_eq!(kinds.tiny, 0);
}

#[test]
fn edits_floats_without_losing_precision() {
    let (mut app, entity) = kinds_app();

    edit_kinds(
        &mut app,
        entity,
        "single",
        FieldValue::Number(NumericValue::F32(0.3)),
    );
    edit_kinds(
        &mut app,
        entity,
        "double",
        FieldValue::Number(NumericValue::F64(0.1)),
    );

    let kinds = app.world().get::<Kinds>(entity).unwrap();
    assert_eq!(kinds.single, 0.3);
    assert_eq!(kinds.double, 0.1);
}

#[test]
fn edits_chars_from_the_last_typed_character() {
    let (mut app, entity) = kinds_app();

    edit_kinds(
        &mut app,
        entity,
        "letter",
        FieldValue::Text("q".to_string()),
    );
    edit_kinds(&mut app, entity, "letter", FieldValue::Text(String::new()));
    assert_eq!(app.world().get::<Kinds>(entity).unwrap().letter, 'q');

    edit_kinds(
        &mut app,
        entity,
        "letter",
        FieldValue::Text("qz".to_string()),
    );
    assert_eq!(app.world().get::<Kinds>(entity).unwrap().letter, 'z');
    edit_kinds(
        &mut app,
        entity,
        "letter",
        FieldValue::Text("xy".to_string()),
    );
    assert_eq!(app.world().get::<Kinds>(entity).unwrap().letter, 'y');
}

#[test]
fn edits_strings() {
    let (mut app, entity) = kinds_app();

    edit_kinds(
        &mut app,
        entity,
        "text",
        FieldValue::Text("owned".to_string()),
    );
    edit_kinds(&mut app, entity, "cow", FieldValue::Text("cow".to_string()));
    edit_kinds(
        &mut app,
        entity,
        "fixed",
        FieldValue::Text("static".to_string()),
    );

    let kinds = app.world().get::<Kinds>(entity).unwrap();
    assert_eq!(kinds.text, "owned");
    assert_eq!(kinds.cow, "cow");
    assert_eq!(kinds.fixed, "");
}

#[test]
fn edits_colors_in_their_own_space() {
    let (mut app, entity) = kinds_app();
    app.world_mut().get_mut::<Kinds>(entity).unwrap().color = Color::hsla(0.0, 0.5, 0.5, 1.0);
    let red = parse_hex_color("#FF0000").unwrap();
    assert_eq!(parse_hex_color("#F00"), None);
    assert_eq!(parse_hex_color("#GG0000"), None);
    assert!(parse_hex_color("#00FF0080").is_some());

    edit_kinds(&mut app, entity, "color", FieldValue::Color(red));
    edit_kinds(&mut app, entity, "srgba", FieldValue::Color(red));
    edit_kinds(&mut app, entity, "linear", FieldValue::Color(red));

    let kinds = app.world().get::<Kinds>(entity).unwrap();
    assert!(matches!(kinds.color, Color::Hsla(_)));
    assert_eq!(color_to_hex(kinds.color), "#FF0000");
    assert_eq!(kinds.srgba, Srgba::RED);
    assert_eq!(kinds.linear, LinearRgba::RED);
}

#[test]
fn switches_data_enum_variants_from_defaults() {
    let (mut app, entity) = kinds_app();
    let shape = ["Empty", "Circle", "Custom"];

    edit_kinds(&mut app, entity, "shape", variant(&shape, 1));
    assert_eq!(
        app.world().get::<Kinds>(entity).unwrap().shape,
        Shape::Circle { radius: 0.0 }
    );

    edit_kinds(
        &mut app,
        entity,
        "shape.radius",
        FieldValue::Number(NumericValue::F32(2.0)),
    );
    edit_kinds(&mut app, entity, "shape", variant(&shape, 2));
    assert_eq!(
        app.world().get::<Kinds>(entity).unwrap().shape,
        Shape::Circle { radius: 2.0 }
    );

    edit_kinds(&mut app, entity, "shape", variant(&shape, 0));
    assert_eq!(
        app.world().get::<Kinds>(entity).unwrap().shape,
        Shape::Empty
    );
}

#[test]
fn switches_an_option_between_none_and_some() {
    let (mut app, entity) = kinds_app();
    let options = ["None", "Some"];

    edit_kinds(&mut app, entity, "maybe", variant(&options, 1));
    assert_eq!(app.world().get::<Kinds>(entity).unwrap().maybe, Some(0));

    edit_kinds(&mut app, entity, "maybe.0", int(5));
    assert_eq!(app.world().get::<Kinds>(entity).unwrap().maybe, Some(5));

    edit_kinds(&mut app, entity, "maybe", variant(&options, 0));
    assert_eq!(app.world().get::<Kinds>(entity).unwrap().maybe, None);
}

#[test]
fn edits_nested_and_indexed_leaves() {
    let (mut app, entity) = kinds_app();
    {
        let mut kinds = app.world_mut().get_mut::<Kinds>(entity).unwrap();
        kinds.tags = alloc::vec![1, 2, 3];
        kinds.queue = [1, 2].into_iter().collect();
    }

    edit_kinds(&mut app, entity, "outer.inner.depth", int(4));
    edit_kinds(
        &mut app,
        entity,
        "pair.1",
        FieldValue::Number(NumericValue::F32(1.5)),
    );
    edit_kinds(
        &mut app,
        entity,
        "point.0",
        FieldValue::Number(NumericValue::F32(2.5)),
    );
    edit_kinds(&mut app, entity, "tags[1]", int(20));
    edit_kinds(
        &mut app,
        entity,
        "array[2]",
        FieldValue::Number(NumericValue::F32(3.5)),
    );
    edit_kinds(&mut app, entity, "queue[0]", int(10));

    let kinds = app.world().get::<Kinds>(entity).unwrap();
    assert_eq!(kinds.outer.inner.depth, 4);
    assert_eq!(kinds.pair.1, 1.5);
    assert_eq!(kinds.point, Point(2.5, 0.0));
    assert_eq!(kinds.tags, alloc::vec![1, 20, 3]);
    assert_eq!(kinds.array, [0.0, 0.0, 3.5]);
    assert_eq!(kinds.queue, VecDeque::from([10, 2]));
}

#[test]
fn dragging_a_number_input_edits_the_field() {
    let (mut app, subject) = widget_app();
    let input = field_widget(&mut app, "scale");

    for (value, is_final) in [(1.5_f32, false), (2.0, false), (2.5, true)] {
        app.world_mut().trigger(ValueChange {
            source: input,
            value,
            is_final,
        });
        app.update();
        assert_eq!(app.world().get::<Subject>(subject).unwrap().scale, value);
        assert_eq!(
            app.world().get::<NumericValue>(input),
            Some(&NumericValue::F32(value))
        );
    }

    settle(&mut app);
    assert_eq!(app.world().get::<Subject>(subject).unwrap().scale, 2.5);
    assert_eq!(
        app.world().get::<NumericValue>(input),
        Some(&NumericValue::F32(2.5))
    );
}

#[test]
fn dragging_a_number_input_edits_a_map_value() {
    #[derive(Component, Reflect, Debug, Default)]
    #[reflect(Component)]
    struct Weights(HashMap<String, f32>);

    let mut app = test_app();
    app.add_plugins(bevy_text::TextPlugin);
    app.register_type::<Weights>();
    app.world_mut().spawn((InspectorUi, InspectorDetailsBody));
    let subject = app
        .world_mut()
        .spawn(Weights(
            [("a \"b\"".to_string(), 1.0)].into_iter().collect(),
        ))
        .id();
    app.world_mut().resource_mut::<InspectorSelection>().0 =
        Some(InspectableEntity::new(subject, InspectorSource::Local));
    app.update();
    let input = field_widget(&mut app, "0[\"a \\\"b\\\"\"]");

    app.world_mut().trigger(ValueChange {
        source: input,
        value: 2.5_f32,
        is_final: true,
    });
    app.update();
    settle(&mut app);

    assert_eq!(
        app.world()
            .get::<Weights>(subject)
            .unwrap()
            .0
            .get("a \"b\""),
        Some(&2.5)
    );
    assert_eq!(
        app.world().get::<NumericValue>(input),
        Some(&NumericValue::F32(2.5))
    );
}

#[test]
fn clicking_a_checkbox_edits_the_field() {
    let (mut app, subject) = widget_app();
    let checkbox = field_widget(&mut app, "enabled");

    for value in [true, false, true] {
        app.world_mut().trigger(ValueChange {
            source: checkbox,
            value,
            is_final: true,
        });
        app.update();
        assert_eq!(app.world().get::<Subject>(subject).unwrap().enabled, value);
        assert_eq!(app.world().entity(checkbox).contains::<Checked>(), value);
    }

    settle(&mut app);
    assert!(app.world().get::<Subject>(subject).unwrap().enabled);
    assert!(app.world().entity(checkbox).contains::<Checked>());
}

#[test]
fn typing_into_a_text_input_edits_the_field() {
    let (mut app, subject) = widget_app();
    let input = field_widget(&mut app, "name");
    settle(&mut app);

    app.world_mut()
        .get_mut::<EditableText>(input)
        .unwrap()
        .queue_edit(TextEdit::Insert("!".into()));
    app.update();
    let typed = app
        .world()
        .get::<EditableText>(input)
        .unwrap()
        .value()
        .to_string();
    assert_ne!(typed, "start");
    assert_eq!(app.world().get::<Subject>(subject).unwrap().name, typed);

    settle(&mut app);
    assert_eq!(app.world().get::<Subject>(subject).unwrap().name, typed);
    assert_eq!(
        app.world()
            .get::<EditableText>(input)
            .unwrap()
            .value()
            .to_string(),
        typed
    );
}

#[test]
fn choosing_a_select_option_edits_the_field() {
    let mut app = test_app();
    app.register_type::<Forecast>();
    let subject = inspect(&mut app, Forecast::default());
    let select = field_widget(&mut app, "weather");
    let listbox = descendant_with::<bevy_ui_widgets::ListBox>(app.world(), select).unwrap();
    let rain = option_rows(&app, select)[2].0;

    app.world_mut().trigger(ValueChange {
        source: listbox,
        value: rain,
        is_final: true,
    });
    app.update();
    assert_eq!(
        app.world().get::<Forecast>(subject).unwrap().weather,
        Weather::Rain
    );

    settle(&mut app);
    assert_eq!(
        app.world().get::<Forecast>(subject).unwrap().weather,
        Weather::Rain
    );
    let select = field_widget(&mut app, "weather");
    let selected: Vec<usize> = option_rows(&app, select)
        .into_iter()
        .filter(|(_, _, selected)| *selected)
        .map(|(_, index, _)| index)
        .collect();
    assert_eq!(selected, alloc::vec![2]);
}

#[test]
fn keeps_immutable_components_read_only() {
    let mut app = test_app();
    app.register_type::<Frozen>();
    let entity = inspect(&mut app, Frozen::default());
    let component = Frozen::type_path();

    let index = app.world().resource::<DetailsIndex>();
    assert!(!index.is_empty());
    assert!(index
        .fields
        .values()
        .all(|widget| matches!(widget.value, FieldValue::Label(_))));
    let mut fields = app.world_mut().query::<&InspectorField>();
    assert!(fields
        .iter(app.world())
        .all(|field| &*field.component.0 != component));

    edit_component::<Frozen>(&mut app, entity, "flag", FieldValue::Bool(true));
    refresh(&mut app);
    assert!(!app.world().get::<Frozen>(entity).unwrap().flag);
}

#[test]
fn edits_the_entity_the_panel_was_built_for() {
    let (mut app, first) = widget_app();
    let input = field_widget(&mut app, "scale");
    let second = app
        .world_mut()
        .spawn(Subject {
            scale: 1.0,
            ..Default::default()
        })
        .id();

    app.world_mut().resource_mut::<InspectorSelection>().0 =
        Some(InspectableEntity::new(second, InspectorSource::Local));
    trigger_change(&mut app, input, 3.0_f32);

    assert_eq!(app.world().get::<Subject>(first).unwrap().scale, 3.0);
    assert_eq!(app.world().get::<Subject>(second).unwrap().scale, 1.0);
}

#[test]
fn rejects_non_finite_numbers() {
    let (mut app, entity) = kinds_app();

    for value in [
        NumericValue::F32(f32::NAN),
        NumericValue::F32(f32::INFINITY),
        NumericValue::F64(1e300),
    ] {
        edit_kinds(&mut app, entity, "single", FieldValue::Number(value));
    }
    edit_kinds(
        &mut app,
        entity,
        "double",
        FieldValue::Number(NumericValue::F64(f64::NEG_INFINITY)),
    );

    let kinds = app.world().get::<Kinds>(entity).unwrap();
    assert_eq!(kinds.single, 0.0);
    assert_eq!(kinds.double, 0.0);
}

#[test]
fn only_successful_edits_mark_the_component_changed() {
    let (mut app, entity) = kinds_app();
    let changed = |app: &App| {
        app.world()
            .entity(entity)
            .get_ref::<Kinds>()
            .unwrap()
            .last_changed()
    };
    app.world_mut().increment_change_tick();
    let before = changed(&app);

    edit_kinds(&mut app, entity, "byte", int(300));
    edit_kinds(&mut app, entity, "missing", int(1));
    assert_eq!(changed(&app), before);

    app.world_mut().increment_change_tick();
    edit_kinds(&mut app, entity, "byte", int(12));
    assert_ne!(changed(&app), before);
}

#[test]
fn only_rejected_edits_force_a_sync() {
    let (mut app, entity) = kinds_app();
    let elapsed = |app: &mut App| {
        let mut sync = app.world_mut().resource_mut::<DetailsPanelSync>();
        let elapsed = sync.timer.elapsed();
        sync.timer.reset();
        elapsed
    };
    elapsed(&mut app);

    let byte = |value| FieldValue::Number(NumericValue::I32(value));
    edit_kinds(&mut app, entity, "byte", byte(12));
    assert_eq!(elapsed(&mut app), Duration::ZERO);

    let red = Color::hsla(0.0, 1.0, 0.5, 1.0);
    edit_kinds(&mut app, entity, "color", FieldValue::Color(red));
    assert_eq!(elapsed(&mut app), Duration::ZERO);

    edit_kinds(&mut app, entity, "byte", byte(300));
    assert_ne!(elapsed(&mut app), Duration::ZERO);
}

#[test]
fn dragging_an_integer_to_the_same_value_does_not_mark_it_changed() {
    let (mut app, entity) = kinds_app();
    app.world_mut().spawn((InspectorUi, InspectorDetailsBody));
    select(
        &mut app,
        Some(InspectableEntity::new(entity, InspectorSource::Local)),
    );
    let input = field_widget(&mut app, "byte");
    let changed = |app: &App| {
        app.world()
            .entity(entity)
            .get_ref::<Kinds>()
            .unwrap()
            .last_changed()
    };

    trigger_change(&mut app, input, 7_i32);
    assert_eq!(app.world().get::<Kinds>(entity).unwrap().byte, 7);
    let before = changed(&app);

    for _ in 0..3 {
        app.world_mut().increment_change_tick();
        trigger_change(&mut app, input, 7_i32);
    }
    assert_eq!(changed(&app), before);
}

#[test]
fn an_unchanged_text_edit_keeps_the_string_and_change_tick() {
    let (mut app, entity) = kinds_app();
    edit_kinds(&mut app, entity, "text", FieldValue::Text("same".into()));
    let state = |app: &App| {
        let kinds = app.world().entity(entity).get_ref::<Kinds>().unwrap();
        (kinds.text.as_ptr(), kinds.last_changed())
    };
    let before = state(&app);

    app.world_mut().increment_change_tick();
    edit_kinds(&mut app, entity, "text", FieldValue::Text("same".into()));
    assert_eq!(state(&app), before);
}

#[test]
fn an_unchanged_variant_choice_does_not_mark_it_changed() {
    let mut app = test_app();
    let subject = app.world_mut().spawn(Subject::default()).id();
    let changed = |app: &App| {
        app.world()
            .entity(subject)
            .get_ref::<Subject>()
            .unwrap()
            .last_changed()
    };
    app.world_mut().increment_change_tick();
    let before = changed(&app);

    edit(&mut app, subject, "mode", variant(&["Idle", "Running"], 0));
    assert_eq!(changed(&app), before);

    edit(&mut app, subject, "mode", variant(&["Idle", "Running"], 1));
    assert_ne!(changed(&app), before);
}

#[test]
fn reverts_the_widget_after_a_rejected_edit() {
    let (mut app, entity) = kinds_app();
    app.world_mut().spawn((InspectorUi, InspectorDetailsBody));
    select(
        &mut app,
        Some(InspectableEntity::new(entity, InspectorSource::Local)),
    );
    let byte = field_widget(&mut app, "byte");

    trigger_change(&mut app, byte, 300_i32);
    app.update();

    assert_eq!(app.world().get::<Kinds>(entity).unwrap().byte, 0);
    assert_eq!(
        app.world().get::<NumericValue>(byte),
        Some(&NumericValue::I32(0))
    );
}

#[test]
fn applies_an_edit_back_to_a_value_the_game_replaced() {
    let (mut app, subject) = widget_app();
    let input = field_widget(&mut app, "scale");

    app.world_mut().get_mut::<Subject>(subject).unwrap().scale = 5.0;
    trigger_change(&mut app, input, 1.0_f32);

    assert_eq!(app.world().get::<Subject>(subject).unwrap().scale, 1.0);
}

#[test]
fn edits_a_root_level_unit_enum_component() {
    let mut app = test_app();
    app.register_type::<Level>();
    let entity = inspect(&mut app, Level::Low);
    let component = Level::type_path();
    assert_eq!(tracked_paths(&app), alloc::vec![String::new()]);

    edit_component::<Level>(&mut app, entity, "", variant(&["Low", "High"], 1));
    app.update();

    assert_eq!(app.world().get::<Level>(entity), Some(&Level::High));
    let index = app.world().resource::<DetailsIndex>();
    assert_eq!(
        index.fields[&(component.into(), String::new())].value,
        variant(&["Low", "High"], 1)
    );
}

#[cfg(feature = "remote")]
#[test]
fn ignores_edits_outside_local_mode() {
    let mut app = test_app();
    app.world_mut()
        .insert_resource(InspectorSources::remote_main("localhost", 1));
    let subject = inspect(
        &mut app,
        Subject {
            scale: 1.0,
            ..Default::default()
        },
    );

    edit(
        &mut app,
        subject,
        "scale",
        FieldValue::Number(NumericValue::F32(2.0)),
    );

    assert_eq!(app.world().get::<Subject>(subject).unwrap().scale, 1.0);
    let mut fields = app.world_mut().query::<&InspectorField>();
    assert_eq!(fields.iter(app.world()).count(), 0);
}

#[test]
fn picking_a_color_edits_the_field() {
    let (mut app, entity) = kinds_app();
    app.world_mut().spawn((InspectorUi, InspectorDetailsBody));
    app.world_mut().resource_mut::<InspectorSelection>().0 =
        Some(InspectableEntity::new(entity, InspectorSource::Local));
    app.update();
    let mut pickers = app
        .world_mut()
        .query_filtered::<(Entity, &InspectorField), With<FeathersColorInput>>();
    let picker = pickers
        .iter(app.world())
        .find(|(_, field)| field.path == "color")
        .map(|(picker, _)| picker)
        .unwrap();

    for value in [Color::srgb(1.0, 0.0, 0.0), Color::srgb(0.0, 1.0, 0.0)] {
        app.world_mut().trigger(ValueChange {
            source: picker,
            value,
            is_final: false,
        });
        app.update();
        let color = app.world().get::<Kinds>(entity).unwrap().color;
        assert_eq!(color_to_hex(color), color_to_hex(value));
        assert_eq!(
            app.world()
                .get::<ColorInputValue>(picker)
                .map(|input| input.0),
            Some(value)
        );
    }

    settle(&mut app);
    let color = app.world().get::<Kinds>(entity).unwrap().color;
    assert_eq!(color_to_hex(color), "#00FF00");
}

#[test]
fn hex_input_commits_on_focus_lost() {
    let (mut app, entity) = kinds_app();
    let input = hex_input(&mut app, entity);
    let before = app.world().get::<Kinds>(entity).unwrap().color;

    type_text(&mut app, input, "#00FF00");
    assert_eq!(app.world().get::<Kinds>(entity).unwrap().color, before);

    app.world_mut().trigger(FocusLost { entity: input });
    app.update();
    let color = app.world().get::<Kinds>(entity).unwrap().color;
    assert_eq!(color_to_hex(color), "#00FF00");
}

#[test]
fn hex_input_restores_invalid_text() {
    let (mut app, entity) = kinds_app();
    let input = hex_input(&mut app, entity);
    let before = app.world().get::<Kinds>(entity).unwrap().color;

    type_text(&mut app, input, "#00FF");
    app.world_mut().trigger(FocusLost { entity: input });
    app.update();

    assert_eq!(app.world().get::<Kinds>(entity).unwrap().color, before);
    assert_eq!(input_text(&app, input), color_to_hex(before));
}

#[test]
fn detects_out_of_range_colors() {
    assert!(!is_out_of_range(Color::WHITE));
    assert!(!is_out_of_range(Color::hsla(0.0, 0.5, 0.5, 1.0)));
    assert!(is_out_of_range(Color::linear_rgb(2.0, 1.0, 1.0)));
    assert!(is_out_of_range(Color::srgb(-0.5, 0.0, 0.0)));
}
