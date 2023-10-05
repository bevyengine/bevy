use proc_macro2::Span;
use proc_macro2::TokenStream;
use quote::ToTokens;
use quote::{format_ident, quote};
use syn::ExprClosure;
use syn::{parse::Parse, Expr, ExprPath, Ident, Pat, PatTupleStruct, Path, Token, Visibility};

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
    every: bool,
}

struct MatcherClosure {
    state_type: Path,
    closure: ExprClosure,
}

enum Matcher {
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
                    return Ok(Self::Expression(expr));
                }
            }
        } else {
            None
        };

        if let Some(state_type) = state_type {
            if input.parse::<Token![,]>().is_ok() {
                let is_closure = input.peek(Token![|]);
                if is_closure {
                    Ok(Self::Closure(MatcherClosure::parse_with_state_type(
                        input,
                        Some(state_type),
                    )?))
                } else {
                    Ok(Self::Pattern(MatcherPattern::parse_with_state_type(
                        input,
                        Some(state_type),
                    )?))
                }
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
        Self::parse_with_state_type(input, None)
    }
}

impl MatcherPattern {
    fn parse_with_state_type(
        input: syn::parse::ParseStream,
        state_type: Option<Path>,
    ) -> syn::Result<Self> {
        let state_type = if let Some(state_type) = state_type {
            state_type
        } else {
            let state_type = Path::parse(input)?;
            input.parse::<Token![,]>()?;
            state_type
        };

        let every = {
            let ahead = input.fork();
            let every: syn::Result<Ident> = ahead.parse();

            if let Ok(every) = every {
                if every == "every" {
                    input.parse::<Ident>()?;
                    true
                } else {
                    false
                }
            } else {
                false
            }
        };
        let pattern = Pat::parse_multi_with_leading_vert(input)?;

        let pattern = inject_state_type(pattern, &state_type);

        Ok(Self {
            state_type,
            pattern,
            every,
        })
    }
}

impl Parse for MatcherClosure {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        Self::parse_with_state_type(input, None)
    }
}

impl MatcherClosure {
    fn parse_with_state_type(
        input: syn::parse::ParseStream,
        state_type: Option<Path>,
    ) -> syn::Result<Self> {
        let state_type = if let Some(state_type) = state_type {
            state_type
        } else {
            let state_type = Path::parse(input)?;
            input.parse::<Token![,]>()?;
            state_type
        };

        let closure = ExprClosure::parse(input)?;

        Ok(Self {
            state_type,
            closure,
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

pub fn define_state_matcher(
    input: proc_macro::TokenStream,
) -> syn::Result<proc_macro::TokenStream> {
    let StateMatcher {
        visibility,
        name,
        matcher,
    } = syn::parse(input)?;

    let MatcherPattern {
        state_type,
        pattern,
        ..
    } = matcher;

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

pub enum MatchMacroResult {
    SimpleEquality(TokenStream),
    Pattern {
        every: bool,
        tokens: TokenStream,
        state_type: Path,
    },
    Closure {
        tokens: TokenStream,
        state_type: Path,
    },
}

pub fn define_match_macro(input: proc_macro::TokenStream) -> syn::Result<MatchMacroResult> {
    let matcher = syn::parse::<Matcher>(input)?;

    Ok(match matcher {
        Matcher::Expression(exp) => MatchMacroResult::SimpleEquality(quote!(
            #exp
        )),
        Matcher::Pattern(MatcherPattern {
            state_type,
            pattern,
            every,
        }) => MatchMacroResult::Pattern {
            every,
            state_type: state_type.clone(),
            tokens: quote!(matches!(state, #pattern)),
        },
        Matcher::Closure(MatcherClosure {
            state_type,
            closure: pattern,
        }) => MatchMacroResult::Closure {
            tokens: quote!(#pattern),
            state_type,
        },
    })
}

pub fn simple_state_transition_macros(
    macro_type: MatchMacro,
    every_override: bool,
    match_result: MatchMacroResult,
) -> proc_macro::TokenStream {
    let mut module_path = bevy_ecs_path();
    module_path.segments.push(format_ident!("schedule").into());

    match match_result {
        MatchMacroResult::SimpleEquality(expr) => {
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
        MatchMacroResult::Pattern {
            every,
            tokens,
            state_type,
        } => {
            let every = every || every_override;
            let mut module_path = state_type.clone();
            let state_type = state_type.clone().into_token_stream();

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

            let call = format_ident!("{}", if every { "every_entrance" } else { "matching" });

            let matches = quote!(fn matches(state: &#state_type) -> bool {
                #tokens
            });

            quote!({
                #matches

                #module_path().#call::<()>(matches)
            })
            .into()
        }
        MatchMacroResult::Closure { tokens, state_type } => {
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
}

pub fn state_matches_macro(match_result: MatchMacroResult) -> proc_macro::TokenStream {
    let mut module_path = bevy_ecs_path();
    module_path.segments.push(format_ident!("schedule").into());
    match match_result {
        MatchMacroResult::SimpleEquality(expr) => {
            module_path
                .segments
                .push(Ident::new("in_state", Span::call_site()).into());
            quote!(#module_path(#expr)).into()
        }
        MatchMacroResult::Pattern {
            tokens, state_type, ..
        } => quote!(|state: Option<Res<State<#state_type>>>| {
            let Some(state) = state else {
                return false;
            };
            let state : &#state_type = &state;
            #tokens
        })
        .into(),
        MatchMacroResult::Closure { tokens, state_type } => quote!(
            |state: Option<Res<State<#state_type>>>| {
                let Some(state) = state else {
                    return false;
                };
                let f = #tokens;

                f(&state)
            }
        )
        .into(),
    }
}
