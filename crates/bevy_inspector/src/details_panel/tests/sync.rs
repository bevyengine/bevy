use super::*;
use crate::details_panel::rows::color_to_hex;

#[test]
fn survives_repeated_selection_changes() {
    let mut app = test_app();
    app.world_mut().spawn((InspectorUi, InspectorDetailsBody));
    let subjects: Vec<Entity> = (0..3)
        .map(|index| {
            app.world_mut()
                .spawn(Subject {
                    scale: index as f32,
                    name: index.to_string(),
                    ..Default::default()
                })
                .id()
        })
        .collect();

    for subject in subjects {
        app.world_mut().resource_mut::<InspectorSelection>().0 =
            Some(InspectableEntity::new(subject, InspectorSource::Local));
        for _ in 0..5 {
            app.update();
        }

        let index = app.world().resource::<DetailsIndex>();
        assert!(!index.is_empty());
        let widgets: Vec<Entity> = index.fields.values().map(|field| field.entity).collect();
        for widget in widgets {
            assert!(app.world().get_entity(widget).is_ok());
        }
    }

    app.world_mut().resource_mut::<InspectorSelection>().0 = None;
    for _ in 0..5 {
        app.update();
    }
    assert!(app.world().resource::<DetailsIndex>().is_empty());
}

#[test]
fn rebuilds_on_selection_and_updates_values_in_place() {
    let mut app = test_app();
    let ui_root = app.world_mut().spawn(InspectorUi).id();
    app.world_mut().spawn((InspectorTreeView, ChildOf(ui_root)));
    let panel = app
        .world_mut()
        .spawn((InspectorDetailsBody, ChildOf(ui_root)))
        .id();
    let subject = app
        .world_mut()
        .spawn(Subject {
            scale: 1.0,
            ..Default::default()
        })
        .id();

    app.update();
    assert!(app.world().resource::<DetailsIndex>().is_empty());
    assert_eq!(
        app.world()
            .get::<Children>(panel)
            .map(|children| children.iter().count()),
        Some(1)
    );

    app.world_mut().resource_mut::<InspectorSelection>().0 =
        Some(InspectableEntity::new(subject, InspectorSource::Local));
    app.update();

    let component = Subject::type_path();
    let index = app.world().resource::<DetailsIndex>();
    let scale = index.widget(component, "scale").unwrap();
    let enabled = index.widget(component, "enabled").unwrap();
    assert_eq!(
        app.world().get::<NumericValue>(scale),
        Some(&NumericValue::F32(1.0))
    );

    app.world_mut().get_mut::<Subject>(subject).unwrap().scale = 4.0;
    app.world_mut()
        .resource_mut::<DetailsPanelSync>()
        .set_dirty();
    app.update();

    let index = app.world().resource::<DetailsIndex>();
    assert_eq!(index.widget(component, "scale"), Some(scale));
    assert_eq!(index.widget(component, "enabled"), Some(enabled));
    assert_eq!(
        app.world().get::<NumericValue>(scale),
        Some(&NumericValue::F32(4.0))
    );
}

#[test]
fn groups_start_collapsed_when_configured_and_expand_when_toggled() {
    let mut app = test_app();
    app.world_mut()
        .resource_mut::<DetailsCollapsed>()
        .by_default = true;
    inspect(&mut app, Subject::default());
    let subject = Subject::type_path();
    let index = app.world().resource::<DetailsIndex>();
    assert!(index.group(subject).is_some());
    assert!(index.widget(subject, "scale").is_none());
    let fields = fields_container(&app, subject);
    assert_eq!(
        app.world().get::<Node>(fields).unwrap().display,
        Display::None
    );

    let toggle = app
        .world_mut()
        .spawn(InspectorDetailsComponent(subject.into()))
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
        .widget(subject, "scale")
        .is_some());

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
        .widget(subject, "scale")
        .is_none());
}

