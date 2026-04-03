//! BSN asset catalog: load and save named asset definitions in `.bsn` format.
//!
//! **Loading**: [`load_bsn_assets`] parses BSN text containing named asset
//! definitions and inserts them into `Assets<T>` stores via reflection.
//!
//! **Saving**: [`serialize_assets_to_bsn`] reflects named assets from the world
//! and emits BSN text with default-diffing (only non-default fields are written).

use core::any::TypeId;
use core::fmt::Write;

use bevy_asset::{AssetServer, ReflectHandle, UntypedAssetId, UntypedHandle};
use bevy_ecs::{prelude::*, reflect::AppTypeRegistry};
use bevy_reflect::{prelude::ReflectDefault, PartialReflect, ReflectMut, ReflectRef, TypeRegistry};

use crate::dynamic_bsn::{BsnAst, BsnExpr, BsnNameStore, BsnPatch, BsnPatches};
use crate::dynamic_bsn_grammar::TopLevelPatchesParser;
use crate::dynamic_bsn_lexer::Lexer;

/// A named asset entry produced by [`load_bsn_assets`].
pub struct CatalogEntry {
    /// The `#Name` from the BSN catalog.
    pub name: String,
    /// Handle to the created asset in the `Assets<T>` store.
    pub handle: UntypedHandle,
}

/// A named asset reference for serialization by [`serialize_assets_to_bsn`].
pub struct CatalogAssetRef {
    /// Display name for the asset in the catalog (becomes `#Name` in BSN).
    pub name: String,
    /// The concrete asset type (e.g., `TypeId::of::<StandardMaterial>()`).
    pub type_id: TypeId,
    /// The asset's ID in its `Assets<T>` store.
    pub asset_id: UntypedAssetId,
}

// ---------------------------------------------------------------------------
// Loading
// ---------------------------------------------------------------------------

/// Parse BSN text containing named asset definitions and insert them into
/// the corresponding `Assets<T>` stores via reflection.
pub fn load_bsn_assets(
    world: &mut World,
    bsn_text: &str,
) -> Result<Vec<CatalogEntry>, String> {
    let mut parse_world = World::new();
    parse_world.init_resource::<BsnNameStore>();
    let ast = core::cell::RefCell::new(BsnAst(parse_world));
    let lexer = Lexer::new(bsn_text);
    let patches_id = TopLevelPatchesParser::new()
        .parse(&ast, lexer)
        .map_err(|e| format!("BSN asset parse error: {e:?}"))?;
    let ast = ast.into_inner();

    let entries = unwrap_children_wrapper(&ast, patches_id)?;

    let registry = world.resource::<AppTypeRegistry>().clone();
    let reg = registry.read();

    let mut results = Vec::new();
    for entry_id in entries {
        let Some(patches) = ast.0.get::<BsnPatches>(entry_id) else { continue };

        let mut name = None;
        let mut handle = None;

        for &pid in &patches.0 {
            let Some(patch) = ast.0.get::<BsnPatch>(pid) else { continue };
            match patch {
                BsnPatch::Name(n, _) => name = Some(n.clone()),
                BsnPatch::Struct(s) => {
                    handle = create_asset_from_struct(world, &s.0.as_path(), &s.1, &ast, &reg);
                }
                BsnPatch::Var(v) => {
                    handle = create_asset_from_struct(world, &v.0.as_path(), &[], &ast, &reg);
                }
                _ => {}
            }
        }

        if let (Some(name), Some(handle)) = (name, handle) {
            results.push(CatalogEntry { name, handle });
        }
    }

    Ok(results)
}

/// If the top-level is a single `Children` relation, unwrap to get the entries inside.
fn unwrap_children_wrapper(ast: &BsnAst, patches_id: Entity) -> Result<Vec<Entity>, String> {
    let patches = ast
        .0
        .get::<BsnPatches>(patches_id)
        .ok_or("No top-level patches found")?;
    if patches.0.len() == 1 {
        if let Some(BsnPatch::Relation(relation)) = ast.0.get::<BsnPatch>(patches.0[0]) {
            return Ok(relation.1.clone());
        }
    }
    Ok(vec![patches_id])
}

/// Create an asset instance from a BSN struct definition via reflection.
fn create_asset_from_struct(
    world: &mut World,
    type_path: &str,
    fields: &[crate::dynamic_bsn::BsnField],
    ast: &BsnAst,
    registry: &TypeRegistry,
) -> Option<UntypedHandle> {
    let registration = registry.get_with_type_path(type_path)?;
    let reflect_default = registration.data::<ReflectDefault>()?;
    let mut value = reflect_default.default();

    if let Ok(struct_info) = registration.type_info().as_struct() {
        if let ReflectMut::Struct(s) = value.reflect_mut() {
            for field in fields {
                let Some(fi) = struct_info.field(&field.0) else { continue };
                let Some(expr) = ast.0.get::<BsnExpr>(field.1) else { continue };
                apply_bsn_expr(s, &field.0, expr, fi.ty().id(), registry, ast);
            }
        }
    }

    let reflect_asset = registration.data::<bevy_asset::ReflectAsset>()?;
    Some(reflect_asset.add(world, value.as_partial_reflect()))
}

