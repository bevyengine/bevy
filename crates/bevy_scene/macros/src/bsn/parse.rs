use crate::bsn::types::{
    Bsn, BsnConstructor, BsnEntry, BsnFields, BsnInheritedScene, BsnListRoot, BsnNamedField,
    BsnRelatedSceneList, BsnRoot, BsnSceneList, BsnSceneListItem, BsnSceneListItems, BsnTuple,
    BsnType, BsnUnnamedField, BsnValue,
};
use bevy_macro_utils::{path_to_string, PathType};
use proc_macro2::{Delimiter, TokenStream, TokenTree};
use quote::quote;
use syn::{
    braced, bracketed,
    buffer::Cursor,
    parenthesized,
    parse::{Parse, ParseBuffer, ParseStream},
    spanned::Spanned,
    token::{At, Brace, Bracket, Colon, Comma, Paren},
    Block, Expr, Ident, Lit, LitStr, Path, Result, Token,
};

/// Functionally identical to [`Punctuated`](syn::punctuated::Punctuated), but fills the given `$list` Vec instead
/// of allocating a new one inside [`Punctuated`](syn::punctuated::Punctuated). This exists to avoid allocating an intermediate Vec.
///
/// This also attempts to parse $parse a second time _before_ parsing $separator, as this enables autocomplete to work in cases where
/// it is being typed in the middle of a list
macro_rules! parse_punctuated_vec_autocomplete_friendly {
    ($list:ident, $input:ident, $parse:ident, $separator:ident) => {
        loop {
            if $input.is_empty() {
                break;
            }
            let value = $input.parse::<$parse>()?;
            $list.push(value);
            if $input.is_empty() {
                break;
            }

            // Try parsing without a comma separator first. This makes autocomplete
            // work in more places
            if !$input.is_empty() && !$input.peek($separator) {
                let value = $input.parse::<$parse>()?;
                $list.push(value);
            }
            $input.parse::<$separator>()?;
        }
    };
}

impl Parse for BsnRoot {
    fn parse(input: ParseStream) -> Result<Self> {
        Ok(BsnRoot(input.parse::<Bsn<true>>()?))
    }
}

impl Parse for BsnListRoot {
    fn parse(input: ParseStream) -> Result<Self> {
        Ok(BsnListRoot(input.parse::<BsnSceneListItems>()?))
    }
}

impl<const ALLOW_FLAT: bool> Parse for Bsn<ALLOW_FLAT> {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut entries = Vec::new();
        let mut found_inherited_scene = false;
        if input.peek(Paren) {
            let content;
            parenthesized![content in input];
            while !content.is_empty() {
                let entry = BsnEntry::parse(&content, found_inherited_scene)?;
                if matches!(entry, BsnEntry::InheritedScene(_)) {
                    found_inherited_scene = true;
                }
                entries.push(entry);
            }
        } else if ALLOW_FLAT {
            while !input.is_empty() {
                let entry = BsnEntry::parse(input, found_inherited_scene)?;
                if matches!(entry, BsnEntry::InheritedScene(_)) {
                    found_inherited_scene = true;
                }
                entries.push(entry);
                if input.peek(Comma) {
                    // Not ideal, but this anticipatory break allows us to parse non-parenthesized
                    // flat Bsn entries in SceneLists
                    break;
                }
            }
        } else {
            entries.push(BsnEntry::parse(input, found_inherited_scene)?);
        }

        Ok(Self { entries })
    }
}