#[test]
fn keeps_each_group_column_split_across_respawns() {
    let mut app = test_app();
    app.register_type::<Wrapper>();
    inspect(&mut app, (Subject::default(), Wrapper::default()));
    let subject = Subject::type_path();
    let wrapper = Wrapper::type_path();

    let container = fields_container(&app, subject);
    app.world_mut()
        .get_mut::<ColumnSplit>(container)
        .unwrap()
        .fraction = 0.6;
    app.update();

    app.world_mut()
        .resource_mut::<DetailsCollapsed>()
        .toggled
        .insert(subject.into());
    refresh(&mut app);
    app.world_mut()
        .resource_mut::<DetailsCollapsed>()
        .toggled
        .remove(subject);
    refresh(&mut app);

    let respawned = fields_container(&app, subject);
    assert_ne!(respawned, container);
    let fraction = |entity| app.world().get::<ColumnSplit>(entity).unwrap().fraction;
    assert_eq!(fraction(respawned), 0.6);
    assert_eq!(fraction(fields_container(&app, wrapper)), 0.4);
}

#[test]
fn keeps_collapsed_and_column_split_state_across_selections() {
    let mut app = test_app();
    app.register_type::<Wrapper>();
    inspect(&mut app, (Subject::default(), Wrapper::default()));
    let subject = Subject::type_path();
    let wrapper = Wrapper::type_path();

    let container = fields_container(&app, wrapper);
    app.world_mut()
        .get_mut::<ColumnSplit>(container)
        .unwrap()
        .fraction = 0.7;
    app.update();
    app.world_mut()
        .resource_mut::<DetailsCollapsed>()
        .toggled
        .insert(subject.into());
    refresh(&mut app);

    let other = app
        .world_mut()
        .spawn((Subject::default(), Wrapper::default()))
        .id();
    select(
        &mut app,
        Some(InspectableEntity::new(other, InspectorSource::Local)),
    );

    let index = app.world().resource::<DetailsIndex>();
    assert!(index.group(subject).is_some());
    assert!(index.widget(subject, "scale").is_none());
    let kept = fields_container(&app, wrapper);
    assert_eq!(kept, container);
    assert_eq!(app.world().get::<ColumnSplit>(kept).unwrap().fraction, 0.7);
}

#[test]
fn shows_an_unreflected_component_under_a_name_based_key() {
    let mut app = test_app();
    inspect(&mut app, (Subject::default(), Marker));
    let id = app.world().component_id::<Marker>().unwrap();

    let key = ComponentKey::of(app.world(), id);
    let expected = if DebugName::ENABLED {
        core::any::type_name::<Marker>().to_string()
    } else {
        format!("{id:?}")
    };
    assert_eq!(&*key.0, expected);
    let index = app.world().resource::<DetailsIndex>();
    assert!(index.group(&key.0).is_some());
    assert!(index.group(Subject::type_path()).is_some());
    assert_eq!(body_children(&mut app).len(), 2);
}

#[test]
fn removes_rows_that_no_longer_exist() {
    let mut app = test_app();
    app.register_type::<Changing>();
    let subject = inspect(
        &mut app,
        Changing {
            items: alloc::vec![1, 2, 3],
            maybe: Some(2.0),
            ..Default::default()
        },
    );

    {
        let mut changing = app.world_mut().get_mut::<Changing>(subject).unwrap();
        changing.items.truncate(1);
        changing.maybe = None;
    }
    refresh(&mut app);

    let changing = app.world().get::<Changing>(subject).unwrap();
    let mut expected: Vec<String> = field_entries(changing, &TypeRegistry::new())
        .into_iter()
        .map(|entry| entry.path)
        .collect();
    expected.sort();
    assert_eq!(tracked_paths(&app), expected);

    let rows: usize = app
        .world_mut()
        .query_filtered::<&Children, With<InspectorDetailsFields>>()
        .iter(app.world())
        .flat_map(|rows| rows.iter())
        .filter(|&&row| !app.world().entity(row).contains::<ColumnSplitHandle>())
        .count();
    assert_eq!(rows, expected.len());
}

#[test]
fn replaces_widgets_when_a_field_changes_kind() {
    let mut app = test_app();
    app.register_type::<Changing>();
    let subject = inspect(&mut app, Changing::default());
    let before = widget_at(&app, "payload.0").unwrap();
    assert!(app.world().get::<NumericValue>(before).is_some());

    app.world_mut()
        .get_mut::<Changing>(subject)
        .unwrap()
        .payload = Payload::Text("hello".to_string());
    refresh(&mut app);

    let after = widget_at(&app, "payload.0").unwrap();
    assert!(app.world().get::<EditableText>(after).is_some());
    assert!(app.world().get::<NumericValue>(after).is_none());
}

