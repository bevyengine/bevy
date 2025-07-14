use proc_macro2::TokenStream;
use syn::{punctuated::Punctuated, Block, Expr, Ident, Lit, LitStr, Path, Token};

#[derive(Debug)]
pub struct BsnRoot(pub Bsn<true>);

#[derive(Debug)]
pub struct Bsn<const ALLOW_FLAT: bool> {
    pub entries: Vec<BsnEntry>,
}

#[derive(Debug)]
pub enum BsnEntry {
    Name(Ident),
    GetTemplatePatch(BsnType),
    TemplatePatch(BsnType),
    GetTemplateConstructor(BsnConstructor),
    TemplateConstructor(BsnConstructor),
    TemplateConst { type_path: Path, const_ident: Ident },
    SceneExpression(TokenStream),
    InheritedScene(BsnInheritedScene),
    RelatedSceneList(BsnRelatedSceneList),
    ChildrenSceneList(BsnSceneList),
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
    Expression(Block),
}

#[derive(Debug)]
pub enum BsnInheritedScene {
    Asset(LitStr),
    Fn {
        function: Ident,
        args: Option<Punctuated<Expr, Token![,]>>,
    },
}

#[derive(Debug)]
pub struct BsnConstructor {
    pub type_path: Path,
    pub function: Ident,
    pub args: Option<Punctuated<Expr, Token![,]>>,
}

#[derive(Debug)]
pub enum BsnFields {
    Named(Vec<BsnNamedField>),
    Tuple(Vec<BsnUnnamedField>),
}

#[derive(Debug)]
pub struct BsnTuple(pub Vec<BsnValue>);

#[derive(Debug)]
pub struct BsnNamedField {
    pub name: Ident,
    /// This is an Option to enable autocomplete when the field name is being typed
    /// To improve autocomplete further we'll need to forgo a lot of the syn parsing
    pub value: Option<BsnValue>,
    pub is_template: bool,
}

#[derive(Debug)]
pub struct BsnUnnamedField {
    pub value: BsnValue,
    pub is_template: bool,
}

#[derive(Debug)]
pub enum BsnValue {
    Expr(TokenStream),
    Closure(TokenStream),
    Ident(Ident),
    Lit(Lit),
    Type(BsnType),
    Tuple(BsnTuple),
}
