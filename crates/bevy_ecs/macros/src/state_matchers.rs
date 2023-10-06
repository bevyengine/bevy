use proc_macro2::Span;
use proc_macro2::TokenStream;
use quote::ToTokens;
use quote::{format_ident, quote};
use syn::Error;
use syn::ExprClosure;
use syn::{parse::Parse, Expr, ExprPath, Ident, Pat, PatTupleStruct, Path, Token, Visibility};

use crate::bevy_ecs_path;

struct StateMatcher {
    visibility: Visibility,
    name: Ident,
    matcher: MatcherPattern,
    state_type: Option<Path>,
}

impl Parse for StateMatcher {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let visibility: Visibility = input.parse()?;
        let name: Ident = input.parse()?;
        input.parse::<Token![,]>()?;
        let matcher: Matcher = input.parse()?;
        let state_type = matcher.state_type.clone();

        let matcher = matcher.matchers.first().ok_or(syn::Error::new(
            Span::call_site(),
            "State Matcher must have a matcher set",
        ))?;

        let (_, MatcherType::Pattern(matcher)) = matcher else {
            return Err(syn::Error::new(
                Span::call_site(),
                "State Matcher must be given a pattern",
            ));
        };

        Ok(Self {
            state_type,
            visibility,
            name,
            matcher: matcher.clone(),
        })
    }
}

#[derive(Clone)]

struct MatcherPattern {
    pattern: Pat,
}

#[derive(Clone)]
struct MatcherClosure {
    closure: ExprClosure,
}

#[derive(Clone)]
struct Matcher {
    state_type: Option<Path>,
    matchers: Vec<(bool, MatcherType)>,
}

#[derive(Clone)]
enum MatcherType {
    Expression(Expr),
    Pattern(MatcherPattern),
    Closure(MatcherClosure),
}

impl Parse for Matcher {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let state_type = if let Ok(expr) = Expr::parse(input) {
            match &expr {
                Expr::Path(p) => Some(p.path.clone()),
                Expr::Closure(_) => return Err(syn::Error::new(Span::call_site(), "Closures must define the state type at the start of the matcher macro, like so !(StateType, |...| {})")),
                _ => {
                    return Ok(Self { state_type: None, matchers: vec![(false, MatcherType::Expression(expr))]});
                }
            }
        } else {
            None
        };

        let Some(state_type) = state_type else {
            return Err(syn::Error::new(Span::call_site(), "Couldn't determine the state type. Define the state type at the start of the matcher macro, like so !(StateType, Pattern or Closure)"));
        };

        if !input.peek(Token![,]) {
            return Ok(Self {
                state_type: None,
                matchers: vec![(
                    false,
                    MatcherType::Expression(Expr::Path(ExprPath {
                        path: state_type,
                        attrs: vec![],
                        qself: None,
                    })),
                )],
            });
        }

        let mut matchers = vec![];

        while input.parse::<Token![,]>().is_ok() {
            matchers.push(
                MatcherType::parse_with_state_type(input, &state_type).map_err(|e| {
                    Error::new(
                        e.span(),
                        format!("Failed to parse matcher with a given state type - {e:?}"),
                    )
                })?,
            );
        }

        Ok(Self {
            state_type: Some(state_type),
            matchers,
        })
    }
}

impl MatcherType {
    fn parse_with_state_type(
        input: syn::parse::ParseStream,
        state_type: &Path,
    ) -> syn::Result<(bool, Self)> {
        let every = {
            let ahead = input.fork();
            let every: syn::Result<Ident> = ahead.parse();

            if let Ok(every) = every {
                if every == "every" {
                    input.parse::<Ident>().map_err(|e| {
                        Error::new(e.span(), format!("Every should exist but doesn't - {e:?}"))
                    })?;
                    true
                } else {
                    false
                }
            } else {
                false
            }
        };
        let is_closure = input.peek(Token![|]) || input.peek(Token![move]);
        if is_closure {
            Ok((
                every,
                Self::Closure(MatcherClosure::parse(input).map_err(|e| {
                    Error::new(e.span(), format!("Failed to parse closure: {e:?}"))
                })?),
            ))
        } else {
            let pattern = MatcherPattern::parse_with_state_type(input, state_type)
                .map_err(|e| Error::new(e.span(), format!("Failed to parse pattern: {e:?}")))?;
            Ok((every, Self::Pattern(pattern)))
        }
    }
}

impl MatcherPattern {
    fn parse_with_state_type(
        input: syn::parse::ParseStream,
        state_type: &Path,
    ) -> syn::Result<Self> {
        let pattern = Pat::parse_multi_with_leading_vert(input)
            .map_err(|e| syn::Error::new(e.span(), format!("Couldn't parse pattern: {e:?}")))?;

        let pattern = inject_state_type(pattern, state_type);

        Ok(Self { pattern })
    }
}