#[test]
fn relabels_rows_when_map_keys_change() {
    let mut app = test_app();
    app.register_type::<Changing>();
    let mut lookup = HashMap::default();
    lookup.insert("old".to_string(), 1);
    let subject = inspect(
        &mut app,
        Changing {
            lookup,
            ..Default::default()
        },
    );
    assert!(row_labels(&app).iter().any(|label| label.contains("old")));

    {
        let mut changing = app.world_mut().get_mut::<Changing>(subject).unwrap();
        changing.lookup.clear();
        changing.lookup.insert("new".to_string(), 1);
    }
    refresh(&mut app);

    let labels = row_labels(&app);
    assert!(
        labels.iter().any(|label| label.contains("new")),
        "{labels:?}"
    );
    assert!(
        !labels.iter().any(|label| label.contains("old")),
        "{labels:?}"
    );
}

#[test]
fn tracks_components_sharing_a_short_name() {
    let mut app = test_app();
    app.register_type::<first::Same>();
    app.register_type::<second::Same>();
    inspect(
        &mut app,
        (first::Same { value: 1.0 }, second::Same { value: 2.0 }),
    );

    let values = tracked_paths(&app)
        .into_iter()
        .filter(|path| path == "value")
        .count();
    assert_eq!(values, 2);
}

#[test]
fn clears_a_label_that_becomes_empty() {
    let mut app = test_app();
    app.register_type::<Name>();
    let subject = inspect(&mut app, Name::new("hello"));

    *app.world_mut().get_mut::<Name>(subject).unwrap() = Name::new("");
    refresh(&mut app);

    let caption = widget_at(&app, "").unwrap();
    assert_eq!(app.world().get::<Text>(caption).unwrap().0, "");
}

#[test]
fn separates_components_sharing_a_short_name() {
    let mut app = test_app();
    app.register_type::<first::Duplicate>();
    app.register_type::<second::Duplicate>();
    app.world_mut().spawn((InspectorUi, InspectorDetailsBody));
    let subject = app
        .world_mut()
        .spawn((first::Duplicate::default(), second::Duplicate::default()))
        .id();
    let first = first::Duplicate::type_path();
    let second = second::Duplicate::type_path();
    assert_eq!(
        crate::component_short_name(
            app.world(),
            app.world().component_id::<first::Duplicate>().unwrap()
        ),
        crate::component_short_name(
            app.world(),
            app.world().component_id::<second::Duplicate>().unwrap()
        )
    );

    select(
        &mut app,
        Some(InspectableEntity::new(subject, InspectorSource::Local)),
    );

    let index = app.world().resource::<DetailsIndex>();
    assert!(index.widget(first, "first").is_some());
    assert!(index.widget(second, "second").is_some());
    assert_ne!(index.group(first), index.group(second));
    assert_eq!(body_children(&mut app).len(), 2);

    app.world_mut()
        .resource_mut::<DetailsCollapsed>()
        .toggled
        .insert(first.into());
    refresh(&mut app);

    let index = app.world().resource::<DetailsIndex>();
    assert!(index.widget(first, "first").is_none());
    assert!(index.widget(second, "second").is_some());
    assert_eq!(body_children(&mut app).len(), 2);
}

#[test]
fn keeps_the_empty_state_caption_across_sync_ticks() {
    let mut app = test_app();
    app.world_mut().spawn((InspectorUi, InspectorDetailsBody));
    app.update();
    let caption = body_children(&mut app);
    assert_eq!(caption.len(), 1);

    for _ in 0..3 {
        refresh(&mut app);
    }

    assert_eq!(body_children(&mut app), caption);
    assert_eq!(
        body_message(&mut app).as_deref(),
        Some("No entity selected")
    );
}

#[test]
fn empty_state_is_present_on_the_first_frame_the_panel_is_shown() {
    use bevy_camera::visibility::{Visibility, VisibilitySystems};
    use bevy_ecs::schedule::IntoScheduleConfigs;

    let mut app = test_app();
    app.init_resource::<MessageAtVisibility>().add_systems(
        bevy_app::PostUpdate,
        record_message.in_set(VisibilitySystems::VisibilityPropagate),
    );
    let panel = app.world_mut().spawn(Visibility::Hidden).id();
    app.world_mut()
        .spawn((InspectorUi, InspectorDetailsBody, ChildOf(panel)));
    app.update();
    assert!(body_children(&mut app).is_empty());

    app.world_mut()
        .entity_mut(panel)
        .insert(Visibility::Inherited);
    app.update();

    assert_eq!(
        app.world().resource::<MessageAtVisibility>().0.as_deref(),
        Some("No entity selected")
    );
}

