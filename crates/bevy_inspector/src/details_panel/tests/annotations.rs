use super::*;
use crate::details_panel::walk::walk_entries;

#[test]
fn reads_annotations_into_field_entries() {
    let entries = field_entries(&annotated(), &TypeRegistry::new());

    let speed = entry(&entries, "speed");
    assert_eq!(speed.limit, Some(NumericRange::F32(0.0..=10.0)));
    assert_eq!(speed.annotations.step, Some(0.5));
    assert_eq!(speed.annotations.unit, Some("m/s"));
    assert_eq!(entry(&entries, "jump").label, "Jump Height");
    assert_eq!(entry(&entries, "span.0").label, "Left");
    assert_eq!(entry(&entries, "span.1").label, "1");
    assert_eq!(
        entry(&entries, "level").limit,
        Some(NumericRange::I32(0..=255))
    );
    assert_eq!(
        entry(&entries, "count").limit,
        Some(NumericRange::I64(1..=10))
    );
    assert_eq!(entry(&entries, "height.0").annotations.unit, Some("m"));
    assert_eq!(
        entry(&entries, "gait.pace").limit,
        Some(NumericRange::F32(0.0..=1.0))
    );
}

#[test]
fn shows_annotated_fields_read_only() {
    let entries = field_entries(&annotated(), &TypeRegistry::new());

    let grounded = entry(&entries, "grounded");
    assert_eq!(grounded.value, FieldValue::Label("false".to_string()));
    assert!(grounded.annotations.read_only);
    assert_eq!(
        entry(&entries, "mass").value,
        FieldValue::Label("2.5 kg".to_string())
    );
    assert_eq!(
        entry(&entries, "nested.depth").value,
        FieldValue::Label("0".to_string())
    );
    assert_eq!(entry(&entries, "jump").value.kind(), FieldKind::Number);
}

#[test]
fn rejects_edits_to_read_only_fields() {
    let mut app = test_app();
    app.register_type::<Annotated>();
    let entity = app.world_mut().spawn(annotated()).id();

    edit_component::<Annotated>(&mut app, entity, "grounded", FieldValue::Bool(true));
    edit_component::<Annotated>(&mut app, entity, "nested.depth", int(4));
    edit_component::<Annotated>(&mut app, entity, "jump", float(1.5));

    let annotated = app.world().get::<Annotated>(entity).unwrap();
    assert!(!annotated.grounded);
    assert_eq!(annotated.nested.depth, 0);
    assert_eq!(annotated.jump, 1.5);
}

#[test]
fn clamps_edits_to_the_annotated_range() {
    let mut app = test_app();
    app.register_type::<Annotated>();
    let entity = app.world_mut().spawn(annotated()).id();

    edit_component::<Annotated>(&mut app, entity, "speed", float(25.0));
    edit_component::<Annotated>(&mut app, entity, "count", int(50));
    edit_component::<Annotated>(&mut app, entity, "gait.pace", float(-3.0));
    edit_component::<Annotated>(&mut app, entity, "height.0", float(-3.0));

    let annotated = app.world().get::<Annotated>(entity).unwrap();
    assert_eq!(annotated.speed, 10.0);
    assert_eq!(annotated.count, 10);
    assert!(matches!(annotated.gait, Gait::Walk { pace: 0.0 }));
    assert_eq!(annotated.height.0, -3.0);

    edit_component::<Annotated>(&mut app, entity, "speed", float(-1.0));
    assert_eq!(app.world().get::<Annotated>(entity).unwrap().speed, 0.0);
}

#[test]
fn constrains_wrapped_values_with_their_field_annotations() {
    let entries = field_entries(&wrapped(), &TypeRegistry::new());

    let reach = entry(&entries, "reach.0");
    assert_eq!(reach.limit, Some(NumericRange::F32(0.0..=2.0)));
    assert_eq!(reach.annotations.step, Some(0.25));
    assert_eq!(reach.annotations.unit, Some("m"));
    assert_eq!(reach.label, "0");
    assert_eq!(entry(&entries, "reach").label, "Reach");
    assert_eq!(
        entry(&entries, "samples[0]").limit,
        Some(NumericRange::F32(0.0..=1.0))
    );
}

#[test]
fn clamps_edits_to_wrapped_values() {
    let mut app = test_app();
    app.register_type::<Wrapped>();
    let entity = app.world_mut().spawn(wrapped()).id();

    edit_component::<Wrapped>(&mut app, entity, "reach.0", float(5.0));
    edit_component::<Wrapped>(&mut app, entity, "samples[0]", float(-1.0));

    let wrapped = app.world().get::<Wrapped>(entity).unwrap();
    assert_eq!(wrapped.reach, Some(2.0));
    assert_eq!(wrapped.samples, vec![0.0]);
}

