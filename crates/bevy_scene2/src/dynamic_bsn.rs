//! BSN assets loaded from `.bsn` files.

use bevy_asset::{io::Reader, AssetLoader, AssetPath, LoadContext};
use bevy_ecs::{
    entity::Entity,
    error::{BevyError, Result as EcsResult},
    hierarchy::ChildOf,
    name::Name,
    prelude::{Component, Resource},
    reflect::{AppTypeRegistry, ReflectFromTemplate, ReflectTemplate},
    template::{ErasedTemplate, TemplateContext},
    world::{FromWorld, World},
};
use bevy_log::error;
use bevy_platform::collections::HashMap;
use bevy_reflect::{
    enums::{DynamicEnum, DynamicVariant, StructVariantInfo, VariantInfoError},
    list::DynamicList,
    prelude::ReflectDefault,
    structs::{DynamicStruct, StructInfo},
    tuple_struct::DynamicTupleStruct,
    NamedField, PartialReflect, Reflect, ReflectMut, TypePath, TypeRegistration, TypeRegistry,
};
use core::{
    any::{Any, TypeId},
    cell::RefCell,
    fmt::Write,
    mem,
    str::Utf8Error,
};
use std::io::Error as IoError;
use thiserror::Error;

use crate::{
    dynamic_bsn_grammar::TopLevelPatchesParser, dynamic_bsn_lexer::Lexer, InheritSceneAsset,
    NameEntityReference, RelatedResolvedScenes, ResolveContext, ResolveSceneError, ResolvedScene,
    Scene, SceneDependencies, ScenePatch, SceneScope,
};

#[derive(Default)]
pub struct BsnAst(pub World);

#[derive(Resource, Default)]
pub struct BsnNameStore {
    pub name_indices: HashMap<String, usize>,
    pub next_name_index: usize,
}

#[derive(Component)]
pub struct BsnPatches(pub Vec<Entity>);

#[derive(Component)]
pub enum BsnPatch {
    Name(String, usize),
    Base(String),
    Var(BsnVar),
    Struct(BsnStruct),
    NamedTuple(BsnNamedTuple),
    Relation(BsnRelation),
}

#[derive(Clone)]
pub struct BsnVar(pub BsnSymbol, pub bool);

#[derive(Clone)]
pub struct BsnSymbol(pub Vec<String>, pub String);

pub struct BsnStruct(pub BsnSymbol, pub Vec<BsnField>, pub bool);

pub struct BsnField(pub String, pub Entity);

pub struct BsnNamedTuple(pub BsnSymbol, pub Vec<Entity>, pub bool);

pub struct BsnRelation(pub BsnSymbol, pub Vec<Entity>);

#[derive(Component)]
pub enum BsnExpr {
    Var(BsnVar),
    Struct(BsnStruct),
    NamedTuple(BsnNamedTuple),
    StringLit(String),
    FloatLit(f64),
    BoolLit(bool),
    IntLit(i128),
    List(Vec<Entity>),
}

impl BsnSymbol {
    pub fn from_ident(ident: String) -> BsnSymbol {
        BsnSymbol(vec![], ident)
    }

    pub fn append(mut self, ident: String) -> BsnSymbol {
        self.0.push(mem::replace(&mut self.1, ident));
        self
    }
}

#[derive(TypePath)]
pub struct DynamicBsnLoader {
    type_registry: AppTypeRegistry,
}

impl FromWorld for DynamicBsnLoader {
    fn from_world(world: &mut World) -> Self {
        DynamicBsnLoader {
            type_registry: world.resource::<AppTypeRegistry>().clone(),
        }
    }
}

// TODO: Report multiple errors
#[derive(Error, Debug)]
pub enum DynamicBsnLoaderError {
    #[error("I/O error: {0}")]
    Io(#[from] IoError),
    #[error("UTF-8 error: {0}")]
    Utf8(#[from] Utf8Error),
    #[error("parse error: {0}")]
    Parse(String),
    #[error("no such AST node")]
    NoSuchAstNode,
    #[error("only `Children` relations supported")]
    OnlyChildRelationsSupported,
    #[error("type doesn't implement `Default`: {0}")]
    TypeDoesntImplementDefault(String),
    #[error("type isn't a tuple structure")]
    TypeNotNamedTuple,
    #[error("type isn't a structure")]
    TypeNotStruct,
    #[error("variant isn't a tuple variant: {0}")]
    VariantNotTuple(#[from] VariantInfoError),
    #[error("structure doesn't have a field named `{0}`")]
    StructDoesntHaveField(String),
    #[error("unknown type: `{0}`")]
    UnknownType(String),
    #[error("type mismatch")]
    TypeMismatch,
    #[error("type mismatch, expected `f32` or `f64`")]
    FloatLitTypeMismatch,
    #[error(
        "type mismatch, expected `i8`, `u8`, `i16`, `u16`, `i32`, `u32`, `i64`, `u64`, \
        `isize`, or `usize`"
    )]
    IntLitTypeMismatch,
}

impl AssetLoader for DynamicBsnLoader {
    type Asset = ScenePatch;

