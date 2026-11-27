//! Field edits from the details panel widgets, and writing them into the inspected entity.

use core::any::Any;

use alloc::{
    borrow::Cow,
    string::{String, ToString},
};

use bevy_color::{
    Color, ColorToComponents, Hsla, Hsva, Hwba, Laba, Lcha, LinearRgba, Okhsla, Okhsva, Okhwba,
    Oklaba, Oklcha, Srgba, Xyza,
};
use bevy_ecs::{
    change_detection::DetectChangesMut,
    component::ComponentInfo,
    entity::Entity,
    event::EntityEvent,
    observer::On,
    reflect::{AppTypeRegistry, ReflectComponent},
    system::{Commands, Query, ResMut},
    world::World,
};
use bevy_feathers::controls::OptionIndex;
use bevy_input::{
    keyboard::{KeyCode, KeyboardInput},
    ButtonState,
};
use bevy_input_focus::{FocusLost, FocusedInput};
use bevy_log::{warn, warn_once};
use bevy_reflect::{
    enums::{DynamicEnum, DynamicVariant, VariantInfo},
    prelude::ReflectDefault,
    structs::DynamicStruct,
    tuple::DynamicTuple,
    GetPath, PartialReflect, ReflectRef, TypeInfo, TypeRegistry,
};
use bevy_text::{EditableText, TextEditChange};
use bevy_ui_widgets::{NumericValue, ValueChange};

use super::{
    annotations::clamp_to_range,
    rows::{apply_value, color_to_hex, replace_text},
    sync::refresh_details_panel,
    ticks::record_inspector_write,
    walk::{scalar_value, walk_entries},
    ComponentKey, DetailsIndex, FieldEdit, FieldValue, InspectorField,
};
use crate::{InspectorSource, InspectorSources};

/// Records a widget's new value, writes it back into the widget and emits a [`FieldEdit`] for the
/// entity the panel was built for.
///
/// `value` maps the value the widget last showed to the new one, or returns `None` to skip the
/// change. The feathers controls do not update themselves when the user changes them, so the
/// widget only shows the new value once it is written back.
pub(super) fn emit_field_edit(
    widget: Entity,
    value: impl FnOnce(&FieldValue) -> Option<FieldValue>,
    fields: &Query<&InspectorField>,
    index: &mut DetailsIndex,
    commands: &mut Commands,
) {
    let Ok(field) = fields.get(widget) else {
        return;
    };
    let Some(selection) = index.selection.clone() else {
        return;
    };
    let Some(stored) = index
        .fields
        .get_mut(&(field.component.clone(), field.path.clone()))
    else {
        return;
    };
    let Some(value) = value(&stored.value) else {
        return;
    };

    stored.value = value.clone();
    let displayed = stored.clone();
    commands.queue(move |world: &mut World| apply_value(world, &displayed, &displayed.value));
    commands.trigger(FieldEdit {
        entity: selection.entity,
        source: selection.source,
        component: field.component.0.to_string(),
        path: field.path.clone(),
        value,
    });
}

/// Observer that turns a details panel checkbox change into a [`FieldEdit`].
pub(crate) fn inspector_field_bool_changed(
    change: On<ValueChange<bool>>,
    fields: Query<&InspectorField>,
    mut index: ResMut<DetailsIndex>,
    mut commands: Commands,
) {
    let value = FieldValue::Bool(change.value);
    emit_field_edit(
        change.source,
        |_| Some(value),
        &fields,
        &mut index,
        &mut commands,
    );
}

macro_rules! number_changed_observer {
    ($name:ident, $number:ty, $variant:ident) => {
        /// Observer that turns a details panel number input change into a [`FieldEdit`].
        pub(crate) fn $name(
            change: On<ValueChange<$number>>,
            fields: Query<&InspectorField>,
            mut index: ResMut<DetailsIndex>,
            mut commands: Commands,
        ) {
            let value = FieldValue::Number(NumericValue::$variant(change.value));
            emit_field_edit(
                change.source,
                |_| Some(value),
                &fields,
                &mut index,
                &mut commands,
            );
        }
    };
}

