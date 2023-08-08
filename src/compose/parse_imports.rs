use std::collections::HashMap;

use super::{
    tokenizer::{Token, Tokenizer},
    Composer, ImportDefWithOffset, ImportDefinition,
};

pub fn parse_imports<'a>(
    input: &'a str,
    declared_imports: &mut HashMap<String, Vec<String>>,
) -> Result<(), (&'a str, usize)> {
    let mut tokens = Tokenizer::new(input, false).peekable();

    match tokens.next() {
        Some(Token::Other('#', _)) => (),
        Some(other) => return Err(("expected `#import`", other.pos())),
        None => return Err(("expected #import", input.len())),
    };
    match tokens.next() {
        Some(Token::Identifier("import", _)) => (),
        Some(other) => return Err(("expected `#import`", other.pos())),
        None => return Err(("expected `#import`", input.len())),
    };

    let mut stack = Vec::default();
    let mut current = String::default();
    let mut as_name = None;
    let mut is_deprecated_itemlist = false;

    loop {
        match tokens.peek() {
            Some(Token::Identifier(ident, _)) => {
                current.push_str(ident);
                tokens.next();

                if tokens.peek().and_then(Token::identifier) == Some("as") {
                    let pos = tokens.next().unwrap().pos();
                    let Some(Token::Identifier(name, _)) = tokens.next() else {
                        return Err(("expected identifier after `as`", pos));
                    };

                    as_name = Some(name);
                }

                // support deprecated #import mod item
                if let Some(Token::Identifier(..)) = tokens.peek() {
                    is_deprecated_itemlist = true;
                    stack.push(format!("{}::", current));
                    current = String::default();
                    as_name = None;
                }

                continue;
            }
            Some(Token::Other('{', pos)) => {
                if !current.ends_with("::") {
                    return Err(("open brace must follow `::`", *pos));
                }
                stack.push(current);
                current = String::default();
                as_name = None;
            }
            Some(Token::Other(',', _))
            | Some(Token::Other('}', _))
            | Some(Token::Other('\n', _))
            | None => {
                if !current.is_empty() {
                    let used_name = as_name.map(ToString::to_string).unwrap_or_else(|| {
                        current
                            .rsplit_once("::")
                            .map(|(_, name)| name.to_owned())
                            .unwrap_or(current.clone())
                    });
                    declared_imports.entry(used_name).or_default().push(format!(
                        "{}{}",
                        stack.join(""),
                        current
                    ));
                    current = String::default();
                    as_name = None;
                }

                if let Some(Token::Other('}', pos)) = tokens.peek() {
                    if stack.pop().is_none() {
                        return Err(("close brace without open", *pos));
                    }
                }

                if tokens.peek().is_none() {
                    break;
                }
            }
            Some(Token::Other(';', _)) => {
                tokens.next();
                if let Some(token) = tokens.peek() {
                    return Err(("unexpected token after ';'", token.pos()));
                }
            }
            Some(Token::Other(_, pos)) => return Err(("unexpected token", *pos)),
            Some(Token::Whitespace(..)) => unreachable!(),
        }

        tokens.next();
    }

    if !(stack.is_empty() || is_deprecated_itemlist && stack.len() == 1) {
        return Err(("missing close brace", input.len()));
    }

    Ok(())
}

pub fn substitute_identifiers(
    input: &str,
    offset: usize,
    declared_imports: &HashMap<String, Vec<String>>,
    used_imports: &mut HashMap<String, ImportDefWithOffset>,
    allow_ambiguous: bool,
) -> Result<String, usize> {
    let tokens = Tokenizer::new(input, true);
    let mut output = String::with_capacity(input.len());
    let mut in_substitution_position = true;

    for token in tokens {
        match token {
            Token::Identifier(ident, token_pos) => {
                if in_substitution_position {
                    let (first, residual) = ident.split_once("::").unwrap_or((ident, ""));
                    let full_paths = declared_imports
                        .get(first)
                        .cloned()
                        .unwrap_or(vec![first.to_owned()]);

                    if !allow_ambiguous && full_paths.len() > 1 {
                        return Err(offset + token_pos);
                    }

                    for mut full_path in full_paths {
                        if !residual.is_empty() {
                            full_path.push_str("::");
                            full_path.push_str(residual);
                        }

                        if let Some((module, item)) = full_path.rsplit_once("::") {
                            used_imports
                                .entry(module.to_owned())
                                .or_insert_with(|| ImportDefWithOffset {
                                    definition: ImportDefinition {
                                        import: module.to_owned(),
                                        ..Default::default()
                                    },
                                    offset: offset + token_pos,
                                })
                                .definition
                                .items
                                .push(item.to_owned());
                            output.push_str(item);
                            output.push_str(&Composer::decorate(module));
                        } else if full_path.find('"').is_some() {
                            // we don't want to replace local variables that shadow quoted module imports with the
                            // quoted name as that won't compile.
                            // since quoted items always refer to modules, we can just emit the original ident
                            // in this case
                            output.push_str(ident);
                        } else {
                            // if there are no quotes we do the replacement. this means that individually imported
                            // items can be used, and any shadowing local variables get harmlessly renamed.
                            // TODO: it can lead to weird errors, but such is life
                            output.push_str(&full_path);
                        }
                    }
                } else {
                    output.push_str(ident);
                }
            }
            Token::Other(other, _) => {
                output.push(other);
                if other == '.' || other == '@' {
                    in_substitution_position = false;
                    continue;
                }
            }
            Token::Whitespace(ws, _) => output.push_str(ws),
        }

        in_substitution_position = true;
    }

    Ok(output)
}