    type Settings = ();

    type Error = DynamicBsnLoaderError;

    fn extensions(&self) -> &[&str] {
        &["bsn"]
    }

    async fn load(
        &self,
        reader: &mut dyn Reader,
        _settings: &Self::Settings,
        load_context: &mut LoadContext<'_>,
    ) -> Result<Self::Asset, Self::Error> {
        let mut buffer = vec![];
        reader.read_to_end(&mut buffer).await?;
        let input = str::from_utf8(&buffer)?;

        let mut world = World::new();
        world.init_resource::<BsnNameStore>();
        let ast = RefCell::new(BsnAst(world));

        let lexer = Lexer::new(input);
        let patches_id = match TopLevelPatchesParser::new().parse(&ast, lexer) {
            Ok(patches_id) => patches_id,
            Err(err) => {
                return Err(DynamicBsnLoaderError::Parse(format!("{:?}", err)));
            }
        };

        let ast = ast.into_inner();

        // Register named asset entries as labeled sub-assets (e.g., materials)
        self.register_labeled_assets(&ast, patches_id, load_context);

        let patch = ast.convert_bsn_patches_to_patch(patches_id, &self.type_registry)?;

        Ok(ScenePatch {
            scene: Box::new(SceneScope(patch.scene)),
            dependencies: patch.dependencies,
            resolved: None,
        })
    }
}

impl DynamicBsnLoader {
    /// Scan the parsed AST for named entries with asset types and register them
    /// as labeled sub-assets so they're resolvable via
    /// `asset_server.load("file.bsn#name")`.
    fn register_labeled_assets(
        &self,
        ast: &BsnAst,
        patches_id: Entity,
        load_context: &mut LoadContext<'_>,
    ) {
        let Some(patches) = ast.0.get::<BsnPatches>(patches_id) else {
            return;
        };

        // Unwrap Children wrapper if present
        let entries: Vec<Entity> = if patches.0.len() == 1 {
            if let Some(BsnPatch::Relation(relation)) = ast.0.get::<BsnPatch>(patches.0[0]) {
                relation.1.clone()
            } else {
                vec![patches_id]
            }
        } else {
            vec![patches_id]
        };

        let type_registry = self.type_registry.read();

        for entry_id in entries {
            let Some(entry_patches) = ast.0.get::<BsnPatches>(entry_id) else {
                continue;
            };

            let mut name: Option<String> = None;
            let mut struct_patch: Option<&BsnStruct> = None;

            for &pid in &entry_patches.0 {
                let Some(patch) = ast.0.get::<BsnPatch>(pid) else {
                    continue;
                };
                match patch {
                    BsnPatch::Name(n, _) => name = Some(n.clone()),
                    BsnPatch::Struct(s) => struct_patch = Some(s),
                    _ => {}
                }
            }

            let (Some(name), Some(bsn_struct)) = (name, struct_patch) else {
                continue;
            };

            let type_path = bsn_struct.0.as_path();
            let Some(registration) = type_registry.get_with_type_path(&type_path) else {
                continue;
            };
            let Some(reflect_asset) =
                registration.data::<bevy_asset::ReflectAsset>()
            else {
                continue;
            };
            let Some(reflect_default) =
                registration.data::<bevy_reflect::prelude::ReflectDefault>()
            else {
                continue;
            };

            let mut value = reflect_default.default();
            if let bevy_reflect::ReflectMut::Struct(s) = value.reflect_mut() {
                if let Ok(struct_info) = registration.type_info().as_struct() {
                    for field in &bsn_struct.1 {
                        if let Some(field_info) = struct_info.field(&field.0) {
                            if let Ok(reflected) = ast.convert_bsn_expr_to_reflect(
                                field.1,
                                &self.type_registry,
                                field_info.ty().id(),
                            ) {
                                if let Some(target) = s.field_mut(&field.0) {
                                    target.apply(&*reflected);
                                }
                            }
                        }
                    }
                }
            }

            if let Some(erased) = reflect_asset.into_loaded_asset(value.as_partial_reflect()) {
                load_context.add_loaded_labeled_asset_erased(
                    name,
                    erased,
                    registration.type_id(),
                );
            }
        }
    }
}

impl BsnAst {
    fn convert_bsn_patches_to_patch(
        &self,
        patches_id: Entity,
        app_type_registry: &AppTypeRegistry,
    ) -> Result<ScenePatch, DynamicBsnLoaderError> {
        let Some(patches) = self.0.get::<BsnPatches>(patches_id) else {
            return Err(DynamicBsnLoaderError::NoSuchAstNode);
        };
        let mut scene_patches: Vec<_> = patches
            .0
            .iter()
            .map(|patch_id| self.convert_bsn_patch_to_patch(*patch_id, app_type_registry))
            .collect::<Result<Vec<_>, _>>()?;
        let dependencies: Vec<_> = scene_patches
            .iter_mut()
            .flat_map(|scene_patch| mem::take(&mut scene_patch.dependencies))
            .collect();
        Ok(ScenePatch {
            scene: Box::new(MultiPatch(
                scene_patches
                    .into_iter()
                    .map(|scene_patch| scene_patch.scene)
                    .collect(),
            )),
            dependencies,
            resolved: None,
        })
    }

