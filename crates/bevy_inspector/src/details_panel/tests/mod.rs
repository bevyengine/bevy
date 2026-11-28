use super::*;
use crate::{column_split::ColumnSplitHandle, entity_tree::InspectorTreeView, InspectorPlugin};
use alloc::collections::VecDeque;
use bevy_app::{App, TaskPoolPlugin};
use bevy_asset::{AssetApp, AssetPlugin};
use bevy_ecs::change_detection::DetectChanges;
use bevy_ecs::{component::Component, query::With};
use bevy_platform::sync::Arc;
use bevy_reflect::TypePath;
use core::time::Duration;

use alloc::{
    borrow::Cow,
    format,
    string::{String, ToString},
    vec::Vec,
};

use bevy_annotations::{DisplayName, ReadOnly, Step, Unit, ValueRange};
use bevy_color::{Color, LinearRgba, Srgba};
use bevy_ecs::{
    entity::Entity,
    hierarchy::{ChildOf, Children},
    name::Name,
    observer::On,
    reflect::{AppTypeRegistry, ReflectComponent},
    resource::Resource,
    system::{Query, ResMut},
    world::World,
};
use bevy_feathers::controls::{
    ButtonVariant, ColorInputValue, FeathersColorInput, FeathersNumberInput, FeathersSelect,
    HardLimit, NumberInputSnapToStep, NumberInputStep, OptionIndex,
};
use bevy_input_focus::{FocusLost, InputFocus};
use bevy_platform::collections::{HashMap, HashSet};
use bevy_reflect::{prelude::ReflectDefault, GetPath, Reflect, TypeRegistry};
use bevy_text::{EditableText, LineBreak, TextEdit, TextLayout};
use bevy_ui::{px, widget::Text, Checked, Display, Node, Overflow};
use bevy_ui_widgets::{Activate, NumericRange, NumericValue, ValueChange};
use bevy_utils::prelude::DebugName;

use crate::details_panel::{
    rows::{descendant_with, replace_text},
    sync::find_body,
    walk::scalar_value,
};
use crate::{
    column_split::{ColumnSplit, ColumnSplitLeading},
    entity_tree::InspectorUi,
    InspectableEntity, InspectorSelection, InspectorSource, InspectorSources,
};
// #[cfg(feature = "remote")]
// use bevy_remote::http::DEFAULT_PORT;

mod annotations;
mod edit;
mod entity_section;
mod rows;
mod sync;
mod ticks;
mod walk;

#[derive(Reflect, Debug, Default)]
struct Nested {
    depth: u32,
}

#[derive(Component, Reflect, Debug, Default)]
#[reflect(Component, Default)]
struct Wrapper(Nested);

#[derive(Component, Reflect, Debug, Default)]
#[reflect(Component, Default)]
struct Tagged {
    label: Name,
}

#[derive(Reflect, Debug, Default)]
struct StrongHandle;

#[derive(Component, Reflect, Debug, Default)]
#[reflect(Component, Default)]
struct Holder(Arc<StrongHandle>);

#[derive(Reflect, Debug, Default, Clone, Copy, PartialEq)]
enum Mode {
    #[default]
    Idle,
    Running,
}

#[derive(Component, Reflect, Debug, Default)]
#[reflect(Component, Default)]
struct Subject {
    enabled: bool,
    scale: f32,
    name: String,
    nested: Nested,
    values: Vec<u32>,
    mode: Mode,
}

#[derive(Reflect, Debug, Default, PartialEq)]
struct Locked(u8);

#[derive(Reflect, Debug, Default, PartialEq)]
enum Shape {
    #[default]
    Empty,
    Circle {
        radius: f32,
    },
    Custom(Locked),
}

#[derive(Reflect, Debug, Default, PartialEq)]
struct Point(f32, f32);

#[derive(Component, Reflect, Debug)]
#[reflect(Component)]
struct Pointer(bevy_asset::Handle<bevy_image::Image>);