number_changed_observer!(inspector_field_f32_changed, f32, F32);
number_changed_observer!(inspector_field_f64_changed, f64, F64);
number_changed_observer!(inspector_field_i32_changed, i32, I32);
number_changed_observer!(inspector_field_i64_changed, i64, I64);

/// Observer that turns a details panel text input edit into a [`FieldEdit`].
///
/// Text equal to the value the input last showed is skipped, since it is the panel writing that
/// value into the input. Hexadecimal color inputs are committed by [`inspector_field_hex_entered`]
/// and [`inspector_field_hex_focus_lost`] instead.
pub(crate) fn inspector_field_text_changed(
    change: On<TextEditChange>,
    texts: Query<&EditableText>,
    fields: Query<&InspectorField>,
    mut index: ResMut<DetailsIndex>,
    mut commands: Commands,
) {
    let widget = change.event_target();
    let Ok(editable) = texts.get(widget) else {
        return;
    };
    let text = editable.value().to_string();
    emit_field_edit(
        widget,
        |current| match current {
            FieldValue::Text(current) if *current == text => None,
            FieldValue::Color(_) => None,
            _ => Some(FieldValue::Text(text)),
        },
        &fields,
        &mut index,
        &mut commands,
    );
}

/// Observer that commits a details panel hexadecimal color input when Enter is pressed.
pub(crate) fn inspector_field_hex_entered(
    key: On<FocusedInput<KeyboardInput>>,
    texts: Query<&EditableText>,
    fields: Query<&InspectorField>,
    mut index: ResMut<DetailsIndex>,
    mut commands: Commands,
) {
    if key.input.state != ButtonState::Pressed
        || !matches!(key.input.key_code, KeyCode::Enter | KeyCode::NumpadEnter)
    {
        return;
    }
    commit_hex_color(
        key.event_target(),
        &texts,
        &fields,
        &mut index,
        &mut commands,
    );
}

/// Observer that commits a details panel hexadecimal color input when it loses focus.
pub(crate) fn inspector_field_hex_focus_lost(
    focus_lost: On<FocusLost>,
    texts: Query<&EditableText>,
    fields: Query<&InspectorField>,
    mut index: ResMut<DetailsIndex>,
    mut commands: Commands,
) {
    commit_hex_color(
        focus_lost.event_target(),
        &texts,
        &fields,
        &mut index,
        &mut commands,
    );
}

/// Emits a [`FieldEdit`] for the color typed into the hexadecimal input `widget`.
///
/// Text that does not parse, or that names the stored color, is replaced by the stored color's
/// hexadecimal code.
fn commit_hex_color(
    widget: Entity,
    texts: &Query<&EditableText>,
    fields: &Query<&InspectorField>,
    index: &mut DetailsIndex,
    commands: &mut Commands,
) {
    let Ok(editable) = texts.get(widget) else {
        return;
    };
    let text = editable.value().to_string();
    let mut restore = None;
    emit_field_edit(
        widget,
        |current| {
            let FieldValue::Color(stored) = current else {
                return None;
            };
            match parse_hex_color(&text) {
                Some(color) if color_to_hex(color) != color_to_hex(*stored) => {
                    edited_color(*stored, color)
                }
                _ => {
                    restore = Some(color_to_hex(*stored));
                    None
                }
            }
        },
        fields,
        index,
        commands,
    );
    if let Some(hex) = restore {
        commands.queue(move |world: &mut World| replace_text(world, Some(widget), &hex));
    }
}