impl Parse for MatcherClosure {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let closure = ExprClosure::parse(input)?;

        Ok(Self { closure })
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

pub fn define_state_matcher(
    input: proc_macro::TokenStream,
) -> syn::Result<proc_macro::TokenStream> {
    let StateMatcher {
        visibility,
        name,
        state_type,
        matcher,
    } = syn::parse(input)?;

    let Some(state_type) = state_type else {
        return Err(syn::Error::new(
            Span::call_site(),
            "Couldn't determine state type",
        ));
    };

    let MatcherPattern { pattern, .. } = matcher;

    let mut trait_path = bevy_ecs_path();
    trait_path.segments.push(format_ident!("schedule").into());
    trait_path
        .segments
        .push(format_ident!("StateMatcher").into());

    Ok(quote! {

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
    .into())
}

pub enum MatchMacro {
    OnEnter,
    OnExit,
}

pub struct MatchMacroResult {
    state_type: Option<Path>,
    matchers: Vec<(bool, MatchTypes)>,
}

pub enum MatchTypes {
    Expression(TokenStream),
    Pattern(TokenStream),
    Closure(TokenStream),
}

pub fn define_match_macro(input: proc_macro::TokenStream) -> syn::Result<MatchMacroResult> {
    let matcher = syn::parse::<Matcher>(input)
        .map_err(|e| Error::new(e.span(), format!("Attempting to parse matcher: {e:?}")))?;

    let state_type = matcher.state_type;

    if matcher.matchers.is_empty() {
        return Err(Error::new(Span::call_site(), "No matcher statements found"));
    };

    let matchers = matcher
        .matchers
        .iter()
        .map(|matcher| match matcher {
            (every, MatcherType::Expression(exp)) => (
                *every,
                MatchTypes::Expression(quote!(
                    #exp
                )),
            ),
            (every, MatcherType::Pattern(MatcherPattern { pattern })) => (
                *every,
                MatchTypes::Pattern(quote!(matches!(state, #pattern))),
            ),
            (every, MatcherType::Closure(MatcherClosure { closure: pattern })) => {
                (*every, MatchTypes::Closure(quote!(#pattern)))
            }
        })
        .collect();

    Ok(MatchMacroResult {
        state_type,
        matchers,
    })
}

pub fn simple_state_transition_macros(
    macro_type: MatchMacro,
    match_result: MatchMacroResult,
) -> proc_macro::TokenStream {
    let mut module_path = bevy_ecs_path();
    module_path.segments.push(format_ident!("schedule").into());

    let state_type = match_result.state_type;

    if match_result.matchers.len() > 1 {
        let Some(state_type) = state_type else {
            panic!("Couldn't determine state type");
        };
        let mut module_path = state_type.clone();
        match macro_type {
            MatchMacro::OnEnter => {
                module_path
                    .segments
                    .push(format_ident!("on_state_entry_schedule").into());
            }
            MatchMacro::OnExit => {
                module_path
                    .segments
                    .push(format_ident!("on_state_exit_schedule").into());
            }
        }

        let tokens = match_result
            .matchers
            .iter()
            .map(|(every, matcher)| match matcher {
                MatchTypes::Expression(e) => {
                    if *every {
                        quote!(if #e.match_state(main) { return true; }
                        )
                    } else {
                        quote!(if #e.match_state(main) {
                                if let Some(secondary) = secondary {
                                    return !#e.match_state(secondary);
                                }
                                return false;
                            }
                        )
                    }
                }
                MatchTypes::Pattern(tokens) => {
                    if *every {
                        quote!({
                                fn matches(state: &#state_type) -> bool {
                                    #tokens
                                }

                                if matches(main) { return true; }
                            }
                        )
                    } else {
                        quote!({
                                fn matches(state: &#state_type) -> bool {
                                    #tokens
                                }

                                if matches(main) {  if let Some(secondary) = secondary {
                                    return !matches(secondary);
                                } }
                            }
                        )
                    }
                }
                MatchTypes::Closure(tokens) => {
                    if *every {
                        quote!({
                                let matches = #tokens;

                                if matches(main) { return true; }
                            }
                        )
                    } else {
                        quote!({
                                let matches = #tokens;

                                if matches(main) {  if let Some(secondary) = secondary {
                                    return !matches(secondary);
                                } }
                            }
                        )
                    }
                }
            })
            .collect::<Vec<_>>();

        let tokens = TokenStream::from_iter(tokens);

        let result = quote!({
            let matcher = |main: Option<&#state_type>, secondary: Option<&#state_type>| {
                let Some(main) = main else {
                    return false;
                };

                #tokens

                return false;
            };

            #module_path().from_closure(matcher)
        });

        result.into()
    } else if let Some((every, match_result)) = match_result.matchers.first() {
        let every = *every;
        match match_result {
            MatchTypes::Expression(expr) => {
                module_path.segments.push(
                    Ident::new(
                        match macro_type {
                            MatchMacro::OnEnter => "OnEnter",
                            MatchMacro::OnExit => "OnExit",
                        },
                        Span::call_site(),
                    )
                    .into(),
                );

                quote!(#module_path(#expr)).into()
            }
            MatchTypes::Pattern(tokens) => {
                let Some(state_type) = state_type else {
                    panic!("Couldn't determine state type");
                };
                let mut module_path = state_type.clone();
                let state_type = state_type.clone().into_token_stream();

                let call = match macro_type {
                    MatchMacro::OnEnter => {
                        module_path
                            .segments
                            .push(format_ident!("on_state_entry_schedule").into());

                        format_ident!("{}", if every { "every_entrance" } else { "matching" })
                    }
                    MatchMacro::OnExit => {
                        module_path
                            .segments
                            .push(format_ident!("on_state_exit_schedule").into());

                        format_ident!("{}", if every { "every_exit" } else { "matching" })
                    }
                };

                let matches = quote!(fn matches(state: &#state_type) -> bool {
                    #tokens
                });

                quote!({
                    #matches

                    #module_path().#call::<()>(matches)
                })
                .into()
            }
            MatchTypes::Closure(tokens) => {
                let Some(state_type) = state_type else {
                    panic!("Couldn't determine state type");
                };
                let mut module_path = state_type.clone();
                match macro_type {
                    MatchMacro::OnEnter => {
                        module_path
                            .segments
                            .push(format_ident!("on_state_entry_schedule").into());
                    }
                    MatchMacro::OnExit => {
                        module_path
                            .segments
                            .push(format_ident!("on_state_exit_schedule").into());
                    }
                }

                let matches = quote!(let matches = #tokens;);

                quote!({
                    #matches

                    #module_path().from_closure(matches)
                })
                .into()
            }
        }
    } else {
        panic!("No matchers found");
    }
}

pub fn state_matches_macro(match_result: MatchMacroResult) -> proc_macro::TokenStream {
    let mut module_path = bevy_ecs_path();
    module_path.segments.push(format_ident!("schedule").into());
    let state_type = match_result.state_type;

    if match_result.matchers.len() > 1 {
        let Some(state_type) = state_type else {
            panic!("Couldn't determine state type");
        };
        let tokens = match_result
            .matchers
            .iter()
            .map(|(_, matcher)| match matcher {
                MatchTypes::Expression(e) => {
                    quote!(if #e.match_state(main) { return true; }
                    )
                }
                MatchTypes::Pattern(tokens) => {
                    quote!({
                            fn matches(state: &#state_type) -> bool {
                                #tokens
                            }

                            if matches(main) { return true; }
                        }
                    )
                }
                MatchTypes::Closure(tokens) => {
                    quote!({
                            let matches = #tokens;

                            if matches(main) { return true; }
                        }
                    )
                }
            })
            .collect::<Vec<_>>();

        let tokens = TokenStream::from_iter(tokens);

        let result = quote!({

            |state: Option<Res<State<#state_type>>>| {
                let Some(state) = state else {
                    return false;
                };
                let f = #tokens;

                f(&state)
            }
        });

        result.into()
    } else if let Some((_, match_result)) = match_result.matchers.first() {
        match match_result {
            MatchTypes::Expression(expr) => {
                module_path
                    .segments
                    .push(Ident::new("in_state", Span::call_site()).into());
                quote!(#module_path(#expr)).into()
            }
            MatchTypes::Pattern(tokens) => {
                let Some(state_type) = state_type else {
                    panic!("Couldn't determine state type");
                };
                quote!(|state: Option<Res<State<#state_type>>>| {
                    let Some(state) = state else {
                        return false;
                    };
                    let state : &#state_type = &state;
                    #tokens
                })
                .into()
            }
            MatchTypes::Closure(tokens) => {
                let Some(state_type) = state_type else {
                    panic!("Couldn't determine state type");
                };
                quote!(
                    |state: Option<Res<State<#state_type>>>| {
                        let Some(state) = state else {
                            return false;
                        };
                        let f = #tokens;

                        f(&state)
                    }
                )
                .into()
            }
        }
    } else {
        panic!("No matchers found");
    }
}