impl BsnEntry {
    fn parse(input: ParseStream, found_inherited_scene: bool) -> Result<Self> {
        Ok(if input.peek(Token![:]) {
            BsnEntry::InheritedScene(BsnInheritedScene::parse(input, found_inherited_scene)?)
        } else if input.peek(Token![#]) {
            input.parse::<Token![#]>()?;
            if input.peek(Brace) {
                BsnEntry::NameExpression(braced_tokens(input)?)
            } else {
                BsnEntry::Name(input.parse::<Ident>()?)
            }
        } else if input.peek(Brace) {
            BsnEntry::SceneExpression(braced_tokens(input)?)
        } else {
            let is_template = input.peek(At);
            if is_template {
                input.parse::<At>()?;
            }
            let mut path = input.parse::<Path>()?;
            let path_type = PathType::new(&path);
            match path_type {
                PathType::Type | PathType::Enum => {
                    let enum_variant = if matches!(path_type, PathType::Enum) {
                        take_last_path_ident(&mut path)
                    } else {
                        None
                    };
                    if input.peek(Bracket) {
                        // TODO: fail if this is an enum variant
                        BsnEntry::RelatedSceneList(BsnRelatedSceneList {
                            relationship_path: path,
                            scene_list: input.parse::<BsnSceneList>()?,
                        })
                    } else {
                        let fields = input.parse::<BsnFields>()?;
                        let bsn_type = BsnType {
                            path,
                            enum_variant,
                            fields,
                        };
                        if is_template {
                            BsnEntry::TemplatePatch(bsn_type)
                        } else {
                            BsnEntry::FromTemplatePatch(bsn_type)
                        }
                    }
                }
                PathType::TypeConst => {
                    let const_ident = take_last_path_ident(&mut path).unwrap();
                    BsnEntry::TemplateConst {
                        type_path: path,
                        const_ident,
                    }
                }
                PathType::Const => {
                    return Err(syn::Error::new(
                        path.span(),
                        "Consts are not currently supported in this position",
                    ))
                }
                PathType::TypeFunction => {
                    let function = take_last_path_ident(&mut path).unwrap();
                    let args = if input.peek(Paren) {
                        let content;
                        parenthesized!(content in input);
                        Some(content.parse_terminated(Expr::parse, Token![,])?)
                    } else {
                        None
                    };

                    let bsn_constructor = BsnConstructor {
                        type_path: path,
                        function,
                        args,
                    };
                    if is_template {
                        BsnEntry::TemplateConstructor(bsn_constructor)
                    } else {
                        BsnEntry::FromTemplateConstructor(bsn_constructor)
                    }
                }
                PathType::Function => {
                    if input.peek(Paren) {
                        let tokens = parenthesized_tokens(input)?;
                        BsnEntry::SceneExpression(quote! {#path(#tokens)})
                    } else {
                        BsnEntry::SceneExpression(quote! {#path})
                    }
                }
            }
        })
    }
}

impl Parse for BsnSceneList {
    fn parse(input: ParseStream) -> Result<Self> {
        let content;
        bracketed!(content in input);
        Ok(BsnSceneList(content.parse::<BsnSceneListItems>()?))
    }
}

impl Parse for BsnSceneListItems {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut scenes = Vec::new();
        parse_punctuated_vec_autocomplete_friendly!(scenes, input, BsnSceneListItem, Comma);
        Ok(BsnSceneListItems(scenes))
    }
}

impl Parse for BsnSceneListItem {
    fn parse(input: ParseStream) -> Result<Self> {
        Ok(if input.peek(Brace) {
            let block = input.parse::<Block>()?;
            BsnSceneListItem::Expression(block.stmts)
        } else {
            BsnSceneListItem::Scene(input.parse::<Bsn<true>>()?)
        })
    }
}

impl BsnInheritedScene {
    fn parse(input: ParseStream, found_inherited_scene: bool) -> Result<Self> {
        let colon = input.parse::<Token![:]>()?;
        if found_inherited_scene {
            return Err(syn::Error::new(
                colon.span(),
                "Cannot inherit scenes more than once",
            ));
        }
        Ok(if input.peek(LitStr) {
            let path = input.parse::<LitStr>()?;
            BsnInheritedScene::Asset(path)
        } else if input.peek(Brace) {
            BsnInheritedScene::Expression(braced_tokens(input)?)
        } else {
            // PERF: do we really need this fork here?
            let path = input.fork().parse::<Path>()?;
            match PathType::new(&path) {
                PathType::Type | PathType::Enum => {
                    BsnInheritedScene::Type(input.parse::<BsnType>()?)
                }
                PathType::Function | PathType::TypeFunction => {
                    let path = input.parse::<Path>()?;
                    let args = if input.peek(Paren) {
                        let content;
                        parenthesized!(content in input);
                        Some(content.parse_terminated(Expr::parse, Token![,])?)
                    } else {
                        None
                    };
                    BsnInheritedScene::Fn { path, args }
                }
                path_type => {
                    return Err(syn::Error::new(
                        path.span(),
                        format!(
                            "Cannot inherit from path {} of type {:?}",
                            path_to_string(&path),
                            path_type,
                        ),
                    ))
                }
            }
        })
    }
}

impl Parse for BsnType {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut path = input.parse::<Path>()?;
        let enum_variant = match PathType::new(&path) {
            PathType::Type => None,
            PathType::Enum => take_last_path_ident(&mut path),
            PathType::Function | PathType::TypeFunction => {
                return Err(syn::Error::new(
                    path.span(),
                    "Expected a path to a BSN type but encountered a path to a function.",
                ))
            }
            PathType::Const | PathType::TypeConst => {
                return Err(syn::Error::new(
                    path.span(),
                    "Expected a path to a BSN type but encountered a path to a const.",
                ))
            }
        };
        let fields = input.parse::<BsnFields>()?;
        Ok(BsnType {
            path,
            enum_variant,
            fields,
        })
    }
}

impl Parse for BsnTuple {
    fn parse(input: ParseStream) -> Result<Self> {
        let content;
        parenthesized![content in input];
        let mut fields = Vec::new();

        while !content.is_empty() {
            fields.push(content.parse::<BsnValue>()?);
        }

        Ok(BsnTuple(fields))
    }
}

impl Parse for BsnFields {
    fn parse(input: ParseStream) -> Result<Self> {
        Ok(if input.peek(Brace) {
            let content;
            braced![content in input];
            let mut fields = Vec::new();
            parse_punctuated_vec_autocomplete_friendly!(fields, content, BsnNamedField, Comma);
            BsnFields::Named(fields)
        } else if input.peek(Paren) {
            let content;
            parenthesized![content in input];
            let mut fields = Vec::new();
            parse_punctuated_vec_autocomplete_friendly!(fields, content, BsnUnnamedField, Comma);
            BsnFields::Tuple(fields)
        } else {
            BsnFields::Named(Vec::new())
        })
    }
}

impl Parse for BsnNamedField {
    fn parse(input: ParseStream) -> Result<Self> {
        let is_prop = if input.peek(At) {
            input.parse::<At>()?;
            true
        } else {
            false
        };
        let name = input.parse::<Ident>()?;
        let value = if input.peek(Colon) {
            input.parse::<Colon>()?;

            if input.is_empty() || input.peek(Comma) {
                None
            } else {
                Some(input.parse::<BsnValue>()?)
            }
        } else {
            None
        };
        Ok(BsnNamedField {
            name,
            value,
            is_prop,
        })
    }
}

impl Parse for BsnUnnamedField {
    fn parse(input: ParseStream) -> Result<Self> {
        let value = input.parse::<BsnValue>()?;
        Ok(BsnUnnamedField { value })
    }
}

/// Parse a closure "loosely" without caring about the tokens between `|...|` and `{...}`. This ensures autocomplete works.
fn parse_closure_loose(input: &ParseBuffer) -> Result<TokenStream> {
    let start = input.cursor();
    input.parse::<Token![|]>()?;
    let tokens = input.step(|cursor| {
        let mut rest = *cursor;
        while let Some((tt, next)) = rest.token_tree() {
            match &tt {
                TokenTree::Punct(punct) if punct.as_char() == '|' => {
                    if let Some((TokenTree::Group(group), next)) = next.token_tree()
                        && group.delimiter() == Delimiter::Brace
                    {
                        return Ok((tokens_between(start, next), next));
                    } else {
                        return Err(cursor.error("closures expect '{' to follow '|'"));
                    }
                }
                _ => rest = next,
            }
        }
        Err(cursor.error("no matching `|` was found after this point"))
    })?;
    Ok(tokens)
}

// Used to parse a block "loosely" without caring about the content in `{...}`. This ensures autocomplete works.
fn braced_tokens(input: &ParseBuffer) -> Result<TokenStream> {
    let content;
    braced!(content in input);
    content.parse::<TokenStream>()
}

// Used to parse parenthesized tokens "loosely" without caring about the content in `(...)`. This ensures autocomplete works.
fn parenthesized_tokens(input: &ParseBuffer) -> Result<TokenStream> {
    let content;
    parenthesized!(content in input);
    content.parse::<TokenStream>()
}

fn tokens_between(begin: Cursor, end: Cursor) -> TokenStream {
    assert!(begin <= end);
    let mut cursor = begin;
    let mut tokens = TokenStream::new();
    while cursor < end {
        let (token, next) = cursor.token_tree().unwrap();
        tokens.extend(std::iter::once(token));
        cursor = next;
    }
    tokens
}

impl Parse for BsnValue {
    fn parse(input: ParseStream) -> Result<Self> {
        Ok(if input.peek(Brace) {
            BsnValue::Expr(braced_tokens(input)?)
        } else if input.peek(Token![|]) {
            let tokens = parse_closure_loose(input)?;
            BsnValue::Closure(tokens)
        } else if input.peek(Ident) {
            let forked = input.fork();
            let path = forked.parse::<Path>()?;
            if path.segments.len() == 1 && (forked.is_empty() || forked.peek(Comma)) {
                return Ok(BsnValue::Ident(input.parse::<Ident>()?));
            }
            match PathType::new(&path) {
                PathType::TypeFunction | PathType::Function => {
                    input.parse::<Path>()?;
                    let token_stream = parenthesized_tokens(input)?;
                    BsnValue::Expr(quote! { #path(#token_stream) })
                }
                PathType::Const | PathType::TypeConst => {
                    input.parse::<Path>()?;
                    BsnValue::Expr(quote! { #path })
                }
                PathType::Type | PathType::Enum => BsnValue::Type(input.parse::<BsnType>()?),
            }
        } else if input.peek(Lit) {
            BsnValue::Lit(input.parse::<Lit>()?)
        } else if input.peek(Paren) {
            BsnValue::Tuple(input.parse::<BsnTuple>()?)
        } else if input.peek(Token![#]) {
            input.parse::<Token![#]>()?;
            BsnValue::Name(input.parse::<Ident>()?)
        } else {
            return Err(input.error("Unexpected input: Invalid BsnValue. This does not match any expected BSN value type."));
        })
    }
}

fn take_last_path_ident(path: &mut Path) -> Option<Ident> {
    let ident = path.segments.pop().map(|s| s.into_value().ident);
    path.segments.pop_punct();
    ident
}
