use proc_macro2::Span;
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::Error;
use syn::ExprClosure;
use syn::{parse::Parse, Expr, ExprPath, Ident, Pat, PatTupleStruct, Path, Token};

use crate::bevy_ecs_path;

#[derive(Clone)]

struct MatcherPattern {
    pattern: Pat,
}

#[derive(Clone)]
struct MatcherClosure {
    closure: ExprClosure,
}

#[derive(Clone)]
enum MatcherType {
    Expression(Expr),
    Pattern(MatcherPattern),
    Closure(MatcherClosure),
}

#[derive(Clone)]
struct Matcher {
    state_type: Option<Path>,
    matchers: Vec<(bool, MatcherType)>,
}

impl Parse for Matcher {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        Self::parse_with_state_type(input, None)
    }
}
impl Matcher {
    fn parse_with_state_type(
        input: syn::parse::ParseStream,
        state_type: Option<Path>,
    ) -> syn::Result<Self> {
        let state_type = if let Some(state_type) = state_type {
            state_type
        } else {
            let state_type = if let Ok(expr) = Expr::parse(input) {
                match &expr {
                    Expr::Path(p) => Some(p.path.clone()),
                    _ => return Err(syn::Error::new(Span::call_site(), "Closures must define the state type at the start of the matcher macro, like so !(StateType, |...| {})")),
                }
            } else {
                None
            };

            let Some(state_type) = state_type else {
                return Err(syn::Error::new(Span::call_site(), "Couldn't determine the state type. Define the state type at the start of the matcher macro, like so !(StateType, Pattern or Closure)"));
            };

            input.parse::<Token![,]>()?;
            state_type
        };

        let mut matchers = vec![];

        loop {
            matchers.push(
                MatcherType::parse_with_state_type(input, &state_type).map_err(|e| {
                    Error::new(
                        e.span(),
                        format!("Failed to parse matcher with a given state type - {e:?}"),
                    )
                })?,
            );
            if input.parse::<Token![,]>().is_err() {
                break;
            }
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
        let is_expr = input.peek(Token![=]);
        if is_closure {
            Ok((
                every,
                Self::Closure(MatcherClosure::parse(input).map_err(|e| {
                    Error::new(e.span(), format!("Failed to parse closure: {e:?}"))
                })?),
            ))
        } else if is_expr {
            input.parse::<Token![=]>()?;
            let expr = Expr::parse(input)
                .map_err(|e| Error::new(e.span(), format!("Failed to parse expression: {e:?}")))?;
            Ok((every, Self::Expression(expr)))
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

pub struct MatchMacroResult {
    state_type: Option<Path>,
    matchers: Vec<(bool, MatchTypes)>,
}

pub enum MatchTypes {
    Expression(TokenStream),
    Pattern(TokenStream),
    Closure(TokenStream),
}

impl MatchTypes {
    fn from_matcher_type_vec(value: Vec<(bool, MatcherType)>) -> Vec<(bool, Self)> {
        value
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
            .collect()
    }
}

pub fn define_match_macro(input: proc_macro::TokenStream) -> syn::Result<MatchMacroResult> {
    let matcher = syn::parse::<Matcher>(input)
        .map_err(|e| Error::new(e.span(), format!("Attempting to parse matcher: {e:?}")))?;

    let state_type = matcher.state_type;

    if matcher.matchers.is_empty() {
        return Err(Error::new(Span::call_site(), "No matcher statements found"));
    };

    let matchers = MatchTypes::from_matcher_type_vec(matcher.matchers);

    Ok(MatchMacroResult {
        state_type,
        matchers,
    })
}

pub fn state_matches_macro(match_result: MatchMacroResult) -> proc_macro::TokenStream {
    let MatchMacroResult {
        state_type,
        matchers,
    } = match_result;

    match (state_type, matchers.first(), matchers.len()) {
        (_, Some((_, MatchTypes::Expression(expr))), 1) => {
            let mut module_path = bevy_ecs_path();
            module_path.segments.push(format_ident!("schedule").into());
            module_path
                .segments
                .push(format_ident!("common_conditions").into());
            module_path
                .segments
                .push(format_ident!("state_matches").into());

            quote!({
                #expr
            })
        }
        (Some(state_type), _, _) => {
            let match_function = generate_match_function(&state_type, &matchers);

            quote!({
                #match_function
            })
        }
        _ => panic!("No State Type"),
    }
    .into()
}

fn generate_match_function(state_type: &Path, matchers: &[(bool, MatchTypes)]) -> TokenStream {
    let mut module_path = bevy_ecs_path();
    module_path.segments.push(format_ident!("schedule").into());

    let tokens = matchers
        .iter()
        .map(|(every, matcher)| match matcher {
            MatchTypes::Expression(e) => {
                if *every {
                    quote!(if main.matches(#e) { return #module_path::MatchesStateTransition::TransitionMatches; }
                    )
                } else {
                    quote!(match #state_type::matches_transition(#e,Some(main), secondary) {
                            #module_path::MatchesStateTransition::TransitionMatches => { return #module_path::MatchesStateTransition::TransitionMatches; },
                            #module_path::MatchesStateTransition::MainMatches => { return #module_path::MatchesStateTransition::MainMatches; },
                            _ => {}
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

                            if matches(main) { return #module_path::MatchesStateTransition::TransitionMatches; }
                        }
                    )
                } else {
                    quote!({
                            fn matches(state: &#state_type) -> bool {
                                #tokens
                            }

                            if matches(main) {  if let Some(secondary) = secondary {
                                if matches(secondary) {
                                    return #module_path::MatchesStateTransition::MainMatches;
                                } else {
                                    return #module_path::MatchesStateTransition::TransitionMatches;
                                }
                            } else {
                                return #module_path::MatchesStateTransition::TransitionMatches;
                            } }
                        }
                    )
                }
            }
            MatchTypes::Closure(tokens) => {
                if *every {
                    quote!({
                            let matches = #tokens;

                            if main.matches(matches) { return #module_path::MatchesStateTransition::TransitionMatches; }
                        }
                    )
                } else {
                    quote!({
                            let matches = #tokens;


                            match #state_type::matches_transition(matches, Some(main), secondary) {
                                #module_path::MatchesStateTransition::TransitionMatches => { return #module_path::MatchesStateTransition::TransitionMatches; },
                                #module_path::MatchesStateTransition::MainMatches => { return #module_path::MatchesStateTransition::MainMatches; },
                                _ => {}
                            }
                        }
                    )
                }
            }
        })
        .collect::<Vec<_>>();

    let tokens = TokenStream::from_iter(tokens);

    quote!(
        |main: Option<&#state_type>, secondary: Option<&#state_type>| {
            let Some(main) = main else {
                return  #module_path::MatchesStateTransition::NoMatch;
            };

            #tokens

            return  #module_path::MatchesStateTransition::NoMatch;
        }
    )
}