    fn convert_bsn_patch_to_patch(
        &self,
        patch_id: Entity,
        app_type_registry: &AppTypeRegistry,
    ) -> Result<ScenePatch, DynamicBsnLoaderError> {
        let Some(patch) = self.0.get::<BsnPatch>(patch_id) else {
            return Err(DynamicBsnLoaderError::NoSuchAstNode);
        };

        let patch = match *patch {
            BsnPatch::Name(ref name, index) => Box::new(NameEntityReference {
                name: Name(name.clone().into()),
                index,
            }) as Box<dyn Scene>,

            BsnPatch::Base(ref base) => {
                Box::new(InheritSceneAsset::from(base.clone())) as Box<dyn Scene>
            }

            BsnPatch::Var(BsnVar(ref symbol, is_template)) => {
                let symbol = symbol.clone();

                let type_registry = app_type_registry.read();
                let resolved_symbol =
                    symbol.resolve_type_or_enum_variant_to_template(&type_registry, is_template)?;

                let app_type_registry = app_type_registry.clone();

                Box::new(ErasedTemplatePatch {
                    template_type_id: resolved_symbol.template_type_id,
                    app_type_registry: app_type_registry.clone(),
                    fun: move |reflect, _context| {
                        // This could be an enum variant
                        // (`some_crate::Enum::Variant`) or a unit struct.
                        if !resolved_symbol.template_is_enum {
                            // This is a unit struct. It should already be instantiated.
                            return;
                        }

                        let ReflectMut::Enum(enum_reflect) = reflect.reflect_mut() else {
                            error!("Expected an enum");
                            return;
                        };

                        let dynamic_enum = DynamicEnum::new(symbol.1.clone(), DynamicVariant::Unit);
                        enum_reflect.apply(&dynamic_enum);
                    },
                }) as Box<dyn Scene>
            }

            BsnPatch::Struct(BsnStruct(ref symbol, ref fields, is_template)) => {
                let symbol = symbol.clone();

                let type_registry = app_type_registry.read();
                let resolved_symbol =
                    symbol.resolve_type_or_enum_variant_to_template(&type_registry, is_template)?;

                let template_type_registration =
                    type_registry.get(resolved_symbol.template_type_id).unwrap();
                let field_infos = if let Ok(structure) =
                    template_type_registration.type_info().as_struct()
                {
                    StructOrStructVariant::Struct(structure)
                } else if let Ok(enumeration) = template_type_registration.type_info().as_enum() {
                    StructOrStructVariant::StructVariant(
                        enumeration
                            .variant(&symbol.1)
                            .unwrap()
                            .as_struct_variant()?,
                    )
                } else {
                    return Err(DynamicBsnLoaderError::TypeNotStruct);
                };

                let mut dynamic_struct = DynamicStruct::default();
                for field in fields.iter() {
                    let Some(field_info) = field_infos.get(&field.0) else {
                        return Err(DynamicBsnLoaderError::StructDoesntHaveField(
                            field.0.clone(),
                        ));
                    };
                    let reflect = self.convert_bsn_expr_to_reflect(
                        field.1,
                        app_type_registry,
                        field_info.ty().id(),
                    )?;
                    dynamic_struct.insert_boxed(field.0.clone(), reflect);
                }

                let app_type_registry = app_type_registry.clone();

                Box::new(ErasedTemplatePatch {
                    template_type_id: resolved_symbol.template_type_id,
                    app_type_registry: app_type_registry.clone(),
                    fun: move |reflect, _context| {
                        // This could be an enum variant
                        // (`some_crate::Enum::Variant`) or a unit struct.
                        // First, look for a struct.
                        let struct_type_path = symbol.as_path();
                        if !resolved_symbol.template_is_enum {
                            // This is a struct.
                            let ReflectMut::Struct(reflect_struct) = reflect.reflect_mut() else {
                                error!("Expected a struct: `{}`", struct_type_path);
                                return;
                            };

                            reflect_struct.apply(&dynamic_struct);
                            return;
                        }

                        // TODO: struct-like enum variants. Might need to
                        // convert the `DynamicStruct` into a `DynamicEnum`
                        // which should be doable
                        error!("Unknown type: `{}`", struct_type_path);
                    },
                }) as Box<dyn Scene>
            }

            BsnPatch::NamedTuple(BsnNamedTuple(ref symbol, ref fields, is_template)) => {
                let symbol = symbol.clone();

                let type_registry = app_type_registry.read();
                let resolved_symbol =
                    symbol.resolve_type_or_enum_variant_to_template(&type_registry, is_template)?;

                let template_type_registration =
                    type_registry.get(resolved_symbol.template_type_id).unwrap();
                let field_infos = if let Ok(tuple_struct) =
                    template_type_registration.type_info().as_tuple_struct()
                {
                    tuple_struct.iter()
                } else if let Ok(enumeration) = template_type_registration.type_info().as_enum() {
                    enumeration
                        .variant(&symbol.1)
                        .unwrap()
                        .as_tuple_variant()?
                        .iter()
                } else {
                    return Err(DynamicBsnLoaderError::TypeNotNamedTuple);
                };

                let mut dynamic_tuple_struct = DynamicTupleStruct::default();
                for (field, field_info) in fields.iter().zip(field_infos) {
                    let reflect = self.convert_bsn_expr_to_reflect(
                        *field,
                        app_type_registry,
                        field_info.ty().id(),
                    )?;
                    dynamic_tuple_struct.insert_boxed(reflect);
                }

                let app_type_registry = app_type_registry.clone();

                Box::new(ErasedTemplatePatch {
                    template_type_id: resolved_symbol.template_type_id,
                    app_type_registry: app_type_registry.clone(),
                    fun: move |reflect, _context| {
                        // This could be an enum variant
                        // (`some_crate::Enum::Variant`) or a tuple struct.
                        // First, look for a struct.
                        let struct_type_path = symbol.as_path();
                        if !resolved_symbol.template_is_enum {
                            // This is a struct.
                            let ReflectMut::TupleStruct(reflect_tuple_struct) =
                                reflect.reflect_mut()
                            else {
                                error!("Expected a tuple struct: `{}`", struct_type_path);
                                return;
                            };

                            reflect_tuple_struct.apply(&dynamic_tuple_struct);
                            return;
                        }

                        // TODO: struct-like enum variants. Might need to
                        // convert the `DynamicStruct` into a `DynamicEnum`
                        // which should be doable
                        error!("Unknown type: `{}`", struct_type_path);
                    },
                }) as Box<dyn Scene>
            }

            BsnPatch::Relation(BsnRelation(ref relation_symbol, ref patches)) => {
                // FIXME: What a hack!
                if &*relation_symbol.as_path() != "bevy_ecs::hierarchy::Children" {
                    return Err(DynamicBsnLoaderError::OnlyChildRelationsSupported);
                }
                let related_template_list: Vec<_> = patches
                    .iter()
                    .map(|patches_id| {
                        // FIXME: seems fishy to throw away dependencies like this
                        Ok(self
                            .convert_bsn_patches_to_patch(*patches_id, app_type_registry)?
                            .scene)
                    })
                    .collect::<Result<Vec<_>, DynamicBsnLoaderError>>()?;
                Box::new(DynamicRelatedScenes {
                    relationship: TypeId::of::<ChildOf>(),
                    related_template_list,
                }) as Box<dyn Scene>
            }
        };

        Ok(ScenePatch {
            scene: patch,
            dependencies: vec![],
            resolved: None,
        })
    }