/// Apply a BSN expression value to a struct field via reflection.
fn apply_bsn_expr(
    target: &mut dyn bevy_reflect::structs::Struct,
    field_name: &str,
    expr: &BsnExpr,
    expected_type: TypeId,
    registry: &TypeRegistry,
    ast: &BsnAst,
) {
    let Some(field) = target.field_mut(field_name) else { return };

    match expr {
        BsnExpr::FloatLit(f) => {
            if expected_type == TypeId::of::<f32>() {
                field.apply(&(*f as f32));
            } else if expected_type == TypeId::of::<f64>() {
                field.apply(f);
            }
        }
        BsnExpr::IntLit(i) => {
            macro_rules! try_int {
                ($($t:ty),*) => {
                    $(if expected_type == TypeId::of::<$t>() { field.apply(&(*i as $t)); return; })*
                };
            }
            try_int!(i8, u8, i16, u16, i32, u32, i64, u64, usize, isize);
        }
        BsnExpr::BoolLit(b) => field.apply(b),
        BsnExpr::StringLit(s) => {
            if expected_type == TypeId::of::<String>() {
                field.apply(s);
            }
        }
        BsnExpr::Struct(bsn_struct) => {
            let type_path = bsn_struct.0.as_path();
            let Some(reg) = registry.get_with_type_path(&type_path) else { return };
            let Ok(si) = reg.type_info().as_struct() else { return };
            let ReflectMut::Struct(s) = field.reflect_mut() else { return };
            for f in &bsn_struct.1 {
                let Some(fi) = si.field(&f.0) else { continue };
                let Some(e) = ast.0.get::<BsnExpr>(f.1) else { continue };
                apply_bsn_expr(s, &f.0, e, fi.ty().id(), registry, ast);
            }
        }
        _ => {}
    }
}

// ---------------------------------------------------------------------------
// Serialization
// ---------------------------------------------------------------------------

/// Serialize named assets to a BSN catalog string.
///
/// Each entry is reflected from its `Assets<T>` store, compared against its
/// default, and emitted with only non-default fields.
pub fn serialize_assets_to_bsn(world: &World, assets: &[CatalogAssetRef]) -> String {
    if assets.is_empty() {
        return String::new();
    }

    let registry = world.resource::<AppTypeRegistry>().clone();
    let reg = registry.read();
    let asset_server = world.get_resource::<AssetServer>();

    let mut entries: Vec<(String, String)> = Vec::new();

    for asset_ref in assets {
        let Some(registration) = reg.get(asset_ref.type_id) else { continue };
        let Some(reflect_asset) = registration.data::<bevy_asset::ReflectAsset>() else { continue };
        let Some(asset_data) = reflect_asset.get(world, asset_ref.asset_id) else { continue };

        let type_path = registration.type_info().type_path_table().path();
        let default_value = registration.data::<ReflectDefault>().map(|rd| rd.default());

        let mut entry = String::new();
        write_name(&asset_ref.name, 1, &mut entry);

        if let ReflectRef::Struct(s) = asset_data.reflect_ref() {
            let default_struct = default_value.as_ref().and_then(|d| match d.reflect_ref() {
                ReflectRef::Struct(ds) => Some(ds),
                _ => None,
            });
            let fields = diff_struct_fields(s, default_struct, &reg, asset_server);
            write_struct(type_path, &fields, 1, &mut entry);
        } else {
            write_indent(&mut entry, 1, &format!("{type_path}\n"));
        }

        entries.push((asset_ref.name.clone(), entry));
    }

    entries.sort_by(|a, b| a.0.cmp(&b.0));

    let mut out = String::from("bevy_ecs::hierarchy::Children [\n");
    for (i, (_, entry)) in entries.iter().enumerate() {
        out.push_str(entry);
        if i + 1 < entries.len() {
            out.push_str("    ,\n");
        }
    }
    out.push_str("]\n");
    out
}

/// Collect struct fields that differ from defaults.
fn diff_struct_fields(
    s: &dyn bevy_reflect::structs::Struct,
    default: Option<&dyn bevy_reflect::structs::Struct>,
    registry: &TypeRegistry,
    asset_server: Option<&AssetServer>,
) -> Vec<(String, String)> {
    let mut fields = Vec::new();
    for i in 0..s.field_len() {
        let name = s.name_at(i).unwrap();
        let value = s.field_at(i).unwrap();

        // Skip fields that match the default
        if let Some(ds) = default {
            if let Some(df) = ds.field(name) {
                if value.reflect_partial_eq(df).unwrap_or(false) {
                    continue;
                }
            }
        }

        // Handle<T> -> asset path string
        if let Some(path) = resolve_handle_path(value, registry, asset_server) {
            fields.push((name.to_string(), format!("\"{path}\"")));
            continue;
        }

        // Option<Handle<T>> -> unwrap Some, skip None
        if let ReflectRef::Enum(e) = value.reflect_ref() {
            if e.variant_name() == "None" {
                continue;
            }
            if e.variant_name() == "Some" {
                if let Some(inner) = e.field_at(0) {
                    if let Some(path) = resolve_handle_path(inner, registry, asset_server) {
                        fields.push((name.to_string(), format!("\"{path}\"")));
                        continue;
                    }
                }
            }
        }

        // Skip generic types the BSN parser can't round-trip
        if let Some(ti) = value.get_represented_type_info() {
            if ti.type_path().contains('<') {
                continue;
            }
        }

        let mut val = String::new();
        write_value(value, registry, asset_server, &mut val);
        fields.push((name.to_string(), val));
    }
    fields
}