/// Observer that turns a details panel color picker change into a [`FieldEdit`].
pub(crate) fn inspector_field_color_changed(
    change: On<ValueChange<Color>>,
    fields: Query<&InspectorField>,
    mut index: ResMut<DetailsIndex>,
    mut commands: Commands,
) {
    let value = change.value;
    emit_field_edit(
        change.source,
        |current| match current {
            FieldValue::Color(stored) => edited_color(*stored, value),
            _ => Some(FieldValue::Color(value)),
        },
        &fields,
        &mut index,
        &mut commands,
    );
}

/// The edit replacing `stored` with `color`, logging a warning once if `stored` is outside the
/// `0..=1` range that the color picker and hexadecimal input are limited to.
fn edited_color(stored: Color, color: Color) -> Option<FieldValue> {
    if is_out_of_range(stored) {
        warn_once!("Color values outside 0..=1 are clamped by the inspector color picker");
    }
    Some(FieldValue::Color(color))
}

/// Whether any sRGB channel of `color` is outside `0..=1`, allowing for rounding errors.
pub(super) fn is_out_of_range(color: Color) -> bool {
    const TOLERANCE: f32 = 1e-4;
    Srgba::from(color)
        .to_f32_array()
        .into_iter()
        .any(|channel| !(-TOLERANCE..=1.0 + TOLERANCE).contains(&channel))
}

/// Observer that turns a details panel variant selection into a [`FieldEdit`].
pub(crate) fn inspector_field_variant_changed(
    change: On<ValueChange<Entity>>,
    options: Query<&OptionIndex>,
    fields: Query<&InspectorField>,
    mut index: ResMut<DetailsIndex>,
    mut commands: Commands,
) {
    let Ok(option) = options.get(change.value) else {
        return;
    };
    let selected = option.0;
    emit_field_edit(
        change.source,
        |current| match current {
            FieldValue::Variant { variants, .. } => Some(FieldValue::Variant {
                variants: variants.clone(),
                selected,
            }),
            _ => None,
        },
        &fields,
        &mut index,
        &mut commands,
    );
}

/// Observer that writes a [`FieldEdit`] into the component of the inspected entity.
pub(crate) fn apply_field_edit(edit: On<FieldEdit>, mut commands: Commands) {
    let edit = edit.event().clone();
    commands.queue(move |world: &mut World| write_field_edit(world, &edit));
}

/// Writes `edit` into its component, marking the component changed only if its value changes.
///
/// The details panel is refreshed on the next tick whenever the field does not end up holding the
/// edited value, so that its widget is reverted.
fn write_field_edit(world: &mut World, edit: &FieldEdit) {
    let Some(inspector_sources) = world.get_resource::<InspectorSources>() else {
        return;
    };
    if !inspector_sources.sources.contains(&edit.source) {
        return;
    }
    if edit.source != InspectorSource::Local {
        return;
    }
    let Some(registry) = world.get_resource::<AppTypeRegistry>().cloned() else {
        return;
    };
    let registry = registry.read();
    let Some((registration, reflect_component)) = registry
        .get_with_type_path(&edit.component)
        .and_then(|registration| Some((registration, registration.data::<ReflectComponent>()?)))
    else {
        warn!("the inspector cannot edit `{}`", edit.component);
        return;
    };
    if world.get_entity(edit.entity).is_err() {
        return;
    }
    let id = world.components().get_id(registration.type_id());
    let mutable = id
        .and_then(|id| world.components().get_info(id))
        .map(ComponentInfo::mutable);
    if mutable == Some(false) {
        warn!("`{}` is immutable and cannot be edited", edit.component);
        return;
    }

    let component =
        mutable.and_then(|_| reflect_component.reflect_mut(world.entity_mut(edit.entity)));
    let Some(mut component) = component else {
        warn!("`{}` is no longer on the entity", edit.component);
        refresh_details_panel(world);
        return;
    };

    let annotations = walk_entries(component.as_partial_reflect(), &registry, Some(&edit.path))
        .into_iter()
        .find(|entry| entry.path == edit.path)
        .map(|entry| entry.annotations)
        .unwrap_or_default();
    if annotations.read_only {
        warn!("`{}` at `{}` is read-only", edit.component, edit.path);
        return;
    }
    let value = clamp_to_range(&edit.value, annotations.range.as_ref());

    let target = component.bypass_change_detection();
    let field = if edit.path.is_empty() {
        Ok(target.as_partial_reflect_mut())
    } else {
        target.reflect_path_mut(edit.path.as_str())
    };
    let (write, held, typing_a_char) = match field {
        Ok(field) => {
            let write = write_field_value(field, &value, &registry);
            let held = write != FieldWrite::Rejected && holds_value(field, &edit.value);
            let typing_a_char = matches!(edit.value, FieldValue::Text(_))
                && field.try_downcast_ref::<char>().is_some();
            (write, held, typing_a_char)
        }
        Err(_) => (FieldWrite::Rejected, false, false),
    };
    if write == FieldWrite::Written {
        component.set_changed();
        if let Some(id) = id {
            record_inspector_write(
                world,
                edit.entity,
                ComponentKey::from(edit.component.as_str()),
                id,
            );
        }
    }
    if write == FieldWrite::Rejected && !typing_a_char {
        warn!(
            "the inspector cannot edit `{}` at `{}`",
            edit.component, edit.path
        );
    }
    if !held {
        refresh_details_panel(world);
    }
}