#[derive(Reflect, Debug)]
struct Texture(bevy_asset::Handle<bevy_image::Image>);

#[derive(Component, Reflect, Debug)]
#[reflect(Component)]
struct Skin {
    texture: Texture,
    direct: bevy_asset::Handle<bevy_image::Image>,
    mode: Mode,
}

#[derive(Reflect, Debug, Default)]
struct Outer {
    inner: Nested,
}

#[derive(Component, Reflect, Debug, Default)]
#[reflect(Component, Default)]
struct Kinds {
    byte: u8,
    short: u16,
    word: u32,
    long: u64,
    huge: u128,
    size: usize,
    tiny: i8,
    small: i16,
    int: i32,
    big: i64,
    giant: i128,
    signed_size: isize,
    single: f32,
    double: f64,
    letter: char,
    text: String,
    cow: Cow<'static, str>,
    fixed: &'static str,
    color: Color,
    srgba: Srgba,
    linear: LinearRgba,
    shape: Shape,
    maybe: Option<u32>,
    pair: (u32, f32),
    point: Point,
    outer: Outer,
    tags: Vec<u32>,
    array: [f32; 3],
    queue: VecDeque<u32>,
    map: HashMap<String, u32>,
    set: HashSet<u32>,
}

#[derive(Reflect, Debug)]
struct Linked {
    target: Entity,
}

#[derive(Resource, Debug, Default)]
struct EditCount(usize);

fn edit(app: &mut App, entity: Entity, path: &str, value: FieldValue) {
    edit_component::<Subject>(app, entity, path, value);
}

fn edit_component<C: TypePath>(app: &mut App, entity: Entity, path: &str, value: FieldValue) {
    app.world_mut().trigger(FieldEdit {
        entity,
        source: InspectorSource::Local,
        component: C::type_path().to_string(),
        path: path.to_string(),
        value,
    });
    app.world_mut().flush();
}

fn edit_kinds(app: &mut App, entity: Entity, path: &str, value: FieldValue) {
    edit_component::<Kinds>(app, entity, path, value);
}

fn kinds_app() -> (App, Entity) {
    let mut app = test_app();
    app.register_type::<Kinds>();
    let entity = app.world_mut().spawn(Kinds::default()).id();
    (app, entity)
}

fn int(value: i64) -> FieldValue {
    FieldValue::Number(NumericValue::I64(value))
}

fn variant(names: &[&str], selected: usize) -> FieldValue {
    FieldValue::Variant {
        variants: names.iter().map(ToString::to_string).collect(),
        selected,
    }
}

fn entry<'a>(entries: &'a [FieldEntry], path: &str) -> &'a FieldEntry {
    entries
        .iter()
        .find(|entry| entry.path == path)
        .unwrap_or_else(|| panic!("no entry at `{path}` in {entries:?}"))
}

mod first {
    use bevy_ecs::{component::Component, reflect::ReflectComponent};
    use bevy_reflect::{prelude::ReflectDefault, Reflect};

    #[derive(Component, Reflect, Debug, Default)]
    #[reflect(Component, Default)]
    pub struct Duplicate {
        pub first: f32,
    }

    #[derive(Component, Reflect, Debug, Default)]
    #[reflect(Component)]
    pub struct Same {
        pub value: f32,
    }
}

mod second {
    use bevy_ecs::{component::Component, reflect::ReflectComponent};
    use bevy_reflect::{prelude::ReflectDefault, Reflect};

    #[derive(Component, Reflect, Debug, Default)]
    #[reflect(Component, Default)]
    pub struct Duplicate {
        pub second: bool,
    }

    #[derive(Component, Reflect, Debug, Default)]
    #[reflect(Component)]
    pub struct Same {
        pub value: f32,
    }
}

#[derive(Component)]
struct Marker;