/// Try to resolve a reflected value as a Handle and return its asset path.
fn resolve_handle_path(
    value: &dyn PartialReflect,
    registry: &TypeRegistry,
    asset_server: Option<&AssetServer>,
) -> Option<String> {
    let asset_server = asset_server?;
    let concrete = value.try_as_reflect()?;
    let type_id = concrete.reflect_type_info().type_id();
    let reflect_handle = registry.get_type_data::<ReflectHandle>(type_id)?;
    let handle = reflect_handle.downcast_handle_untyped(concrete.as_any())?;
    let path = asset_server.get_path(handle.id())?;
    Some(path.path().to_string_lossy().into_owned())
}

/// Write a single reflected value as inline BSN text.
fn write_value(
    value: &dyn PartialReflect,
    registry: &TypeRegistry,
    asset_server: Option<&AssetServer>,
    out: &mut String,
) {
    if let Some(v) = value.try_downcast_ref::<f32>() {
        return write_float(*v, out);
    }
    if let Some(v) = value.try_downcast_ref::<f64>() {
        return write_float(*v as f32, out);
    }
    if let Some(v) = value.try_downcast_ref::<bool>() {
        return write!(out, "{v}").unwrap();
    }
    if let Some(v) = value.try_downcast_ref::<String>() {
        return write!(out, "\"{}\"", v.replace('\\', "\\\\").replace('"', "\\\"")).unwrap();
    }
    macro_rules! try_int {
        ($($t:ty),*) => {
            $(if let Some(v) = value.try_downcast_ref::<$t>() { return write!(out, "{v}").unwrap(); })*
        };
    }
    try_int!(i8, u8, i16, u16, i32, u32, i64, u64, isize, usize);

    if let Some(path) = resolve_handle_path(value, registry, asset_server) {
        return write!(out, "\"{path}\"").unwrap();
    }

    let tp = value
        .get_represented_type_info()
        .map(|i| i.type_path())
        .unwrap_or("unknown");

    match value.reflect_ref() {
        ReflectRef::Struct(s) if s.field_len() > 0 => {
            write!(out, "{tp} {{ ").unwrap();
            for i in 0..s.field_len() {
                if i > 0 { write!(out, ", ").unwrap(); }
                write!(out, "{}: ", s.name_at(i).unwrap()).unwrap();
                write_value(s.field_at(i).unwrap(), registry, asset_server, out);
            }
            write!(out, " }}").unwrap();
        }
        ReflectRef::Struct(_) => write!(out, "{tp}").unwrap(),
        ReflectRef::TupleStruct(ts) => {
            write!(out, "{tp}(").unwrap();
            for i in 0..ts.field_len() {
                if i > 0 { write!(out, ", ").unwrap(); }
                write_value(ts.field(i).unwrap(), registry, asset_server, out);
            }
            write!(out, ")").unwrap();
        }
        ReflectRef::Enum(e) => write!(out, "{tp}::{}", e.variant_name()).unwrap(),
        _ => write!(out, "\"<unsupported>\"").unwrap(),
    }
}

// ---------------------------------------------------------------------------
// BSN formatting helpers
// ---------------------------------------------------------------------------

fn write_struct(type_path: &str, fields: &[(String, String)], indent: usize, out: &mut String) {
    if fields.is_empty() {
        write_indent(out, indent, &format!("{type_path}\n"));
    } else {
        write_indent(out, indent, &format!("{type_path} {{\n"));
        for (name, val) in fields {
            write_indent(out, indent + 1, &format!("{name}: {val},\n"));
        }
        write_indent(out, indent, "}\n");
    }
}

fn write_name(name: &str, indent: usize, out: &mut String) {
    if !name.is_empty() && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
        write_indent(out, indent, &format!("#{name}\n"));
    } else {
        let escaped = name.replace('\\', "\\\\").replace('"', "\\\"");
        write_indent(out, indent, &format!("#\"{escaped}\"\n"));
    }
}

fn write_indent(out: &mut String, indent: usize, text: &str) {
    for _ in 0..indent {
        out.push_str("    ");
    }
    out.push_str(text);
}

fn write_float(f: f32, out: &mut String) {
    if f.fract() == 0.0 {
        write!(out, "{f:.1}").unwrap();
    } else {
        write!(out, "{f}").unwrap();
    }
}