#[test]
fn shows_a_message_when_nothing_is_selected() {
    let mut app = test_app();
    app.world_mut().spawn((InspectorUi, InspectorDetailsBody));

    app.update();

    assert_eq!(
        body_message(&mut app).as_deref(),
        Some("No entity selected")
    );
}

#[test]
fn refreshing_every_kind_emits_no_edits() {
    let (mut app, entity) = kinds_app();
    app.init_resource::<EditCount>();
    app.add_observer(|_: On<FieldEdit>, mut count: ResMut<EditCount>| count.0 += 1);
    app.world_mut().spawn((InspectorUi, InspectorDetailsBody));
    app.world_mut().get_mut::<Kinds>(entity).unwrap().color = Color::hsla(0.0, 0.5, 0.5, 1.0);

    app.world_mut().resource_mut::<InspectorSelection>().0 =
        Some(InspectableEntity::new(entity, InspectorSource::Local));
    for _ in 0..3 {
        app.world_mut()
            .resource_mut::<DetailsPanelSync>()
            .set_dirty();
        app.update();
    }
    app.world_mut().get_mut::<Kinds>(entity).unwrap().color = Color::hsla(120.0, 0.5, 0.5, 1.0);
    for _ in 0..3 {
        app.world_mut()
            .resource_mut::<DetailsPanelSync>()
            .set_dirty();
        app.update();
    }

    assert_eq!(app.world().resource::<EditCount>().0, 0);
    let component = Kinds::type_path();
    let index = app.world().resource::<DetailsIndex>();
    let color = &index.fields[&(component.into(), "color".to_string())];
    assert!(app
        .world()
        .get::<InspectorField>(color.text.unwrap())
        .is_some());
}

#[test]
fn shared_component_groups_keep_their_widgets_across_a_selection_change() {
    let (mut app, first) = widget_app();
    app.register_type::<Wrapper>();
    app.world_mut().entity_mut(first).insert(Wrapper::default());
    settle(&mut app);
    let subject = Subject::type_path();
    let index = app.world().resource::<DetailsIndex>();
    let group = index.group(subject).unwrap();
    let input = index.widget(subject, "scale").unwrap();
    assert!(index.group(Wrapper::type_path()).is_some());

    let second = app
        .world_mut()
        .spawn(Subject {
            scale: 3.0,
            ..Default::default()
        })
        .id();
    select(
        &mut app,
        Some(InspectableEntity::new(second, InspectorSource::Local)),
    );

    let index = app.world().resource::<DetailsIndex>();
    assert_eq!(index.group(subject), Some(group));
    assert_eq!(index.widget(subject, "scale"), Some(input));
    assert!(index.group(Wrapper::type_path()).is_none());
    assert_eq!(
        app.world().get::<NumericValue>(input),
        Some(&NumericValue::F32(3.0))
    );
}

#[test]
fn groups_are_spawned_before_the_ui_stack_is_built() {
    use bevy_ecs::schedule::IntoScheduleConfigs;

    let mut app = test_app();
    app.init_resource::<GroupsAtStack>().add_systems(
        bevy_app::PostUpdate,
        record_groups.in_set(bevy_ui::UiSystems::Stack),
    );
    inspect(&mut app, Subject::default());
    assert!(app.world().resource::<GroupsAtStack>().0);
}

#[test]
fn a_kept_widget_edits_the_new_selection() {
    let (mut app, first) = widget_app();
    let input = field_widget(&mut app, "scale");
    let second = app.world_mut().spawn(Subject::default()).id();
    select(
        &mut app,
        Some(InspectableEntity::new(second, InspectorSource::Local)),
    );
    assert_eq!(field_widget(&mut app, "scale"), input);

    app.world_mut().trigger(ValueChange {
        source: input,
        value: 5.0_f32,
        is_final: true,
    });
    app.update();
    assert_eq!(app.world().get::<Subject>(second).unwrap().scale, 5.0);
    assert_eq!(app.world().get::<Subject>(first).unwrap().scale, 1.0);
}