fn test_app() -> App {
    let mut app = App::new();
    app.add_plugins((
        TaskPoolPlugin::default(),
        bevy_time::TimePlugin,
        AssetPlugin::default(),
        bevy_scene::ScenePlugin,
        InspectorPlugin,
    ));
    let mut units = bevy_feathers::controls::UnitsRegistry::default();
    units.insert(&bevy_feathers::controls::Dimensionless);
    app.insert_resource(units);
    app.insert_resource(bevy_feathers::theme::UiTheme(
        bevy_feathers::dark_theme::create_dark_theme(),
    ));
    app.init_resource::<InputFocus>();
    app.init_asset::<bevy_image::Image>();
    app.init_asset::<bevy_text::Font>();
    app.register_type::<Subject>();
    app
}

fn kinds(entries: &[FieldEntry]) -> Vec<(String, FieldKind)> {
    entries
        .iter()
        .map(|entry| (entry.path.clone(), entry.value.kind()))
        .collect()
}

fn fields_container(app: &App, component: &str) -> Entity {
    let world = app.world();
    let group = world.resource::<DetailsIndex>().group(component).unwrap();
    descendant_with::<InspectorDetailsFields>(world, group).unwrap()
}

#[derive(Reflect, Debug, Clone, PartialEq)]
enum Payload {
    Number(f32),
    Text(String),
}

impl Default for Payload {
    fn default() -> Self {
        Payload::Number(1.0)
    }
}

#[derive(Component, Reflect, Debug, Default)]
#[reflect(Component, Default)]
struct Changing {
    items: Vec<u32>,
    maybe: Option<f32>,
    payload: Payload,
    lookup: HashMap<String, u32>,
}

#[derive(Component, Reflect, Debug)]
#[reflect(Component)]
enum Stage {
    A(f32),
    B(f32),
}

#[derive(Component, Reflect, Debug, Default)]
#[reflect(Component, Default)]
struct Items(Vec<u32>);

#[derive(Reflect, Debug, Default)]
struct Large {
    unsigned: u64,
    size: usize,
}

#[derive(Reflect, Debug, Default)]
struct Pair(u8, u8);

#[derive(Reflect, Debug)]
struct Everything {
    nested: Nested,
    wrapper: Wrapper,
    pair: Pair,
    tuple: (bool, f32),
    list: Vec<u32>,
    array: [u8; 2],
    option: Option<f32>,
    shape: Shape,
}

#[derive(Reflect, Debug)]
struct Keyed {
    map: HashMap<String, u32>,
    numbers: HashMap<i32, f32>,
    set: HashSet<u32>,
}

fn inspect<B: bevy_ecs::bundle::Bundle>(app: &mut App, bundle: B) -> Entity {
    app.world_mut().spawn((InspectorUi, InspectorDetailsBody));
    let entity = app.world_mut().spawn(bundle).id();
    app.world_mut().resource_mut::<InspectorSelection>().0 =
        Some(InspectableEntity::new(entity, InspectorSource::Local));
    app.update();
    entity
}

fn select(app: &mut App, entity: Option<InspectableEntity>) {
    app.world_mut().resource_mut::<InspectorSelection>().0 = entity;
    app.update();
}

fn refresh(app: &mut App) {
    app.world_mut()
        .resource_mut::<DetailsPanelSync>()
        .set_dirty();
    app.update();
}

fn tracked_paths(app: &App) -> Vec<String> {
    let mut paths: Vec<String> = app
        .world()
        .resource::<DetailsIndex>()
        .fields
        .keys()
        .map(|(_, path)| path.clone())
        .collect();
    paths.sort();
    paths
}

fn widget_at(app: &App, path: &str) -> Option<Entity> {
    app.world()
        .resource::<DetailsIndex>()
        .fields
        .iter()
        .find(|((_, key), _)| key == path)
        .map(|(_, widget)| widget.entity)
}