/// Whether `field` holds `value`, so that a widget showing `value` is up to date.
fn holds_value(field: &dyn PartialReflect, value: &FieldValue) -> bool {
    scalar_value(field).is_some_and(|current| same_value(value, &current))
}

/// Whether a widget showing `shown` is up to date with a field holding `held`.
///
/// Floats are compared bit for bit so that a NaN equals itself. Colors are compared channel by
/// channel after converting `shown` into the color space of `held`.
pub(super) fn same_value(shown: &FieldValue, held: &FieldValue) -> bool {
    use FieldValue::{Color, Number};
    match (shown, held) {
        (Color(shown), Color(held)) => in_color_space(*shown, *held) == *held,
        (Number(NumericValue::F32(shown)), Number(NumericValue::F32(held))) => {
            shown.to_bits() == held.to_bits()
        }
        (Number(NumericValue::F64(shown)), Number(NumericValue::F64(held))) => {
            shown.to_bits() == held.to_bits()
        }
        _ => shown == held,
    }
}

/// Describes the outcome of a field write attempt.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum FieldWrite {
    /// The field now holds a new value.
    Written,
    /// The field already held the value, so nothing was written.
    Unchanged,
    /// The value cannot be written into the field.
    Rejected,
}

/// Assigns `value` to `target`, reporting the change based on previous data.
fn assign<T: PartialEq>(target: &mut T, value: T) -> FieldWrite {
    if *target == value {
        FieldWrite::Unchanged
    } else {
        *target = value;
        FieldWrite::Written
    }
}

fn write_field_value(
    field: &mut dyn PartialReflect,
    value: &FieldValue,
    registry: &TypeRegistry,
) -> FieldWrite {
    match value {
        FieldValue::Bool(new) => match field.try_downcast_mut::<bool>() {
            Some(target) => assign(target, *new),
            None => FieldWrite::Rejected,
        },
        FieldValue::Number(number) => write_number_field(field, *number),
        FieldValue::Text(text) => write_text_field(field, text),
        FieldValue::Color(color) => write_color_field(field, *color),
        FieldValue::Variant { variants, selected } => match variants.get(*selected) {
            Some(name) => write_variant_field(field, name, registry),
            None => FieldWrite::Rejected,
        },
        FieldValue::Label(_) => FieldWrite::Rejected,
    }
}

