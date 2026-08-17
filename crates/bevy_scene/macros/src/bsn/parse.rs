use crate::_bsn::types::{
    Bsn, BsnConstructor, BsnEntry, BsnFields, BsnFnArg, BsnFnArgs, BsnListRoot, BsnNamedField,
    BsnRelatedSceneList, BsnRoot, BsnScene, BsnSceneFn, BsnSceneList, BsnSceneListItem,
    BsnSceneListItems, BsnTuple, BsnType, BsnUnnamedField, BsnValue,
};
use bevy_macro_utils::{path_to_string, PathType};
use proc_macro2::{Delimiter, TokenStream, TokenTree};
use quote::quote;
use syn::{
    braced, bracketed,
    buffer::Cursor,
    parenthesized,
    parse::{discouraged::Speculative, Parse, ParseBuffer, ParseStream},
    spanned::Spanned,
    token::{At, Brace, Bracket, Colon, Comma, Paren, Tilde},
    Block, Ident, Lit, LitStr, Path, Result, Token,
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
        if input.peek(Paren) {
            let content;
            parenthesized![content in input];
            while !content.is_empty() {
                let entry = BsnEntry::parse(&content)?;
                if matches!(entry, BsnEntry::CachedScene(_)) && !entries.is_empty() {
                    return Err(syn::Error::new(
                        content.span(),
                        "Caching entries after the first is not supported, remove the ':' prefix or make this the first entry.",
                    ));
                }
                entries.push(entry);
            }
        } else if ALLOW_FLAT {
            while !input.is_empty() {
                let entry = BsnEntry::parse(input)?;
                if matches!(entry, BsnEntry::CachedScene(_)) && !entries.is_empty() {
                    return Err(syn::Error::new(
                        input.span(),
                        "Caching entries after the first is not supported, remove the ':' prefix or make this the first entry.",
                    ));
                }
                entries.push(entry);
                if input.peek(Comma) {
                    // Not ideal, but this anticipatory break allows us to parse non-parenthesized
                    // flat Bsn entries in SceneLists
                    break;
                }
            }
        } else {
            entries.push(BsnEntry::parse(input)?);
        }

        Ok(Self { entries })
    }
}

