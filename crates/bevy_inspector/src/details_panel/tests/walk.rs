use super::*;
use crate::details_panel::walk::ASSET_HANDLE_PREFIX;

#[test]
fn walks_a_struct_into_field_entries() {
    let subject = Subject {
        enabled: true,
        scale: 2.0,
        name: "subject".to_string(),
        nested: Nested { depth: 3 },
        values: alloc::vec![7, 8],
        mode: Mode::Idle,
    };

    let entries = field_entries(&subject, &TypeRegistry::new());

    assert_eq!(
        kinds(&entries),
        alloc::vec![
            ("enabled".to_string(), FieldKind::Bool),
            ("scale".to_string(), FieldKind::Number),
            ("name".to_string(), FieldKind::Text),
            ("nested".to_string(), FieldKind::Label),
            ("nested.depth".to_string(), FieldKind::Number),
            ("values".to_string(), FieldKind::Label),
            ("values[0]".to_string(), FieldKind::Number),
            ("values[1]".to_string(), FieldKind::Number),
            ("mode".to_string(), FieldKind::Variant),
        ]
    );
    assert_eq!(entries[4].depth, 1);
    assert_eq!(entries[5].value, FieldValue::Label("2 items".to_string()));
}

#[test]
fn collapses_newtype_tuple_structs() {
    let entries = field_entries(&Wrapper(Nested { depth: 3 }), &TypeRegistry::new());

    assert_eq!(
        kinds(&entries),
        alloc::vec![("0.depth".to_string(), FieldKind::Number)]
    );
    assert_eq!(entries[0].depth, 0);
}

#[test]
fn renders_a_name_as_a_single_caption() {
    let entries = field_entries(&Name::new("Left Cube"), &TypeRegistry::new());

    assert_eq!(
        entries.len(),
        1,
        "expected one caption row, got {entries:?}"
    );
    assert_eq!(entries[0].value, FieldValue::Label("Left Cube".to_string()));
}

#[test]
fn asset_handles_are_read_only_labels() {
    let handle = bevy_asset::Handle::<bevy_image::Image>::default();
    assert!(bevy_asset::Handle::<bevy_image::Image>::type_path().starts_with(ASSET_HANDLE_PREFIX));
    let mut registry = TypeRegistry::new();
    registry.register::<Mode>();
    registry.register_type_data::<Mode, ReflectDefault>();

    let entries = field_entries(&Pointer(handle.clone()), &registry);
    assert_eq!(entries.len(), 1, "{entries:?}");
    assert_eq!(
        entries[0].value,
        FieldValue::Label("Handle<Image>".to_string())
    );

    let entries = field_entries(
        &Skin {
            texture: Texture(handle.clone()),
            direct: handle,
            mode: Mode::Idle,
        },
        &registry,
    );
    let label = FieldValue::Label("Handle<Image>".to_string());
    assert_eq!(entry(&entries, "texture.0").value, label);
    assert_eq!(entry(&entries, "direct").value, label);
    assert!(
        entries
            .iter()
            .filter(|entry| entry.path.starts_with("texture") || entry.path.starts_with("direct"))
            .all(|entry| entry.value == label),
        "{entries:?}"
    );
    assert!(matches!(
        entry(&entries, "mode").value,
        FieldValue::Variant { .. }
    ));
}

#[test]
fn shortens_opaque_type_paths() {
    let entries = field_entries(&Holder(Arc::new(StrongHandle)), &TypeRegistry::new());

    assert_eq!(entries.len(), 1);
    assert_eq!(
        entries[0].value,
        FieldValue::Label("Arc<StrongHandle>".to_string())
    );
}

#[test]
fn distinguishes_variants_of_an_enum_component() {
    assert_ne!(
        field_entries(&Stage::A(1.0), &TypeRegistry::new()),
        field_entries(&Stage::B(1.0), &TypeRegistry::new())
    );
}

#[test]
fn summarizes_a_list_component() {
    let entries = field_entries(&Items((0..20).collect()), &TypeRegistry::new());

    assert!(
        entries
            .iter()
            .any(|entry| matches!(&entry.value, FieldValue::Label(text) if text.contains("20"))),
        "{entries:?}"
    );
}

#[test]
fn does_not_clamp_large_integers() {
    let entries = field_entries(
        &Large {
            unsigned: u64::MAX,
            size: usize::MAX,
        },
        &TypeRegistry::new(),
    );

    assert_eq!(displayed(&entries[0].value), u64::MAX.to_string());
    assert_eq!(displayed(&entries[1].value), usize::MAX.to_string());
}

#[test]
fn field_paths_resolve_to_their_values() {
    let everything = Everything {
        nested: Nested { depth: 1 },
        wrapper: Wrapper(Nested { depth: 2 }),
        pair: Pair(3, 4),
        tuple: (true, 5.0),
        list: alloc::vec![6],
        array: [8, 9],
        option: Some(11.0),
        shape: Shape::Circle { radius: 12.0 },
    };

    let mismatches = unresolved_paths(&everything);
    assert!(mismatches.is_empty(), "{mismatches:#?}");
}