/// Writes text into a string or `char` field, allocating only when the text differs.
///
/// A `char` field takes the last character of the text, so that typing replaces it.
fn write_text_field(field: &mut dyn PartialReflect, text: &str) -> FieldWrite {
    if let Some(target) = field.try_downcast_mut::<String>() {
        if target == text {
            return FieldWrite::Unchanged;
        }
        *target = text.to_string();
        return FieldWrite::Written;
    }
    if let Some(target) = field.try_downcast_mut::<Cow<'static, str>>() {
        if target == text {
            return FieldWrite::Unchanged;
        }
        *target = Cow::Owned(text.to_string());
        return FieldWrite::Written;
    }
    if let Some(target) = field.try_downcast_mut::<char>()
        && let Some(last) = text.chars().last()
    {
        return assign(target, last);
    }
    FieldWrite::Rejected
}

/// Writes a number into a numeric field, rejecting non-finite values and integers outside the
/// field's range.
///
/// Floats are compared bit for bit, so that writing `-0.0` over `0.0` counts as a change.
fn write_number_field(field: &mut dyn PartialReflect, value: NumericValue) -> FieldWrite {
    let float = match value {
        NumericValue::F32(value) => value as f64,
        NumericValue::F64(value) => value,
        NumericValue::I32(value) => value as f64,
        NumericValue::I64(value) => value as f64,
    };
    if !float.is_finite() {
        return FieldWrite::Rejected;
    }
    if let Some(target) = field.try_downcast_mut::<f32>() {
        let narrowed = match value {
            NumericValue::F32(value) => value,
            _ => float as f32,
        };
        if !narrowed.is_finite() {
            return FieldWrite::Rejected;
        }
        return assign_bits(target, narrowed, f32::to_bits);
    }
    if let Some(target) = field.try_downcast_mut::<f64>() {
        return assign_bits(target, float, f64::to_bits);
    }

    let integer = match value {
        NumericValue::I32(value) => i128::from(value),
        NumericValue::I64(value) => i128::from(value),
        _ if float.fract() == 0.0 => float as i128,
        _ => return FieldWrite::Rejected,
    };

    if let Some(target) = field.try_downcast_mut::<i128>() {
        return assign(target, integer);
    }

    macro_rules! write_as {
        ($($type:ty),*) => {
            $(
                if let Some(target) = field.try_downcast_mut::<$type>() {
                    return match <$type>::try_from(integer) {
                        Ok(value) => assign(target, value),
                        Err(_) => FieldWrite::Rejected,
                    };
                }
            )*
        };
    }

    write_as!(i8, i16, i32, i64, isize, u8, u16, u32, u64, u128, usize);
    FieldWrite::Rejected
}

/// Assigns a float to `target`, comparing the `bits` of both values rather than using `PartialEq`.
fn assign_bits<T: Copy, B: PartialEq>(target: &mut T, value: T, bits: fn(T) -> B) -> FieldWrite {
    if bits(*target) == bits(value) {
        FieldWrite::Unchanged
    } else {
        *target = value;
        FieldWrite::Written
    }
}

/// Writes a color into a color field, keeping the field's color space.
fn write_color_field(field: &mut dyn PartialReflect, color: Color) -> FieldWrite {
    if let Some(target) = field.try_downcast_mut::<Color>() {
        let value = in_color_space(color, *target);
        return assign(target, value);
    }

    let writers: [fn(&mut dyn PartialReflect, Color) -> Option<FieldWrite>; 13] = [
        write_color_as::<Srgba>,
        write_color_as::<LinearRgba>,
        write_color_as::<Hsla>,
        write_color_as::<Hsva>,
        write_color_as::<Hwba>,
        write_color_as::<Laba>,
        write_color_as::<Lcha>,
        write_color_as::<Oklaba>,
        write_color_as::<Oklcha>,
        write_color_as::<Xyza>,
        write_color_as::<Okhsla>,
        write_color_as::<Okhsva>,
        write_color_as::<Okhwba>,
    ];
    writers
        .into_iter()
        .find_map(|write| write(field, color))
        .unwrap_or(FieldWrite::Rejected)
}