#[test]
fn configures_number_inputs_from_annotations() {
    let mut app = test_app();
    app.register_type::<Annotated>();
    inspect(&mut app, annotated());

    let speed = field_widget(&mut app, "speed");
    let world = app.world();
    assert_eq!(
        world.get::<HardLimit>(speed).map(|limit| limit.0.clone()),
        Some(NumericRange::F32(0.0..=10.0))
    );
    assert_eq!(
        world.get::<NumberInputStep>(speed).map(|step| step.0),
        Some(0.5)
    );
    assert!(world.get::<NumberInputSnapToStep>(speed).is_some());
    assert!(has_text(world, speed, "m/s"));

    let jump = widget_at(&app, "jump").unwrap();
    assert!(world.get::<NumberInputStep>(jump).is_none());
    assert!(world.get::<NumberInputSnapToStep>(jump).is_none());
    assert!(!has_text(world, jump, "m/s"));
    assert!(row_labels(&app).contains(&"Jump Height".to_string()));
    assert!(matches!(
        world
            .resource::<DetailsIndex>()
            .fields
            .iter()
            .find(|((_, path), _)| path == "grounded"),
        Some((
            _,
            FieldWidget {
                value: FieldValue::Label(_),
                ..
            }
        ))
    ));
}

#[test]
fn honors_component_annotations() {
    let mut app = test_app();
    app.register_type::<Annotated>();
    app.register_type::<Sealed>();
    let entity = inspect(&mut app, (annotated(), Sealed { value: 1.0 }));

    let names: Vec<String> = inspect_components(
        app.world(),
        Some(InspectableEntity::new(entity, InspectorSource::Local)),
        true,
    )
    .into_iter()
    .map(|component| component.name)
    .collect();
    assert!(
        names.contains(&"Movement Settings".to_string()),
        "{names:?}"
    );
    assert!(names.contains(&"Sealed".to_string()), "{names:?}");

    let value = widget_at(&app, "value").unwrap();
    assert!(app.world().get::<InspectorField>(value).is_none());
    edit_component::<Sealed>(&mut app, entity, "value", float(2.0));
    assert_eq!(app.world().get::<Sealed>(entity).unwrap().value, 1.0);
}

#[test]
fn honors_type_annotations_on_fields() {
    let entries = field_entries(&annotated(), &TypeRegistry::new());

    let caps = entry(&entries, "caps");
    assert_eq!(caps.label, "caps");
    assert!(caps.annotations.read_only);
    assert_eq!(
        entry(&entries, "caps.max").value,
        FieldValue::Label("0".to_string())
    );
    assert_eq!(entry(&entries, "tuning").label, "Custom Tuning");
    assert_eq!(
        entry(&entries, "tuning.gain").value.kind(),
        FieldKind::Number
    );

    let mut app = test_app();
    app.register_type::<Annotated>();
    let entity = app.world_mut().spawn(annotated()).id();
    edit_component::<Annotated>(&mut app, entity, "caps.max", float(4.0));
    assert_eq!(app.world().get::<Annotated>(entity).unwrap().caps.max, 0.0);
    assert!(inspect_components(
        app.world(),
        Some(InspectableEntity::new(entity, InspectorSource::Local)),
        true
    )
    .iter()
    .any(|component| component.name == "Movement Settings"));
}

#[test]
fn shows_variant_display_names() {
    let entries = field_entries(&annotated(), &TypeRegistry::new());
    let stance = entry(&entries, "stance");
    assert_eq!(stance.value, variant(&["Idle", "Crouched"], 0));
    assert_eq!(stance.annotations.variant_label("Idle"), "Standing Still");
    assert_eq!(
        stance.clone().into_read_only().value,
        FieldValue::Label("Standing Still".to_string())
    );

    let mut app = test_app();
    app.add_plugins(bevy_text::TextPlugin);
    app.register_type::<Annotated>();
    let entity = inspect(
        &mut app,
        Annotated {
            stance: Stance::Crouched,
            ..annotated()
        },
    );
    let segments = field_widget(&mut app, "stance");
    assert!(has_text(app.world(), segments, "Standing Still"));
    assert!(!has_text(app.world(), segments, "Idle"));

    let idle = app.world().get::<Children>(segments).unwrap()[0];
    app.world_mut().trigger(Activate { entity: idle });
    app.update();
    assert_eq!(
        app.world().get::<Annotated>(entity).unwrap().stance,
        Stance::Idle
    );
}

#[test]
fn resolves_edit_annotations_like_the_panel() {
    let value = annotated();
    let registry = TypeRegistry::new();
    for full in field_entries(&value, &registry) {
        let targeted = walk_entries(&value, &registry, Some(&full.path));
        let found = entry(&targeted, &full.path);
        assert_eq!(found.annotations, full.annotations, "{}", full.path);
        assert!(targeted.len() < 10, "{}: {targeted:?}", full.path);
    }
}