#[test]
fn map_and_set_paths_resolve_to_their_values() {
    let keyed = Keyed {
        map: [
            ("key".to_string(), 7),
            ("a \"quoted\" \\ key".to_string(), 8),
        ]
        .into_iter()
        .collect(),
        numbers: [(-3, 1.0), (4, 2.0)].into_iter().collect(),
        set: [10, 11].into_iter().collect(),
    };

    let paths = field_paths(&keyed);
    for expected in [
        "map[\"key\"]",
        "map[\"a \\\"quoted\\\" \\\\ key\"]",
        "numbers[\"-3\"]",
        "numbers[\"4\"]",
        "set[\"10\"]",
        "set[\"11\"]",
    ] {
        assert!(paths.iter().any(|path| path == expected), "{paths:?}");
    }
    let mismatches = unresolved_paths(&keyed);
    assert!(mismatches.is_empty(), "{mismatches:#?}");
}

#[test]
fn gives_unexpressible_keys_distinct_unresolvable_paths() {
    #[derive(Reflect, Debug)]
    struct Flags {
        flags: HashMap<bool, u32>,
    }

    let flags = Flags {
        flags: [(false, 1), (true, 2)].into_iter().collect(),
    };
    let entries: Vec<FieldEntry> = field_entries(&flags, &TypeRegistry::new())
        .into_iter()
        .filter(|entry| entry.path.starts_with("flags["))
        .collect();

    assert_eq!(entries.len(), 2, "{entries:?}");
    assert_ne!(entries[0].path, entries[1].path);
    for entry in &entries {
        assert!(
            flags.reflect_path(entry.path.as_str()).is_err(),
            "{entry:?}"
        );
        assert_eq!(entry.value.kind(), FieldKind::Label, "{entry:?}");
    }
}

#[test]
fn classifies_every_field_kind() {
    let mut app = test_app();
    app.register_type::<Kinds>();
    let registry = app.world().resource::<AppTypeRegistry>().read();
    let kinds = Kinds {
        long: u64::MAX,
        huge: 5,
        letter: 'x',
        fixed: "fixed",
        shape: Shape::Circle { radius: 1.0 },
        tags: alloc::vec![1, 2],
        map: [("a".to_string(), 1)].into_iter().collect(),
        set: [4].into_iter().collect(),
        ..Default::default()
    };

    let entries = field_entries(&kinds, &registry);

    assert_eq!(
        entry(&entries, "byte").value,
        FieldValue::Number(NumericValue::I32(0))
    );
    assert_eq!(entry(&entries, "huge").value, int(5));
    assert_eq!(
        entry(&entries, "long").value,
        FieldValue::Label(u64::MAX.to_string())
    );
    assert_eq!(
        entry(&entries, "letter").value,
        FieldValue::Text("x".to_string())
    );
    assert_eq!(entry(&entries, "cow").value.kind(), FieldKind::Text);
    assert_eq!(
        entry(&entries, "fixed").value,
        FieldValue::Label("fixed".to_string())
    );
    assert_eq!(entry(&entries, "linear").value.kind(), FieldKind::Color);
    assert_eq!(
        entry(&entries, "shape").value,
        variant(&["Empty", "Circle"], 1)
    );
    assert_eq!(
        entry(&entries, "shape.radius").value.kind(),
        FieldKind::Number
    );
    assert_eq!(
        entry(&entries, "maybe").value,
        variant(&["None", "Some"], 0)
    );
    assert_eq!(
        entry(&entries, "outer.inner.depth").value.kind(),
        FieldKind::Number
    );
    assert_eq!(entry(&entries, "outer.inner.depth").depth, 2);
    assert_eq!(entry(&entries, "array[2]").value.kind(), FieldKind::Number);
    assert_eq!(
        entry(&entries, "queue").value,
        FieldValue::Label("0 items".to_string())
    );
    assert_eq!(entry(&entries, "map[\"a\"]").value, int(1));
    assert_eq!(
        entry(&entries, "set[\"4\"]").value,
        FieldValue::Label("4".to_string())
    );

    let linked = field_entries(
        &Linked {
            target: Entity::PLACEHOLDER,
        },
        &registry,
    );
    assert_eq!(linked[0].value.kind(), FieldKind::Label);
}

#[test]
fn does_not_offer_variants_without_defaults() {
    let registry = TypeRegistry::new();
    let entries = field_entries(
        &Kinds {
            shape: Shape::Custom(Locked(3)),
            ..Default::default()
        },
        &registry,
    );

    assert_eq!(
        entry(&entries, "shape").value,
        variant(&["Empty", "Circle", "Custom"], 2)
    );

    let entries = field_entries(&Kinds::default(), &registry);
    assert_eq!(
        entry(&entries, "shape").value,
        variant(&["Empty", "Circle"], 0)
    );
}