    fn convert_bsn_expr_to_reflect(
        &self,
        expr_id: Entity,
        app_type_registry: &AppTypeRegistry,
        expected_template_type: TypeId,
    ) -> Result<Box<dyn PartialReflect>, DynamicBsnLoaderError> {
        let Some(expr) = self.0.get::<BsnExpr>(expr_id) else {
            return Err(DynamicBsnLoaderError::NoSuchAstNode);
        };

        let type_registry = app_type_registry.read();

        match *expr {
            BsnExpr::Var(BsnVar(ref symbol, is_template)) => {
                let resolved_symbol =
                    symbol.resolve_type_or_enum_variant_to_template(&type_registry, is_template)?;

                let template_type_registration =
                    type_registry.get(resolved_symbol.template_type_id).unwrap();

                let mut reflect =
                    create_reflect_default_from_type_registration(template_type_registration)?;

                // This could be an enum variant
                // (`some_crate::Enum::Variant`) or a unit struct.
                if !resolved_symbol.template_is_enum {
                    // This is a unit struct. Just instantiate it.
                    return Ok(reflect.into_partial_reflect());
                }

                // This is a unit enum variant.
                let ReflectMut::Enum(enum_reflect) = reflect.reflect_mut() else {
                    return Err(DynamicBsnLoaderError::UnknownType(
                        template_type_registration
                            .type_info()
                            .type_path()
                            .to_owned(),
                    ));
                };

                let dynamic_enum = DynamicEnum::new(symbol.1.clone(), DynamicVariant::Unit);
                enum_reflect.apply(&dynamic_enum);
                Ok(reflect.into_partial_reflect())
            }

            BsnExpr::Struct(ref bsn_struct) => {
                let resolved_symbol = bsn_struct
                    .0
                    .resolve_type_or_enum_variant_to_template(&type_registry, bsn_struct.2)?;

                let template_type_registration =
                    type_registry.get(resolved_symbol.template_type_id).unwrap();
                let mut reflect =
                    create_reflect_default_from_type_registration(template_type_registration)?;

                // This could be an enum variant (`some_crate::Enum::Variant`)
                // or a struct.
                if !resolved_symbol.template_is_enum {
                    // This is a struct.
                    let ReflectMut::Struct(reflect_struct) = reflect.reflect_mut() else {
                        return Err(DynamicBsnLoaderError::UnknownType(
                            template_type_registration
                                .type_info()
                                .type_path()
                                .to_owned(),
                        ));
                    };

                    let Ok(struct_info) = template_type_registration.type_info().as_struct() else {
                        return Err(DynamicBsnLoaderError::TypeNotStruct);
                    };

                    let mut dynamic_struct = DynamicStruct::default();
                    for field in &bsn_struct.1 {
                        let Some(field_info) = struct_info.field(&field.0) else {
                            return Err(DynamicBsnLoaderError::StructDoesntHaveField(
                                field.0.clone(),
                            ));
                        };
                        let reflect = self.convert_bsn_expr_to_reflect(
                            field.1,
                            app_type_registry,
                            field_info.ty().id(),
                        )?;
                        dynamic_struct.insert_boxed(field.0.clone(), reflect);
                    }
                    reflect_struct.apply(&dynamic_struct);
                    return Ok(reflect.into_partial_reflect());
                }

                // TODO: struct-like enum variants. Might need to
                // convert the `DynamicStruct` into a `DynamicEnum`
                // which should be doable
                Err(DynamicBsnLoaderError::UnknownType(
                    template_type_registration
                        .type_info()
                        .type_path()
                        .to_owned(),
                ))
            }

            BsnExpr::NamedTuple(ref named_tuple) => {
                let resolved_symbol = named_tuple
                    .0
                    .resolve_type_or_enum_variant_to_template(&type_registry, named_tuple.2)?;

                let template_type_registration =
                    type_registry.get(resolved_symbol.template_type_id).unwrap();
                let mut reflect =
                    create_reflect_default_from_type_registration(template_type_registration)?;

                let Ok(tuple_info) = template_type_registration.type_info().as_tuple_struct()
                else {
                    return Err(DynamicBsnLoaderError::TypeNotNamedTuple);
                };

                let mut dynamic_tuple_struct = DynamicTupleStruct::default();
                for (field_id, field_info) in named_tuple.1.iter().zip(tuple_info.iter()) {
                    let reflect_val = self.convert_bsn_expr_to_reflect(
                        *field_id,
                        app_type_registry,
                        field_info.ty().id(),
                    )?;
                    dynamic_tuple_struct.insert_boxed(reflect_val);
                }

                if let ReflectMut::TupleStruct(ts) = reflect.reflect_mut() {
                    ts.apply(&dynamic_tuple_struct);
                }
                Ok(reflect.into_partial_reflect())
            }

            BsnExpr::StringLit(ref string) => {
                let expected_type_registration = type_registry.get(expected_template_type).unwrap();
                let mut reflect =
                    create_reflect_default_from_type_registration(expected_type_registration)?;

                // TODO: Support `&str`, `Cow<str>`, `Arc<str>`, etc. too?
                if expected_template_type == TypeId::of::<String>() {
                    reflect.apply(string);
                    return Ok(reflect.into_partial_reflect());
                }

                // FIXME: This is a total hack. We should have a generic
                // `ReflectConvert` or `ReflectFrom` or something.
                if expected_type_registration
                    .type_info()
                    .type_path()
                    .starts_with("bevy_asset::handle::HandleTemplate<")
                {
                    let asset_path: AssetPath<'static> = AssetPath::parse(string).into_owned();
                    let ReflectMut::Enum(reflect_enum) = reflect.reflect_mut() else {
                        panic!("`HandleTemplate` wasn't an enum")
                    };
                    // `HandleTemplate::Path` is the default, so we don't have
                    // to set it.
                    reflect_enum.field_at_mut(0).unwrap().apply(&asset_path);
                    return Ok(reflect.into_partial_reflect());
                }

                Err(DynamicBsnLoaderError::TypeMismatch)
            }

            BsnExpr::FloatLit(float_lit) => {
                let mut reflect = create_reflect_default(&type_registry, expected_template_type)?;

                if expected_template_type == TypeId::of::<f32>() {
                    reflect.apply(&(float_lit as f32));
                    return Ok(reflect.into_partial_reflect());
                }
                if expected_template_type == TypeId::of::<f64>() {
                    reflect.apply(&float_lit);
                    return Ok(reflect.into_partial_reflect());
                }
                Err(DynamicBsnLoaderError::FloatLitTypeMismatch)
            }

            BsnExpr::BoolLit(bool_lit) => {
                let mut reflect = create_reflect_default(&type_registry, expected_template_type)?;

                if expected_template_type == TypeId::of::<bool>() {
                    reflect.apply(&bool_lit);
                    return Ok(reflect.into_partial_reflect());
                }
                Err(DynamicBsnLoaderError::TypeMismatch)
            }

            BsnExpr::List(ref items) => {
                let type_registration =
                    type_registry.get(expected_template_type).ok_or_else(|| {
                        DynamicBsnLoaderError::UnknownType(format!(
                            "TypeId {:?}",
                            expected_template_type
                        ))
                    })?;
                let list_info = type_registration
                    .type_info()
                    .as_list()
                    .map_err(|_| DynamicBsnLoaderError::TypeMismatch)?;
                let item_type_id = list_info.item_ty().id();

                let mut dynamic_list = DynamicList::default();
                for &item_id in items {
                    let reflect =
                        self.convert_bsn_expr_to_reflect(item_id, app_type_registry, item_type_id)?;
                    dynamic_list.push_box(reflect);
                }
                dynamic_list.set_represented_type(Some(type_registration.type_info()));
                Ok(Box::new(dynamic_list) as Box<dyn PartialReflect>)
            }

            BsnExpr::IntLit(int_lit) => {
                let mut reflect = create_reflect_default(&type_registry, expected_template_type)?;

                if expected_template_type == TypeId::of::<i8>() {
                    reflect.apply(&(int_lit as i8));
                    return Ok(reflect.into_partial_reflect());
                }
                if expected_template_type == TypeId::of::<u8>() {
                    reflect.apply(&(int_lit as u8));
                    return Ok(reflect.into_partial_reflect());
                }
                if expected_template_type == TypeId::of::<i16>() {
                    reflect.apply(&(int_lit as i16));
                    return Ok(reflect.into_partial_reflect());
                }
                if expected_template_type == TypeId::of::<u16>() {
                    reflect.apply(&(int_lit as u16));
                    return Ok(reflect.into_partial_reflect());
                }
                if expected_template_type == TypeId::of::<i32>() {
                    reflect.apply(&(int_lit as i32));
                    return Ok(reflect.into_partial_reflect());
                }
                if expected_template_type == TypeId::of::<u32>() {
                    reflect.apply(&(int_lit as u32));
                    return Ok(reflect.into_partial_reflect());
                }
                if expected_template_type == TypeId::of::<i64>() {
                    reflect.apply(&(int_lit as i64));
                    return Ok(reflect.into_partial_reflect());
                }
                if expected_template_type == TypeId::of::<u64>() {
                    reflect.apply(&(int_lit as u64));
                    return Ok(reflect.into_partial_reflect());
                }
                if expected_template_type == TypeId::of::<isize>() {
                    reflect.apply(&(int_lit as isize));
                    return Ok(reflect.into_partial_reflect());
                }
                if expected_template_type == TypeId::of::<usize>() {
                    reflect.apply(&(int_lit as usize));
                    return Ok(reflect.into_partial_reflect());
                }
                Err(DynamicBsnLoaderError::IntLitTypeMismatch)
            }
        }
    }