/// Writes `color` into `field` if it is a `T`.
fn write_color_as<T: From<Color> + PartialEq + Any>(
    field: &mut dyn PartialReflect,
    color: Color,
) -> Option<FieldWrite> {
    field
        .try_downcast_mut::<T>()
        .map(|target| assign(target, T::from(color)))
}

/// Converts `color` into the color space of the `space` variant.
fn in_color_space(color: Color, space: Color) -> Color {
    match space {
        Color::Srgba(_) => Color::Srgba(color.into()),
        Color::LinearRgba(_) => Color::LinearRgba(color.into()),
        Color::Hsla(_) => Color::Hsla(color.into()),
        Color::Hsva(_) => Color::Hsva(color.into()),
        Color::Hwba(_) => Color::Hwba(color.into()),
        Color::Laba(_) => Color::Laba(color.into()),
        Color::Lcha(_) => Color::Lcha(color.into()),
        Color::Oklaba(_) => Color::Oklaba(color.into()),
        Color::Oklcha(_) => Color::Oklcha(color.into()),
        Color::Xyza(_) => Color::Xyza(color.into()),
        Color::Okhsla(_) => Color::Okhsla(color.into()),
        Color::Okhsva(_) => Color::Okhsva(color.into()),
        Color::Okhwba(_) => Color::Okhwba(color.into()),
    }
}

/// Parses a `#RRGGBB` or `#RRGGBBAA` hexadecimal sRGB color.
pub(super) fn parse_hex_color(text: &str) -> Option<Color> {
    let text = text.trim();
    let digits = text.strip_prefix('#').unwrap_or(text);
    if !matches!(digits.len(), 6 | 8) {
        return None;
    }
    Srgba::hex(digits).ok().map(Color::from)
}

/// Switches an enum field to the variant `name`, building any variant fields from their defaults.
fn write_variant_field(
    field: &mut dyn PartialReflect,
    name: &str,
    registry: &TypeRegistry,
) -> FieldWrite {
    let ReflectRef::Enum(current) = field.reflect_ref() else {
        return FieldWrite::Rejected;
    };
    if current.variant_name() == name {
        return FieldWrite::Unchanged;
    }
    let Some(TypeInfo::Enum(info)) = field.get_represented_type_info() else {
        return FieldWrite::Rejected;
    };
    let Some(variant) = info
        .variant(name)
        .and_then(|variant| default_variant(variant, registry))
    else {
        return FieldWrite::Rejected;
    };
    match field.try_apply(&DynamicEnum::new(name, variant)) {
        Ok(()) => FieldWrite::Written,
        Err(_) => FieldWrite::Rejected,
    }
}

fn default_variant(variant: &VariantInfo, registry: &TypeRegistry) -> Option<DynamicVariant> {
    let default = |type_id| {
        registry
            .get_type_data::<ReflectDefault>(type_id)
            .map(|default| default.default().into_partial_reflect())
    };
    Some(match variant {
        VariantInfo::Unit(_) => DynamicVariant::Unit,
        VariantInfo::Tuple(info) => {
            let mut tuple = DynamicTuple::default();
            for field in info.iter() {
                tuple.insert_boxed(default(field.type_id())?);
            }
            DynamicVariant::Tuple(tuple)
        }
        VariantInfo::Struct(info) => {
            let mut fields = DynamicStruct::default();
            for field in info.iter() {
                fields.insert_boxed(field.name(), default(field.type_id())?);
            }
            DynamicVariant::Struct(fields)
        }
    })
}

/// Whether every field of `variant` has a registered [`ReflectDefault`].
pub(super) fn has_defaults(variant: &VariantInfo, registry: &TypeRegistry) -> bool {
    let has_default = |type_id| registry.get_type_data::<ReflectDefault>(type_id).is_some();
    match variant {
        VariantInfo::Unit(_) => true,
        VariantInfo::Tuple(info) => info.iter().all(|field| has_default(field.type_id())),
        VariantInfo::Struct(info) => info.iter().all(|field| has_default(field.type_id())),
    }
}
