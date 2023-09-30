use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::{format_ident, quote};
use syn::{
    parse::Parse, parse_macro_input, Expr, ExprPath, Ident, Pat, PatTupleStruct, Path, Token,
    Visibility,
};

use crate::bevy_ecs_path;

struct StateMatcher {
    visibility: Visibility,
    name: Ident,
    matcher: MatcherPattern,
}

impl Parse for StateMatcher {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let visibility: Visibility = input.parse()?;
        let name: Ident = input.parse()?;
        input.parse::<Token![,]>()?;
        let matcher = input.parse()?;

        Ok(Self {
            visibility,
            name,
            matcher,
        })
    }
}

struct MatcherPattern {
    state_type: Path,
    pattern: Pat,
}

enum Matcher {
    Expression(Expr),
    Pattern(MatcherPattern),
}

impl Parse for Matcher {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let state_type = if let Ok(expr) = Expr::parse(input) {
            match &expr {
                Expr::Path(p) => Some(p.path.clone()),
                _ => {
                    return Ok(Self::Expression(expr));
                }
            }
        } else {
            None
        };

        if let Some(state_type) = state_type {
            if input.parse::<Token![,]>().is_ok() {
                let pattern = Pat::parse_multi_with_leading_vert(input)?;

                let pattern = inject_state_type(pattern, &state_type);
                Ok(Self::Pattern(MatcherPattern {
                    state_type,
                    pattern,
                }))
            } else {
                Ok(Self::Expression(Expr::Path(ExprPath {
                    attrs: vec![],
                    qself: None,
                    path: state_type,
                })))
            }
        } else {
            Ok(Self::Pattern(MatcherPattern::parse(input)?))
        }
    }
}

impl Parse for MatcherPattern {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let state_type = Path::parse(input)?;
        input.parse::<Token![,]>()?;
        let pattern = Pat::parse_multi_with_leading_vert(input)?;

        let pattern = inject_state_type(pattern, &state_type);

        Ok(Self {
            state_type,
            pattern,
        })
    }
}

fn inject_state_type(pattern: Pat, state_type: &Path) -> Pat {
    match &pattern {
        Pat::Ident(i) => {
            let mut path = state_type.clone();
            path.segments.push(i.ident.clone().into());
            Pat::Path(ExprPath {
                attrs: vec![],
                qself: None,
                path,
            })
        }
        Pat::Or(or) => {
            let mut or = or.clone();
            for pat in or.cases.iter_mut() {
                *pat = inject_state_type(pat.clone(), state_type);
            }
            Pat::Or(or)
        }
        Pat::Paren(p) => {
            let mut pat = p.clone();
            pat.pat = Box::new(inject_state_type(pat.pat.as_ref().clone(), state_type));
            Pat::Paren(pat)
        }
        Pat::Path(p) => {
            if state_type
                .segments
                .iter()
                .zip(p.path.segments.iter())
                .all(|(a, b)| a == b)
            {
                Pat::Path(p.clone())
            } else {
                let mut path = state_type.clone();
                path.segments.extend(p.path.segments.iter().cloned());
                Pat::Path(ExprPath {
                    path,
                    attrs: p.attrs.clone(),
                    qself: p.qself.clone(),
                })
            }
        }
        Pat::Struct(s) => {
            let mut s = s.clone();
            let path = &s.path;
            let path = if state_type
                .segments
                .iter()
                .zip(path.segments.iter())
                .all(|(a, b)| a == b)
            {
                path.clone()
            } else {
                let mut p = state_type.clone();
                p.segments.extend(path.segments.iter().cloned());
                p
            };
            s.path = path;
            Pat::Struct(s)
        }
        Pat::Tuple(t) => Pat::TupleStruct(PatTupleStruct {
            attrs: t.attrs.clone(),
            qself: None,
            path: state_type.clone(),
            paren_token: t.paren_token,
            elems: t.elems.clone(),
        }),
        Pat::TupleStruct(t) => {
            let mut t = t.clone();
            let path = &t.path;
            let path = if state_type
                .segments
                .iter()
                .zip(path.segments.iter())
                .all(|(a, b)| a == b)
            {
                path.clone()
            } else {
                let mut p = state_type.clone();
                p.segments.extend(path.segments.iter().cloned());
                p
            };
            t.path = path;
            Pat::TupleStruct(t)
        }
        _ => pattern,
    }
}

pub fn define_state_matcher(input: TokenStream) -> TokenStream {
    let StateMatcher {
        visibility,
        name,
        matcher,
    } = parse_macro_input!(input as StateMatcher);

    let MatcherPattern {
        state_type,
        pattern,
    } = matcher;

    let mut trait_path = bevy_ecs_path();
    trait_path.segments.push(format_ident!("schedule").into());
    trait_path
        .segments
        .push(format_ident!("StateMatcher").into());

    quote! {

       #[derive(Debug, Eq, PartialEq, Hash, Clone)]
        #visibility struct #name;

        impl #trait_path<#state_type> for #name {
            fn match_state(&self, state: &#state_type) -> bool {
                match state {
                    #pattern => true,
                    _ => false
                }
            }
        }

    }
    .into()
}

pub enum MatchMacro {
    OnEnter,
    OnExit,
    InState,
}

pub fn define_match_macro(
    match_macro: MatchMacro,
    strict: Option<bool>,
    input: TokenStream,
) -> TokenStream {
    let matcher = parse_macro_input!(input as Matcher);

    let mut module_path = bevy_ecs_path();
    module_path.segments.push(format_ident!("prelude").into());

    let mut matcher_type_path = module_path.clone();
    matcher_type_path
        .segments
        .push(Ident::new("StateMatcherFunction", Span::call_site()).into());

    let mut call_path = module_path.clone();
    let call = match match_macro {
        MatchMacro::OnEnter => "OnEnter",
        MatchMacro::OnExit => "OnExit",
        MatchMacro::InState => "in_state",
    };

    call_path
        .segments
        .push(Ident::new(call, Span::call_site()).into());

    if let Some(strict) = strict {
        let call = if strict {
            "matching_strict"
        } else {
            "matching"
        };
        call_path
            .segments
            .push(Ident::new(call, Span::call_site()).into());
    }

    match matcher {
        Matcher::Expression(exp) => quote!(
            #call_path(#exp)
        )
        .into(),
        Matcher::Pattern(MatcherPattern {
            state_type,
            pattern,
        }) => quote!({
            let matcher = |state: &#state_type| matches!(state.clone(), #pattern);
            let matcher : #matcher_type_path<#state_type> = matcher.into_state_matcher();

            #call_path(matcher)
        }
        )
        .into(),
    }
}