    pub fn create_patches(&mut self, patches: Vec<Entity>) -> Entity {
        self.0.spawn(BsnPatches(patches)).id()
    }

    pub fn create_patch(&mut self, patch: BsnPatch) -> Entity {
        self.0.spawn(patch).id()
    }

    pub fn create_expr(&mut self, expr: BsnExpr) -> Entity {
        self.0.spawn(expr).id()
    }

    pub fn create_name_patch(&mut self, name: String) -> Entity {
        let mut name_store = self.0.resource_mut::<BsnNameStore>();
        let index = match name_store.name_indices.get(&*name) {
            Some(index) => *index,
            None => {
                let index = name_store.next_name_index;
                name_store.next_name_index += 1;
                name_store.name_indices.insert(name.clone(), index);
                index
            }
        };
        self.create_patch(BsnPatch::Name(name, index))
    }
}

fn create_reflect_default(
    type_registry: &TypeRegistry,
    expected_template_type: TypeId,
) -> Result<Box<dyn Reflect>, DynamicBsnLoaderError> {
    let expected_type_registration = type_registry.get(expected_template_type).unwrap();
    create_reflect_default_from_type_registration(expected_type_registration)
}

fn create_reflect_default_from_type_registration(
    expected_type_registration: &TypeRegistration,
) -> Result<Box<dyn Reflect>, DynamicBsnLoaderError> {
    let Some(reflect_default) = expected_type_registration.data::<ReflectDefault>() else {
        return Err(DynamicBsnLoaderError::TypeDoesntImplementDefault(
            expected_type_registration
                .type_info()
                .type_path()
                .to_owned(),
        ));
    };
    Ok(reflect_default.default())
}

pub struct MultiPatch(Vec<Box<dyn Scene>>);

impl Scene for MultiPatch {
    fn resolve(
        &self,
        context: &mut ResolveContext,
        scene: &mut ResolvedScene,
    ) -> Result<(), ResolveSceneError> {
        for subscene in self.0.iter() {
            subscene.resolve(context, scene)?;
        }

        Ok(())
    }