#[test]
fn a_variant_that_differs_from_the_previous_selection_respawns_the_group() {
    let (mut app, _) = widget_app();
    let group = app
        .world()
        .resource::<DetailsIndex>()
        .group(Subject::type_path());

    select_subject(
        &mut app,
        Subject {
            mode: Mode::Running,
            ..Default::default()
        },
    );

    let index = app.world().resource::<DetailsIndex>();
    assert_ne!(index.group(Subject::type_path()), group);
    let mode = index
        .fields
        .get(&(ComponentKey::from(Subject::type_path()), "mode".to_string()))
        .unwrap();
    assert_eq!(mode.value, variant(&["Idle", "Running"], 1));
}

#[test]
fn a_collection_longer_than_in_the_previous_selection_respawns_the_group() {
    let (mut app, _) = widget_app();
    let group = app
        .world()
        .resource::<DetailsIndex>()
        .group(Subject::type_path());
    assert!(widget_at(&app, "values[1]").is_none());

    select_subject(
        &mut app,
        Subject {
            values: alloc::vec![4, 5],
            ..Default::default()
        },
    );

    let index = app.world().resource::<DetailsIndex>();
    assert_ne!(index.group(Subject::type_path()), group);
    let item = widget_at(&app, "values[1]").unwrap();
    assert_eq!(
        app.world().get::<NumericValue>(item),
        Some(&NumericValue::I64(5))
    );
}

#[test]
fn a_draft_in_a_focused_input_is_discarded_when_the_selection_changes() {
    let (mut app, first) = kinds_app();
    app.add_plugins(bevy_input_focus::InputFocusPlugin);
    let input = hex_input(&mut app, first);
    let before = app.world().get::<Kinds>(first).unwrap().color;
    app.world_mut()
        .resource_mut::<InputFocus>()
        .set(input, bevy_input_focus::FocusCause::Navigated);
    type_text(&mut app, input, "#00FF00");

    let second = app.world_mut().spawn(Kinds::default()).id();
    select(
        &mut app,
        Some(InspectableEntity::new(second, InspectorSource::Local)),
    );
    app.update();

    assert_eq!(app.world().resource::<InputFocus>().get(), None);
    assert!(app.world().get_entity(input).is_err());
    assert_eq!(app.world().get::<Kinds>(first).unwrap().color, before);
    assert_eq!(app.world().get::<Kinds>(second).unwrap().color, before);
    let shown = color_text_input(&mut app);
    assert_eq!(input_text(&app, shown), color_to_hex(before));
}

#[test]
fn a_color_with_the_same_hex_code_as_the_previous_selection_is_shown_exactly() {
    let (mut app, first) = kinds_app();
    app.world_mut().get_mut::<Kinds>(first).unwrap().linear = LinearRgba::rgb(2.0, 0.0, 0.0);
    app.world_mut().spawn((InspectorUi, InspectorDetailsBody));
    select(
        &mut app,
        Some(InspectableEntity::new(first, InspectorSource::Local)),
    );
    let picker = app
        .world()
        .resource::<DetailsIndex>()
        .widget(Kinds::type_path(), "linear")
        .unwrap();

    let second = app
        .world_mut()
        .spawn(Kinds {
            linear: LinearRgba::RED,
            ..Default::default()
        })
        .id();
    select(
        &mut app,
        Some(InspectableEntity::new(second, InspectorSource::Local)),
    );

    assert_eq!(
        app.world()
            .resource::<DetailsIndex>()
            .widget(Kinds::type_path(), "linear"),
        Some(picker)
    );
    assert_eq!(
        app.world()
            .get::<ColorInputValue>(picker)
            .map(|input| input.0),
        Some(Color::LinearRgba(LinearRgba::RED))
    );
}

#[test]
fn shows_a_message_for_an_entity_without_components() {
    let mut app = test_app();
    app.world_mut().spawn((InspectorUi, InspectorDetailsBody));
    let subject = app.world_mut().spawn_empty().id();

    select(
        &mut app,
        Some(InspectableEntity::new(subject, InspectorSource::Local)),
    );

    assert_eq!(
        body_message(&mut app).as_deref(),
        Some("The selected entity has no components")
    );
}

