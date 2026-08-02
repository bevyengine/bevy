use proc_macro2::TokenStream;
use syn::{Ident, Lit, LitStr, Path, Stmt};

#[derive(Debug)]
pub struct BsnRoot(pub Bsn<true>);

#[derive(Debug)]
pub struct BsnListRoot(pub BsnSceneListItems);

#[derive(Debug)]
pub struct Bsn<const ALLOW_FLAT: bool> {
    pub entries: Vec<BsnEntry>,
}

#[derive(Debug)]
pub enum BsnEntry {
    Name(Ident),
    FromTemplatePatch(BsnType),
    TemplatePatch(BsnType),
    FromTemplateConstructor(BsnConstructor),
    TemplateConstructor(BsnConstructor),
    TemplateConst { type_path: Path, const_ident: Ident },
    UncachedScene(BsnScene),
    CachedScene(BsnScene),
    RelatedSceneList(BsnRelatedSceneList),
}

#[derive(Debug)]
pub struct BsnType {
    pub path: Path,
    pub enum_variant: Option<Ident>,
    pub fields: BsnFields,
}

#[derive(Debug)]
pub struct BsnRelatedSceneList {
    pub relationship_path: Path,
    pub scene_list: BsnSceneList,
}

#[derive(Debug)]
pub struct BsnSceneList(pub BsnSceneListItems);

#[derive(Debug)]
pub struct BsnSceneListItems(pub Vec<BsnSceneListItem>);

#[derive(Debug)]
pub enum BsnSceneListItem {
    Scene(Bsn<true>),
    Expression(Vec<Stmt>),
}

#[derive(Debug)]
pub struct BsnSceneFn {
    pub path: Path,
    pub args: BsnFnArgs,
}

#[derive(Debug)]
pub enum BsnScene {
    Asset(LitStr),
    Fn(BsnSceneFn),
    SceneComponent(BsnType),
    Expression(TokenStream),
}

#[derive(Debug)]
pub struct BsnConstructor {
    pub type_path: Path,
    pub function: Ident,
    pub args: BsnFnArgs,
}

#[derive(Debug)]
pub enum BsnFields {
    Named(Vec<BsnNamedField>),
    Tuple(Vec<BsnUnnamedField>),
}
impl BsnFields {
    pub fn len(&self) -> usize {
        match self {
            BsnFields::Named(vec) => vec.len(),
            BsnFields::Tuple(vec) => vec.len(),
        }
    }
}

#[derive(Debug)]
pub struct BsnTuple(pub Vec<BsnValue>);

#[derive(Debug)]
pub struct BsnNamedField {
    pub is_prop: bool,
    /// This is a `Struct { field }` shorthand for `Struct { field: field }`
    pub is_name_shorthand: bool,
    pub name: Ident,
    /// This is an Option to enable autocomplete when the field name is being typed
    /// To improve autocomplete further we'll need to forgo a lot of the syn parsing
    pub value: Option<BsnValue>,
}

#[derive(Debug)]
pub struct BsnUnnamedField {
    pub value: BsnValue,
}

#[derive(Debug)]
pub enum BsnValue {
    Expr(TokenStream),
    Closure(TokenStream),
    Ident(Ident),
    Lit(Lit),
    Type(BsnType),
    Tuple(BsnTuple),
    Name(Ident),
}

#[derive(Debug)]
pub enum BsnFnArg {
    EntityName(Ident),
    Tokens(TokenStream),
}

#[derive(Debug)]
pub struct BsnFnArgs(pub Vec<BsnFnArg>);