#[cfg(test)]
fn test_parse(input: &str) -> Result<HashMap<String, Vec<String>>, (&str, usize)> {
    let mut declared_imports = HashMap::default();
    parse_imports(input, &mut declared_imports)?;
    Ok(declared_imports)
}

#[test]
fn import_tokens() {
    let input = r"
        #import a::b
    ";
    assert_eq!(
        test_parse(input),
        Ok(HashMap::from_iter([(
            "b".to_owned(),
            vec!("a::b".to_owned())
        )]))
    );

    let input = r"
        #import a::{b, c}
    ";
    assert_eq!(
        test_parse(input),
        Ok(HashMap::from_iter([
            ("b".to_owned(), vec!("a::b".to_owned())),
            ("c".to_owned(), vec!("a::c".to_owned())),
        ]))
    );

    let input = r"
        #import a::{b as d, c}
    ";
    assert_eq!(
        test_parse(input),
        Ok(HashMap::from_iter([
            ("d".to_owned(), vec!("a::b".to_owned())),
            ("c".to_owned(), vec!("a::c".to_owned())),
        ]))
    );

    let input = r"
        #import a::{b::{c, d}, e}
    ";
    assert_eq!(
        test_parse(input),
        Ok(HashMap::from_iter([
            ("c".to_owned(), vec!("a::b::c".to_owned())),
            ("d".to_owned(), vec!("a::b::d".to_owned())),
            ("e".to_owned(), vec!("a::e".to_owned())),
        ]))
    );

    let input = r"
        #import a::b::{c, d}, e
    ";
    assert_eq!(
        test_parse(input),
        Ok(HashMap::from_iter([
            ("c".to_owned(), vec!("a::b::c".to_owned())),
            ("d".to_owned(), vec!("a::b::d".to_owned())),
            ("e".to_owned(), vec!("e".to_owned())),
        ]))
    );

    let input = r"
        #import a, b
    ";
    assert_eq!(
        test_parse(input),
        Ok(HashMap::from_iter([
            ("a".to_owned(), vec!("a".to_owned())),
            ("b".to_owned(), vec!("b".to_owned())),
        ]))
    );

    let input = r"
        #import a::b c, d
    ";
    assert_eq!(
        test_parse(input),
        Ok(HashMap::from_iter([
            ("c".to_owned(), vec!("a::b::c".to_owned())),
            ("d".to_owned(), vec!("a::b::d".to_owned())),
        ]))
    );

    let input = r"
        #import a::b c
    ";
    assert_eq!(
        test_parse(input),
        Ok(HashMap::from_iter([(
            "c".to_owned(),
            vec!("a::b::c".to_owned())
        ),]))
    );

    let input = r"
        #import a::b::{c::{d, e}, f, g::{h as i, j}}
    ";
    assert_eq!(
        test_parse(input),
        Ok(HashMap::from_iter([
            ("d".to_owned(), vec!("a::b::c::d".to_owned())),
            ("e".to_owned(), vec!("a::b::c::e".to_owned())),
            ("f".to_owned(), vec!("a::b::f".to_owned())),
            ("i".to_owned(), vec!("a::b::g::h".to_owned())),
            ("j".to_owned(), vec!("a::b::g::j".to_owned())),
        ]))
    );

    let input = r"
        #import a::b::{
            c::{d, e}, 
            f, 
            g::{
                h as i, 
                j::k::l as m,
            }
        }
    ";
    assert_eq!(
        test_parse(input),
        Ok(HashMap::from_iter([
            ("d".to_owned(), vec!("a::b::c::d".to_owned())),
            ("e".to_owned(), vec!("a::b::c::e".to_owned())),
            ("f".to_owned(), vec!("a::b::f".to_owned())),
            ("i".to_owned(), vec!("a::b::g::h".to_owned())),
            ("m".to_owned(), vec!("a::b::g::j::k::l".to_owned())),
        ]))
    );

    let input = r#"
        #import "path//with\ all sorts of .stuff"::{a, b}
    "#;
    assert_eq!(
        test_parse(input),
        Ok(HashMap::from_iter([
            (
                "a".to_owned(),
                vec!(r#""path//with\ all sorts of .stuff"::a"#.to_owned())
            ),
            (
                "b".to_owned(),
                vec!(r#""path//with\ all sorts of .stuff"::b"#.to_owned())
            ),
        ]))
    );

    let input = r"
        #import a::b::{
    ";
    assert!(test_parse(input).is_err());

    let input = r"
        #import a::b::{c}}
    ";
    assert!(test_parse(input).is_err());

    let input = r"
        #import a::b::{c}}
    ";
    assert!(test_parse(input).is_err());

    let input = r"
        #import a::b{{c,d}}
    ";
    assert!(test_parse(input).is_err());

    let input = r"
        #import a:b
    ";
    assert!(test_parse(input).is_err());
}