#[test]
fn shows_a_message_when_the_selection_is_despawned() {
    let mut app = test_app();
    app.world_mut().spawn((InspectorUi, InspectorDetailsBody));
    let subject = app.world_mut().spawn(Subject::default()).id();
    select(
        &mut app,
        Some(InspectableEntity::new(subject, InspectorSource::Local)),
    );
    assert_eq!(body_message(&mut app), None);

    app.world_mut().entity_mut(subject).despawn();
    refresh(&mut app);

    assert_eq!(
        body_message(&mut app).as_deref(),
        Some("The selected entity no longer exists")
    );
    assert!(app.world().resource::<DetailsIndex>().is_empty());
}

#[test]
fn keeps_unchanged_groups_when_components_change() {
    let mut app = test_app();
    app.register_type::<Holder>();
    app.world_mut().spawn((InspectorUi, InspectorDetailsBody));
    let subject = app.world_mut().spawn(Subject::default()).id();
    let subject_id = Subject::type_path();
    select(
        &mut app,
        Some(InspectableEntity::new(subject, InspectorSource::Local)),
    );
    let group = app
        .world()
        .resource::<DetailsIndex>()
        .group(subject_id)
        .unwrap();
    let scale = app
        .world()
        .resource::<DetailsIndex>()
        .widget(subject_id, "scale")
        .unwrap();

    app.world_mut()
        .entity_mut(subject)
        .insert(Holder(Arc::new(StrongHandle)));
    refresh(&mut app);

    let holder_id = Holder::type_path();
    let index = app.world().resource::<DetailsIndex>();
    let holder = index.group(holder_id).unwrap();
    assert_eq!(index.group(subject_id), Some(group));
    assert_eq!(index.widget(subject_id, "scale"), Some(scale));
    assert_eq!(body_children(&mut app), alloc::vec![holder, group]);

    app.world_mut().entity_mut(subject).remove::<Holder>();
    refresh(&mut app);

    let index = app.world().resource::<DetailsIndex>();
    assert_eq!(index.group(holder_id), None);
    assert_eq!(index.group(subject_id), Some(group));
    assert!(app.world().get_entity(holder).is_err());
    assert_eq!(body_children(&mut app), alloc::vec![group]);
}

#[test]
fn does_not_rewrite_a_nan_field_on_each_sync() {
    let mut app = test_app();
    inspect(
        &mut app,
        Subject {
            scale: f32::NAN,
            ..Default::default()
        },
    );
    let input = widget_at(&app, "scale").unwrap();
    let changed = |app: &App| {
        app.world()
            .entity(input)
            .get_ref::<NumericValue>()
            .unwrap()
            .last_changed()
    };
    let before = changed(&app);

    settle(&mut app);

    assert_eq!(changed(&app), before);
}

#[test]
fn refreshes_external_changes_on_the_timer() {
    let (mut app, subject) = widget_app();
    app.insert_resource(bevy_time::TimeUpdateStrategy::ManualDuration(
        Duration::from_millis(100),
    ));
    let input = field_widget(&mut app, "scale");
    app.update();

    app.world_mut().get_mut::<Subject>(subject).unwrap().scale = 4.0;
    for _ in 0..4 {
        app.update();
    }

    assert_eq!(
        app.world().get::<NumericValue>(input),
        Some(&NumericValue::F32(4.0))
    );
}

#[test]
fn respawns_the_group_when_an_edited_map_key_is_removed() {
    let mut app = test_app();
    app.register_type::<Lookup>();
    let entity = inspect(
        &mut app,
        Lookup(
            [("a".to_string(), 1.0), ("b".to_string(), 2.0)]
                .into_iter()
                .collect(),
        ),
    );
    let component = Lookup::type_path();
    let group = app
        .world()
        .resource::<DetailsIndex>()
        .group(component)
        .unwrap();
    let input = field_widget(&mut app, "0[\"a\"]");

    app.world_mut()
        .get_mut::<Lookup>(entity)
        .unwrap()
        .0
        .remove("a");
    trigger_change(&mut app, input, 3.0_f32);

    let lookup = &app.world().get::<Lookup>(entity).unwrap().0;
    assert_eq!(lookup.get("a"), None);
    assert_eq!(lookup.get("b"), Some(&2.0));
    let index = app.world().resource::<DetailsIndex>();
    assert_ne!(index.group(component), Some(group));
    assert_eq!(index.widget(component, "0[\"a\"]"), None);
    assert!(index.widget(component, "0[\"b\"]").is_some());
}