impl BsnEntry {
    fn parse(input: ParseStream) -> Result<Self> {
        Ok(if input.peek(Token![:]) && !input.peek(Token![::]) {
            BsnEntry::CachedScene(BsnScene::parse(input)?)
        } else if input.peek(Token![#]) {
            input.parse::<Token![#]>()?;
            BsnEntry::Name(input.parse::<Ident>()?)
        } else if input.peek(Brace) || input.peek(At) {
            BsnEntry::UncachedScene(BsnScene::parse(input)?)
        } else {
            let is_template = input.peek(Tilde);
            if is_template {
                input.parse::<Tilde>()?;
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
                    let args = input.parse::<BsnFnArgs>()?;
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
                        let args = input.parse::<BsnFnArgs>()?;
                        BsnEntry::UncachedScene(BsnScene::Fn(BsnSceneFn { path, args }))
                    } else {
                        BsnEntry::UncachedScene(BsnScene::Expression(quote! {#path}))
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

impl BsnScene {
    fn parse(input: ParseStream) -> Result<Self> {
        let cached = if input.peek(Token![:]) {
            Some(input.parse::<Token![:]>()?)
        } else {
            None
        };

        let err_if_cached = |msg: &str| {
            if let Some(colon) = cached {
                Err(syn::Error::new(colon.span(), msg))
            } else {
                Ok(())
            }
        };

        // It may seem odd how this is checking LitStr again
        // and how there doesn't seem to be a need for all the specific `err_if_cached`
        // in later code. But since caching is planned, and will very likely
        // have the limitations which are ensured by the other errors below,
        // this is its own block so its very simple to remove once caching is implemented.
        if !input.peek(LitStr) {
            err_if_cached("Currently, caching is only supported for scene assets. Please remove the ':' prefix for now.")?;
        }

        Ok(if input.peek(LitStr) {
            let path = input.parse::<LitStr>()?;
            if cached.is_none() {
                return Err(syn::Error::new(
                    path.span(),
                    "Cannot use scene assets without caching, please add the ':' prefix.",
                ));
            }
            BsnScene::Asset(path)
        } else if input.peek(Brace) {
            err_if_cached("Cannot cache scene expressions")?;
            BsnScene::Expression(braced_tokens(input)?)
        } else if input.peek(At) {
            input.parse::<At>()?;
            let sc = input.parse::<BsnType>()?;
            if sc.fields.len() > 0 {
                err_if_cached("Cannot cache Scene Components with props/fields")?;
            }
            BsnScene::SceneComponent(sc)
        } else {
            // PERF: do we really need this fork here?
            let path = input.fork().parse::<Path>()?;
            match PathType::new(&path) {
                PathType::Type | PathType::Enum => {
                    // Scene components are parsed before this if an @ is found.
                    // If this path is hit, that means it wasn't prefixed by @
                    return Err(syn::Error::new(
                        path.span(),
                        format!(
                            "Scene component {} needs to be prefixed by '@'",
                            path_to_string(&path),
                        ),
                    ));
                }
                PathType::Function | PathType::TypeFunction => {
                    let path = input.parse::<Path>()?;
                    let args = input.parse::<BsnFnArgs>()?;
                    if !args.0.is_empty() {
                        err_if_cached("Cannot cache Scene function with arguments")?;
                    }
                    BsnScene::Fn(BsnSceneFn { path, args })
                }
                path_type => {
                    return Err(syn::Error::new(
                        path.span(),
                        format!(
                            "Cannot cache path {} of type {:?}",
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
        let mut is_name_shorthand = false;
        let value = if input.peek(Colon) {
            input.parse::<Colon>()?;

            if input.is_empty() || input.peek(Comma) {
                None
            } else {
                Some(input.parse::<BsnValue>()?)
            }
        } else {
            is_name_shorthand = true;
            None
        };
        Ok(BsnNamedField {
            name,
            value,
            is_prop,
            is_name_shorthand,
        })
    }
}

impl Parse for BsnUnnamedField {
    fn parse(input: ParseStream) -> Result<Self> {
        let value = input.parse::<BsnValue>()?;
        Ok(BsnUnnamedField { value })
    }
}

/// Parses tuple arguments into a list of [`TokenStream`]s. This avoids
/// fully parsing Rust expressions, which makes this less strict and cheaper to parse.
/// This also allows autocomplete to work, even if the tokens aren't a valid rust expression.
///
/// This will accept anything "tuple-like" in the form (X1, ..., XY), where XY is a TokenStream.
fn parse_tuple_loose(input: &ParseBuffer) -> Result<Vec<TokenStream>> {
    let content;
    parenthesized!(content in input);
    let mut args = Vec::new();
    let mut current_tokens = Vec::new();
    let mut in_closure_args = false;
    let mut generic_scope = 0;
    while !content.is_empty() {
        let tt = content.parse::<TokenTree>()?;
        match &tt {
            TokenTree::Punct(punct) => match punct.as_char() {
                ',' if !in_closure_args && generic_scope == 0 => {
                    args.push(TokenStream::from_iter(current_tokens.drain(..)));
                }
                '|' => {
                    in_closure_args = !in_closure_args;
                    current_tokens.push(tt);
                }
                '<' => {
                    generic_scope += 1;
                    current_tokens.push(tt);
                }
                '>' => {
                    generic_scope -= 1;
                    current_tokens.push(tt);
                }
                _ => current_tokens.push(tt),
            },
            _ => current_tokens.push(tt),
        }
    }

    if !current_tokens.is_empty() {
        args.push(TokenStream::from_iter(current_tokens));
    }
    Ok(args)
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

// Used to parse bracketed tokens "loosely" without caring about the content in `[...]`. This ensures autocomplete works.
fn bracketed_tokens(input: &ParseBuffer) -> Result<TokenStream> {
    let content;
    bracketed!(content in input);
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
        } else if input.peek(Token![const]) && input.peek2(Brace) {
            let const_token = input.parse::<Token![const]>()?;
            let braced = braced_tokens(input)?;

            BsnValue::Expr(quote! {#const_token {#braced}})
        } else if input.peek(Token![unsafe]) && input.peek2(Brace) {
            let unsafe_token = input.parse::<Token![unsafe]>()?;
            let braced = braced_tokens(input)?;

            BsnValue::Expr(quote! {#unsafe_token {#braced}})
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
                    let maybe_macro = input.parse::<Token![!]>().ok();
                    if input.peek(Paren) {
                        let token_stream = parenthesized_tokens(input)?;
                        BsnValue::Expr(quote! { #path #maybe_macro (#token_stream) })
                    } else if input.peek(Bracket) {
                        let token_stream = bracketed_tokens(input)?;
                        BsnValue::Expr(quote! { #path #maybe_macro [#token_stream] })
                    } else if input.peek(Brace) {
                        let token_stream = braced_tokens(input)?;
                        BsnValue::Expr(quote! { #path #maybe_macro { #token_stream } })
                    } else {
                        return Err(input.error("Unexpected input after function name"));
                    }
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

impl Parse for BsnFnArgs {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut fn_args = Vec::new();
        for tokens in parse_tuple_loose(input)? {
            fn_args.push(syn::parse2::<BsnFnArg>(tokens)?)
        }
        Ok(BsnFnArgs(fn_args))
    }
}

impl Parse for BsnFnArg {
    fn parse(input: ParseStream) -> Result<Self> {
        Ok(if input.peek(Token![#]) {
            let forked = input.fork();
            if let Ok(ident) = forked.parse::<EntityNameIdent>() {
                input.advance_to(&forked);
                BsnFnArg::EntityName(ident.0)
            } else {
                BsnFnArg::Tokens(input.parse::<TokenStream>()?)
            }
        } else {
            BsnFnArg::Tokens(input.parse::<TokenStream>()?)
        })
    }
}

struct EntityNameIdent(Ident);

impl Parse for EntityNameIdent {
    fn parse(input: ParseStream) -> Result<Self> {
        input.parse::<Token![#]>()?;
        Ok(EntityNameIdent(input.parse::<Ident>()?))
    }
}

fn take_last_path_ident(path: &mut Path) -> Option<Ident> {
    let ident = path.segments.pop().map(|s| s.into_value().ident);
    path.segments.pop_punct();
    ident
}