    fn register_dependencies(&self, dependencies: &mut SceneDependencies) {
        for subscene in self.0.iter() {
            subscene.register_dependencies(dependencies);
        }
    }
}

pub struct DynamicRelatedScenes {
    relationship: TypeId,
    related_template_list: Vec<Box<dyn Scene>>,
}

impl Scene for DynamicRelatedScenes {
    fn resolve(
        &self,
        context: &mut ResolveContext,
        scene: &mut ResolvedScene,
    ) -> Result<(), ResolveSceneError> {
        if self.relationship != TypeId::of::<ChildOf>() {
            return Err(ResolveSceneError::UnsupportedRelationship);
        }

        let related = scene.related.entry(self.relationship).or_insert_with(|| {
            RelatedResolvedScenes {
                scenes: vec![],
                insert: |entity, target| {
                    // TODO: There should probably be a `ReflectRelationship`
                    let child_of = ChildOf(target);
                    entity.insert(child_of);
                },
                relationship_name: "ChildOf",
            }
        });

        for scene in self.related_template_list.iter() {
            let mut resolved_scene = ResolvedScene::default();
            scene.resolve(context, &mut resolved_scene)?;
            related.scenes.push(resolved_scene);
        }

        Ok(())
    }

    fn register_dependencies(&self, dependencies: &mut SceneDependencies) {
        for scene in self.related_template_list.iter() {
            scene.register_dependencies(dependencies);
        }
    }
}

impl BsnSymbol {
    fn resolve_type_or_enum_variant_to_template(
        &self,
        type_registry: &TypeRegistry,
        is_template: bool,
    ) -> Result<ResolvedSymbol, DynamicBsnLoaderError> {
        // First, look for a unit struct.
        let unit_struct_type_path = self.as_path();
        if let Some(type_registration) = type_registry.get_with_type_path(&unit_struct_type_path) {
            return Ok(ResolvedSymbol::new(type_registration, false, is_template));
        }

        // Next, look for a unit enum variant.
        let Some(enum_type_path) = self.as_path_skip_last() else {
            return Err(DynamicBsnLoaderError::UnknownType(
                unit_struct_type_path.to_owned(),
            ));
        };
        let Some(type_registration) = type_registry.get_with_type_path(&enum_type_path) else {
            return Err(DynamicBsnLoaderError::UnknownType(
                enum_type_path.to_owned(),
            ));
        };
        Ok(ResolvedSymbol::new(type_registration, true, is_template))
    }