/// The label text of every tracked row, the first text in the row.
fn row_labels(app: &App) -> Vec<String> {
    let world = app.world();
    row_entities(app)
        .into_iter()
        .filter_map(|row| {
            let label = descendant_with::<Text>(world, row)?;
            Some(world.get::<Text>(label)?.0.clone())
        })
        .collect()
}

fn row_entities(app: &App) -> Vec<Entity> {
    let world = app.world();
    world
        .resource::<DetailsIndex>()
        .fields
        .values()
        .filter_map(|widget| {
            let mut row = widget.entity;
            loop {
                let parent = world.get::<ChildOf>(row)?.parent();
                let container = world.entity(parent);
                if container.contains::<InspectorDetailsFields>()
                    || container.contains::<InspectorFieldGroupBody>()
                {
                    return Some(row);
                }
                row = parent;
            }
        })
        .collect()
}

fn displayed(value: &FieldValue) -> String {
    match value {
        FieldValue::Number(number) => number.to_string(),
        FieldValue::Text(text) | FieldValue::Label(text) => text.clone(),
        other => panic!("unexpected value {other:?}"),
    }
}

fn unresolved_paths<T: Reflect>(value: &T) -> Vec<String> {
    field_entries(value, &TypeRegistry::new())
        .into_iter()
        .filter_map(|entry| match value.reflect_path(entry.path.as_str()) {
            Err(error) => Some(format!("{:?}: {error}", entry.path)),
            Ok(resolved) => match scalar_value(resolved) {
                Some(scalar) if read_only(scalar.clone()) != read_only(entry.value.clone()) => {
                    Some(format!("{:?}: {scalar:?} != {:?}", entry.path, entry.value))
                }
                _ => None,
            },
        })
        .collect()
}

fn field_paths<T: Reflect>(value: &T) -> Vec<String> {
    field_entries(value, &TypeRegistry::new())
        .into_iter()
        .map(|entry| entry.path)
        .collect()
}

fn body_children(app: &mut App) -> Vec<Entity> {
    let body = find_body(app.world_mut()).unwrap();
    app.world()
        .get::<Children>(body)
        .map(|children| children.to_vec())
        .unwrap_or_default()
}

/// The children of the panel body other than the entity section.
fn component_groups(app: &mut App) -> Vec<Entity> {
    let section = app
        .world()
        .resource::<DetailsIndex>()
        .group(crate::details_panel::entity_section::SECTION_KEY);
    body_children(app)
        .into_iter()
        .filter(|child| Some(*child) != section)
        .collect()
}

fn body_message(app: &mut App) -> Option<String> {
    let children = component_groups(app);
    let [caption] = children.as_slice() else {
        return None;
    };
    app.world().get::<Text>(*caption).map(|text| text.0.clone())
}

#[derive(Resource, Default)]
struct MessageAtVisibility(Option<String>);

fn record_message(
    bodies: Query<&Children, With<InspectorDetailsBody>>,
    texts: Query<&Text>,
    mut seen: ResMut<MessageAtVisibility>,
) {
    seen.0 = bodies
        .iter()
        .flat_map(|children| children.iter())
        .find_map(|child| texts.get(*child).ok())
        .map(|text| text.0.clone());
}

fn widget_app() -> (App, Entity) {
    let mut app = test_app();
    app.add_plugins(bevy_text::TextPlugin);
    app.world_mut().spawn((InspectorUi, InspectorDetailsBody));
    let subject = app
        .world_mut()
        .spawn(Subject {
            scale: 1.0,
            name: "start".to_string(),
            ..Default::default()
        })
        .id();
    app.insert_resource(InspectorSources::local());
    app.world_mut().resource_mut::<InspectorSelection>().0 =
        Some(InspectableEntity::new(subject, InspectorSource::Local));
    app.update();
    (app, subject)
}

