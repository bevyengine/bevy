use crate::bsn::types::{
    Bsn, BsnConstructor, BsnEntry, BsnFields, BsnInheritedScene, BsnNamedField,
    BsnRelatedSceneList, BsnRoot, BsnSceneList, BsnSceneListItem, BsnSceneListItems, BsnTuple,
    BsnType, BsnUnnamedField, BsnValue,
};
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
macro_rules! parse_punctuated_vec {
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
            $input.parse::<$separator>()?;
        }
    };
}

impl Parse for BsnRoot {
    fn parse(input: ParseStream) -> Result<Self> {
        Ok(BsnRoot(input.parse::<Bsn<true>>()?))
    }
}

impl<const ALLOW_FLAT: bool> Parse for Bsn<ALLOW_FLAT> {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut entries = Vec::new();
        if input.peek(Paren) {
            let content;
            parenthesized![content in input];
            while !content.is_empty() {
                entries.push(content.parse::<BsnEntry>()?);
            }
        } else {
            if ALLOW_FLAT {
                while !input.is_empty() {
                    entries.push(input.parse::<BsnEntry>()?);
                    if input.peek(Comma) {
                        // Not ideal, but this anticipatory break allows us to parse non-parenthesized
                        // flat Bsn entries in SceneLists
                        break;
                    }
                }
            } else {
                entries.push(input.parse::<BsnEntry>()?);
            }
        }

        Ok(Self { entries })
    }
}

impl Parse for BsnEntry {
    fn parse(input: ParseStream) -> Result<Self> {
        Ok(if input.peek(Token![:]) {
            BsnEntry::InheritedScene(input.parse::<BsnInheritedScene>()?)
        } else if input.peek(Token![#]) {
            input.parse::<Token![#]>()?;
            if input.peek(Brace) {
                BsnEntry::NameExpression(braced_tokens(input)?)
            } else {
                BsnEntry::Name(input.parse::<Ident>()?)
            }
        } else if input.peek(Brace) {
            BsnEntry::SceneExpression(braced_tokens(input)?)
        } else if input.peek(Bracket) {
            BsnEntry::ChildrenSceneList(input.parse::<BsnSceneList>()?)
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
                            BsnEntry::GetTemplatePatch(bsn_type)
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
                    todo!("A floating type-unknown const should be assumed to be a const scene right?")
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
                        BsnEntry::GetTemplateConstructor(bsn_constructor)
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
        parse_punctuated_vec!(scenes, input, BsnSceneListItem, Comma);
        Ok(BsnSceneListItems(scenes))
    }
}

impl Parse for BsnSceneListItem {
    fn parse(input: ParseStream) -> Result<Self> {
        Ok(if input.peek(Brace) {
            BsnSceneListItem::Expression(input.parse::<Block>()?)
        } else {
            BsnSceneListItem::Scene(input.parse::<Bsn<true>>()?)
        })
    }
}

impl Parse for BsnInheritedScene {
    fn parse(input: ParseStream) -> Result<Self> {
        input.parse::<Token![:]>()?;
        Ok(if input.peek(LitStr) {
            let path = input.parse::<LitStr>()?;
            BsnInheritedScene::Asset(path)
        } else {
            let function = input.parse::<Ident>()?;
            let args = if input.peek(Paren) {
                let content;
                parenthesized!(content in input);
                Some(content.parse_terminated(Expr::parse, Token![,])?)
            } else {
                None
            };
            BsnInheritedScene::Fn { function, args }
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
            parse_punctuated_vec!(fields, content, BsnNamedField, Comma);
            BsnFields::Named(fields)
        } else if input.peek(Paren) {
            let content;
            parenthesized![content in input];
            let mut fields = Vec::new();
            parse_punctuated_vec!(fields, content, BsnUnnamedField, Comma);
            BsnFields::Tuple(fields)
        } else {
            BsnFields::Named(Vec::new())
        })
    }
}

impl Parse for BsnNamedField {
    fn parse(input: ParseStream) -> Result<Self> {
        let name = input.parse::<Ident>()?;
        let mut is_template = false;
        let value = if input.peek(Colon) {
            input.parse::<Colon>()?;
            if input.peek(At) {
                input.parse::<At>()?;
                is_template = true;
            }
            Some(input.parse::<BsnValue>()?)
        } else {
            None
        };
        Ok(BsnNamedField {
            name,
            value,
            is_template,
        })
    }
}

impl Parse for BsnUnnamedField {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut is_template = false;
        if input.peek(At) {
            input.parse::<At>()?;
            is_template = true;
        }
        let value = input.parse::<BsnValue>()?;
        Ok(BsnUnnamedField { value, is_template })
    }
}

/// Parse a closure "loosely" without caring about the tokens between `|...|` and `{...}`. This ensures autocomplete works.
fn parse_closure_loose<'a>(input: &'a ParseBuffer) -> Result<TokenStream> {
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
fn braced_tokens<'a>(input: &'a ParseBuffer) -> Result<TokenStream> {
    let content;
    braced!(content in input);
    Ok(content.parse::<TokenStream>()?)
}

// Used to parse parenthesized tokens "loosely" without caring about the content in `(...)`. This ensures autocomplete works.
fn parenthesized_tokens<'a>(input: &'a ParseBuffer) -> Result<TokenStream> {
    let content;
    parenthesized!(content in input);
    Ok(content.parse::<TokenStream>()?)
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
        } else {
            return Err(input.error(
                "BsnValue parse for this input is not supported yet, nor is proper error handling :)"
            ));
        })
    }
}

enum PathType {
    Type,
    Enum,
    Const,
    TypeConst,
    TypeFunction,
    Function,
}

impl PathType {
    fn new(path: &Path) -> PathType {
        let mut iter = path.segments.iter().rev();
        if let Some(last_segment) = iter.next() {
            let last_string = last_segment.ident.to_string();
            let mut last_string_chars = last_string.chars();
            let last_ident_first_char = last_string_chars.next().unwrap();
            let is_const = last_string_chars
                .next()
                .map(|last_ident_second_char| last_ident_second_char.is_uppercase())
                .unwrap_or(false);
            if last_ident_first_char.is_uppercase() {
                if let Some(second_to_last_segment) = iter.next() {
                    // PERF: is there some way to avoid this string allocation?
                    let second_to_last_string = second_to_last_segment.ident.to_string();
                    let first_char = second_to_last_string.chars().next().unwrap();
                    if first_char.is_uppercase() {
                        if is_const {
                            PathType::TypeConst
                        } else {
                            PathType::Enum
                        }
                    } else {
                        if is_const {
                            PathType::Const
                        } else {
                            PathType::Type
                        }
                    }
                } else {
                    PathType::Type
                }
            } else {
                if let Some(second_to_last) = iter.next() {
                    // PERF: is there some way to avoid this string allocation?
                    let second_to_last_string = second_to_last.ident.to_string();
                    let first_char = second_to_last_string.chars().next().unwrap();
                    if first_char.is_uppercase() {
                        PathType::TypeFunction
                    } else {
                        PathType::Function
                    }
                } else {
                    PathType::Function
                }
            }
        } else {
            // This won't be hit so just pick one to make it easy on consumers
            PathType::Type
        }
    }
}

fn take_last_path_ident(path: &mut Path) -> Option<Ident> {
    let ident = path.segments.pop().map(|s| s.into_value().ident);
    path.segments.pop_punct();
    ident
}