    pub(crate) fn as_path(&self) -> String {
        let mut path = String::new();
        for component in &self.0 {
            let _ = write!(&mut path, "{}::", &**component);
        }
        path.push_str(&self.1);
        path
    }

    fn as_path_skip_last(&self) -> Option<String> {
        if self.0.is_empty() {
            return None;
        }
        let mut enum_type_path = String::new();
        for component_index in 0..(self.0.len() - 1) {
            let _ = write!(&mut enum_type_path, "{}::", self.0[component_index]);
        }
        enum_type_path.push_str(self.0.last().unwrap());
        Some(enum_type_path)
    }
}

struct ErasedTemplatePatch<F>
where
    F: Fn(&mut dyn PartialReflect, &mut ResolveContext),
{
    pub fun: F,
    pub template_type_id: TypeId,
    // FIXME: Not a good place for this. Put it in the patch context instead?
    pub app_type_registry: AppTypeRegistry,
}

struct DefaultDynamicErasedTemplate(Box<dyn Reflect>);

impl<F> Scene for ErasedTemplatePatch<F>
where
    F: Fn(&mut dyn PartialReflect, &mut ResolveContext) + Send + Sync + 'static,
{
    fn resolve(
        &self,
        context: &mut ResolveContext,
        scene: &mut ResolvedScene,
    ) -> Result<(), ResolveSceneError> {
        let template_type_id = self.template_type_id;
        let app_type_registry = self.app_type_registry.clone();

        // Verify that everything is OK before we enter the closure below and
        // start unwrapping things.
        {
            let type_registry = app_type_registry.read();
            let Some(template_type_registration) = type_registry.get(template_type_id) else {
                return Err(ResolveSceneError::TypeNotReflectable);
            };
            if !template_type_registration.contains::<ReflectDefault>() {
                return Err(ResolveSceneError::TypeDoesntReflectDefault);
            };
        }

        let template =
            scene.get_or_insert_erased_template(context, self.template_type_id, move || {
                let reflect = {
                    let type_registry = app_type_registry.read();
                    let type_registration = type_registry.get(template_type_id).unwrap();
                    let reflect_default = type_registration.data::<ReflectDefault>().unwrap();
                    reflect_default.default()
                };
                Box::new(DefaultDynamicErasedTemplate(reflect))
            });
        let Some(reflect) = template.try_as_partial_reflect_mut() else {
            return Err(ResolveSceneError::TypeNotReflectable);
        };
        (self.fun)(reflect, context);

        Ok(())
    }
}

impl ErasedTemplate for DefaultDynamicErasedTemplate {
    fn apply(&self, context: &mut TemplateContext) -> EcsResult<(), BevyError> {
        let maybe_build_template = {
            let app_type_registry = context.resource::<AppTypeRegistry>();
            let type_registry = app_type_registry.read();
            let Some(template_type_registration) = type_registry.get(self.0.as_any().type_id())
            else {
                return Err("Template type wasn't registered".into());
            };
            template_type_registration
                .data::<ReflectTemplate>()
                .map(|reflect_template| reflect_template.build_template)
        };

        // If the template type supports `ReflectTemplate`, then call its build
        // function. Otherwise, just clone it, under the assumption that the
        // template type is the output type.
        //
        // FIXME: This is undoubtedly convenient, but it might not be the right
        // thing to do. It feels a bit dodgy.
        let output = match maybe_build_template {
            Some(build_template) => build_template(&*self.0, context)?,
            None => self.0.reflect_clone()?,
        };

        context.entity.insert_reflect(output.into_partial_reflect());
        Ok(())
    }