fn field_widget(app: &mut App, path: &str) -> Entity {
    let mut query = app.world_mut().query::<(Entity, &InspectorField)>();
    query
        .iter(app.world())
        .find(|(_, field)| field.path == path)
        .map(|(entity, _)| entity)
        .unwrap_or_else(|| panic!("no widget for `{path}`"))
}

fn settle(app: &mut App) {
    for _ in 0..3 {
        app.world_mut()
            .resource_mut::<DetailsPanelSync>()
            .set_dirty();
        app.update();
    }
}

#[derive(Resource, Default)]
struct GroupsAtStack(bool);

fn record_groups(index: bevy_ecs::system::Res<DetailsIndex>, mut seen: ResMut<GroupsAtStack>) {
    seen.0 = index.group(Subject::type_path()).is_some();
}

fn select_subject(app: &mut App, subject: Subject) -> Entity {
    let entity = app
        .world_mut()
        .spawn(Subject {
            scale: 1.0,
            name: "start".to_string(),
            ..subject
        })
        .id();
    select(
        app,
        Some(InspectableEntity::new(entity, InspectorSource::Local)),
    );
    entity
}

fn color_text_input(app: &mut App) -> Entity {
    let mut inputs = app
        .world_mut()
        .query_filtered::<(Entity, &InspectorField), With<EditableText>>();
    inputs
        .iter(app.world())
        .find(|(_, field)| field.path == "color")
        .map(|(input, _)| input)
        .unwrap()
}

fn option_rows(app: &App, select: Entity) -> Vec<(Entity, usize, bool)> {
    let mut rows = Vec::new();
    let mut stack = alloc::vec![select];
    while let Some(entity) = stack.pop() {
        let entity_ref = app.world().entity(entity);
        if let Some(index) = entity_ref.get::<OptionIndex>() {
            rows.push((entity, index.0, entity_ref.contains::<bevy_ui::Selected>()));
        }
        if let Some(children) = entity_ref.get::<Children>() {
            stack.extend(children.iter().copied());
        }
    }
    rows.sort_by_key(|(_, index, _)| *index);
    rows
}

#[derive(Reflect, Debug, Default, Clone, Copy, PartialEq)]
enum Weather {
    #[default]
    Clear,
    Cloudy,
    Rain,
    Snow,
}

#[derive(Component, Reflect, Debug, Default)]
#[reflect(Component, Default)]
struct Forecast {
    weather: Weather,
}

#[derive(Component, Reflect, Debug, Default)]
#[reflect(Component, Default)]
struct Motion {
    velocity: bevy_math::Vec3,
    speed: f32,
}

fn segment_variants(app: &App, segments: Entity) -> Vec<ButtonVariant> {
    app.world()
        .get::<Children>(segments)
        .unwrap()
        .iter()
        .map(|segment| app.world().get::<ButtonVariant>(*segment).unwrap().clone())
        .collect()
}

#[derive(Component, Reflect, Debug, Default)]
#[component(immutable)]
#[reflect(Component, Default)]
struct Frozen {
    flag: bool,
    mode: Mode,
    tint: Color,
}

#[derive(Component, Reflect, Debug, Default, PartialEq)]
#[reflect(Component, Default)]
enum Level {
    #[default]
    Low,
    High,
}

#[derive(Component, Reflect, Debug, Default)]
#[reflect(Component)]
struct Lookup(HashMap<String, f32>);

fn trigger_change<T: Send + Sync + 'static>(app: &mut App, source: Entity, value: T) {
    app.world_mut().trigger(ValueChange {
        source,
        value,
        is_final: true,
    });
    app.world_mut().flush();
    app.update();
}