    fn clone_template(&self) -> Box<dyn ErasedTemplate> {
        Box::new(DefaultDynamicErasedTemplate(
            self.0.reflect_clone().unwrap(),
        ))
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self.0.as_any_mut()
    }

    fn try_as_partial_reflect_mut(&mut self) -> Option<&mut dyn PartialReflect> {
        Some(self.0.as_partial_reflect_mut())
    }
}

#[derive(Clone)]
struct ResolvedSymbol {
    template_type_id: TypeId,
    template_is_enum: bool,
}

impl ResolvedSymbol {
    fn new(
        type_registration: &TypeRegistration,
        template_is_enum: bool,
        is_template: bool,
    ) -> ResolvedSymbol {
        if is_template {
            return ResolvedSymbol {
                template_type_id: type_registration.type_id(),
                template_is_enum,
            };
        }

        // Fetch the template type, if available. Otherwise, we assume the
        // `FromTemplate` type is the same as the `Template` type (which it
        // will be for clonable, `Default`able things).
        ResolvedSymbol {
            template_type_id: match type_registration.data::<ReflectFromTemplate>() {
                Some(reflect_get_template) => reflect_get_template.template_type_id,
                None => type_registration.type_id(),
            },
            template_is_enum,
        }
    }
}

enum StructOrStructVariant<'a> {
    Struct(&'a StructInfo),
    StructVariant(&'a StructVariantInfo),
}

impl<'a> StructOrStructVariant<'a> {
    fn get(&self, field_name: &str) -> Option<&'a NamedField> {
        match *self {
            StructOrStructVariant::Struct(structure) => structure.field(field_name),
            StructOrStructVariant::StructVariant(struct_variant) => {
                struct_variant.field(field_name)
            }
        }
    }
}