fn hex_input(app: &mut App, entity: Entity) -> Entity {
    app.init_resource::<bevy_text::FontCx>()
        .init_resource::<bevy_text::LayoutCx>()
        .init_resource::<bevy_clipboard::Clipboard>()
        .add_systems(bevy_app::Update, bevy_text::apply_text_edits);
    app.world_mut().spawn((InspectorUi, InspectorDetailsBody));
    app.world_mut().resource_mut::<InspectorSelection>().0 =
        Some(InspectableEntity::new(entity, InspectorSource::Local));
    app.update();
    let mut inputs = app
        .world_mut()
        .query_filtered::<(Entity, &InspectorField), With<EditableText>>();
    inputs
        .iter(app.world())
        .find(|(_, field)| field.path == "color")
        .map(|(input, _)| input)
        .unwrap()
}

fn type_text(app: &mut App, input: Entity, text: &str) {
    replace_text(app.world_mut(), Some(input), text);
    app.update();
}

fn input_text(app: &App, input: Entity) -> String {
    app.world()
        .get::<EditableText>(input)
        .unwrap()
        .value()
        .to_string()
}

#[derive(Reflect, Debug, Default)]
struct Meters(#[reflect(@Unit("m"))] f32);

#[derive(Reflect, Debug, Default)]
struct Span(#[reflect(@DisplayName("Left"))] f32, f32);

#[derive(Reflect, Debug, Default)]
enum Gait {
    #[default]
    Still,
    Walk {
        #[reflect(@ValueRange(0.0..=1.0))]
        pace: f32,
    },
}

#[derive(Reflect, Debug, Default)]
#[reflect(@DisplayName("Limits"), @ReadOnly)]
struct Caps {
    max: f32,
}

#[derive(Reflect, Debug, Default)]
#[reflect(@DisplayName("Tuning"))]
struct Tuning {
    gain: f32,
}

#[derive(Reflect, Debug, Default, PartialEq)]
enum Stance {
    #[default]
    #[reflect(@DisplayName("Standing Still"))]
    Idle,
    Crouched,
}

#[derive(Component, Reflect, Debug, Default)]
#[reflect(Component, Default, @DisplayName("Movement Settings"))]
struct Annotated {
    #[reflect(@ValueRange(0.0..=10.0), @Step(0.5), @Unit("m/s"))]
    speed: f32,
    #[reflect(@DisplayName("Jump Height"))]
    jump: f32,
    #[reflect(@ReadOnly)]
    grounded: bool,
    #[reflect(@ValueRange(-5.0..=500.0))]
    level: u8,
    #[reflect(@1_u32..=10_u32)]
    count: u32,
    #[reflect(@ReadOnly, @Unit("kg"))]
    mass: f32,
    #[reflect(@ReadOnly)]
    nested: Nested,
    height: Meters,
    span: Span,
    gait: Gait,
    caps: Caps,
    #[reflect(@DisplayName("Custom Tuning"))]
    tuning: Tuning,
    stance: Stance,
}

fn has_text(world: &World, entity: Entity, expected: &str) -> bool {
    world
        .get::<Text>(entity)
        .is_some_and(|text| text.0 == expected)
        || world.get::<Children>(entity).is_some_and(|children| {
            children
                .iter()
                .any(|&child| has_text(world, child, expected))
        })
}

fn annotated() -> Annotated {
    Annotated {
        count: 1,
        mass: 2.5,
        gait: Gait::Walk { pace: 0.5 },
        ..Default::default()
    }
}

#[derive(Component, Reflect, Debug, Default)]
#[reflect(Component, Default, @ReadOnly)]
struct Sealed {
    value: f32,
}

fn float(value: f32) -> FieldValue {
    FieldValue::Number(NumericValue::F32(value))
}

#[derive(Component, Reflect, Debug, Default)]
#[reflect(Component, Default)]
struct Wrapped {
    #[reflect(@ValueRange(0.0..=2.0), @Step(0.25), @Unit("m"), @DisplayName("Reach"))]
    reach: Option<f32>,
    #[reflect(@ValueRange(0.0..=1.0))]
    samples: Vec<f32>,
}

fn wrapped() -> Wrapped {
    Wrapped {
        reach: Some(1.0),
        samples: vec![0.5],
    }
}
